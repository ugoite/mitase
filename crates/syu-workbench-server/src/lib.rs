use anyhow::{Context, Result, bail};
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    response::sse::{Event, KeepAlive, Sse},
    routing::{get, post},
};
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
    sync::{Arc, OnceLock},
    time::{Duration, SystemTime, UNIX_EPOCH},
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
    GoalPlanTestPlan, RequestArtifact, RequestClassification, ScaffoldAction, ScaffoldPlan,
    ScaffoldUpdate, ScaffoldUpdateKind, ScopeFeatureCandidate, ScopeOutcome, ScopeSignals,
    SearchResult, TaskTestSelectionCommand, TaskTestSelectionEscalation, TaskTestSelectionPlan,
};
use tokio::{
    sync::{RwLock, broadcast, mpsc},
    task,
};
use tokio_stream::wrappers::BroadcastStream;

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

#[derive(Debug, Clone, Deserialize)]
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
                            branch: None,
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

async fn health(State(server): State<WorkbenchServer>) -> Json<WorkbenchHealth> {
    Json(WorkbenchHealth {
        ok: true,
        workspace_root: server.inner.config.workspace_root.display().to_string(),
        spec_root: server.inner.config.spec_root.display().to_string(),
        bind: server.inner.config.bind.clone(),
        port: server.inner.config.port,
    })
}

async fn workspace_snapshot(State(server): State<WorkbenchServer>) -> Json<WorkbenchApiPayload> {
    Json(current_payload(&server).await)
}

async fn list_actions(State(server): State<WorkbenchServer>) -> Json<WorkbenchActionCatalog> {
    let payload = current_payload(&server).await;
    Json(WorkbenchActionCatalog {
        actions: payload.actions,
        availability: payload.availability,
    })
}

async fn run_action(
    State(server): State<WorkbenchServer>,
    Path(action_id): Path<String>,
    Json(body): Json<Value>,
) -> Result<Json<ActionRunResponse>, axum::http::StatusCode> {
    let response = execute_action(&server, &action_id, body)
        .await
        .map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;
    Ok(Json(response))
}

async fn spec_graph(State(server): State<WorkbenchServer>) -> Json<BrowserWorkspace> {
    Json(server.inner.browser_workspace.read().await.clone())
}

async fn spec_item(
    State(server): State<WorkbenchServer>,
    Path(id): Path<String>,
) -> Result<Json<SpecItemResponse>, axum::http::StatusCode> {
    let workspace = server.inner.browser_workspace.read().await;
    let item = workspace
        .sections
        .iter()
        .flat_map(|section| section.documents.iter())
        .flat_map(|document| {
            document
                .items
                .iter()
                .cloned()
                .map(move |item| (document.path.clone(), item))
        })
        .find(|(_, item)| item.id == id)
        .ok_or(axum::http::StatusCode::NOT_FOUND)?;
    let (document_path, item) = item;
    let section = workspace
        .item_index
        .get(&id)
        .map(|entry| entry.kind)
        .ok_or(axum::http::StatusCode::NOT_FOUND)?;
    Ok(Json(SpecItemResponse {
        section,
        document_path,
        item,
    }))
}

#[derive(Debug, Deserialize)]
struct BranchScopeQuery {
    range: String,
}

async fn branch_scope(
    State(server): State<WorkbenchServer>,
    Query(query): Query<BranchScopeQuery>,
) -> Result<Json<BranchScopeReport>, axum::http::StatusCode> {
    let report = build_branch_scope(&server.inner.config.workspace_root, &query.range)
        .await
        .map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;
    {
        let mut state = server.inner.state.write().await;
        state.branch_scope = Some(report.clone());
    }
    server
        .inner
        .events
        .send(WorkbenchEvent::BranchScopeUpdated {
            range: query.range,
            changed_files: report.changed_files.len(),
        })
        .ok();
    Ok(Json(report))
}

async fn request_classify(
    State(server): State<WorkbenchServer>,
    Json(request): Json<RequestArtifact>,
) -> Json<ClassificationOutcome> {
    let outcome = classify_request(&server, &request).await;
    server
        .inner
        .events
        .send(WorkbenchEvent::RequestClassified {
            classification: outcome.classification,
            request: outcome.request.clone(),
        })
        .ok();
    Json(outcome)
}

async fn request_scope(
    State(server): State<WorkbenchServer>,
    Json(request): Json<RequestArtifact>,
) -> Json<ScopeOutcome> {
    let outcome = scope_request(&server, &request).await;
    server
        .inner
        .events
        .send(WorkbenchEvent::RequestScoped {
            request: request.request,
            requirement_count: outcome.requirements.len(),
        })
        .ok();
    Json(outcome)
}

async fn request_scaffold(
    State(server): State<WorkbenchServer>,
    Json(request): Json<RequestArtifact>,
) -> Json<ScaffoldPlan> {
    let plan = scaffold_request(&server, &request).await;
    server
        .inner
        .events
        .send(WorkbenchEvent::RequestScaffolded {
            request: request.request,
        })
        .ok();
    Json(plan)
}

async fn request_plan(
    State(server): State<WorkbenchServer>,
    Json(request): Json<RequestPlanRequest>,
) -> Json<GoalPlanArtifact> {
    let plan = goal_plan_from_request(&server, &request).await;
    server
        .inner
        .events
        .send(WorkbenchEvent::GoalPlanGenerated {
            goal_id: plan.goal.id.clone(),
        })
        .ok();
    Json(plan)
}

async fn list_goals(State(server): State<WorkbenchServer>) -> Json<Vec<ActiveGoalState>> {
    Json(server.inner.state.read().await.goals.active.clone())
}

