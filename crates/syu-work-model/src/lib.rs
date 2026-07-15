#![forbid(unsafe_code)]
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use syu_diagnostics::Diagnostic;
use syu_spec_model::{
    BindingRole, BoundTargetRef, ContractKind, RepoPath, SpecAnchor, SpecItemRef,
};

pub const WORK_REQUEST_SCHEMA: &str = "syu/work-request/v1";
pub const WORK_PLAN_SCHEMA: &str = "syu/work-plan/v1";
pub const VERIFICATION_RECEIPT_SCHEMA: &str = "syu/verification-receipt/v2";
pub const COMPLETION_REPORT_SCHEMA: &str = "syu/completion-report/v1";
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
    ArtifactIdentity { artifact_identity: String },
    ChangedUnit { changed_unit: String },
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
    /// Complete pre-state fingerprint retained for UI freshness checks and
    /// exact pre-state validation. Post-state execution must not reject an
    /// editable artifact merely because this value changed.
    pub workspace_fingerprint: String,
    /// Specification and configuration inputs that must remain stable while a
    /// plan is being executed.
    pub spec_fingerprint: String,
    /// Binding, target, and ownership relationships that define the plan's
    /// current ownership basis.
    pub ownership_fingerprint: String,
    /// Content and identity snapshot for every readonly or run-only target.
    /// Editable target content is intentionally excluded from this digest.
    pub readonly_fingerprint: String,
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
    /// When a request originates from a changed semantic unit, retain that
    /// exact identity even when its owner binding also declares a broader
    /// entrypoint target.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact_identity: Option<String>,
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
    pub target: BoundTargetRef,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VerificationReceipt {
    pub schema: String,
    pub plan_digest: String,
    pub slice_id: String,
    pub revision: String,
    pub workspace_fingerprint: String,
    pub started_at: String,
    pub completed_at: String,
    pub executions: Vec<VerificationExecution>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VerificationExecution {
    pub target: BoundTargetRef,
    pub runner: String,
    pub command: Vec<String>,
    pub exit_code: i32,
    pub stdout_digest: String,
    pub stderr_digest: String,
    pub proof: ExactTestEvidence,
    pub implementation_digests: std::collections::BTreeMap<BoundTargetRef, String>,
    pub verification_digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExactTestEvidence {
    pub identity: String,
    pub matched_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CompletionStatus {
    Complete,
    Blocked,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompletionCriterionEvidence {
    pub anchor: SpecAnchor,
    pub statement: String,
    pub verification_targets: Vec<BoundTargetRef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompletionCheckEvidence {
    pub check: CompletionCheck,
    pub passed: bool,
    #[serde(default)]
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompletionBlocker {
    pub code: String,
    pub message: String,
    pub next_action: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompletionReport {
    pub schema: String,
    pub plan_digest: String,
    pub slice_id: String,
    pub status: CompletionStatus,
    pub demonstrated: Vec<CompletionCriterionEvidence>,
    pub checks: Vec<CompletionCheckEvidence>,
    pub blockers: Vec<CompletionBlocker>,
}

/// Return a stable digest of the targets that are not editable by a work
/// slice. This is the immutable execution boundary used by post-state plan
/// validation; editable target content is deliberately omitted.
pub fn readonly_targets_fingerprint(slices: &[ExecutionSlice]) -> String {
    let mut hash = Sha256::new();
    for slice in slices {
        hash.update(slice.id.as_bytes());
        for target in slice
            .verification_targets
            .iter()
            .chain(slice.readonly_context.iter())
            .filter(|target| target.access != TargetAccessMode::Editable)
        {
            hash.update(target.reference.to_string().as_bytes());
            hash.update(format!("{:?}", target.transition).as_bytes());
            hash.update(format!("{:?}", target.lifecycle).as_bytes());
            hash.update(format!("{:?}", target.access).as_bytes());
            hash.update(target.resolved_path.as_bytes());
            hash.update(target.resolved_selector.description.as_bytes());
            for symbol in &target.resolved_selector.symbols {
                hash.update(symbol.as_bytes());
            }
            hash.update(target.content_hash.as_bytes());
            hash.update(target.excerpt_hash.as_bytes());
            hash.update(target.adapter.as_bytes());
            hash.update(target.facet.as_bytes());
            hash.update(format!("{:?}", target.role).as_bytes());
        }
    }
    format!("sha256:{:x}", hash.finalize())
}

pub fn work_plan_digest(plan: &WorkPlan) -> String {
    let mut copy = plan.clone();
    copy.canonical_digest.clear();
    let bytes = serde_json::to_vec(&copy).expect("serialize work plan digest");
    let mut hash = Sha256::new();
    hash.update(bytes);
    format!("sha256:{:x}", hash.finalize())
}
