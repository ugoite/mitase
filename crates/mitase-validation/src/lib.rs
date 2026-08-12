#![forbid(unsafe_code)]
mod readiness;
use anyhow::{Context, Result, bail};
use mitase_diagnostics::{Diagnostic, ValidationPhase, ValidationResult};
use mitase_planner::{plan as canonical_plan, validate_work_origin};
use mitase_project_model::{
    ProjectConfig, ReadinessLevel, ValidationPreset, VerificationRunnerAdapter,
};
use mitase_spec_model::format_sha256;
use mitase_spec_model::{
    ArtifactTarget, ArtifactTargetLifecycle, BindingRole, BoundTargetRef, ItemStatus,
    LocalAnchorKind, OwnershipSelector, RepoPath, RuleLevel, Selector, SpecAnchor, SpecDocument,
    TargetClaim, VerificationRunnerRef,
};
use mitase_work_model::{
    COMPLETION_REPORT_SCHEMA, CompletionBlocker, CompletionCheck, CompletionCheckEvidence,
    CompletionCriterionEvidence, CompletionReport, CompletionStatus, ExecutionSlice,
    PlanConfidence, PlanExecution, TargetAccessMode, TargetLifecycle, TargetTransition,
    VERIFICATION_PROOF_SCHEMA, VERIFICATION_RECEIPT_SCHEMA, VerificationAttemptFailure,
    VerificationAttemptResult, VerificationAttemptStatus, VerificationClaimRef,
    VerificationExecution, VerificationExecutionAttempt, VerificationProof,
    VerificationProofStatus, VerificationReceipt, WORK_PLAN_SCHEMA, WorkPlan,
    readonly_targets_fingerprint_for_execution, work_plan_digest,
};
use mitase_workspace::{
    AnchorValue, ResolvedTarget, SpecIndex, SpecWorkspace, resolve_artifact_unit,
    resolve_indexed_target, resolve_target_in_workspace, selector_supports_editable,
};
pub use readiness::{
    ReadinessAxis, ReadinessAxisId, ReadinessReport, ReadinessSubject,
    evaluate as evaluate_readiness, required_axes,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};
use tempfile::TempDir;

#[derive(Debug, Clone, Copy)]
pub enum OverridePolicy {
    FixedError,
    Suppressible,
}

#[derive(Debug, Clone, Copy)]
pub struct RuleMetadata {
    pub id: &'static str,
    pub title: &'static str,
    pub default_error: bool,
    pub override_policy: OverridePolicy,
    pub presets: &'static [ValidationPreset],
}
macro_rules! metadata {
    ($id:literal) => {
        RuleMetadata {
            id: $id,
            title: $id,
            default_error: true,
            override_policy: OverridePolicy::Suppressible,
            presets: &[
                ValidationPreset::Standard,
                ValidationPreset::Strict,
                ValidationPreset::AgentReady,
            ],
        }
    };
}
macro_rules! strict_metadata {
    ($id:literal) => {
        RuleMetadata {
            id: $id,
            title: $id,
            default_error: true,
            override_policy: OverridePolicy::Suppressible,
            presets: &[ValidationPreset::Strict, ValidationPreset::AgentReady],
        }
    };
}
macro_rules! fixed_metadata {
    ($id:literal) => {
        RuleMetadata {
            id: $id,
            title: $id,
            default_error: true,
            override_policy: OverridePolicy::FixedError,
            presets: &[
                ValidationPreset::Standard,
                ValidationPreset::Strict,
                ValidationPreset::AgentReady,
            ],
        }
    };
}
pub static RULES: &[RuleMetadata] = &[
    fixed_metadata!("MITASE-SCHEMA-001"),
    fixed_metadata!("MITASE-SCHEMA-002"),
    fixed_metadata!("MITASE-ID-001"),
    fixed_metadata!("MITASE-ID-002"),
    fixed_metadata!("MITASE-ANCHOR-001"),
    fixed_metadata!("MITASE-ANCHOR-002"),
    fixed_metadata!("MITASE-ANCHOR-003"),
    metadata!("MITASE-PHILOSOPHY-001"),
    metadata!("MITASE-POLICY-001"),
    metadata!("MITASE-POLICY-002"),
    metadata!("MITASE-POLICY-003"),
    metadata!("MITASE-REQUIREMENT-001"),
    metadata!("MITASE-REQUIREMENT-002"),
    metadata!("MITASE-FEATURE-001"),
    fixed_metadata!("MITASE-FEATURE-002"),
    fixed_metadata!("MITASE-FEATURE-003"),
    metadata!("MITASE-BINDING-001"),
    metadata!("MITASE-BINDING-002"),
    metadata!("MITASE-BINDING-003"),
    fixed_metadata!("MITASE-BINDING-004"),
    fixed_metadata!("MITASE-TARGET-001"),
    fixed_metadata!("MITASE-TARGET-002"),
    fixed_metadata!("MITASE-TARGET-003"),
    fixed_metadata!("MITASE-TARGET-004"),
    fixed_metadata!("MITASE-TARGET-005"),
    metadata!("MITASE-FACET-001"),
    metadata!("MITASE-FACET-002"),
    fixed_metadata!("MITASE-CONTRACT-001"),
    fixed_metadata!("MITASE-CONTRACT-002"),
    fixed_metadata!("MITASE-CONTRACT-003"),
    metadata!("MITASE-CONTRACT-004"),
    fixed_metadata!("MITASE-CONTRACT-005"),
    metadata!("MITASE-CONTRACT-006"),
    fixed_metadata!("MITASE-CONTRACT-007"),
    metadata!("MITASE-DOC-001"),
    metadata!("MITASE-DOC-002"),
    fixed_metadata!("MITASE-GENERATED-001"),
    fixed_metadata!("MITASE-GENERATED-002"),
    metadata!("MITASE-OPERATION-001"),
    strict_metadata!("MITASE-CHANGE-001"),
    strict_metadata!("MITASE-CHANGE-002"),
    strict_metadata!("MITASE-CHANGE-003"),
    strict_metadata!("MITASE-CHANGE-004"),
    strict_metadata!("MITASE-CHANGE-005"),
    metadata!("MITASE-WORK-001"),
    metadata!("MITASE-WORK-002"),
    fixed_metadata!("MITASE-WORK-003"),
    metadata!("MITASE-WORK-004"),
    fixed_metadata!("MITASE-WORK-005"),
    fixed_metadata!("MITASE-WORK-006"),
    fixed_metadata!("MITASE-WORK-007"),
    fixed_metadata!("MITASE-WORK-008"),
    fixed_metadata!("MITASE-WORK-009"),
    fixed_metadata!("MITASE-WORK-010"),
    fixed_metadata!("MITASE-WORK-011"),
    fixed_metadata!("MITASE-WORK-012"),
    fixed_metadata!("MITASE-READINESS-001"),
    fixed_metadata!("MITASE-VERIFICATION-001"),
    fixed_metadata!("MITASE-VERIFICATION-002"),
];

/// Canonical rule-to-phase classification for presentation clients.  This is
/// intentionally kept beside the validator so no caller needs to infer
/// semantics from a rule-id string.
pub fn phase_for_rule(rule: &str) -> ValidationPhase {
    if rule.starts_with("MITASE-WORK-") || rule.starts_with("MITASE-READINESS-") {
        ValidationPhase::Plan
    } else if rule.starts_with("MITASE-CHANGE-") || rule == "MITASE-OPERATION-001" {
        ValidationPhase::Scope
    } else if [
        "MITASE-BINDING-",
        "MITASE-TARGET-",
        "MITASE-CONTRACT-",
        "MITASE-FACET-",
        "MITASE-GENERATED-",
        "MITASE-VERIFICATION-",
    ]
    .iter()
    .any(|prefix| rule.starts_with(prefix))
    {
        ValidationPhase::Targets
    } else if [
        "MITASE-ID-",
        "MITASE-ANCHOR-",
        "MITASE-PHILOSOPHY-",
        "MITASE-POLICY-",
        "MITASE-REQUIREMENT-",
        "MITASE-FEATURE-",
        "MITASE-DOC-",
    ]
    .iter()
    .any(|prefix| rule.starts_with(prefix))
    {
        ValidationPhase::Graph
    } else {
        ValidationPhase::Config
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ChangeStatus {
    Added,
    Modified,
    Deleted,
    Renamed,
    Untracked,
    Binary,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangedRange {
    pub old_start: usize,
    pub old_end: usize,
    pub new_start: usize,
    pub new_end: usize,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangedFile {
    pub status: ChangeStatus,
    pub old_path: Option<RepoPath>,
    pub new_path: Option<RepoPath>,
    pub hunks: Vec<ChangedRange>,
}

/// Return the complete working-tree diff against a plan basis revision,
/// including staged, unstaged, and untracked files. Result validation uses
/// this canonical representation so callers cannot omit a changed artifact
/// by reporting only a hand-picked file list.
pub fn changed_files_against_revision(root: &Path, revision: &str) -> Result<Vec<ChangedFile>> {
    let status_output = Command::new("git")
        .args(["-C"])
        .arg(root)
        .args(["diff", "--name-status", "-z", "-M", "--relative", revision])
        .output()?;
    if !status_output.status.success() {
        bail!("git diff --name-status {revision} failed");
    }
    let untracked_output = Command::new("git")
        .args(["-C"])
        .arg(root)
        .args(["ls-files", "--others", "--exclude-standard", "-z"])
        .output()?;
    if !untracked_output.status.success() {
        bail!("git ls-files --others failed");
    }
    let patch_output = Command::new("git")
        .args(["-C"])
        .arg(root)
        .args([
            "diff",
            "-M",
            "--relative",
            "--unified=0",
            "--no-color",
            revision,
        ])
        .output()?;
    if !patch_output.status.success() {
        bail!("git diff --unified=0 {revision} failed");
    }
    let mut files = parse_changed_files(
        &status_output.stdout,
        &untracked_output.stdout,
        &String::from_utf8(patch_output.stdout)?,
    )?;
    synthesize_untracked_ranges(root, &mut files)?;
    Ok(files)
}

fn parse_changed_files(status: &[u8], untracked: &[u8], patch: &str) -> Result<Vec<ChangedFile>> {
    let mut files = Vec::new();
    let mut entries = status
        .split(|byte| *byte == 0)
        .filter(|entry| !entry.is_empty());
    while let Some(kind) = entries.next() {
        let kind = String::from_utf8(kind.to_vec())?;
        let (status, old_path, new_path) = match kind.chars().next().unwrap_or('M') {
            'A' => (
                ChangeStatus::Added,
                None,
                Some(String::from_utf8(
                    entries.next().context("missing added path")?.to_vec(),
                )?),
            ),
            'M' => {
                let path =
                    String::from_utf8(entries.next().context("missing modified path")?.to_vec())?;
                (ChangeStatus::Modified, Some(path.clone()), Some(path))
            }
            'D' => (
                ChangeStatus::Deleted,
                Some(String::from_utf8(
                    entries.next().context("missing deleted path")?.to_vec(),
                )?),
                None,
            ),
            'R' => (
                ChangeStatus::Renamed,
                Some(String::from_utf8(
                    entries.next().context("missing rename source")?.to_vec(),
                )?),
                Some(String::from_utf8(
                    entries.next().context("missing rename target")?.to_vec(),
                )?),
            ),
            _ => {
                let path = entries
                    .next()
                    .map(|value| String::from_utf8(value.to_vec()))
                    .transpose()?;
                (ChangeStatus::Modified, path.clone(), path)
            }
        };
        files.push(ChangedFile {
            status,
            old_path: old_path
                .map(RepoPath::new)
                .transpose()
                .map_err(anyhow::Error::msg)?,
            new_path: new_path
                .map(RepoPath::new)
                .transpose()
                .map_err(anyhow::Error::msg)?,
            hunks: Vec::new(),
        });
    }
    for path in untracked
        .split(|byte| *byte == 0)
        .filter(|entry| !entry.is_empty())
    {
        files.push(ChangedFile {
            status: ChangeStatus::Untracked,
            old_path: None,
            new_path: Some(
                RepoPath::new(String::from_utf8(path.to_vec())?).map_err(anyhow::Error::msg)?,
            ),
            hunks: Vec::new(),
        });
    }
    let mut current: Option<usize> = None;
    for line in patch.lines() {
        if line.starts_with("diff --git ") {
            current = None;
            continue;
        }
        if let Some(path) = line.strip_prefix("+++ b/") {
            current = files.iter().position(|file| {
                file.new_path
                    .as_ref()
                    .is_some_and(|value| value.to_string_lossy() == path)
            });
            continue;
        }
        if line == "+++ /dev/null" {
            continue;
        }
        if let Some(path) = line.strip_prefix("--- a/") {
            current = files.iter().position(|file| {
                file.old_path
                    .as_ref()
                    .is_some_and(|value| value.to_string_lossy() == path)
            });
            continue;
        }
        if let Some((old_path, new_path)) = parse_binary_patch_paths(line) {
            if let Some(index) = files.iter().position(|file| {
                file.old_path
                    .as_ref()
                    .is_some_and(|value| value.to_string_lossy() == old_path)
                    || file
                        .new_path
                        .as_ref()
                        .is_some_and(|value| value.to_string_lossy() == new_path)
            }) {
                files[index].status = ChangeStatus::Binary;
            }
            current = None;
            continue;
        }
        if let Some(hunk) = line.strip_prefix("@@ ")
            && let Some(index) = current
        {
            files[index].hunks.push(parse_hunk_header(hunk)?);
        }
    }
    Ok(files)
}

fn synthesize_untracked_ranges(root: &Path, files: &mut [ChangedFile]) -> Result<()> {
    for file in files {
        if file.status != ChangeStatus::Untracked || !file.hunks.is_empty() {
            continue;
        }
        let Some(path) = file.new_path.as_ref() else {
            continue;
        };
        let contents = match fs::read(root.join(path.as_path())) {
            Ok(contents) => contents,
            Err(_) => continue,
        };
        if std::str::from_utf8(&contents).is_err() {
            continue;
        }
        let line_count = contents.iter().filter(|byte| **byte == b'\n').count()
            + usize::from(!contents.is_empty() && *contents.last().unwrap_or(&b'\n') != b'\n');
        file.hunks.push(ChangedRange {
            old_start: 0,
            old_end: 0,
            new_start: 1,
            new_end: line_count.max(1),
        });
    }
    Ok(())
}

fn parse_binary_patch_paths(line: &str) -> Option<(&str, &str)> {
    let paths = line.strip_prefix("Binary files ")?;
    let (old_path, remainder) = paths.split_once(" and ")?;
    let new_path = remainder.strip_suffix(" differ")?;
    let old_path = old_path.strip_prefix("a/").unwrap_or(old_path);
    let new_path = new_path.strip_prefix("b/").unwrap_or(new_path);
    Some((old_path, new_path))
}

fn parse_hunk_header(header: &str) -> Result<ChangedRange> {
    let range = header
        .split(" @@")
        .next()
        .context("malformed hunk header")?;
    let mut parts = range.split_whitespace();
    let old = parts.next().context("missing old hunk range")?;
    let new = parts.next().context("missing new hunk range")?;
    let (old_start, old_end) = parse_diff_span(old.trim_start_matches('-'))?;
    let (new_start, new_end) = parse_diff_span(new.trim_start_matches('+'))?;
    Ok(ChangedRange {
        old_start,
        old_end,
        new_start,
        new_end,
    })
}

fn parse_diff_span(span: &str) -> Result<(usize, usize)> {
    let (start, len) = span
        .split_once(',')
        .map_or((span, "1"), |(start, len)| (start, len));
    let start = start.parse::<usize>()?;
    let len = len.parse::<usize>()?;
    if len == 0 {
        return Ok((0, 0));
    }
    Ok((start, start + len - 1))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanValidationMode {
    PreState,
    PostState,
}

pub struct ValidationContext<'a> {
    pub config: &'a ProjectConfig,
    pub workspace: &'a SpecWorkspace,
    pub index: &'a SpecIndex,
    pub changed_files: Option<&'a [ChangedFile]>,
    pub reported_changed_files: Option<&'a [ChangedFile]>,
    pub work_plan: Option<&'a WorkPlan>,
    pub selected_slice: Option<&'a ExecutionSlice>,
    pub plan_mode: PlanValidationMode,
    pub preset: ValidationPreset,
    pub revision: Option<&'a str>,
    pub change_base_revision: Option<&'a str>,
}

pub trait ValidationRule {
    fn metadata(&self) -> &'static RuleMetadata;
    fn evaluate(&self, ctx: &ValidationContext<'_>, out: &mut Vec<Diagnostic>);
}

pub fn validate(ctx: &ValidationContext<'_>) -> ValidationResult {
    // Workspace validation is a structural operation. External verification
    // is an explicit POST/readiness action and must never be reached through
    // preview, overlay, or ordinary plan validation.
    validate_inner(ctx, false)
}

/// Validate a canonical workspace and enforce its configured readiness target.
/// This is the explicit CLI workspace command; Workbench previews continue to
/// use `validate_without_readiness` so previews never execute external tests.
pub fn validate_workspace(ctx: &ValidationContext<'_>) -> ValidationResult {
    validate_inner(ctx, true)
}

pub fn validate_without_readiness(ctx: &ValidationContext<'_>) -> ValidationResult {
    validate_inner(ctx, false)
}

/// Rebuild and validate a submitted plan immediately before any verification
/// command is allowed to run. Callers must provide the plan they received
/// from the canonical planner; a serialized `status: ready` is never trusted.
pub fn canonical_plan_for_execution(
    workspace: &SpecWorkspace,
    index: &SpecIndex,
    submitted: &WorkPlan,
    revision: &str,
) -> Result<WorkPlan> {
    if index.inventory_error.is_some() {
        bail!("verification is blocked because inventory failed");
    }
    if submitted.schema != WORK_PLAN_SCHEMA {
        bail!("plan schema must be {WORK_PLAN_SCHEMA}");
    }
    mitase_planner::validate_work_request(index, &submitted.request)
        .context("submitted plan contains targets outside its exact Work origin")?;
    validate_work_origin(index, &submitted.request.origin)
        .context("submitted plan requires an exact implemented Work origin")?;
    if submitted.basis.revision != revision {
        bail!("plan basis revision is stale");
    }
    if submitted.basis.spec_fingerprint != workspace.spec_fingerprint()? {
        bail!("plan specification basis is stale");
    }
    if submitted.basis.ownership_fingerprint != lifecycle_ownership_fingerprint(index, submitted) {
        bail!("plan ownership basis is stale");
    }
    if submitted.basis.readonly_fingerprint
        != current_readonly_fingerprint(workspace, index, submitted)
    {
        bail!("plan readonly or run-only target basis is stale");
    }
    if submitted.canonical_digest != work_plan_digest(submitted) {
        bail!("plan canonical digest is tampered");
    }
    let current_fingerprint = workspace.try_fingerprint()?;
    if submitted.basis.workspace_fingerprint == current_fingerprint
        && !plan_has_lifecycle_transition(submitted)
    {
        let mut canonical = canonical_plan(&submitted.request, workspace, index, revision)?;
        canonical.basis = submitted.basis.clone();
        canonical.canonical_digest = work_plan_digest(&canonical);
        if canonical != *submitted {
            bail!("submitted plan does not match deterministic canonical planner output");
        }
    } else {
        let baseline = load_workspace_at_revision(&workspace.root, &submitted.basis.revision)
            .ok_or_else(|| anyhow::anyhow!("cannot reconstruct the work-plan basis workspace"))?;
        let mut canonical = canonical_plan(
            &submitted.request,
            &baseline.workspace,
            &baseline.index,
            revision,
        )?;
        canonical.basis = submitted.basis.clone();
        canonical.canonical_digest = work_plan_digest(&canonical);
        if canonical != *submitted {
            bail!("submitted plan does not match deterministic canonical planner output");
        }
    }
    if !matches!(submitted.status, mitase_work_model::PlanStatus::Ready) {
        bail!("verification requires a ready canonical plan");
    }
    if submitted.slices.is_empty() {
        bail!("verification requires at least one canonical slice");
    }
    Ok(submitted.clone())
}

fn lifecycle_ownership_fingerprint(index: &SpecIndex, plan: &WorkPlan) -> String {
    index.ownership_fingerprint_excluding(
        &plan
            .slices
            .iter()
            .flat_map(|slice| slice.editable_targets.iter())
            .filter(|target| {
                matches!(
                    target.transition,
                    TargetTransition::Add | TargetTransition::Remove
                )
            })
            .map(|target| target.reference.clone())
            .collect(),
    )
}

fn plan_has_lifecycle_transition(plan: &WorkPlan) -> bool {
    plan.slices
        .iter()
        .flat_map(|slice| &slice.editable_targets)
        .any(|target| {
            matches!(
                target.transition,
                TargetTransition::Add | TargetTransition::Remove
            )
        })
}

fn current_readonly_fingerprint(
    workspace: &SpecWorkspace,
    index: &SpecIndex,
    plan: &WorkPlan,
) -> String {
    let mut slices = plan.slices.clone();
    for slice in &mut slices {
        for target in slice
            .verification_targets
            .iter_mut()
            .chain(slice.readonly_context.iter_mut())
            .filter(|target| {
                matches!(
                    target.access,
                    TargetAccessMode::Readonly | TargetAccessMode::RunOnly
                )
            })
        {
            match resolve_planned_target_for_workspace(workspace, index, target) {
                Some(resolved) => {
                    target.resolved_path = resolved.path.to_string_lossy().into_owned();
                    target.resolved_selector.description = resolved.description;
                    target.resolved_selector.symbols = resolved.symbols;
                    target.content_hash = resolved.content_hash;
                    target.excerpt_hash = resolved.excerpt_hash;
                }
                None if target.lifecycle == TargetLifecycle::EnsureAbsent => {}
                None if target.access == mitase_work_model::TargetAccessMode::RunOnly
                    && target.transition == mitase_work_model::TargetTransition::Add
                    && index.item_status.get(&target.reference.binding.item)
                        == Some(&mitase_spec_model::ItemStatus::Planned) => {}
                None => {
                    // Preserve a deterministic mismatch for a missing stable
                    // readonly/run-only target. Ensure-absent transitions are
                    // the only missing target state that is valid here.
                    target.content_hash = "missing".into();
                    target.excerpt_hash = "missing".into();
                }
            }
        }
    }
    readonly_targets_fingerprint_for_execution(&slices)
}

pub(crate) fn resolve_planned_target_for_workspace(
    workspace: &SpecWorkspace,
    index: &SpecIndex,
    target: &mitase_work_model::PlannedTarget,
) -> Option<ResolvedTarget> {
    if let Some(identity) = &target.artifact_identity {
        let unit = index
            .artifact_units
            .iter()
            .find(|unit| &unit.identity == identity)?;
        return resolve_artifact_unit(workspace, unit).ok();
    }
    let declared = index.target(&target.reference)?;
    if let Some(identity) = index.target_to_artifact.get(&target.reference)
        && let Some(unit) = index
            .artifact_units
            .iter()
            .find(|unit| &unit.identity == identity)
        && let Ok(Some(resolved)) = resolve_indexed_target(workspace, declared, unit)
    {
        return Some(resolved);
    }
    resolve_target_in_workspace(workspace, declared).ok()
}

/// Resolve the one claim selected by a plan or receipt. A verification target
/// may serve several criteria, but every execution must name precisely one.
pub(crate) fn resolve_verification_claim<'a>(
    index: &'a SpecIndex,
    claim: &VerificationClaimRef,
) -> Result<(
    &'a ArtifactTarget,
    &'a VerificationRunnerRef,
    &'a [BoundTargetRef],
)> {
    let target = index
        .target(&claim.target)
        .ok_or_else(|| anyhow::anyhow!("verification target {} is unresolved", claim.target))?;
    let mut matching = target.claims.iter().filter_map(|entry| match entry {
        TargetClaim::Verifies {
            criterion,
            covers,
            runner,
        } if criterion == &claim.criterion => Some((runner, covers.as_slice())),
        _ => None,
    });
    let Some((runner, covers)) = matching.next() else {
        bail!(
            "verification target {} has no claim for criterion {}",
            claim.target,
            claim.criterion
        );
    };
    if matching.next().is_some() {
        bail!(
            "verification target {} has multiple claims for criterion {}",
            claim.target,
            claim.criterion
        );
    }
    if covers.is_empty() {
        bail!(
            "verification target {} has no covers for criterion {}",
            claim.target,
            claim.criterion
        );
    }
    Ok((target, runner, covers))
}

/// Execute exactly the verification claims selected by a canonical slice.
/// Runner executable and arguments come only from the workspace registry and
/// selected claim; no planner or caller guesses a command.
pub fn execute_verification(
    workspace: &SpecWorkspace,
    index: &SpecIndex,
    submitted: &WorkPlan,
    slice_id: &str,
    revision: &str,
) -> Result<VerificationReceipt> {
    let plan = canonical_plan_for_execution(workspace, index, submitted, revision)?;
    let slice = plan
        .slices
        .iter()
        .find(|slice| slice.id == slice_id)
        .ok_or_else(|| anyhow::anyhow!("slice {slice_id} not found"))?;
    if slice.verification_targets.is_empty() {
        bail!("selected slice has no verification targets");
    }
    let plan_mode = if plan.basis.workspace_fingerprint == workspace.try_fingerprint()? {
        PlanValidationMode::PreState
    } else {
        PlanValidationMode::PostState
    };
    let pre_state = validate_without_readiness(&ValidationContext {
        config: &workspace.config,
        workspace,
        index,
        changed_files: None,
        reported_changed_files: None,
        work_plan: Some(&plan),
        selected_slice: Some(slice),
        plan_mode,
        preset: workspace.config.validation.preset,
        revision: Some(revision),
        change_base_revision: None,
    });
    if pre_state
        .diagnostics
        .iter()
        .any(|diagnostic| matches!(diagnostic.severity, mitase_diagnostics::Severity::Error))
    {
        bail!(
            "canonical plan validation failed before verification: {}",
            pre_state
                .diagnostics
                .iter()
                .map(|diagnostic| diagnostic.message.clone())
                .collect::<Vec<_>>()
                .join("; ")
        );
    }

    let started_at = epoch_seconds();
    let mut executions = Vec::with_capacity(slice.verification_targets.len());
    for planned in &slice.verification_targets {
        let claim = planned.verification_claim.as_ref().ok_or_else(|| {
            anyhow::anyhow!(
                "verification target {} is missing its selected claim",
                planned.reference
            )
        })?;
        if claim.target != planned.reference {
            bail!(
                "verification target {} does not match its selected claim target {}",
                planned.reference,
                claim.target
            );
        }
        let (target, runner_ref, covers) = resolve_verification_claim(index, claim)?;
        let configured = workspace
            .config
            .verification
            .runners
            .get(&runner_ref.runner)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "verification runner {} is not configured",
                    runner_ref.runner
                )
            })?;
        if configured
            .arguments
            .iter()
            .any(|argument| has_unresolved_runner_placeholder(argument, &runner_ref.arguments))
        {
            bail!(
                "verification runner {} has unresolved arguments",
                runner_ref.runner
            );
        }
        let arguments = configured
            .arguments
            .iter()
            .map(|argument| {
                expand_runner_argument_for_adapter(
                    configured.adapter,
                    argument,
                    &runner_ref.arguments,
                )
            })
            .collect::<Vec<_>>();
        let arguments = canonical_runner_arguments_for_adapter(configured.adapter, arguments);
        if runner_ref
            .arguments
            .get("test")
            .is_none_or(|identity| identity.is_empty())
        {
            bail!("verification claim must name the exact test identity");
        }
        validate_exact_claim_identity(
            configured.adapter,
            target,
            &runner_ref.arguments,
            Some(&workspace.root),
        )?;
        require_exact_runner_filter(configured.adapter, &arguments, &runner_ref.arguments)?;
        let mut command = Command::new(&configured.executable);
        command.args(&arguments).current_dir(&workspace.root);
        if configured.adapter == VerificationRunnerAdapter::Pytest {
            // Do not let ambient pytest configuration add selectors or disable
            // capture after the exact command has been validated.
            command.env_remove("PYTEST_ADDOPTS");
        }
        if configured.adapter == VerificationRunnerAdapter::CargoLibtest
            && arguments.iter().any(|argument| argument == "-Z")
        {
            command.env("RUSTC_BOOTSTRAP", "1");
        }
        if configured.adapter == VerificationRunnerAdapter::CargoLibtest {
            // Reuse one target directory for all exact verification jobs in a
            // workspace. Keep an explicitly supplied target directory (the
            // test runner uses this to stay off the source volume); otherwise
            // use a workspace-specific temporary directory so verification
            // never mutates or nests a target directory inside a temporary
            // clone's source tree.
            let target_root = std::env::var_os("CARGO_TARGET_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|| std::env::temp_dir().join("mitase-verification"));
            // Keep each workspace (including temporary self-hosted clones)
            // isolated even when the caller supplies one shared target root.
            // `sha256:` is part of Mitase's digest wire format, but `:` is a
            // library-path separator on Unix.
            let target_dir = target_root.join(
                digest(workspace.root.to_string_lossy().as_bytes()).trim_start_matches("sha256:"),
            );
            command.env("CARGO_TARGET_DIR", target_dir);
        }
        let output = command
            .output()
            .with_context(|| format!("execute verification runner {}", runner_ref.runner))?;
        if !output.status.success() {
            bail!(
                "verification runner {} failed with exit code {}; exact-test proof unavailable",
                runner_ref.runner,
                output.status.code().unwrap_or(-1),
            );
        }
        let proof = ensure_exact_test_executed(
            configured.adapter,
            target,
            &runner_ref.arguments,
            Some(&workspace.root),
            &arguments,
            &output.stdout,
        )?;
        let mut implementation_digests = BTreeMap::new();
        for covered in covers {
            implementation_digests.insert(
                covered.clone(),
                implementation_digest_for_receipt(workspace, index, slice, covered)?,
            );
        }
        let verification = mitase_workspace::resolve_target_in_workspace(workspace, target)?;
        executions.push(VerificationExecution {
            target: planned.reference.clone(),
            claim: Some(claim.clone()),
            runner: runner_ref.runner.clone(),
            command: std::iter::once(configured.executable.clone())
                .chain(arguments)
                .collect(),
            exit_code: output.status.code().unwrap_or(-1),
            stdout_digest: digest(&output.stdout),
            stderr_digest: digest(&output.stderr),
            proof,
            implementation_digests,
            verification_digest: verification.content_hash,
        });
    }
    let receipt = VerificationReceipt {
        schema: VERIFICATION_RECEIPT_SCHEMA.into(),
        plan_digest: plan.canonical_digest.clone(),
        slice_id: slice_id.into(),
        revision: revision.into(),
        workspace_fingerprint: workspace.try_fingerprint()?,
        started_at,
        completed_at: epoch_seconds(),
        executions,
        lifecycle_proofs: target_lifecycle_proofs(workspace, index, slice)?,
    };
    validate_verification_receipt(workspace, index, &plan, slice_id, &receipt, revision)?;
    Ok(receipt)
}

/// Execute verification while preserving every expected execution failure as
/// a structured completion result. Callers persist the returned value as one
/// immutable attempt instead of dropping runner errors through propagation.
pub fn execute_verification_attempt(
    workspace: &SpecWorkspace,
    index: &SpecIndex,
    submitted: &WorkPlan,
    slice_id: &str,
    revision: &str,
    attempt_id: &str,
) -> Result<(
    VerificationAttemptResult,
    Option<VerificationReceipt>,
    CompletionReport,
)> {
    let result = match execute_verification(workspace, index, submitted, slice_id, revision) {
        Ok(receipt) => {
            let mut report = match evaluate_completion(workspace, index, submitted, &receipt) {
                Ok(report) => report,
                Err(error) => CompletionReport {
                    schema: COMPLETION_REPORT_SCHEMA.into(),
                    attempt_id: attempt_id.into(),
                    plan_digest: submitted.canonical_digest.clone(),
                    slice_id: slice_id.into(),
                    receipt_digest: None,
                    status: CompletionStatus::Blocked,
                    demonstrated: vec![],
                    checks: vec![],
                    blockers: vec![CompletionBlocker {
                        code: "MITASE-COMPLETION-EVALUATION".into(),
                        message: durable_failure_message(&error),
                        next_action:
                            "Resolve the completion evaluation failure, then retry the same approved plan and slice."
                                .into(),
                    }],
                },
            };
            report.attempt_id = attempt_id.into();
            report.receipt_digest = Some(verification_receipt_digest(&receipt)?);
            let executions = receipt
                .executions
                .iter()
                .map(|execution| VerificationExecutionAttempt {
                    target: Some(execution.target.clone()),
                    claim: execution.claim.clone(),
                    runner: execution.runner.clone(),
                    command: execution.command.clone(),
                    exit_code: Some(execution.exit_code),
                    stdout_digest: Some(execution.stdout_digest.clone()),
                    stderr_digest: Some(execution.stderr_digest.clone()),
                    proof: Some(execution.proof.clone()),
                    error: None,
                })
                .collect();
            (
                VerificationAttemptResult {
                    status: VerificationAttemptStatus::Complete,
                    executions,
                    failure: None,
                },
                Some(receipt),
                report,
            )
        }
        Err(error) => {
            let failure = VerificationAttemptFailure {
                code: "MITASE-VERIFICATION-FAILED".into(),
                message: durable_failure_message(&error),
                next_action:
                    "Resolve the verification failure, then retry the same approved plan and slice."
                        .into(),
            };
            let report = CompletionReport {
                schema: COMPLETION_REPORT_SCHEMA.into(),
                attempt_id: attempt_id.into(),
                plan_digest: submitted.canonical_digest.clone(),
                slice_id: slice_id.into(),
                receipt_digest: None,
                status: CompletionStatus::Blocked,
                demonstrated: vec![],
                checks: vec![],
                blockers: vec![CompletionBlocker {
                    code: failure.code.clone(),
                    message: failure.message.clone(),
                    next_action: failure.next_action.clone(),
                }],
            };
            (
                VerificationAttemptResult {
                    status: VerificationAttemptStatus::Failed,
                    executions: vec![],
                    failure: Some(failure),
                },
                None,
                report,
            )
        }
    };
    Ok(result)
}

fn durable_failure_message(error: &anyhow::Error) -> String {
    let message = error.to_string();
    let summary = message
        .split_once("\nstdout:\n")
        .map_or(message.as_str(), |(summary, _)| summary);
    const MAX_CHARS: usize = 4_096;
    let mut bounded = summary.chars().take(MAX_CHARS).collect::<String>();
    if summary.chars().count() > MAX_CHARS {
        bounded.push('…');
    }
    if summary.len() != message.len() {
        bounded.push_str("; runner output omitted from durable evidence");
    }
    bounded
}

pub fn validate_verification_receipt(
    workspace: &SpecWorkspace,
    index: &SpecIndex,
    plan: &WorkPlan,
    slice_id: &str,
    receipt: &VerificationReceipt,
    revision: &str,
) -> Result<()> {
    let canonical = canonical_plan_for_execution(workspace, index, plan, revision)?;
    let slice = canonical
        .slices
        .iter()
        .find(|slice| slice.id == slice_id)
        .ok_or_else(|| anyhow::anyhow!("slice {slice_id} not found"))?;
    if receipt.schema != VERIFICATION_RECEIPT_SCHEMA
        || receipt.plan_digest != canonical.canonical_digest
        || receipt.slice_id != slice_id
        || receipt.revision != revision
        || receipt.workspace_fingerprint != workspace.try_fingerprint()?
    {
        bail!("verification receipt basis is stale or does not match the selected slice");
    }
    if receipt.lifecycle_proofs != target_lifecycle_proofs(workspace, index, slice)? {
        bail!("verification receipt lifecycle proof is stale or incomplete");
    }
    let expected = slice
        .verification_targets
        .iter()
        .map(|target| {
            target.verification_claim.clone().ok_or_else(|| {
                anyhow::anyhow!(
                    "verification target {} is missing its selected claim",
                    target.reference
                )
            })
        })
        .collect::<Result<BTreeSet<_>>>()?;
    let actual = receipt
        .executions
        .iter()
        .map(|execution| {
            execution.claim.clone().ok_or_else(|| {
                anyhow::anyhow!(
                    "verification receipt execution {} is missing its selected claim",
                    execution.target
                )
            })
        })
        .collect::<Result<Vec<_>>>()?;
    if actual.len() != expected.len() || actual.into_iter().collect::<BTreeSet<_>>() != expected {
        bail!("verification receipt execution set is not exact");
    }
    for execution in &receipt.executions {
        if execution.exit_code != 0 {
            bail!("verification receipt contains failed executions");
        }
        validate_verification_proof(&execution.proof)?;
        let claim = execution.claim.as_ref().ok_or_else(|| {
            anyhow::anyhow!(
                "verification receipt execution {} is missing its selected claim",
                execution.target
            )
        })?;
        if execution.target != claim.target {
            bail!("verification receipt target does not match its selected claim");
        }
        let (target, runner_ref, covers) = resolve_verification_claim(index, claim)?;
        let configured = workspace
            .config
            .verification
            .runners
            .get(&runner_ref.runner)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "verification runner {} is not configured",
                    runner_ref.runner
                )
            })?;
        if configured
            .arguments
            .iter()
            .any(|argument| has_unresolved_runner_placeholder(argument, &runner_ref.arguments))
        {
            bail!(
                "verification runner {} has unresolved arguments",
                runner_ref.runner
            );
        }
        let arguments = configured
            .arguments
            .iter()
            .map(|argument| {
                expand_runner_argument_for_adapter(
                    configured.adapter,
                    argument,
                    &runner_ref.arguments,
                )
            })
            .collect::<Vec<_>>();
        let arguments = canonical_runner_arguments_for_adapter(configured.adapter, arguments);
        if runner_ref
            .arguments
            .get("test")
            .is_none_or(|identity| identity.is_empty())
        {
            bail!("verification claim must name the exact test identity");
        }
        validate_exact_claim_identity(
            configured.adapter,
            target,
            &runner_ref.arguments,
            Some(&workspace.root),
        )?;
        require_exact_runner_filter(configured.adapter, &arguments, &runner_ref.arguments)?;
        let expected_command = std::iter::once(configured.executable.clone())
            .chain(arguments)
            .collect::<Vec<_>>();
        if execution.runner != runner_ref.runner
            || execution.command != expected_command
            || execution.command.is_empty()
        {
            bail!("verification receipt command does not match the configured runner");
        }
        let verification = mitase_workspace::resolve_target_in_workspace(workspace, target)?;
        if execution.verification_digest != verification.content_hash {
            bail!("verification target digest is stale");
        }
        for covered in covers {
            let digest = implementation_digest_for_receipt(workspace, index, slice, covered)?;
            if execution.implementation_digests.get(covered) != Some(&digest) {
                bail!("verification implementation digest is stale");
            }
        }
        if execution.implementation_digests.len() != covers.len() {
            bail!("receipt implementation digest set is not exact");
        }
        if execution.proof.identity
            != runner_ref
                .arguments
                .get("test")
                .cloned()
                .unwrap_or_default()
            || execution.proof.identity.is_empty()
        {
            bail!("verification receipt exact-test proof is stale");
        }
    }
    Ok(())
}

