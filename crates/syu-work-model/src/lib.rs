#![forbid(unsafe_code)]
use serde::{Deserialize, Deserializer, Serialize, Serializer, de};
use sha2::{Digest, Sha256};
use syu_diagnostics::Diagnostic;
use syu_spec_model::format_sha256;
use syu_spec_model::{
    BindingRole, BoundTargetRef, ContractKind, RepoPath, SpecAnchor, SpecItemRef,
};

pub const WORK_REQUEST_SCHEMA: &str = "syu/work-request/v1";
pub const WORK_PLAN_SCHEMA: &str = "syu/work-plan/v1";
pub const VERIFICATION_RECEIPT_SCHEMA: &str = "syu/verification-receipt/v3";
pub const COMPLETION_REPORT_SCHEMA: &str = "syu/completion-report/v1";
pub const CONTEXT_PACK_SCHEMA: &str = "syu/context-pack/v1";
pub const PLAN_APPROVAL_SCHEMA: &str = "syu/plan-approval/v1";
pub const COMPLETION_ATTEMPT_SCHEMA: &str = "syu/completion-attempt/v1";
pub const FINALIZATION_RECEIPT_SCHEMA: &str = "syu/finalization-receipt/v1";
pub const AGENT_RUN_SCHEMA: &str = "syu/agent-run/v1";
pub const AGENT_PATCH_SCHEMA: &str = "syu/agent-patch/v1";
pub const AGENT_EVENT_SCHEMA: &str = "syu/agent-event/v1";
pub const AGENT_CONTEXT_SCHEMA: &str = "syu/agent-context/v1";
pub const WORK_ORIGIN_CAPABILITY_SCHEMA: &str = "syu/work-origin-capability/v1";
pub const WORK_SELECT_SLICE_SCHEMA: &str = "syu/work-select-slice/v1";
pub const WORK_SELECT_SLICE_RESPONSE_SCHEMA: &str = "syu/work-select-slice-response/v1";
pub const WORK_ERROR_SCHEMA: &str = "syu/work-error/v1";
pub const WORK_SPLIT_RECOVERY_SCHEMA: &str = "syu/work-split-recovery/v1";
pub const WORK_EXECUTION_IDENTITY_DIGEST_DOMAIN: &str = "syu/work-execution-identity-digest/v1\0";
pub const VERIFICATION_RECEIPT_DIGEST_DOMAIN: &str = "syu/verification-receipt-digest/v1\0";
pub const FINALIZATION_RECEIPT_DIGEST_DOMAIN: &str = "syu/finalization-receipt-digest/v1\0";

fn is_stable_target_lifecycle(value: &TargetLifecycle) -> bool {
    matches!(value, TargetLifecycle::Stable)
}

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
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum WorkOrigin {
    RequirementCriterion {
        criterion: SpecAnchor,
    },
    FeatureImplementationBinding {
        binding: SpecAnchor,
        criterion: SpecAnchor,
        targets: Vec<BoundTargetRef>,
    },
    FeatureImplementationTarget {
        target: BoundTargetRef,
        binding: SpecAnchor,
        criterion: SpecAnchor,
    },
}

impl WorkOrigin {
    pub fn criterion(&self) -> &SpecAnchor {
        match self {
            Self::RequirementCriterion { criterion }
            | Self::FeatureImplementationBinding { criterion, .. }
            | Self::FeatureImplementationTarget { criterion, .. } => criterion,
        }
    }

    pub fn targets(&self) -> &[BoundTargetRef] {
        match self {
            Self::RequirementCriterion { .. } => &[],
            Self::FeatureImplementationBinding { targets, .. } => targets,
            Self::FeatureImplementationTarget { target, .. } => std::slice::from_ref(target),
        }
    }
}

/// The only authority that identifies an executable Work boundary. Locator
/// ids such as run and attempt ids are deliberately not part of this value.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutionIdentity {
    pub plan_digest: String,
    pub slice_id: String,
}

