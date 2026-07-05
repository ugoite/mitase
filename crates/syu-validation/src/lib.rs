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
    ExecutionSlice, PlanConfidence, TargetLifecycle, WORK_PLAN_SCHEMA, WorkPlan, work_plan_digest,
};
use syu_workspace::{AnchorValue, SpecIndex, SpecWorkspace, resolve_target_with_adapters};
use tempfile::TempDir;

#[derive(Debug, Clone, Copy)]
pub struct RuleMetadata {
    pub id: &'static str,
    pub title: &'static str,
    pub default_error: bool,
    pub presets: &'static [ValidationPreset],
}
macro_rules! metadata {
    ($id:literal) => {
        RuleMetadata {
            id: $id,
            title: $id,
            default_error: true,
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
            presets: &[ValidationPreset::Strict, ValidationPreset::AgentReady],
        }
    };
}
pub static RULES: &[RuleMetadata] = &[
    metadata!("SYU-SCHEMA-001"),
    metadata!("SYU-SCHEMA-002"),
    metadata!("SYU-ID-001"),
    metadata!("SYU-ID-002"),
    metadata!("SYU-ANCHOR-001"),
    metadata!("SYU-ANCHOR-002"),
    metadata!("SYU-ANCHOR-003"),
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
    metadata!("SYU-BINDING-004"),
    metadata!("SYU-TARGET-001"),
    metadata!("SYU-TARGET-002"),
    metadata!("SYU-TARGET-003"),
    metadata!("SYU-TARGET-004"),
    metadata!("SYU-TARGET-005"),
    metadata!("SYU-FACET-001"),
    metadata!("SYU-FACET-002"),
    metadata!("SYU-CONTRACT-001"),
    metadata!("SYU-CONTRACT-002"),
    metadata!("SYU-CONTRACT-003"),
    metadata!("SYU-CONTRACT-004"),
    metadata!("SYU-CONTRACT-005"),
    metadata!("SYU-CONTRACT-006"),
    metadata!("SYU-CONTRACT-007"),
    metadata!("SYU-DOC-001"),
    metadata!("SYU-DOC-002"),
    metadata!("SYU-GENERATED-001"),
    metadata!("SYU-GENERATED-002"),
    metadata!("SYU-OPERATION-001"),
    strict_metadata!("SYU-CHANGE-001"),
    strict_metadata!("SYU-CHANGE-002"),
    strict_metadata!("SYU-CHANGE-003"),
    strict_metadata!("SYU-CHANGE-004"),
    strict_metadata!("SYU-CHANGE-005"),
    metadata!("SYU-WORK-001"),
    metadata!("SYU-WORK-002"),
    metadata!("SYU-WORK-003"),
    metadata!("SYU-WORK-004"),
    metadata!("SYU-WORK-005"),
    metadata!("SYU-WORK-006"),
    metadata!("SYU-WORK-007"),
    metadata!("SYU-WORK-008"),
    metadata!("SYU-WORK-009"),
    metadata!("SYU-WORK-010"),
    metadata!("SYU-WORK-011"),
    metadata!("SYU-WORK-012"),
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
    validate_document_shapes(ctx, &mut diagnostics);
    validate_graph(ctx, &mut diagnostics);
    validate_targets(ctx, &mut diagnostics);
    validate_contracts(ctx, &mut diagnostics);
    validate_changes(ctx, &mut diagnostics);
    if let Some(plan) = ctx.work_plan {
        validate_plan(ctx, plan, &mut diagnostics);
    }
    diagnostics.retain_mut(|diagnostic| {
        let integrity = diagnostic.rule_id.starts_with("SYU-SCHEMA")
            || diagnostic.rule_id.starts_with("SYU-ANCHOR")
            || diagnostic.rule_id.starts_with("SYU-ID");
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
        }
    }
}