/// Produce durable proof that every editable target reached the lifecycle
/// state approved by the selected slice. The before hashes come from the
/// immutable plan; the after hashes come from the candidate workspace used by
/// verification and finalization.
fn target_lifecycle_proofs(
    workspace: &SpecWorkspace,
    index: &SpecIndex,
    slice: &ExecutionSlice,
) -> Result<Vec<mitase_work_model::TargetLifecycleProof>> {
    let mut proofs = Vec::with_capacity(slice.editable_targets.len());
    for target in &slice.editable_targets {
        let current = resolve_planned_target_for_workspace(workspace, index, target);
        let proof = match (target.lifecycle, current) {
            (TargetLifecycle::EnsureAbsent, None) => mitase_work_model::TargetLifecycleProof {
                reference: target.reference.clone(),
                transition: target.transition,
                lifecycle: target.lifecycle,
                before_content_hash: target.content_hash.clone(),
                after_content_hash: String::new(),
                before_excerpt_hash: target.excerpt_hash.clone(),
                after_excerpt_hash: String::new(),
            },
            (TargetLifecycle::EnsureAbsent, Some(_)) => {
                bail!("target {} remains after verification", target.reference)
            }
            (_, Some(resolved)) => mitase_work_model::TargetLifecycleProof {
                reference: target.reference.clone(),
                transition: target.transition,
                lifecycle: target.lifecycle,
                before_content_hash: target.content_hash.clone(),
                after_content_hash: resolved.content_hash,
                before_excerpt_hash: target.excerpt_hash.clone(),
                after_excerpt_hash: resolved.excerpt_hash,
            },
            (_, None) => bail!("target {} is absent after verification", target.reference),
        };
        proofs.push(proof);
    }
    Ok(proofs)
}

fn implementation_digest_for_receipt(
    workspace: &SpecWorkspace,
    index: &SpecIndex,
    slice: &ExecutionSlice,
    covered: &BoundTargetRef,
) -> Result<String> {
    let lifecycle = slice
        .editable_targets
        .iter()
        .find(|target| target.reference == *covered);
    let declared = index
        .target(covered)
        .ok_or_else(|| anyhow::anyhow!("covered target {covered} is unresolved"))?;
    let resolved = mitase_workspace::resolve_target_in_workspace(workspace, declared).ok();
    if lifecycle.is_some_and(|target| target.lifecycle == TargetLifecycle::EnsureAbsent) {
        if resolved.is_some() {
            bail!("covered removal target {covered} remains in the workspace");
        }
        return Ok(String::new());
    }
    resolved
        .map(|target| target.content_hash)
        .ok_or_else(|| anyhow::anyhow!("covered target {covered} is unresolved"))
}

/// Evaluate the complete post-change closure for one execution slice.
///
/// This is intentionally the shared completion contract used by CLI callers
/// and future Workbench/agent callers. Expected user errors are represented in
/// the report so callers can render a precise blocker and next action instead
/// of losing the reason behind an anyhow error.
pub fn evaluate_completion(
    workspace: &SpecWorkspace,
    index: &SpecIndex,
    submitted: &WorkPlan,
    receipt: &VerificationReceipt,
) -> Result<CompletionReport> {
    let current_revision = repository_revision(&workspace.root)?;
    let blocked = |plan_digest: String, blockers: Vec<CompletionBlocker>| CompletionReport {
        schema: COMPLETION_REPORT_SCHEMA.into(),
        attempt_id: String::new(),
        plan_digest,
        slice_id: receipt.slice_id.clone(),
        receipt_digest: None,
        status: CompletionStatus::Blocked,
        demonstrated: vec![],
        checks: vec![],
        blockers,
    };

    let canonical = match canonical_plan_for_execution(
        workspace,
        index,
        submitted,
        &current_revision,
    ) {
        Ok(plan) => plan,
        Err(error) => {
            return Ok(blocked(
                submitted.canonical_digest.clone(),
                vec![completion_blocker(
                    "MITASE-COMPLETION-PLAN",
                    format!("cannot use the submitted work plan: {error}"),
                    "Regenerate the work plan from the original request, then rerun verification.",
                )],
            ));
        }
    };
    let Some(slice) = canonical
        .slices
        .iter()
        .find(|slice| slice.id == receipt.slice_id)
    else {
        return Ok(blocked(
            canonical.canonical_digest.clone(),
            vec![completion_blocker(
                "MITASE-COMPLETION-SLICE",
                format!(
                    "receipt slice {} is absent from the canonical plan",
                    receipt.slice_id
                ),
                "Select a slice from the current plan and rerun exact verification.",
            )],
        ));
    };

    let mut blockers = Vec::new();
    let receipt_valid = if let Err(error) = validate_verification_receipt(
        workspace,
        index,
        &canonical,
        &receipt.slice_id,
        receipt,
        &current_revision,
    ) {
        blockers.push(completion_blocker(
            "MITASE-COMPLETION-RECEIPT",
            format!("verification receipt is not a valid closure: {error}"),
            "Rerun the exact verification command for this unchanged plan and slice.",
        ));
        false
    } else {
        true
    };

    let changed_files =
        match changed_files_against_revision(&workspace.root, &canonical.basis.revision) {
            Ok(files) => files,
            Err(error) => {
                blockers.push(completion_blocker(
                "MITASE-COMPLETION-DIFF",
                format!("post-state diff could not be reconstructed: {error}"),
                "Restore the plan basis revision or regenerate the plan before completing work.",
            ));
                vec![]
            }
        };
    let post_state = validate(&ValidationContext {
        config: &workspace.config,
        workspace,
        index,
        changed_files: Some(&changed_files),
        reported_changed_files: None,
        work_plan: Some(&canonical),
        selected_slice: Some(slice),
        plan_mode: PlanValidationMode::PostState,
        preset: workspace.config.validation.preset,
        revision: Some(&current_revision),
        change_base_revision: Some(&canonical.basis.revision),
    });
    for diagnostic in post_state
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.severity == mitase_diagnostics::Severity::Error)
    {
        blockers.push(completion_blocker(
            diagnostic.rule_id.clone(),
            diagnostic.message.clone(),
            next_action_for_rule(&diagnostic.rule_id),
        ));
    }
    blockers.extend(readiness_regression_blockers(
        workspace,
        index,
        &canonical,
        &current_revision,
    )?);

    let mut checks = Vec::new();
    for check in &slice.completion {
        let (passed, evidence) = completion_check_result(check, workspace, index, &post_state);
        if !passed {
            blockers.push(completion_blocker(
                "MITASE-COMPLETION-CHECK",
                format!("completion check is not satisfied: {check:?}"),
                "Resolve the listed check, then rerun validation and exact verification.",
            ));
        }
        checks.push(CompletionCheckEvidence {
            check: check.clone(),
            passed,
            evidence,
        });
    }

    let demonstrated = if receipt_valid {
        acceptance_evidence(slice, receipt, &mut blockers)
    } else {
        vec![]
    };
    blockers.sort_by(|a, b| {
        (&a.code, &a.message, &a.next_action).cmp(&(&b.code, &b.message, &b.next_action))
    });
    blockers.dedup();
    checks.sort_by(|a, b| format!("{:?}", a.check).cmp(&format!("{:?}", b.check)));
    let status = if blockers.is_empty() {
        CompletionStatus::Complete
    } else {
        CompletionStatus::Blocked
    };
    Ok(CompletionReport {
        schema: COMPLETION_REPORT_SCHEMA.into(),
        attempt_id: String::new(),
        plan_digest: canonical.canonical_digest,
        slice_id: receipt.slice_id.clone(),
        receipt_digest: Some(verification_receipt_digest(receipt)?),
        status,
        demonstrated,
        checks,
        blockers,
    })
}

fn completion_blocker(
    code: impl Into<String>,
    message: impl Into<String>,
    next_action: impl Into<String>,
) -> CompletionBlocker {
    CompletionBlocker {
        code: code.into(),
        message: message.into(),
        next_action: next_action.into(),
    }
}

fn next_action_for_rule(rule: &str) -> &'static str {
    match rule {
        "MITASE-WORK-005" => "Revert readonly or run-only changes, then rerun completion.",
        "MITASE-WORK-006" => {
            "Revert out-of-scope changes or create a new explicitly approved plan."
        }
        "MITASE-WORK-009" => "Regenerate the plan and rerun exact verification.",
        "MITASE-WORK-011" => {
            "Complete the expected target lifecycle transition, then rerun validation."
        }
        "MITASE-READINESS-001" => "Restore the readiness baseline before completing the slice.",
        _ => "Resolve the validation diagnostic, then rerun validation and exact verification.",
    }
}

fn completion_check_result(
    check: &CompletionCheck,
    workspace: &SpecWorkspace,
    index: &SpecIndex,
    post_state: &ValidationResult,
) -> (bool, Vec<String>) {
    match check {
        CompletionCheck::TargetExists { target } => {
            let exists = index
                .target(target)
                .and_then(|declared| resolve_target_in_workspace(workspace, declared).ok())
                .is_some();
            (exists, vec![format!("target {target} exists: {exists}")])
        }
        CompletionCheck::TargetAbsent { target } => {
            let absent = index
                .target(target)
                .and_then(|declared| resolve_target_in_workspace(workspace, declared).ok())
                .is_none();
            (absent, vec![format!("target {target} absent: {absent}")])
        }
        CompletionCheck::DiffWithinScope => {
            let passed = !post_state.diagnostics.iter().any(|diagnostic| {
                diagnostic.severity == mitase_diagnostics::Severity::Error
                    && matches!(
                        diagnostic.rule_id.as_str(),
                        "MITASE-WORK-005" | "MITASE-WORK-006"
                    )
            });
            (passed, vec![format!("scope diagnostics clear: {passed}")])
        }
        CompletionCheck::Validate { preset } => {
            let passed = post_state.is_valid();
            (
                passed,
                vec![format!("validation preset {preset} passed: {passed}")],
            )
        }
        CompletionCheck::ContractConsistent { contract } => {
            let passed = !post_state.diagnostics.iter().any(|diagnostic| {
                diagnostic.severity == mitase_diagnostics::Severity::Error
                    && diagnostic.rule_id.starts_with("MITASE-CONTRACT")
                    && diagnostic.anchor.as_ref() == Some(contract)
            });
            (
                passed,
                vec![format!("contract {contract} consistent: {passed}")],
            )
        }
        CompletionCheck::Command { .. } | CompletionCheck::RuleSet { .. } => (
            false,
            vec!["this completion check has no canonical execution adapter".into()],
        ),
    }
}

fn acceptance_evidence(
    slice: &ExecutionSlice,
    receipt: &VerificationReceipt,
    blockers: &mut Vec<CompletionBlocker>,
) -> Vec<CompletionCriterionEvidence> {
    let executed = receipt
        .executions
        .iter()
        .filter_map(|execution| execution.claim.clone())
        .collect::<BTreeSet<_>>();
    let mut demonstrated = Vec::new();
    for acceptance in &slice.acceptance {
        let verification_targets = slice
            .verification_targets
            .iter()
            .filter(|planned| {
                planned.verification_claim.as_ref().is_some_and(|claim| {
                    claim.criterion == acceptance.anchor && executed.contains(claim)
                })
            })
            .map(|planned| planned.reference.clone())
            .collect::<Vec<_>>();
        if verification_targets.is_empty() {
            blockers.push(completion_blocker(
                "MITASE-COMPLETION-CRITERION",
                format!(
                    "acceptance criterion {} has no exact verification evidence",
                    acceptance.anchor
                ),
                "Add or select the required verification target, then rerun exact verification.",
            ));
        } else {
            demonstrated.push(CompletionCriterionEvidence {
                anchor: acceptance.anchor.clone(),
                statement: acceptance.statement.clone(),
                verification_targets,
            });
        }
    }
    demonstrated
}

fn readiness_regression_blockers(
    workspace: &SpecWorkspace,
    index: &SpecIndex,
    plan: &WorkPlan,
    current_revision: &str,
) -> Result<Vec<CompletionBlocker>> {
    let Some(basis) = load_workspace_at_revision(&workspace.root, &plan.basis.revision) else {
        if plan.basis.workspace_fingerprint == workspace.try_fingerprint()? {
            return Ok(vec![]);
        }
        return Ok(vec![completion_blocker(
            "MITASE-READINESS-002",
            "the plan readiness baseline cannot be reconstructed",
            "Restore the plan basis revision or regenerate the plan before completing work.",
        )]);
    };
    let before = evaluate_readiness(&basis.workspace, &basis.index, &plan.basis.revision, false)?;
    let after = evaluate_readiness(workspace, index, current_revision, false)?;
    let approved_removed_targets = plan
        .slices
        .iter()
        .flat_map(|slice| slice.editable_targets.iter())
        .filter(|target| target.lifecycle == TargetLifecycle::EnsureAbsent)
        .map(|target| target.reference.clone())
        .collect::<BTreeSet<_>>();
    let approved_removed_artifacts = approved_removed_targets
        .iter()
        .filter_map(|target| basis.index.all_target_to_artifact.get(target))
        .map(|identity| identity.as_str())
        .collect::<BTreeSet<_>>();
    let approved_removed_inventory = approved_removed_artifacts
        .iter()
        .map(|identity| format!("inventory:{identity}"))
        .collect::<BTreeSet<_>>();
    let approved_removed_ownership = approved_removed_artifacts
        .iter()
        .map(|identity| format!("ownership:{identity}"))
        .collect::<BTreeSet<_>>();
    let approved_removed_features = approved_removed_targets
        .iter()
        .map(|target| format!("feature:{}", target.binding.item))
        .collect::<BTreeSet<_>>();
    let approved_removed_seedability = basis
        .index
        .criteria_to_implementation_targets
        .iter()
        .chain(basis.index.criteria_to_verification_targets.iter())
        .flat_map(|(criterion, targets)| {
            targets
                .iter()
                .filter(|target| approved_removed_targets.contains(*target))
                .map(move |target| format!("criterion:{criterion}/target:{target}"))
        })
        .collect::<BTreeSet<_>>();
    let axes = [
        ("inventory", &before.inventory, &after.inventory),
        ("ownership", &before.ownership, &after.ownership),
        ("seedability", &before.seedability, &after.seedability),
        ("workability", &before.workability, &after.workability),
        ("verification", &before.verification, &after.verification),
    ];
    let mut blockers = Vec::new();
    for (name, before_axis, after_axis) in axes {
        let before_ready = before_axis
            .subjects
            .iter()
            .filter(|subject| subject.ready && subject.blockers.is_empty())
            .map(|subject| subject.id.clone())
            .collect::<BTreeSet<_>>();
        let after_ready = after_axis
            .subjects
            .iter()
            .filter(|subject| subject.ready && subject.blockers.is_empty())
            .map(|subject| subject.id.clone())
            .collect::<BTreeSet<_>>();
        let regressed = before_ready
            .difference(&after_ready)
            .filter(|subject| match name {
                "inventory" => !approved_removed_inventory.contains(*subject),
                "ownership" => !approved_removed_ownership.contains(*subject),
                "seedability" => {
                    approved_removed_seedability
                        .iter()
                        .all(|target| !subject.starts_with(target))
                        && !approved_removed_features.contains(*subject)
                }
                _ => true,
            })
            .cloned()
            .collect::<Vec<_>>();
        if !regressed.is_empty() {
            blockers.push(completion_blocker(
                "MITASE-READINESS-002",
                format!("readiness axis {name} regressed for {}", regressed.join(", ")),
                "Restore the regressed ownership, trace, or verification evidence before completing the slice.",
            ));
        }
    }
    Ok(blockers)
}

/// Validate the runner-neutral proof contract shared by every receipt.
pub fn validate_verification_proof(proof: &VerificationProof) -> Result<()> {
    if proof.schema != VERIFICATION_PROOF_SCHEMA {
        bail!("verification proof schema is unsupported");
    }
    if proof.identity.trim().is_empty() {
        bail!("verification proof identity is empty");
    }
    if proof.status != VerificationProofStatus::Passed || proof.matched_count != 1 {
        bail!("verification proof must report one matched test with passed status");
    }
    Ok(())
}

fn ensure_exact_test_executed(
    adapter: VerificationRunnerAdapter,
    target: &mitase_spec_model::ArtifactTarget,
    claim_arguments: &BTreeMap<String, String>,
    workspace_root: Option<&Path>,
    runner_arguments: &[String],
    stdout: &[u8],
) -> Result<VerificationProof> {
    let Some(test_identity) = claim_arguments.get("test") else {
        bail!("verification claim must name the exact test identity");
    };
    validate_exact_claim_identity(adapter, target, claim_arguments, workspace_root)?;
    let (matched_count, failed) = match adapter {
        VerificationRunnerAdapter::CargoLibtest => {
            parse_cargo_libtest_output(test_identity, stdout)?
        }
        VerificationRunnerAdapter::Pytest => {
            parse_pytest_output(test_identity, runner_arguments, stdout)?
        }
        VerificationRunnerAdapter::NodeTest => {
            let name = claim_arguments
                .get("name")
                .map(String::as_str)
                .or_else(|| test_identity.rsplit("::").next())
                .unwrap_or(test_identity);
            parse_node_test_output(name, stdout)
        }
        VerificationRunnerAdapter::GoTest => parse_go_test_output(test_identity, stdout),
        VerificationRunnerAdapter::Shell => {
            bail!("shell runners cannot produce runner-neutral verification proof")
        }
    };
    let proof = VerificationProof {
        schema: VERIFICATION_PROOF_SCHEMA.into(),
        identity: test_identity.clone(),
        matched_count,
        status: if failed || matched_count != 1 {
            VerificationProofStatus::Failed
        } else {
            VerificationProofStatus::Passed
        },
    };
    validate_verification_proof(&proof).map_err(|error| {
        anyhow::anyhow!(
            "configured verification command did not produce a valid proof for {test_identity}: {error}"
        )
    })?;
    Ok(proof)
}

fn exact_selector_name(target: &mitase_spec_model::ArtifactTarget) -> Option<&str> {
    match &target.selector {
        Selector::Symbol { name } => Some(name),
        Selector::File
        | Selector::Operation { .. }
        | Selector::Heading { .. }
        | Selector::JsonPointer { .. }
        | Selector::Marker { .. } => None,
    }
}

fn test_identity_matches_selector(identity: &str, selector: &str) -> bool {
    identity == selector
        || identity.ends_with(&format!("::{selector}"))
        || identity.ends_with(&format!("#{selector}"))
        || identity.ends_with(&format!(".{selector}"))
}

fn validate_exact_claim_identity(
    adapter: VerificationRunnerAdapter,
    target: &mitase_spec_model::ArtifactTarget,
    claim_arguments: &BTreeMap<String, String>,
    workspace_root: Option<&Path>,
) -> Result<()> {
    let test_identity = claim_arguments
        .get("test")
        .ok_or_else(|| anyhow::anyhow!("verification claim must name the exact test identity"))?;
    if let Some(selector_name) = exact_selector_name(target)
        && !test_identity_matches_selector(test_identity, selector_name)
    {
        bail!("verification argument {test_identity} does not identify selector {selector_name}");
    }
    match adapter {
        VerificationRunnerAdapter::CargoLibtest if exact_selector_name(target).is_none() => {
            bail!("cargo verification targets must use an exact symbol selector");
        }
        VerificationRunnerAdapter::Pytest => {
            let path = test_identity.split("::").next().unwrap_or(test_identity);
            require_claim_path_matches_target(adapter, target, path)?;
        }
        VerificationRunnerAdapter::NodeTest => {
            let path = claim_arguments.get("path").ok_or_else(|| {
                anyhow::anyhow!("node-test verification claim must name the exact test file")
            })?;
            let name = claim_arguments.get("name").ok_or_else(|| {
                anyhow::anyhow!("node-test verification claim must name the exact test title")
            })?;
            require_claim_path_matches_target(adapter, target, path)?;
            let expected_identity = format!("{path}::{name}");
            if test_identity != &expected_identity {
                bail!(
                    "node-test verification identity {test_identity} must match {expected_identity}"
                );
            }
            validate_node_test_file_identity(target, name, workspace_root)?;
        }
        VerificationRunnerAdapter::GoTest => {
            let package = claim_arguments.get("package").ok_or_else(|| {
                anyhow::anyhow!("go-test verification claim must name the exact package")
            })?;
            validate_go_package_identity(target, package, workspace_root)?;
            validate_go_test_file_identity(target, test_identity, workspace_root)?;
        }
        VerificationRunnerAdapter::CargoLibtest | VerificationRunnerAdapter::Shell => {}
    }
    Ok(())
}

fn validate_go_package_identity(
    target: &mitase_spec_model::ArtifactTarget,
    package: &str,
    workspace_root: Option<&Path>,
) -> Result<()> {
    let target_parent = Path::new(target.path.as_path())
        .parent()
        .map(|path| path.to_string_lossy().replace('\\', "/"))
        .unwrap_or_default();
    let package_path = package.trim_start_matches("./").trim_end_matches('/');
    let target_parent = target_parent.trim_start_matches("./");
    let is_local_package = package == "." || package.starts_with("./");
    if is_local_package
        && (package_path == target_parent || (package_path == "." && target_parent.is_empty()))
    {
        return Ok(());
    }

    if let Some(workspace_root) = workspace_root
        && let Ok(source) = fs::read_to_string(workspace_root.join("go.mod"))
        && let Some(module) = source.lines().find_map(|line| {
            line.strip_prefix("module ")
                .map(str::trim)
                .filter(|module| !module.is_empty())
        })
    {
        let expected = if target_parent.is_empty() {
            module.to_owned()
        } else {
            format!("{module}/{target_parent}")
        };
        if package == expected {
            return Ok(());
        }
    }

    bail!(
        "go-test package {package} does not identify verification target {}",
        target.path.to_string_lossy()
    )
}

