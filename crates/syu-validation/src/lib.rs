#![forbid(unsafe_code)]
mod readiness;
use anyhow::{Context, Result, bail};
pub use readiness::{
    ReadinessAxis, ReadinessAxisId, ReadinessReport, evaluate as evaluate_readiness, required_axes,
};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};
use syu_diagnostics::{Diagnostic, ValidationPhase, ValidationResult};
use syu_planner::plan as canonical_plan;
use syu_project_model::{ProjectConfig, ReadinessLevel, ValidationPreset};
use syu_spec_model::{
    BindingRole, BoundTargetRef, ItemStatus, LocalAnchorKind, OwnershipSelector, RepoPath,
    RuleLevel, Selector, SpecAnchor, SpecDocument, TargetClaim,
};
use syu_work_model::{
    ExecutionSlice, PlanConfidence, PlanExecution, TargetLifecycle, VERIFICATION_RECEIPT_SCHEMA,
    VerificationExecution, VerificationReceipt, WORK_PLAN_SCHEMA, WorkPlan, work_plan_digest,
};
use syu_workspace::{
    AnchorValue, ResolvedTarget, SpecIndex, SpecWorkspace, resolve_indexed_target,
    resolve_target_in_workspace, selector_supports_editable,
};
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
    fixed_metadata!("SYU-SCHEMA-001"),
    fixed_metadata!("SYU-SCHEMA-002"),
    fixed_metadata!("SYU-ID-001"),
    fixed_metadata!("SYU-ID-002"),
    fixed_metadata!("SYU-ANCHOR-001"),
    fixed_metadata!("SYU-ANCHOR-002"),
    fixed_metadata!("SYU-ANCHOR-003"),
    metadata!("SYU-PHILOSOPHY-001"),
    metadata!("SYU-POLICY-001"),
    metadata!("SYU-POLICY-002"),
    metadata!("SYU-POLICY-003"),
    metadata!("SYU-REQUIREMENT-001"),
    metadata!("SYU-REQUIREMENT-002"),
    metadata!("SYU-FEATURE-001"),
    metadata!("SYU-BINDING-001"),
    metadata!("SYU-BINDING-002"),
    metadata!("SYU-BINDING-003"),
    fixed_metadata!("SYU-BINDING-004"),
    fixed_metadata!("SYU-TARGET-001"),
    fixed_metadata!("SYU-TARGET-002"),
    fixed_metadata!("SYU-TARGET-003"),
    fixed_metadata!("SYU-TARGET-004"),
    fixed_metadata!("SYU-TARGET-005"),
    metadata!("SYU-FACET-001"),
    metadata!("SYU-FACET-002"),
    fixed_metadata!("SYU-CONTRACT-001"),
    fixed_metadata!("SYU-CONTRACT-002"),
    fixed_metadata!("SYU-CONTRACT-003"),
    metadata!("SYU-CONTRACT-004"),
    fixed_metadata!("SYU-CONTRACT-005"),
    metadata!("SYU-CONTRACT-006"),
    fixed_metadata!("SYU-CONTRACT-007"),
    metadata!("SYU-DOC-001"),
    metadata!("SYU-DOC-002"),
    fixed_metadata!("SYU-GENERATED-001"),
    fixed_metadata!("SYU-GENERATED-002"),
    metadata!("SYU-OPERATION-001"),
    strict_metadata!("SYU-CHANGE-001"),
    strict_metadata!("SYU-CHANGE-002"),
    strict_metadata!("SYU-CHANGE-003"),
    strict_metadata!("SYU-CHANGE-004"),
    strict_metadata!("SYU-CHANGE-005"),
    metadata!("SYU-WORK-001"),
    metadata!("SYU-WORK-002"),
    fixed_metadata!("SYU-WORK-003"),
    metadata!("SYU-WORK-004"),
    fixed_metadata!("SYU-WORK-005"),
    fixed_metadata!("SYU-WORK-006"),
    fixed_metadata!("SYU-WORK-007"),
    fixed_metadata!("SYU-WORK-008"),
    fixed_metadata!("SYU-WORK-009"),
    fixed_metadata!("SYU-WORK-010"),
    fixed_metadata!("SYU-WORK-011"),
    fixed_metadata!("SYU-WORK-012"),
    fixed_metadata!("SYU-READINESS-001"),
    fixed_metadata!("SYU-VERIFICATION-001"),
    fixed_metadata!("SYU-VERIFICATION-002"),
];