fn rule_metadata(rule_id: &str) -> Option<&'static RuleMetadata> {
    RULES.iter().find(|metadata| metadata.id == rule_id)
}
fn validate_changes(ctx: &ValidationContext<'_>, out: &mut Vec<Diagnostic>) {
    let Some(files) = ctx.changed_files else {
        return;
    };
    let changed_paths = changed_paths(files);
    let changed_spec_documents = files
        .iter()
        .filter_map(|file| file.new_path.as_ref().or(file.old_path.as_ref()))
        .filter(|path| ctx.workspace.path_is_spec(path.as_path()))
        .map(|path| ctx.workspace.root.join(path.as_path()))
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
    validate_changed_spec_impact(ctx, &changed_spec_documents, &changed_paths, out);
}

fn changed_paths(files: &[ChangedFile]) -> BTreeSet<String> {
    files
        .iter()
        .flat_map(|file| file.old_path.iter().chain(file.new_path.iter()))
        .map(|path| path.to_string_lossy().into_owned())
        .collect()
}

fn validate_changed_spec_impact(
    ctx: &ValidationContext<'_>,
    changed_spec_documents: &BTreeSet<std::path::PathBuf>,
    changed_paths: &BTreeSet<String>,
    out: &mut Vec<Diagnostic>,
) {
    if changed_spec_documents.is_empty() {
        return;
    }
    let changed_documents = changed_spec_documents
        .iter()
        .map(|document| normalize_workspace_path(document))
        .collect::<BTreeSet<_>>();
    let baseline = ctx
        .change_base_revision
        .or(ctx.revision)
        .and_then(|revision| load_workspace_at_revision(&ctx.workspace.root, revision));

    for anchor in changed_anchors_for_documents(
        &changed_documents,
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
                ctx.index,
                binding,
                changed_paths,
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
                ctx.index,
                binding,
                changed_paths,
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
        &changed_documents,
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
                    ctx.index,
                    binding,
                    changed_paths,
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
        &changed_documents,
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
            ctx.index,
            &anchor,
            changed_paths,
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
    current: &SpecIndex,
    binding: &SpecAnchor,
    changed_paths: &BTreeSet<String>,
) -> bool {
    binding_targets_for_index(baseline, binding)
        .into_iter()
        .chain(binding_targets_for_index(Some(current), binding))
        .any(|path| changed_paths.contains(&path))
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
    changed_documents: &BTreeSet<PathBuf>,
    current_workspace: &SpecWorkspace,
    current_index: &SpecIndex,
    baseline_workspace: Option<&SpecWorkspace>,
    baseline_index: Option<&SpecIndex>,
    kind: LocalAnchorKind,
) -> BTreeSet<SpecAnchor> {
    let mut anchors = BTreeSet::new();
    collect_changed_anchors(&mut anchors, changed_documents, current_index, kind);
    let _ = current_workspace;
    if let (Some(workspace), Some(index)) = (baseline_workspace, baseline_index) {
        let _ = workspace;
        collect_changed_anchors(&mut anchors, changed_documents, index, kind);
    }
    anchors
}

fn collect_changed_anchors(
    out: &mut BTreeSet<SpecAnchor>,
    changed_documents: &BTreeSet<PathBuf>,
    index: &SpecIndex,
    kind: LocalAnchorKind,
) {
    for (item, path) in &index.item_paths {
        if !changed_documents.contains(&normalize_workspace_path(path)) {
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

fn binding_targets_for_index(index: Option<&SpecIndex>, binding: &SpecAnchor) -> BTreeSet<String> {
    index
        .and_then(|index| index.bindings.get(binding))
        .map(|artifact| {
            artifact
                .targets
                .iter()
                .map(|target| target.path.to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default()
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
                for p in &contract.participants {
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
                let kind = format!("{:?}", contract.kind).to_ascii_lowercase();
                if rule.kind != kind {
                    continue;
                }
                for required in &rule.require_participants {
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
                            }) => {}
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
                syu_work_model::CompletionCheck::TargetAbsent { target }
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
                        .is_some() =>
                {
                    push(
                        out,
                        "SYU-WORK-011",
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
                .any(|target| target_matches_changed_file_path(ctx, target, file));
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