fn validate_go_test_file_identity(
    target: &mitase_spec_model::ArtifactTarget,
    identity: &str,
    workspace_root: Option<&Path>,
) -> Result<()> {
    let Some(workspace_root) = workspace_root else {
        return Ok(());
    };
    if !target
        .path
        .as_path()
        .file_name()
        .is_some_and(|name| name.to_string_lossy().ends_with("_test.go"))
    {
        bail!(
            "go-test verification target {} must be a *_test.go file",
            target.path.to_string_lossy()
        );
    }
    let test_name = identity.split('/').next().unwrap_or(identity);
    let source_path = workspace_root.join(target.path.as_path());
    let source = fs::read_to_string(&source_path).with_context(|| {
        format!(
            "read Go verification target source {}",
            target.path.to_string_lossy()
        )
    })?;
    let declaration = format!("func {test_name}(");
    let declaration_with_space = format!("func {test_name} (");
    if !source.lines().any(|line| {
        let line = line.trim_start();
        line.starts_with(&declaration) || line.starts_with(&declaration_with_space)
    }) {
        bail!(
            "go-test identity {identity} is not defined in verification target {}",
            target.path.to_string_lossy()
        );
    }
    Ok(())
}

fn validate_node_test_file_identity(
    target: &mitase_spec_model::ArtifactTarget,
    name: &str,
    workspace_root: Option<&Path>,
) -> Result<()> {
    let Some(workspace_root) = workspace_root else {
        return Ok(());
    };
    let source_path = workspace_root.join(target.path.as_path());
    let source = fs::read_to_string(&source_path).with_context(|| {
        format!(
            "read Node verification target source {}",
            target.path.to_string_lossy()
        )
    })?;
    if !node_test_source_declares_name(&source, name) {
        bail!(
            "node-test identity {name} is not defined in verification target {}",
            target.path.to_string_lossy()
        );
    }
    Ok(())
}

fn node_test_source_declares_name(source: &str, expected_name: &str) -> bool {
    let bytes = source.as_bytes();
    let mut cursor = 0;
    while cursor < bytes.len() {
        if bytes[cursor] == b'/' && bytes.get(cursor + 1) == Some(&b'/') {
            cursor = source[cursor + 2..]
                .find('\n')
                .map(|offset| cursor + 2 + offset + 1)
                .unwrap_or(bytes.len());
            continue;
        }
        if bytes[cursor] == b'/' && bytes.get(cursor + 1) == Some(&b'*') {
            cursor = source[cursor + 2..]
                .find("*/")
                .map(|offset| cursor + 2 + offset + 2)
                .unwrap_or(bytes.len());
            continue;
        }
        if matches!(bytes[cursor], b'\'' | b'"' | b'`') {
            cursor = skip_node_js_string(source, cursor);
            continue;
        }
        if is_node_identifier_start(bytes[cursor]) {
            let start = cursor;
            cursor += 1;
            while cursor < bytes.len() && is_node_identifier_continue(bytes[cursor]) {
                cursor += 1;
            }
            let identifier = &source[start..cursor];
            if identifier != "test" && identifier != "it" {
                continue;
            }
            let mut argument_start = skip_node_js_whitespace(bytes, cursor);
            if bytes.get(argument_start) == Some(&b'.') {
                argument_start += 1;
                while argument_start < bytes.len()
                    && is_node_identifier_continue(bytes[argument_start])
                {
                    argument_start += 1;
                }
                argument_start = skip_node_js_whitespace(bytes, argument_start);
            }
            if bytes.get(argument_start) != Some(&b'(') {
                continue;
            }
            argument_start = skip_node_js_whitespace(bytes, argument_start + 1);
            if let Some((name, _)) = parse_node_js_string(source, argument_start)
                && name == expected_name
            {
                return true;
            }
            continue;
        }
        cursor += source[cursor..]
            .chars()
            .next()
            .map(char::len_utf8)
            .unwrap_or(1);
    }
    false
}

fn is_node_identifier_start(byte: u8) -> bool {
    byte.is_ascii_alphabetic() || byte == b'_' || byte == b'$'
}

fn is_node_identifier_continue(byte: u8) -> bool {
    is_node_identifier_start(byte) || byte.is_ascii_digit()
}

fn skip_node_js_whitespace(bytes: &[u8], mut cursor: usize) -> usize {
    while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
        cursor += 1;
    }
    cursor
}

fn skip_node_js_string(source: &str, cursor: usize) -> usize {
    parse_node_js_string(source, cursor)
        .map(|(_, end)| end)
        .unwrap_or_else(|| source.len())
}

fn parse_node_js_string(source: &str, cursor: usize) -> Option<(String, usize)> {
    let bytes = source.as_bytes();
    let quote = *bytes.get(cursor)?;
    if !matches!(quote, b'\'' | b'"' | b'`') {
        return None;
    }
    let mut value = String::new();
    let mut cursor = cursor + 1;
    while cursor < bytes.len() {
        match bytes[cursor] {
            byte if byte == quote => return Some((value, cursor + 1)),
            b'\\' => {
                cursor += 1;
                let escaped = *bytes.get(cursor)?;
                match escaped {
                    b'n' => value.push('\n'),
                    b'r' => value.push('\r'),
                    b't' => value.push('\t'),
                    b'\\' => value.push('\\'),
                    b'\'' => value.push('\''),
                    b'"' => value.push('"'),
                    other => value.push(other as char),
                }
                cursor += 1;
            }
            _ => {
                let character = source[cursor..].chars().next()?;
                value.push(character);
                cursor += character.len_utf8();
            }
        }
    }
    None
}

fn go_test_regex_fragment(identity: &str) -> String {
    identity
        .split('/')
        .map(regex_escape)
        .collect::<Vec<_>>()
        .join("$/^")
}

fn go_test_exact_pattern(identity: &str) -> String {
    format!("^{}$", go_test_regex_fragment(identity))
}

fn require_claim_path_matches_target(
    adapter: VerificationRunnerAdapter,
    target: &mitase_spec_model::ArtifactTarget,
    claim_path: &str,
) -> Result<()> {
    if claim_path != target.path.to_string_lossy() {
        bail!(
            "{} verification path {claim_path} does not match selected target {}",
            match adapter {
                VerificationRunnerAdapter::Pytest => "pytest",
                VerificationRunnerAdapter::NodeTest => "node-test",
                _ => "runner",
            },
            target.path.to_string_lossy()
        );
    }
    Ok(())
}

fn regex_escape(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        if matches!(
            character,
            '\\' | '^' | '$' | '.' | '*' | '+' | '?' | '(' | ')' | '[' | ']' | '{' | '}' | '|'
        ) {
            escaped.push('\\');
        }
        escaped.push(character);
    }
    escaped
}

fn require_exact_runner_filter(
    adapter: VerificationRunnerAdapter,
    arguments: &[String],
    claim_arguments: &BTreeMap<String, String>,
) -> Result<()> {
    let test_identity = claim_arguments
        .get("test")
        .ok_or_else(|| anyhow::anyhow!("verification claim must name the exact test identity"))?;
    match adapter {
        VerificationRunnerAdapter::CargoLibtest => {
            let separator = arguments
                .iter()
                .position(|argument| argument == "--")
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "cargo verification runner must provide an exact harness filter"
                    )
                })?;
            if !arguments[..separator]
                .iter()
                .any(|argument| argument == test_identity)
            {
                bail!(
                    "cargo verification runner must pass the exact test identity {test_identity} before --"
                );
            }
            if !arguments[separator + 1..]
                .iter()
                .any(|argument| argument == "--exact")
            {
                bail!("cargo verification runner must pass --exact to the test harness");
            }
        }
        VerificationRunnerAdapter::Pytest => {
            let mut matched_selectors = 0;
            let mut option_value = false;
            for argument in arguments {
                if option_value {
                    option_value = false;
                    continue;
                }
                if argument == test_identity {
                    matched_selectors += 1;
                    continue;
                }
                if argument.starts_with('-') {
                    option_value = pytest_option_takes_value(argument);
                    continue;
                }
                // Pytest accepts a path/node-id for every positional selector.
                // A second positional token would let unrelated tests execute
                // while the proof parser still finds the requested node.
                bail!(
                    "pytest verification runner must pass exactly one positional test selector; found {argument}"
                );
            }
            if matched_selectors != 1 {
                bail!(
                    "pytest verification runner must pass the exact test identity {test_identity} exactly once"
                );
            }
            validate_pytest_output_arguments(arguments)?;
        }
        VerificationRunnerAdapter::NodeTest => {
            if !arguments.iter().any(|argument| argument == "--test") {
                bail!("node-test verification runner must enable Node's --test mode");
            }
            let name = claim_arguments.get("name").ok_or_else(|| {
                anyhow::anyhow!("node-test verification claim must name the exact test title")
            })?;
            let path = claim_arguments.get("path").ok_or_else(|| {
                anyhow::anyhow!("node-test verification claim must name the exact test file")
            })?;
            let expected_identity = format!("{path}::{name}");
            if test_identity != &expected_identity {
                bail!(
                    "node-test verification identity {test_identity} must match {expected_identity}"
                );
            }
            let exact_name_pattern = format!("^{}$", regex_escape(name));
            let patterns = node_test_name_patterns(arguments);
            if patterns.len() != 1 || patterns[0] != exact_name_pattern {
                bail!("node-test verification runner must pass an exact test-name pattern");
            }
            let test_files = node_test_file_selectors(arguments);
            if test_files.len() != 1 || test_files[0] != path {
                bail!(
                    "node-test verification runner must pass exactly the selected test file {path}"
                );
            }
        }
        VerificationRunnerAdapter::GoTest => {
            let exact_pattern = go_test_exact_pattern(test_identity);
            let run_filters = go_test_run_filters(arguments);
            if run_filters.len() != 1 || run_filters[0] != exact_pattern {
                bail!("go-test verification runner must pass an exact -run filter");
            }
            if !arguments.iter().any(|argument| argument == "-json") {
                bail!("go-test verification runner must request JSON output");
            }
            let package = claim_arguments.get("package").ok_or_else(|| {
                anyhow::anyhow!("go-test verification claim must name the exact package")
            })?;
            let packages = go_test_package_selectors(arguments);
            if packages.len() != 1 || packages[0] != package {
                bail!(
                    "go-test verification runner must pass exactly the selected package {package}"
                );
            }
        }
        VerificationRunnerAdapter::Shell => {
            bail!("shell runners cannot provide exact verification proof")
        }
    }
    Ok(())
}

fn pytest_option_takes_value(argument: &str) -> bool {
    matches!(
        argument,
        "-k" | "--keyword"
            | "-m"
            | "--markexpr"
            | "--rootdir"
            | "--confcutdir"
            | "--basetemp"
            | "--override-ini"
            | "--ignore"
            | "--ignore-glob"
            | "--deselect"
            | "--import-mode"
            | "--doctest-glob"
            | "--log-level"
            | "--log-format"
            | "--log-date-format"
            | "--log-cli-level"
            | "--log-cli-format"
            | "--capture"
            | "--tb"
            | "--color"
            | "--maxfail"
            | "--durations"
            | "--durations-min"
            | "--junitxml"
            | "--junit-prefix"
            | "--show-capture"
            | "--assert"
            | "--pdbcls"
            | "--pastebin"
            | "--code-highlight"
            | "-o"
            | "-p"
    )
}

fn validate_pytest_output_arguments(arguments: &[String]) -> Result<()> {
    let mut verbose = false;
    let mut pending_value = None;
    for argument in arguments {
        if let Some(option) = pending_value.take() {
            match option {
                "capture" => validate_pytest_capture_mode(argument)?,
                "override" => validate_pytest_override(argument)?,
                _ => {}
            }
            continue;
        }
        if argument == "-s" {
            bail!("pytest verification runner must keep output capture enabled");
        }
        if argument == "--capture" {
            pending_value = Some("capture");
        } else if let Some(mode) = argument.strip_prefix("--capture=") {
            validate_pytest_capture_mode(mode)?;
        } else if argument == "-o" || argument == "--override-ini" {
            pending_value = Some("override");
        } else if argument == "--verbose"
            || argument.strip_prefix('-').is_some_and(|value| {
                !value.is_empty() && value.chars().all(|character| character == 'v')
            })
        {
            verbose = true;
        }
    }
    if pending_value.is_some() {
        bail!("pytest verification runner has an incomplete output option");
    }
    if !verbose {
        bail!("pytest verification runner must request verbose authoritative output");
    }
    Ok(())
}

fn validate_pytest_capture_mode(mode: &str) -> Result<()> {
    if mode != "fd" {
        bail!("pytest verification runner must use fd output capture");
    }
    Ok(())
}

fn validate_pytest_override(value: &str) -> Result<()> {
    let Some((key, setting)) = value.split_once('=') else {
        return Ok(());
    };
    if key == "capture" {
        validate_pytest_capture_mode(setting)?;
    }
    if key == "addopts"
        && setting.split_whitespace().any(|option| {
            option == "-s" || option == "--capture" || option.starts_with("--capture=")
        })
    {
        bail!("pytest verification runner must not override output capture through addopts");
    }
    Ok(())
}

fn node_test_file_selectors(arguments: &[String]) -> Vec<&str> {
    let mut selectors = Vec::new();
    let mut option_value = false;
    for (index, argument) in arguments.iter().enumerate() {
        if option_value {
            option_value = false;
            continue;
        }
        if argument == "--" {
            selectors.extend(arguments[index + 1..].iter().map(String::as_str));
            break;
        }
        if argument.starts_with('-') {
            option_value = node_option_takes_value(argument);
            continue;
        }
        selectors.push(argument.as_str());
    }
    selectors
}

fn node_test_name_patterns(arguments: &[String]) -> Vec<&str> {
    let mut patterns = Vec::new();
    let mut option_value = false;
    for argument in arguments {
        if option_value {
            patterns.push(argument.as_str());
            option_value = false;
            continue;
        }
        if let Some(pattern) = argument.strip_prefix("--test-name-pattern=") {
            patterns.push(pattern);
        } else if argument == "--test-name-pattern" {
            option_value = true;
        }
    }
    patterns
}

fn node_option_takes_value(argument: &str) -> bool {
    matches!(
        argument,
        "-e" | "--eval"
            | "-r"
            | "--require"
            | "--import"
            | "--loader"
            | "--conditions"
            | "--test-name-pattern"
            | "--test-reporter"
            | "--test-reporter-destination"
            | "--test-concurrency"
            | "--test-shard"
            | "--watch-path"
            | "--inspect"
            | "--inspect-brk"
            | "--inspect-port"
    )
}

fn go_test_package_selectors(arguments: &[String]) -> Vec<&str> {
    let mut selectors = Vec::new();
    let mut after_test = false;
    let mut option_value = false;
    for argument in arguments {
        if !after_test {
            after_test = argument == "test";
            continue;
        }
        if option_value {
            option_value = false;
            continue;
        }
        if argument.starts_with('-') {
            option_value = go_test_option_takes_value(argument);
            continue;
        }
        selectors.push(argument.as_str());
    }
    selectors
}

fn go_test_run_filters(arguments: &[String]) -> Vec<&str> {
    let mut filters = Vec::new();
    let mut option_value = false;
    for argument in arguments {
        if option_value {
            filters.push(argument.as_str());
            option_value = false;
            continue;
        }
        if let Some(filter) = argument.strip_prefix("-run=") {
            filters.push(filter);
        } else if argument == "-run" {
            option_value = true;
        }
    }
    filters
}

fn go_test_option_takes_value(argument: &str) -> bool {
    matches!(
        argument,
        "-run"
            | "-bench"
            | "-benchtime"
            | "-count"
            | "-cpu"
            | "-list"
            | "-parallel"
            | "-shuffle"
            | "-skip"
            | "-timeout"
            | "-trace"
            | "-vet"
            | "-cover"
            | "-covermode"
            | "-coverpkg"
            | "-coverprofile"
            | "-gocoverdir"
            | "-exec"
            | "-mod"
            | "-modfile"
            | "-overlay"
            | "-p"
            | "-o"
            | "-fuzz"
            | "-fuzztime"
            | "-fuzzminimizetime"
    )
}

fn parse_cargo_libtest_output(identity: &str, stdout: &[u8]) -> Result<(usize, bool)> {
    let mut matched_count = 0;
    let mut failed = false;
    for line in String::from_utf8_lossy(stdout).lines() {
        let Ok(event) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        if event.get("type").and_then(serde_json::Value::as_str) != Some("test")
            || event.get("name").and_then(serde_json::Value::as_str) != Some(identity)
        {
            continue;
        }
        match event.get("event").and_then(serde_json::Value::as_str) {
            Some("ok") => matched_count += 1,
            Some("ignored") | Some("failed") | Some("timeout") => failed = true,
            _ => {}
        }
    }
    Ok((matched_count, failed))
}

fn parse_pytest_output(
    identity: &str,
    runner_arguments: &[String],
    stdout: &[u8],
) -> Result<(usize, bool)> {
    validate_pytest_output_arguments(runner_arguments)?;
    let mut matched_count = 0;
    let mut failed = false;
    for line in String::from_utf8_lossy(stdout).lines() {
        let line = line.trim();
        let Some(status_and_progress) = line.strip_prefix(identity) else {
            continue;
        };
        if !status_and_progress.starts_with(char::is_whitespace) {
            continue;
        }
        let Some(status) = status_and_progress.split_whitespace().next() else {
            continue;
        };
        match status {
            "PASSED" => matched_count += 1,
            "FAILED" | "SKIPPED" | "XFAIL" | "XPASS" => failed = true,
            _ => {}
        }
    }
    Ok((matched_count, failed))
}

fn parse_node_test_output(name: &str, stdout: &[u8]) -> (usize, bool) {
    let mut matched_count = 0;
    let mut failed = false;
    for line in String::from_utf8_lossy(stdout).lines() {
        let trimmed = line.trim_start();
        let (is_pass, remainder) = if let Some(remainder) = trimmed.strip_prefix("ok ") {
            (true, remainder)
        } else if let Some(remainder) = trimmed.strip_prefix("not ok ") {
            (false, remainder)
        } else {
            continue;
        };
        let Some((title, directive)) = parse_node_tap_title(remainder) else {
            continue;
        };
        if title != name {
            continue;
        }
        if is_pass
            && !directive.is_some_and(|directive| {
                directive.eq_ignore_ascii_case("skip") || directive.eq_ignore_ascii_case("todo")
            })
        {
            matched_count += 1;
        } else {
            failed = true;
        }
    }
    (matched_count, failed)
}

fn parse_node_tap_title(remainder: &str) -> Option<(String, Option<String>)> {
    let remainder = remainder.trim_start();
    let mut fields = remainder.splitn(2, char::is_whitespace);
    let first = fields.next()?;
    let title = if first.chars().all(|character| character.is_ascii_digit()) {
        fields.next()?.trim_start().strip_prefix("- ")?.trim()
    } else {
        remainder.strip_prefix("- ")?.trim()
    };
    if let Some((title, directive)) = title.rsplit_once(" # ")
        && (directive.trim().eq_ignore_ascii_case("skip")
            || directive.trim().eq_ignore_ascii_case("todo"))
    {
        return Some((
            decode_node_tap_escapes(title.trim()),
            Some(directive.trim().to_owned()),
        ));
    }
    Some((decode_node_tap_escapes(title), None))
}

fn decode_node_tap_escapes(value: &str) -> String {
    let mut decoded = String::with_capacity(value.len());
    let mut characters = value.chars();
    while let Some(character) = characters.next() {
        if character == '\\' {
            match characters.next() {
                Some('#') => decoded.push('#'),
                Some('\\') => decoded.push('\\'),
                Some(next) => {
                    decoded.push('\\');
                    decoded.push(next);
                }
                None => decoded.push('\\'),
            }
        } else {
            decoded.push(character);
        }
    }
    decoded
}

fn parse_go_test_output(identity: &str, stdout: &[u8]) -> (usize, bool) {
    let mut matched_count = 0;
    let mut failed = false;
    for line in String::from_utf8_lossy(stdout).lines() {
        let Ok(event) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        if event.get("Test").and_then(serde_json::Value::as_str) != Some(identity) {
            continue;
        }
        match event.get("Action").and_then(serde_json::Value::as_str) {
            Some("pass") => matched_count += 1,
            Some("fail") | Some("skip") => failed = true,
            _ => {}
        }
    }
    (matched_count, failed)
}

pub(crate) fn canonical_runner_arguments_for_adapter(
    adapter: VerificationRunnerAdapter,
    mut arguments: Vec<String>,
) -> Vec<String> {
    if adapter == VerificationRunnerAdapter::Pytest {
        // Pytest merges `addopts` from configuration files after parsing the
        // command line. Override it last so an exact receipt cannot be widened
        // by a repository-local pytest.ini or pyproject.toml.
        arguments.extend(["-o".into(), "addopts=".into()]);
    }
    if adapter == VerificationRunnerAdapter::CargoLibtest
        && arguments.iter().any(|argument| argument == "test")
    {
        if let Some(index) = arguments.iter().position(|argument| argument == "--") {
            let harness_start = index + 1;
            if !arguments[harness_start..]
                .iter()
                .any(|argument| argument == "--exact")
            {
                arguments.insert(harness_start, "--exact".into());
            }
            if !arguments[harness_start..]
                .iter()
                .any(|argument| argument == "--format")
            {
                arguments.splice(
                    harness_start..harness_start,
                    [
                        "-Z".into(),
                        "unstable-options".into(),
                        "--format".into(),
                        "json".into(),
                    ],
                );
            }
        } else {
            arguments.extend([
                "--".into(),
                "--exact".into(),
                "-Z".into(),
                "unstable-options".into(),
                "--format".into(),
                "json".into(),
            ]);
        }
    }
    arguments
}

/// Cargo's human test output is not an authority boundary: a test can print a
/// line that looks like a harness result. Force libtest's structured stream
/// and keep the injected arguments in the receipt's canonical command.
pub fn canonical_runner_arguments(executable: &str, arguments: Vec<String>) -> Vec<String> {
    let adapter = if executable == "cargo" {
        VerificationRunnerAdapter::CargoLibtest
    } else {
        VerificationRunnerAdapter::Shell
    };
    canonical_runner_arguments_for_adapter(adapter, arguments)
}

pub(crate) fn expand_runner_argument_for_adapter(
    adapter: VerificationRunnerAdapter,
    template: &str,
    values: &BTreeMap<String, String>,
) -> String {
    values
        .iter()
        .fold(template.to_owned(), |value, (key, replacement)| {
            let replacement = match adapter {
                VerificationRunnerAdapter::NodeTest if key == "name" => regex_escape(replacement),
                VerificationRunnerAdapter::GoTest if key == "test" => {
                    go_test_regex_fragment(replacement)
                }
                _ => replacement.clone(),
            };
            value.replace(&format!("{{{key}}}"), &replacement)
        })
}

fn has_unresolved_runner_placeholder(template: &str, values: &BTreeMap<String, String>) -> bool {
    let mut remaining = template;
    while let Some(start) = remaining.find('{') {
        let after_start = &remaining[start + 1..];
        let Some(end) = after_start.find('}') else {
            break;
        };
        let key = &after_start[..end];
        if !key.is_empty() && !values.contains_key(key) {
            return true;
        }
        remaining = &after_start[end + 1..];
    }
    false
}

fn digest(bytes: &[u8]) -> String {
    let mut hash = Sha256::new();
    hash.update(bytes);
    format_sha256(hash.finalize())
}

fn verification_receipt_digest<T: Serialize>(value: &T) -> Result<String> {
    let bytes = mitase_work_model::canonical_json_bytes(serde_json::to_value(value)?);
    let mut hash = Sha256::new();
    hash.update(mitase_work_model::VERIFICATION_RECEIPT_DIGEST_DOMAIN.as_bytes());
    hash.update(bytes);
    Ok(format_sha256(hash.finalize()))
}

fn epoch_seconds() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs().to_string())
        .unwrap_or_else(|_| "0".into())
}

fn validate_inner(ctx: &ValidationContext<'_>, include_readiness: bool) -> ValidationResult {
    let mut diagnostics = Vec::new();
    let start = diagnostics.len();
    validate_config(ctx, &mut diagnostics);
    set_phase(&mut diagnostics[start..], ValidationPhase::Config);
    let start = diagnostics.len();
    validate_document_shapes(ctx, &mut diagnostics);
    set_phase(&mut diagnostics[start..], ValidationPhase::Graph);
    let start = diagnostics.len();
    validate_graph(ctx, &mut diagnostics);
    set_phase(&mut diagnostics[start..], ValidationPhase::Graph);
    let start = diagnostics.len();
    validate_targets(ctx, &mut diagnostics);
    set_phase(&mut diagnostics[start..], ValidationPhase::Targets);
    let start = diagnostics.len();
    validate_contracts(ctx, &mut diagnostics);
    set_phase(&mut diagnostics[start..], ValidationPhase::Targets);
    let start = diagnostics.len();
    validate_changes(ctx, &mut diagnostics);
    set_phase(&mut diagnostics[start..], ValidationPhase::Scope);
    if let Some(plan) = ctx.work_plan {
        let start = diagnostics.len();
        validate_plan(ctx, plan, &mut diagnostics);
        set_phase(&mut diagnostics[start..], ValidationPhase::Plan);
    }
    diagnostics.retain_mut(|diagnostic| {
        if !is_fixed_error_rule(&diagnostic.rule_id)
            && rule_metadata(&diagnostic.rule_id)
                .is_some_and(|metadata| !metadata.presets.contains(&ctx.preset))
        {
            return false;
        }
        true
    });
    diagnostics.sort_by(|a, b| {
        (&a.rule_id, &a.primary.path, &a.message).cmp(&(&b.rule_id, &b.primary.path, &b.message))
    });
    let readiness = include_readiness.then(|| {
        crate::evaluate_readiness(
            ctx.workspace,
            ctx.index,
            ctx.revision.unwrap_or("working-tree"),
            true,
        )
    });
    if let Some(Ok(report)) = &readiness {
        if readiness_required(ctx.config.validation.readiness.target)
            && !report.meets_configured(ctx.config)
        {
            diagnostics.push(mitase_diagnostics::Diagnostic::error(
                "MITASE-READINESS-001",
                "workspace does not meet the configured readiness target",
                "mitase.yaml",
            ));
        }
    } else if readiness.as_ref().is_some_and(Result::is_err)
        && readiness_required(ctx.config.validation.readiness.target)
    {
        diagnostics.push(mitase_diagnostics::Diagnostic::error(
            "MITASE-READINESS-001",
            format!(
                "readiness evaluation failed: {}",
                readiness
                    .as_ref()
                    .and_then(|result| result.as_ref().err())
                    .expect("readiness error exists")
            ),
            "workspace",
        ));
    }
    ValidationResult {
        diagnostics,
        readiness: readiness
            .and_then(Result::ok)
            .and_then(|report| serde_json::to_value(report).ok()),
    }
}

fn readiness_required(level: ReadinessLevel) -> bool {
    !matches!(level, ReadinessLevel::Off)
}

fn set_phase(diagnostics: &mut [Diagnostic], phase: ValidationPhase) {
    for diagnostic in diagnostics {
        diagnostic.phase = phase;
    }
}

fn validate_config(ctx: &ValidationContext<'_>, out: &mut Vec<Diagnostic>) {
    if !ctx
        .config
        .inventory
        .profiles
        .iter()
        .any(|profile| profile.id == ctx.config.inventory.active_profile)
    {
        push(
            out,
            "MITASE-SCHEMA-002",
            format!(
                "active inventory profile {} is not defined",
                ctx.config.inventory.active_profile
            ),
            "mitase.yaml",
            None,
        );
    }
    if let Some(probe) = &ctx.config.validation.readiness.probes.public_entrypoints
        && matches!(
            probe.level,
            ReadinessLevel::Traceable | ReadinessLevel::Verifiable | ReadinessLevel::ClosedLoop
        )
    {
        push(
            out,
            "MITASE-SCHEMA-003",
            "public entrypoint readiness probes support only off, seedable, or work-ready in v1",
            "mitase.yaml",
            None,
        );
    }
}

fn rule_metadata(rule_id: &str) -> Option<&'static RuleMetadata> {
    RULES.iter().find(|metadata| metadata.id == rule_id)
}