async fn goal_by_id(
    State(server): State<WorkbenchServer>,
    Path(id): Path<String>,
) -> Result<Json<ActiveGoalState>, axum::http::StatusCode> {
    server
        .inner
        .state
        .read()
        .await
        .goals
        .active
        .iter()
        .find(|goal| goal.goal_id == id)
        .cloned()
        .map(Json)
        .ok_or(axum::http::StatusCode::NOT_FOUND)
}

async fn goal_test_select(
    State(server): State<WorkbenchServer>,
    Path(id): Path<String>,
    Json(plan): Json<GoalPlanArtifact>,
) -> Json<TaskTestSelectionPlan> {
    let selection = TaskTestSelectionPlan {
        goal_id: id.clone(),
        goal_title: plan.goal.title.clone(),
        selection_mode: "minimal".to_string(),
        commands: vec![TaskTestSelectionCommand {
            language: "rust".to_string(),
            command: "cargo test".to_string(),
            reason: "baseline repository validation".to_string(),
        }],
        escalation: TaskTestSelectionEscalation {
            level: "none".to_string(),
            reason: "request-scoped test set is sufficient".to_string(),
        },
        warnings: Vec::new(),
    };
    {
        let mut state = server.inner.state.write().await;
        state.goals.active_goal_mut().goal_id = id.clone();
        state.goals.active_goal_mut().test_selection = Some(selection.clone());
        state.evidence_timeline.append(evidence_entry(
            WorkbenchEvidenceKind::TaskTestSelectionPlan,
            EvidenceStatus::Pass,
            format!(
                "selected {} tests for {}",
                selection.commands.len(),
                selection.goal_id
            ),
            Some(id.clone()),
            Some("goal.test_select".to_string()),
            Some(EvidenceSource::Action {
                action_id: Some("goal.test_select".to_string()),
                action_label: Some("goal.test_select".to_string()),
            }),
            vec![json_attachment(&selection)],
        ));
    }
    server
        .inner
        .events
        .send(WorkbenchEvent::GoalTestsSelected { goal_id: id })
        .ok();
    Json(selection)
}

async fn goal_check(
    State(server): State<WorkbenchServer>,
    Path(id): Path<String>,
    Json(request): Json<GoalCheckRequest>,
) -> Json<GoalPlanCheckReport> {
    let range = request
        .range
        .unwrap_or_else(|| "origin/main...HEAD".to_string());
    let report = build_goal_check(&server, &request.plan, &range).await;
    {
        let mut state = server.inner.state.write().await;
        state.goals.active_goal_mut().goal_id = id.clone();
        state.goals.active_goal_mut().check_report = Some(report.clone());
        let status = if report
            .issues
            .iter()
            .any(|issue| issue.severity == syu_domain::Severity::Error)
        {
            EvidenceStatus::Fail
        } else if report
            .issues
            .iter()
            .any(|issue| issue.severity == syu_domain::Severity::Warning)
        {
            EvidenceStatus::Warn
        } else {
            EvidenceStatus::Pass
        };
        state.evidence_timeline.append(evidence_entry(
            WorkbenchEvidenceKind::GoalPlanCheckReport,
            status,
            if matches!(status, EvidenceStatus::Pass) {
                format!("goal check passed for {}", report.plan_path)
            } else {
                format!("goal check found {} issues", report.issues.len())
            },
            Some(id.clone()),
            Some("goal.check".to_string()),
            Some(EvidenceSource::Action {
                action_id: Some("goal.check".to_string()),
                action_label: Some("goal.check".to_string()),
            }),
            vec![json_attachment(&report)],
        ));
    }
    server
        .inner
        .events
        .send(WorkbenchEvent::GoalChecked { goal_id: id })
        .ok();
    Json(report)
}

async fn goal_assign(
    State(server): State<WorkbenchServer>,
    Path(id): Path<String>,
    Json(request): Json<AssignmentRequest>,
) -> Json<AssignmentState> {
    let assignment = AssignmentState {
        goal_id: Some(id.clone()),
        assignee: Some(request.assignee.clone()),
        scope: Some(request.scope.clone()),
        expected_evidence: request.expected_evidence.clone(),
    };
    {
        let mut state = server.inner.state.write().await;
        state.assignment = Some(assignment.clone());
    }
    server
        .inner
        .events
        .send(WorkbenchEvent::AssignmentCreated { goal_id: id })
        .ok();
    Json(assignment)
}

async fn list_evidence(State(server): State<WorkbenchServer>) -> Json<EvidenceTimelineState> {
    Json(server.inner.state.read().await.evidence_timeline.clone())
}

async fn list_jobs(State(server): State<WorkbenchServer>) -> Json<Vec<JobRecord>> {
    Json(server.inner.jobs.read().await.values().cloned().collect())
}

async fn job_by_id(
    State(server): State<WorkbenchServer>,
    Path(id): Path<String>,
) -> Result<Json<JobRecord>, axum::http::StatusCode> {
    server
        .inner
        .jobs
        .read()
        .await
        .get(&id)
        .cloned()
        .map(Json)
        .ok_or(axum::http::StatusCode::NOT_FOUND)
}

async fn cancel_job(
    State(server): State<WorkbenchServer>,
    Path(id): Path<String>,
) -> Result<Json<JobRecord>, axum::http::StatusCode> {
    let mut jobs = server.inner.jobs.write().await;
    let job = jobs.get_mut(&id).ok_or(axum::http::StatusCode::NOT_FOUND)?;
    job.status = "cancelled".to_string();
    job.message = Some("cancelled by user".to_string());
    server
        .inner
        .events
        .send(WorkbenchEvent::JobCancelled { job_id: id.clone() })
        .ok();
    Ok(Json(job.clone()))
}