pub fn execution_identity_digest(identity: &ExecutionIdentity) -> String {
    let bytes = canonical_json_bytes(serde_json::to_value(identity).expect("serialize identity"));
    let mut hash = Sha256::new();
    hash.update(WORK_EXECUTION_IDENTITY_DIGEST_DOMAIN.as_bytes());
    hash.update(bytes);
    format_sha256(hash.finalize())
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
    /// Internal v1 boundary used when a user selects one already-planned
    /// execution slice. It prevents criterion-wide context expansion during
    /// the canonical replan of that selection.
    #[serde(default)]
    pub exact_scope: bool,
    /// Generated targets retained by the selected execution slice. Their
    /// access mode is derived from the target graph, so a selected slice
    /// carries this small authoritative boundary across a replan.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub exact_generated_targets: Vec<BoundTargetRef>,
    /// Contract closure retained by the selected execution slice.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub exact_contracts: Vec<SpecAnchor>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkRequest {
    pub schema: String,
    pub id: String,
    pub title: String,
    pub operation: WorkOperation,
    pub origin: WorkOrigin,
    pub constraints: WorkConstraints,
    pub requested_targets: Vec<RequestedTarget>,
}

#[derive(Debug, Clone)]
pub struct RequestedTarget {
    pub reference: BoundTargetRef,
    pub criterion: Option<SpecAnchor>,
    pub transition: TargetTransition,
}

impl PartialEq for RequestedTarget {
    fn eq(&self, other: &Self) -> bool {
        self.reference == other.reference && self.transition == other.transition
    }
}

impl Eq for RequestedTarget {}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RequestedTargetWire {
    reference: BoundTargetRef,
    transition: TargetTransition,
    access: TargetAccessMode,
    lifecycle: TargetLifecycle,
}

impl Serialize for RequestedTarget {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let (access, lifecycle) = requested_target_modes(self.transition);
        RequestedTargetWireRef {
            reference: &self.reference,
            transition: self.transition,
            access,
            lifecycle,
        }
        .serialize(serializer)
    }
}

#[derive(Serialize)]
struct RequestedTargetWireRef<'a> {
    reference: &'a BoundTargetRef,
    transition: TargetTransition,
    access: TargetAccessMode,
    lifecycle: TargetLifecycle,
}

impl<'de> Deserialize<'de> for RequestedTarget {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let wire = RequestedTargetWire::deserialize(deserializer)?;
        let (expected_access, expected_lifecycle) = requested_target_modes(wire.transition);
        if wire.access != expected_access || wire.lifecycle != expected_lifecycle {
            return Err(de::Error::custom(
                "requested target access/lifecycle do not match its transition",
            ));
        }
        Ok(Self {
            reference: wire.reference,
            criterion: None,
            transition: wire.transition,
        })
    }
}

