#![forbid(unsafe_code)]
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use syu_diagnostics::{Diagnostic, Severity, ValidationResult};
use syu_planner::plan as canonical_plan;
use syu_project_model::{ProjectConfig, RuleOverride, ValidationPreset};
use syu_spec_model::{
    BindingRole, BoundTargetRef, ItemStatus, LocalAnchorKind, RepoPath, RuleLevel, Selector,
    SpecAnchor, SpecDocument,
};
use syu_work_model::{
    ExecutionSlice, PlanConfidence, PlanExecution, TargetLifecycle, WORK_PLAN_SCHEMA, WorkPlan,
    work_plan_digest,
};
use syu_workspace::{
    AnchorValue, ResolvedTarget, SpecIndex, SpecWorkspace, resolve_target_with_adapters,
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
    metadata!("SYU-COVERAGE-001"),
    metadata!("SYU-COVERAGE-002"),
    metadata!("SYU-COVERAGE-003"),
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
];

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
    let mut diagnostics = Vec::new();
    validate_rule_overrides(ctx, &mut diagnostics);
    validate_config(ctx, &mut diagnostics);
    validate_document_shapes(ctx, &mut diagnostics);
    validate_graph(ctx, &mut diagnostics);
    validate_targets(ctx, &mut diagnostics);
    validate_contracts(ctx, &mut diagnostics);
    validate_changes(ctx, &mut diagnostics);
    if let Some(plan) = ctx.work_plan {
        validate_plan(ctx, plan, &mut diagnostics);
    }
    diagnostics.retain_mut(|diagnostic| {
        let integrity = is_fixed_error_rule(&diagnostic.rule_id);
        match ctx
            .config
            .validation
            .rules
            .get(&diagnostic.rule_id)
            .copied()
        {
            Some(RuleOverride::Off) if !integrity => return false,
            Some(RuleOverride::Warning) => diagnostic.severity = Severity::Warning,
            Some(RuleOverride::Info) => diagnostic.severity = Severity::Info,
            Some(RuleOverride::Error) => diagnostic.severity = Severity::Error,
            None if !integrity
                && rule_metadata(&diagnostic.rule_id)
                    .is_some_and(|metadata| !metadata.presets.contains(&ctx.preset)) =>
            {
                return false;
            }
            None => {}
            _ => {}
        }
        if ctx.config.validation.deny_warnings && diagnostic.severity == Severity::Warning {
            diagnostic.severity = Severity::Error;
        }
        true
    });
    diagnostics.sort_by(|a, b| {
        (&a.rule_id, &a.primary.path, &a.message).cmp(&(&b.rule_id, &b.primary.path, &b.message))
    });
    ValidationResult { diagnostics }
}

fn validate_rule_overrides(ctx: &ValidationContext<'_>, out: &mut Vec<Diagnostic>) {
    for rule_id in ctx.config.validation.rules.keys() {
        if rule_metadata(rule_id).is_none() {
            push(
                out,
                "SYU-SCHEMA-002",
                format!("unknown validation rule override: {rule_id}"),
                "syu.yaml",
                None,
            );
        } else if matches!(
            ctx.config.validation.rules.get(rule_id),
            Some(RuleOverride::Off | RuleOverride::Warning | RuleOverride::Info)
        ) && is_fixed_error_rule(rule_id)
        {
            push(
                out,
                "SYU-SCHEMA-002",
                format!("validation rule {rule_id} cannot be downgraded or suppressed"),
                "syu.yaml",
                None,
            );
        }
    }
}