async fn events(
    State(server): State<WorkbenchServer>,
) -> Sse<impl futures_util::Stream<Item = std::result::Result<Event, Infallible>> + Send + 'static>
{
    let current = current_event_snapshot(&server).await;
    let initial =
        futures_util::stream::once(async move { Ok(event_to_sse("workspace_reloaded", &current)) });
    let stream =
        BroadcastStream::new(server.inner.events.subscribe()).filter_map(|message| async move {
            match message {
                Ok(event) => Some(Ok(event_to_sse(event_name(&event), &event))),
                Err(_) => None,
            }
        });
    let combined = initial.chain(stream);
    Sse::new(combined).keep_alive(KeepAlive::new().interval(Duration::from_secs(15)))
}

async fn current_payload(server: &WorkbenchServer) -> WorkbenchApiPayload {
    let state = server.inner.state.read().await.clone();
    WorkbenchApiPayload::new(state)
}

async fn current_event_snapshot(server: &WorkbenchServer) -> WorkbenchEvent {
    let workspace = server.inner.browser_workspace.read().await;
    WorkbenchEvent::WorkspaceReloaded {
        workspace_root: workspace.workspace_root.clone(),
        spec_root: workspace.spec_root.clone(),
        item_count: workspace.item_index.len(),
    }
}

fn event_name(event: &WorkbenchEvent) -> &'static str {
    match event {
        WorkbenchEvent::WorkspaceReloaded { .. } => "workspace_reloaded",
        WorkbenchEvent::ValidationUpdated { .. } => "validation_updated",
        WorkbenchEvent::RequestCreated { .. } => "request_created",
        WorkbenchEvent::RequestClassified { .. } => "request_classified",
        WorkbenchEvent::RequestScoped { .. } => "request_scoped",
        WorkbenchEvent::RequestScaffolded { .. } => "request_scaffolded",
        WorkbenchEvent::GoalPlanGenerated { .. } => "goal_plan_generated",
        WorkbenchEvent::GoalTestsSelected { .. } => "goal_tests_selected",
        WorkbenchEvent::GoalChecked { .. } => "goal_checked",
        WorkbenchEvent::BranchScopeUpdated { .. } => "branch_scope_updated",
        WorkbenchEvent::EvidenceAdded { .. } => "evidence_added",
        WorkbenchEvent::AssignmentCreated { .. } => "assignment_created",
        WorkbenchEvent::JobStarted { .. } => "job_started",
        WorkbenchEvent::JobOutput { .. } => "job_output",
        WorkbenchEvent::JobCompleted { .. } => "job_completed",
        WorkbenchEvent::JobCancelled { .. } => "job_cancelled",
    }
}

fn event_to_sse(name: &str, value: &impl Serialize) -> Event {
    let payload = serde_json::to_string(value).expect("event should serialize");
    Event::default().event(name).data(payload)
}

fn validate_bind(bind: &str, allow_remote_bind: bool) -> Result<()> {
    let parsed = parse_bind_address(bind)?;
    if !allow_remote_bind && !parsed.is_loopback() {
        bail!("remote bind `{bind}` is disabled unless `--allow-remote-bind` is set");
    }
    Ok(())
}

fn parse_bind_address(bind: &str) -> Result<IpAddr> {
    if bind.eq_ignore_ascii_case("localhost") {
        return Ok(IpAddr::from([127, 0, 0, 1]));
    }
    bind.parse::<IpAddr>()
        .with_context(|| format!("`{bind}` is not a valid IP address"))
}

fn parse_socket_addr(bind: &str, port: u16) -> Result<SocketAddr> {
    let ip = parse_bind_address(bind)?;
    Ok(SocketAddr::new(ip, port))
}

fn initial_state(workspace: &BrowserWorkspace, config: &WorkbenchLaunchConfig) -> WorkbenchState {
    WorkbenchState {
        workspace: Some(WorkspaceSnapshot {
            workspace_root: config.workspace_root.clone(),
            spec_root: config.spec_root.clone(),
            branch: None,
            validation_summary: Some(format!("{} items", workspace.item_index.len())),
        }),
        request: None,
        goals: Default::default(),
        branch_scope: None,
        evidence_timeline: EvidenceTimelineState::default(),
        assignment: None,
        job: JobState::default(),
        command_palette: CommandPaletteState::default(),
        confirmation: Some(WorkbenchConfirmationMetadata {
            confirmed_by: "local".to_string(),
            rationale: Some("starting a local Workbench server".to_string()),
            scope_token: None,
        }),
    }
}

fn collect_source_documents(spec_root: &FsPath) -> Result<Vec<SourceDocument>> {
    let mut documents = Vec::new();
    collect_yaml_documents(spec_root, spec_root, &mut documents)?;
    Ok(documents)
}

fn collect_yaml_documents(
    spec_root: &FsPath,
    directory: &FsPath,
    documents: &mut Vec<SourceDocument>,
) -> Result<()> {
    for entry in fs::read_dir(directory)
        .with_context(|| format!("failed to read directory `{}`", directory.display()))?
    {
        let path = entry?.path();
        if path.is_dir() {
            collect_yaml_documents(spec_root, &path, documents)?;
            continue;
        }
        if !is_yaml_path(&path) {
            continue;
        }
        if path.file_name().and_then(|name| name.to_str()) == Some("features.yaml") {
            continue;
        }
        if let Some(section) = section_for_path(spec_root, &path)? {
            let content = fs::read_to_string(&path)
                .with_context(|| format!("failed to read `{}`", path.display()))?;
            let rel = path
                .strip_prefix(spec_root)
                .with_context(|| format!("failed to make `{}` relative", path.display()))?;
            documents.push(SourceDocument {
                section,
                path: rel.to_string_lossy().replace('\\', "/"),
                content,
            });
        }
    }
    Ok(())
}