fn requested_target_modes(transition: TargetTransition) -> (TargetAccessMode, TargetLifecycle) {
    match transition {
        TargetTransition::Add => (TargetAccessMode::Editable, TargetLifecycle::EnsurePresent),
        TargetTransition::Modify => (TargetAccessMode::Editable, TargetLifecycle::Stable),
        TargetTransition::Remove => (TargetAccessMode::Editable, TargetLifecycle::EnsureAbsent),
        TargetTransition::RunOnly => (TargetAccessMode::RunOnly, TargetLifecycle::Stable),
        TargetTransition::Readonly => (TargetAccessMode::Readonly, TargetLifecycle::Stable),
    }
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum TargetTransition {
    Add,
    #[default]
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
    /// Editable targets and derived generated outputs are intentionally
    /// excluded from this digest.
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
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VerificationClaimRef {
    pub target: BoundTargetRef,
    pub criterion: SpecAnchor,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlannedTarget {
    #[serde(rename = "ref")]
    pub reference: BoundTargetRef,
    /// Exact verification claim selected for a run-only target. A target can
    /// prove several criteria, so the target reference alone is insufficient
    /// to determine the runner, covered implementation, or receipt evidence.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verification_claim: Option<VerificationClaimRef>,
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
    /// For an intended semantic target, the approved hash of the existing
    /// containing file. File creation intentionally has no container hash.
    /// This keeps an Add operation from silently applying to a file that
    /// changed after the plan was approved.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub container_content_hash: Option<String>,
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
    /// Derived output may change during execution, but implementation tools
    /// cannot write it directly. Its exact generated-from source must be an
    /// editable target in the same slice.
    Generated,
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

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OriginClosure {
    pub implementation_targets: Vec<BoundTargetRef>,
    pub verification_targets: Vec<BoundTargetRef>,
    pub readonly_targets: Vec<BoundTargetRef>,
    pub contracts: Vec<SpecAnchor>,
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
    pub origin_closure: OriginClosure,
    pub origin_closure_digest: String,
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
    pub plan_digest: String,
    pub slice_id: String,
    pub basis: PlanBasis,
    pub instructions: ContextInstructions,
    pub spec_context: Vec<SpecContextEntry>,
    pub artifact_context: Vec<ArtifactContextEntry>,
    pub completion: Vec<CompletionCheck>,
}

/// The provider-neutral execution envelope handed to an implementation tool.
/// It deliberately repeats the immutable plan identity so a tool cannot
/// accidentally reuse a context pack for another approved slice.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentContextPack {
    pub schema: String,
    pub plan_digest: String,
    pub slice_id: String,
    pub context: ContextPack,
    pub budget: SliceBudgetUsage,
    pub editable_targets: Vec<AgentTargetDigest>,
    pub verification_targets: Vec<AgentTargetDigest>,
    pub readonly_targets: Vec<AgentTargetDigest>,
}

impl AgentContextPack {
    pub fn from_slice(plan_digest: &str, context: ContextPack, slice: &ExecutionSlice) -> Self {
        let targets = |targets: &[PlannedTarget]| {
            targets
                .iter()
                .map(|target| AgentTargetDigest {
                    reference: target.reference.clone(),
                    path: target.resolved_path.clone(),
                    access: target.access,
                    transition: target.transition,
                    lifecycle: target.lifecycle,
                    content_hash: target.content_hash.clone(),
                    excerpt_hash: target.excerpt_hash.clone(),
                    container_content_hash: target.container_content_hash.clone(),
                    line_start: target.line_start,
                    line_end: target.line_end,
                    budget_bytes: target.budget_bytes,
                    budget_lines: target.budget_lines,
                })
                .collect()
        };
        Self {
            schema: AGENT_CONTEXT_SCHEMA.into(),
            plan_digest: plan_digest.into(),
            slice_id: slice.id.clone(),
            context,
            budget: slice.budget.clone(),
            editable_targets: targets(&slice.editable_targets),
            verification_targets: targets(&slice.verification_targets),
            readonly_targets: targets(&slice.readonly_context),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentTargetDigest {
    #[serde(rename = "ref")]
    pub reference: BoundTargetRef,
    pub path: String,
    pub access: TargetAccessMode,
    pub transition: TargetTransition,
    #[serde(default)]
    #[serde(skip_serializing_if = "is_stable_target_lifecycle")]
    pub lifecycle: TargetLifecycle,
    pub content_hash: String,
    pub excerpt_hash: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub container_content_hash: Option<String>,
    pub line_start: usize,
    pub line_end: usize,
    pub budget_bytes: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub budget_lines: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AgentRunStatus {
    Active,
    Blocked,
    Completed,
    Abandoned,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AgentPatchStatus {
    Accepted,
    Rejected,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind", deny_unknown_fields)]
pub enum AgentTargetWrite {
    Replace {
        #[serde(rename = "ref")]
        target: BoundTargetRef,
        expected_excerpt_hash: String,
        content: String,
    },
    /// Add one exact semantic target to an existing approved file. The
    /// container digest binds the insertion to the reviewed file state.
    AddToFile {
        #[serde(rename = "ref")]
        target: BoundTargetRef,
        expected_path_hash: String,
        content: String,
    },
    /// Create one approved file that did not exist when the plan was
    /// approved. Existing paths are always rejected rather than overwritten.
    CreateFile {
        #[serde(rename = "ref")]
        target: BoundTargetRef,
        content: String,
    },
    /// Remove one exact semantic target while proving its reviewed excerpt is
    /// still current.
    Remove {
        #[serde(rename = "ref")]
        target: BoundTargetRef,
        expected_excerpt_hash: String,
    },
    /// Remove one approved file while proving its reviewed full-file digest is
    /// still current.
    RemoveFile {
        #[serde(rename = "ref")]
        target: BoundTargetRef,
        expected_content_hash: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentPatch {
    pub schema: String,
    pub run_id: String,
    pub expected_workspace_fingerprint: String,
    pub writes: Vec<AgentTargetWrite>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentBlocker {
    pub code: String,
    pub message: String,
    pub next_action: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScopeExpansionRequest {
    pub request_id: String,
    pub run_id: String,
    pub plan_digest: String,
    pub slice_id: String,
    pub reason: String,
    pub requested_targets: Vec<BoundTargetRef>,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentRun {
    pub schema: String,
    pub run_id: String,
    pub approval_id: String,
    pub plan_digest: String,
    pub slice_id: String,
    pub status: AgentRunStatus,
    pub context: AgentContextPack,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentPatchRecord {
    pub schema: String,
    pub patch_id: String,
    pub run_id: String,
    pub plan_digest: String,
    pub slice_id: String,
    pub status: AgentPatchStatus,
    pub writes: Vec<AgentTargetWrite>,
    pub changes: Vec<AgentTargetChange>,
    pub before_workspace_fingerprint: String,
    pub after_workspace_fingerprint: String,
    pub blockers: Vec<AgentBlocker>,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TargetLifecycleProof {
    #[serde(rename = "ref")]
    pub reference: BoundTargetRef,
    #[serde(default)]
    pub transition: TargetTransition,
    #[serde(default)]
    #[serde(skip_serializing_if = "is_stable_target_lifecycle")]
    pub lifecycle: TargetLifecycle,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub before_content_hash: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub after_content_hash: String,
    pub before_excerpt_hash: String,
    pub after_excerpt_hash: String,
}

/// Agent patches retain exactly the same lifecycle proof that completion and
/// finalization use, so the execution event and durable closure evidence stay
/// connected.
pub type AgentTargetChange = TargetLifecycleProof;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum AgentEventKind {
    RunStarted { run: Box<AgentRun> },
    PatchRecorded { patch: AgentPatchRecord },
    BlockerRecorded { blocker: AgentBlocker },
    ScopeExpansionRequested { request: ScopeExpansionRequest },
    VerificationRecorded { attempt_id: String },
    RunAbandoned { reason: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentEvent {
    pub schema: String,
    pub event_id: String,
    pub event_digest: String,
    pub run_id: String,
    pub plan_digest: String,
    pub slice_id: String,
    pub created_at: String,
    pub event: AgentEventKind,
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
    /// Exact lifecycle proof for every editable target in the completed slice.
    pub lifecycle_proofs: Vec<TargetLifecycleProof>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VerificationExecution {
    pub target: BoundTargetRef,
    /// v3 receipts carry the criterion-specific claim explicitly. `null` is
    /// retained for a verification execution that is not claim-bound.
    pub claim: Option<VerificationClaimRef>,
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
    #[serde(default)]
    pub attempt_id: String,
    pub plan_digest: String,
    pub slice_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub receipt_digest: Option<String>,
    pub status: CompletionStatus,
    pub demonstrated: Vec<CompletionCriterionEvidence>,
    pub checks: Vec<CompletionCheckEvidence>,
    pub blockers: Vec<CompletionBlocker>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum VerificationAttemptStatus {
    Complete,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VerificationExecutionAttempt {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<BoundTargetRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub claim: Option<VerificationClaimRef>,
    pub runner: String,
    pub command: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stdout_digest: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stderr_digest: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proof: Option<ExactTestEvidence>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VerificationAttemptFailure {
    pub code: String,
    pub message: String,
    pub next_action: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VerificationAttemptResult {
    pub status: VerificationAttemptStatus,
    pub executions: Vec<VerificationExecutionAttempt>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failure: Option<VerificationAttemptFailure>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlanApproval {
    pub schema: String,
    pub approval_id: String,
    pub plan_digest: String,
    pub slice_id: String,
    pub workspace_fingerprint: String,
    pub revision: String,
    pub reviewed_at: String,
    pub plan: WorkPlan,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompletionAttempt {
    pub schema: String,
    pub attempt_id: String,
    pub attempt_digest: String,
    pub plan_digest: String,
    pub slice_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_run_id: Option<String>,
    pub approved_plan_digest: String,
    pub started_at: String,
    pub completed_at: String,
    pub verification: VerificationAttemptResult,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub receipt: Option<VerificationReceipt>,
    pub report: CompletionReport,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FinalizationPreview {
    pub schema: String,
    pub attempt_id: String,
    pub attempt_digest: String,
    pub plan_digest: String,
    pub slice_id: String,
    pub preview_token: String,
    pub status: CompletionStatus,
    pub pre_workspace_fingerprint: String,
    pub promoted_items: Vec<SpecItemRef>,
    pub changed_files: Vec<String>,
    pub blockers: Vec<CompletionBlocker>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FinalizationReceipt {
    pub schema: String,
    pub finalization_id: String,
    pub finalization_digest: String,
    pub attempt_id: String,
    pub attempt_digest: String,
    pub plan_digest: String,
    pub slice_id: String,
    pub pre_workspace_fingerprint: String,
    pub post_workspace_fingerprint: String,
    pub promoted_items: Vec<SpecItemRef>,
    pub changed_files: Vec<String>,
    /// The validated target lifecycle evidence preserved from the completion
    /// receipt that authorized this finalization.
    pub lifecycle_proofs: Vec<TargetLifecycleProof>,
    pub completed_at: String,
}

/// Return a stable digest of immutable readonly and run-only targets. This is
/// the guarded execution boundary used by post-state plan validation;
/// editable source and derived generated content are deliberately omitted.
pub fn readonly_targets_fingerprint(slices: &[ExecutionSlice]) -> String {
    readonly_targets_fingerprint_excluding_paths(slices, &std::collections::BTreeSet::new())
}

/// Lifecycle writes may legitimately change the containing file of a readonly
/// context target (for example, adding a new symbol beside an existing one).
/// Excluding only those approved paths keeps the readonly guard strict for all
/// unrelated targets while allowing the plan's own Add/Remove transition.
pub fn readonly_targets_fingerprint_for_execution(slices: &[ExecutionSlice]) -> String {
    let lifecycle_paths = slices
        .iter()
        .flat_map(|slice| slice.editable_targets.iter())
        .filter(|target| {
            matches!(
                target.transition,
                TargetTransition::Add | TargetTransition::Remove
            )
        })
        .map(|target| target.resolved_path.clone())
        .collect();
    readonly_targets_fingerprint_excluding_paths(slices, &lifecycle_paths)
}

fn readonly_targets_fingerprint_excluding_paths(
    slices: &[ExecutionSlice],
    excluded_paths: &std::collections::BTreeSet<String>,
) -> String {
    let mut hash = Sha256::new();
    for slice in slices {
        hash.update(slice.id.as_bytes());
        for target in slice
            .verification_targets
            .iter()
            .chain(slice.readonly_context.iter())
            .filter(|target| {
                matches!(
                    target.access,
                    TargetAccessMode::Readonly | TargetAccessMode::RunOnly
                ) && !excluded_paths.contains(&target.resolved_path)
            })
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
            if let Some(claim) = &target.verification_claim {
                hash.update(claim.target.to_string().as_bytes());
                hash.update(claim.criterion.to_string().as_bytes());
            }
        }
    }
    format_sha256(hash.finalize())
}

pub fn work_plan_digest(plan: &WorkPlan) -> String {
    let value = serde_json::json!({
        "request": {
            "schema": plan.request.schema,
            "operation": plan.request.operation,
            "origin": plan.request.origin,
            "constraints": plan.request.constraints,
            "requested_targets": plan.request.requested_targets.iter().map(requested_target_digest_value).collect::<Vec<_>>(),
        },
        "basis": plan.basis,
        "execution": plan.execution,
        "origin_closure": plan.origin_closure,
        "origin_closure_digest": plan.origin_closure_digest,
        "slices": plan.slices.iter().map(execution_slice_digest_value).collect::<Vec<_>>(),
        "diagnostics": plan.diagnostics,
    });
    let bytes = canonical_json_bytes(value);
    let mut hash = Sha256::new();
    hash.update(b"syu/work-plan-digest/v1\0");
    hash.update(bytes);
    format_sha256(hash.finalize())
}

fn requested_target_digest_value(target: &RequestedTarget) -> serde_json::Value {
    let access = match target.transition {
        TargetTransition::RunOnly => TargetAccessMode::RunOnly,
        TargetTransition::Readonly => TargetAccessMode::Readonly,
        TargetTransition::Add | TargetTransition::Modify | TargetTransition::Remove => {
            TargetAccessMode::Editable
        }
    };
    let lifecycle = match target.transition {
        TargetTransition::Add => TargetLifecycle::EnsurePresent,
        TargetTransition::Remove => TargetLifecycle::EnsureAbsent,
        TargetTransition::Modify | TargetTransition::RunOnly | TargetTransition::Readonly => {
            TargetLifecycle::Stable
        }
    };
    serde_json::json!({
        "reference": target.reference,
        "transition": target.transition,
        "access": access,
        "lifecycle": lifecycle,
    })
}

fn planned_target_digest_value(target: &PlannedTarget) -> serde_json::Value {
    serde_json::json!({
        "reference": target.reference,
        "verification_claim": target.verification_claim,
        "artifact_identity": target.artifact_identity,
        "transition": target.transition,
        "lifecycle": target.lifecycle,
        "access": target.access,
        "resolved_path": target.resolved_path,
        "resolved_selector": target.resolved_selector,
        "content_hash": target.content_hash,
        "excerpt_hash": target.excerpt_hash,
        "container_content_hash": target.container_content_hash,
        "adapter": target.adapter,
        "facet": target.facet,
        "role": target.role,
        "byte_start": target.byte_start,
        "byte_end": target.byte_end,
        "line_start": target.line_start,
        "line_end": target.line_end,
        "budget_bytes": target.budget_bytes,
        "budget_lines": target.budget_lines,
        "reason": target.reason,
    })
}

fn execution_slice_digest_value(slice: &ExecutionSlice) -> serde_json::Value {
    serde_json::json!({
        "id": slice.id,
        "goal": slice.goal,
        "anchors": slice.anchors,
        "editable_targets": slice.editable_targets.iter().map(planned_target_digest_value).collect::<Vec<_>>(),
        "verification_targets": slice.verification_targets.iter().map(planned_target_digest_value).collect::<Vec<_>>(),
        "readonly_context": slice.readonly_context.iter().map(planned_target_digest_value).collect::<Vec<_>>(),
        "acceptance": slice.acceptance,
        "contracts": slice.contracts,
        "non_goals": slice.non_goals,
        "completion": slice.completion,
        "budget": slice.budget,
        "confidence": slice.confidence,
        "blockers": slice.blockers,
    })
}

pub fn canonical_json_bytes(value: serde_json::Value) -> Vec<u8> {
    fn normalize(value: serde_json::Value) -> serde_json::Value {
        match value {
            serde_json::Value::Object(object) => {
                let mut entries = object.into_iter().collect::<Vec<_>>();
                entries.sort_by(|left, right| left.0.cmp(&right.0));
                serde_json::Value::Object(
                    entries
                        .into_iter()
                        .map(|(key, value)| (key, normalize(value)))
                        .collect(),
                )
            }
            serde_json::Value::Array(values) => {
                // Array order is part of the meaning for command arguments,
                // patch writes, executions, and other ordered evidence. Set-
                // like fields must sort themselves before calling this
                // generic canonicalizer; silently sorting every array would
                // let distinct execution plans share a digest.
                serde_json::Value::Array(values.into_iter().map(normalize).collect())
            }
            other => other,
        }
    }
    serde_json::to_vec(&normalize(value)).expect("serialize canonical JSON")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requested_target_wire_shape_is_closed_and_derived() {
        let target = RequestedTarget {
            reference: "FEAT-TEST-001#binding.implementation/target.handler"
                .parse()
                .expect("target reference"),
            criterion: Some(
                "REQ-TEST-001#criterion.behavior"
                    .parse()
                    .expect("criterion anchor"),
            ),
            transition: TargetTransition::Modify,
        };
        let value = serde_json::to_value(&target).expect("serialize requested target");
        assert_eq!(
            value,
            serde_json::json!({
                "access": "editable",
                "lifecycle": "stable",
                "reference": "FEAT-TEST-001#binding.implementation/target.handler",
                "transition": "modify"
            })
        );
        assert!(
            serde_json::from_value::<RequestedTarget>(serde_json::json!({
                "reference": "FEAT-TEST-001#binding.implementation/target.handler",
                "transition": "modify",
                "access": "editable",
                "lifecycle": "stable",
                "criterion": "REQ-TEST-001#criterion.behavior"
            }))
            .is_err()
        );
        assert!(
            serde_json::from_value::<RequestedTarget>(serde_json::json!({
                "ref": "FEAT-TEST-001#binding.implementation/target.handler",
                "transition": "modify",
                "access": "editable",
                "lifecycle": "stable"
            }))
            .is_err()
        );
        assert!(
            serde_json::from_value::<RequestedTarget>(serde_json::json!({
                "reference": "FEAT-TEST-001#binding.implementation/target.handler",
                "transition": "modify",
                "access": "readonly",
                "lifecycle": "stable"
            }))
            .is_err()
        );
    }

    #[test]
    fn work_request_rejects_the_removed_seed_boundary() {
        let legacy = serde_json::json!({
            "schema": WORK_REQUEST_SCHEMA,
            "id": "WORK-LEGACY",
            "title": "legacy",
            "operation": "modify",
            "origin": { "kind": "requirement-criterion", "criterion": "REQ-TEST-001#criterion.behavior" },
            "seeds": [{ "kind": "anchor", "anchor": "REQ-TEST-001#criterion.behavior" }],
            "constraints": {},
            "requested_targets": []
        });
        assert!(serde_json::from_value::<WorkRequest>(legacy).is_err());
    }

    #[test]
    fn canonical_digest_domains_have_literal_vectors() {
        assert_eq!(
            canonical_json_bytes(serde_json::json!({
                "z": { "b": 2, "a": 1 },
                "a": [3, 1, 2]
            })),
            br#"{"a":[3,1,2],"z":{"a":1,"b":2}}"#
        );
        assert_ne!(
            canonical_json_bytes(serde_json::json!(["cargo", "test"])),
            canonical_json_bytes(serde_json::json!(["test", "cargo"])),
        );
        assert_eq!(
            execution_identity_digest(&ExecutionIdentity {
                plan_digest: "sha256:plan".into(),
                slice_id: "slice-1".into(),
            }),
            "sha256:1b160a8b0208ad66de7d8500e7e6f6428804f45aabf5234e663eea29d49a3419"
        );
    }
}
