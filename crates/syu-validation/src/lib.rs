#![forbid(unsafe_code)]
use std::collections::BTreeSet;
use syu_diagnostics::{Diagnostic, Severity, ValidationResult};
use syu_planner::plan as canonical_plan;
use syu_project_model::{ProjectConfig, RuleOverride, ValidationPreset};
use syu_spec_model::{
    BindingRole, ItemStatus, LocalAnchorKind, RepoPath, RuleLevel, Selector, SpecAnchor,
    SpecDocument,
};
use syu_work_model::{
    ExecutionSlice, PlanConfidence, WORK_PLAN_SCHEMA, WorkPlan, work_plan_digest,
};
use syu_workspace::{AnchorValue, SpecIndex, SpecWorkspace, resolve_target_with_adapters};

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
            presets: &[ValidationPreset::Standard],
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
    metadata!("SYU-CHANGE-001"),
    metadata!("SYU-CHANGE-002"),
    metadata!("SYU-CHANGE-003"),
    metadata!("SYU-CHANGE-004"),
    metadata!("SYU-CHANGE-005"),
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

pub struct ValidationContext<'a> {
    pub config: &'a ProjectConfig,
    pub workspace: &'a SpecWorkspace,
    pub index: &'a SpecIndex,
    pub changed_files: Option<&'a [ChangedFile]>,
    pub work_plan: Option<&'a WorkPlan>,
    pub selected_slice: Option<&'a ExecutionSlice>,
    pub preset: ValidationPreset,
    pub revision: Option<&'a str>,
}
pub trait ValidationRule {
    fn metadata(&self) -> &'static RuleMetadata;
    fn evaluate(&self, ctx: &ValidationContext<'_>, out: &mut Vec<Diagnostic>);
}

pub fn validate(ctx: &ValidationContext<'_>) -> ValidationResult {
    let mut diagnostics = Vec::new();
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
fn validate_changes(ctx: &ValidationContext<'_>, out: &mut Vec<Diagnostic>) {
    let Some(files) = ctx.changed_files else {
        return;
    };
    for file in files {
        let Some(path) = file.new_path.as_ref().or(file.old_path.as_ref()) else {
            continue;
        };
        if ctx.workspace.path_is_spec(path.as_path())
            || path.as_path() == std::path::Path::new("syu.yaml")
        {
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
            if let Err(e) = resolve_target_with_adapters(
                &ctx.workspace.root,
                target,
                &ctx.config.adapters.enabled,
            ) {
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
    if ctx
        .revision
        .is_some_and(|revision| plan.basis.revision != revision)
        || plan.basis.workspace_fingerprint != ctx.workspace.fingerprint()
    {
        push(
            out,
            "SYU-WORK-009",
            "plan basis revision or workspace fingerprint is stale",
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
    match canonical_plan(
        &plan.request,
        ctx.workspace,
        ctx.index,
        &plan.basis.revision,
    ) {
        Ok(canonical) => {
            if canonical.status != plan.status
                || canonical.slices != plan.slices
                || canonical.diagnostics != plan.diagnostics
                || canonical.canonical_digest != plan.canonical_digest
            {
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
        let actual_bytes: usize = all_targets
            .clone()
            .filter_map(|target| ctx.index.target(&target.reference))
            .filter_map(|declared| {
                resolve_target_with_adapters(
                    &ctx.workspace.root,
                    declared,
                    &ctx.config.adapters.enabled,
                )
                .ok()
            })
            .map(|resolved| resolved.excerpt.len())
            .sum();
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
            validate_slice_scope(files, slice, out);
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

fn validate_slice_scope(files: &[ChangedFile], slice: &ExecutionSlice, out: &mut Vec<Diagnostic>) {
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
        let hunks = if file.hunks.is_empty() {
            vec![ChangedRange {
                old_start: 1,
                old_end: usize::MAX,
                new_start: 1,
                new_end: usize::MAX,
            }]
        } else {
            file.hunks.clone()
        };
        for hunk in hunks {
            let readonly_hit = guarded_targets
                .iter()
                .any(|target| target_hits_hunk(target, path, &hunk));
            let editable_hit = editable_targets
                .iter()
                .any(|target| target_hits_hunk(target, path, &hunk));
            if readonly_hit && !editable_hit {
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

fn target_hits_hunk(
    target: &syu_work_model::PlannedTarget,
    changed_path: &RepoPath,
    hunk: &ChangedRange,
) -> bool {
    if target.resolved_path != changed_path.to_string_lossy() {
        return false;
    }
    if target.resolved_selector.description == "file" {
        return true;
    }
    let start = if hunk.new_start == 0 {
        hunk.old_start
    } else {
        hunk.new_start
    };
    let end = if hunk.new_end == 0 {
        hunk.old_end
    } else {
        hunk.new_end
    };
    line_ranges_overlap(start, end, target.line_start, target.line_end)
}

fn line_ranges_overlap(a_start: usize, a_end: usize, b_start: usize, b_end: usize) -> bool {
    let a_end = if a_end == 0 { a_start } else { a_end };
    let b_end = if b_end == 0 { b_start } else { b_end };
    a_start <= b_end && b_start <= a_end
}
