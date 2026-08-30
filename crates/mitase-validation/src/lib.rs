#![forbid(unsafe_code)]
mod readiness;
use anyhow::{Context, Result, bail};
use mitase_diagnostics::{Diagnostic, ValidationPhase, ValidationResult};
use mitase_inventory::ArtifactUnitKind;
use mitase_project_model::{ProjectConfig, ReadinessLevel, ValidationPreset};
use mitase_spec_model::{
    ArtifactTarget, ArtifactTargetLifecycle, BindingRole, BoundTargetRef, ItemStatus,
    LocalAnchorKind, OwnershipSelector, RepoPath, RuleLevel, Selector, SpecAnchor, SpecDocument,
    TargetClaim, VerificationRunnerRef,
};
use mitase_workspace::{
    AnchorValue, ResolvedTarget, SpecIndex, SpecWorkspace, resolve_target_in_workspace,
    selector_supports_editable,
};
pub use readiness::{
    ReadinessAxis, ReadinessAxisId, ReadinessReport, ReadinessSubject,
    evaluate as evaluate_readiness, required_axes,
};
use serde::Serialize;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct VerificationClaimRef {
    pub target: BoundTargetRef,
    pub criterion: SpecAnchor,
}

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
            presets: &[ValidationPreset::Standard, ValidationPreset::Strict],
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
            presets: &[ValidationPreset::Strict],
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
            presets: &[ValidationPreset::Standard, ValidationPreset::Strict],
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
    fixed_metadata!("MITASE-READINESS-001"),
    fixed_metadata!("MITASE-VERIFICATION-001"),
    fixed_metadata!("MITASE-VERIFICATION-002"),
];