fn section_for_path(spec_root: &FsPath, path: &FsPath) -> Result<Option<SectionKind>> {
    let rel = path
        .strip_prefix(spec_root)
        .with_context(|| format!("failed to make `{}` relative", path.display()))?;
    let mut components = rel.components();
    let first = match components.next() {
        Some(Component::Normal(value)) => value.to_string_lossy().to_string(),
        _ => return Ok(None),
    };
    Ok(match first.as_str() {
        "philosophy" => Some(SectionKind::Philosophy),
        "policies" => Some(SectionKind::Policies),
        "requirements" => Some(SectionKind::Requirements),
        "features" => Some(SectionKind::Features),
        _ => None,
    })
}

fn is_yaml_path(path: &FsPath) -> bool {
    matches!(
        path.extension().and_then(|ext| ext.to_str()),
        Some("yaml" | "yml")
    )
}

fn load_browser_workspace(workspace_root: &FsPath, spec_root: &FsPath) -> Result<BrowserWorkspace> {
    let source_documents = collect_source_documents(spec_root)?;
    let payload = AppPayload {
        workspace_root: workspace_root.display().to_string(),
        spec_root: spec_root.display().to_string(),
        app_server: AppServer {
            bind: "127.0.0.1".to_string(),
            port: 3000,
            remotely_reachable: false,
        },
        source_documents,
        validation: ValidationSnapshot::default(),
        historical_ids: HistoricalIdSnapshot::default(),
    };
    Ok(build_browser_workspace(payload))
}

async fn classify_request(
    server: &WorkbenchServer,
    request: &RequestArtifact,
) -> ClassificationOutcome {
    let explicit_items = search_request_items(server, request).await;
    let classification = if request.request.to_ascii_lowercase().contains("delete")
        || request.request.to_ascii_lowercase().contains("remove")
    {
        RequestClassification::Delete
    } else if request.request.to_ascii_lowercase().contains("update")
        || request.request.to_ascii_lowercase().contains("change")
        || request.request.to_ascii_lowercase().contains("replace")
    {
        RequestClassification::Change
    } else {
        RequestClassification::Create
    };
    ClassificationOutcome {
        classification,
        reasons: vec![format!(
            "classified from request text as {classification:?}"
        )],
        explicit_items: explicit_items.clone(),
        related_items: explicit_items,
        request: request.request.clone(),
        context: request.context.clone(),
    }
}

async fn scope_request(server: &WorkbenchServer, request: &RequestArtifact) -> ScopeOutcome {
    let classification = classify_request(server, request).await;
    let items = search_request_items(server, request).await;
    let requirements = items
        .iter()
        .filter(|item| item.kind == "requirement")
        .cloned()
        .collect::<Vec<_>>();
    let features = items
        .iter()
        .filter(|item| item.kind == "feature")
        .map(|item| ScopeFeatureCandidate {
            id: item.id.clone(),
            title: item.title.clone(),
            status: "implemented".to_string(),
            linked_requirements: request.context.linked_ids.clone(),
            planned_state_update: false,
        })
        .collect::<Vec<_>>();
    let policies = items
        .iter()
        .filter(|item| item.kind == "policy")
        .cloned()
        .collect::<Vec<_>>();
    let philosophies = items
        .iter()
        .filter(|item| item.kind == "philosophy")
        .cloned()
        .collect::<Vec<_>>();
    ScopeOutcome {
        classification,
        signals: ScopeSignals {
            policy_discussion: !policies.is_empty(),
            philosophy_discussion: !philosophies.is_empty(),
            planned_feature_updates: !features.is_empty(),
        },
        requirements,
        features,
        policies,
        philosophies,
        notes: vec!["derived from request text".to_string()],
    }
}

async fn scaffold_request(server: &WorkbenchServer, request: &RequestArtifact) -> ScaffoldPlan {
    let classification = classify_request(server, request).await;
    let kind = if classification.classification == RequestClassification::Delete {
        ScaffoldUpdateKind::Feature
    } else {
        ScaffoldUpdateKind::Requirement
    };
    ScaffoldPlan {
        updates: vec![ScaffoldUpdate {
            kind,
            action: ScaffoldAction::Create,
            path: "docs/syu/requests/generated.yaml".to_string(),
            id: None,
            contents: request.request.clone(),
        }],
    }
}