/// Canonical rule-to-phase classification for presentation clients.  This is
/// intentionally kept beside the validator so no caller needs to infer
/// semantics from a rule-id string.
pub fn phase_for_rule(rule: &str) -> ValidationPhase {
    if rule.starts_with("SYU-WORK-") || rule.starts_with("SYU-READINESS-") {
        ValidationPhase::Plan
    } else if rule.starts_with("SYU-CHANGE-") || rule == "SYU-OPERATION-001" {
        ValidationPhase::Scope
    } else if [
        "SYU-BINDING-",
        "SYU-TARGET-",
        "SYU-CONTRACT-",
        "SYU-FACET-",
        "SYU-GENERATED-",
        "SYU-VERIFICATION-",
    ]
    .iter()
    .any(|prefix| rule.starts_with(prefix))
    {
        ValidationPhase::Targets
    } else if [
        "SYU-ID-",
        "SYU-ANCHOR-",
        "SYU-PHILOSOPHY-",
        "SYU-POLICY-",
        "SYU-REQUIREMENT-",
        "SYU-FEATURE-",
        "SYU-DOC-",
    ]
    .iter()
    .any(|prefix| rule.starts_with(prefix))
    {
        ValidationPhase::Graph
    } else {
        ValidationPhase::Config
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    if submitted.basis.revision != revision {
        bail!("plan basis revision is stale");
    }
    if submitted.basis.workspace_fingerprint != workspace.try_fingerprint()? {
        bail!("plan workspace fingerprint is stale");
    }
    if submitted.canonical_digest != work_plan_digest(submitted) {
        bail!("plan canonical digest is tampered");
    }
    let canonical = canonical_plan(&submitted.request, workspace, index, revision)?;
    if canonical != *submitted {
        bail!("submitted plan does not match deterministic canonical planner output");
    }
    if !matches!(canonical.status, syu_work_model::PlanStatus::Ready) {
        bail!("verification requires a ready canonical plan");
    }
    if canonical.slices.is_empty() {
        bail!("verification requires at least one canonical slice");
    }
    Ok(canonical)
}

/// Execute exactly the verification targets selected by a canonical slice.
/// Runner executable and arguments come only from the workspace registry and
/// target claim; no planner or caller guesses a command.
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
    let pre_state = validate_without_readiness(&ValidationContext {
        config: &workspace.config,
        workspace,
        index,
        changed_files: None,
        reported_changed_files: None,
        work_plan: Some(&plan),
        selected_slice: Some(slice),
        plan_mode: PlanValidationMode::PreState,
        preset: workspace.config.validation.preset,
        revision: Some(revision),
        change_base_revision: None,
    });
    if pre_state
        .diagnostics
        .iter()
        .any(|diagnostic| matches!(diagnostic.severity, syu_diagnostics::Severity::Error))
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
        let target = index.target(&planned.reference).ok_or_else(|| {
            anyhow::anyhow!("verification target {} is unresolved", planned.reference)
        })?;
        let claims = target
            .claims
            .iter()
            .filter_map(|claim| match claim {
                TargetClaim::Verifies {
                    criterion,
                    covers,
                    runner,
                } => Some((criterion, covers, runner)),
                _ => None,
            })
            .collect::<Vec<_>>();
        if claims.len() != 1 {
            bail!(
                "verification target {} must have exactly one verification claim",
                planned.reference
            );
        }
        let (_, covers, runner_ref) = claims[0];
        if covers.is_empty() {
            bail!("verification target {} has no covers", planned.reference);
        }
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
        let arguments = configured
            .arguments
            .iter()
            .map(|argument| expand_runner_argument(argument, &runner_ref.arguments))
            .collect::<Vec<_>>();
        if arguments.iter().any(|argument| argument.contains('{')) {
            bail!(
                "verification runner {} has unresolved arguments",
                runner_ref.runner
            );
        }
        let mut command = Command::new(&configured.executable);
        command.args(&arguments).current_dir(&workspace.root);
        if configured.executable == "cargo" {
            // Reuse one ignored target directory for all exact verification
            // jobs in this workspace. A fresh target per test is isolated but
            // needlessly consumes gigabytes and makes a readiness report fail
            // before the actual tests can run.
            command.env(
                "CARGO_TARGET_DIR",
                workspace.root.join("target").join("syu-verification"),
            );
        }
        let output = command
            .output()
            .with_context(|| format!("execute verification runner {}", runner_ref.runner))?;
        if !output.status.success() {
            bail!(
                "verification runner {} failed with exit code {}\nstdout:\n{}\nstderr:\n{}",
                runner_ref.runner,
                output.status.code().unwrap_or(-1),
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr),
            );
        }
        ensure_exact_test_executed(
            &configured.executable,
            target,
            &runner_ref.arguments,
            &output.stdout,
        )?;
        let mut implementation_digests = BTreeMap::new();
        for covered in covers {
            let covered_target = index
                .target(covered)
                .ok_or_else(|| anyhow::anyhow!("covered target {covered} is unresolved"))?;
            let resolved = syu_workspace::resolve_target_in_workspace(workspace, covered_target)?;
            implementation_digests.insert(covered.clone(), resolved.content_hash);
        }
        let verification = syu_workspace::resolve_target_in_workspace(workspace, target)?;
        executions.push(VerificationExecution {
            target: planned.reference.clone(),
            runner: runner_ref.runner.clone(),
            command: std::iter::once(configured.executable.clone())
                .chain(arguments)
                .collect(),
            exit_code: output.status.code().unwrap_or(-1),
            stdout_digest: digest(&output.stdout),
            stderr_digest: digest(&output.stderr),
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
    };
    validate_verification_receipt(workspace, index, &plan, slice_id, &receipt, revision)?;
    Ok(receipt)
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
    let expected = slice
        .verification_targets
        .iter()
        .map(|target| target.reference.clone())
        .collect::<BTreeSet<_>>();
    let actual = receipt
        .executions
        .iter()
        .map(|execution| execution.target.clone())
        .collect::<Vec<_>>();
    if actual.len() != expected.len() || actual.into_iter().collect::<BTreeSet<_>>() != expected {
        bail!("verification receipt execution set is not exact");
    }
    for execution in &receipt.executions {
        if execution.exit_code != 0 {
            bail!("verification receipt contains failed executions");
        }
        let target = index.target(&execution.target).ok_or_else(|| {
            anyhow::anyhow!("verification target {} is unresolved", execution.target)
        })?;
        let (runner_ref, covers) = target
            .claims
            .iter()
            .find_map(|claim| match claim {
                TargetClaim::Verifies { runner, covers, .. } => Some((runner, covers)),
                _ => None,
            })
            .ok_or_else(|| anyhow::anyhow!("{} is not a verification target", execution.target))?;
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
        let arguments = configured
            .arguments
            .iter()
            .map(|argument| expand_runner_argument(argument, &runner_ref.arguments))
            .collect::<Vec<_>>();
        let expected_command = std::iter::once(configured.executable.clone())
            .chain(arguments)
            .collect::<Vec<_>>();
        if execution.runner != runner_ref.runner
            || execution.command != expected_command
            || execution.command.is_empty()
        {
            bail!("verification receipt command does not match the configured runner");
        }
        let verification = syu_workspace::resolve_target_in_workspace(workspace, target)?;
        if execution.verification_digest != verification.content_hash {
            bail!("verification target digest is stale");
        }
        for covered in covers {
            let covered_target = index
                .target(covered)
                .ok_or_else(|| anyhow::anyhow!("covered target {covered} is unresolved"))?;
            let resolved = syu_workspace::resolve_target_in_workspace(workspace, covered_target)?;
            if execution.implementation_digests.get(covered) != Some(&resolved.content_hash) {
                bail!("verification implementation digest is stale");
            }
        }
        if execution.implementation_digests.len() != covers.len() {
            bail!("receipt implementation digest set is not exact");
        }
    }
    Ok(())
}

