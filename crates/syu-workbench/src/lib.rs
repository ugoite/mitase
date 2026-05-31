use serde::{Deserialize, Serialize};
use std::{
    path::PathBuf,
    sync::OnceLock,
    time::{SystemTime, UNIX_EPOCH},
};

pub use syu_actions::{HistoryResponse, ValidationReport};
pub use syu_code_intel::{
    AffectedSpecItem, BranchScopeEvidence, BranchScopeReport, ChangedFileReport, OutOfScopeChange,
    OwnershipStatus, SpecImpactGraphEdge, SpecImpactGraphNode, SpecImpactGraphReport,
    SpecImpactReport, SuggestedGoalSplit,
};
pub use syu_task_model::{
    ClassificationOutcome, GoalPlanArtifact, GoalPlanCheckReport, RequestArtifact,
    RequestArtifactContext, ScaffoldPlan, ScopeOutcome, TaskTestSelectionPlan,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkbenchActionRisk {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkbenchActionMutability {
    ReadOnly,
    MutatesState,
    MutatesFiles,
    MutatesStateAndFiles,
}

impl WorkbenchActionMutability {
    pub const fn mutates_files(self) -> bool {
        matches!(self, Self::MutatesFiles | Self::MutatesStateAndFiles)
    }

    pub const fn mutates_state(self) -> bool {
        matches!(self, Self::MutatesState | Self::MutatesStateAndFiles)
    }

    pub const fn requires_confirmation(self) -> bool {
        !matches!(self, Self::ReadOnly)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkbenchActionId {
    #[serde(rename = "request.new")]
    RequestNew,
    #[serde(rename = "request.classify")]
    RequestClassify,
    #[serde(rename = "request.scope")]
    RequestScope,
    #[serde(rename = "request.scaffold")]
    RequestScaffold,
    #[serde(rename = "request.plan")]
    RequestPlan,
    #[serde(rename = "goal.test_select")]
    GoalTestSelect,
    #[serde(rename = "goal.check")]
    GoalCheck,
    #[serde(rename = "branch.scope")]
    BranchScope,
    #[serde(rename = "branch.infer_goal")]
    BranchInferGoal,
    #[serde(rename = "spec.impact")]
    SpecImpact,
    #[serde(rename = "trace.range")]
    TraceRange,
    #[serde(rename = "relate.range")]
    RelateRange,
    #[serde(rename = "validation.run")]
    ValidationRun,
    #[serde(rename = "history.show")]
    HistoryShow,
    #[serde(rename = "assignment.create")]
    AssignmentCreate,
    #[serde(rename = "agent.run")]
    AgentRun,
}

impl WorkbenchActionId {
    pub const fn label(self) -> &'static str {
        match self {
            Self::RequestNew => "request.new",
            Self::RequestClassify => "request.classify",
            Self::RequestScope => "request.scope",
            Self::RequestScaffold => "request.scaffold",
            Self::RequestPlan => "request.plan",
            Self::GoalTestSelect => "goal.test_select",
            Self::GoalCheck => "goal.check",
            Self::BranchScope => "branch.scope",
            Self::BranchInferGoal => "branch.infer_goal",
            Self::SpecImpact => "spec.impact",
            Self::TraceRange => "trace.range",
            Self::RelateRange => "relate.range",
            Self::ValidationRun => "validation.run",
            Self::HistoryShow => "history.show",
            Self::AssignmentCreate => "assignment.create",
            Self::AgentRun => "agent.run",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkbenchActionFunction {
    ScaffoldRequest,
    ClassifyRequest,
    ScopeRequest,
    GenerateGoalPlan,
    SelectGoalTests,
    CheckGoalPlan,
    BranchScope,
    TraceRange,
    InferGoalPlanFromDiff,
    RelateRange,
    ValidateWorkspace,
    HistoryForItem,
    AssignmentCreate,
    AgentRun,
}

impl WorkbenchActionFunction {
    pub const fn label(self) -> &'static str {
        match self {
            Self::ScaffoldRequest => "scaffold_request",
            Self::ClassifyRequest => "classify_request",
            Self::ScopeRequest => "scope_request",
            Self::GenerateGoalPlan => "generate_goal_plan",
            Self::SelectGoalTests => "select_goal_tests",
            Self::CheckGoalPlan => "check_goal_plan",
            Self::BranchScope => "branch.scope",
            Self::TraceRange => "trace_range",
            Self::InferGoalPlanFromDiff => "infer_goal_plan_from_diff",
            Self::RelateRange => "relate_range",
            Self::ValidateWorkspace => "validate_workspace",
            Self::HistoryForItem => "history_for_item",
            Self::AssignmentCreate => "assignment.create",
            Self::AgentRun => "agent.run",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkbenchActionOutputEvent {
    RequestCreated,
    RequestClassified,
    RequestScoped,
    RequestScaffolded,
    GoalPlanGenerated,
    GoalTestsSelected,
    GoalChecked,
    BranchScoped,
    GoalInferred,
    SpecImpactAssessed,
    ValidationRun,
    HistoryShown,
    AssignmentCreated,
    AgentRunQueued,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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
    SpecImpactReport,
    ValidationReport,
    HistoryResponse,
    AssignmentState,
    JobState,
}

impl WorkbenchEvidenceKind {
    pub const fn label(self) -> &'static str {
        match self {
            Self::RequestArtifact => "request_artifact",
            Self::ClassificationOutcome => "classification_outcome",
            Self::ScopeOutcome => "scope_outcome",
            Self::ScaffoldPlan => "scaffold_plan",
            Self::GoalPlanArtifact => "goal_plan_artifact",
            Self::TaskTestSelectionPlan => "task_test_selection_plan",
            Self::GoalPlanCheckReport => "goal_plan_check_report",
            Self::BranchScopeReport => "branch_scope_report",
            Self::SpecImpactReport => "spec_impact_report",
            Self::ValidationReport => "validation_report",
            Self::HistoryResponse => "history_response",
            Self::AssignmentState => "assignment_state",
            Self::JobState => "job_state",
        }
    }
}

pub type EvidenceKind = WorkbenchEvidenceKind;
pub type EvidenceSummary = String;
pub type EvidenceTimeline = EvidenceTimelineState;

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

impl EvidenceStatus {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Pass => "pass",
            Self::Warn => "warn",
            Self::Fail => "fail",
            Self::Skipped => "skipped",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceSeverity {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceSubject {
    Workspace,
    Request,
    Goal,
    Branch,
    SpecItem,
    File,
    Test,
    Assignment,
    AgentRun,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct EvidenceCommand {
    pub command: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<String>,
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
pub enum EvidenceSource {
    Action {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        action_id: Option<WorkbenchActionId>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        action_label: Option<String>,
    },
    Command {
        command: String,
    },
    Manual {
        actor: String,
    },
    System {
        component: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceRecord {
    pub kind: EvidenceKind,
    pub status: EvidenceStatus,
    pub summary: EvidenceSummary,
    pub timestamp: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub goal_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subject: Option<EvidenceSubject>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub severity: Option<EvidenceSeverity>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<EvidenceSource>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action_id: Option<WorkbenchActionId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<EvidenceCommand>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attachments: Vec<EvidenceAttachment>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub related_spec_id: Option<String>,
}

pub type EvidenceEntry = EvidenceRecord;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Default)]
pub struct EvidenceTimelineFilter {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub goal_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<EvidenceKind>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<EvidenceStatus>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_action_id: Option<WorkbenchActionId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub related_spec_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub since_timestamp: Option<u64>,
}

impl EvidenceRecord {
    pub fn new(
        kind: EvidenceKind,
        status: EvidenceStatus,
        summary: impl Into<String>,
        source: Option<EvidenceSource>,
    ) -> Self {
        Self {
            kind,
            status,
            summary: summary.into(),
            timestamp: evidence_timestamp(),
            goal_id: None,
            subject: None,
            severity: None,
            source,
            action_id: None,
            command: None,
            attachments: Vec::new(),
            related_spec_id: None,
        }
    }

    pub fn with_goal_id(mut self, goal_id: Option<String>) -> Self {
        self.goal_id = goal_id;
        self
    }

    pub fn with_subject(mut self, subject: EvidenceSubject) -> Self {
        self.subject = Some(subject);
        self
    }

    pub fn with_severity(mut self, severity: EvidenceSeverity) -> Self {
        self.severity = Some(severity);
        self
    }

    pub fn with_action_id(mut self, action_id: WorkbenchActionId) -> Self {
        self.action_id = Some(action_id);
        self
    }

    pub fn with_command(mut self, command: EvidenceCommand) -> Self {
        self.command = Some(command);
        self
    }

    pub fn with_related_spec_id(mut self, related_spec_id: Option<String>) -> Self {
        self.related_spec_id = related_spec_id;
        self
    }

    pub fn with_attachment(mut self, attachment: EvidenceAttachment) -> Self {
        self.attachments.push(attachment);
        self
    }
}

impl EvidenceTimelineState {
    pub fn append(&mut self, record: EvidenceRecord) {
        self.entries.push(record);
    }

    pub fn append_action_result(
        &mut self,
        action_id: WorkbenchActionId,
        goal_id: Option<String>,
        result: &WorkbenchActionResult,
    ) {
        self.append(evidence_record_for_action(action_id, goal_id, result));
    }
}

fn evidence_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or_default()
}

fn is_false(value: &bool) -> bool {
    !*value
}

fn truncated_json<T: Serialize>(value: &T) -> EvidenceAttachment {
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

fn evidence_record_for_action(
    action_id: WorkbenchActionId,
    goal_id: Option<String>,
    result: &WorkbenchActionResult,
) -> EvidenceRecord {
    let kind = result.evidence_kind();
    let (status, severity, subject, summary) = match result {
        WorkbenchActionResult::RequestArtifact(_) => (
            EvidenceStatus::Pass,
            Some(EvidenceSeverity::Low),
            Some(EvidenceSubject::Request),
            "request artifact captured".to_string(),
        ),
        WorkbenchActionResult::ClassificationOutcome(outcome) => (
            EvidenceStatus::Pass,
            Some(EvidenceSeverity::Low),
            Some(EvidenceSubject::Request),
            format!("request classified as {}", outcome.classification.label()),
        ),
        WorkbenchActionResult::ScopeOutcome(outcome) => (
            EvidenceStatus::Pass,
            Some(EvidenceSeverity::Low),
            Some(EvidenceSubject::Request),
            format!(
                "request scope identified {} requirements",
                outcome.requirements.len()
            ),
        ),
        WorkbenchActionResult::ScaffoldPlan(plan) => (
            EvidenceStatus::Pass,
            Some(EvidenceSeverity::Low),
            Some(EvidenceSubject::Request),
            format!("scaffold preview includes {} updates", plan.updates.len()),
        ),
        WorkbenchActionResult::GoalPlanArtifact(plan) => (
            EvidenceStatus::Pass,
            Some(EvidenceSeverity::Low),
            Some(EvidenceSubject::Goal),
            format!("goal plan generated for {}", plan.goal.id),
        ),
        WorkbenchActionResult::TaskTestSelectionPlan(plan) => (
            if plan.commands.is_empty() {
                EvidenceStatus::Pending
            } else {
                EvidenceStatus::Pass
            },
            Some(EvidenceSeverity::Low),
            Some(EvidenceSubject::Goal),
            format!(
                "selected {} tests for {}",
                plan.commands.len(),
                plan.goal_id
            ),
        ),
        WorkbenchActionResult::GoalPlanCheckReport(report) => (
            if report.error_count() > 0 {
                EvidenceStatus::Fail
            } else if report.warning_count() > 0 {
                EvidenceStatus::Warn
            } else {
                EvidenceStatus::Pass
            },
            Some(if report.error_count() > 0 {
                EvidenceSeverity::High
            } else if report.warning_count() > 0 {
                EvidenceSeverity::Medium
            } else {
                EvidenceSeverity::Low
            }),
            Some(EvidenceSubject::Goal),
            if report.passed() {
                format!("goal check passed for {}", report.plan_path)
            } else {
                format!("goal check found {} issues", report.issues.len())
            },
        ),
        WorkbenchActionResult::BranchScopeReport(report) => (
            EvidenceStatus::Pass,
            Some(EvidenceSeverity::Low),
            Some(EvidenceSubject::Branch),
            format!(
                "branch scope summarized {} files",
                report.changed_files.len()
            ),
        ),
        WorkbenchActionResult::SpecImpactReport(report) => (
            EvidenceStatus::Pass,
            Some(EvidenceSeverity::Low),
            Some(EvidenceSubject::SpecItem),
            format!(
                "spec impact mapped {} affected items",
                report.affected_items.len()
            ),
        ),
        WorkbenchActionResult::ValidationReport(report) => (
            if report.is_success() {
                EvidenceStatus::Pass
            } else {
                EvidenceStatus::Fail
            },
            Some(if report.is_success() {
                if report.issues.is_empty() {
                    EvidenceSeverity::Low
                } else {
                    EvidenceSeverity::Medium
                }
            } else {
                EvidenceSeverity::High
            }),
            Some(EvidenceSubject::Workspace),
            if report.is_success() {
                "workspace validation passed".to_string()
            } else {
                format!("workspace validation found {} issues", report.issues.len())
            },
        ),
        WorkbenchActionResult::HistoryResponse(history) => (
            EvidenceStatus::Pass,
            Some(EvidenceSeverity::Low),
            Some(EvidenceSubject::Workspace),
            format!("history loaded for {} {}", history.entity_kind, history.id),
        ),
        WorkbenchActionResult::AssignmentState(assignment) => (
            EvidenceStatus::Pending,
            Some(EvidenceSeverity::Low),
            Some(EvidenceSubject::Assignment),
            match &assignment.goal_id {
                Some(goal_id) => format!("assignment created for {goal_id}"),
                None => "assignment created".to_string(),
            },
        ),
        WorkbenchActionResult::JobState(job) => (
            match job.status {
                JobStatus::Idle | JobStatus::Queued | JobStatus::Running => EvidenceStatus::Pending,
                JobStatus::Completed => EvidenceStatus::Pass,
                JobStatus::Failed => EvidenceStatus::Fail,
            },
            Some(EvidenceSeverity::Low),
            Some(EvidenceSubject::AgentRun),
            job.message
                .clone()
                .unwrap_or_else(|| format!("job {}", job.status.label())),
        ),
    };

    let source = Some(EvidenceSource::Action {
        action_id: Some(action_id),
        action_label: Some(action_id.label().to_string()),
    });
    let command = match result {
        WorkbenchActionResult::ValidationReport(_) => Some(EvidenceCommand {
            command: "validation.run".to_string(),
            args: Vec::new(),
        }),
        WorkbenchActionResult::GoalPlanCheckReport(_) => Some(EvidenceCommand {
            command: "goal.check".to_string(),
            args: Vec::new(),
        }),
        WorkbenchActionResult::TaskTestSelectionPlan(_) => Some(EvidenceCommand {
            command: "goal.test_select".to_string(),
            args: Vec::new(),
        }),
        _ => None,
    };
    let attachment = truncated_json(result);

    EvidenceRecord::new(kind, status, summary, source)
        .with_goal_id(goal_id)
        .with_subject(subject.expect("subject is always set"))
        .with_severity(severity.expect("severity is always set"))
        .with_action_id(action_id)
        .with_related_spec_id(None)
        .with_attachment(attachment)
        .with_command(command.unwrap_or(EvidenceCommand {
            command: action_id.label().to_string(),
            args: Vec::new(),
        }))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkbenchStateRequirement {
    WorkspaceLoaded,
    ActiveRequest,
    ActiveGoalPlan,
    BranchScopeLoaded,
    AssignmentLoaded,
    ConfirmationMetadata,
    BoundedScope,
}

impl WorkbenchStateRequirement {
    pub const fn label(self) -> &'static str {
        match self {
            Self::WorkspaceLoaded => "workspace_loaded",
            Self::ActiveRequest => "active_request",
            Self::ActiveGoalPlan => "active_goal_plan",
            Self::BranchScopeLoaded => "branch_scope_loaded",
            Self::AssignmentLoaded => "assignment_loaded",
            Self::ConfirmationMetadata => "confirmation_metadata",
            Self::BoundedScope => "bounded_scope",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkbenchActionInputSchema {
    None,
    RequestDraft,
    RequestArtifact,
    GoalPlanArtifact,
    BranchScope,
    Selector,
    HistoryQuery,
    Assignment,
    AgentRun,
    ValidationRequest,
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkbenchActionInput {
    None,
    RequestDraft {
        request: String,
        #[serde(default)]
        context: RequestArtifactContext,
    },
    RequestArtifact(RequestArtifact),
    GoalPlanArtifact(GoalPlanArtifact),
    BranchScope {
        range: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        allowed_ids: Vec<String>,
    },
    Selector {
        selector: String,
    },
    HistoryQuery {
        item: String,
    },
    Assignment {
        assignee: AssignmentAssignee,
        scope: BoundedScope,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        expected_evidence: Vec<WorkbenchEvidenceKind>,
    },
    AgentRun {
        goal_id: String,
        scope: BoundedScope,
    },
    ValidationRequest {
        workspace_root: PathBuf,
    },
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkbenchActionResult {
    RequestArtifact(RequestArtifact),
    ClassificationOutcome(ClassificationOutcome),
    ScopeOutcome(ScopeOutcome),
    ScaffoldPlan(ScaffoldPlan),
    GoalPlanArtifact(GoalPlanArtifact),
    TaskTestSelectionPlan(TaskTestSelectionPlan),
    GoalPlanCheckReport(GoalPlanCheckReport),
    BranchScopeReport(BranchScopeReport),
    SpecImpactReport(SpecImpactReport),
    ValidationReport(ValidationReport),
    HistoryResponse(HistoryResponse),
    AssignmentState(AssignmentState),
    JobState(JobState),
}

impl WorkbenchActionResult {
    pub fn evidence_kind(&self) -> WorkbenchEvidenceKind {
        match self {
            Self::RequestArtifact(_) => WorkbenchEvidenceKind::RequestArtifact,
            Self::ClassificationOutcome(_) => WorkbenchEvidenceKind::ClassificationOutcome,
            Self::ScopeOutcome(_) => WorkbenchEvidenceKind::ScopeOutcome,
            Self::ScaffoldPlan(_) => WorkbenchEvidenceKind::ScaffoldPlan,
            Self::GoalPlanArtifact(_) => WorkbenchEvidenceKind::GoalPlanArtifact,
            Self::TaskTestSelectionPlan(_) => WorkbenchEvidenceKind::TaskTestSelectionPlan,
            Self::GoalPlanCheckReport(_) => WorkbenchEvidenceKind::GoalPlanCheckReport,
            Self::BranchScopeReport(_) => WorkbenchEvidenceKind::BranchScopeReport,
            Self::SpecImpactReport(_) => WorkbenchEvidenceKind::SpecImpactReport,
            Self::ValidationReport(_) => WorkbenchEvidenceKind::ValidationReport,
            Self::HistoryResponse(_) => WorkbenchEvidenceKind::HistoryResponse,
            Self::AssignmentState(_) => WorkbenchEvidenceKind::AssignmentState,
            Self::JobState(_) => WorkbenchEvidenceKind::JobState,
        }
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Default)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AssignmentAssignee {
    Human { name: String },
    Ai { model: String },
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    Idle,
    Queued,
    Running,
    Completed,
    Failed,
}

impl JobStatus {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Queued => "queued",
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Failed => "failed",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct JobState {
    pub status: JobStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action_id: Option<WorkbenchActionId>,
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

#[derive(Debug, Clone, PartialEq, Serialize, Default)]
pub struct EvidenceTimelineState {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub entries: Vec<EvidenceEntry>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Default)]
pub struct BranchScopeState {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub range: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub report: Option<BranchScopeReport>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bounded_scope: Option<BoundedScope>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub allowed_ids: Vec<String>,
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
    pub selected_action_id: Option<WorkbenchActionId>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub visible_actions: Vec<WorkbenchActionId>,
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
    pub branch_scope: Option<BranchScopeState>,
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

impl WorkbenchState {
    pub fn action_context(&self) -> WorkbenchActionContext {
        WorkbenchActionContext {
            workspace: self.workspace.clone(),
            request: self.request.clone(),
            goals: self.goals.clone(),
            branch_scope: self.branch_scope.clone(),
            evidence_timeline: self.evidence_timeline.clone(),
            assignment: self.assignment.clone(),
            job: self.job.clone(),
            confirmation: self.confirmation.clone(),
        }
    }

    pub fn apply_result(&mut self, action_id: WorkbenchActionId, result: &WorkbenchActionResult) {
        let active_goal_id = self.goals.active_goal().map(|goal| goal.goal_id.clone());
        let evidence_goal_id = match result {
            WorkbenchActionResult::GoalPlanArtifact(plan) => Some(plan.goal.id.clone()),
            WorkbenchActionResult::TaskTestSelectionPlan(selection) => {
                Some(selection.goal_id.clone())
            }
            WorkbenchActionResult::GoalPlanCheckReport(_) => active_goal_id.clone(),
            WorkbenchActionResult::AssignmentState(assignment) => {
                assignment.goal_id.clone().or(active_goal_id.clone())
            }
            WorkbenchActionResult::JobState(_) => active_goal_id.clone(),
            _ => None,
        };
        self.evidence_timeline
            .append_action_result(action_id, evidence_goal_id, result);

        match result {
            WorkbenchActionResult::RequestArtifact(artifact) => {
                self.request
                    .get_or_insert_with(ActiveRequestState::default)
                    .artifact = Some(artifact.clone());
            }
            WorkbenchActionResult::ClassificationOutcome(outcome) => {
                self.request
                    .get_or_insert_with(ActiveRequestState::default)
                    .classification = Some(outcome.clone());
            }
            WorkbenchActionResult::ScopeOutcome(outcome) => {
                self.request
                    .get_or_insert_with(ActiveRequestState::default)
                    .scope = Some(outcome.clone());
            }
            WorkbenchActionResult::ScaffoldPlan(plan) => {
                self.request
                    .get_or_insert_with(ActiveRequestState::default)
                    .scaffold = Some(plan.clone());
            }
            WorkbenchActionResult::GoalPlanArtifact(goal_plan) => {
                self.goals.active_goal_mut().goal_plan = Some(goal_plan.clone());
            }
            WorkbenchActionResult::TaskTestSelectionPlan(selection) => {
                self.goals.active_goal_mut().test_selection = Some(selection.clone());
            }
            WorkbenchActionResult::GoalPlanCheckReport(report) => {
                self.goals.active_goal_mut().check_report = Some(report.clone());
            }
            WorkbenchActionResult::BranchScopeReport(report) => {
                let allowed_ids = report
                    .spec_impact
                    .affected_items
                    .iter()
                    .map(|item| item.id.clone())
                    .collect::<Vec<_>>();
                self.branch_scope = Some(BranchScopeState {
                    range: Some(report.range.clone()),
                    report: Some(report.clone()),
                    bounded_scope: Some(BoundedScope {
                        range: Some(report.range.clone()),
                        allowed_ids: allowed_ids.clone(),
                        max_files: Some(report.changed_files.len()),
                    }),
                    allowed_ids,
                });
            }
            WorkbenchActionResult::SpecImpactReport(_) => {}
            WorkbenchActionResult::ValidationReport(report) => {
                self.workspace
                    .get_or_insert_with(WorkspaceSnapshot::default)
                    .validation_summary = Some(format!("{} issues", report.issues.len()));
            }
            WorkbenchActionResult::HistoryResponse(_) => {}
            WorkbenchActionResult::AssignmentState(assignment) => {
                self.assignment = Some(assignment.clone());
            }
            WorkbenchActionResult::JobState(job) => {
                self.job = job.clone();
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Default)]
pub struct WorkbenchActionContext {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace: Option<WorkspaceSnapshot>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request: Option<ActiveRequestState>,
    #[serde(default)]
    pub goals: GoalListState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub branch_scope: Option<BranchScopeState>,
    #[serde(default)]
    pub evidence_timeline: EvidenceTimelineState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assignment: Option<AssignmentState>,
    #[serde(default)]
    pub job: JobState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confirmation: Option<WorkbenchConfirmationMetadata>,
}

impl WorkbenchActionContext {
    fn has_workspace(&self) -> bool {
        self.workspace.is_some()
    }

    fn has_request(&self) -> bool {
        self.request.is_some()
    }

    fn has_active_goal_plan(&self) -> bool {
        self.goals.has_active_goal_plan()
    }

    fn has_branch_scope(&self) -> bool {
        self.branch_scope.is_some()
    }

    fn has_assignment(&self) -> bool {
        self.assignment.is_some()
    }

    fn has_confirmation(&self) -> bool {
        self.confirmation.is_some()
    }

    fn has_bounded_scope(&self) -> bool {
        self.branch_scope
            .as_ref()
            .and_then(|scope| scope.bounded_scope.as_ref())
            .is_some_and(BoundedScope::is_bounded)
            || self
                .assignment
                .as_ref()
                .and_then(|assignment| assignment.scope.as_ref())
                .is_some_and(BoundedScope::is_bounded)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct WorkbenchActionAvailability {
    pub id: WorkbenchActionId,
    pub title: String,
    pub available: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub missing_state: Vec<WorkbenchStateRequirement>,
    pub mutability: WorkbenchActionMutability,
    pub risk: WorkbenchActionRisk,
    pub ai_eligible: bool,
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

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct WorkbenchAction {
    pub id: WorkbenchActionId,
    pub title: String,
    pub description: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub required_state: Vec<WorkbenchStateRequirement>,
    pub input_schema: WorkbenchActionInputSchema,
    pub mutability: WorkbenchActionMutability,
    pub risk: WorkbenchActionRisk,
    pub function: WorkbenchActionFunction,
    pub output_event: WorkbenchActionOutputEvent,
    pub evidence_kind: WorkbenchEvidenceKind,
    pub ai_eligible: bool,
}

impl WorkbenchAction {
    pub fn availability(&self, context: &WorkbenchActionContext) -> WorkbenchActionAvailability {
        let mut missing_state = Vec::new();
        for requirement in &self.required_state {
            let satisfied = match requirement {
                WorkbenchStateRequirement::WorkspaceLoaded => context.has_workspace(),
                WorkbenchStateRequirement::ActiveRequest => context.has_request(),
                WorkbenchStateRequirement::ActiveGoalPlan => context.has_active_goal_plan(),
                WorkbenchStateRequirement::BranchScopeLoaded => context.has_branch_scope(),
                WorkbenchStateRequirement::AssignmentLoaded => context.has_assignment(),
                WorkbenchStateRequirement::ConfirmationMetadata => context.has_confirmation(),
                WorkbenchStateRequirement::BoundedScope => context.has_bounded_scope(),
            };

            if !satisfied {
                missing_state.push(*requirement);
            }
        }

        WorkbenchActionAvailability {
            id: self.id,
            title: self.title.clone(),
            available: missing_state.is_empty(),
            missing_state,
            mutability: self.mutability,
            risk: self.risk,
            ai_eligible: self.ai_eligible,
        }
    }
}

#[derive(Debug, Clone)]
pub struct WorkbenchActionRegistry {
    actions: &'static [WorkbenchAction],
}

impl Default for WorkbenchActionRegistry {
    fn default() -> Self {
        Self::standard()
    }
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

    pub fn action(&self, id: WorkbenchActionId) -> Option<&WorkbenchAction> {
        self.actions.iter().find(|action| action.id == id)
    }

    pub fn availability(&self, state: &WorkbenchState) -> Vec<WorkbenchActionAvailability> {
        let context = state.action_context();
        self.actions
            .iter()
            .map(|action| action.availability(&context))
            .collect()
    }
}

fn build_registry() -> Vec<WorkbenchAction> {
    vec![
        WorkbenchAction {
            id: WorkbenchActionId::RequestNew,
            title: "New request".to_string(),
            description: "Capture a new request artifact for the active workspace.".to_string(),
            required_state: vec![
                WorkbenchStateRequirement::WorkspaceLoaded,
                WorkbenchStateRequirement::ConfirmationMetadata,
            ],
            input_schema: WorkbenchActionInputSchema::RequestDraft,
            mutability: WorkbenchActionMutability::MutatesStateAndFiles,
            risk: WorkbenchActionRisk::Medium,
            function: WorkbenchActionFunction::ScaffoldRequest,
            output_event: WorkbenchActionOutputEvent::RequestCreated,
            evidence_kind: WorkbenchEvidenceKind::RequestArtifact,
            ai_eligible: false,
        },
        WorkbenchAction {
            id: WorkbenchActionId::RequestClassify,
            title: "Classify request".to_string(),
            description: "Determine whether the active request is a create, change, or delete."
                .to_string(),
            required_state: vec![WorkbenchStateRequirement::ActiveRequest],
            input_schema: WorkbenchActionInputSchema::None,
            mutability: WorkbenchActionMutability::ReadOnly,
            risk: WorkbenchActionRisk::Low,
            function: WorkbenchActionFunction::ClassifyRequest,
            output_event: WorkbenchActionOutputEvent::RequestClassified,
            evidence_kind: WorkbenchEvidenceKind::ClassificationOutcome,
            ai_eligible: false,
        },
        WorkbenchAction {
            id: WorkbenchActionId::RequestScope,
            title: "Scope request".to_string(),
            description:
                "Map the active request to the relevant specification graph and impact area."
                    .to_string(),
            required_state: vec![WorkbenchStateRequirement::ActiveRequest],
            input_schema: WorkbenchActionInputSchema::None,
            mutability: WorkbenchActionMutability::ReadOnly,
            risk: WorkbenchActionRisk::Low,
            function: WorkbenchActionFunction::ScopeRequest,
            output_event: WorkbenchActionOutputEvent::RequestScoped,
            evidence_kind: WorkbenchEvidenceKind::ScopeOutcome,
            ai_eligible: false,
        },
        WorkbenchAction {
            id: WorkbenchActionId::RequestScaffold,
            title: "Scaffold request".to_string(),
            description: "Preview the spec and file updates needed to realize the active request."
                .to_string(),
            required_state: vec![
                WorkbenchStateRequirement::ActiveRequest,
                WorkbenchStateRequirement::ConfirmationMetadata,
            ],
            input_schema: WorkbenchActionInputSchema::RequestArtifact,
            mutability: WorkbenchActionMutability::MutatesStateAndFiles,
            risk: WorkbenchActionRisk::Medium,
            function: WorkbenchActionFunction::ScaffoldRequest,
            output_event: WorkbenchActionOutputEvent::RequestScaffolded,
            evidence_kind: WorkbenchEvidenceKind::ScaffoldPlan,
            ai_eligible: false,
        },
        WorkbenchAction {
            id: WorkbenchActionId::RequestPlan,
            title: "Plan request".to_string(),
            description: "Turn the scoped request into a temporary Goal Plan.".to_string(),
            required_state: vec![
                WorkbenchStateRequirement::ActiveRequest,
                WorkbenchStateRequirement::ConfirmationMetadata,
            ],
            input_schema: WorkbenchActionInputSchema::RequestArtifact,
            mutability: WorkbenchActionMutability::MutatesStateAndFiles,
            risk: WorkbenchActionRisk::Medium,
            function: WorkbenchActionFunction::GenerateGoalPlan,
            output_event: WorkbenchActionOutputEvent::GoalPlanGenerated,
            evidence_kind: WorkbenchEvidenceKind::GoalPlanArtifact,
            ai_eligible: false,
        },
        WorkbenchAction {
            id: WorkbenchActionId::GoalTestSelect,
            title: "Select goal tests".to_string(),
            description: "Choose the narrowest tests that cover the active Goal Plan.".to_string(),
            required_state: vec![WorkbenchStateRequirement::ActiveGoalPlan],
            input_schema: WorkbenchActionInputSchema::GoalPlanArtifact,
            mutability: WorkbenchActionMutability::ReadOnly,
            risk: WorkbenchActionRisk::Low,
            function: WorkbenchActionFunction::SelectGoalTests,
            output_event: WorkbenchActionOutputEvent::GoalTestsSelected,
            evidence_kind: WorkbenchEvidenceKind::TaskTestSelectionPlan,
            ai_eligible: false,
        },
        WorkbenchAction {
            id: WorkbenchActionId::GoalCheck,
            title: "Check goal".to_string(),
            description: "Compare the active Goal Plan against the current branch range."
                .to_string(),
            required_state: vec![WorkbenchStateRequirement::ActiveGoalPlan],
            input_schema: WorkbenchActionInputSchema::GoalPlanArtifact,
            mutability: WorkbenchActionMutability::ReadOnly,
            risk: WorkbenchActionRisk::Low,
            function: WorkbenchActionFunction::CheckGoalPlan,
            output_event: WorkbenchActionOutputEvent::GoalChecked,
            evidence_kind: WorkbenchEvidenceKind::GoalPlanCheckReport,
            ai_eligible: false,
        },
        WorkbenchAction {
            id: WorkbenchActionId::BranchScope,
            title: "Load branch scope".to_string(),
            description: "Summarize the current branch scope and visible impact surface."
                .to_string(),
            required_state: vec![WorkbenchStateRequirement::WorkspaceLoaded],
            input_schema: WorkbenchActionInputSchema::ValidationRequest,
            mutability: WorkbenchActionMutability::ReadOnly,
            risk: WorkbenchActionRisk::Low,
            function: WorkbenchActionFunction::BranchScope,
            output_event: WorkbenchActionOutputEvent::BranchScoped,
            evidence_kind: WorkbenchEvidenceKind::BranchScopeReport,
            ai_eligible: false,
        },
        WorkbenchAction {
            id: WorkbenchActionId::BranchInferGoal,
            title: "Infer goal from branch".to_string(),
            description: "Infer a Goal Plan from the current branch diff and tracked scope."
                .to_string(),
            required_state: vec![
                WorkbenchStateRequirement::BranchScopeLoaded,
                WorkbenchStateRequirement::ConfirmationMetadata,
            ],
            input_schema: WorkbenchActionInputSchema::BranchScope,
            mutability: WorkbenchActionMutability::MutatesStateAndFiles,
            risk: WorkbenchActionRisk::Medium,
            function: WorkbenchActionFunction::InferGoalPlanFromDiff,
            output_event: WorkbenchActionOutputEvent::GoalInferred,
            evidence_kind: WorkbenchEvidenceKind::GoalPlanArtifact,
            ai_eligible: false,
        },
        WorkbenchAction {
            id: WorkbenchActionId::SpecImpact,
            title: "Show spec impact".to_string(),
            description: "Explain the likely specification impact of the current branch scope."
                .to_string(),
            required_state: vec![WorkbenchStateRequirement::BranchScopeLoaded],
            input_schema: WorkbenchActionInputSchema::Selector,
            mutability: WorkbenchActionMutability::ReadOnly,
            risk: WorkbenchActionRisk::Low,
            function: WorkbenchActionFunction::RelateRange,
            output_event: WorkbenchActionOutputEvent::SpecImpactAssessed,
            evidence_kind: WorkbenchEvidenceKind::SpecImpactReport,
            ai_eligible: false,
        },
        WorkbenchAction {
            id: WorkbenchActionId::TraceRange,
            title: "Trace range".to_string(),
            description: "Trace changed files and symbols for the selected branch range."
                .to_string(),
            required_state: vec![WorkbenchStateRequirement::WorkspaceLoaded],
            input_schema: WorkbenchActionInputSchema::BranchScope,
            mutability: WorkbenchActionMutability::ReadOnly,
            risk: WorkbenchActionRisk::Low,
            function: WorkbenchActionFunction::TraceRange,
            output_event: WorkbenchActionOutputEvent::BranchScoped,
            evidence_kind: WorkbenchEvidenceKind::BranchScopeReport,
            ai_eligible: false,
        },
        WorkbenchAction {
            id: WorkbenchActionId::RelateRange,
            title: "Relate range".to_string(),
            description: "Relate the selected branch range to affected specs and tests."
                .to_string(),
            required_state: vec![WorkbenchStateRequirement::BranchScopeLoaded],
            input_schema: WorkbenchActionInputSchema::BranchScope,
            mutability: WorkbenchActionMutability::ReadOnly,
            risk: WorkbenchActionRisk::Low,
            function: WorkbenchActionFunction::RelateRange,
            output_event: WorkbenchActionOutputEvent::SpecImpactAssessed,
            evidence_kind: WorkbenchEvidenceKind::SpecImpactReport,
            ai_eligible: false,
        },
        WorkbenchAction {
            id: WorkbenchActionId::ValidationRun,
            title: "Run validation".to_string(),
            description: "Run the repository validation pass for the active workspace.".to_string(),
            required_state: vec![WorkbenchStateRequirement::WorkspaceLoaded],
            input_schema: WorkbenchActionInputSchema::ValidationRequest,
            mutability: WorkbenchActionMutability::ReadOnly,
            risk: WorkbenchActionRisk::Low,
            function: WorkbenchActionFunction::ValidateWorkspace,
            output_event: WorkbenchActionOutputEvent::ValidationRun,
            evidence_kind: WorkbenchEvidenceKind::ValidationReport,
            ai_eligible: false,
        },
        WorkbenchAction {
            id: WorkbenchActionId::HistoryShow,
            title: "Show history".to_string(),
            description: "Show the evidence trail for the active request or goal.".to_string(),
            required_state: vec![WorkbenchStateRequirement::WorkspaceLoaded],
            input_schema: WorkbenchActionInputSchema::HistoryQuery,
            mutability: WorkbenchActionMutability::ReadOnly,
            risk: WorkbenchActionRisk::Low,
            function: WorkbenchActionFunction::HistoryForItem,
            output_event: WorkbenchActionOutputEvent::HistoryShown,
            evidence_kind: WorkbenchEvidenceKind::HistoryResponse,
            ai_eligible: false,
        },
        WorkbenchAction {
            id: WorkbenchActionId::AssignmentCreate,
            title: "Create assignment".to_string(),
            description:
                "Assign the active goal to a human or AI with explicit scope and evidence."
                    .to_string(),
            required_state: vec![
                WorkbenchStateRequirement::ActiveGoalPlan,
                WorkbenchStateRequirement::ConfirmationMetadata,
            ],
            input_schema: WorkbenchActionInputSchema::Assignment,
            mutability: WorkbenchActionMutability::MutatesState,
            risk: WorkbenchActionRisk::Medium,
            function: WorkbenchActionFunction::AssignmentCreate,
            output_event: WorkbenchActionOutputEvent::AssignmentCreated,
            evidence_kind: WorkbenchEvidenceKind::AssignmentState,
            ai_eligible: false,
        },
        WorkbenchAction {
            id: WorkbenchActionId::AgentRun,
            title: "Run agent".to_string(),
            description: "Launch an AI run against a bounded goal scope and assignment."
                .to_string(),
            required_state: vec![
                WorkbenchStateRequirement::ActiveGoalPlan,
                WorkbenchStateRequirement::AssignmentLoaded,
                WorkbenchStateRequirement::BoundedScope,
                WorkbenchStateRequirement::ConfirmationMetadata,
            ],
            input_schema: WorkbenchActionInputSchema::AgentRun,
            mutability: WorkbenchActionMutability::MutatesStateAndFiles,
            risk: WorkbenchActionRisk::High,
            function: WorkbenchActionFunction::AgentRun,
            output_event: WorkbenchActionOutputEvent::AgentRunQueued,
            evidence_kind: WorkbenchEvidenceKind::JobState,
            ai_eligible: true,
        },
    ]
}

pub fn workbench_actions() -> &'static [WorkbenchAction] {
    registry_actions()
}

pub fn workbench_action_registry() -> WorkbenchActionRegistry {
    WorkbenchActionRegistry::standard()
}

pub fn workbench_api_payload(state: WorkbenchState) -> WorkbenchApiPayload {
    WorkbenchApiPayload::new(state)
}

fn registry_actions() -> &'static [WorkbenchAction] {
    static REGISTRY: OnceLock<Vec<WorkbenchAction>> = OnceLock::new();
    REGISTRY.get_or_init(build_registry).as_slice()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn action(id: WorkbenchActionId) -> &'static WorkbenchAction {
        workbench_actions()
            .iter()
            .find(|candidate| candidate.id == id)
            .expect("action should exist")
    }

    #[test]
    fn request_scope_is_unavailable_without_a_request() {
        let availability = action(WorkbenchActionId::RequestScope)
            .availability(&WorkbenchActionContext::default());

        assert!(!availability.available);
        assert!(
            availability
                .missing_state
                .contains(&WorkbenchStateRequirement::ActiveRequest)
        );
    }

    #[test]
    fn request_classify_is_available_with_an_active_request() {
        let mut state = WorkbenchState::default();
        state.request = Some(ActiveRequestState::default());

        let availability =
            action(WorkbenchActionId::RequestClassify).availability(&state.action_context());

        assert!(availability.available);
    }

    #[test]
    fn goal_test_select_is_available_with_an_active_goal_plan() {
        let mut state = WorkbenchState::default();
        state.goals.active.push(ActiveGoalState {
            goal_id: "goal-1".to_string(),
            goal_plan: Some(GoalPlanArtifact {
                version: 1,
                kind: "goal_plan".to_string(),
                request_path: None,
                request: None,
                classification: None,
                source: Default::default(),
                goal: syu_task_model::GoalPlanGoal {
                    id: "goal-1".to_string(),
                    title: "Goal".to_string(),
                    statement: "Do the thing".to_string(),
                    non_goals: Vec::new(),
                    inferred: false,
                },
                spec_mapping: Default::default(),
                implementation_plan: syu_task_model::GoalPlanImplementationPlan {
                    confidence: Some(syu_task_model::GoalPlanConfidence::High),
                    scope: Default::default(),
                    steps: vec!["implement".to_string()],
                },
                test_plan: syu_task_model::GoalPlanTestPlan {
                    selection_mode: syu_task_model::GoalPlanSelectionMode::Minimal,
                    confidence: Some(syu_task_model::GoalPlanConfidence::High),
                    required_tests: std::collections::BTreeMap::new(),
                    suggested_tests: std::collections::BTreeMap::new(),
                },
                coverage: syu_task_model::GoalPlanCoverage {
                    mode: syu_task_model::GoalPlanCoverageMode::ChangedLines,
                    threshold: 0,
                    include: Vec::new(),
                    exclude: Vec::new(),
                },
                completion: Default::default(),
                warnings: Vec::new(),
            }),
            ..ActiveGoalState::default()
        });

        let availability =
            action(WorkbenchActionId::GoalTestSelect).availability(&state.action_context());

        assert!(availability.available);
    }

    #[test]
    fn goal_test_select_uses_the_selected_goal_only() {
        let mut state = WorkbenchState::default();
        state.goals.active.push(ActiveGoalState {
            goal_id: "goal-1".to_string(),
            goal_plan: Some(GoalPlanArtifact {
                version: 1,
                kind: "goal_plan".to_string(),
                request_path: None,
                request: None,
                classification: None,
                source: Default::default(),
                goal: syu_task_model::GoalPlanGoal {
                    id: "goal-1".to_string(),
                    title: "Goal".to_string(),
                    statement: "Do the thing".to_string(),
                    non_goals: Vec::new(),
                    inferred: false,
                },
                spec_mapping: Default::default(),
                implementation_plan: syu_task_model::GoalPlanImplementationPlan {
                    confidence: Some(syu_task_model::GoalPlanConfidence::High),
                    scope: Default::default(),
                    steps: vec!["implement".to_string()],
                },
                test_plan: syu_task_model::GoalPlanTestPlan {
                    selection_mode: syu_task_model::GoalPlanSelectionMode::Minimal,
                    confidence: Some(syu_task_model::GoalPlanConfidence::High),
                    required_tests: std::collections::BTreeMap::new(),
                    suggested_tests: std::collections::BTreeMap::new(),
                },
                coverage: syu_task_model::GoalPlanCoverage {
                    mode: syu_task_model::GoalPlanCoverageMode::ChangedLines,
                    threshold: 0,
                    include: Vec::new(),
                    exclude: Vec::new(),
                },
                completion: Default::default(),
                warnings: Vec::new(),
            }),
            ..ActiveGoalState::default()
        });
        state.goals.active.push(ActiveGoalState {
            goal_id: "goal-2".to_string(),
            goal_plan: None,
            ..ActiveGoalState::default()
        });
        state.goals.selected_goal_id = Some("goal-2".to_string());

        let availability =
            action(WorkbenchActionId::GoalTestSelect).availability(&state.action_context());

        assert!(!availability.available);
        assert!(
            availability
                .missing_state
                .contains(&WorkbenchStateRequirement::ActiveGoalPlan)
        );
    }

    #[test]
    fn history_response_is_recorded_once() {
        let mut state = WorkbenchState::default();

        state.apply_result(
            WorkbenchActionId::HistoryShow,
            &WorkbenchActionResult::HistoryResponse(HistoryResponse {
                id: "goal-1".to_string(),
                entity_kind: "goal",
                title: "Goal".to_string(),
                status: "active",
                repository_root: "/repo".to_string(),
                kind: "goal",
                include_related: false,
                scope: None,
                path_filter: None,
                tracked_paths: Vec::new(),
                lifecycle_events: Vec::new(),
                commits: Vec::new(),
            }),
        );

        assert_eq!(state.evidence_timeline.entries.len(), 1);
        assert_eq!(
            state.evidence_timeline.entries[0].kind,
            WorkbenchEvidenceKind::HistoryResponse
        );
    }

    #[test]
    fn evidence_record_serializes_with_goal_and_attachment() {
        let record = EvidenceRecord::new(
            WorkbenchEvidenceKind::GoalPlanCheckReport,
            EvidenceStatus::Warn,
            "goal check found issues",
            Some(EvidenceSource::Action {
                action_id: Some(WorkbenchActionId::GoalCheck),
                action_label: Some("goal.check".to_string()),
            }),
        )
        .with_goal_id(Some("goal-1".to_string()))
        .with_action_id(WorkbenchActionId::GoalCheck)
        .with_subject(EvidenceSubject::Goal)
        .with_severity(EvidenceSeverity::Medium)
        .with_command(EvidenceCommand {
            command: "goal.check".to_string(),
            args: Vec::new(),
        })
        .with_attachment(EvidenceAttachment {
            label: "result".to_string(),
            mime_type: Some("application/json".to_string()),
            summary: Some("JSON payload".to_string()),
            content: Some("{\"ok\":false}".to_string()),
            truncated: false,
        });

        let json = serde_json::to_string(&record).expect("evidence record should serialize");
        let roundtrip: EvidenceRecord =
            serde_json::from_str(&json).expect("evidence record should roundtrip");

        assert_eq!(roundtrip.goal_id.as_deref(), Some("goal-1"));
        assert_eq!(roundtrip.action_id, Some(WorkbenchActionId::GoalCheck));
        assert_eq!(roundtrip.status, EvidenceStatus::Warn);
        assert_eq!(roundtrip.attachments.len(), 1);
    }

    #[test]
    fn goal_check_appends_goal_scoped_evidence() {
        let mut state = WorkbenchState::default();
        state.goals.active.push(ActiveGoalState {
            goal_id: "goal-1".to_string(),
            ..ActiveGoalState::default()
        });

        state.apply_result(
            WorkbenchActionId::GoalCheck,
            &WorkbenchActionResult::GoalPlanCheckReport(GoalPlanCheckReport {
                plan_path: "plan.yaml".to_string(),
                range: "HEAD~1..HEAD".to_string(),
                changed_files: vec!["src/lib.rs".to_string()],
                issues: Vec::new(),
            }),
        );

        assert_eq!(state.evidence_timeline.entries.len(), 1);
        let entry = &state.evidence_timeline.entries[0];
        assert_eq!(entry.goal_id.as_deref(), Some("goal-1"));
        assert_eq!(entry.status, EvidenceStatus::Pass);
        assert_eq!(entry.action_id, Some(WorkbenchActionId::GoalCheck));
        assert_eq!(
            entry
                .command
                .as_ref()
                .map(|command| command.command.as_str()),
            Some("goal.check")
        );
    }

    #[test]
    fn branch_infer_goal_is_available_with_branch_scope() {
        let mut state = WorkbenchState::default();
        state.branch_scope = Some(BranchScopeState {
            range: Some("HEAD~1..HEAD".to_string()),
            bounded_scope: Some(BoundedScope {
                range: Some("HEAD~1..HEAD".to_string()),
                allowed_ids: vec!["REQ-WORKBENCH-001".to_string()],
                max_files: Some(3),
            }),
            ..BranchScopeState::default()
        });
        state.confirmation = Some(WorkbenchConfirmationMetadata {
            confirmed_by: "tester".to_string(),
            rationale: Some("needed for the mutating action".to_string()),
            scope_token: Some("scope-token".to_string()),
        });

        let availability =
            action(WorkbenchActionId::BranchInferGoal).availability(&state.action_context());

        assert!(availability.available);
    }

    #[test]
    fn mutating_actions_require_confirmation_metadata() {
        let mutating_actions = workbench_actions()
            .iter()
            .filter(|candidate| candidate.mutability.requires_confirmation())
            .collect::<Vec<_>>();

        assert!(!mutating_actions.is_empty());
        for action in mutating_actions {
            assert!(
                action
                    .required_state
                    .contains(&WorkbenchStateRequirement::ConfirmationMetadata),
                "{} should require confirmation metadata",
                action.id.label()
            );
        }
    }

    #[test]
    fn agent_eligible_actions_have_bounded_scope() {
        let ai_actions = workbench_actions()
            .iter()
            .filter(|candidate| candidate.ai_eligible)
            .collect::<Vec<_>>();

        assert_eq!(ai_actions.len(), 1);
        for action in ai_actions {
            assert!(
                action
                    .required_state
                    .contains(&WorkbenchStateRequirement::BoundedScope),
                "{} should require bounded scope",
                action.id.label()
            );
        }
    }

    #[test]
    fn api_payload_exposes_actions_and_availability() {
        let payload = workbench_api_payload(WorkbenchState::default());

        assert_eq!(payload.actions.len(), workbench_actions().len());
        assert_eq!(payload.availability.len(), payload.actions.len());
        assert!(
            payload
                .availability
                .iter()
                .any(|availability| availability.id == WorkbenchActionId::RequestScope)
        );
    }
}
