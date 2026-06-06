use anyhow::{Context, Result, bail};
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::header,
    response::sse::{Event, KeepAlive, Sse},
    response::{Html, IntoResponse, Response},
    routing::{get, post},
};
use dioxus::prelude::*;
use dioxus_ssr::render_element;
use futures_util::StreamExt;
use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::BTreeMap,
    convert::Infallible,
    fs,
    net::{IpAddr, SocketAddr},
    path::{Component, Path as FsPath, PathBuf},
    process::Command,
    sync::{Arc, OnceLock},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use syu_app_ui::{
    AppShell, HelpTopic, Locale, WorkbenchActionRunPreview, WorkbenchPane, WorkbenchUiState,
    model::{
        CliCommandPreview, SpecBrowserDocument, SpecBrowserItem, SpecBrowserModel,
        SpecBrowserSection, SpecBrowserTraceGroup, SpecBrowserTraceReference, cli_command_catalog,
    },
};
use syu_code_intel::{
    BranchScopeEvidence, BranchScopeReport, ChangedFileReport, OwnershipStatus,
    resolve_git_range_changed_files,
};
use syu_core::{
    AppPayload, AppServer, BrowserItem, BrowserWorkspace, HistoricalIdSnapshot, SectionKind,
    SourceDocument, ValidationSnapshot, build_browser_workspace,
};
use syu_domain::Issue;
use syu_task_model::{
    ClassificationOutcome, GoalPlanArtifact, GoalPlanCheckReport, GoalPlanCompletion,
    GoalPlanConfidence, GoalPlanCoverage, GoalPlanCoverageMode, GoalPlanGoal,
    GoalPlanImplementationPlan, GoalPlanPersistentItem, GoalPlanPersistentItemDetails,
    GoalPlanPersistentItems, GoalPlanScope, GoalPlanScopeInclude, GoalPlanSelectionMode,
    GoalPlanSource, GoalPlanSourceEvidence, GoalPlanSourceMode, GoalPlanSpecMapping,
    GoalPlanTestPlan, RequestArtifact, RequestArtifactContext, RequestClassification,
    ScaffoldAction, ScaffoldPlan, ScaffoldUpdate, ScaffoldUpdateKind, ScopeFeatureCandidate,
    ScopeOutcome, ScopeSignals, SearchResult, TaskTestSelectionCommand,
    TaskTestSelectionEscalation, TaskTestSelectionPlan,
};
use syu_workbench as shared_workbench;
use tokio::{
    sync::{RwLock, broadcast, mpsc},
    task,
};
use tokio_stream::wrappers::BroadcastStream;