fn ensure_exact_test_executed(
    executable: &str,
    target: &syu_spec_model::ArtifactTarget,
    claim_arguments: &BTreeMap<String, String>,
    stdout: &[u8],
) -> Result<()> {
    if executable != "cargo" {
        return Ok(());
    }
    let Some(Selector::Symbol { name }) = Some(&target.selector) else {
        bail!("cargo verification targets must use an exact symbol selector");
    };
    let Some(test_identity) = claim_arguments.get("test") else {
        bail!("cargo verification claim must name the exact test identity");
    };
    if test_identity != name && !test_identity.ends_with(&format!("::{name}")) {
        bail!("cargo verification argument {test_identity} does not identify selector {name}");
    }
    let output = String::from_utf8_lossy(stdout);
    let marker = format!("test {test_identity} ");
    if !output
        .lines()
        .any(|line| line.trim_start().starts_with(&marker))
    {
        bail!("configured verification command ran zero exact tests for {name}");
    }
    Ok(())
}

fn expand_runner_argument(template: &str, values: &BTreeMap<String, String>) -> String {
    values
        .iter()
        .fold(template.to_owned(), |value, (key, replacement)| {
            value.replace(&format!("{{{key}}}"), replacement)
        })
}

fn digest(bytes: &[u8]) -> String {
    let mut hash = Sha256::new();
    hash.update(bytes);
    format!("sha256:{:x}", hash.finalize())
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
        let unmet = required_axes(ctx.config.validation.readiness.target)
            .iter()
            .any(|axis| match axis {
                ReadinessAxisId::Inventory => !report.inventory.is_ready(),
                ReadinessAxisId::Ownership => !report.ownership.is_ready(),
                ReadinessAxisId::Seedability => !report.seedability.is_ready(),
                ReadinessAxisId::Workability => !report.workability.is_ready(),
                ReadinessAxisId::Verification => !report.verification.is_ready(),
                ReadinessAxisId::ClosedLoop => !report.closed_loop.is_ready(),
            });
        if readiness_required(ctx.config.validation.readiness.target) && unmet {
            diagnostics.push(syu_diagnostics::Diagnostic::error(
                "SYU-READINESS-001",
                "workspace does not meet the configured readiness target",
                "syu.yaml",
            ));
        }
    } else if readiness.as_ref().is_some_and(Result::is_err)
        && readiness_required(ctx.config.validation.readiness.target)
    {
        diagnostics.push(syu_diagnostics::Diagnostic::error(
            "SYU-READINESS-001",
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
            "SYU-SCHEMA-002",
            format!(
                "active inventory profile {} is not defined",
                ctx.config.inventory.active_profile
            ),
            "syu.yaml",
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
        if !ctx.workspace.path_is_artifact(path.as_path())
            || ctx.workspace.path_is_excluded(path.as_path())
        {
            if ctx.config.validation.changed.require_owned_changes {
                push(
                    out,
                    "SYU-CHANGE-001",
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
                !matches!(unit.kind, syu_inventory::ArtifactUnitKind::File)
                    && unit.exposure != syu_inventory::ArtifactExposure::Support
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
                syu_inventory::ArtifactUnit {
                    adapter: "declared".into(),
                    identity: format!("declared:{}", old_path.to_string_lossy()),
                    path: old_path,
                    kind: syu_inventory::ArtifactUnitKind::File,
                    exposure: syu_inventory::ArtifactExposure::Workspace,
                    reachability: syu_inventory::ArtifactReachability::Active,
                    span: syu_inventory::SourceSpan {
                        byte_start: 0,
                        byte_end: 0,
                        line_start: 1,
                        line_end: end.max(1),
                    },
                    digest: "deleted".into(),
                },
                false,
            ));
        }
        if units.is_empty() {
            if ctx.config.validation.changed.require_owned_changes {
                push(
                    out,
                    "SYU-CHANGE-001",
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
            let owned = index
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
                                matches.then(|| syu_workspace::OwnershipRef {
                                    binding: binding_anchor.clone(),
                                    scope_id: scope.id.clone(),
                                    target_id: None,
                                })
                            })
                        })
                        .collect::<Vec<_>>()
                });
            let owners = Some(owned.as_slice());
            if ctx.config.validation.changed.require_owned_changes
                && owners.is_none_or(|owners| owners.is_empty())
            {
                push(
                    out,
                    "SYU-CHANGE-001",
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
                    && !is_capability_binding(ctx, &owner.binding)
                    && !binding.targets.iter().any(|target| {
                        target.claims.iter().any(|claim| {
                            matches!(claim, syu_spec_model::TargetClaim::Satisfies { .. })
                        })
                    })
                {
                    push(
                        out,
                        "SYU-CHANGE-002",
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
        if !(implementation_changed || verification_changed) {
            push(
                out,
                "SYU-CHANGE-003",
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
                "SYU-CHANGE-004",
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
                "SYU-CHANGE-005",
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
    let syu_config = git_show(root, revision, Path::new("syu.yaml")).ok()?;
    let config: ProjectConfig = serde_yaml::from_str(&syu_config).ok()?;
    let tempdir = tempfile::Builder::new()
        .prefix("syu-baseline-")
        .tempdir()
        .ok()?;
    let workspace_dir = tempdir.path();
    fs::write(workspace_dir.join("syu.yaml"), syu_config).ok()?;
    let files = git_ls_tree(root, revision).ok()?;
    for relative in &files {
        let include = relative == Path::new("syu.yaml")
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
            fs::create_dir_all(parent).ok()?;
        }
        fs::write(destination, contents).ok()?;
    }
    let workspace = SpecWorkspace::load(workspace_dir).ok()?;
    let index = workspace.index().ok()?;
    Some(BaselineWorkspace {
        _tempdir: tempdir,
        workspace,
        index,
    })
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
        .unwrap_or_else(|| "syu-spec".into())
}

fn normalize_workspace_path(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

fn workspace_relative_repo_path(path: &Path, root: &Path) -> Option<RepoPath> {
    workspace_relative_display(path, root).and_then(|relative| RepoPath::new(relative).ok())
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
                            "SYU-PHILOSOPHY-001",
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
                            "SYU-BINDING-002",
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
                        push(out, "SYU-POLICY-001", "policy has no Rule", &path, None);
                    }
                }
            }
            SpecDocument::Requirements { requirements, .. } => {
                for item in requirements {
                    if item.status == ItemStatus::Implemented && item.criteria.is_empty() {
                        push(
                            out,
                            "SYU-REQUIREMENT-001",
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
                            "SYU-BINDING-002",
                            "requirement cannot own implementation bindings",
                            &path,
                            None,
                        );
                    }
                }
            }
            SpecDocument::Features { features, .. } => {
                for item in features {
                    if item.status == ItemStatus::Implemented
                        && !item
                            .bindings
                            .iter()
                            .any(|binding| binding.role == BindingRole::Implementation)
                    {
                        push(
                            out,
                            "SYU-FEATURE-001",
                            "implemented feature has no implementation binding",
                            &path,
                            None,
                        );
                    }
                }
            }
        }
    }
}
fn is_capability_binding(ctx: &ValidationContext<'_>, anchor: &SpecAnchor) -> bool {
    let Some(path) = ctx.index.item_paths.get(&anchor.item) else {
        return false;
    };
    ctx.workspace
        .documents
        .iter()
        .find(|loaded| &loaded.path == path)
        .is_some_and(|loaded| {
            matches!(
                &loaded.document,
                SpecDocument::Features { namespace, .. } if namespace == "capabilities"
            )
        })
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
                        "SYU-POLICY-002",
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
                        .any(|b| b.targets.iter().any(|target| target.claims.iter().any(|claim| matches!(claim, syu_spec_model::TargetClaim::Enforces { rule } if rule == anchor) || matches!(claim, syu_spec_model::TargetClaim::Evidences { anchor: evidence } if evidence == anchor))))
                        || rule.enforcement.is_some();
                    if !covered {
                        push(
                            out,
                            "SYU-POLICY-003",
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
                        "SYU-REQUIREMENT-002",
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
                        "SYU-BINDING-003",
                        "binding responsibility is empty",
                        &path,
                        Some(anchor.clone()),
                    );
                }
                if binding.targets.is_empty() {
                    push(
                        out,
                        "SYU-BINDING-004",
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
                        syu_spec_model::TargetClaim::Satisfies { criterion }
                            if binding.role == BindingRole::Implementation =>
                        {
                            Some(criterion)
                        }
                        syu_spec_model::TargetClaim::Verifies { criterion, .. }
                            if binding.role == BindingRole::Verification =>
                        {
                            Some(criterion)
                        }
                        syu_spec_model::TargetClaim::Documents { anchor }
                            if binding.role == BindingRole::Documentation =>
                        {
                            Some(anchor)
                        }
                        syu_spec_model::TargetClaim::Enforces { rule }
                            if binding.role == BindingRole::Enforcement =>
                        {
                            Some(rule)
                        }
                        syu_spec_model::TargetClaim::Evidences { anchor }
                            if binding.role == BindingRole::Evidence =>
                        {
                            Some(anchor)
                        }
                        _ => None,
                    })
                    .collect::<Vec<_>>();
                if matches!(
                    binding.role,
                    BindingRole::Implementation
                        | BindingRole::Verification
                        | BindingRole::Documentation
                        | BindingRole::Enforcement
                        | BindingRole::Evidence
                ) && relation.is_empty()
                    && !is_capability_binding(ctx, anchor)
                {
                    push(
                        out,
                        "SYU-BINDING-001",
                        "binding role requires its canonical relation",
                        &path,
                        Some(anchor.clone()),
                    );
                }
                for target in &relation {
                    if !ctx.index.anchors.contains_key(target) {
                        push(
                            out,
                            "SYU-ANCHOR-002",
                            format!("unresolved relation {target}"),
                            &path,
                            Some(anchor.clone()),
                        );
                    }
                }
                for artifact_target in &binding.targets {
                    for claim in &artifact_target.claims {
                        if let syu_spec_model::TargetClaim::Verifies { covers, .. } = claim {
                            if covers.is_empty() {
                                push(
                                    out,
                                    "SYU-VERIFICATION-001",
                                    "verification target must cover at least one exact target",
                                    &path,
                                    Some(anchor.clone()),
                                );
                            }
                            for covered in covers {
                                if ctx.index.target(covered).is_none() {
                                    push(
                                        out,
                                        "SYU-VERIFICATION-002",
                                        format!("verification covers unresolved target {covered}"),
                                        &path,
                                        Some(anchor.clone()),
                                    );
                                }
                            }
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
                let generated_from = binding
                    .targets
                    .iter()
                    .flat_map(|target| target.claims.iter())
                    .filter_map(|claim| match claim {
                        syu_spec_model::TargetClaim::GeneratedFrom { targets } => {
                            Some(targets.as_slice())
                        }
                        _ => None,
                    })
                    .flatten()
                    .collect::<Vec<_>>();
                if binding.role == BindingRole::Generated && generated_from.is_empty() {
                    push(
                        out,
                        "SYU-GENERATED-001",
                        "generated binding has no generated_from target",
                        &path,
                        Some(anchor.clone()),
                    );
                }
                if binding.role == BindingRole::Generated && !generated_from.is_empty() {
                    validate_generated_binding(ctx, anchor, &generated_from, out, &path);
                }
            }
            AnchorValue::Contract(contract) => {
                if ctx.index.target(&contract.source).is_none() {
                    push(
                        out,
                        "SYU-CONTRACT-001",
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
                        "SYU-CONTRACT-002",
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
                            "SYU-CONTRACT-005",
                            format!("contract guarantee is duplicated: {guarantee}"),
                            &path,
                            Some(anchor.clone()),
                        );
                        continue;
                    }
                    match ctx.index.anchor(guarantee) {
                        None => push(
                            out,
                            "SYU-CONTRACT-005",
                            format!("contract guarantee target does not exist: {guarantee}"),
                            &path,
                            Some(anchor.clone()),
                        ),
                        Some(AnchorValue::Criterion(_)) | Some(AnchorValue::Rule(_)) => {}
                        Some(_) => push(
                            out,
                            "SYU-CONTRACT-005",
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
                            "SYU-CONTRACT-007",
                            "contract participant role must not be empty",
                            &path,
                            Some(anchor.clone()),
                        );
                    }
                    if !seen_participants.insert((p.target.clone(), p.role.clone())) {
                        push(
                            out,
                            "SYU-CONTRACT-007",
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
                            "SYU-CONTRACT-003",
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

fn validate_generated_binding(
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
                "SYU-GENERATED-002",
                format!("generated binding cannot reference itself: {generated}"),
                path,
                Some(anchor.clone()),
            );
            continue;
        }
        if !seen.insert((*generated).clone()) {
            push(
                out,
                "SYU-GENERATED-002",
                format!("generated_from target is duplicated: {generated}"),
                path,
                Some(anchor.clone()),
            );
        }
        if ctx.index.target(generated).is_none() {
            push(
                out,
                "SYU-GENERATED-002",
                format!("generated_from target does not exist: {generated}"),
                path,
                Some(anchor.clone()),
            );
        }
    }
    if generated_binding_has_cycle(ctx, anchor, &mut BTreeSet::new(), &mut BTreeSet::new()) {
        push(
            out,
            "SYU-GENERATED-002",
            "generated binding contains a generated_from cycle",
            path,
            Some(anchor.clone()),
        );
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
            syu_spec_model::TargetClaim::GeneratedFrom { targets } => Some(targets),
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
                "SYU-ANCHOR-003",
                format!("{target} must reference a {}", expected.label()),
                path,
                Some(target.clone()),
            );
        }
    } else {
        push(
            out,
            "SYU-ANCHOR-002",
            format!("unresolved anchor {target}"),
            path,
            Some(target.clone()),
        );
    }
}

fn validate_targets(ctx: &ValidationContext<'_>, out: &mut Vec<Diagnostic>) {
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
                "SYU-FACET-001",
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
                    "SYU-TARGET-003",
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
                            "SYU-TARGET-001",
                            "symbol selector must contain at least one name",
                            target.path.to_string_lossy(),
                            Some(anchor.clone()),
                        );
                    }
                }
                Selector::Heading { value } if value.trim().is_empty() => {
                    push(
                        out,
                        "SYU-TARGET-001",
                        "heading selector must not be empty",
                        target.path.to_string_lossy(),
                        Some(anchor.clone()),
                    );
                }
                Selector::Marker { value } if value.trim().is_empty() => {
                    push(
                        out,
                        "SYU-TARGET-001",
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
                        "SYU-TARGET-001",
                        "operation selector must not be empty",
                        target.path.to_string_lossy(),
                        Some(anchor.clone()),
                    );
                }
                Selector::JsonPointer { value } if value.trim().is_empty() => {
                    push(
                        out,
                        "SYU-TARGET-001",
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
                    matches!(target.adapter.as_str(), "yaml" | "json" | "openapi")
                }
                Selector::Marker { .. } => true,
            };
            if !expected {
                push(
                    out,
                    "SYU-TARGET-005",
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
                    "SYU-TARGET-004",
                    "implementation target must use an exact editable selector",
                    target.path.to_string_lossy(),
                    Some(anchor.clone()),
                );
            }
            let target_ref = BoundTargetRef {
                binding: anchor.clone(),
                target_id: target.id.clone(),
            };
            if ctx.plan_mode == PlanValidationMode::PostState
                && allowed_absent_targets.contains(&target_ref)
                && ctx.index.target(&target_ref).is_some()
            {
                push(
                    out,
                    "SYU-WORK-011",
                    format!("removed target declaration still exists: {target_ref}"),
                    target.path.to_string_lossy(),
                    Some(anchor.clone()),
                );
            }
            if let Err(e) = resolve_target_in_workspace(ctx.workspace, target)
                && !allowed_absent_targets.contains(&target_ref)
            {
                push(
                    out,
                    "SYU-TARGET-002",
                    e.to_string(),
                    target.path.to_string_lossy(),
                    Some(anchor.clone()),
                );
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
                        "SYU-VERIFICATION-001",
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
                            "SYU-VERIFICATION-002",
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
                        "SYU-VERIFICATION-002",
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
                            "SYU-VERIFICATION-002",
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
                push(out, "SYU-VERIFICATION-002", format!("implementation target {implementation} is not covered by a verification target for {criterion}"), "workspace", Some(criterion.clone()));
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
                    "SYU-CONTRACT-006",
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
            "SYU-SCHEMA-001",
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
            "SYU-WORK-009",
            "plan basis revision is stale",
            "work-plan",
            None,
        );
    }
    let basis_workspace = load_workspace_at_revision(&ctx.workspace.root, &plan.basis.revision);
    // A Workbench plan may be created against the current working tree while
    // its revision still names HEAD.  Prefer the live indexed workspace when
    // its fingerprint is the submitted basis; reconstructing HEAD here would
    // incorrectly compare a valid dirty-tree plan with an older filesystem.
    let current_workspace_is_basis =
        plan.basis.workspace_fingerprint == ctx.workspace.try_fingerprint().unwrap_or_default();
    if basis_workspace.is_none() && !current_workspace_is_basis {
        push(
            out,
            "SYU-WORK-009",
            "plan basis revision cannot be reconstructed",
            "work-plan",
            None,
        );
    }
    if plan.canonical_digest != work_plan_digest(plan) {
        push(
            out,
            "SYU-WORK-009",
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
                        "SYU-WORK-009",
                        "plan structure does not match the canonical planner output",
                        "work-plan",
                        None,
                    );
                }
            }
            Err(error) => push(
                out,
                "SYU-WORK-009",
                format!("plan request no longer replans cleanly: {error:#}"),
                "work-plan",
                None,
            ),
        }
    }
    if plan.execution != PlanExecution::IsolatedSlices {
        push(
            out,
            "SYU-WORK-009",
            "work plan execution mode must be isolated-slices",
            "work-plan",
            None,
        );
    }
    if allow_post_state && plan.slices.len() > 1 && ctx.selected_slice.is_none() {
        push(
            out,
            "SYU-WORK-009",
            "post-state validation requires --slice when a plan has multiple slices",
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
            "SYU-WORK-009",
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
                "SYU-WORK-009",
                format!("duplicate slice id: {}", slice.id),
                "work-plan",
                None,
            );
        }
        if slice.completion.is_empty() {
            push(
                out,
                "SYU-WORK-011",
                "slice has no completion check",
                "work-plan",
                None,
            );
        }
        if slice.confidence == PlanConfidence::Low && !slice.editable_targets.is_empty() {
            push(
                out,
                "SYU-WORK-010",
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
            .filter(|target| target.access == syu_work_model::TargetAccessMode::Editable)
            .map(|target| target.resolved_path.as_str())
            .collect::<BTreeSet<_>>()
            .len();
        let actual_symbols: usize = slice
            .editable_targets
            .iter()
            .chain(&slice.verification_targets)
            .filter(|target| target.access == syu_work_model::TargetAccessMode::Editable)
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
                "SYU-WORK-009",
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
                "SYU-WORK-003",
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
                            "SYU-WORK-003",
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
                        && target.access == syu_work_model::TargetAccessMode::Editable
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
                    "SYU-WORK-009",
                    format!("target snapshot is stale: {}", target.reference),
                    &target.resolved_path,
                    Some(target.reference.binding.clone()),
                ),
            }
        }
        for completion in &slice.completion {
            match completion {
                syu_work_model::CompletionCheck::TargetExists { target } => {
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
                            "SYU-WORK-011",
                            format!("expected target is still missing: {target}"),
                            "work-plan",
                            Some(target.binding.clone()),
                        );
                    }
                }
                syu_work_model::CompletionCheck::TargetAbsent { target } => {
                    if allow_post_state && ctx.index.target(target).is_some() {
                        push(
                            out,
                            "SYU-WORK-011",
                            format!("removed target declaration still exists: {target}"),
                            "work-plan",
                            Some(target.binding.clone()),
                        );
                    } else if ctx
                        .index
                        .target(target)
                        .and_then(|declared| {
                            resolve_target_in_workspace(ctx.workspace, declared).ok()
                        })
                        .is_some()
                    {
                        push(
                            out,
                            "SYU-WORK-011",
                            format!("expected removed target still exists: {target}"),
                            "work-plan",
                            Some(target.binding.clone()),
                        );
                    }
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
                    "SYU-WORK-007",
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
                            "SYU-WORK-008",
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
                    "SYU-WORK-012",
                    "acceptance statement differs from criterion",
                    "work-plan",
                    Some(acceptance.anchor.clone()),
                );
            }
        }
    }
}

