#![forbid(unsafe_code)]
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use syu_diagnostics::Diagnostic;
use syu_spec_model::{
    BindingRole, BoundTargetRef, ContractKind, RepoPath, SpecAnchor, SpecItemRef,
};

pub const WORK_REQUEST_SCHEMA: &str = "syu/work-request/v1";
pub const WORK_PLAN_SCHEMA: &str = "syu/work-plan/v1";
pub const CONTEXT_PACK_SCHEMA: &str = "syu/context-pack/v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum WorkOperation {
    Add,
    Modify,
    Remove,
    Refactor,
    Document,
    Investigate,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum WorkSeed {
    Anchor(SpecAnchor),
    Item(SpecItemRef),
}
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkConstraints {
    #[serde(default)]
    pub include_facets: Vec<String>,
    #[serde(default)]
    pub exclude_paths: Vec<String>,
    #[serde(default)]
    pub max_slices: Option<usize>,
    #[serde(default)]
    pub max_added_bytes_per_target: Option<usize>,
    #[serde(default)]
    pub max_added_lines_per_target: Option<usize>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkRequest {
    pub schema: String,
    pub id: String,
    pub summary: String,
    pub operation: WorkOperation,
    pub seeds: Vec<WorkSeed>,
    #[serde(default)]
    pub constraints: WorkConstraints,
    #[serde(default)]
    pub requested_targets: Vec<RequestedTarget>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RequestedTarget {
    #[serde(rename = "ref")]
    pub reference: BoundTargetRef,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub criterion: Option<SpecAnchor>,
    pub transition: TargetTransition,
}

impl RequestedTarget {
    pub fn reference(&self) -> &BoundTargetRef {
        &self.reference
    }

    pub fn transition(&self, _default: TargetTransition) -> TargetTransition {
        self.transition
    }

    pub fn criterion(&self) -> Option<&SpecAnchor> {
        self.criterion.as_ref()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TargetTransition {
    Add,
    Modify,
    Remove,
    RunOnly,
    Readonly,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlanBasis {
    pub revision: String,
    pub workspace_fingerprint: String,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PlanStatus {
    Ready,
    Blocked,
    NeedsReview,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PlanConfidence {
    Exact,
    ReviewedSuggestion,
    Low,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResolvedSelector {
    pub description: String,
    #[serde(default)]
    pub symbols: Vec<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlannedTarget {
    #[serde(rename = "ref")]
    pub reference: BoundTargetRef,
    pub transition: TargetTransition,
    #[serde(default)]
    pub lifecycle: TargetLifecycle,
    pub access: TargetAccessMode,
    pub resolved_path: String,
    pub resolved_selector: ResolvedSelector,
    pub content_hash: String,
    pub excerpt_hash: String,
    pub adapter: String,
    pub facet: String,
    pub role: BindingRole,
    pub byte_start: usize,
    pub byte_end: usize,
    pub line_start: usize,
    pub line_end: usize,
    #[serde(default)]
    pub budget_bytes: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub budget_lines: Option<usize>,
    pub reason: String,
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TargetLifecycle {
    #[default]
    Stable,
    EnsurePresent,
    EnsureAbsent,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TargetAccessMode {
    Editable,
    RunOnly,
    Readonly,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AcceptanceRef {
    pub anchor: SpecAnchor,
    pub statement: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NonGoal {
    pub code: String,
    pub statement: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum CompletionCheck {
    Command {
        program: String,
        #[serde(default)]
        args: Vec<String>,
        #[serde(default)]
        cwd: Option<RepoPath>,
    },
    Validate {
        preset: String,
    },
    RuleSet {
        rules: Vec<String>,
    },
    TargetExists {
        target: BoundTargetRef,
    },
    TargetAbsent {
        target: BoundTargetRef,
    },
    DiffWithinScope,
    ContractConsistent {
        contract: SpecAnchor,
    },
}
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SliceBudgetUsage {
    pub editable_files: usize,
    pub editable_symbols: usize,
    pub verification_targets: usize,
    pub readonly_targets: usize,
    #[serde(default)]
    pub total_bytes: usize,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutionSlice {
    pub id: String,
    pub goal: String,
    pub anchors: Vec<SpecAnchor>,
    pub editable_targets: Vec<PlannedTarget>,
    pub verification_targets: Vec<PlannedTarget>,
    pub readonly_context: Vec<PlannedTarget>,
    pub acceptance: Vec<AcceptanceRef>,
    #[serde(default)]
    pub contracts: Vec<SpecAnchor>,
    pub non_goals: Vec<NonGoal>,
    pub completion: Vec<CompletionCheck>,
    pub budget: SliceBudgetUsage,
    pub confidence: PlanConfidence,
    pub blockers: Vec<Diagnostic>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkPlan {
    pub schema: String,
    pub id: String,
    pub basis: PlanBasis,
    #[serde(default)]
    pub execution: PlanExecution,
    pub request: WorkRequest,
    pub canonical_digest: String,
    pub status: PlanStatus,
    pub slices: Vec<ExecutionSlice>,
    #[serde(default)]
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PlanExecution {
    #[default]
    IsolatedSlices,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ContextMode {
    Editable,
    Verification,
    Readonly,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "entry_kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum SpecContextEntry {
    Statement {
        anchor: SpecAnchor,
        text: String,
    },
    Contract {
        anchor: SpecAnchor,
        kind: ContractKind,
        source: BoundTargetRef,
        guarantees: Vec<SpecAnchor>,
        participants: Vec<ContractParticipantContext>,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContractParticipantContext {
    pub binding: SpecAnchor,
    pub role: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "entry_kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum ArtifactContextEntry {
    Target(TargetContext),
    IntendedTarget(IntendedTargetContext),
    Support(SupportContext),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TargetContext {
    #[serde(rename = "ref")]
    pub reference: BoundTargetRef,
    pub transition: TargetTransition,
    #[serde(default)]
    pub lifecycle: TargetLifecycle,
    pub mode: ContextMode,
    pub access: TargetAccessMode,
    pub path: String,
    pub selector: ResolvedSelector,
    pub line_start: usize,
    pub line_end: usize,
    pub byte_start: usize,
    pub byte_end: usize,
    pub adapter: String,
    pub facet: String,
    pub role: BindingRole,
    pub content_hash: String,
    pub excerpt_hash: String,
    pub reason: String,
    pub excerpt: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IntendedTargetContext {
    #[serde(rename = "ref")]
    pub reference: BoundTargetRef,
    pub transition: TargetTransition,
    #[serde(default)]
    pub lifecycle: TargetLifecycle,
    pub mode: ContextMode,
    pub access: TargetAccessMode,
    pub path: String,
    pub selector: ResolvedSelector,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub budget_bytes: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub budget_lines: Option<usize>,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SupportContext {
    pub support_id: String,
    pub supports: BoundTargetRef,
    pub mode: ContextMode,
    pub access: TargetAccessMode,
    pub path: String,
    pub selector: ResolvedSelector,
    pub line_start: usize,
    pub line_end: usize,
    pub byte_start: usize,
    pub byte_end: usize,
    pub adapter: String,
    pub facet: String,
    pub role: BindingRole,
    pub content_hash: String,
    pub excerpt_hash: String,
    pub reason: String,
    pub excerpt: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContextInstructions {
    pub goal: String,
    pub non_goals: Vec<NonGoal>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContextPack {
    pub schema: String,
    pub plan: String,
    pub slice: String,
    pub basis: PlanBasis,
    pub instructions: ContextInstructions,
    pub spec_context: Vec<SpecContextEntry>,
    pub artifact_context: Vec<ArtifactContextEntry>,
    pub completion: Vec<CompletionCheck>,
}

pub fn work_plan_digest(plan: &WorkPlan) -> String {
    let mut copy = plan.clone();
    copy.canonical_digest.clear();
    let bytes = serde_json::to_vec(&copy).expect("serialize work plan digest");
    let mut hash = Sha256::new();
    hash.update(bytes);
    format!("sha256:{:x}", hash.finalize())
}