mod browser;
use browser::*;
mod adapters;
use adapters::*;
mod routes;
use routes::*;
mod workspace;
use workspace::*;
mod actions;
use actions::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceStatus {
    Pending,
    Pass,
    Warn,
    Fail,
    Skipped,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceSource {
    Action {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        action_id: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        action_label: Option<String>,
    },
    Command {
        command: String,
    },
    System {
        component: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct EvidenceAttachment {
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkbenchEvidenceKind {
    RequestArtifact,
    ClassificationOutcome,
    ScopeOutcome,
    ScaffoldPlan,
    GoalPlanArtifact,
    TaskTestSelectionPlan,
    GoalPlanCheckReport,
    BranchScopeReport,
    ValidationReport,
    HistoryResponse,
    AssignmentState,
    JobState,
}

fn is_false(value: &bool) -> bool {
    !*value
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssignmentAssignee {
    Human { name: String },
    Ai { model: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct BoundedScope {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub range: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub allowed_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_files: Option<usize>,
}

impl BoundedScope {
    pub fn is_bounded(&self) -> bool {
        self.range.is_some() || !self.allowed_ids.is_empty() || self.max_files.is_some()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Default)]
pub struct WorkspaceSnapshot {
    pub workspace_root: PathBuf,
    pub spec_root: PathBuf,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub validation_summary: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Default)]
pub struct ActiveRequestState {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_path: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact: Option<RequestArtifact>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub classification: Option<ClassificationOutcome>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<ScopeOutcome>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scaffold: Option<ScaffoldPlan>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Default)]
pub struct ActiveGoalState {
    pub goal_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub goal_plan: Option<GoalPlanArtifact>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub test_selection: Option<TaskTestSelectionPlan>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub check_report: Option<GoalPlanCheckReport>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Default)]
pub struct GoalListState {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub active: Vec<ActiveGoalState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_goal_id: Option<String>,
}

impl GoalListState {
    pub fn has_active_goal_plan(&self) -> bool {
        self.active_goal()
            .is_some_and(|goal| goal.goal_plan.is_some())
    }

    pub fn active_goal(&self) -> Option<&ActiveGoalState> {
        if let Some(selected_goal_id) = &self.selected_goal_id
            && let Some(goal) = self
                .active
                .iter()
                .find(|goal| &goal.goal_id == selected_goal_id)
        {
            return Some(goal);
        }
        self.active.first()
    }

    pub fn active_goal_mut(&mut self) -> &mut ActiveGoalState {
        if self.active.is_empty() {
            self.active.push(ActiveGoalState {
                goal_id: "goal-1".to_string(),
                ..ActiveGoalState::default()
            });
        }
        if let Some(selected_goal_id) = &self.selected_goal_id
            && let Some(index) = self
                .active
                .iter()
                .position(|goal| &goal.goal_id == selected_goal_id)
        {
            return &mut self.active[index];
        }
        &mut self.active[0]
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Default)]
pub struct AssignmentState {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub goal_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assignee: Option<AssignmentAssignee>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<BoundedScope>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub expected_evidence: Vec<WorkbenchEvidenceKind>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    Idle,
    Queued,
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct JobState {
    pub status: JobStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl Default for JobState {
    fn default() -> Self {
        Self {
            status: JobStatus::Idle,
            action_id: None,
            message: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvidenceEntry {
    pub kind: WorkbenchEvidenceKind,
    pub status: EvidenceStatus,
    pub summary: String,
    pub timestamp: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub goal_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<EvidenceSource>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attachments: Vec<EvidenceAttachment>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct EvidenceTimelineState {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub entries: Vec<EvidenceEntry>,
}

impl EvidenceTimelineState {
    fn append(&mut self, entry: EvidenceEntry) {
        self.entries.push(entry);
    }
}

fn evidence_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or_default()
}

fn json_attachment<T: Serialize>(value: &T) -> EvidenceAttachment {
    let json = serde_json::to_string_pretty(value).unwrap_or_else(|_| "{}".to_string());
    const MAX_ATTACHMENT_CHARS: usize = 4096;
    let truncated = json.len() > MAX_ATTACHMENT_CHARS;
    let content = if truncated {
        Some(json.chars().take(MAX_ATTACHMENT_CHARS).collect())
    } else {
        Some(json)
    };
    EvidenceAttachment {
        label: "result".to_string(),
        mime_type: Some("application/json".to_string()),
        summary: Some(if truncated {
            "truncated JSON payload".to_string()
        } else {
            "JSON payload".to_string()
        }),
        content,
        truncated,
    }
}

fn evidence_entry(
    kind: WorkbenchEvidenceKind,
    status: EvidenceStatus,
    summary: impl Into<String>,
    goal_id: Option<String>,
    action_id: Option<String>,
    source: Option<EvidenceSource>,
    attachments: Vec<EvidenceAttachment>,
) -> EvidenceEntry {
    EvidenceEntry {
        kind,
        status,
        summary: summary.into(),
        timestamp: evidence_timestamp(),
        goal_id,
        action_id,
        source,
        attachments,
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Default)]
pub struct WorkbenchConfirmationMetadata {
    pub confirmed_by: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rationale: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope_token: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Default)]
pub struct CommandPaletteState {
    pub query: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_action_id: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub visible_actions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Default)]
pub struct WorkbenchState {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace: Option<WorkspaceSnapshot>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request: Option<ActiveRequestState>,
    #[serde(default)]
    pub goals: GoalListState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub branch_scope: Option<BranchScopeReport>,
    #[serde(default)]
    pub evidence_timeline: EvidenceTimelineState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assignment: Option<AssignmentState>,
    #[serde(default)]
    pub job: JobState,
    #[serde(default)]
    pub command_palette: CommandPaletteState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confirmation: Option<WorkbenchConfirmationMetadata>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct WorkbenchAction {
    pub id: String,
    pub title: String,
    pub description: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub required_state: Vec<String>,
    pub mutability: String,
    pub risk: String,
    pub ai_eligible: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct WorkbenchActionAvailability {
    pub id: String,
    pub title: String,
    pub available: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub missing_state: Vec<String>,
    pub mutability: String,
    pub risk: String,
    pub ai_eligible: bool,
}

#[derive(Debug, Clone)]
pub struct WorkbenchActionRegistry {
    actions: &'static [WorkbenchAction],
}

impl WorkbenchActionRegistry {
    pub fn standard() -> Self {
        Self {
            actions: registry_actions(),
        }
    }

    pub fn actions(&self) -> &'static [WorkbenchAction] {
        self.actions
    }

    pub fn availability(&self, state: &WorkbenchState) -> Vec<WorkbenchActionAvailability> {
        let context = WorkbenchActionContext {
            workspace: state.workspace.is_some(),
            request: state.request.is_some(),
            active_goal_plan: state.goals.has_active_goal_plan(),
            branch_scope: state.branch_scope.is_some(),
            assignment: state.assignment.is_some(),
            confirmation: state.confirmation.is_some(),
            bounded_scope: state.branch_scope.is_some()
                || state
                    .assignment
                    .as_ref()
                    .and_then(|assignment| assignment.scope.as_ref())
                    .is_some_and(BoundedScope::is_bounded),
        };

        self.actions
            .iter()
            .map(|action| action.availability(&context))
            .collect()
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct WorkbenchActionContext {
    workspace: bool,
    request: bool,
    active_goal_plan: bool,
    branch_scope: bool,
    assignment: bool,
    confirmation: bool,
    bounded_scope: bool,
}

impl WorkbenchAction {
    fn availability(&self, context: &WorkbenchActionContext) -> WorkbenchActionAvailability {
        let mut missing_state = Vec::new();
        for requirement in &self.required_state {
            let satisfied = match requirement.as_str() {
                "workspace_loaded" => context.workspace,
                "active_request" => context.request,
                "active_goal_plan" => context.active_goal_plan,
                "branch_scope_loaded" => context.branch_scope,
                "assignment_loaded" => context.assignment,
                "confirmation_metadata" => context.confirmation,
                "bounded_scope" => context.bounded_scope,
                _ => true,
            };
            if !satisfied {
                missing_state.push(requirement.clone());
            }
        }

        WorkbenchActionAvailability {
            id: self.id.clone(),
            title: self.title.clone(),
            available: missing_state.is_empty(),
            missing_state,
            mutability: self.mutability.clone(),
            risk: self.risk.clone(),
            ai_eligible: self.ai_eligible,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct WorkbenchApiPayload {
    pub state: WorkbenchState,
    pub actions: Vec<WorkbenchAction>,
    pub availability: Vec<WorkbenchActionAvailability>,
}

impl WorkbenchApiPayload {
    pub fn new(state: WorkbenchState) -> Self {
        let registry = WorkbenchActionRegistry::standard();
        let actions = registry.actions().to_vec();
        let availability = registry.availability(&state);
        Self {
            state,
            actions,
            availability,
        }
    }
}

fn registry_actions() -> &'static [WorkbenchAction] {
    static REGISTRY: OnceLock<Vec<WorkbenchAction>> = OnceLock::new();
    REGISTRY.get_or_init(|| {
        vec![
            WorkbenchAction {
                id: "request.new".to_string(),
                title: "New request".to_string(),
                description: "Capture a new request artifact for the active workspace.".to_string(),
                required_state: vec![
                    "workspace_loaded".to_string(),
                    "confirmation_metadata".to_string(),
                ],
                mutability: "mutates_state_and_files".to_string(),
                risk: "medium".to_string(),
                ai_eligible: false,
            },
            WorkbenchAction {
                id: "request.classify".to_string(),
                title: "Classify request".to_string(),
                description: "Determine whether the active request is a create, change, or delete."
                    .to_string(),
                required_state: vec!["active_request".to_string()],
                mutability: "read_only".to_string(),
                risk: "low".to_string(),
                ai_eligible: false,
            },
            WorkbenchAction {
                id: "request.scope".to_string(),
                title: "Scope request".to_string(),
                description:
                    "Map the active request to the relevant specification graph and impact area."
                        .to_string(),
                required_state: vec!["active_request".to_string()],
                mutability: "read_only".to_string(),
                risk: "low".to_string(),
                ai_eligible: false,
            },
            WorkbenchAction {
                id: "request.scaffold".to_string(),
                title: "Scaffold request".to_string(),
                description:
                    "Preview the spec and file updates needed to realize the active request."
                        .to_string(),
                required_state: vec![
                    "active_request".to_string(),
                    "confirmation_metadata".to_string(),
                ],
                mutability: "mutates_state_and_files".to_string(),
                risk: "medium".to_string(),
                ai_eligible: false,
            },
            WorkbenchAction {
                id: "request.plan".to_string(),
                title: "Plan request".to_string(),
                description: "Turn the scoped request into a temporary Goal Plan.".to_string(),
                required_state: vec![
                    "active_request".to_string(),
                    "confirmation_metadata".to_string(),
                ],
                mutability: "mutates_state_and_files".to_string(),
                risk: "medium".to_string(),
                ai_eligible: false,
            },
            WorkbenchAction {
                id: "goal.test_select".to_string(),
                title: "Select goal tests".to_string(),
                description: "Choose the narrowest tests that cover the active Goal Plan."
                    .to_string(),
                required_state: vec!["active_goal_plan".to_string()],
                mutability: "read_only".to_string(),
                risk: "low".to_string(),
                ai_eligible: false,
            },
            WorkbenchAction {
                id: "goal.check".to_string(),
                title: "Check goal".to_string(),
                description: "Compare the active Goal Plan against the current branch range."
                    .to_string(),
                required_state: vec!["active_goal_plan".to_string()],
                mutability: "read_only".to_string(),
                risk: "low".to_string(),
                ai_eligible: false,
            },
            WorkbenchAction {
                id: "branch.scope".to_string(),
                title: "Load branch scope".to_string(),
                description: "Summarize the current branch scope and visible impact surface."
                    .to_string(),
                required_state: vec!["workspace_loaded".to_string()],
                mutability: "read_only".to_string(),
                risk: "low".to_string(),
                ai_eligible: false,
            },
            WorkbenchAction {
                id: "validation.run".to_string(),
                title: "Run validation".to_string(),
                description: "Run the repository validation pass for the active workspace."
                    .to_string(),
                required_state: vec!["workspace_loaded".to_string()],
                mutability: "read_only".to_string(),
                risk: "low".to_string(),
                ai_eligible: false,
            },
            WorkbenchAction {
                id: "history.show".to_string(),
                title: "Show history".to_string(),
                description: "Show the evidence trail for the active request or goal.".to_string(),
                required_state: vec!["workspace_loaded".to_string()],
                mutability: "read_only".to_string(),
                risk: "low".to_string(),
                ai_eligible: false,
            },
            WorkbenchAction {
                id: "assignment.create".to_string(),
                title: "Create assignment".to_string(),
                description:
                    "Assign the active goal to a human or AI with explicit scope and evidence."
                        .to_string(),
                required_state: vec![
                    "active_goal_plan".to_string(),
                    "confirmation_metadata".to_string(),
                ],
                mutability: "mutates_state".to_string(),
                risk: "medium".to_string(),
                ai_eligible: false,
            },
            WorkbenchAction {
                id: "agent.run".to_string(),
                title: "Run agent".to_string(),
                description: "Launch an AI run against a bounded goal scope and assignment."
                    .to_string(),
                required_state: vec![
                    "active_goal_plan".to_string(),
                    "assignment_loaded".to_string(),
                    "bounded_scope".to_string(),
                    "confirmation_metadata".to_string(),
                ],
                mutability: "mutates_state_and_files".to_string(),
                risk: "high".to_string(),
                ai_eligible: true,
            },
        ]
    })
}

#[derive(Debug, Clone)]
pub struct WorkbenchLaunchConfig {
    pub workspace_root: PathBuf,
    pub spec_root: PathBuf,
    pub bind: String,
    pub port: u16,
    pub allow_remote_bind: bool,
    pub show_log: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum WorkbenchEvent {
    WorkspaceReloaded {
        workspace_root: String,
        spec_root: String,
        item_count: usize,
    },
    ValidationUpdated {
        summary: String,
    },
    RequestCreated {
        request: String,
    },
    RequestClassified {
        classification: RequestClassification,
        request: String,
    },
    RequestScoped {
        request: String,
        requirement_count: usize,
    },
    RequestScaffolded {
        request: String,
    },
    GoalPlanGenerated {
        goal_id: String,
    },
    GoalTestsSelected {
        goal_id: String,
    },
    GoalChecked {
        goal_id: String,
    },
    BranchScopeUpdated {
        range: String,
        changed_files: usize,
    },
    EvidenceAdded {
        kind: String,
        summary: String,
    },
    AssignmentCreated {
        goal_id: String,
    },
    JobStarted {
        job_id: String,
    },
    JobOutput {
        job_id: String,
        line: String,
    },
    JobCompleted {
        job_id: String,
    },
    JobCancelled {
        job_id: String,
    },
}

#[derive(Debug, Clone, Serialize)]
pub struct WorkbenchHealth {
    pub ok: bool,
    pub workspace_root: String,
    pub spec_root: String,
    pub bind: String,
    pub port: u16,
}

#[derive(Debug, Clone, Serialize)]
pub struct WorkbenchActionCatalog {
    pub actions: Vec<WorkbenchAction>,
    pub availability: Vec<WorkbenchActionAvailability>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SpecItemResponse {
    pub section: SectionKind,
    pub document_path: String,
    pub item: BrowserItem,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RequestPlanRequest {
    pub request: RequestArtifact,
    #[serde(default)]
    pub request_path: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GoalCheckRequest {
    pub plan: GoalPlanArtifact,
    #[serde(default)]
    pub range: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssignmentRequest {
    pub assignee: AssignmentAssignee,
    pub scope: BoundedScope,
    #[serde(default)]
    pub expected_evidence: Vec<WorkbenchEvidenceKind>,
}

#[derive(Debug, Clone, Serialize)]
pub struct JobRecord {
    pub id: String,
    pub action_id: Option<String>,
    pub status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ActionRunResponse {
    pub action_id: String,
    pub event: WorkbenchEvent,
    pub result: Value,
}

#[derive(Debug)]
struct WorkbenchServerInner {
    config: WorkbenchLaunchConfig,
    browser_workspace: RwLock<BrowserWorkspace>,
    state: RwLock<WorkbenchState>,
    jobs: RwLock<BTreeMap<String, JobRecord>>,
    events: broadcast::Sender<WorkbenchEvent>,
}

#[derive(Clone, Debug)]
pub struct WorkbenchServer {
    inner: Arc<WorkbenchServerInner>,
}

impl WorkbenchServer {
    pub fn new(config: WorkbenchLaunchConfig) -> Result<Self> {
        validate_bind(&config.bind, config.allow_remote_bind)?;
        let browser_workspace = load_browser_workspace(&config.workspace_root, &config.spec_root)?;
        let state = initial_state(&browser_workspace, &config);
        let (events, _) = broadcast::channel(256);
        let workspace_root = browser_workspace.workspace_root.clone();
        let spec_root = browser_workspace.spec_root.clone();
        let item_count = browser_workspace.item_index.len();
        let server = Self {
            inner: Arc::new(WorkbenchServerInner {
                config,
                browser_workspace: RwLock::new(browser_workspace),
                state: RwLock::new(state),
                jobs: RwLock::new(BTreeMap::new()),
                events,
            }),
        };
        let _ = server.inner.events.send(WorkbenchEvent::WorkspaceReloaded {
            workspace_root,
            spec_root,
            item_count,
        });
        Ok(server)
    }

    pub fn router(&self) -> Router {
        Router::new()
            .route("/", get(workbench_index))
            .route("/assets/tailwind.css", get(workbench_css))
            .route("/api/health", get(health))
            .route("/api/workspace/snapshot", get(workspace_snapshot))
            .route("/api/actions", get(list_actions))
            .route("/api/actions/{id}/run", post(run_action))
            .route("/api/spec/graph", get(spec_graph))
            .route("/api/spec/item/{id}", get(spec_item))
            .route("/api/branch/scope", get(branch_scope))
            .route("/api/request/classify", post(request_classify))
            .route("/api/request/scope", post(request_scope))
            .route("/api/request/scaffold", post(request_scaffold))
            .route("/api/request/plan", post(request_plan))
            .route("/api/goals", get(list_goals))
            .route("/api/goals/{id}", get(goal_by_id))
            .route("/api/goals/{id}/test-select", post(goal_test_select))
            .route("/api/goals/{id}/check", post(goal_check))
            .route("/api/goals/{id}/assign", post(goal_assign))
            .route("/api/evidence", get(list_evidence))
            .route("/api/jobs", get(list_jobs))
            .route("/api/jobs/{id}", get(job_by_id))
            .route("/api/jobs/{id}/cancel", post(cancel_job))
            .route("/api/events", get(events))
            .with_state(self.clone())
    }

    pub async fn serve(self) -> Result<()> {
        let addr = parse_socket_addr(&self.inner.config.bind, self.inner.config.port)?;
        let listener = tokio::net::TcpListener::bind(addr)
            .await
            .with_context(|| format!("failed to bind workbench server at `{addr}`"))?;
        let local_addr = listener
            .local_addr()
            .context("failed to read workbench listener address")?;
        println!("Syu Workbench listening at http://{local_addr}");
        if self.inner.config.show_log {
            println!("Workbench command logs are visible in result views.");
        }
        let watcher = self.spawn_watcher()?;
        let _keep_watcher_alive = watcher;
        axum::serve(listener, self.router())
            .await
            .context("workbench server stopped unexpectedly")
    }

    fn spawn_watcher(&self) -> Result<RecommendedWatcher> {
        let (reload_tx, mut reload_rx) = mpsc::unbounded_channel::<()>();
        let config = self.inner.config.clone();
        let inner = Arc::clone(&self.inner);

        task::spawn(async move {
            while reload_rx.recv().await.is_some() {
                if let Ok(browser_workspace) =
                    load_browser_workspace(&config.workspace_root, &config.spec_root)
                {
                    let item_count = browser_workspace.item_index.len();
                    {
                        let mut workspace = inner.browser_workspace.write().await;
                        *workspace = browser_workspace;
                    }
                    {
                        let mut state = inner.state.write().await;
                        state.workspace = Some(WorkspaceSnapshot {
                            workspace_root: config.workspace_root.clone(),
                            spec_root: config.spec_root.clone(),
                            branch: current_git_branch(&config.workspace_root),
                            validation_summary: Some(format!("{item_count} items")),
                        });
                    }
                    let _ = inner.events.send(WorkbenchEvent::WorkspaceReloaded {
                        workspace_root: config.workspace_root.display().to_string(),
                        spec_root: config.spec_root.display().to_string(),
                        item_count,
                    });
                }
            }
        });

        let workspace_root = self.inner.config.workspace_root.clone();
        let spec_root = self.inner.config.spec_root.clone();
        let mut watcher =
            notify::recommended_watcher(move |result: Result<notify::Event, notify::Error>| {
                if let Ok(event) = result
                    && matches!(
                        event.kind,
                        EventKind::Any | EventKind::Create(_) | EventKind::Modify(_)
                    )
                {
                    let _ = reload_tx.send(());
                }
            })
            .context("failed to create filesystem watcher")?;
        watcher
            .watch(&workspace_root, RecursiveMode::Recursive)
            .with_context(|| {
                format!(
                    "failed to watch workspace root `{}`",
                    workspace_root.display()
                )
            })?;
        if spec_root != workspace_root {
            watcher
                .watch(&spec_root, RecursiveMode::Recursive)
                .with_context(|| format!("failed to watch spec root `{}`", spec_root.display()))?;
        }
        Ok(watcher)
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
struct WorkbenchViewQuery {
    #[serde(default)]
    pane: Option<String>,
    #[serde(default)]
    sidebar: Option<String>,
    #[serde(default)]
    query: Option<String>,
    #[serde(default)]
    action: Option<String>,
    #[serde(default)]
    run: Option<String>,
    #[serde(default)]
    action_input: Option<String>,
    #[serde(default)]
    action_confirm: Option<String>,
    #[serde(default)]
    cli: Option<String>,
    #[serde(default)]
    cli_arg: Option<String>,
    #[serde(default)]
    cli_confirm: Option<String>,
    #[serde(default)]
    show_log: Option<String>,
    #[serde(default)]
    spec_item: Option<String>,
    #[serde(default)]
    goal: Option<String>,
    #[serde(default)]
    lang: Option<String>,
    #[serde(default)]
    help: Option<String>,
}

#[cfg(test)]
mod tests;
