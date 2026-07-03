#![forbid(unsafe_code)]
use serde::{Deserialize, Serialize};
use syu_diagnostics::Diagnostic;
use syu_spec_model::{BindingRole, BoundTargetRef, SpecAnchor, SpecItemRef};

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
    pub requested_targets: Vec<BoundTargetRef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlanBasis {
    pub revision: String,
    pub workspace_fingerprint: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EmbeddedRequest {
    pub id: String,
    pub summary: String,
    pub operation: WorkOperation,
    pub seeds: Vec<WorkSeed>,
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
    pub reason: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AcceptanceRef {
    pub anchor: SpecAnchor,
    pub statement: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum CompletionCheck {
    Command { command: String },
    Validate { preset: String },
    RuleSet { rules: Vec<String> },
    TargetExists { target: BoundTargetRef },
    DiffWithinScope,
    ContractConsistent { contract: SpecAnchor },
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
    pub non_goals: Vec<String>,
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
    pub request: EmbeddedRequest,
    pub status: PlanStatus,
    pub slices: Vec<ExecutionSlice>,
    #[serde(default)]
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ContextMode {
    Editable,
    Verification,
    Readonly,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpecExcerpt {
    pub anchor: SpecAnchor,
    pub text: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactExcerpt {
    #[serde(rename = "ref")]
    pub reference: BoundTargetRef,
    pub mode: ContextMode,
    pub excerpt: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContextInstructions {
    pub goal: String,
    pub non_goals: Vec<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContextPack {
    pub schema: String,
    pub plan: String,
    pub slice: String,
    pub basis: PlanBasis,
    pub instructions: ContextInstructions,
    pub spec_context: Vec<SpecExcerpt>,
    pub artifact_context: Vec<ArtifactExcerpt>,
    pub completion: Vec<CompletionCheck>,
}