/// Canonical rule-to-phase classification for presentation clients.  This is
/// intentionally kept beside the validator so no caller needs to infer
/// semantics from a rule-id string.
pub fn phase_for_rule(rule: &str) -> ValidationPhase {
    if rule.starts_with("MITASE-READINESS-") {
        ValidationPhase::Readiness
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

pub struct ValidationContext<'a> {
    pub config: &'a ProjectConfig,
    pub workspace: &'a SpecWorkspace,
    pub index: &'a SpecIndex,
    pub changed_files: Option<&'a [ChangedFile]>,
    pub reported_changed_files: Option<&'a [ChangedFile]>,
    pub preset: ValidationPreset,
    pub revision: Option<&'a str>,
    pub change_base_revision: Option<&'a str>,
}

pub trait ValidationRule {
    fn metadata(&self) -> &'static RuleMetadata;
    fn evaluate(&self, ctx: &ValidationContext<'_>, out: &mut Vec<Diagnostic>);
}

pub fn validate(ctx: &ValidationContext<'_>) -> ValidationResult {
    validate_inner(ctx, false)
}

/// Validate a canonical workspace and enforce its configured readiness target.
pub fn validate_workspace(ctx: &ValidationContext<'_>) -> ValidationResult {
    validate_inner(ctx, true)
}

pub fn validate_without_readiness(ctx: &ValidationContext<'_>) -> ValidationResult {
    validate_inner(ctx, false)
}

/// Resolve one configured verification claim. A verification target may serve
/// several criteria, so readiness inspection names precisely one criterion.
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
            ReadinessLevel::Traceable | ReadinessLevel::Verifiable
        )
    {
        push(
            out,
            "MITASE-SCHEMA-003",
            "public entrypoint readiness probes support only off or seedable in v1",
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

fn requires_change_ownership(
    unit: &mitase_inventory::ArtifactUnit,
    owners: &[mitase_workspace::OwnershipRef],
) -> bool {
    // Declared file units describe repository support and control-plane
    // artifacts. Their delivery contract lives in repository tooling, so an
    // unowned declared file must not be mistaken for an unowned product
    // implementation or verification target.
    owners.is_empty()
        && !(unit.adapter == "declared"
            && matches!(unit.kind, mitase_inventory::ArtifactUnitKind::File))
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
                        ) || (matches!(unit.kind, ArtifactUnitKind::File)
                            && hunk.new_start == 0
                            && hunk.new_end == 0)
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
                // A deleted file may not be present in a reconstructed
                // baseline inventory when another provider fails. It still
                // belongs to the baseline ownership graph.
                baseline.is_some(),
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
            let still_present = ctx
                .index
                .artifact_units
                .iter()
                .any(|current| current.identity == unit.identity);
            if from_baseline && owned.is_empty() && !still_present {
                // Removing an unowned semantic artifact cannot introduce an
                // unowned implementation. This permits retiring orphaned
                // transitional code while preserving ownership checks for
                // deleted artifacts that are still part of the specification.
                continue;
            }
            if ctx.config.validation.changed.require_owned_changes
                && owners.is_some_and(|owners| requires_change_ownership(&unit, owners))
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
    // A pre-v1 cutover may remove configuration fields in the same change as
    // the implementation that stopped using them. Normalize only this
    // historical baseline snapshot; the current product parser remains
    // strict and does not accept the retired shape.
    let config = parse_baseline_config(&mitase_config)?;
    let normalized_config = serde_yaml::to_string(&config).context("serialize baseline config")?;
    let tempdir = tempfile::Builder::new()
        .prefix("mitase-baseline-")
        .tempdir()
        .context("create baseline workspace")?;
    let workspace_dir = tempdir.path();
    fs::write(workspace_dir.join("mitase.yaml"), normalized_config)
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

fn parse_baseline_config(source: &str) -> Result<ProjectConfig> {
    let mut value: serde_yaml::Value =
        serde_yaml::from_str(source).context("parse baseline mitase.yaml as YAML")?;
    let Some(root) = value.as_mapping_mut() else {
        bail!("baseline mitase.yaml must be a mapping");
    };
    root.remove(serde_yaml::Value::String("work".into()));
    if let Some(serde_yaml::Value::Mapping(validation)) =
        root.get_mut(serde_yaml::Value::String("validation".into()))
    {
        if let Some(serde_yaml::Value::Mapping(readiness)) =
            validation.get_mut(serde_yaml::Value::String("readiness".into()))
            && let Some(serde_yaml::Value::Mapping(limits)) =
                readiness.get_mut(serde_yaml::Value::String("limits".into()))
        {
            limits.remove(serde_yaml::Value::String("max_targets_per_binding".into()));
            limits.remove(serde_yaml::Value::String("max_slices_per_origin".into()));
        }
        if let Some(serde_yaml::Value::Mapping(changed)) =
            validation.get_mut(serde_yaml::Value::String("changed".into()))
        {
            changed.remove(serde_yaml::Value::String("require_plan".into()));
        }
    }
    serde_yaml::from_value(value).context("parse v1 baseline mitase.yaml")
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

#[derive(Debug, Clone, Copy)]
enum TargetRangeSide {
    Old,
    New,
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
    // exact targets can intentionally be absent until a later implementation
    // change creates them.
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
                Err(e) if target.lifecycle == ArtifactTargetLifecycle::Present && !advisory => {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::process::Command;
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

    fn validate_loaded_workspace(workspace: &SpecWorkspace, index: &SpecIndex) -> ValidationResult {
        validate_without_readiness(&ValidationContext {
            config: &workspace.config,
            workspace,
            index,
            changed_files: None,
            reported_changed_files: None,
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
                "    limits: { max_ownership_scope_units: 64 }\n",
                "  changed:\n",
                "    require_owned_changes: false\n",
                "verification: { runners: {} }\n",
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
            assert!(status.success(), "git {args:?} failed");
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
        let report = evaluate_readiness(&workspace, &index, "readiness-test").expect("readiness");
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
            level: mitase_project_model::ReadinessLevel::Verifiable,
        }];
        let index = workspace.index().unwrap();
        let report = evaluate_readiness(&workspace, &index, "readiness-test").expect("readiness");
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
        copy_dir(&fixture_root(), tempdir.path());
        fs::create_dir_all(tempdir.path().join("src")).unwrap();
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
        let report = evaluate_readiness(&workspace, &index, "readiness-test").expect("readiness");
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
    fn deleted_declared_artifact_uses_baseline_ownership_when_inventory_is_unavailable() {
        let repository = tempdir().expect("temporary repository");
        fs::create_dir_all(repository.path().join("spec")).expect("spec directory");
        fs::write(
            repository.path().join("mitase.yaml"),
            concat!(
                "schema: mitase/config/v1\n",
                "workspace:\n",
                "  spec_roots: [spec]\n",
                "  excludes: []\n",
                "inventory:\n",
                "  active_profile: default\n",
                "  profiles:\n",
                "    - id: default\n",
                "      providers:\n",
                "        declared: { roots: [obsolete.txt] }\n",
                "        rust: {}\n",
                "validation:\n",
                "  preset: standard\n",
                "  readiness:\n",
                "    target: off\n",
                "    limits: { max_ownership_scope_units: 64 }\n",
                "  changed:\n",
                "    require_owned_changes: true\n",
                "verification: { runners: {} }\n",
            ),
        )
        .expect("config");
        fs::write(repository.path().join("obsolete.txt"), "legacy\n").expect("artifact");
        fs::write(
            repository.path().join("spec/feature.yaml"),
            concat!(
                "schema: mitase/spec/v1\n",
                "kind: features\n",
                "namespace: sample\n",
                "category: Sample\n",
                "features:\n",
                "  - id: FEAT-TEST-001\n",
                "    title: Retire legacy artifact\n",
                "    summary: Retire a legacy artifact.\n",
                "    status: implemented\n",
                "    bindings:\n",
                "      - id: maintenance\n",
                "        role: implementation\n",
                "        facet: repository-tooling\n",
                "        responsibility: Own the legacy artifact during retirement.\n",
                "        owns:\n",
                "          - id: legacy-artifact\n",
                "            adapter: declared\n",
                "            path: obsolete.txt\n",
                "            selector: { kind: file }\n",
                "        targets: []\n",
            ),
        )
        .expect("feature spec");
        let baseline = init_git_repo(repository.path());
        fs::remove_file(repository.path().join("obsolete.txt")).expect("delete artifact");

        let workspace = SpecWorkspace::load(repository.path()).expect("current workspace");
        let index = workspace.index().expect("current index");
        let changed_files = vec![ChangedFile {
            status: ChangeStatus::Deleted,
            old_path: Some(RepoPath::new("obsolete.txt").unwrap()),
            new_path: None,
            hunks: vec![ChangedRange {
                old_start: 1,
                old_end: 1,
                new_start: 0,
                new_end: 0,
            }],
        }];
        let result = validate(&ValidationContext {
            config: &workspace.config,
            workspace: &workspace,
            index: &index,
            changed_files: Some(&changed_files),
            reported_changed_files: None,
            preset: workspace.config.validation.preset,
            revision: None,
            change_base_revision: Some(&baseline),
        });
        assert!(
            !result.diagnostics.iter().any(|diagnostic| {
                diagnostic.rule_id == "MITASE-CHANGE-001"
                    && diagnostic.message.contains("no ownership binding")
            }),
            "deleted baseline-owned artifact was rejected: {:?}",
            result.diagnostics
        );
    }

    #[test]
    fn deleted_unowned_semantic_artifact_does_not_require_new_ownership() {
        let repository = tempdir().expect("temporary repository");
        copy_dir(&fixture_root(), repository.path());
        fs::write(
            repository.path().join("api/orphan.rs"),
            "pub fn transitional_orphan() {}\n",
        )
        .expect("orphan artifact");
        let baseline = init_git_repo(repository.path());
        fs::remove_file(repository.path().join("api/orphan.rs")).expect("delete artifact");

        let workspace = SpecWorkspace::load(repository.path()).expect("current workspace");
        let index = workspace.index().expect("current index");
        let changed_files = vec![ChangedFile {
            status: ChangeStatus::Deleted,
            old_path: Some(RepoPath::new("api/orphan.rs").unwrap()),
            new_path: None,
            hunks: vec![ChangedRange {
                old_start: 1,
                old_end: 1,
                new_start: 0,
                new_end: 0,
            }],
        }];
        let result = validate(&ValidationContext {
            config: &workspace.config,
            workspace: &workspace,
            index: &index,
            changed_files: Some(&changed_files),
            reported_changed_files: None,
            preset: workspace.config.validation.preset,
            revision: None,
            change_base_revision: Some(&baseline),
        });
        assert!(
            !result.diagnostics.iter().any(|diagnostic| {
                diagnostic.rule_id == "MITASE-CHANGE-001"
                    && diagnostic.message.contains("no ownership binding")
            }),
            "deleted unowned semantic artifact was rejected: {:?}",
            result.diagnostics
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