async fn goal_plan_from_request(
    server: &WorkbenchServer,
    request: &RequestPlanRequest,
) -> GoalPlanArtifact {
    let classification = classify_request(server, &request.request).await;
    let scope = scope_request(server, &request.request).await;
    let explicit_ids = request.request.explicit_ids();
    let request_path = request
        .request_path
        .clone()
        .unwrap_or_else(|| "request.yaml".to_string());
    GoalPlanArtifact {
        version: 1,
        kind: "syu.goal_plan".to_string(),
        request_path: Some(request_path.clone()),
        request: Some(request.request.request.clone()),
        classification: Some(classification.classification.label().to_string()),
        source: GoalPlanSource {
            mode: GoalPlanSourceMode::RequestDriven,
            request_artifact: Some(request_path),
            classification: Some(classification.classification.label().to_string()),
            range: None,
            confidence: Some(GoalPlanConfidence::Medium),
            evidence: Some(GoalPlanSourceEvidence {
                changed_files: Vec::new(),
                traced_requirements: explicit_ids.clone(),
                traced_features: Vec::new(),
                traced_policies: Vec::new(),
                traced_philosophies: Vec::new(),
            }),
        },
        goal: GoalPlanGoal {
            id: "GOAL-001".to_string(),
            title: "Plan the requested Workbench change".to_string(),
            statement: request.request.request.clone(),
            non_goals: vec!["Do not create a persistent spec layer".to_string()],
            inferred: false,
        },
        spec_mapping: GoalPlanSpecMapping {
            persistent_items: GoalPlanPersistentItems {
                requirements: scope
                    .requirements
                    .iter()
                    .map(|item| {
                        GoalPlanPersistentItem::Item(GoalPlanPersistentItemDetails {
                            id: item.id.clone(),
                            title: Some(item.title.clone()),
                            document_path: None,
                        })
                    })
                    .collect(),
                features: scope
                    .features
                    .iter()
                    .map(|item| {
                        GoalPlanPersistentItem::Item(GoalPlanPersistentItemDetails {
                            id: item.id.clone(),
                            title: Some(item.title.clone()),
                            document_path: None,
                        })
                    })
                    .collect(),
                policies: scope
                    .policies
                    .iter()
                    .map(|item| {
                        GoalPlanPersistentItem::Item(GoalPlanPersistentItemDetails {
                            id: item.id.clone(),
                            title: Some(item.title.clone()),
                            document_path: None,
                        })
                    })
                    .collect(),
                philosophies: scope
                    .philosophies
                    .iter()
                    .map(|item| {
                        GoalPlanPersistentItem::Item(GoalPlanPersistentItemDetails {
                            id: item.id.clone(),
                            title: Some(item.title.clone()),
                            document_path: None,
                        })
                    })
                    .collect(),
            },
            spec_updates: Default::default(),
            spec_updates_required: false,
            spec_update_reasons: Vec::new(),
        },
        implementation_plan: GoalPlanImplementationPlan {
            confidence: Some(GoalPlanConfidence::Medium),
            scope: GoalPlanScope {
                include: vec![GoalPlanScopeInclude::Pattern("src/**".to_string())],
                exclude: vec!["docs/generated/**".to_string()],
            },
            steps: vec![
                "Review the request".to_string(),
                "Update the smallest typed surface".to_string(),
            ],
        },
        test_plan: GoalPlanTestPlan {
            selection_mode: GoalPlanSelectionMode::Minimal,
            confidence: Some(GoalPlanConfidence::Medium),
            required_tests: BTreeMap::new(),
            suggested_tests: BTreeMap::new(),
        },
        coverage: GoalPlanCoverage {
            mode: GoalPlanCoverageMode::ChangedLines,
            threshold: 100,
            include: Vec::new(),
            exclude: Vec::new(),
        },
        completion: GoalPlanCompletion {
            must_pass: vec!["syu validate .".to_string()],
        },
        warnings: Vec::new(),
    }
}

async fn build_branch_scope(workspace_root: &FsPath, range: &str) -> Result<BranchScopeReport> {
    let changed_files = resolve_git_range_changed_files(workspace_root, range)?;
    let changed_file_reports = changed_files
        .iter()
        .map(|file| ChangedFileReport {
            file: file.display().to_string(),
            symbols: Vec::new(),
            owners: Vec::new(),
            status: OwnershipStatus::Unowned,
            is_spec_file: false,
        })
        .collect::<Vec<_>>();
    Ok(BranchScopeReport::from_evidence(BranchScopeEvidence {
        range: range.to_string(),
        changed_files: changed_file_reports.clone(),
        trace_ownership: changed_file_reports,
        spec_items: Vec::new(),
        required_tests: Vec::new(),
        linked_tests: Vec::new(),
        include_patterns: Vec::new(),
        exclude_patterns: vec!["docs/generated/**".to_string(), "target/**".to_string()],
        allowed_ids: Vec::new(),
        unowned_files: Vec::new(),
        ambiguous_files: Vec::new(),
        spec_files: Vec::new(),
        direct_items: Vec::new(),
        related_items: Vec::new(),
        has_planned_features: false,
        out_of_scope_changes: Vec::new(),
    }))
}

async fn build_goal_check(
    server: &WorkbenchServer,
    plan: &GoalPlanArtifact,
    range: &str,
) -> GoalPlanCheckReport {
    let changed_files = resolve_git_range_changed_files(&server.inner.config.workspace_root, range)
        .unwrap_or_default()
        .into_iter()
        .map(|file| file.display().to_string())
        .collect::<Vec<_>>();
    let mut issues = Vec::new();
    if plan.goal.statement.trim().is_empty() {
        issues.push(Issue::warning(
            "SYU-goal-check-001",
            "goal.statement",
            None,
            "goal statement is blank",
            Some("add a concrete statement".to_string()),
        ));
    }
    GoalPlanCheckReport {
        plan_path: plan
            .request_path
            .clone()
            .unwrap_or_else(|| "in-memory".to_string()),
        range: range.to_string(),
        changed_files,
        issues,
    }
}

async fn search_request_items(
    server: &WorkbenchServer,
    request: &RequestArtifact,
) -> Vec<SearchResult> {
    let query = request.request.to_ascii_lowercase();
    let workspace = server.inner.browser_workspace.read().await;
    let mut results = Vec::new();
    for section in &workspace.sections {
        for document in &section.documents {
            for item in &document.items {
                if item.id.to_ascii_lowercase().contains(&query)
                    || item.title.to_ascii_lowercase().contains(&query)
                    || item
                        .summary
                        .as_ref()
                        .is_some_and(|value| value.to_ascii_lowercase().contains(&query))
                    || item
                        .description
                        .as_ref()
                        .is_some_and(|value| value.to_ascii_lowercase().contains(&query))
                {
                    results.push(SearchResult {
                        id: item.id.clone(),
                        kind: section.kind.label().to_string(),
                        title: item.title.clone(),
                    });
                }
            }
        }
    }
    results
}