fn is_fixed_error_rule(rule_id: &str) -> bool {
    rule_metadata(rule_id)
        .is_some_and(|metadata| matches!(metadata.override_policy, OverridePolicy::FixedError))
}
fn validate_changes(ctx: &ValidationContext<'_>, out: &mut Vec<Diagnostic>) {
    let Some(files) = ctx.changed_files else {
        return;
    };
    let baseline = ctx
        .change_base_revision
        .or(ctx.revision)
        .and_then(|revision| load_workspace_at_revision(&ctx.workspace.root, revision));
    // The pre-v1 brand cutover intentionally replaces the repository identity
    // and config root in one atomic change. Its old baseline cannot be loaded
    // as the current canonical workspace, so there is no meaningful ownership
    // graph to compare against. Only an explicit root-config rename from a
    // legacy v1 config can enter this boundary; missing or malformed baselines
    // fail closed and keep ordinary ownership validation active.
    let identity_cutover = is_pre_v1_identity_cutover(
        &ctx.workspace.root,
        ctx.change_base_revision.or(ctx.revision),
        baseline.is_some(),
        ctx.config.schema.as_str(),
        files,
    );
    if identity_cutover {
        return;
    }
    let changed_spec_documents = files
        .iter()
        .flat_map(|file| file.old_path.iter().chain(file.new_path.iter()))
        .filter(|path| ctx.workspace.path_is_spec(path.as_path()))
        .cloned()
        .collect::<BTreeSet<_>>();
    for file in files {
        let Some(path) = file.new_path.as_ref().or(file.old_path.as_ref()) else {
            continue;
        };
        let rendered = path.to_string_lossy();
        if ctx.workspace.path_is_spec(path.as_path()) {
            continue;
        }
        if ctx.workspace.path_is_excluded(path.as_path()) {
            continue;
        }
        if !ctx.workspace.path_is_artifact(path.as_path()) {
            if ctx.config.validation.changed.require_owned_changes {
                push(
                    out,
                    "MITASE-CHANGE-001",
                    format!("changed file is outside the active inventory: {rendered}"),
                    rendered.to_string(),
                    None,
                );
            }
            continue;
        }
        let current_units = ctx
            .index
            .artifact_units
            .iter()
            .filter(|unit| unit.path.to_string_lossy() == rendered)
            .filter(|unit| {
                file.hunks.is_empty()
                    || file.hunks.iter().any(|hunk| {
                        changed_side_overlaps(
                            hunk.new_start,
                            hunk.new_end,
                            (unit.span.line_start, unit.span.line_end),
                        ) || changed_side_overlaps(
                            hunk.old_start,
                            hunk.old_end,
                            (unit.span.line_start, unit.span.line_end),
                        )
                    })
            })
            .cloned()
            .collect::<Vec<_>>();
        let baseline_units = baseline
            .as_ref()
            .map(|baseline| {
                baseline
                    .index
                    .artifact_units
                    .iter()
                    .filter(|unit| unit.path.to_string_lossy() == rendered)
                    .filter(|unit| {
                        file.hunks.is_empty()
                            || file.hunks.iter().any(|hunk| {
                                changed_side_overlaps(
                                    hunk.old_start,
                                    hunk.old_end,
                                    (unit.span.line_start, unit.span.line_end),
                                )
                            })
                    })
                    .cloned()
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let mut units = current_units
            .into_iter()
            .map(|unit| (unit, false))
            .chain(baseline_units.into_iter().map(|unit| (unit, true)))
            .collect::<Vec<_>>();
        let semantic_units = units
            .iter()
            .filter(|(unit, _)| {
                !matches!(unit.kind, mitase_inventory::ArtifactUnitKind::File)
                    && unit.exposure != mitase_inventory::ArtifactExposure::Support
            })
            .cloned()
            .collect::<Vec<_>>();
        if !semantic_units.is_empty() {
            units = semantic_units;
        } else if file
            .hunks
            .iter()
            .any(|hunk| hunk.new_end > hunk.new_start || hunk.old_end > hunk.old_start)
            && units
                .iter()
                .any(|(unit, _)| semantic_change_adapter(&unit.adapter))
        {
            // A source-file unit is only a file-level inventory fact. It must
            // never stand in for a changed symbol when a language provider is
            // enabled. Keeping it here would let a broad file owner hide a
            // hunk that has no exact semantic identity in the current or
            // baseline inventory.
            units.clear();
        }
        if units.is_empty()
            && matches!(file.status, ChangeStatus::Deleted)
            && let Some(old_path) = file.old_path.clone()
        {
            let end = file
                .hunks
                .iter()
                .map(|hunk| hunk.old_end)
                .max()
                .unwrap_or(1);
            units.push((
                mitase_inventory::ArtifactUnit {
                    adapter: "declared".into(),
                    identity: format!("declared:{}", old_path.to_string_lossy()),
                    path: old_path,
                    kind: mitase_inventory::ArtifactUnitKind::File,
                    exposure: mitase_inventory::ArtifactExposure::Workspace,
                    reachability: mitase_inventory::ArtifactReachability::Active,
                    span: mitase_inventory::SourceSpan {
                        byte_start: 0,
                        byte_end: 0,
                        line_start: 1,
                        line_end: end.max(1),
                    },
                    digest: "deleted".into(),
                    structural_digest: "deleted".into(),
                },
                false,
            ));
        }
        if units.is_empty() {
            if ctx.config.validation.changed.require_owned_changes {
                push(
                    out,
                    "MITASE-CHANGE-001",
                    format!("changed hunk has no active semantic artifact identity: {rendered}"),
                    rendered.to_string(),
                    None,
                );
            }
            continue;
        }
        for (unit, from_baseline) in units {
            let index = from_baseline
                .then(|| baseline.as_ref().map(|baseline| &baseline.index))
                .flatten()
                .unwrap_or(ctx.index);
            let mut owned = index
                .artifact_owners
                .get(&unit.identity)
                .cloned()
                .unwrap_or_else(|| {
                    index
                        .bindings
                        .iter()
                        .flat_map(|(binding_anchor, binding)| {
                            binding.owns.iter().filter_map(|scope| {
                                let matches = scope.adapter == unit.adapter
                                    && scope.path == unit.path
                                    && match &scope.selector {
                                        OwnershipSelector::File => true,
                                        OwnershipSelector::Module { name } => name == "*",
                                        OwnershipSelector::PathPrefix { .. } => false,
                                    };
                                matches.then(|| mitase_workspace::OwnershipRef {
                                    binding: binding_anchor.clone(),
                                    scope_id: scope.id.clone(),
                                    target_id: None,
                                })
                            })
                        })
                        .collect::<Vec<_>>()
                });
            if owned.is_empty() && from_baseline {
                // Ownership migrations are part of the current specification boundary. A
                // changed artifact that was unowned in the baseline must be checked against
                // the current exact owner before it is rejected; otherwise a code change and
                // its first explicit ownership binding can never land atomically.
                owned = ctx
                    .index
                    .artifact_owners
                    .get(&unit.identity)
                    .cloned()
                    .unwrap_or_default();
            }
            let owners = Some(owned.as_slice());
            // Cargo build scripts are compiler-owned entrypoints. Their semantic inventory
            // includes historical helper symbols so branch diffs can be compared across
            // versions, but they are governed as one declared build artifact rather than as
            // user-facing specification targets.
            if unit.path.as_path() == Path::new("build.rs") {
                continue;
            }
            if ctx.config.validation.changed.require_owned_changes
                && owners.is_none_or(|owners| owners.is_empty())
            {
                push(
                    out,
                    "MITASE-CHANGE-001",
                    format!(
                        "changed semantic artifact has no ownership binding: {}",
                        unit.identity
                    ),
                    rendered.to_string(),
                    None,
                );
                continue;
            }
            for owner in owners.into_iter().flatten() {
                if let Some(binding) = ctx.index.bindings.get(&owner.binding)
                    && binding.role == BindingRole::Implementation
                    && !binding.targets.iter().any(|target| {
                        target.claims.iter().any(|claim| match claim {
                            mitase_spec_model::TargetClaim::Satisfies { .. } => true,
                            mitase_spec_model::TargetClaim::Exposes { target } => {
                                ctx.index.target(target).is_some_and(|exposed| {
                                    exposed.claims.iter().any(|claim| {
                                        matches!(
                                            claim,
                                            mitase_spec_model::TargetClaim::Satisfies { .. }
                                        )
                                    })
                                })
                            }
                            _ => false,
                        })
                    })
                {
                    push(
                        out,
                        "MITASE-CHANGE-002",
                        "changed implementation has no Criterion",
                        rendered.to_string(),
                        Some(owner.binding.clone()),
                    );
                }
            }
        }
    }
    validate_changed_spec_impact(ctx, &changed_spec_documents, files, out);
}

fn is_pre_v1_identity_cutover(
    root: &Path,
    revision: Option<&str>,
    baseline_loaded: bool,
    schema: &str,
    files: &[ChangedFile],
) -> bool {
    if baseline_loaded || schema != mitase_project_model::CONFIG_SCHEMA {
        return false;
    }
    let Some(revision) = revision else {
        return false;
    };
    let canonical_config_added = files.iter().any(|file| {
        file.new_path
            .as_ref()
            .is_some_and(|path| path.as_path() == Path::new("mitase.yaml"))
    });
    if !canonical_config_added {
        return false;
    }
    let Some(legacy_path) = files.iter().find_map(|file| {
        let old_path = file.old_path.as_ref()?;
        let is_legacy_rename = file
            .new_path
            .as_ref()
            .is_some_and(|new_path| new_path.as_path() == Path::new("mitase.yaml"));
        let is_legacy_delete = file.new_path.is_none();
        let is_root_yaml = old_path
            .as_path()
            .parent()
            .is_none_or(|parent| parent.as_os_str().is_empty())
            && old_path
                .as_path()
                .extension()
                .is_some_and(|extension| extension == "yaml");
        (is_legacy_rename || is_legacy_delete)
            .then_some(old_path.as_path())
            .filter(|_| old_path.as_path() != Path::new("mitase.yaml") && is_root_yaml)
    }) else {
        return false;
    };
    let Ok(legacy_config) = git_show(root, revision, legacy_path) else {
        return false;
    };
    let Some(legacy_name) = legacy_path.file_stem().and_then(|name| name.to_str()) else {
        return false;
    };
    let expected_schema = format!("{legacy_name}/config/v1");
    let Ok(config) = serde_yaml::from_str::<ProjectConfig>(&legacy_config) else {
        return false;
    };
    if config.schema != expected_schema {
        return false;
    }
    !config.workspace.spec_roots.is_empty()
        && config
            .inventory
            .profiles
            .iter()
            .any(|profile| profile.id == config.inventory.active_profile)
        && git_show(root, revision, Path::new("mitase.yaml")).is_err()
}

fn semantic_change_adapter(adapter: &str) -> bool {
    matches!(
        adapter,
        "rust" | "javascript" | "typescript" | "python" | "go" | "shell"
    )
}

fn validate_changed_spec_impact(
    ctx: &ValidationContext<'_>,
    changed_spec_documents: &BTreeSet<RepoPath>,
    changed_files: &[ChangedFile],
    out: &mut Vec<Diagnostic>,
) {
    if changed_spec_documents.is_empty() {
        return;
    }
    let baseline = ctx
        .change_base_revision
        .or(ctx.revision)
        .and_then(|revision| load_workspace_at_revision(&ctx.workspace.root, revision));

    for anchor in changed_anchors_for_documents(
        changed_spec_documents,
        ctx.workspace,
        ctx.index,
        baseline.as_ref().map(|baseline| &baseline.workspace),
        baseline.as_ref().map(|baseline| &baseline.index),
        LocalAnchorKind::Criterion,
    ) {
        if !anchor_changed(
            &anchor,
            baseline.as_ref().map(|baseline| &baseline.index),
            ctx.index,
        ) {
            continue;
        }
        let criterion_removed = baseline
            .as_ref()
            .and_then(|baseline| baseline.index.anchor(&anchor))
            .is_some()
            && ctx.index.anchor(&anchor).is_none();
        let implementation_changed = binding_set_for_criterion(
            baseline.as_ref().map(|baseline| &baseline.index),
            ctx.index,
            &anchor,
            true,
        )
        .iter()
        .any(|binding| {
            binding_targets_changed_across_indexes(
                baseline.as_ref().map(|baseline| &baseline.index),
                baseline.as_ref().map(|baseline| &baseline.workspace),
                ctx.index,
                ctx.workspace,
                binding,
                changed_files,
            )
        });
        let verification_changed = binding_set_for_criterion(
            baseline.as_ref().map(|baseline| &baseline.index),
            ctx.index,
            &anchor,
            false,
        )
        .iter()
        .any(|binding| {
            binding_targets_changed_across_indexes(
                baseline.as_ref().map(|baseline| &baseline.index),
                baseline.as_ref().map(|baseline| &baseline.workspace),
                ctx.index,
                ctx.workspace,
                binding,
                changed_files,
            )
        });
        let retired_binding_changed = criterion_removed
            && binding_set_for_criterion(
                baseline.as_ref().map(|baseline| &baseline.index),
                ctx.index,
                &anchor,
                true,
            )
            .iter()
            .any(|binding| {
                anchor_changed(
                    binding,
                    baseline.as_ref().map(|baseline| &baseline.index),
                    ctx.index,
                ) && changed_spec_documents.iter().any(|document| {
                    document.to_string_lossy()
                        == changed_anchor_path(
                            binding,
                            baseline.as_ref().map(|baseline| &baseline.workspace),
                            baseline.as_ref().map(|baseline| &baseline.index),
                            ctx.workspace,
                            ctx.index,
                        )
                })
            });
        if !(implementation_changed || verification_changed || retired_binding_changed) {
            push(
                out,
                "MITASE-CHANGE-003",
                format!(
                    "changed criterion has no impacted implementation or verification update: {anchor}"
                ),
                changed_anchor_path(
                    &anchor,
                    baseline.as_ref().map(|baseline| &baseline.workspace),
                    baseline.as_ref().map(|baseline| &baseline.index),
                    ctx.workspace,
                    ctx.index,
                ),
                Some(anchor.clone()),
            );
        }
    }

    for anchor in changed_anchors_for_documents(
        changed_spec_documents,
        ctx.workspace,
        ctx.index,
        baseline.as_ref().map(|baseline| &baseline.workspace),
        baseline.as_ref().map(|baseline| &baseline.index),
        LocalAnchorKind::Contract,
    ) {
        if !anchor_changed(
            &anchor,
            baseline.as_ref().map(|baseline| &baseline.index),
            ctx.index,
        ) {
            continue;
        }
        let participants = participant_bindings_for_contract(
            baseline.as_ref().map(|baseline| &baseline.index),
            ctx.index,
            &anchor,
        );
        if participants.is_empty()
            || participants.iter().any(|binding| {
                !binding_targets_changed_across_indexes(
                    baseline.as_ref().map(|baseline| &baseline.index),
                    baseline.as_ref().map(|baseline| &baseline.workspace),
                    ctx.index,
                    ctx.workspace,
                    binding,
                    changed_files,
                )
            })
        {
            push(
                out,
                "MITASE-CHANGE-004",
                format!("changed contract has no full participant artifact update: {anchor}"),
                changed_anchor_path(
                    &anchor,
                    baseline.as_ref().map(|baseline| &baseline.workspace),
                    baseline.as_ref().map(|baseline| &baseline.index),
                    ctx.workspace,
                    ctx.index,
                ),
                Some(anchor.clone()),
            );
        }
    }

    for anchor in changed_anchors_for_documents(
        changed_spec_documents,
        ctx.workspace,
        ctx.index,
        baseline.as_ref().map(|baseline| &baseline.workspace),
        baseline.as_ref().map(|baseline| &baseline.index),
        LocalAnchorKind::Binding,
    ) {
        if !anchor_changed(
            &anchor,
            baseline.as_ref().map(|baseline| &baseline.index),
            ctx.index,
        ) {
            continue;
        }
        if !binding_targets_changed_across_indexes(
            baseline.as_ref().map(|baseline| &baseline.index),
            baseline.as_ref().map(|baseline| &baseline.workspace),
            ctx.index,
            ctx.workspace,
            &anchor,
            changed_files,
        ) && !binding_definition_changed(
            baseline.as_ref().map(|baseline| &baseline.index),
            ctx.index,
            &anchor,
        ) {
            push(
                out,
                "MITASE-CHANGE-005",
                format!("changed binding has no impacted target update: {anchor}"),
                changed_anchor_path(
                    &anchor,
                    baseline.as_ref().map(|baseline| &baseline.workspace),
                    baseline.as_ref().map(|baseline| &baseline.index),
                    ctx.workspace,
                    ctx.index,
                ),
                Some(anchor.clone()),
            );
        }
    }
}

fn binding_definition_changed(
    baseline: Option<&SpecIndex>,
    current: &SpecIndex,
    binding: &SpecAnchor,
) -> bool {
    baseline.and_then(|index| index.bindings.get(binding)) != current.bindings.get(binding)
}

fn binding_targets_changed_across_indexes(
    baseline: Option<&SpecIndex>,
    baseline_workspace: Option<&SpecWorkspace>,
    current: &SpecIndex,
    current_workspace: &SpecWorkspace,
    binding: &SpecAnchor,
    changed_files: &[ChangedFile],
) -> bool {
    binding_targets_for_index(baseline, binding)
        .into_iter()
        .chain(binding_targets_for_index(Some(current), binding))
        .any(|reference| {
            target_changed_by_files(
                baseline_workspace,
                baseline,
                current_workspace,
                current,
                &reference,
                changed_files,
            )
        })
}

struct BaselineWorkspace {
    _tempdir: TempDir,
    workspace: SpecWorkspace,
    index: SpecIndex,
}

fn load_workspace_at_revision(root: &Path, revision: &str) -> Option<BaselineWorkspace> {
    match try_load_workspace_at_revision(root, revision) {
        Ok(workspace) => Some(workspace),
        Err(error) => {
            if std::env::var_os("MITASE_DEBUG_BASELINE").is_some() {
                eprintln!("could not load validation baseline {revision}: {error:#}");
            }
            None
        }
    }
}

fn try_load_workspace_at_revision(root: &Path, revision: &str) -> Result<BaselineWorkspace> {
    let mitase_config = git_show(root, revision, Path::new("mitase.yaml"))
        .map_err(anyhow::Error::msg)
        .context("read baseline mitase.yaml")?;
    let config = serde_yaml::from_str::<ProjectConfig>(&mitase_config)
        .context("parse v1 baseline mitase.yaml")?;
    let tempdir = tempfile::Builder::new()
        .prefix("mitase-baseline-")
        .tempdir()
        .context("create baseline workspace")?;
    let workspace_dir = tempdir.path();
    fs::write(workspace_dir.join("mitase.yaml"), mitase_config)
        .context("write normalized baseline config")?;
    let files = git_ls_tree(root, revision)
        .map_err(anyhow::Error::msg)
        .context("list baseline files")?;
    for relative in &files {
        if relative == Path::new("mitase.yaml") {
            // Keep the normalized baseline config written above. Copying the
            // historical source here would reintroduce its legacy shape.
            continue;
        }
        let include = relative == Path::new("mitase.yaml")
            || config
                .workspace
                .spec_roots
                .iter()
                .any(|root| relative.starts_with(root.as_path()))
            || !relative.as_os_str().is_empty();
        if !include {
            continue;
        }
        let contents = match git_show(root, revision, relative) {
            Ok(contents) => contents,
            Err(_) => continue,
        };
        let destination = workspace_dir.join(relative);
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).context("create baseline parent directory")?;
        }
        fs::write(destination, contents).context("write baseline file")?;
    }
    let workspace = SpecWorkspace::load(workspace_dir).context("load baseline workspace")?;
    let index = workspace.index().context("index baseline workspace")?;
    Ok(BaselineWorkspace {
        _tempdir: tempdir,
        workspace,
        index,
    })
}

fn repository_revision(root: &Path) -> Result<String> {
    let (repo_root, _) = git_workspace_context(root).map_err(anyhow::Error::msg)?;
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_root)
        .args(["rev-parse", "HEAD"])
        .output()
        .context("resolve repository HEAD")?;
    if !output.status.success() {
        bail!("git rev-parse HEAD failed");
    }
    Ok(String::from_utf8(output.stdout)?.trim().into())
}

fn git_show(root: &Path, revision: &str, relative: &Path) -> Result<String, String> {
    let (repo_root, prefix) = git_workspace_context(root)?;
    let repo_relative = if prefix.as_os_str().is_empty() {
        relative.to_path_buf()
    } else {
        prefix.join(relative)
    };
    let spec = format!("{revision}:{}", repo_relative.to_string_lossy());
    let output = Command::new("git")
        .arg("-C")
        .arg(&repo_root)
        .args(["show", &spec])
        .output()
        .map_err(|error| error.to_string())?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }
    String::from_utf8(output.stdout).map_err(|error| error.to_string())
}

fn git_ls_tree(root: &Path, revision: &str) -> Result<Vec<PathBuf>, String> {
    let (repo_root, prefix) = git_workspace_context(root)?;
    let output = Command::new("git")
        .arg("-C")
        .arg(&repo_root)
        .args(["ls-tree", "-r", "-z", "--name-only", revision])
        .output()
        .map_err(|error| error.to_string())?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }
    Ok(output
        .stdout
        .split(|byte| *byte == 0)
        .filter(|entry| !entry.is_empty())
        .filter_map(|entry| {
            let path = PathBuf::from(String::from_utf8_lossy(entry).to_string());
            if prefix.as_os_str().is_empty() {
                return Some(path);
            }
            path.strip_prefix(&prefix).ok().map(PathBuf::from)
        })
        .collect())
}

fn git_workspace_context(root: &Path) -> Result<(PathBuf, PathBuf), String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .map_err(|error| error.to_string())?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }
    let repo_root = PathBuf::from(String::from_utf8_lossy(&output.stdout).trim());
    let canonical_repo_root = repo_root
        .canonicalize()
        .map_err(|error| error.to_string())?;
    let canonical_root = root.canonicalize().map_err(|error| error.to_string())?;
    let prefix = canonical_root
        .strip_prefix(&canonical_repo_root)
        .map(PathBuf::from)
        .map_err(|error| error.to_string())?;
    Ok((canonical_repo_root, prefix))
}

fn changed_anchors_for_documents(
    changed_documents: &BTreeSet<RepoPath>,
    current_workspace: &SpecWorkspace,
    current_index: &SpecIndex,
    baseline_workspace: Option<&SpecWorkspace>,
    baseline_index: Option<&SpecIndex>,
    kind: LocalAnchorKind,
) -> BTreeSet<SpecAnchor> {
    let mut anchors = BTreeSet::new();
    collect_changed_anchors(
        &mut anchors,
        changed_documents,
        current_workspace,
        current_index,
        kind,
    );
    if let (Some(workspace), Some(index)) = (baseline_workspace, baseline_index) {
        collect_changed_anchors(&mut anchors, changed_documents, workspace, index, kind);
    }
    anchors
}

fn collect_changed_anchors(
    out: &mut BTreeSet<SpecAnchor>,
    changed_documents: &BTreeSet<RepoPath>,
    workspace: &SpecWorkspace,
    index: &SpecIndex,
    kind: LocalAnchorKind,
) {
    for (item, path) in &index.item_paths {
        let Some(relative_path) = workspace_relative_repo_path(path, &workspace.root) else {
            continue;
        };
        if !changed_documents.contains(&relative_path) {
            continue;
        }
        for anchor in index.item_anchors.get(item).into_iter().flatten() {
            if anchor.kind == kind {
                out.insert(anchor.clone());
            }
        }
    }
}

fn anchor_changed(anchor: &SpecAnchor, baseline: Option<&SpecIndex>, current: &SpecIndex) -> bool {
    match (
        baseline.and_then(|index| index.anchor(anchor)),
        current.anchor(anchor),
    ) {
        (Some(AnchorValue::Criterion(left)), Some(AnchorValue::Criterion(right))) => left != right,
        (Some(AnchorValue::Contract(left)), Some(AnchorValue::Contract(right))) => left != right,
        (Some(AnchorValue::Binding(left)), Some(AnchorValue::Binding(right))) => left != right,
        (None, None) => false,
        _ => true,
    }
}

fn binding_set_for_criterion(
    baseline: Option<&SpecIndex>,
    current: &SpecIndex,
    criterion: &SpecAnchor,
    implementation: bool,
) -> BTreeSet<SpecAnchor> {
    let mut bindings = BTreeSet::new();
    if let Some(index) = baseline {
        let source = if implementation {
            &index.criteria_to_implementations
        } else {
            &index.criteria_to_verifications
        };
        bindings.extend(source.get(criterion).into_iter().flatten().cloned());
    }
    let source = if implementation {
        &current.criteria_to_implementations
    } else {
        &current.criteria_to_verifications
    };
    bindings.extend(source.get(criterion).into_iter().flatten().cloned());
    bindings
}

fn participant_bindings_for_contract(
    baseline: Option<&SpecIndex>,
    current: &SpecIndex,
    contract: &SpecAnchor,
) -> BTreeSet<SpecAnchor> {
    let mut bindings = BTreeSet::new();
    if let Some(index) = baseline
        && let Some(old_contract) = index.contracts.get(contract)
    {
        bindings.extend(
            old_contract
                .participants
                .iter()
                .map(|participant| participant.target.binding.clone()),
        );
    }
    if let Some(new_contract) = current.contracts.get(contract) {
        bindings.extend(
            new_contract
                .participants
                .iter()
                .map(|participant| participant.target.binding.clone()),
        );
    }
    bindings
}

fn binding_targets_for_index(
    index: Option<&SpecIndex>,
    binding: &SpecAnchor,
) -> BTreeSet<BoundTargetRef> {
    index
        .and_then(|index| index.bindings.get(binding))
        .map(|artifact| {
            artifact
                .targets
                .iter()
                .map(|target| BoundTargetRef {
                    binding: binding.clone(),
                    target_id: target.id.clone(),
                })
                .collect()
        })
        .unwrap_or_default()
}

fn target_changed_by_files(
    baseline_workspace: Option<&SpecWorkspace>,
    baseline_index: Option<&SpecIndex>,
    current_workspace: &SpecWorkspace,
    current_index: &SpecIndex,
    reference: &BoundTargetRef,
    changed_files: &[ChangedFile],
) -> bool {
    let baseline_hit = baseline_workspace
        .zip(baseline_index.and_then(|index| index.target(reference)))
        .and_then(|(workspace, target)| {
            resolve_target_in_workspace(workspace, target)
                .ok()
                .map(|resolved| {
                    changed_files.iter().any(|file| {
                        changed_file_impacts_target(
                            TargetRangeSide::Old,
                            &resolved.path.to_string_lossy(),
                            Some(&resolved),
                            file,
                        )
                    })
                })
        })
        .unwrap_or(false);
    if baseline_hit {
        return true;
    }
    current_index
        .target(reference)
        .and_then(|target| {
            resolve_target_in_workspace(current_workspace, target)
                .ok()
                .map(|resolved| {
                    changed_files.iter().any(|file| {
                        changed_file_impacts_target(
                            TargetRangeSide::New,
                            &resolved.path.to_string_lossy(),
                            Some(&resolved),
                            file,
                        )
                    })
                })
        })
        .unwrap_or(false)
}

fn changed_file_impacts_target(
    side: TargetRangeSide,
    target_path: &str,
    resolved: Option<&ResolvedTarget>,
    file: &ChangedFile,
) -> bool {
    let target_range = resolved.map(|resolved| (resolved.line_start, resolved.line_end));
    let target_matches_side =
        |path: Option<&RepoPath>| path.is_some_and(|path| path.to_string_lossy() == target_path);
    let structural_change = file.old_path != file.new_path
        || matches!(
            file.status,
            ChangeStatus::Added
                | ChangeStatus::Deleted
                | ChangeStatus::Renamed
                | ChangeStatus::Binary
        );
    let side_path_matches = match side {
        TargetRangeSide::Old => target_matches_side(file.old_path.as_ref()),
        TargetRangeSide::New => target_matches_side(file.new_path.as_ref()),
    };
    if resolved.is_none() {
        return side_path_matches;
    }
    if structural_change && side_path_matches {
        return true;
    }
    if target_range.is_none() {
        return false;
    }
    file.hunks.iter().any(|hunk| match side {
        TargetRangeSide::Old => {
            side_path_matches
                && changed_side_overlaps(hunk.old_start, hunk.old_end, target_range.unwrap())
        }
        TargetRangeSide::New => {
            side_path_matches
                && changed_side_overlaps(hunk.new_start, hunk.new_end, target_range.unwrap())
        }
    })
}

fn changed_anchor_path(
    anchor: &SpecAnchor,
    baseline_workspace: Option<&SpecWorkspace>,
    baseline_index: Option<&SpecIndex>,
    current_workspace: &SpecWorkspace,
    current_index: &SpecIndex,
) -> String {
    current_index
        .item_paths
        .get(&anchor.item)
        .and_then(|path| workspace_relative_display(path, &current_workspace.root))
        .map(|path| path.to_string_lossy().into_owned())
        .or_else(|| {
            baseline_index.and_then(|index| {
                baseline_workspace.and_then(|workspace| {
                    index
                        .item_paths
                        .get(&anchor.item)
                        .and_then(|path| workspace_relative_display(path, &workspace.root))
                        .map(|path| path.to_string_lossy().into_owned())
                })
            })
        })
        .unwrap_or_else(|| "mitase-spec".into())
}