fn validate_config(ctx: &ValidationContext<'_>, out: &mut Vec<Diagnostic>) {
    let active_profiles = ctx
        .config
        .profiles
        .active
        .iter()
        .filter(|name| !ctx.config.profiles.custom.contains_key(*name))
        .collect::<Vec<_>>();
    for profile in active_profiles {
        push(
            out,
            "SYU-SCHEMA-002",
            format!("active profile {profile} is not defined"),
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
        if path.as_path() == std::path::Path::new("syu.yaml") {
            continue;
        }
        if ctx.workspace.path_is_spec(path.as_path()) {
            continue;
        }
        if !ctx.workspace.path_is_artifact(path.as_path())
            || ctx.workspace.path_is_excluded(path.as_path())
        {
            continue;
        }
        let rendered = path.to_string_lossy();
        let owners = ctx.index.path_to_targets.get(rendered.as_ref());
        if ctx.config.validation.changed.require_owned_changes && owners.is_none() {
            push(
                out,
                "SYU-CHANGE-001",
                format!("changed path has no Binding target owner: {rendered}"),
                rendered.to_string(),
                None,
            );
            continue;
        }
        for owner in owners.into_iter().flatten() {
            if let Some(binding) = ctx.index.bindings.get(&owner.binding)
                && binding.role == BindingRole::Implementation
                && binding.satisfies.is_empty()
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
    validate_changed_spec_impact(ctx, &changed_spec_documents, files, out);
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
            || config
                .workspace
                .artifact_roots
                .iter()
                .any(|root| relative.starts_with(root.as_path()));
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
                .map(|participant| participant.binding.clone()),
        );
    }
    if let Some(new_contract) = current.contracts.get(contract) {
        bindings.extend(
            new_contract
                .participants
                .iter()
                .map(|participant| participant.binding.clone()),
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
            resolve_target_with_adapters(
                &workspace.root,
                target,
                &workspace.config.adapters.enabled,
            )
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
            resolve_target_with_adapters(
                &current_workspace.root,
                target,
                &current_workspace.config.adapters.enabled,
            )
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
                        .any(|b| b.enforces.contains(anchor) || b.evidences.contains(anchor))
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
                let status = ctx.index.criterion_status.get(anchor).copied();
                if status == Some(ItemStatus::Implemented)
                    && !ctx.index.criteria_to_implementations.contains_key(anchor)
                {
                    push(
                        out,
                        "SYU-COVERAGE-001",
                        "criterion has no implementation binding",
                        &path,
                        Some(anchor.clone()),
                    );
                }
                if status == Some(ItemStatus::Implemented)
                    && !ctx.index.criteria_to_verifications.contains_key(anchor)
                {
                    push(
                        out,
                        "SYU-COVERAGE-002",
                        "criterion has no verification binding",
                        &path,
                        Some(anchor.clone()),
                    );
                }
                if status == Some(ItemStatus::Deprecated)
                    && ctx.index.criteria_to_implementations.contains_key(anchor)
                {
                    push(
                        out,
                        "SYU-COVERAGE-003",
                        "deprecated requirement retains active implementation bindings",
                        &path,
                        Some(anchor.clone()),
                    );
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
                let relation = match binding.role {
                    BindingRole::Implementation => &binding.satisfies,
                    BindingRole::Verification => &binding.verifies,
                    BindingRole::Documentation => &binding.documents,
                    BindingRole::Enforcement => &binding.enforces,
                    BindingRole::Evidence => &binding.evidences,
                    _ => &Vec::new(),
                };
                if matches!(
                    binding.role,
                    BindingRole::Implementation
                        | BindingRole::Verification
                        | BindingRole::Documentation
                        | BindingRole::Enforcement
                        | BindingRole::Evidence
                ) && relation.is_empty()
                {
                    push(
                        out,
                        "SYU-BINDING-001",
                        "binding role requires its canonical relation",
                        &path,
                        Some(anchor.clone()),
                    );
                }
                for target in relation {
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
                let relation_count = [
                    binding.satisfies.len(),
                    binding.verifies.len(),
                    binding.documents.len(),
                    binding.enforces.len(),
                    binding.evidences.len(),
                ]
                .into_iter()
                .filter(|count| *count > 0)
                .count();
                if relation_count > usize::from(!relation.is_empty()) {
                    push(
                        out,
                        "SYU-BINDING-002",
                        "binding contains a relation field incompatible with its role",
                        &path,
                        Some(anchor.clone()),
                    );
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
                    for target in relation {
                        check_kind(ctx, out, target, expected, &path);
                    }
                }
                if binding.role == BindingRole::Generated && binding.generated_from.is_empty() {
                    push(
                        out,
                        "SYU-GENERATED-001",
                        "generated binding has no generated_from target",
                        &path,
                        Some(anchor.clone()),
                    );
                }
                if binding.role == BindingRole::Generated && !binding.generated_from.is_empty() {
                    validate_generated_binding(ctx, anchor, binding, out, &path);
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
                    if !seen_participants.insert((p.binding.clone(), p.role.clone())) {
                        push(
                            out,
                            "SYU-CONTRACT-007",
                            format!(
                                "contract participant is duplicated: {} {}",
                                p.binding, p.role
                            ),
                            &path,
                            Some(anchor.clone()),
                        );
                    }
                    if !ctx.index.bindings.contains_key(&p.binding) {
                        push(
                            out,
                            "SYU-CONTRACT-003",
                            format!("contract participant {} does not exist", p.binding),
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
    binding: &syu_spec_model::ArtifactBinding,
    out: &mut Vec<Diagnostic>,
    path: &str,
) {
    let mut seen = BTreeSet::<BoundTargetRef>::new();
    for generated in &binding.generated_from {
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
        if !seen.insert(generated.clone()) {
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
    let cycle = binding.generated_from.iter().any(|reference| {
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
    let known_facets = ctx
        .config
        .profiles
        .active
        .iter()
        .filter_map(|name| ctx.config.profiles.custom.get(name))
        .flat_map(|profile| profile.facets.keys())
        .collect::<BTreeSet<_>>();
    let facet_rules = ctx
        .config
        .profiles
        .active
        .iter()
        .filter_map(|name| ctx.config.profiles.custom.get(name))
        .flat_map(|profile| profile.facets.iter())
        .collect::<Vec<_>>();
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
                Selector::Symbol { names } => {
                    if names.is_empty() {
                        push(
                            out,
                            "SYU-TARGET-001",
                            "symbol selector must contain at least one name",
                            target.path.to_string_lossy(),
                            Some(anchor.clone()),
                        );
                    }
                    let mut unique = names.clone();
                    unique.sort();
                    unique.dedup();
                    if unique.len() != names.len() {
                        push(
                            out,
                            "SYU-TARGET-001",
                            "symbol selector must not contain duplicate names",
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
                        "rust" | "typescript" | "shell" | "python" | "go"
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
                && !matches!(target.selector, Selector::Symbol { .. } | Selector::File)
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
            if binding.role == BindingRole::Implementation
                && matches!(target.selector, Selector::File)
                && ctx.preset == ValidationPreset::AgentReady
            {
                push(
                    out,
                    "SYU-TARGET-004",
                    "implementation target is file-only and too broad for executable work",
                    target.path.to_string_lossy(),
                    Some(anchor.clone()),
                );
            }
            if let Some((_, rule)) = facet_rules
                .iter()
                .find(|(facet, _)| facet.as_str() == binding.facet)
            {
                let target_path = target.path.to_string_lossy();
                let matches = rule.include.iter().any(|pattern| {
                    pattern
                        .strip_suffix("/**")
                        .map_or(target_path == pattern.as_str(), |prefix| {
                            target_path == prefix || target_path.starts_with(&format!("{prefix}/"))
                        })
                });
                if !matches {
                    push(
                        out,
                        "SYU-FACET-002",
                        format!(
                            "target path {} contradicts facet {}",
                            target.path.display(),
                            binding.facet
                        ),
                        target.path.to_string_lossy(),
                        Some(anchor.clone()),
                    );
                }
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
            if let Err(e) = resolve_target_with_adapters(
                &ctx.workspace.root,
                target,
                &ctx.config.adapters.enabled,
            ) && !allowed_absent_targets.contains(&target_ref)
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
}

fn validate_contracts(ctx: &ValidationContext<'_>, out: &mut Vec<Diagnostic>) {
    let profiles = ctx
        .config
        .profiles
        .active
        .iter()
        .filter_map(|n| ctx.config.profiles.custom.get(n));
    for (anchor, contract) in &ctx.index.contracts {
        let path = ctx
            .index
            .item_paths
            .get(&anchor.item)
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();
        for profile in profiles.clone() {
            for rule in &profile.contract_rules {
                if rule.kind != contract.kind {
                    continue;
                }
                for required in &rule.require_participants {
                    if required.role.trim().is_empty() {
                        push(
                            out,
                            "SYU-CONTRACT-004",
                            "contract requirement role must not be empty",
                            &path,
                            Some(anchor.clone()),
                        );
                        continue;
                    }
                    let count = contract
                        .participants
                        .iter()
                        .filter(|p| {
                            p.role == required.role
                                && ctx
                                    .index
                                    .bindings
                                    .get(&p.binding)
                                    .is_some_and(|b| required.facets.contains(&b.facet))
                        })
                        .count();
                    if count < required.min {
                        push(
                            out,
                            "SYU-CONTRACT-004",
                            format!(
                                "contract requires at least {} {} participant(s)",
                                required.min, required.role
                            ),
                            &path,
                            Some(anchor.clone()),
                        );
                    }
                }
            }
        }
    }
    for (criterion, implementations) in &ctx.index.criteria_to_implementations {
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
                        .any(|participant| &participant.binding == implementation)
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
    if basis_workspace.is_none() {
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
    if let Some(basis) = basis_workspace.as_ref() {
        match canonical_plan(
            &plan.request,
            &basis.workspace,
            &basis.index,
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
            match ctx.index.target(&target.reference).and_then(|declared| {
                resolve_target_with_adapters(
                    &ctx.workspace.root,
                    declared,
                    &ctx.config.adapters.enabled,
                )
                .ok()
            }) {
                Some(resolved)
                    if target.lifecycle == TargetLifecycle::EnsurePresent
                        && resolved.path.to_string_lossy() == target.resolved_path
                        && resolved.description == target.resolved_selector.description
                        && resolved.symbols == target.resolved_selector.symbols
                        && ctx
                            .index
                            .bindings
                            .get(&target.reference.binding)
                            .is_some_and(|binding| {
                                binding.facet == target.facet
                                    && binding.role == target.role
                                    && ctx
                                        .index
                                        .target(&target.reference)
                                        .is_some_and(|declared| declared.adapter == target.adapter)
                            }) =>
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
                        && ctx
                            .index
                            .bindings
                            .get(&target.reference.binding)
                            .is_some_and(|binding| {
                                binding.facet == target.facet
                                    && binding.role == target.role
                                    && ctx
                                        .index
                                        .target(&target.reference)
                                        .is_some_and(|declared| declared.adapter == target.adapter)
                            }) => {}
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
                        && ctx
                            .index
                            .bindings
                            .get(&target.reference.binding)
                            .is_some_and(|binding| {
                                binding.facet == target.facet
                                    && binding.role == target.role
                                    && ctx
                                        .index
                                        .target(&target.reference)
                                        .is_some_and(|declared| declared.adapter == target.adapter)
                            }) => {}
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
                            resolve_target_with_adapters(
                                &ctx.workspace.root,
                                declared,
                                &ctx.config.adapters.enabled,
                            )
                            .ok()
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
                            resolve_target_with_adapters(
                                &ctx.workspace.root,
                                declared,
                                &ctx.config.adapters.enabled,
                            )
                            .ok()
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
                    if !slice.anchors.contains(&participant.binding)
                        && !slice
                            .readonly_context
                            .iter()
                            .any(|target| target.reference.binding == participant.binding)
                    {
                        push(
                            out,
                            "SYU-WORK-008",
                            format!("contract counterpart is absent: {}", participant.binding),
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
    let current = if matches!(
        target.lifecycle,
        TargetLifecycle::Stable | TargetLifecycle::EnsurePresent
    ) {
        ctx.index.target(&target.reference).and_then(|declared| {
            resolve_target_with_adapters(
                &ctx.workspace.root,
                declared,
                &ctx.config.adapters.enabled,
            )
            .ok()
        })
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
                "  artifact_roots: [src]\n",
                "  excludes: []\n",
                "profiles: { active: [], custom: {} }\n",
                "validation:\n",
                "  preset: standard\n",
                "  deny_warnings: false\n",
                "  rules:\n",
                "    SYU-GENERATED-001: off\n",
                "  changed:\n",
                "    require_owned_changes: false\n",
                "work:\n",
                "  slicing:\n",
                "    max_editable_files: 2\n",
                "    max_editable_symbols: 4\n",
                "    max_verification_targets: 2\n",
                "    max_readonly_targets: 2\n",
                "    max_total_bytes: 4096\n",
                "  context:\n",
                "    include_parent_principles: false\n",
                "    include_parent_rules: false\n",
                "adapters: { enabled: [rust] }\n",
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
                "          - { id: generated-file, adapter: rust, path: src/generated.rs, selector: { kind: file } }\n",
                "        generated_from: []\n",
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
    bindings:
      - id: login-test
        role: verification
        facet: verification
        responsibility: Prove invalid credentials create no session.
        targets:
          - { id: case, adapter: rust, path: tests/login.rs, selector: { kind: symbol, names: [invalid_credentials] } }
        verifies: [REQ-AUTH-001#criterion.invalid-credentials]
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
    bindings:
      - id: login-test
        role: verification
        facet: verification
        responsibility: Prove invalid credentials create no session.
        targets:
          - { id: case, adapter: rust, path: tests/login.rs, selector: { kind: symbol, names: [invalid_credentials] } }
        verifies: [REQ-AUTH-001#criterion.invalid-credentials]
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
    fn fixed_error_structural_rules_cannot_be_suppressed() {
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
        assert!(result.diagnostics.iter().any(|diagnostic| {
            diagnostic.rule_id == "SYU-SCHEMA-002"
                && diagnostic
                    .message
                    .contains("cannot be downgraded or suppressed")
        }));
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