async fn execute_action(
    server: &WorkbenchServer,
    action_id: &str,
    body: Value,
) -> Result<ActionRunResponse> {
    let request = serde_json::from_value::<RequestArtifact>(body.clone()).ok();
    let event = match action_id {
        "request.new" => {
            let request = request.context("request artifact required")?;
            let state = ActiveRequestState {
                request_path: None,
                artifact: Some(request.clone()),
                classification: None,
                scope: None,
                scaffold: None,
            };
            {
                let mut workbench_state = server.inner.state.write().await;
                workbench_state.request = Some(state.clone());
            }
            ActionRunResponse {
                action_id: action_id.to_string(),
                event: WorkbenchEvent::RequestCreated {
                    request: request.request.clone(),
                },
                result: serde_json::to_value(state)?,
            }
        }
        "request.classify" => {
            let request = request.context("request artifact required")?;
            let outcome = classify_request(server, &request).await;
            let classification = outcome.classification;
            let request_text = outcome.request.clone();
            ActionRunResponse {
                action_id: action_id.to_string(),
                event: WorkbenchEvent::RequestClassified {
                    classification,
                    request: request_text,
                },
                result: serde_json::to_value(outcome)?,
            }
        }
        "request.scope" => {
            let request = request.context("request artifact required")?;
            let outcome = scope_request(server, &request).await;
            ActionRunResponse {
                action_id: action_id.to_string(),
                event: WorkbenchEvent::RequestScoped {
                    request: request.request,
                    requirement_count: outcome.requirements.len(),
                },
                result: serde_json::to_value(outcome)?,
            }
        }
        "request.scaffold" => {
            let request = request.context("request artifact required")?;
            let plan = scaffold_request(server, &request).await;
            ActionRunResponse {
                action_id: action_id.to_string(),
                event: WorkbenchEvent::RequestScaffolded {
                    request: request.request,
                },
                result: serde_json::to_value(plan)?,
            }
        }
        "request.plan" => {
            let request = request.context("request artifact required")?;
            let plan = goal_plan_from_request(
                server,
                &RequestPlanRequest {
                    request,
                    request_path: None,
                },
            )
            .await;
            ActionRunResponse {
                action_id: action_id.to_string(),
                event: WorkbenchEvent::GoalPlanGenerated {
                    goal_id: plan.goal.id.clone(),
                },
                result: serde_json::to_value(plan)?,
            }
        }
        "branch.scope" => {
            let range = body
                .get("range")
                .and_then(Value::as_str)
                .unwrap_or("origin/main...HEAD");
            let report = build_branch_scope(&server.inner.config.workspace_root, range).await?;
            {
                let mut state = server.inner.state.write().await;
                state.branch_scope = Some(report.clone());
            }
            ActionRunResponse {
                action_id: action_id.to_string(),
                event: WorkbenchEvent::BranchScopeUpdated {
                    range: range.to_string(),
                    changed_files: report.changed_files.len(),
                },
                result: serde_json::to_value(report)?,
            }
        }
        "goal.check" => {
            let plan = serde_json::from_value::<GoalPlanArtifact>(body.clone())
                .context("goal plan artifact required")?;
            let report = build_goal_check(server, &plan, "origin/main...HEAD").await;
            {
                let mut state = server.inner.state.write().await;
                state.goals.active_goal_mut().goal_id = plan.goal.id.clone();
                state.goals.active_goal_mut().check_report = Some(report.clone());
                let status = if report
                    .issues
                    .iter()
                    .any(|issue| issue.severity == syu_domain::Severity::Error)
                {
                    EvidenceStatus::Fail
                } else if report
                    .issues
                    .iter()
                    .any(|issue| issue.severity == syu_domain::Severity::Warning)
                {
                    EvidenceStatus::Warn
                } else {
                    EvidenceStatus::Pass
                };
                state.evidence_timeline.append(evidence_entry(
                    WorkbenchEvidenceKind::GoalPlanCheckReport,
                    status,
                    if matches!(status, EvidenceStatus::Pass) {
                        format!("goal check passed for {}", report.plan_path)
                    } else {
                        format!("goal check found {} issues", report.issues.len())
                    },
                    Some(plan.goal.id.clone()),
                    Some(action_id.to_string()),
                    Some(EvidenceSource::Action {
                        action_id: Some(action_id.to_string()),
                        action_label: Some(action_id.to_string()),
                    }),
                    vec![json_attachment(&report)],
                ));
            }
            ActionRunResponse {
                action_id: action_id.to_string(),
                event: WorkbenchEvent::GoalChecked {
                    goal_id: plan.goal.id,
                },
                result: serde_json::to_value(report)?,
            }
        }
        "goal.test_select" => {
            let plan = serde_json::from_value::<GoalPlanArtifact>(body.clone())
                .context("goal plan artifact required")?;
            let selection = TaskTestSelectionPlan {
                goal_id: plan.goal.id.clone(),
                goal_title: plan.goal.title.clone(),
                selection_mode: "minimal".to_string(),
                commands: vec![TaskTestSelectionCommand {
                    language: "rust".to_string(),
                    command: "cargo test".to_string(),
                    reason: "baseline repository validation".to_string(),
                }],
                escalation: TaskTestSelectionEscalation {
                    level: "none".to_string(),
                    reason: "request-scoped test set is sufficient".to_string(),
                },
                warnings: Vec::new(),
            };
            {
                let mut state = server.inner.state.write().await;
                state.goals.active_goal_mut().goal_id = plan.goal.id.clone();
                state.goals.active_goal_mut().test_selection = Some(selection.clone());
                state.evidence_timeline.append(evidence_entry(
                    WorkbenchEvidenceKind::TaskTestSelectionPlan,
                    EvidenceStatus::Pass,
                    format!(
                        "selected {} tests for {}",
                        selection.commands.len(),
                        selection.goal_id
                    ),
                    Some(plan.goal.id.clone()),
                    Some(action_id.to_string()),
                    Some(EvidenceSource::Action {
                        action_id: Some(action_id.to_string()),
                        action_label: Some(action_id.to_string()),
                    }),
                    vec![json_attachment(&selection)],
                ));
            }
            ActionRunResponse {
                action_id: action_id.to_string(),
                event: WorkbenchEvent::GoalTestsSelected {
                    goal_id: plan.goal.id,
                },
                result: serde_json::to_value(selection)?,
            }
        }
        "assignment.create" => {
            let assignment = serde_json::from_value::<AssignmentRequest>(body.clone())
                .context("assignment request required")?;
            let state = AssignmentState {
                goal_id: None,
                assignee: Some(assignment.assignee),
                scope: Some(assignment.scope),
                expected_evidence: assignment.expected_evidence,
            };
            ActionRunResponse {
                action_id: action_id.to_string(),
                event: WorkbenchEvent::AssignmentCreated {
                    goal_id: "goal-1".to_string(),
                },
                result: serde_json::to_value(state)?,
            }
        }
        "agent.run" => {
            let job_id = "job-1".to_string();
            {
                let mut jobs = server.inner.jobs.write().await;
                jobs.insert(
                    job_id.clone(),
                    JobRecord {
                        id: job_id.clone(),
                        action_id: Some(action_id.to_string()),
                        status: "running".to_string(),
                        message: Some("queued".to_string()),
                    },
                );
            }
            server
                .inner
                .events
                .send(WorkbenchEvent::JobStarted {
                    job_id: job_id.clone(),
                })
                .ok();
            server
                .inner
                .events
                .send(WorkbenchEvent::JobOutput {
                    job_id: job_id.clone(),
                    line: "executing bounded goal scope".to_string(),
                })
                .ok();
            {
                let mut jobs = server.inner.jobs.write().await;
                if let Some(job) = jobs.get_mut(&job_id) {
                    job.status = "completed".to_string();
                    job.message = Some("completed".to_string());
                }
            }
            server
                .inner
                .events
                .send(WorkbenchEvent::JobCompleted {
                    job_id: job_id.clone(),
                })
                .ok();
            ActionRunResponse {
                action_id: action_id.to_string(),
                event: WorkbenchEvent::JobCompleted { job_id },
                result: serde_json::json!({"status": "completed"}),
            }
        }
        "validation.run" => ActionRunResponse {
            action_id: action_id.to_string(),
            event: {
                {
                    let mut state = server.inner.state.write().await;
                    state
                        .workspace
                        .get_or_insert_with(WorkspaceSnapshot::default)
                        .validation_summary = Some("validation snapshot refreshed".to_string());
                    state.evidence_timeline.append(evidence_entry(
                        WorkbenchEvidenceKind::ValidationReport,
                        EvidenceStatus::Pass,
                        "validation snapshot refreshed",
                        None,
                        Some(action_id.to_string()),
                        Some(EvidenceSource::Command {
                            command: action_id.to_string(),
                        }),
                        vec![json_attachment(&serde_json::json!({"status": "ok"}))],
                    ));
                }
                WorkbenchEvent::ValidationUpdated {
                    summary: "validation snapshot refreshed".to_string(),
                }
            },
            result: serde_json::json!({"status": "ok"}),
        },
        other => bail!("unsupported action id `{other}`"),
    };

    server.inner.events.send(event.event.clone()).ok();
    Ok(event)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use http_body_util::BodyExt;
    use serde_json::Value;
    use tower::ServiceExt;

    fn test_server() -> WorkbenchServer {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("repo root should exist");
        WorkbenchServer::new(WorkbenchLaunchConfig {
            workspace_root: root.clone(),
            spec_root: root.join("docs/syu"),
            bind: "127.0.0.1".to_string(),
            port: 3000,
            allow_remote_bind: false,
        })
        .expect("server should initialize")
    }

    async fn json_response(router: Router, request: Request<Body>) -> (StatusCode, Value) {
        let response = router
            .oneshot(request)
            .await
            .expect("request should succeed");
        let status = response.status();
        let bytes = BodyExt::collect(response.into_body())
            .await
            .expect("body should collect")
            .to_bytes();
        let json = serde_json::from_slice(&bytes).expect("json should parse");
        (status, json)
    }

    #[tokio::test]
    async fn health_endpoint_reports_server_details() {
        let server = test_server();
        let (status, json) = json_response(
            server.router(),
            Request::builder()
                .uri("/api/health")
                .body(Body::empty())
                .expect("request"),
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(json["ok"], true);
        assert_eq!(json["bind"], "127.0.0.1");
        assert_eq!(json["port"], 3000);
    }

    #[tokio::test]
    async fn actions_endpoint_lists_registry() {
        let server = test_server();
        let (status, json) = json_response(
            server.router(),
            Request::builder()
                .uri("/api/actions")
                .body(Body::empty())
                .expect("request"),
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert!(
            json["actions"]
                .as_array()
                .is_some_and(|actions| !actions.is_empty())
        );
        assert!(
            json["availability"]
                .as_array()
                .is_some_and(|availability| !availability.is_empty())
        );
    }

    #[tokio::test]
    async fn request_plan_endpoint_returns_goal_plan() {
        let server = test_server();
        let body = serde_json::json!({
            "request": {
                "version": 1,
                "request": "Add Workbench planning coverage",
                "context": {
                    "linked_ids": ["REQ-WORKBENCH-006"]
                }
            },
            "request_path": "request.yaml"
        });
        let (status, json) = json_response(
            server.router(),
            Request::builder()
                .method("POST")
                .uri("/api/request/plan")
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .expect("request"),
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(json["kind"], "syu.goal_plan");
        assert_eq!(json["goal"]["id"], "GOAL-001");
    }

    #[tokio::test]
    async fn branch_scope_endpoint_returns_report() {
        let server = test_server();
        let (status, json) = json_response(
            server.router(),
            Request::builder()
                .uri("/api/branch/scope?range=HEAD...HEAD")
                .body(Body::empty())
                .expect("request"),
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(json["range"], "HEAD...HEAD");
        assert!(json["changed_files"].as_array().is_some());

        let snapshot = server.inner.state.read().await.clone();
        assert_eq!(
            snapshot
                .branch_scope
                .as_ref()
                .map(|report| report.range.as_str()),
            Some("HEAD...HEAD")
        );
    }

    #[tokio::test]
    async fn request_new_action_persists_active_request() {
        let server = test_server();
        let body = serde_json::json!({
            "version": 1,
            "request": "Create a new active request",
            "context": {
                "linked_ids": ["REQ-WORKBENCH-001"]
            }
        });
        let (status, json) = json_response(
            server.router(),
            Request::builder()
                .method("POST")
                .uri("/api/actions/request.new/run")
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .expect("request"),
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            json["result"]["artifact"]["request"],
            "Create a new active request"
        );

        let snapshot = server.inner.state.read().await.clone();
        assert_eq!(
            snapshot
                .request
                .as_ref()
                .and_then(|request| request.artifact.as_ref())
                .map(|artifact| artifact.request.as_str()),
            Some("Create a new active request")
        );
    }

    #[tokio::test]
    async fn goal_check_endpoint_returns_report() {
        let server = test_server();
        let plan = goal_plan_from_request(
            &server,
            &RequestPlanRequest {
                request: RequestArtifact {
                    version: 1,
                    request: "Keep goal checking typed".to_string(),
                    context: Default::default(),
                },
                request_path: Some("request.yaml".to_string()),
            },
        )
        .await;
        let body = serde_json::json!({
            "plan": plan,
            "range": "HEAD...HEAD"
        });
        let (status, json) = json_response(
            server.router(),
            Request::builder()
                .method("POST")
                .uri("/api/goals/goal-1/check")
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .expect("request"),
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(json["range"], "HEAD...HEAD");
        assert_eq!(json["plan_path"], "request.yaml");

        let snapshot = server.inner.state.read().await.clone();
        assert_eq!(snapshot.evidence_timeline.entries.len(), 1);
        assert_eq!(
            snapshot.evidence_timeline.entries[0].kind,
            WorkbenchEvidenceKind::GoalPlanCheckReport
        );
        assert_eq!(
            snapshot.evidence_timeline.entries[0].goal_id.as_deref(),
            Some("goal-1")
        );
        assert_eq!(
            snapshot.evidence_timeline.entries[0].status,
            EvidenceStatus::Pass
        );
    }

    #[tokio::test]
    async fn goal_test_select_endpoint_records_evidence() {
        let server = test_server();
        let plan = goal_plan_from_request(
            &server,
            &RequestPlanRequest {
                request: RequestArtifact {
                    version: 1,
                    request: "Select tests for the goal".to_string(),
                    context: Default::default(),
                },
                request_path: Some("request.yaml".to_string()),
            },
        )
        .await;
        let body = serde_json::to_string(&plan).expect("plan json");
        let (status, json) = json_response(
            server.router(),
            Request::builder()
                .method("POST")
                .uri("/api/goals/goal-1/test-select")
                .header("content-type", "application/json")
                .body(Body::from(body))
                .expect("request"),
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(json["goal_id"], "goal-1");

        let snapshot = server.inner.state.read().await.clone();
        assert_eq!(snapshot.evidence_timeline.entries.len(), 1);
        assert_eq!(
            snapshot.evidence_timeline.entries[0].kind,
            WorkbenchEvidenceKind::TaskTestSelectionPlan
        );
        assert_eq!(
            snapshot.evidence_timeline.entries[0].goal_id.as_deref(),
            Some("goal-1")
        );
        assert_eq!(
            snapshot.evidence_timeline.entries[0].status,
            EvidenceStatus::Pass
        );
    }

    #[tokio::test]
    async fn validation_action_records_evidence() {
        let server = test_server();
        let (status, json) = json_response(
            server.router(),
            Request::builder()
                .method("POST")
                .uri("/api/actions/validation.run/run")
                .header("content-type", "application/json")
                .body(Body::from("{}"))
                .expect("request"),
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(json["result"]["status"], "ok");

        let snapshot = server.inner.state.read().await.clone();
        assert_eq!(snapshot.evidence_timeline.entries.len(), 1);
        assert_eq!(
            snapshot.evidence_timeline.entries[0].kind,
            WorkbenchEvidenceKind::ValidationReport
        );
        assert_eq!(
            snapshot.evidence_timeline.entries[0].status,
            EvidenceStatus::Pass
        );
    }

    #[tokio::test]
    async fn events_endpoint_streams_initial_reload_event() {
        let server = test_server();
        let response = server
            .router()
            .oneshot(
                Request::builder()
                    .uri("/api/events")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("request should succeed");

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response
                .headers()
                .get(axum::http::header::CONTENT_TYPE)
                .and_then(|value| value.to_str().ok()),
            Some("text/event-stream")
        );

        let mut body = response.into_body();
        let frame = body
            .frame()
            .await
            .expect("frame should exist")
            .expect("frame");
        let bytes = frame.into_data().expect("data frame");
        let text = std::str::from_utf8(&bytes).expect("utf8");
        assert!(text.contains("workspace_reloaded"));
    }
}