fn normalize_workspace_path(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

fn workspace_relative_repo_path(path: &Path, root: &Path) -> Option<RepoPath> {
    workspace_relative_display(path, root).and_then(|relative| RepoPath::from_path(relative).ok())
}

fn workspace_relative_display(path: &Path, root: &Path) -> Option<PathBuf> {
    path.strip_prefix(root).ok().map(PathBuf::from).or_else(|| {
        let normalized_path = normalize_workspace_path(path);
        let normalized_root = normalize_workspace_path(root);
        normalized_path
            .strip_prefix(&normalized_root)
            .ok()
            .map(PathBuf::from)
    })
}
fn validate_document_shapes(ctx: &ValidationContext<'_>, out: &mut Vec<Diagnostic>) {
    for loaded in &ctx.workspace.documents {
        let path = loaded.path.to_string_lossy().into_owned();
        match &loaded.document {
            SpecDocument::Philosophies { philosophies, .. } => {
                for item in philosophies {
                    if item.principles.is_empty() {
                        push(
                            out,
                            "MITASE-PHILOSOPHY-001",
                            "philosophy has no Principle",
                            &path,
                            None,
                        );
                    }
                    if item.bindings.iter().any(|binding| {
                        !matches!(
                            binding.role,
                            BindingRole::Documentation | BindingRole::Evidence
                        )
                    }) {
                        push(
                            out,
                            "MITASE-BINDING-002",
                            "philosophy binding role is not allowed",
                            &path,
                            None,
                        );
                    }
                }
            }
            SpecDocument::Policies { policies, .. } => {
                for item in policies {
                    if item.rules.is_empty() {
                        push(out, "MITASE-POLICY-001", "policy has no Rule", &path, None);
                    }
                }
            }
            SpecDocument::Requirements { requirements, .. } => {
                for item in requirements {
                    if item.status == ItemStatus::Implemented && item.criteria.is_empty() {
                        push(
                            out,
                            "MITASE-REQUIREMENT-001",
                            "implemented requirement has no Criterion",
                            &path,
                            None,
                        );
                    }
                    if item
                        .bindings
                        .iter()
                        .any(|binding| binding.role == BindingRole::Implementation)
                    {
                        push(
                            out,
                            "MITASE-BINDING-002",
                            "requirement cannot own implementation bindings",
                            &path,
                            None,
                        );
                    }
                }
            }
            SpecDocument::Features { features, .. } => {
                for item in features {
                    let implementation_bindings = item
                        .bindings
                        .iter()
                        .filter(|binding| binding.role == BindingRole::Implementation)
                        .collect::<Vec<_>>();
                    if item.status == ItemStatus::Implemented && implementation_bindings.is_empty()
                    {
                        push(
                            out,
                            "MITASE-FEATURE-001",
                            "implemented feature has no implementation binding",
                            &path,
                            None,
                        );
                    }
                    if item.status == ItemStatus::Planned
                        && item.bindings.iter().any(|binding| !binding.owns.is_empty())
                    {
                        push(
                            out,
                            "MITASE-FEATURE-003",
                            "planned feature must not declare current artifact ownership",
                            &path,
                            None,
                        );
                    }
                    if item.status != ItemStatus::Implemented {
                        continue;
                    }
                    for binding in implementation_bindings {
                        let binding_anchor = SpecAnchor {
                            item: item.id.clone(),
                            kind: LocalAnchorKind::Binding,
                            local_id: binding.id.clone(),
                        };
                        for target in &binding.targets {
                            if target.lifecycle
                                == mitase_spec_model::ArtifactTargetLifecycle::Absent
                            {
                                continue;
                            }
                            let target_ref = BoundTargetRef {
                                binding: binding_anchor.clone(),
                                target_id: target.id.clone(),
                            };
                            let mut acceptance = Vec::new();
                            for claim in &target.claims {
                                match claim {
                                    TargetClaim::Satisfies { criterion } => {
                                        acceptance.push((criterion.clone(), target_ref.clone()));
                                    }
                                    TargetClaim::Exposes { target: exposed } => {
                                        let Some(exposed_target) = ctx.index.target(exposed) else {
                                            continue;
                                        };
                                        acceptance.extend(exposed_target.claims.iter().filter_map(
                                            |claim| match claim {
                                                TargetClaim::Satisfies { criterion } => {
                                                    Some((criterion.clone(), exposed.clone()))
                                                }
                                                _ => None,
                                            },
                                        ));
                                    }
                                    _ => {}
                                }
                            }
                            if acceptance.is_empty() {
                                push(
                                    out,
                                    "MITASE-FEATURE-002",
                                    format!(
                                        "implemented feature target {target_ref} has no direct or exposed acceptance criterion"
                                    ),
                                    &path,
                                    Some(binding_anchor.clone()),
                                );
                                continue;
                            }
                            for (criterion, verified_target) in acceptance {
                                if ctx.index.criterion_status.get(&criterion)
                                    != Some(&ItemStatus::Implemented)
                                {
                                    push(
                                        out,
                                        "MITASE-FEATURE-002",
                                        format!(
                                            "implemented feature target {target_ref} references a non-implemented criterion {criterion}"
                                        ),
                                        &path,
                                        Some(binding_anchor.clone()),
                                    );
                                    continue;
                                }
                                let verified = ctx
                                    .index
                                    .verification_by_target
                                    .get(&verified_target)
                                    .into_iter()
                                    .flatten()
                                    .any(|verification_ref| {
                                        ctx.index
                                            .bindings
                                            .get(&verification_ref.binding)
                                            .is_some_and(|binding| {
                                                binding.role == BindingRole::Verification
                                            })
                                            && ctx.index.target(verification_ref).is_some_and(
                                                |verification| {
                                                    verification.claims.iter().any(|claim| {
                                                        matches!(
                                                            claim,
                                                            TargetClaim::Verifies {
                                                                criterion: actual,
                                                                covers,
                                                                runner,
                                                            } if actual == &criterion
                                                                && covers.contains(&verified_target)
                                                                && ctx
                                                                    .config
                                                                    .verification
                                                                    .runners
                                                                    .contains_key(&runner.runner)
                                                        )
                                                    })
                                                },
                                            )
                                    });
                                if !verified {
                                    push(
                                        out,
                                        "MITASE-FEATURE-002",
                                        format!(
                                            "implemented feature target {target_ref} has no exact verification for {criterion} through {verified_target}"
                                        ),
                                        &path,
                                        Some(binding_anchor.clone()),
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn push(
    out: &mut Vec<Diagnostic>,
    rule: &str,
    msg: impl Into<String>,
    path: impl Into<String>,
    anchor: Option<SpecAnchor>,
) {
    let mut d = Diagnostic::error(rule, msg, path);
    d.anchor = anchor;
    out.push(d);
}

fn validate_graph(ctx: &ValidationContext<'_>, out: &mut Vec<Diagnostic>) {
    for (anchor, value) in &ctx.index.anchors {
        let path = ctx
            .index
            .item_paths
            .get(&anchor.item)
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();
        match value {
            AnchorValue::Principle(_) => {}
            AnchorValue::Rule(rule) => {
                if rule.governed_by.is_empty() {
                    push(
                        out,
                        "MITASE-POLICY-002",
                        "rule has no governing Principle",
                        &path,
                        Some(anchor.clone()),
                    );
                }
                for target in &rule.governed_by {
                    check_kind(ctx, out, target, LocalAnchorKind::Principle, &path);
                }
                if rule.level == RuleLevel::Must {
                    let covered = ctx
                        .index
                        .bindings
                        .values()
                        .any(|b| b.targets.iter().any(|target| target.claims.iter().any(|claim| matches!(claim, mitase_spec_model::TargetClaim::Enforces { rule } if rule == anchor) || matches!(claim, mitase_spec_model::TargetClaim::Evidences { anchor: evidence } if evidence == anchor))))
                        || rule.enforcement.is_some();
                    if !covered {
                        push(
                            out,
                            "MITASE-POLICY-003",
                            "must rule has no enforcement or evidence",
                            &path,
                            Some(anchor.clone()),
                        );
                    }
                }
            }
            AnchorValue::Criterion(criterion) => {
                if criterion.governed_by.is_empty() {
                    push(
                        out,
                        "MITASE-REQUIREMENT-002",
                        "criterion has no governing Rule",
                        &path,
                        Some(anchor.clone()),
                    );
                }
                for target in &criterion.governed_by {
                    check_kind(ctx, out, target, LocalAnchorKind::Rule, &path);
                }
            }
            AnchorValue::Binding(binding) => {
                if binding.responsibility.trim().is_empty() {
                    push(
                        out,
                        "MITASE-BINDING-003",
                        "binding responsibility is empty",
                        &path,
                        Some(anchor.clone()),
                    );
                }
                if binding.targets.is_empty() {
                    push(
                        out,
                        "MITASE-BINDING-004",
                        "binding has no exact target",
                        &path,
                        Some(anchor.clone()),
                    );
                }
                let relation = binding
                    .targets
                    .iter()
                    .flat_map(|target| target.claims.iter())
                    .filter_map(|claim| match claim {
                        mitase_spec_model::TargetClaim::Satisfies { criterion }
                            if binding.role == BindingRole::Implementation =>
                        {
                            Some(criterion)
                        }
                        mitase_spec_model::TargetClaim::Verifies { criterion, .. }
                            if binding.role == BindingRole::Verification =>
                        {
                            Some(criterion)
                        }
                        mitase_spec_model::TargetClaim::Documents { anchor }
                            if binding.role == BindingRole::Documentation =>
                        {
                            Some(anchor)
                        }
                        mitase_spec_model::TargetClaim::Enforces { rule }
                            if binding.role == BindingRole::Enforcement =>
                        {
                            Some(rule)
                        }
                        mitase_spec_model::TargetClaim::Evidences { anchor }
                            if binding.role == BindingRole::Evidence =>
                        {
                            Some(anchor)
                        }
                        _ => None,
                    })
                    .collect::<Vec<_>>();
                let has_exposes = binding.targets.iter().any(|target| {
                    target
                        .claims
                        .iter()
                        .any(|claim| matches!(claim, TargetClaim::Exposes { .. }))
                });
                if matches!(
                    binding.role,
                    BindingRole::Implementation
                        | BindingRole::Verification
                        | BindingRole::Documentation
                        | BindingRole::Enforcement
                        | BindingRole::Evidence
                ) && relation.is_empty()
                    && !(binding.role == BindingRole::Implementation && has_exposes)
                {
                    push(
                        out,
                        "MITASE-BINDING-001",
                        "binding role requires its canonical relation",
                        &path,
                        Some(anchor.clone()),
                    );
                }
                for target in &relation {
                    if !ctx.index.anchors.contains_key(target) {
                        push(
                            out,
                            "MITASE-ANCHOR-002",
                            format!("unresolved relation {target}"),
                            &path,
                            Some(anchor.clone()),
                        );
                    }
                }
                for artifact_target in &binding.targets {
                    let exposure_count = artifact_target
                        .claims
                        .iter()
                        .filter(|claim| matches!(claim, TargetClaim::Exposes { .. }))
                        .count();
                    if exposure_count > 1
                        || (binding.facet == "public"
                            && (exposure_count != 1
                                || artifact_target
                                    .claims
                                    .iter()
                                    .any(|claim| matches!(claim, TargetClaim::Satisfies { .. }))))
                    {
                        push(
                            out,
                            "MITASE-EXPOSURE-001",
                            "public governance targets must expose exactly one capability target and must not claim capability acceptance directly",
                            &path,
                            Some(anchor.clone()),
                        );
                    }
                    for claim in &artifact_target.claims {
                        match claim {
                            mitase_spec_model::TargetClaim::Verifies { covers, .. } => {
                                if covers.is_empty() {
                                    push(
                                        out,
                                        "MITASE-VERIFICATION-001",
                                        "verification target must cover at least one exact target",
                                        &path,
                                        Some(anchor.clone()),
                                    );
                                }
                                for covered in covers {
                                    if ctx.index.target(covered).is_none() {
                                        push(
                                            out,
                                            "MITASE-VERIFICATION-002",
                                            format!(
                                                "verification covers unresolved target {covered}"
                                            ),
                                            &path,
                                            Some(anchor.clone()),
                                        );
                                    }
                                }
                            }
                            mitase_spec_model::TargetClaim::Exposes { target } => {
                                let valid = ctx.index.target(target).is_some()
                                    && ctx.index.bindings.get(&target.binding).is_some_and(
                                        |exposed_binding| {
                                            exposed_binding.role == BindingRole::Implementation
                                                && !matches!(
                                                    ctx.index.item_status.get(&target.binding.item),
                                                    Some(ItemStatus::Planned)
                                                )
                                        },
                                    );
                                if binding.role != BindingRole::Implementation || !valid {
                                    push(
                                        out,
                                        "MITASE-EXPOSURE-001",
                                        format!(
                                            "exposes must reference a current exact implementation target: {target}"
                                        ),
                                        &path,
                                        Some(anchor.clone()),
                                    );
                                }
                            }
                            _ => {}
                        }
                    }
                }
                let expected_kind = match binding.role {
                    BindingRole::Implementation | BindingRole::Verification => {
                        Some(LocalAnchorKind::Criterion)
                    }
                    BindingRole::Enforcement => Some(LocalAnchorKind::Rule),
                    BindingRole::Documentation | BindingRole::Evidence => None,
                    _ => None,
                };
                if let Some(expected) = expected_kind {
                    for target in &relation {
                        check_kind(ctx, out, target, expected, &path);
                    }
                }
                for target in &binding.targets {
                    let generated_from = target
                        .claims
                        .iter()
                        .filter_map(|claim| match claim {
                            mitase_spec_model::TargetClaim::GeneratedFrom { targets } => {
                                Some(targets.as_slice())
                            }
                            _ => None,
                        })
                        .flatten()
                        .collect::<Vec<_>>();
                    if binding.role == BindingRole::Generated && generated_from.is_empty() {
                        push(
                            out,
                            "MITASE-GENERATED-001",
                            format!(
                                "generated target {} has no generated_from source",
                                target.id
                            ),
                            &path,
                            Some(anchor.clone()),
                        );
                    } else if binding.role == BindingRole::Generated {
                        validate_generated_target(ctx, anchor, &generated_from, out, &path);
                    } else if !generated_from.is_empty() {
                        push(
                            out,
                            "MITASE-GENERATED-001",
                            format!(
                                "non-generated target {} cannot declare generated_from sources",
                                target.id
                            ),
                            &path,
                            Some(anchor.clone()),
                        );
                    }
                }
                if binding.role == BindingRole::Generated
                    && generated_binding_has_cycle(
                        ctx,
                        anchor,
                        &mut BTreeSet::new(),
                        &mut BTreeSet::new(),
                    )
                {
                    push(
                        out,
                        "MITASE-GENERATED-002",
                        "generated binding contains a generated_from cycle",
                        &path,
                        Some(anchor.clone()),
                    );
                }
            }
            AnchorValue::Contract(contract) => {
                if ctx.index.target(&contract.source).is_none() {
                    push(
                        out,
                        "MITASE-CONTRACT-001",
                        "contract source target does not exist",
                        &path,
                        Some(anchor.clone()),
                    );
                } else if ctx
                    .index
                    .bindings
                    .get(&contract.source.binding)
                    .map(|b| b.role)
                    != Some(BindingRole::ContractSource)
                {
                    push(
                        out,
                        "MITASE-CONTRACT-002",
                        "contract source is not owned by a contract-source binding",
                        &path,
                        Some(anchor.clone()),
                    );
                }
                let mut seen_guarantees = BTreeSet::new();
                for guarantee in &contract.guarantees {
                    if !seen_guarantees.insert(guarantee) {
                        push(
                            out,
                            "MITASE-CONTRACT-005",
                            format!("contract guarantee is duplicated: {guarantee}"),
                            &path,
                            Some(anchor.clone()),
                        );
                        continue;
                    }
                    match ctx.index.anchor(guarantee) {
                        None => push(
                            out,
                            "MITASE-CONTRACT-005",
                            format!("contract guarantee target does not exist: {guarantee}"),
                            &path,
                            Some(anchor.clone()),
                        ),
                        Some(AnchorValue::Criterion(_)) | Some(AnchorValue::Rule(_)) => {}
                        Some(_) => push(
                            out,
                            "MITASE-CONTRACT-005",
                            format!(
                                "contract guarantee must reference a rule or criterion: {guarantee}"
                            ),
                            &path,
                            Some(anchor.clone()),
                        ),
                    }
                }
                let mut seen_participants = BTreeSet::new();
                for p in &contract.participants {
                    if p.role.trim().is_empty() {
                        push(
                            out,
                            "MITASE-CONTRACT-007",
                            "contract participant role must not be empty",
                            &path,
                            Some(anchor.clone()),
                        );
                    }
                    if !seen_participants.insert((p.target.clone(), p.role.clone())) {
                        push(
                            out,
                            "MITASE-CONTRACT-007",
                            format!(
                                "contract participant is duplicated: {} {}",
                                p.target, p.role
                            ),
                            &path,
                            Some(anchor.clone()),
                        );
                    }
                    if ctx.index.target(&p.target).is_none() {
                        push(
                            out,
                            "MITASE-CONTRACT-003",
                            format!("contract participant {} does not exist", p.target),
                            &path,
                            Some(anchor.clone()),
                        );
                    }
                }
            }
        }
    }
}

fn validate_generated_target(
    ctx: &ValidationContext<'_>,
    anchor: &SpecAnchor,
    generated_from: &[&BoundTargetRef],
    out: &mut Vec<Diagnostic>,
    path: &str,
) {
    let mut seen = BTreeSet::<BoundTargetRef>::new();
    for generated in generated_from {
        if generated.binding == *anchor {
            push(
                out,
                "MITASE-GENERATED-002",
                format!("generated binding cannot reference itself: {generated}"),
                path,
                Some(anchor.clone()),
            );
            continue;
        }
        if !seen.insert((*generated).clone()) {
            push(
                out,
                "MITASE-GENERATED-002",
                format!("generated_from target is duplicated: {generated}"),
                path,
                Some(anchor.clone()),
            );
        }
        if ctx.index.target(generated).is_none() {
            push(
                out,
                "MITASE-GENERATED-002",
                format!("generated_from target does not exist: {generated}"),
                path,
                Some(anchor.clone()),
            );
        }
    }
}

fn generated_binding_has_cycle(
    ctx: &ValidationContext<'_>,
    anchor: &SpecAnchor,
    visiting: &mut BTreeSet<SpecAnchor>,
    visited: &mut BTreeSet<SpecAnchor>,
) -> bool {
    if !visiting.insert(anchor.clone()) {
        return true;
    }
    if !visited.insert(anchor.clone()) {
        visiting.remove(anchor);
        return false;
    }
    let Some(binding) = ctx.index.bindings.get(anchor) else {
        visiting.remove(anchor);
        return false;
    };
    if binding.role != BindingRole::Generated {
        visiting.remove(anchor);
        return false;
    }
    let cycle = binding
        .targets
        .iter()
        .flat_map(|target| target.claims.iter())
        .filter_map(|claim| match claim {
            mitase_spec_model::TargetClaim::GeneratedFrom { targets } => Some(targets),
            _ => None,
        })
        .flatten()
        .any(|reference| {
            reference.binding == *anchor
                || ctx
                    .index
                    .bindings
                    .get(&reference.binding)
                    .is_some_and(|next| {
                        next.role == BindingRole::Generated
                            && (visiting.contains(&reference.binding)
                                || generated_binding_has_cycle(
                                    ctx,
                                    &reference.binding,
                                    visiting,
                                    visited,
                                ))
                    })
        });
    visiting.remove(anchor);
    cycle
}

fn check_kind(
    ctx: &ValidationContext<'_>,
    out: &mut Vec<Diagnostic>,
    target: &SpecAnchor,
    expected: LocalAnchorKind,
    path: &str,
) {
    if ctx.index.anchor(target).is_some() {
        if target.kind != expected {
            push(
                out,
                "MITASE-ANCHOR-003",
                format!("{target} must reference a {}", expected.label()),
                path,
                Some(target.clone()),
            );
        }
    } else {
        push(
            out,
            "MITASE-ANCHOR-002",
            format!("unresolved anchor {target}"),
            path,
            Some(target.clone()),
        );
    }
}

fn validate_targets(ctx: &ValidationContext<'_>, out: &mut Vec<Diagnostic>) {
    // Planned items are an advisory catalog, not current ownership. Their
    // exact targets can intentionally be absent until an approved Add plan
    // creates them; result validation later proves the planned lifecycle.
    let advisory_absent_targets = ctx
        .index
        .bindings
        .iter()
        .filter(|(anchor, _)| ctx.index.item_status.get(&anchor.item) == Some(&ItemStatus::Planned))
        .flat_map(|(anchor, binding)| {
            binding.targets.iter().map(|target| BoundTargetRef {
                binding: anchor.clone(),
                target_id: target.id.clone(),
            })
        })
        .collect::<BTreeSet<_>>();
    let allowed_absent_targets = ctx
        .work_plan
        .into_iter()
        .flat_map(|plan| plan.slices.iter())
        .flat_map(|slice| {
            slice
                .editable_targets
                .iter()
                .chain(&slice.verification_targets)
                .chain(&slice.readonly_context)
        })
        .filter(|target| target.lifecycle == TargetLifecycle::EnsureAbsent)
        .map(|target| target.reference.clone())
        .collect::<BTreeSet<_>>();
    let known_facets = BTreeSet::<String>::new();
    for (anchor, binding) in &ctx.index.bindings {
        let path = ctx
            .index
            .item_paths
            .get(&anchor.item)
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();
        if !known_facets.is_empty() && !known_facets.contains(&binding.facet) {
            push(
                out,
                "MITASE-FACET-001",
                format!(
                    "facet {} is not defined by an active profile",
                    binding.facet
                ),
                &path,
                Some(anchor.clone()),
            );
        }
        let mut ids = BTreeSet::new();
        for target in &binding.targets {
            if !ids.insert(&target.id) {
                push(
                    out,
                    "MITASE-TARGET-003",
                    format!("duplicate target id {}", target.id),
                    target.path.to_string_lossy(),
                    Some(anchor.clone()),
                );
            }
            match &target.selector {
                Selector::File => {}
                Selector::Symbol { name } => {
                    if name.trim().is_empty() {
                        push(
                            out,
                            "MITASE-TARGET-001",
                            "symbol selector must contain at least one name",
                            target.path.to_string_lossy(),
                            Some(anchor.clone()),
                        );
                    }
                }
                Selector::Heading { value } if value.trim().is_empty() => {
                    push(
                        out,
                        "MITASE-TARGET-001",
                        "heading selector must not be empty",
                        target.path.to_string_lossy(),
                        Some(anchor.clone()),
                    );
                }
                Selector::Marker { value } if value.trim().is_empty() => {
                    push(
                        out,
                        "MITASE-TARGET-001",
                        "marker selector must not be empty",
                        target.path.to_string_lossy(),
                        Some(anchor.clone()),
                    );
                }
                Selector::Operation { method, path }
                    if method.trim().is_empty() || path.trim().is_empty() =>
                {
                    push(
                        out,
                        "MITASE-TARGET-001",
                        "operation selector must not be empty",
                        target.path.to_string_lossy(),
                        Some(anchor.clone()),
                    );
                }
                Selector::JsonPointer { value } if value.trim().is_empty() => {
                    push(
                        out,
                        "MITASE-TARGET-001",
                        "json pointer selector must not be empty",
                        target.path.to_string_lossy(),
                        Some(anchor.clone()),
                    );
                }
                _ => {}
            }
            let expected = match &target.selector {
                Selector::File => true,
                Selector::Symbol { .. } => {
                    matches!(
                        target.adapter.as_str(),
                        "rust" | "typescript" | "javascript" | "shell" | "python" | "go"
                    )
                }
                Selector::Operation { .. } => target.adapter == "openapi",
                Selector::Heading { .. } => target.adapter == "markdown",
                Selector::JsonPointer { .. } => {
                    matches!(
                        target.adapter.as_str(),
                        "yaml" | "json" | "json-schema" | "openapi"
                    )
                }
                Selector::Marker { .. } => true,
            };
            if !expected {
                push(
                    out,
                    "MITASE-TARGET-005",
                    "adapter and selector kind are incompatible",
                    target.path.to_string_lossy(),
                    Some(anchor.clone()),
                );
            }
            if binding.role == BindingRole::Implementation
                && !selector_supports_editable(&target.selector)
                && ctx.preset == ValidationPreset::AgentReady
            {
                push(
                    out,
                    "MITASE-TARGET-004",
                    "implementation target must use an exact editable selector",
                    target.path.to_string_lossy(),
                    Some(anchor.clone()),
                );
            }
            let target_ref = BoundTargetRef {
                binding: anchor.clone(),
                target_id: target.id.clone(),
            };
            let advisory = advisory_absent_targets.contains(&target_ref);
            match resolve_target_in_workspace(ctx.workspace, target) {
                Ok(_) if target.lifecycle == ArtifactTargetLifecycle::Absent && !advisory => {
                    push(
                        out,
                        "MITASE-TARGET-002",
                        "target is declared absent but still resolves in the workspace",
                        target.path.to_string_lossy(),
                        Some(anchor.clone()),
                    );
                }
                Err(e)
                    if target.lifecycle == ArtifactTargetLifecycle::Present
                        && !allowed_absent_targets.contains(&target_ref)
                        && !advisory =>
                {
                    push(
                        out,
                        "MITASE-TARGET-002",
                        e.to_string(),
                        target.path.to_string_lossy(),
                        Some(anchor.clone()),
                    );
                }
                _ => {}
            }
        }
    }
    for (anchor, binding) in &ctx.index.bindings {
        if binding.role != BindingRole::Verification {
            continue;
        }
        for target in &binding.targets {
            let reference = BoundTargetRef {
                binding: anchor.clone(),
                target_id: target.id.clone(),
            };
            for claim in &target.claims {
                let TargetClaim::Verifies {
                    criterion,
                    covers,
                    runner,
                } = claim
                else {
                    continue;
                };
                if covers.is_empty() {
                    push(
                        out,
                        "MITASE-VERIFICATION-001",
                        "verification target must cover at least one implementation target",
                        target.path.to_string_lossy(),
                        Some(anchor.clone()),
                    );
                }
                for covered in covers {
                    let valid = ctx.index.target(covered).is_some_and(|covered_target| {
                        ctx.index.bindings.get(&covered.binding).is_some_and(|covered_binding| {
                            covered_binding.role == BindingRole::Implementation
                                && covered_target.claims.iter().any(|claim| matches!(claim, TargetClaim::Satisfies { criterion: actual } if actual == criterion))
                        })
                    });
                    if !valid {
                        push(
                            out,
                            "MITASE-VERIFICATION-002",
                            format!(
                                "{reference} covers a non-implementation target or a target for another criterion: {covered}"
                            ),
                            target.path.to_string_lossy(),
                            Some(anchor.clone()),
                        );
                    }
                }
                let Some(configured) = ctx.config.verification.runners.get(&runner.runner) else {
                    push(
                        out,
                        "MITASE-VERIFICATION-002",
                        format!("verification runner {} is not configured", runner.runner),
                        target.path.to_string_lossy(),
                        Some(anchor.clone()),
                    );
                    continue;
                };
                for argument in &configured.arguments {
                    let placeholders = argument
                        .match_indices('{')
                        .filter_map(|(start, _)| {
                            let end = argument[start + 1..].find('}')? + start + 1;
                            Some(&argument[start + 1..end])
                        })
                        .collect::<Vec<_>>();
                    if placeholders
                        .iter()
                        .any(|key| runner.arguments.get(*key).is_none_or(String::is_empty))
                    {
                        push(
                            out,
                            "MITASE-VERIFICATION-002",
                            format!(
                                "verification runner argument is not exactly configured: {argument}"
                            ),
                            target.path.to_string_lossy(),
                            Some(anchor.clone()),
                        );
                    }
                }
            }
        }
    }
    for (criterion, implementations) in &ctx.index.criteria_to_implementation_targets {
        if ctx.index.criterion_status.get(criterion) != Some(&ItemStatus::Implemented) {
            continue;
        }
        for implementation in implementations {
            if !ctx.index.criteria_to_verification_targets.get(criterion).into_iter().flatten().any(|verification| {
                ctx.index.target(verification).is_some_and(|target| target.claims.iter().any(|claim| matches!(claim, TargetClaim::Verifies { criterion: actual, covers, .. } if actual == criterion && covers.contains(implementation))))
            }) {
                push(out, "MITASE-VERIFICATION-002", format!("implementation target {implementation} is not covered by a verification target for {criterion}"), "workspace", Some(criterion.clone()));
            }
        }
    }
}

fn validate_contracts(ctx: &ValidationContext<'_>, out: &mut Vec<Diagnostic>) {
    for (criterion, implementations) in &ctx.index.criteria_to_implementations {
        if ctx.index.criterion_status.get(criterion) != Some(&ItemStatus::Implemented) {
            continue;
        }
        let facets = implementations
            .iter()
            .filter_map(|anchor| ctx.index.bindings.get(anchor))
            .map(|binding| binding.facet.as_str())
            .collect::<BTreeSet<_>>();
        if facets.len() > 1 {
            let connected = ctx.index.contracts.values().any(|contract| {
                implementations.iter().all(|implementation| {
                    contract
                        .participants
                        .iter()
                        .any(|participant| &participant.target.binding == implementation)
                })
            });
            if !connected {
                push(
                    out,
                    "MITASE-CONTRACT-006",
                    "cross-facet implementations of a criterion are not connected by one contract",
                    "workspace",
                    Some(criterion.clone()),
                );
            }
        }
    }
}

fn validate_plan(ctx: &ValidationContext<'_>, plan: &WorkPlan, out: &mut Vec<Diagnostic>) {
    if plan.schema != WORK_PLAN_SCHEMA {
        push(
            out,
            "MITASE-SCHEMA-001",
            format!("plan schema must be {WORK_PLAN_SCHEMA}"),
            "work-plan",
            None,
        );
    }
    let allow_post_state = ctx.plan_mode == PlanValidationMode::PostState;
    if !allow_post_state
        && ctx
            .revision
            .is_some_and(|revision| plan.basis.revision != revision)
    {
        push(
            out,
            "MITASE-WORK-009",
            "plan basis revision is stale",
            "work-plan",
            None,
        );
    }
    let basis_workspace = load_workspace_at_revision(&ctx.workspace.root, &plan.basis.revision);
    if plan.basis.spec_fingerprint != ctx.workspace.spec_fingerprint().unwrap_or_default() {
        push(
            out,
            "MITASE-WORK-009",
            "plan specification basis is stale",
            "work-plan",
            None,
        );
    }
    if plan.basis.ownership_fingerprint != lifecycle_ownership_fingerprint(ctx.index, plan) {
        push(
            out,
            "MITASE-WORK-009",
            "plan ownership basis is stale",
            "work-plan",
            None,
        );
    }
    if plan.basis.readonly_fingerprint
        != current_readonly_fingerprint(ctx.workspace, ctx.index, plan)
    {
        push(
            out,
            "MITASE-WORK-009",
            "plan readonly or run-only target basis is stale",
            "work-plan",
            None,
        );
    }
    // A Workbench plan may be created against the current working tree while
    // its revision still names HEAD.  Prefer the live indexed workspace when
    // its fingerprint is the submitted basis; reconstructing HEAD here would
    // incorrectly compare a valid dirty-tree plan with an older filesystem.
    let current_workspace_is_basis = plan.basis.workspace_fingerprint
        == ctx.workspace.try_fingerprint().unwrap_or_default()
        && !plan_has_lifecycle_transition(plan);
    if basis_workspace.is_none() && !current_workspace_is_basis && !allow_post_state {
        push(
            out,
            "MITASE-WORK-009",
            "plan basis revision cannot be reconstructed",
            "work-plan",
            None,
        );
    }
    if plan.canonical_digest != work_plan_digest(plan) {
        push(
            out,
            "MITASE-WORK-009",
            "plan canonical digest is tampered",
            "work-plan",
            None,
        );
    }
    let canonical_basis = if current_workspace_is_basis {
        Some((ctx.workspace, ctx.index))
    } else {
        basis_workspace
            .as_ref()
            .map(|basis| (&basis.workspace, &basis.index))
    };
    if let Some((basis_workspace, basis_index)) = canonical_basis {
        match canonical_plan(
            &plan.request,
            basis_workspace,
            basis_index,
            &plan.basis.revision,
        ) {
            Ok(mut canonical) => {
                canonical.basis = plan.basis.clone();
                canonical.canonical_digest = work_plan_digest(&canonical);
                let structures_match = canonical.status == plan.status
                    && canonical.slices == plan.slices
                    && canonical.diagnostics == plan.diagnostics
                    && canonical.canonical_digest == plan.canonical_digest;
                if !structures_match {
                    push(
                        out,
                        "MITASE-WORK-009",
                        "plan structure does not match the canonical planner output",
                        "work-plan",
                        None,
                    );
                }
            }
            Err(error) => push(
                out,
                "MITASE-WORK-009",
                format!("plan request no longer replans cleanly: {error:#}"),
                "work-plan",
                None,
            ),
        }
    }
    if plan.execution != PlanExecution::IsolatedSlices {
        push(
            out,
            "MITASE-WORK-009",
            "work plan execution mode must be isolated-slices",
            "work-plan",
            None,
        );
    }
    if allow_post_state && plan.slices.len() > 1 && ctx.selected_slice.is_none() {
        push(
            out,
            "MITASE-WORK-009",
            "post-state validation requires --slice-id when a plan has multiple slices",
            "work-plan",
            None,
        );
    }
    if allow_post_state
        && let (Some(actual), Some(reported)) = (ctx.changed_files, ctx.reported_changed_files)
        && !reported_change_set_covers(actual, reported)
    {
        push(
            out,
            "MITASE-WORK-009",
            "reported range does not cover all actual post-state changes from the plan basis",
            "work-plan",
            None,
        );
    }
    let mut slice_ids = BTreeSet::new();
    let slices: Vec<&ExecutionSlice> = ctx
        .selected_slice
        .map_or_else(|| plan.slices.iter().collect(), |s| vec![s]);
    for slice in slices {
        if !slice_ids.insert(slice.id.as_str()) {
            push(
                out,
                "MITASE-WORK-009",
                format!("duplicate slice id: {}", slice.id),
                "work-plan",
                None,
            );
        }
        if slice.completion.is_empty() {
            push(
                out,
                "MITASE-WORK-011",
                "slice has no completion check",
                "work-plan",
                None,
            );
        }
        if slice.confidence == PlanConfidence::Low && !slice.editable_targets.is_empty() {
            push(
                out,
                "MITASE-WORK-010",
                "low-confidence target cannot be executable",
                "work-plan",
                None,
            );
        }
        let limits = &ctx.config.work.slicing;
        let all_targets = slice
            .editable_targets
            .iter()
            .chain(&slice.verification_targets)
            .chain(&slice.readonly_context);
        let actual_bytes: usize = all_targets.clone().map(target_budget_bytes).sum();
        let actual_files = slice
            .editable_targets
            .iter()
            .chain(&slice.verification_targets)
            .filter(|target| target.access == mitase_work_model::TargetAccessMode::Editable)
            .map(|target| target.resolved_path.as_str())
            .collect::<BTreeSet<_>>()
            .len();
        let actual_symbols: usize = slice
            .editable_targets
            .iter()
            .chain(&slice.verification_targets)
            .filter(|target| target.access == mitase_work_model::TargetAccessMode::Editable)
            .map(|target| target.resolved_selector.symbols.len())
            .sum();
        if slice.budget.editable_files != actual_files
            || slice.budget.editable_symbols != actual_symbols
            || slice.budget.verification_targets != slice.verification_targets.len()
            || slice.budget.readonly_targets != slice.readonly_context.len()
            || slice.budget.total_bytes != actual_bytes
        {
            push(
                out,
                "MITASE-WORK-009",
                "plan budget snapshot is tampered",
                "work-plan",
                None,
            );
        }
        if slice.budget.editable_files > limits.max_editable_files
            || slice.budget.editable_symbols > limits.max_editable_symbols
            || slice.budget.verification_targets > limits.max_verification_targets
            || slice.budget.readonly_targets > limits.max_readonly_targets
            || slice.budget.total_bytes > limits.max_total_bytes
        {
            push(
                out,
                "MITASE-WORK-003",
                "slice exceeds configured budget",
                "work-plan",
                None,
            );
        }
        for target in slice
            .editable_targets
            .iter()
            .chain(&slice.verification_targets)
            .chain(&slice.readonly_context)
        {
            match resolve_planned_target(ctx, target) {
                Some(resolved)
                    if target.lifecycle == TargetLifecycle::EnsurePresent
                        && resolved.path.to_string_lossy() == target.resolved_path
                        && resolved.description == target.resolved_selector.description
                        && resolved.symbols == target.resolved_selector.symbols
                        && planned_target_metadata_matches(ctx, target) =>
                {
                    if ensure_present_target_exceeds_budget(target, &resolved) {
                        push(
                            out,
                            "MITASE-WORK-003",
                            format!(
                                "ensure-present target exceeds planned post-state budget: {}",
                                target.reference
                            ),
                            &target.resolved_path,
                            Some(target.reference.binding.clone()),
                        );
                    }
                }
                None if target.lifecycle == TargetLifecycle::EnsureAbsent => {}
                Some(resolved)
                    if allow_post_state
                        && target.lifecycle == TargetLifecycle::Stable
                        && target.access == mitase_work_model::TargetAccessMode::Editable
                        && resolved.path.to_string_lossy() == target.resolved_path
                        && resolved.description == target.resolved_selector.description
                        && resolved.symbols == target.resolved_selector.symbols
                        && planned_target_metadata_matches(ctx, target) => {}
                Some(resolved)
                    if allow_post_state
                        && target.lifecycle == TargetLifecycle::Stable
                        && matches!(
                            target.access,
                            mitase_work_model::TargetAccessMode::Readonly
                                | mitase_work_model::TargetAccessMode::RunOnly
                        )
                        && lifecycle_transition_shares_path(slice, target)
                        && (resolved.excerpt_hash == target.excerpt_hash
                            || (target.access == mitase_work_model::TargetAccessMode::RunOnly
                                && target.content_hash.is_empty()
                                && target.excerpt_hash.is_empty()))
                        && resolved.path.to_string_lossy() == target.resolved_path
                        && resolved.description == target.resolved_selector.description
                        && resolved.symbols == target.resolved_selector.symbols
                        && planned_target_metadata_matches(ctx, target) => {}
                Some(resolved)
                    if allow_post_state
                        && target.lifecycle == TargetLifecycle::Stable
                        && target.access == mitase_work_model::TargetAccessMode::Generated
                        && ctx.changed_files.is_some_and(|files| {
                            generated_target_has_changed_source(ctx, slice, target, files)
                        })
                        && resolved.path.to_string_lossy() == target.resolved_path
                        && resolved.description == target.resolved_selector.description
                        && resolved.symbols == target.resolved_selector.symbols
                        && planned_target_metadata_matches(ctx, target) => {}
                Some(resolved)
                    if resolved.content_hash == target.content_hash
                        && resolved.excerpt_hash == target.excerpt_hash
                        && resolved.path.to_string_lossy() == target.resolved_path
                        && resolved.description == target.resolved_selector.description
                        && resolved.symbols == target.resolved_selector.symbols
                        && resolved.byte_start == target.byte_start
                        && resolved.byte_end == target.byte_end
                        && resolved.line_start == target.line_start
                        && resolved.line_end == target.line_end
                        && planned_target_metadata_matches(ctx, target) => {}
                _ => push(
                    out,
                    "MITASE-WORK-009",
                    format!("target snapshot is stale: {}", target.reference),
                    &target.resolved_path,
                    Some(target.reference.binding.clone()),
                ),
            }
        }
        if allow_post_state && let Some(changed_files) = ctx.changed_files {
            for target in slice
                .editable_targets
                .iter()
                .filter(|target| target.transition == mitase_work_model::TargetTransition::Modify)
            {
                if !planned_target_changed(ctx, target, changed_files) {
                    push(
                        out,
                        "MITASE-WORK-011",
                        format!(
                            "expected modified target is unchanged: {}",
                            target.reference
                        ),
                        &target.resolved_path,
                        Some(target.reference.binding.clone()),
                    );
                }
            }
        }
        for completion in &slice.completion {
            match completion {
                mitase_work_model::CompletionCheck::TargetExists { target } => {
                    if ctx
                        .index
                        .target(target)
                        .and_then(|declared| {
                            resolve_target_in_workspace(ctx.workspace, declared).ok()
                        })
                        .is_none()
                    {
                        push(
                            out,
                            "MITASE-WORK-011",
                            format!("expected target is still missing: {target}"),
                            "work-plan",
                            Some(target.binding.clone()),
                        );
                    }
                }
                mitase_work_model::CompletionCheck::TargetAbsent { target }
                    if ctx
                        .index
                        .target(target)
                        .and_then(|declared| {
                            resolve_target_in_workspace(ctx.workspace, declared).ok()
                        })
                        .is_some() =>
                {
                    push(
                        out,
                        "MITASE-WORK-011",
                        format!("expected removed target still exists: {target}"),
                        "work-plan",
                        Some(target.binding.clone()),
                    );
                }
                _ => {}
            }
        }
        for required in slice
            .acceptance
            .iter()
            .filter_map(|a| ctx.index.criteria_to_verifications.get(&a.anchor))
            .flatten()
        {
            if !slice
                .verification_targets
                .iter()
                .any(|target| &target.reference.binding == required)
            {
                push(
                    out,
                    "MITASE-WORK-007",
                    format!("required verification binding is missing: {required}"),
                    "work-plan",
                    Some(required.clone()),
                );
            }
        }
        for contract_anchor in &slice.contracts {
            if let Some(contract) = ctx.index.contracts.get(contract_anchor) {
                for participant in &contract.participants {
                    if !slice.anchors.contains(&participant.target.binding)
                        && !slice
                            .readonly_context
                            .iter()
                            .any(|target| target.reference == participant.target)
                        && !slice
                            .verification_targets
                            .iter()
                            .any(|target| target.reference == participant.target)
                    {
                        push(
                            out,
                            "MITASE-WORK-008",
                            format!("contract counterpart is absent: {}", participant.target),
                            "work-plan",
                            Some(contract_anchor.clone()),
                        );
                    }
                }
            }
        }
        if let Some(files) = ctx.changed_files {
            validate_slice_scope(ctx, files, slice, out);
        }
        for acceptance in &slice.acceptance {
            if let Some(AnchorValue::Criterion(c)) = ctx.index.anchor(&acceptance.anchor)
                && c.statement != acceptance.statement
            {
                push(
                    out,
                    "MITASE-WORK-012",
                    "acceptance statement differs from criterion",
                    "work-plan",
                    Some(acceptance.anchor.clone()),
                );
            }
        }
    }
}

fn lifecycle_transition_shares_path(
    slice: &ExecutionSlice,
    target: &mitase_work_model::PlannedTarget,
) -> bool {
    slice.editable_targets.iter().any(|editable| {
        matches!(
            editable.transition,
            TargetTransition::Add | TargetTransition::Remove
        ) && editable.resolved_path == target.resolved_path
    })
}

fn run_only_target_is_post_state_add(
    slice: &ExecutionSlice,
    target: &mitase_work_model::PlannedTarget,
) -> bool {
    target.access == mitase_work_model::TargetAccessMode::RunOnly
        && target.content_hash.is_empty()
        && target.excerpt_hash.is_empty()
        && lifecycle_transition_shares_path(slice, target)
}

fn target_budget_bytes(target: &mitase_work_model::PlannedTarget) -> usize {
    target
        .budget_bytes
        .max(target.byte_end.saturating_sub(target.byte_start))
}

fn validate_slice_scope(
    ctx: &ValidationContext<'_>,
    files: &[ChangedFile],
    slice: &ExecutionSlice,
    out: &mut Vec<Diagnostic>,
) {
    let editable_targets = slice
        .editable_targets
        .iter()
        .chain(&slice.verification_targets)
        .filter(|target| target.access == mitase_work_model::TargetAccessMode::Editable)
        .collect::<Vec<_>>();
    let guarded_targets = slice
        .verification_targets
        .iter()
        .filter(|target| {
            target.access == mitase_work_model::TargetAccessMode::RunOnly
                && !run_only_target_is_post_state_add(slice, target)
        })
        .chain(
            slice
                .readonly_context
                .iter()
                .filter(|target| target.access != mitase_work_model::TargetAccessMode::Generated),
        )
        .collect::<Vec<_>>();
    let generated_targets = slice
        .readonly_context
        .iter()
        .filter(|target| target.access == mitase_work_model::TargetAccessMode::Generated)
        .collect::<Vec<_>>();
    for file in files {
        let Some(path) = file.new_path.as_ref().or(file.old_path.as_ref()) else {
            continue;
        };
        if !ctx.workspace.path_is_artifact(path.as_path())
            || ctx.workspace.path_is_excluded(path.as_path())
        {
            continue;
        }
        if file.hunks.is_empty() {
            let readonly_hit = guarded_targets
                .iter()
                .any(|target| target_matches_changed_file_path(ctx, target, file));
            let editable_hit = editable_targets.iter().any(|target| {
                editable_target_matches_hunkless_change(ctx, target, file)
                    || editable_add_target_matches_file(ctx, target, file)
            });
            let generated_hit = generated_targets.iter().any(|target| {
                target_matches_changed_file_path(ctx, target, file)
                    && generated_target_has_changed_source(ctx, slice, target, files)
            });
            let unbacked_generated_hit = generated_targets
                .iter()
                .any(|target| target_matches_changed_file_path(ctx, target, file))
                && !generated_hit;
            if readonly_hit || unbacked_generated_hit {
                push(
                    out,
                    "MITASE-WORK-005",
                    format!("readonly or run-only target changed: {}", path.display()),
                    path.to_string_lossy(),
                    None,
                );
            } else if !editable_hit && !generated_hit {
                push(
                    out,
                    "MITASE-WORK-006",
                    format!("change is outside editable scope: {}", path.display()),
                    path.to_string_lossy(),
                    None,
                );
            }
            continue;
        }
        let hunks = file.hunks.clone();
        for hunk in hunks {
            let readonly_hit = guarded_targets
                .iter()
                .any(|target| target_overlaps_change(ctx, target, file, &hunk));
            let editable_hit = change_is_within_editable_scope(ctx, &editable_targets, file, &hunk);
            let generated_hit = generated_targets.iter().any(|target| {
                target_overlaps_change(ctx, target, file, &hunk)
                    && generated_target_has_changed_source(ctx, slice, target, files)
            });
            let unbacked_generated_hit = generated_targets
                .iter()
                .any(|target| target_overlaps_change(ctx, target, file, &hunk))
                && !generated_hit;
            if readonly_hit || unbacked_generated_hit {
                push(
                    out,
                    "MITASE-WORK-005",
                    format!("readonly or run-only target changed: {}", path.display()),
                    path.to_string_lossy(),
                    None,
                );
            } else if !editable_hit && !generated_hit {
                push(
                    out,
                    "MITASE-WORK-006",
                    format!("change is outside editable scope: {}", path.display()),
                    path.to_string_lossy(),
                    None,
                );
            }
        }
    }
}

fn generated_target_has_changed_source(
    ctx: &ValidationContext<'_>,
    slice: &ExecutionSlice,
    generated: &mitase_work_model::PlannedTarget,
    files: &[ChangedFile],
) -> bool {
    ctx.index
        .generated_from
        .get(&generated.reference)
        .into_iter()
        .flatten()
        .any(|source| {
            slice
                .editable_targets
                .iter()
                .find(|target| {
                    target.reference == *source
                        && target.access == mitase_work_model::TargetAccessMode::Editable
                })
                .is_some_and(|target| planned_target_changed(ctx, target, files))
        })
}

fn editable_add_target_matches_file(
    ctx: &ValidationContext<'_>,
    target: &mitase_work_model::PlannedTarget,
    file: &ChangedFile,
) -> bool {
    target.transition == mitase_work_model::TargetTransition::Add
        && target.content_hash.is_empty()
        && target.excerpt_hash.is_empty()
        && target_selector_is_file(target)
        && target_matches_changed_file_path(ctx, target, file)
}

fn target_matches_changed_file_path(
    ctx: &ValidationContext<'_>,
    target: &mitase_work_model::PlannedTarget,
    file: &ChangedFile,
) -> bool {
    file.old_path
        .as_ref()
        .and_then(|path| target_line_range(ctx, target, TargetRangeSide::Old, path))
        .is_some()
        || file
            .new_path
            .as_ref()
            .and_then(|path| target_line_range(ctx, target, TargetRangeSide::New, path))
            .is_some()
}

fn planned_target_changed(
    ctx: &ValidationContext<'_>,
    target: &mitase_work_model::PlannedTarget,
    changed_files: &[ChangedFile],
) -> bool {
    changed_files.iter().any(|file| {
        if file.hunks.is_empty() {
            target_matches_changed_file_path(ctx, target, file)
        } else {
            file.hunks
                .iter()
                .any(|hunk| target_overlaps_change(ctx, target, file, hunk))
        }
    })
}

fn editable_target_matches_hunkless_change(
    ctx: &ValidationContext<'_>,
    target: &mitase_work_model::PlannedTarget,
    file: &ChangedFile,
) -> bool {
    target_selector_is_file(target) && target_matches_changed_file_path(ctx, target, file)
}

fn change_is_within_editable_scope(
    ctx: &ValidationContext<'_>,
    editable_targets: &[&mitase_work_model::PlannedTarget],
    file: &ChangedFile,
    hunk: &ChangedRange,
) -> bool {
    let old_ok = match file.old_path.as_ref() {
        Some(path) => changed_side_is_fully_covered(
            hunk.old_start,
            hunk.old_end,
            editable_targets
                .iter()
                .filter_map(|target| target_line_range(ctx, target, TargetRangeSide::Old, path)),
        ),
        None => true,
    };
    let new_ok = match file.new_path.as_ref() {
        Some(path) => {
            changed_side_is_fully_covered(
                hunk.new_start,
                hunk.new_end,
                editable_targets.iter().filter_map(|target| {
                    target_line_range(ctx, target, TargetRangeSide::New, path)
                }),
            ) || editable_targets.iter().any(|target| {
                target.transition == TargetTransition::Add
                    && target.container_content_hash.is_some()
                    && hunk.old_start == hunk.old_end
                    && target_line_range(ctx, target, TargetRangeSide::New, path).is_some_and(
                        |range| changed_side_overlaps(hunk.new_start, hunk.new_end, range),
                    )
            })
        }
        None => true,
    };
    old_ok && new_ok
}

fn changed_side_is_fully_covered(
    changed_start: usize,
    changed_end: usize,
    ranges: impl Iterator<Item = (usize, usize)>,
) -> bool {
    if changed_start == 0 && changed_end == 0 {
        return true;
    }
    let mut ranges = ranges.collect::<Vec<_>>();
    if ranges.is_empty() {
        return false;
    }
    if ranges
        .iter()
        .any(|range| range.0 == 0 && range.1 == usize::MAX)
    {
        return true;
    }
    ranges.sort_unstable_by_key(|range| range.0);
    let changed_end = normalize_end(changed_start, changed_end);
    let mut covered_until = changed_start.saturating_sub(1);
    for (start, end) in ranges {
        let end = normalize_end(start, end);
        if start > covered_until.saturating_add(1) {
            continue;
        }
        covered_until = covered_until.max(end);
        if covered_until >= changed_end {
            return true;
        }
    }
    false
}

fn target_overlaps_change(
    ctx: &ValidationContext<'_>,
    target: &mitase_work_model::PlannedTarget,
    file: &ChangedFile,
    hunk: &ChangedRange,
) -> bool {
    file.old_path
        .as_ref()
        .and_then(|path| target_line_range(ctx, target, TargetRangeSide::Old, path))
        .is_some_and(|range| changed_side_overlaps(hunk.old_start, hunk.old_end, range))
        || file
            .new_path
            .as_ref()
            .and_then(|path| target_line_range(ctx, target, TargetRangeSide::New, path))
            .is_some_and(|range| changed_side_overlaps(hunk.new_start, hunk.new_end, range))
}

#[derive(Debug, Clone, Copy)]
enum TargetRangeSide {
    Old,
    New,
}

fn target_line_range(
    ctx: &ValidationContext<'_>,
    target: &mitase_work_model::PlannedTarget,
    side: TargetRangeSide,
    changed_path: &RepoPath,
) -> Option<(usize, usize)> {
    if let Some(identity) = &target.artifact_identity {
        // The plan is the pre-state authority.  The identity can be absent or
        // moved after a rename/deletion, so never use the post-state inventory
        // to judge an old-side hunk.
        if matches!(side, TargetRangeSide::Old) {
            return (target.resolved_path == changed_path.to_string_lossy())
                .then_some((target.line_start, target.line_end));
        }
        let unit = ctx
            .index
            .artifact_units
            .iter()
            .find(|unit| &unit.identity == identity)?;
        let resolved = resolve_artifact_unit(ctx.workspace, unit).ok()?;
        return (resolved.path.to_string_lossy() == changed_path.to_string_lossy())
            .then_some((resolved.line_start, resolved.line_end));
    }
    let current = if matches!(
        target.lifecycle,
        TargetLifecycle::Stable | TargetLifecycle::EnsurePresent
    ) {
        ctx.index
            .target(&target.reference)
            .and_then(|declared| resolve_target_in_workspace(ctx.workspace, declared).ok())
    } else {
        None
    };
    let current_path = current
        .as_ref()
        .map(|resolved| resolved.path.to_string_lossy().into_owned());
    let path_matches = match side {
        TargetRangeSide::Old => target.resolved_path == changed_path.to_string_lossy(),
        TargetRangeSide::New => {
            current_path.as_deref().unwrap_or(&target.resolved_path)
                == changed_path.to_string_lossy()
        }
    };
    if !path_matches {
        return None;
    }
    let description = current
        .as_ref()
        .map(|resolved| resolved.description.as_str())
        .unwrap_or(target.resolved_selector.description.as_str());
    if description == "file" {
        return Some((0, usize::MAX));
    }
    Some(match side {
        TargetRangeSide::Old => (target.line_start, target.line_end),
        TargetRangeSide::New => current
            .as_ref()
            .map(|resolved| (resolved.line_start, resolved.line_end))
            .unwrap_or((target.line_start, target.line_end)),
    })
}

fn resolve_planned_target(
    ctx: &ValidationContext<'_>,
    target: &mitase_work_model::PlannedTarget,
) -> Option<ResolvedTarget> {
    if let Some(identity) = &target.artifact_identity {
        let unit = ctx
            .index
            .artifact_units
            .iter()
            .find(|unit| &unit.identity == identity)?;
        return resolve_artifact_unit(ctx.workspace, unit).ok();
    }
    let declared = ctx.index.target(&target.reference)?;
    if let Some(identity) = ctx.index.target_to_artifact.get(&target.reference)
        && let Some(unit) = ctx
            .index
            .artifact_units
            .iter()
            .find(|unit| &unit.identity == identity)
        && let Ok(Some(resolved)) = resolve_indexed_target(ctx.workspace, declared, unit)
    {
        return Some(resolved);
    }
    resolve_target_in_workspace(ctx.workspace, declared).ok()
}

fn planned_target_metadata_matches(
    ctx: &ValidationContext<'_>,
    target: &mitase_work_model::PlannedTarget,
) -> bool {
    let Some(binding) = ctx.index.bindings.get(&target.reference.binding) else {
        return false;
    };
    let Some(declared) = ctx.index.target(&target.reference) else {
        return false;
    };
    if binding.facet != target.facet
        || binding.role != target.role
        || declared.adapter != target.adapter
    {
        return false;
    }
    target.artifact_identity.as_ref().is_none_or(|identity| {
        ctx.index
            .artifact_owners
            .get(identity)
            .is_some_and(|owners| {
                owners
                    .iter()
                    .any(|owner| owner.binding == target.reference.binding)
            })
    })
}

fn target_selector_is_file(target: &mitase_work_model::PlannedTarget) -> bool {
    target.resolved_selector.description == "file"
}

fn ensure_present_target_exceeds_budget(
    target: &mitase_work_model::PlannedTarget,
    resolved: &ResolvedTarget,
) -> bool {
    let actual_bytes = resolved.byte_end.saturating_sub(resolved.byte_start);
    if actual_bytes > target.budget_bytes {
        return true;
    }
    let planned_lines = target.budget_lines.unwrap_or_else(|| {
        target
            .line_end
            .saturating_sub(target.line_start)
            .saturating_add(1)
    });
    let actual_lines = resolved
        .line_end
        .saturating_sub(resolved.line_start)
        .saturating_add(1);
    actual_lines > planned_lines
}

fn changed_side_overlaps(changed_start: usize, changed_end: usize, target: (usize, usize)) -> bool {
    if changed_start == 0 && changed_end == 0 {
        return false;
    }
    let changed_end = normalize_end(changed_start, changed_end);
    let target_end = normalize_end(target.0, target.1);
    changed_start <= target_end && target.0 <= changed_end
}

fn normalize_end(start: usize, end: usize) -> usize {
    if end == 0 { start } else { end }
}

fn reported_change_set_covers(actual: &[ChangedFile], reported: &[ChangedFile]) -> bool {
    actual.iter().all(|actual_file| {
        reported.iter().any(|reported_file| {
            same_changed_file_identity(actual_file, reported_file)
                && (actual_file.hunks.is_empty()
                    || reported_file.hunks.is_empty()
                    || actual_file.hunks.iter().all(|actual_hunk| {
                        reported_file
                            .hunks
                            .iter()
                            .any(|reported_hunk| changed_range_covers(actual_hunk, reported_hunk))
                    }))
        })
    })
}

fn same_changed_file_identity(left: &ChangedFile, right: &ChangedFile) -> bool {
    left.old_path == right.old_path && left.new_path == right.new_path
}

fn changed_range_covers(actual: &ChangedRange, reported: &ChangedRange) -> bool {
    changed_side_covers(
        actual.old_start,
        actual.old_end,
        reported.old_start,
        reported.old_end,
    ) && changed_side_covers(
        actual.new_start,
        actual.new_end,
        reported.new_start,
        reported.new_end,
    )
}

fn changed_side_covers(
    actual_start: usize,
    actual_end: usize,
    reported_start: usize,
    reported_end: usize,
) -> bool {
    if actual_start == 0 && actual_end == 0 {
        return true;
    }
    if reported_start == 0 && reported_end == 0 {
        return false;
    }
    let actual_end = normalize_end(actual_start, actual_end);
    let reported_end = normalize_end(reported_start, reported_end);
    reported_start <= actual_start && reported_end >= actual_end
}

#[cfg(test)]
mod tests {
    use super::*;
    use mitase_work_model::TargetTransition;
    use std::fs;
    use std::process::Command;
    use tempfile::tempdir;

    fn fixture_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/v1/valid-web-app")
            .canonicalize()
            .expect("fixture root")
    }

    fn workbench_fixture_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/v1/valid-workbench-flow")
            .canonicalize()
            .expect("Workbench fixture root")
    }

    fn copy_dir(from: &Path, to: &Path) {
        fs::create_dir_all(to).expect("create dir");
        for entry in fs::read_dir(from).expect("read dir") {
            let entry = entry.expect("entry");
            let path = entry.path();
            let destination = to.join(entry.file_name());
            if entry.file_type().expect("file type").is_dir() {
                copy_dir(&path, &destination);
            } else {
                fs::copy(&path, &destination).expect("copy file");
            }
        }
    }

    fn load_fixture_workspace() -> (tempfile::TempDir, SpecWorkspace, SpecIndex) {
        let tempdir = tempdir().expect("tempdir");
        copy_dir(&fixture_root(), tempdir.path());
        let workspace = SpecWorkspace::load(tempdir.path()).expect("workspace");
        let index = workspace.index().expect("index");
        (tempdir, workspace, index)
    }

    fn validate_loaded_workspace(workspace: &SpecWorkspace, index: &SpecIndex) -> ValidationResult {
        validate_without_readiness(&ValidationContext {
            config: &workspace.config,
            workspace,
            index,
            changed_files: None,
            reported_changed_files: None,
            work_plan: None,
            selected_slice: None,
            plan_mode: PlanValidationMode::PreState,
            preset: workspace.config.validation.preset,
            revision: None,
            change_base_revision: None,
        })
    }

    fn write_generated_binding_workspace(root: &Path) {
        fs::create_dir_all(root.join("spec")).expect("spec dir");
        fs::create_dir_all(root.join("src")).expect("src dir");
        fs::write(
            root.join("mitase.yaml"),
            concat!(
                "schema: mitase/config/v1\n",
                "workspace:\n",
                "  spec_roots: [spec]\n",
                "  excludes: []\n",
                "inventory:\n",
                "  active_profile: default\n",
                "  profiles:\n",
                "    - id: default\n",
                "      providers: { rust: {} }\n",
                "validation:\n",
                "  preset: standard\n",
                "  readiness:\n",
                "    target: off\n",
                "    limits: { max_ownership_scope_units: 64, max_targets_per_binding: 12, max_slices_per_origin: 4 }\n",
                "  changed:\n",
                "    require_owned_changes: false\n",
                "    require_plan: false\n",
                "verification: { runners: {} }\n",
                "work:\n",
                "  slicing:\n",
                "    max_editable_files: 2\n",
                "    max_editable_symbols: 4\n",
                "    max_verification_targets: 2\n",
                "    max_readonly_targets: 2\n",
                "    max_total_bytes: 4096\n",
            ),
        )
        .expect("config");
        fs::write(root.join("src/generated.rs"), "pub fn generated() {}\n").expect("artifact");
        fs::write(
            root.join("spec/feature.yaml"),
            concat!(
                "schema: mitase/spec/v1\n",
                "kind: features\n",
                "namespace: sample\n",
                "category: Sample\n",
                "features:\n",
                "  - id: FEAT-TEST-001\n",
                "    title: Test\n",
                "    summary: Test feature.\n",
                "    status: implemented\n",
                "    bindings:\n",
                "      - id: gen\n",
                "        role: generated\n",
                "        facet: backend\n",
                "        responsibility: Generated artifact.\n",
                "        targets:\n",
                "          - { id: generated-file, adapter: rust, path: src/generated.rs, selector: { kind: marker, value: 'pub fn generated' }, claims: [] }\n",
            ),
        )
        .expect("feature spec");
    }

    fn init_git_repo(root: &Path) -> String {
        let run = |args: &[&str]| {
            let status = Command::new("git")
                .args(args)
                .current_dir(root)
                .status()
                .unwrap();
            assert!(status.success(), "git {:?} failed", args);
        };
        run(&["init"]);
        run(&["config", "user.name", "Codex"]);
        run(&["config", "user.email", "codex@example.com"]);
        run(&["add", "."]);
        run(&["commit", "-m", "baseline"]);
        let output = Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(root)
            .output()
            .unwrap();
        assert!(output.status.success());
        String::from_utf8(output.stdout).unwrap().trim().to_string()
    }

    fn git_revision(root: &Path) -> String {
        let output = Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(root)
            .output()
            .expect("read revision");
        assert!(output.status.success(), "read revision");
        String::from_utf8(output.stdout)
            .expect("revision text")
            .trim()
            .to_string()
    }

    fn git_commit(root: &Path, message: &str) -> String {
        for args in [vec!["add", "."], vec!["commit", "-qm", message]] {
            let status = Command::new("git")
                .args(args)
                .current_dir(root)
                .status()
                .unwrap();
            assert!(status.success(), "git commit step failed");
        }
        let output = Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(root)
            .output()
            .unwrap();
        assert!(output.status.success());
        String::from_utf8(output.stdout).unwrap().trim().to_string()
    }

    #[test]
    fn identity_cutover_requires_explicit_legacy_baseline() {
        let repository = tempdir().expect("temporary repository");
        let valid_config =
            fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../mitase.yaml"))
                .expect("canonical config");
        let legacy_config = valid_config.replace("mitase", "legacy");
        let parsed_legacy: ProjectConfig =
            serde_yaml::from_str(&legacy_config).expect("complete legacy config");
        assert_eq!(parsed_legacy.schema, "legacy/config/v1");
        assert!(!parsed_legacy.workspace.spec_roots.is_empty());
        assert!(
            parsed_legacy
                .inventory
                .profiles
                .iter()
                .any(|profile| profile.id == parsed_legacy.inventory.active_profile)
        );
        fs::write(repository.path().join("legacy.yaml"), legacy_config).expect("legacy config");
        let baseline = init_git_repo(repository.path());
        assert!(git_show(repository.path(), &baseline, Path::new("legacy.yaml")).is_ok());
        assert!(git_show(repository.path(), &baseline, Path::new("mitase.yaml")).is_err());
        let files = vec![ChangedFile {
            status: ChangeStatus::Renamed,
            old_path: Some(RepoPath::new("legacy.yaml").unwrap()),
            new_path: Some(RepoPath::new("mitase.yaml").unwrap()),
            hunks: vec![],
        }];

        assert!(is_pre_v1_identity_cutover(
            repository.path(),
            Some(&baseline),
            false,
            mitase_project_model::CONFIG_SCHEMA,
            &files,
        ));
        assert!(!is_pre_v1_identity_cutover(
            repository.path(),
            Some("missing-baseline"),
            false,
            mitase_project_model::CONFIG_SCHEMA,
            &files,
        ));
        assert!(!is_pre_v1_identity_cutover(
            repository.path(),
            Some(&baseline),
            true,
            mitase_project_model::CONFIG_SCHEMA,
            &files,
        ));

        let malformed = tempdir().expect("malformed baseline repository");
        fs::write(
            malformed.path().join("legacy.yaml"),
            "schema: legacy/config/v1\n",
        )
        .expect("malformed legacy config");
        let malformed_baseline = init_git_repo(malformed.path());
        assert!(!is_pre_v1_identity_cutover(
            malformed.path(),
            Some(&malformed_baseline),
            false,
            mitase_project_model::CONFIG_SCHEMA,
            &files,
        ));

        fs::write(
            repository.path().join("mitase.yaml"),
            "schema: mitase/config/v1\n",
        )
        .expect("canonical config");
        git_commit(repository.path(), "add canonical config");
        let current = git_revision(repository.path());
        assert!(!is_pre_v1_identity_cutover(
            repository.path(),
            Some(&current),
            false,
            mitase_project_model::CONFIG_SCHEMA,
            &files,
        ));
    }

    fn fixture_execution_plan(root: &Path, revision: &str) -> WorkPlan {
        let workspace = SpecWorkspace::load(root).unwrap();
        let index = workspace.index().unwrap();
        let request = mitase_work_model::WorkRequest {
            schema: mitase_work_model::WORK_REQUEST_SCHEMA.into(),
            id: "WORK-VALIDATION-BASIS".into(),
            title: "modify the fixture behavior".into(),
            operation: mitase_work_model::WorkOperation::Modify,
            origin: mitase_work_model::WorkOrigin::RequirementCriterion {
                criterion: "REQ-FIXTURE-001#criterion.behavior".parse().unwrap(),
            },
            constraints: Default::default(),
            requested_targets: vec![],
        };
        let plan = mitase_planner::plan(&request, &workspace, &index, revision).unwrap();
        assert_eq!(
            plan.status,
            mitase_work_model::PlanStatus::Ready,
            "{plan:?}"
        );
        plan
    }

    fn sample_target(
        path: &str,
        description: &str,
        lines: (usize, usize),
    ) -> mitase_work_model::PlannedTarget {
        mitase_work_model::PlannedTarget {
            reference: "FEAT-AUTH-001#binding.ui/target.requested".parse().unwrap(),
            verification_claim: None,
            artifact_identity: None,
            transition: TargetTransition::Add,
            lifecycle: TargetLifecycle::EnsurePresent,
            access: mitase_work_model::TargetAccessMode::Editable,
            resolved_path: path.to_string(),
            resolved_selector: mitase_work_model::ResolvedSelector {
                description: description.to_string(),
                symbols: if description == "file" {
                    Vec::new()
                } else {
                    vec!["requested_function".to_string()]
                },
            },
            content_hash: "sha256:0".to_string(),
            excerpt_hash: "sha256:0".to_string(),
            container_content_hash: None,
            adapter: "rust".to_string(),
            facet: "ui".to_string(),
            role: BindingRole::Implementation,
            byte_start: 0,
            byte_end: 32,
            line_start: lines.0,
            line_end: lines.1,
            budget_bytes: 32,
            budget_lines: None,
            reason: "test".to_string(),
        }
    }

    #[test]
    fn changed_anchors_include_deleted_baseline_anchor_by_repo_path() {
        let baseline_dir = tempdir().expect("baseline dir");
        copy_dir(&fixture_root(), baseline_dir.path());
        let current_dir = tempdir().expect("current dir");
        copy_dir(&fixture_root(), current_dir.path());
        fs::write(
            current_dir.path().join("spec/requirement.yaml"),
            r#"schema: mitase/spec/v1
kind: requirements
namespace: auth
category: Authentication
requirements:
  - id: REQ-AUTH-001
    title: Reject invalid credentials
    description: Invalid credentials do not create a session.
    priority: high
    status: implemented
    criteria: []
"#,
        )
        .unwrap();
        let current_workspace = SpecWorkspace::load(current_dir.path()).unwrap();
        let current_index = current_workspace.index().unwrap();
        let baseline_workspace = SpecWorkspace::load(baseline_dir.path()).unwrap();
        let baseline_index = baseline_workspace.index().unwrap();
        let changed = BTreeSet::from([RepoPath::new("spec/requirement.yaml").unwrap()]);
        let anchors = changed_anchors_for_documents(
            &changed,
            &current_workspace,
            &current_index,
            Some(&baseline_workspace),
            Some(&baseline_index),
            LocalAnchorKind::Criterion,
        );
        assert!(
            anchors.contains(
                &"REQ-AUTH-001#criterion.invalid-credentials"
                    .parse()
                    .unwrap()
            )
        );
    }

    #[test]
    fn planned_feature_ownership_is_rejected() {
        let (tempdir, _, _) = load_fixture_workspace();
        let path = tempdir.path().join("spec/feature.yaml");
        let source = fs::read_to_string(&path).unwrap();
        let source = source
            .replacen("status: implemented", "status: planned", 1)
            .replacen(
                "        responsibility: Submit login and show generic failure.\n",
                concat!(
                    "        responsibility: Submit login and show generic failure.\n",
                    "        owns:\n",
                    "          - { id: login-file, adapter: typescript, path: web/login.ts, selector: { kind: file } }\n",
                ),
                1,
            );
        fs::write(path, source).unwrap();
        let workspace = SpecWorkspace::load(tempdir.path()).unwrap();
        let index = workspace.index().unwrap();
        let result = validate_loaded_workspace(&workspace, &index);
        assert!(result.diagnostics.iter().any(|diagnostic| {
            diagnostic.rule_id == "MITASE-FEATURE-003"
                && diagnostic.message.contains("planned feature")
        }));
    }

    #[test]
    fn implemented_feature_target_requires_acceptance() {
        let (tempdir, _, _) = load_fixture_workspace();
        let path = tempdir.path().join("spec/feature.yaml");
        let source = fs::read_to_string(&path).unwrap().replacen(
            "            claims: [{ kind: satisfies, criterion: REQ-AUTH-001#criterion.invalid-credentials }]",
            "            claims: []",
            1,
        );
        fs::write(path, source).unwrap();
        let workspace = SpecWorkspace::load(tempdir.path()).unwrap();
        let index = workspace.index().unwrap();
        let result = validate_loaded_workspace(&workspace, &index);
        assert!(result.diagnostics.iter().any(|diagnostic| {
            diagnostic.rule_id == "MITASE-FEATURE-002"
                && diagnostic
                    .message
                    .contains("has no direct or exposed acceptance criterion")
        }));
    }

    #[test]
    fn implemented_feature_target_requires_exact_verification() {
        let (tempdir, _, _) = load_fixture_workspace();
        let path = tempdir.path().join("spec/requirement.yaml");
        let source = fs::read_to_string(&path).unwrap().replace(
            "                  - FEAT-AUTH-001#binding.ui/target.submit\n",
            "",
        );
        fs::write(path, source).unwrap();
        let workspace = SpecWorkspace::load(tempdir.path()).unwrap();
        let index = workspace.index().unwrap();
        let result = validate_loaded_workspace(&workspace, &index);
        assert!(result.diagnostics.iter().any(|diagnostic| {
            diagnostic.rule_id == "MITASE-FEATURE-002"
                && diagnostic.message.contains("target.submit")
                && diagnostic.message.contains("has no exact verification")
        }));
    }

    #[test]
    fn implemented_feature_target_cannot_self_verify() {
        let (tempdir, _, _) = load_fixture_workspace();
        let feature_path = tempdir.path().join("spec/feature.yaml");
        let feature = fs::read_to_string(&feature_path)
            .unwrap()
            .replacen(
                "            claims: [{ kind: satisfies, criterion: REQ-AUTH-001#criterion.invalid-credentials }]",
                "            claims: [{ kind: satisfies, criterion: REQ-AUTH-001#criterion.invalid-credentials }, { kind: verifies, criterion: REQ-AUTH-001#criterion.invalid-credentials, covers: [FEAT-AUTH-001#binding.ui/target.submit], runner: { runner: cargo-test, arguments: { package: app, test: invalid_credentials } } }]",
                1,
            );
        fs::write(feature_path, feature).unwrap();
        let requirement_path = tempdir.path().join("spec/requirement.yaml");
        let requirement = fs::read_to_string(&requirement_path).unwrap().replace(
            "                  - FEAT-AUTH-001#binding.ui/target.submit\n",
            "",
        );
        fs::write(requirement_path, requirement).unwrap();
        let workspace = SpecWorkspace::load(tempdir.path()).unwrap();
        let index = workspace.index().unwrap();
        let result = validate_loaded_workspace(&workspace, &index);
        assert!(result.diagnostics.iter().any(|diagnostic| {
            diagnostic.rule_id == "MITASE-FEATURE-002"
                && diagnostic.message.contains("target.submit")
                && diagnostic.message.contains("has no exact verification")
        }));
    }

    #[test]
    fn duplicate_exact_owner_is_a_repository_wide_readiness_blocker() {
        let (tempdir, _, _) = load_fixture_workspace();
        let path = tempdir.path().join("spec/feature.yaml");
        let source = fs::read_to_string(&path).unwrap().replacen(
            "            claims: [{ kind: satisfies, criterion: REQ-AUTH-001#criterion.invalid-credentials }]",
            concat!(
                "            claims: [{ kind: satisfies, criterion: REQ-AUTH-001#criterion.invalid-credentials }]\n",
                "          - id: submit-alias\n",
                "            adapter: typescript\n",
                "            path: web/login.ts\n",
                "            selector: { kind: symbol, name: submitLogin }\n",
                "            claims: [{ kind: satisfies, criterion: REQ-AUTH-001#criterion.invalid-credentials }]",
            ),
            1,
        );
        fs::write(path, source).unwrap();
        let workspace = SpecWorkspace::load(tempdir.path()).unwrap();
        let index = workspace.index().unwrap();
        let report =
            evaluate_readiness(&workspace, &index, "readiness-test", false).expect("readiness");
        assert!(report.ownership.blockers.iter().any(|blocker| {
            blocker.contains("typescript:web/login.ts") && blocker.contains("has 2 owners")
        }));
    }

    #[test]
    fn ownership_scope_limit_is_not_reduced_by_a_bounded_criterion_probe() {
        let (tempdir, _, _) = load_fixture_workspace();
        let path = tempdir.path().join("spec/feature.yaml");
        let source = fs::read_to_string(&path).unwrap().replacen(
            "        responsibility: Submit login and show generic failure.\n",
            concat!(
                "        responsibility: Submit login and show generic failure.\n",
                "        owns:\n",
                "          - { id: repository-wide, adapter: openapi, path: openapi.yaml, selector: { kind: path-prefix, value: openapi.yaml } }\n",
            ),
            1,
        );
        fs::write(path, source).unwrap();
        let mut workspace = SpecWorkspace::load(tempdir.path()).unwrap();
        workspace
            .config
            .validation
            .readiness
            .limits
            .max_ownership_scope_units = 0;
        workspace
            .config
            .validation
            .readiness
            .probes
            .implemented_criteria = vec![mitase_project_model::ReadinessCriterionProbe {
            criterion: "REQ-AUTH-001#criterion.invalid-credentials"
                .parse()
                .unwrap(),
            level: mitase_project_model::ReadinessLevel::WorkReady,
        }];
        let index = workspace.index().unwrap();
        let report =
            evaluate_readiness(&workspace, &index, "readiness-test", false).expect("readiness");
        assert!(
            report.ownership.blockers.iter().any(|blocker| {
                blocker.contains("ownership-scope:FEAT-AUTH-001#binding.ui/repository-wide")
                    && blocker.contains("max_ownership_scope_units")
            }),
            "{:?}",
            report.ownership.blockers
        );
    }

    #[test]
    fn language_public_symbol_without_an_exposes_claim_is_not_a_public_contract_subject() {
        let tempdir = tempdir().unwrap();
        copy_dir(&workbench_fixture_root(), tempdir.path());
        fs::write(
            tempdir.path().join("src/lib.rs"),
            "mod removable;\n\npub fn behavior() -> bool {\n    true\n}\n\npub fn ungoverned() {}\n",
        )
        .unwrap();
        let mut workspace = SpecWorkspace::load(tempdir.path()).unwrap();
        workspace.config.validation.readiness.target =
            mitase_project_model::ReadinessLevel::Traceable;
        workspace
            .config
            .validation
            .readiness
            .probes
            .public_entrypoints = Some(mitase_project_model::ReadinessSelectionProbe {
            selection: mitase_project_model::ReadinessSelection::All,
            level: mitase_project_model::ReadinessLevel::Seedable,
        });
        let index = workspace.index().unwrap();
        let report =
            evaluate_readiness(&workspace, &index, "readiness-test", false).expect("readiness");
        assert!(
            !report
                .seedability
                .blockers
                .iter()
                .any(|blocker| blocker.contains("ungoverned")),
            "language visibility alone must not invent a public contract: {:?}",
            report.seedability.blockers
        );
    }

    #[test]
    fn canonical_execution_reconstructs_basis_after_editable_change() {
        let tempdir = tempdir().unwrap();
        copy_dir(&workbench_fixture_root(), tempdir.path());
        let revision = init_git_repo(tempdir.path());
        let plan = fixture_execution_plan(tempdir.path(), &revision);
        fs::write(
            tempdir.path().join("src/lib.rs"),
            "mod removable;\n\npub fn behavior() -> bool {\n    1 == 1\n}\n",
        )
        .unwrap();

        let workspace = SpecWorkspace::load(tempdir.path()).unwrap();
        let index = workspace.index().unwrap();
        let canonical = canonical_plan_for_execution(&workspace, &index, &plan, &revision)
            .expect("a valid basis must be reconstructed after an editable change");
        assert_eq!(canonical, plan);
    }

    #[test]
    fn canonical_execution_rejects_missing_basis_revision() {
        let tempdir = tempdir().unwrap();
        copy_dir(&workbench_fixture_root(), tempdir.path());
        let revision = init_git_repo(tempdir.path());
        let mut plan = fixture_execution_plan(tempdir.path(), &revision);
        fs::write(
            tempdir.path().join("src/lib.rs"),
            "mod removable;\n\npub fn behavior() -> bool {\n    1 == 1\n}\n",
        )
        .unwrap();
        plan.basis.revision = "missing-basis-revision".into();
        plan.canonical_digest = work_plan_digest(&plan);

        let workspace = SpecWorkspace::load(tempdir.path()).unwrap();
        let index = workspace.index().unwrap();
        let error =
            canonical_plan_for_execution(&workspace, &index, &plan, "missing-basis-revision")
                .unwrap_err()
                .to_string();
        assert!(error.contains("cannot reconstruct the work-plan basis workspace"));
    }

    #[test]
    fn canonical_execution_rejects_basis_with_unrestorable_config() {
        let tempdir = tempdir().unwrap();
        copy_dir(&workbench_fixture_root(), tempdir.path());
        init_git_repo(tempdir.path());
        let valid_config = fs::read_to_string(tempdir.path().join("mitase.yaml")).unwrap();
        fs::write(
            tempdir.path().join("mitase.yaml"),
            "schema: mitase/config/v1\n",
        )
        .unwrap();
        let revision = git_commit(tempdir.path(), "invalid basis config");
        fs::write(tempdir.path().join("mitase.yaml"), valid_config).unwrap();
        let plan = fixture_execution_plan(tempdir.path(), &revision);
        fs::write(
            tempdir.path().join("src/lib.rs"),
            "mod removable;\n\npub fn behavior() -> bool {\n    1 == 1\n}\n",
        )
        .unwrap();

        let workspace = SpecWorkspace::load(tempdir.path()).unwrap();
        let index = workspace.index().unwrap();
        let error = canonical_plan_for_execution(&workspace, &index, &plan, &revision)
            .unwrap_err()
            .to_string();
        assert!(error.contains("cannot reconstruct the work-plan basis workspace"));
    }

    #[test]
    fn canonical_execution_rejects_basis_with_unbuildable_inventory() {
        let tempdir = tempdir().unwrap();
        copy_dir(&workbench_fixture_root(), tempdir.path());
        init_git_repo(tempdir.path());
        let valid_config = fs::read_to_string(tempdir.path().join("mitase.yaml")).unwrap();
        let broken_config = valid_config.replace(
            "rust: { mode: test, include_tests: true }",
            "unsupported: {}",
        );
        assert_ne!(broken_config, valid_config);
        fs::write(tempdir.path().join("mitase.yaml"), broken_config).unwrap();
        let revision = git_commit(tempdir.path(), "unbuildable basis inventory");
        fs::write(tempdir.path().join("mitase.yaml"), valid_config).unwrap();
        let plan = fixture_execution_plan(tempdir.path(), &revision);
        fs::write(
            tempdir.path().join("src/lib.rs"),
            "mod removable;\n\npub fn behavior() -> bool {\n    1 == 1\n}\n",
        )
        .unwrap();

        let workspace = SpecWorkspace::load(tempdir.path()).unwrap();
        let index = workspace.index().unwrap();
        let error = canonical_plan_for_execution(&workspace, &index, &plan, &revision)
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("verification is blocked because inventory failed")
                || error.contains("inventory failed; plan generation is refused"),
            "unexpected inventory reconstruction error: {error}"
        );
    }

    #[test]
    fn validate_reports_deleted_criterion_without_artifact_update() {
        let (tempdir, _, _) = load_fixture_workspace();
        let baseline = init_git_repo(tempdir.path());
        fs::write(
            tempdir.path().join("spec/requirement.yaml"),
            r#"schema: mitase/spec/v1
kind: requirements
namespace: auth
category: Authentication
requirements:
  - id: REQ-AUTH-001
    title: Reject invalid credentials
    description: Invalid credentials do not create a session.
    priority: high
    status: implemented
    criteria: []
"#,
        )
        .unwrap();
        let workspace = SpecWorkspace::load(tempdir.path()).unwrap();
        let index = workspace.index().unwrap();
        let changed_files = vec![ChangedFile {
            status: ChangeStatus::Modified,
            old_path: Some(RepoPath::new("spec/requirement.yaml").unwrap()),
            new_path: Some(RepoPath::new("spec/requirement.yaml").unwrap()),
            hunks: vec![],
        }];
        let result = validate(&ValidationContext {
            config: &workspace.config,
            workspace: &workspace,
            index: &index,
            changed_files: Some(&changed_files),
            reported_changed_files: None,
            work_plan: None,
            selected_slice: None,
            plan_mode: PlanValidationMode::PreState,
            preset: workspace.config.validation.preset,
            revision: None,
            change_base_revision: Some(&baseline),
        });
        assert!(
            result
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.rule_id == "MITASE-CHANGE-003")
        );
    }

    #[test]
    fn hunkless_changes_require_file_scope_for_editable_targets() {
        let (_tempdir, workspace, index) = load_fixture_workspace();
        let changed = ChangedFile {
            status: ChangeStatus::Untracked,
            old_path: None,
            new_path: Some(RepoPath::new("web/new.ts").unwrap()),
            hunks: vec![],
        };
        let ctx = ValidationContext {
            config: &workspace.config,
            workspace: &workspace,
            index: &index,
            changed_files: None,
            reported_changed_files: None,
            work_plan: None,
            selected_slice: None,
            plan_mode: PlanValidationMode::PreState,
            preset: workspace.config.validation.preset,
            revision: None,
            change_base_revision: None,
        };
        assert!(!editable_target_matches_hunkless_change(
            &ctx,
            &sample_target("web/new.ts", "symbols requested_function", (1, 1)),
            &changed,
        ));
        assert!(editable_target_matches_hunkless_change(
            &ctx,
            &sample_target("web/new.ts", "file", (1, 1)),
            &changed,
        ));
    }

    #[test]
    fn json_openapi_operation_scope_rejects_a_sibling_operation_change() {
        let tempdir = tempdir().expect("tempdir");
        fs::write(
            tempdir.path().join("openapi.json"),
            concat!(
                "{\n",
                "  \"paths\": {\n",
                "    \"/users\": {\n",
                "      \"get\": { \"summary\": \"read\" },\n",
                "      \"post\": { \"summary\": \"create\" }\n",
                "    }\n",
                "  }\n",
                "}\n",
            ),
        )
        .expect("openapi");
        let declared = mitase_spec_model::ArtifactTarget {
            id: "get-users".into(),
            adapter: "openapi".into(),
            path: RepoPath::new("openapi.json").unwrap(),
            selector: Selector::Operation {
                method: "get".into(),
                path: "/users".into(),
            },
            lifecycle: mitase_spec_model::ArtifactTargetLifecycle::Present,
            claims: vec![],
        };
        let resolved = mitase_workspace::resolve_target(tempdir.path(), &declared).unwrap();
        let (_fixture, workspace, index) = load_fixture_workspace();
        let ctx = ValidationContext {
            config: &workspace.config,
            workspace: &workspace,
            index: &index,
            changed_files: None,
            reported_changed_files: None,
            work_plan: None,
            selected_slice: None,
            plan_mode: PlanValidationMode::PostState,
            preset: workspace.config.validation.preset,
            revision: None,
            change_base_revision: None,
        };
        let mut target = sample_target(
            "openapi.json",
            "operation GET /users",
            (resolved.line_start, resolved.line_end),
        );
        target.adapter = "openapi".into();
        target.lifecycle = TargetLifecycle::Stable;
        let sibling_change = ChangedFile {
            status: ChangeStatus::Modified,
            old_path: Some(RepoPath::new("openapi.json").unwrap()),
            new_path: Some(RepoPath::new("openapi.json").unwrap()),
            hunks: vec![ChangedRange {
                old_start: 5,
                old_end: 5,
                new_start: 5,
                new_end: 5,
            }],
        };
        assert!(!change_is_within_editable_scope(
            &ctx,
            &[&target],
            &sibling_change,
            &sibling_change.hunks[0],
        ));
        let slice = ExecutionSlice {
            id: "json-openapi-scope".into(),
            goal: "Change only GET /users".into(),
            anchors: vec![],
            editable_targets: vec![target],
            verification_targets: vec![],
            readonly_context: vec![],
            acceptance: vec![],
            contracts: vec![],
            non_goals: vec![],
            completion: vec![CompletionCheck::DiffWithinScope],
            budget: Default::default(),
            confidence: PlanConfidence::Exact,
            blockers: vec![],
        };
        let mut diagnostics = Vec::new();
        validate_slice_scope(&ctx, &[sibling_change], &slice, &mut diagnostics);
        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.rule_id == "MITASE-WORK-006")
        );
    }

    #[test]
    fn add_to_existing_file_scope_rejects_a_sibling_change() {
        let (tempdir, _, _) = load_fixture_workspace();
        fs::write(
            tempdir.path().join("web/login.ts"),
            "export function submitLogin() { return fetch('/sessions', { method: 'POST' }); }\nexport function sibling() {}\n",
        )
        .expect("post-state source");
        let workspace = SpecWorkspace::load(tempdir.path()).expect("workspace");
        let index = workspace.index().expect("workspace index");
        let reference: BoundTargetRef = "FEAT-AUTH-001#binding.ui/target.submit".parse().unwrap();
        let declared = index.target(&reference).expect("declared target");
        let resolved = resolve_target_in_workspace(&workspace, declared).expect("target");
        let mut target = sample_target(
            "web/login.ts",
            &resolved.description,
            (resolved.line_start, resolved.line_end),
        );
        target.reference = reference;
        target.resolved_selector.symbols = resolved.symbols.clone();
        target.content_hash.clear();
        target.excerpt_hash.clear();
        target.byte_start = resolved.byte_start;
        target.byte_end = resolved.byte_end;

        let ctx = ValidationContext {
            config: &workspace.config,
            workspace: &workspace,
            index: &index,
            changed_files: None,
            reported_changed_files: None,
            work_plan: None,
            selected_slice: None,
            plan_mode: PlanValidationMode::PostState,
            preset: workspace.config.validation.preset,
            revision: None,
            change_base_revision: None,
        };
        let sibling_change = ChangedFile {
            status: ChangeStatus::Modified,
            old_path: Some(RepoPath::new("web/login.ts").unwrap()),
            new_path: Some(RepoPath::new("web/login.ts").unwrap()),
            hunks: vec![ChangedRange {
                old_start: 2,
                old_end: 2,
                new_start: 2,
                new_end: 2,
            }],
        };
        assert!(!editable_add_target_matches_file(
            &ctx,
            &target,
            &sibling_change
        ));
        assert!(!change_is_within_editable_scope(
            &ctx,
            &[&target],
            &sibling_change,
            &sibling_change.hunks[0],
        ));
        let target_and_sibling_change = ChangedFile {
            hunks: vec![ChangedRange {
                old_start: 1,
                old_end: 1,
                new_start: 1,
                new_end: 2,
            }],
            ..sibling_change.clone()
        };
        assert!(!change_is_within_editable_scope(
            &ctx,
            &[&target],
            &target_and_sibling_change,
            &target_and_sibling_change.hunks[0],
        ));

        let slice = ExecutionSlice {
            id: "add-existing-file-scope".into(),
            goal: "Add only the approved target".into(),
            anchors: vec![],
            editable_targets: vec![target],
            verification_targets: vec![],
            readonly_context: vec![],
            acceptance: vec![],
            contracts: vec![],
            non_goals: vec![],
            completion: vec![CompletionCheck::DiffWithinScope],
            budget: Default::default(),
            confidence: PlanConfidence::Exact,
            blockers: vec![],
        };
        let mut diagnostics = Vec::new();
        validate_slice_scope(&ctx, &[sibling_change], &slice, &mut diagnostics);
        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.rule_id == "MITASE-WORK-006")
        );
    }

    #[test]
    fn generated_binding_without_source_is_rejected() {
        let tempdir = tempdir().expect("tempdir");
        write_generated_binding_workspace(tempdir.path());
        let workspace = SpecWorkspace::load(tempdir.path()).expect("workspace");
        let index = workspace.index().expect("index");
        let result = validate(&ValidationContext {
            config: &workspace.config,
            workspace: &workspace,
            index: &index,
            changed_files: None,
            reported_changed_files: None,
            work_plan: None,
            selected_slice: None,
            plan_mode: PlanValidationMode::PreState,
            preset: workspace.config.validation.preset,
            revision: None,
            change_base_revision: None,
        });
        assert!(
            result
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.rule_id == "MITASE-GENERATED-001")
        );
    }

    #[test]
    fn every_generated_target_requires_its_own_exact_source_relation() {
        let tempdir = tempdir().expect("tempdir");
        write_generated_binding_workspace(tempdir.path());
        fs::write(
            tempdir.path().join("src/generated.rs"),
            "pub fn source() {}\npub fn generated_a() {}\npub fn generated_b() {}\n",
        )
        .expect("artifacts");
        fs::write(
            tempdir.path().join("spec/feature.yaml"),
            concat!(
                "schema: mitase/spec/v1\n",
                "kind: features\n",
                "namespace: sample\n",
                "category: Sample\n",
                "features:\n",
                "  - id: FEAT-TEST-001\n",
                "    title: Test\n",
                "    summary: Test feature.\n",
                "    status: implemented\n",
                "    bindings:\n",
                "      - id: source\n",
                "        role: implementation\n",
                "        facet: backend\n",
                "        responsibility: Generate artifacts.\n",
                "        targets:\n",
                "          - { id: source, adapter: rust, path: src/generated.rs, selector: { kind: symbol, name: source }, claims: [] }\n",
                "      - id: generated\n",
                "        role: generated\n",
                "        facet: backend\n",
                "        responsibility: Generated artifacts.\n",
                "        targets:\n",
                "          - id: generated-a\n",
                "            adapter: rust\n",
                "            path: src/generated.rs\n",
                "            selector: { kind: symbol, name: generated_a }\n",
                "            claims:\n",
                "              - kind: generated-from\n",
                "                targets: [FEAT-TEST-001#binding.source/target.source]\n",
                "          - { id: generated-b, adapter: rust, path: src/generated.rs, selector: { kind: symbol, name: generated_b }, claims: [] }\n",
            ),
        )
        .expect("feature spec");
        let workspace = SpecWorkspace::load(tempdir.path()).expect("workspace");
        let index = workspace.index().expect("index");
        let result = validate_loaded_workspace(&workspace, &index);
        assert!(result.diagnostics.iter().any(|diagnostic| {
            diagnostic.rule_id == "MITASE-GENERATED-001"
                && diagnostic.message.contains("generated-b")
        }));
        assert!(!result.diagnostics.iter().any(|diagnostic| {
            diagnostic.rule_id == "MITASE-GENERATED-001"
                && diagnostic.message.contains("generated-a")
        }));
    }

    #[test]
    fn generated_scope_requires_a_changed_exact_source() {
        let (_tempdir, workspace, mut index) = load_fixture_workspace();
        let source_ref: BoundTargetRef =
            "FEAT-AUTH-001#binding.ui/target.requested".parse().unwrap();
        let generated_ref: BoundTargetRef =
            "FEAT-AUTH-001#binding.ui/target.generated".parse().unwrap();
        index
            .generated_from
            .insert(generated_ref.clone(), vec![source_ref.clone()]);

        let mut source = sample_target("web/login.ts", "symbol requested_function", (1, 1));
        source.reference = source_ref;
        source.transition = TargetTransition::Modify;
        source.lifecycle = TargetLifecycle::Stable;
        let mut generated = sample_target("web/generated.ts", "symbol requested_function", (1, 1));
        generated.reference = generated_ref;
        generated.transition = TargetTransition::Readonly;
        generated.lifecycle = TargetLifecycle::Stable;
        generated.access = TargetAccessMode::Generated;
        generated.role = BindingRole::Generated;

        let slice = ExecutionSlice {
            id: "generated-scope".into(),
            goal: "Generate output from source".into(),
            anchors: vec![],
            editable_targets: vec![source],
            verification_targets: vec![],
            readonly_context: vec![generated],
            acceptance: vec![],
            contracts: vec![],
            non_goals: vec![],
            completion: vec![CompletionCheck::DiffWithinScope],
            budget: Default::default(),
            confidence: PlanConfidence::Exact,
            blockers: vec![],
        };
        let source_change = ChangedFile {
            status: ChangeStatus::Modified,
            old_path: Some(RepoPath::new("web/login.ts").unwrap()),
            new_path: Some(RepoPath::new("web/login.ts").unwrap()),
            hunks: vec![ChangedRange {
                old_start: 1,
                old_end: 1,
                new_start: 1,
                new_end: 1,
            }],
        };
        let generated_change = ChangedFile {
            status: ChangeStatus::Modified,
            old_path: Some(RepoPath::new("web/generated.ts").unwrap()),
            new_path: Some(RepoPath::new("web/generated.ts").unwrap()),
            hunks: vec![ChangedRange {
                old_start: 1,
                old_end: 1,
                new_start: 1,
                new_end: 1,
            }],
        };
        let ctx = ValidationContext {
            config: &workspace.config,
            workspace: &workspace,
            index: &index,
            changed_files: None,
            reported_changed_files: None,
            work_plan: None,
            selected_slice: None,
            plan_mode: PlanValidationMode::PostState,
            preset: workspace.config.validation.preset,
            revision: None,
            change_base_revision: None,
        };

        let mut diagnostics = Vec::new();
        validate_slice_scope(
            &ctx,
            &[source_change.clone(), generated_change.clone()],
            &slice,
            &mut diagnostics,
        );
        assert!(diagnostics.is_empty(), "{diagnostics:?}");

        validate_slice_scope(&ctx, &[generated_change], &slice, &mut diagnostics);
        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.rule_id == "MITASE-WORK-005")
        );
    }

    #[test]
    fn ensure_present_targets_use_actual_post_state_budget() {
        let target = sample_target("web/new.ts", "symbols requested_function", (1, 1));
        let resolved = ResolvedTarget {
            path: PathBuf::from("web/new.ts"),
            description: "symbols requested_function".to_string(),
            symbols: vec!["requested_function".to_string()],
            content_hash: "sha256:1".to_string(),
            bytes: 80,
            byte_start: 0,
            byte_end: 80,
            line_start: 1,
            line_end: 4,
            excerpt: "fn requested_function() {}\nfn extra() {}\n".to_string(),
            excerpt_hash: "sha256:1".to_string(),
        };
        assert!(ensure_present_target_exceeds_budget(&target, &resolved));
    }

    #[test]
    fn non_file_targets_ignore_same_file_sibling_changes() {
        let resolved = ResolvedTarget {
            path: PathBuf::from("web/login.ts"),
            description: "symbols submitLogin".to_string(),
            symbols: vec!["submitLogin".to_string()],
            content_hash: "sha256:1".to_string(),
            bytes: 64,
            byte_start: 0,
            byte_end: 64,
            line_start: 1,
            line_end: 1,
            excerpt: "export function submitLogin() {}\n".to_string(),
            excerpt_hash: "sha256:1".to_string(),
        };
        let unrelated_change = ChangedFile {
            status: ChangeStatus::Modified,
            old_path: Some(RepoPath::new("web/login.ts").unwrap()),
            new_path: Some(RepoPath::new("web/login.ts").unwrap()),
            hunks: vec![ChangedRange {
                old_start: 4,
                old_end: 4,
                new_start: 4,
                new_end: 4,
            }],
        };
        assert!(!changed_file_impacts_target(
            TargetRangeSide::Old,
            "web/login.ts",
            Some(&resolved),
            &unrelated_change,
        ));
    }

    #[test]
    fn file_targets_remain_path_based() {
        let changed = ChangedFile {
            status: ChangeStatus::Modified,
            old_path: Some(RepoPath::new("web/login.ts").unwrap()),
            new_path: Some(RepoPath::new("web/login.ts").unwrap()),
            hunks: vec![],
        };
        assert!(changed_file_impacts_target(
            TargetRangeSide::Old,
            "web/login.ts",
            None,
            &changed,
        ));
    }

    #[test]
    fn exact_test_execution_requires_match() {
        let target = mitase_spec_model::ArtifactTarget {
            id: "exact-test".into(),
            adapter: "rust".into(),
            path: RepoPath::new("src/lib.rs").unwrap(),
            selector: mitase_spec_model::ExactSelector::Symbol {
                name: "exact_test_execution_requires_match".into(),
            },
            lifecycle: mitase_spec_model::ArtifactTargetLifecycle::Present,
            claims: vec![],
        };
        let arguments = BTreeMap::from([(
            "test".to_string(),
            "tests::exact_test_execution_requires_match".to_string(),
        )]);
        let proof = ensure_exact_test_executed(
            VerificationRunnerAdapter::CargoLibtest,
            &target,
            &arguments,
            None,
            &[],
            br#"{"type":"test","name":"tests::exact_test_execution_requires_match","event":"ok"}
{"type":"suite","event":"ok"}
"#,
        )
        .expect("exact test marker");
        assert_eq!(proof.identity, "tests::exact_test_execution_requires_match");
        assert_eq!(proof.matched_count, 1);
        assert_eq!(proof.schema, VERIFICATION_PROOF_SCHEMA);
        assert_eq!(proof.status, VerificationProofStatus::Passed);
        assert!(
            ensure_exact_test_executed(
                VerificationRunnerAdapter::CargoLibtest,
                &target,
                &arguments,
                None,
                &[],
                br#"{"type":"test","name":"tests::other","event":"ok"}
"#,
            )
            .is_err()
        );
        assert!(ensure_exact_test_executed(
            VerificationRunnerAdapter::CargoLibtest,
            &target,
            &arguments,
            None,
            &[],
            br#"{"type":"test","name":"other::tests::exact_test_execution_requires_match","event":"ok"}
"#,
        )
        .is_err());
        assert!(ensure_exact_test_executed(
            VerificationRunnerAdapter::CargoLibtest,
            &target,
            &arguments,
            None,
            &[],
            br#"{"type":"test","name":"tests::exact_test_execution_requires_match","event":"ignored"}
"#,
        )
        .is_err());
        assert!(ensure_exact_test_executed(
            VerificationRunnerAdapter::CargoLibtest,
            &target,
            &arguments,
            None,
            &[],
            br#"{"type":"test","name":"tests::exact_test_execution_requires_match","event":"ok"}
{"type":"test","name":"tests::exact_test_execution_requires_match","event":"ok"}
"#,
        )
        .is_err());
        assert!(
            ensure_exact_test_executed(
                VerificationRunnerAdapter::Pytest,
                &target,
                &arguments,
                None,
                &[],
                b"ok"
            )
            .is_err()
        );

        let pytest_target = mitase_spec_model::ArtifactTarget {
            id: "pytest-exact-test".into(),
            adapter: "python".into(),
            path: RepoPath::new("tests/exact_test.py").unwrap(),
            selector: mitase_spec_model::ExactSelector::File,
            lifecycle: mitase_spec_model::ArtifactTargetLifecycle::Present,
            claims: vec![],
        };
        let pytest_arguments = BTreeMap::from([(
            "test".to_string(),
            "tests/exact_test.py::exact_test_execution_requires_match".to_string(),
        )]);
        let pytest_proof = ensure_exact_test_executed(
            VerificationRunnerAdapter::Pytest,
            &pytest_target,
            &pytest_arguments,
            None,
            &[
                "tests/exact_test.py::exact_test_execution_requires_match".into(),
                "-v".into(),
            ],
            b"tests/exact_test.py::exact_test_execution_requires_match PASSED [100%]\n",
        )
        .expect("pytest proof");
        assert_eq!(pytest_proof.status, VerificationProofStatus::Passed);
        let canonical_pytest_arguments = canonical_runner_arguments_for_adapter(
            VerificationRunnerAdapter::Pytest,
            vec![
                "tests/exact_test.py::exact_test_execution_requires_match".into(),
                "-v".into(),
            ],
        );
        assert_eq!(
            canonical_pytest_arguments,
            vec![
                "tests/exact_test.py::exact_test_execution_requires_match",
                "-v",
                "-o",
                "addopts=",
            ]
        );
        assert!(
            require_exact_runner_filter(
                VerificationRunnerAdapter::Pytest,
                &canonical_pytest_arguments,
                &pytest_arguments,
            )
            .is_ok()
        );
        assert!(
            ensure_exact_test_executed(
                VerificationRunnerAdapter::Pytest,
                &pytest_target,
                &pytest_arguments,
                None,
                &[
                    "tests/exact_test.py::exact_test_execution_requires_match".into(),
                    "-v".into(),
                    "-s".into(),
                ],
                b"tests/exact_test.py::exact_test_execution_requires_match PASSED\n",
            )
            .is_err()
        );

        let pytest_parameterized_arguments = BTreeMap::from([(
            "test".to_string(),
            "tests/exact_test.py::exact_test_execution_requires_match[param value]".to_string(),
        )]);
        assert!(
            ensure_exact_test_executed(
                VerificationRunnerAdapter::Pytest,
                &pytest_target,
                &pytest_parameterized_arguments,
                None,
                &["tests/exact_test.py::exact_test_execution_requires_match[param value]".into(), "-v".into()],
                b"tests/exact_test.py::exact_test_execution_requires_match[param value] PASSED [100%]\n",
            )
            .is_ok()
        );
        let mut mismatched_pytest_arguments = pytest_arguments.clone();
        mismatched_pytest_arguments.insert(
            "test".into(),
            "other/test.py::exact_test_execution_requires_match".into(),
        );
        assert!(
            ensure_exact_test_executed(
                VerificationRunnerAdapter::Pytest,
                &pytest_target,
                &mismatched_pytest_arguments,
                None,
                &[
                    "other/test.py::exact_test_execution_requires_match".into(),
                    "-v".into()
                ],
                b"other/test.py::exact_test_execution_requires_match PASSED\n",
            )
            .is_err()
        );
        assert!(
            require_exact_runner_filter(
                VerificationRunnerAdapter::Pytest,
                &[
                    "tests/exact_test.py::exact_test_execution_requires_match".into(),
                    "-v".into(),
                ],
                &pytest_arguments,
            )
            .is_ok()
        );
        assert!(
            require_exact_runner_filter(
                VerificationRunnerAdapter::Pytest,
                &[
                    "tests/exact_test.py::exact_test_execution_requires_match".into(),
                    "-v".into(),
                    "-s".into(),
                ],
                &pytest_arguments,
            )
            .is_err()
        );
        assert!(
            require_exact_runner_filter(
                VerificationRunnerAdapter::Pytest,
                &[
                    "tests/exact_test.py::exact_test_execution_requires_match".into(),
                    "other/test.py::other_test".into(),
                ],
                &pytest_arguments,
            )
            .is_err()
        );
        assert!(
            require_exact_runner_filter(
                VerificationRunnerAdapter::Pytest,
                &[
                    "-o".into(),
                    "addopts=--strict-markers".into(),
                    "-v".into(),
                    "tests/exact_test.py::exact_test_execution_requires_match".into(),
                ],
                &pytest_arguments,
            )
            .is_ok()
        );
        assert!(
            require_exact_runner_filter(
                VerificationRunnerAdapter::Pytest,
                &[
                    "other/test.py::other_test".into(),
                    "tests/exact_test.py::exact_test_execution_requires_match".into(),
                ],
                &pytest_arguments,
            )
            .is_err()
        );

        let node_arguments = BTreeMap::from([
            (
                "test".to_string(),
                "src/app.test.ts::exact_test_execution_requires_match".to_string(),
            ),
            ("path".to_string(), "src/app.test.ts".to_string()),
            (
                "name".to_string(),
                "exact_test_execution_requires_match".to_string(),
            ),
        ]);
        let node_target = mitase_spec_model::ArtifactTarget {
            id: "node-exact-test".into(),
            adapter: "typescript".into(),
            path: RepoPath::new("src/app.test.ts").unwrap(),
            selector: mitase_spec_model::ExactSelector::File,
            lifecycle: mitase_spec_model::ArtifactTargetLifecycle::Present,
            claims: vec![],
        };
        let node_proof = ensure_exact_test_executed(
            VerificationRunnerAdapter::NodeTest,
            &node_target,
            &node_arguments,
            None,
            &[],
            b"TAP version 13\nok 1 - exact_test_execution_requires_match\n",
        )
        .expect("node test proof");
        assert_eq!(node_proof.matched_count, 1);
        let node_workspace = tempdir().expect("node workspace");
        fs::create_dir_all(node_workspace.path().join("src")).expect("node source directory");
        fs::write(
            node_workspace.path().join("src/app.test.ts"),
            "import './helper.js';\n",
        )
        .expect("node target source");
        fs::write(
            node_workspace.path().join("src/helper.js"),
            "import { test } from 'node:test';\ntest('exact_test_execution_requires_match', () => {});\n",
        )
        .expect("node imported source");
        assert!(
            ensure_exact_test_executed(
                VerificationRunnerAdapter::NodeTest,
                &node_target,
                &node_arguments,
                Some(node_workspace.path()),
                &[],
                b"TAP version 13\nok 1 - exact_test_execution_requires_match\n",
            )
            .is_err()
        );
        fs::write(
            node_workspace.path().join("src/app.test.ts"),
            "import './helper.js';\nimport { test } from 'node:test';\ntest('exact_test_execution_requires_match', () => {});\n",
        )
        .expect("node declared source");
        assert!(
            ensure_exact_test_executed(
                VerificationRunnerAdapter::NodeTest,
                &node_target,
                &node_arguments,
                Some(node_workspace.path()),
                &[],
                b"TAP version 13\nok 1 - exact_test_execution_requires_match\n",
            )
            .is_ok()
        );
        assert!(
            ensure_exact_test_executed(
                VerificationRunnerAdapter::NodeTest,
                &node_target,
                &node_arguments,
                None,
                &[],
                b"TAP version 13\nok 1 - exact_test_execution_requires_match # SKIP\n",
            )
            .is_err()
        );
        let special_node_arguments = BTreeMap::from([
            ("test".to_string(), "src/app.test.ts::foo.bar".to_string()),
            ("path".to_string(), "src/app.test.ts".to_string()),
            ("name".to_string(), "foo.bar".to_string()),
        ]);
        assert!(
            require_exact_runner_filter(
                VerificationRunnerAdapter::NodeTest,
                &[
                    "--test".into(),
                    "--test-name-pattern=^foo\\.bar$".into(),
                    "src/app.test.ts".into(),
                ],
                &special_node_arguments,
            )
            .is_ok()
        );
        assert_eq!(
            expand_runner_argument_for_adapter(
                VerificationRunnerAdapter::NodeTest,
                "--test-name-pattern=^{name}$",
                &special_node_arguments,
            ),
            "--test-name-pattern=^foo\\.bar$"
        );
        let brace_node_arguments = BTreeMap::from([
            ("test".to_string(), "src/app.test.ts::case {id}".to_string()),
            ("path".to_string(), "src/app.test.ts".to_string()),
            ("name".to_string(), "case {id}".to_string()),
        ]);
        let expanded_brace_pattern = expand_runner_argument_for_adapter(
            VerificationRunnerAdapter::NodeTest,
            "--test-name-pattern=^{name}$",
            &brace_node_arguments,
        );
        assert_eq!(
            expanded_brace_pattern,
            "--test-name-pattern=^case \\{id\\}$"
        );
        assert!(!has_unresolved_runner_placeholder(
            "--test-name-pattern=^{name}$",
            &brace_node_arguments,
        ));
        assert!(
            require_exact_runner_filter(
                VerificationRunnerAdapter::NodeTest,
                &[
                    "--test".into(),
                    "--test-name-pattern=^foo\\.bar$".into(),
                    "src/app.test.ts".into(),
                    "src/other.test.ts".into(),
                ],
                &special_node_arguments,
            )
            .is_err()
        );
        assert!(
            require_exact_runner_filter(
                VerificationRunnerAdapter::NodeTest,
                &[
                    "--test".into(),
                    "--test-name-pattern=^foo\\.bar$".into(),
                    "--test-name-pattern=.".into(),
                    "src/app.test.ts".into(),
                ],
                &special_node_arguments,
            )
            .is_err()
        );
        let node_title_arguments = BTreeMap::from([
            (
                "test".to_string(),
                "src/app.test.ts::request - succeeds # quickly".to_string(),
            ),
            ("path".to_string(), "src/app.test.ts".to_string()),
            (
                "name".to_string(),
                "request - succeeds # quickly".to_string(),
            ),
        ]);
        assert!(
            ensure_exact_test_executed(
                VerificationRunnerAdapter::NodeTest,
                &node_target,
                &node_title_arguments,
                None,
                &[],
                b"TAP version 13\nok 1 - request - succeeds \\# quickly\n",
            )
            .is_ok()
        );

        let go_arguments = BTreeMap::from([
            (
                "test".to_string(),
                "tests::exact_test_execution_requires_match".to_string(),
            ),
            ("package".to_string(), "./go".to_string()),
        ]);
        let go_target = mitase_spec_model::ArtifactTarget {
            id: "go-exact-test".into(),
            adapter: "go".into(),
            path: RepoPath::new("go/exact_test.go").unwrap(),
            selector: mitase_spec_model::ExactSelector::File,
            lifecycle: mitase_spec_model::ArtifactTargetLifecycle::Present,
            claims: vec![],
        };
        let go_proof = ensure_exact_test_executed(
            VerificationRunnerAdapter::GoTest,
            &go_target,
            &go_arguments,
            None,
            &[],
            br#"{"Action":"run","Test":"tests::exact_test_execution_requires_match"}
{"Action":"pass","Test":"tests::exact_test_execution_requires_match"}
"#,
        )
        .expect("go test proof");
        assert_eq!(go_proof.schema, VERIFICATION_PROOF_SCHEMA);
        let special_go_arguments = BTreeMap::from([
            ("test".to_string(), "TestFoo/bar.baz".to_string()),
            ("package".to_string(), "./go".to_string()),
        ]);
        assert!(
            require_exact_runner_filter(
                VerificationRunnerAdapter::GoTest,
                &[
                    "test".into(),
                    "-json".into(),
                    "-run".into(),
                    "^TestFoo$/^bar\\.baz$".into(),
                    "./go".into()
                ],
                &special_go_arguments,
            )
            .is_ok()
        );
        assert!(
            require_exact_runner_filter(
                VerificationRunnerAdapter::GoTest,
                &[
                    "test".into(),
                    "-json".into(),
                    "-run".into(),
                    "^TestFoo$/^bar\\.baz$".into(),
                    "./go".into(),
                    "./other".into(),
                ],
                &special_go_arguments,
            )
            .is_err()
        );
        assert!(
            require_exact_runner_filter(
                VerificationRunnerAdapter::GoTest,
                &[
                    "test".into(),
                    "-json".into(),
                    "-run".into(),
                    "^TestFoo$/^bar\\.baz$".into(),
                    "-run=.".into(),
                    "./go".into(),
                ],
                &special_go_arguments,
            )
            .is_err()
        );
        assert_eq!(
            expand_runner_argument_for_adapter(
                VerificationRunnerAdapter::GoTest,
                "^{test}$",
                &special_go_arguments,
            ),
            "^TestFoo$/^bar\\.baz$"
        );
        let go_workspace = tempdir().expect("go module workspace");
        fs::write(
            go_workspace.path().join("go.mod"),
            "module example.com/go-only\n",
        )
        .expect("go module");
        fs::create_dir_all(go_workspace.path().join("go")).expect("go package directory");
        fs::write(
            go_workspace.path().join("go/exact_test.go"),
            "package go\n\nfunc TestGoRequirement(t *testing.T) {}\n",
        )
        .expect("go test target");
        let module_go_arguments = BTreeMap::from([
            ("test".to_string(), "TestGoRequirement".to_string()),
            ("package".to_string(), "example.com/go-only/go".to_string()),
        ]);
        assert!(
            validate_exact_claim_identity(
                VerificationRunnerAdapter::GoTest,
                &go_target,
                &module_go_arguments,
                Some(go_workspace.path()),
            )
            .is_ok()
        );
        let mut bare_go_arguments = module_go_arguments.clone();
        bare_go_arguments.insert("package".into(), "go".into());
        assert!(
            validate_exact_claim_identity(
                VerificationRunnerAdapter::GoTest,
                &go_target,
                &bare_go_arguments,
                Some(go_workspace.path()),
            )
            .is_err()
        );
        let mut implementation_target = go_target.clone();
        implementation_target.path = RepoPath::new("go/app.go").unwrap();
        assert!(
            validate_exact_claim_identity(
                VerificationRunnerAdapter::GoTest,
                &implementation_target,
                &go_arguments,
                Some(go_workspace.path()),
            )
            .is_err()
        );

        let mut zero_match = pytest_proof;
        zero_match.matched_count = 0;
        zero_match.status = VerificationProofStatus::Failed;
        assert!(validate_verification_proof(&zero_match).is_err());
        let command = canonical_runner_arguments(
            "cargo",
            vec!["test".into(), "--package".into(), "sample".into()],
        );
        assert!(command.windows(2).any(|window| window == ["--", "--exact"]));
        assert!(
            command
                .windows(2)
                .any(|window| window == ["--format", "json"])
        );
    }

    #[test]
    fn self_hosted_shared_verification_target_has_claim_closure() {
        let repository = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("repository root");
        let tempdir = tempdir().expect("temporary clone parent");
        let worktree = tempdir.path().join("self-hosted-shared-verification");
        let status = Command::new("git")
            .args(["clone", "--local", "--no-hardlinks", "--quiet"])
            .arg(&repository)
            .arg(&worktree)
            .current_dir(&repository)
            .status()
            .expect("create self-hosted clone");
        assert!(status.success(), "create self-hosted clone");

        let workspace_root = worktree
            .canonicalize()
            .expect("canonical self-hosted clone");
        let revision = git_revision(&workspace_root);
        let workspace = SpecWorkspace::load(&workspace_root).expect("self-hosted workspace");
        let index = workspace.index().expect("self-hosted index");
        let shared_targets = [
            (
                "REQ-CAPABILITY-001#binding.delivery-verification/target.verification-test",
                2,
            ),
            (
                "REQ-WORKBENCH-012#binding.responsiveness-check/target.responsiveness-test",
                2,
            ),
            (
                "FEAT-WORKBENCH-GUIDED-JOURNEY-001#binding.journey-verification/target.journey-test",
                2,
            ),
            (
                "FEAT-AGENT-001#binding.verification/target.agent-http-test",
                4,
            ),
        ];

        let mut representative = None;
        for (target, expected_claims) in shared_targets {
            let target: BoundTargetRef = target.parse().expect("shared verification target ref");
            let criteria = index
                .target(&target)
                .expect("shared verification target")
                .claims
                .iter()
                .filter_map(|claim| match claim {
                    TargetClaim::Verifies { criterion, .. } => Some(criterion.clone()),
                    _ => None,
                })
                .collect::<Vec<_>>();
            assert_eq!(criteria.len(), expected_claims, "{target}");
            for criterion in &criteria {
                let claim = VerificationClaimRef {
                    target: target.clone(),
                    criterion: criterion.clone(),
                };
                resolve_verification_claim(&index, &claim)
                    .expect("shared verification claim closure");
            }
            if representative.is_none() {
                representative = Some((
                    target,
                    criteria
                        .into_iter()
                        .next()
                        .expect("shared verification criterion"),
                ));
            }
        }

        let (target, criterion) = representative.expect("representative shared claim");
        let request = mitase_work_model::WorkRequest {
            schema: mitase_work_model::WORK_REQUEST_SCHEMA.into(),
            id: format!("WORK-SHARED-{}", criterion.local_id),
            title: "Execute a shared verification claim.".into(),
            operation: mitase_work_model::WorkOperation::Modify,
            origin: mitase_work_model::WorkOrigin::RequirementCriterion {
                criterion: criterion.clone(),
            },
            constraints: Default::default(),
            requested_targets: vec![],
        };
        let plan = mitase_planner::plan(&request, &workspace, &index, &revision)
            .expect("shared verification plan");
        assert_eq!(
            plan.status,
            mitase_work_model::PlanStatus::Ready,
            "{plan:#?}"
        );
        let claim = VerificationClaimRef { target, criterion };
        let slice = plan
            .slices
            .iter()
            .find(|slice| {
                slice
                    .verification_targets
                    .iter()
                    .any(|planned| planned.verification_claim.as_ref() == Some(&claim))
            })
            .expect("slice with exact shared verification claim");
        let receipt = execute_verification(&workspace, &index, &plan, &slice.id, &revision)
            .expect("shared verification execution");
        let execution = receipt
            .executions
            .iter()
            .find(|execution| execution.claim.as_ref() == Some(&claim))
            .expect("criterion-specific receipt execution");
        assert_eq!(execution.target, claim.target);
        let (_, _, covers) =
            resolve_verification_claim(&index, &claim).expect("selected verification claim");
        assert_eq!(
            execution
                .implementation_digests
                .keys()
                .cloned()
                .collect::<BTreeSet<_>>(),
            covers.iter().cloned().collect(),
            "{claim:#?}"
        );
    }

    #[test]
    fn durable_failure_message_omits_runner_output() {
        let error = anyhow::anyhow!(
            "verification runner cargo-test failed with exit code 101\nstdout:\nsecret-token\nstderr:\nmore-secret-output"
        );
        let message = durable_failure_message(&error);
        assert_eq!(
            message,
            "verification runner cargo-test failed with exit code 101; runner output omitted from durable evidence"
        );
    }

    #[test]
    fn completion_report_explains_missing_receipt_execution() {
        let tempdir = tempdir().expect("tempdir");
        copy_dir(&workbench_fixture_root(), tempdir.path());
        let revision = init_git_repo(tempdir.path());
        let workspace = SpecWorkspace::load(tempdir.path()).expect("workspace");
        let index = workspace.index().expect("index");
        let plan = fixture_execution_plan(tempdir.path(), &revision);
        let slice = plan
            .slices
            .iter()
            .find(|slice| !slice.verification_targets.is_empty())
            .expect("verification slice");
        let receipt = VerificationReceipt {
            schema: VERIFICATION_RECEIPT_SCHEMA.into(),
            plan_digest: plan.canonical_digest.clone(),
            slice_id: slice.id.clone(),
            revision,
            workspace_fingerprint: workspace.try_fingerprint().expect("fingerprint"),
            started_at: "0".into(),
            completed_at: "1".into(),
            executions: vec![],
            lifecycle_proofs: vec![],
        };
        let report = evaluate_completion(&workspace, &index, &plan, &receipt).expect("report");
        assert_eq!(report.status, CompletionStatus::Blocked);
        assert!(
            report
                .blockers
                .iter()
                .any(|blocker| blocker.code == "MITASE-COMPLETION-RECEIPT")
        );
        assert!(
            report
                .blockers
                .iter()
                .any(|blocker| blocker.next_action.contains("Rerun"))
        );
    }

    #[test]
    fn completion_report_rejects_unchanged_modify_target() {
        let tempdir = tempdir().expect("tempdir");
        copy_dir(&workbench_fixture_root(), tempdir.path());
        let revision = init_git_repo(tempdir.path());
        let workspace = SpecWorkspace::load(tempdir.path()).expect("workspace");
        let index = workspace.index().expect("index");
        let plan = fixture_execution_plan(tempdir.path(), &revision);
        let slice = plan
            .slices
            .iter()
            .find(|slice| !slice.verification_targets.is_empty())
            .expect("verification slice");
        let receipt = execute_verification(&workspace, &index, &plan, &slice.id, &revision)
            .expect("exact verification");
        let report = evaluate_completion(&workspace, &index, &plan, &receipt).expect("report");
        assert_eq!(report.status, CompletionStatus::Blocked);
        assert!(report.blockers.iter().any(|blocker| {
            blocker.code == "MITASE-WORK-011" && blocker.message.contains("unchanged")
        }));
    }

    #[test]
    fn completion_report_closes_verified_slice() {
        let tempdir = tempdir().expect("tempdir");
        copy_dir(&workbench_fixture_root(), tempdir.path());
        let revision = init_git_repo(tempdir.path());
        let plan = fixture_execution_plan(tempdir.path(), &revision);
        let slice = plan
            .slices
            .iter()
            .find(|slice| !slice.verification_targets.is_empty())
            .expect("verification slice");
        fs::write(
            tempdir.path().join("src/lib.rs"),
            "mod removable;\n\npub fn behavior() -> bool {\n    true && (1 == 1)\n}\n",
        )
        .expect("post-state edit");
        let workspace = SpecWorkspace::load(tempdir.path()).expect("workspace");
        let index = workspace.index().expect("index");
        let receipt = execute_verification(&workspace, &index, &plan, &slice.id, &revision)
            .expect("exact verification");
        let report = evaluate_completion(&workspace, &index, &plan, &receipt).expect("report");
        assert_eq!(report.status, CompletionStatus::Complete, "{report:?}");
        assert_eq!(report.demonstrated.len(), 1);
        assert!(report.checks.iter().all(|check| check.passed));
        assert!(report.checks.iter().any(|check| {
            matches!(
                &check.check,
                CompletionCheck::Validate { preset } if preset == "standard"
            )
        }));

        let mut invalid_receipt = receipt;
        invalid_receipt.executions[0].proof.matched_count = 0;
        let invalid_report =
            evaluate_completion(&workspace, &index, &plan, &invalid_receipt).expect("report");
        assert_eq!(invalid_report.status, CompletionStatus::Blocked);
        assert!(invalid_report.demonstrated.is_empty());
    }

    #[test]
    fn ready_plan_does_not_mask_broken_capability_behavior() {
        let tempdir = tempdir().unwrap();
        copy_dir(&workbench_fixture_root(), tempdir.path());
        let revision = init_git_repo(tempdir.path());
        let plan = fixture_execution_plan(tempdir.path(), &revision);
        let slice = plan
            .slices
            .iter()
            .find(|slice| !slice.verification_targets.is_empty())
            .expect("verification slice");
        assert_eq!(plan.status, mitase_work_model::PlanStatus::Ready);
        fs::write(
            tempdir.path().join("src/lib.rs"),
            "mod removable;\n\npub fn behavior() -> bool {\n    false\n}\n",
        )
        .unwrap();
        let workspace = SpecWorkspace::load(tempdir.path()).unwrap();
        let index = workspace.index().unwrap();
        let error = execute_verification(&workspace, &index, &plan, &slice.id, &revision)
            .expect_err("broken behavior must fail its real verification")
            .to_string();
        assert!(
            error.contains("verification runner cargo-test failed"),
            "{error}"
        );
    }
}