fn target_budget_bytes(target: &syu_work_model::PlannedTarget) -> usize {
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
        .filter(|target| target.access == syu_work_model::TargetAccessMode::Editable)
        .collect::<Vec<_>>();
    let guarded_targets = slice
        .verification_targets
        .iter()
        .filter(|target| target.access == syu_work_model::TargetAccessMode::RunOnly)
        .chain(slice.readonly_context.iter())
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
            let editable_hit = editable_targets
                .iter()
                .any(|target| editable_target_matches_hunkless_change(ctx, target, file));
            if readonly_hit {
                push(
                    out,
                    "SYU-WORK-005",
                    format!("readonly or run-only target changed: {}", path.display()),
                    path.to_string_lossy(),
                    None,
                );
            } else if !editable_hit {
                push(
                    out,
                    "SYU-WORK-006",
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
            if readonly_hit {
                push(
                    out,
                    "SYU-WORK-005",
                    format!("readonly or run-only target changed: {}", path.display()),
                    path.to_string_lossy(),
                    None,
                );
            } else if !editable_hit {
                push(
                    out,
                    "SYU-WORK-006",
                    format!("change is outside editable scope: {}", path.display()),
                    path.to_string_lossy(),
                    None,
                );
            }
        }
    }
}

fn target_matches_changed_file_path(
    ctx: &ValidationContext<'_>,
    target: &syu_work_model::PlannedTarget,
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

fn editable_target_matches_hunkless_change(
    ctx: &ValidationContext<'_>,
    target: &syu_work_model::PlannedTarget,
    file: &ChangedFile,
) -> bool {
    target_selector_is_file(target) && target_matches_changed_file_path(ctx, target, file)
}

fn change_is_within_editable_scope(
    ctx: &ValidationContext<'_>,
    editable_targets: &[&syu_work_model::PlannedTarget],
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
        Some(path) => changed_side_is_fully_covered(
            hunk.new_start,
            hunk.new_end,
            editable_targets
                .iter()
                .filter_map(|target| target_line_range(ctx, target, TargetRangeSide::New, path)),
        ),
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
    target: &syu_work_model::PlannedTarget,
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
    target: &syu_work_model::PlannedTarget,
    side: TargetRangeSide,
    changed_path: &RepoPath,
) -> Option<(usize, usize)> {
    if let Some(identity) = &target.artifact_identity {
        let unit = ctx
            .index
            .artifact_units
            .iter()
            .find(|unit| &unit.identity == identity)?;
        if unit.path.to_string_lossy() != changed_path.to_string_lossy() {
            return None;
        }
        return Some((unit.span.line_start, unit.span.line_end));
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
    target: &syu_work_model::PlannedTarget,
) -> Option<ResolvedTarget> {
    if let Some(identity) = &target.artifact_identity {
        let unit = ctx
            .index
            .artifact_units
            .iter()
            .find(|unit| &unit.identity == identity)?;
        if !matches!(
            unit.reachability,
            syu_inventory::ArtifactReachability::Active
        ) {
            return None;
        }
        let bytes = ctx.workspace.read_bytes(unit.path.as_path()).ok()?;
        let byte_start = unit.span.byte_start.min(bytes.len());
        let byte_end = unit.span.byte_end.min(bytes.len()).max(byte_start);
        let symbol = identity.rsplit("::").next().unwrap_or(identity).to_owned();
        return Some(ResolvedTarget {
            path: unit.path.as_path().to_path_buf(),
            description: format!("changed semantic artifact {identity}"),
            symbols: if matches!(unit.kind, syu_inventory::ArtifactUnitKind::File) {
                vec![]
            } else {
                vec![symbol]
            },
            content_hash: unit.digest.clone(),
            bytes: bytes.len(),
            byte_start,
            byte_end,
            line_start: unit.span.line_start,
            line_end: unit.span.line_end,
            excerpt: String::from_utf8_lossy(&bytes[byte_start..byte_end]).into_owned(),
            excerpt_hash: unit.digest.clone(),
        });
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
    target: &syu_work_model::PlannedTarget,
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

fn target_selector_is_file(target: &syu_work_model::PlannedTarget) -> bool {
    target.resolved_selector.description == "file"
}

fn ensure_present_target_exceeds_budget(
    target: &syu_work_model::PlannedTarget,
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
    use std::fs;
    use std::process::Command;
    use syu_work_model::TargetTransition;
    use tempfile::tempdir;

    fn fixture_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/v1/valid-web-app")
            .canonicalize()
            .expect("fixture root")
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

    fn write_generated_binding_workspace(root: &Path) {
        fs::create_dir_all(root.join("spec")).expect("spec dir");
        fs::create_dir_all(root.join("src")).expect("src dir");
        fs::write(
            root.join("syu.yaml"),
            concat!(
                "schema: syu/config/v1\n",
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
                "    limits: { max_ownership_scope_units: 64, max_targets_per_binding: 12, max_slices_per_seed: 4 }\n",
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
                "schema: syu/spec/v1\n",
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

    fn sample_target(
        path: &str,
        description: &str,
        lines: (usize, usize),
    ) -> syu_work_model::PlannedTarget {
        syu_work_model::PlannedTarget {
            reference: "FEAT-AUTH-001#binding.ui/target.requested".parse().unwrap(),
            artifact_identity: None,
            transition: TargetTransition::Add,
            lifecycle: TargetLifecycle::EnsurePresent,
            access: syu_work_model::TargetAccessMode::Editable,
            resolved_path: path.to_string(),
            resolved_selector: syu_work_model::ResolvedSelector {
                description: description.to_string(),
                symbols: if description == "file" {
                    Vec::new()
                } else {
                    vec!["requested_function".to_string()]
                },
            },
            content_hash: "sha256:0".to_string(),
            excerpt_hash: "sha256:0".to_string(),
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
            r#"schema: syu/spec/v1
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
    fn validate_reports_deleted_criterion_without_artifact_update() {
        let (tempdir, _, _) = load_fixture_workspace();
        let baseline = init_git_repo(tempdir.path());
        fs::write(
            tempdir.path().join("spec/requirement.yaml"),
            r#"schema: syu/spec/v1
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
                .any(|diagnostic| diagnostic.rule_id == "SYU-CHANGE-003")
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
                .any(|diagnostic| diagnostic.rule_id == "SYU-GENERATED-001")
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
}
