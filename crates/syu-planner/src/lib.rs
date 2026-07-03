#![forbid(unsafe_code)]
use anyhow::{Context, Result, bail};
use std::collections::{BTreeMap, BTreeSet};
use syu_diagnostics::Diagnostic;
use syu_spec_model::{ArtifactBinding, BoundTargetRef, LocalAnchorKind, SpecAnchor};
use syu_work_model::*;
use syu_workspace::{AnchorValue, SpecIndex, SpecWorkspace, resolve_target_with_adapters};

pub fn plan(
    request: &WorkRequest,
    workspace: &SpecWorkspace,
    index: &SpecIndex,
    revision: &str,
) -> Result<WorkPlan> {
    if request.schema != WORK_REQUEST_SCHEMA {
        bail!("request schema must be {WORK_REQUEST_SCHEMA}");
    }
    if request.seeds.is_empty() && request.requested_targets.is_empty() {
        return Ok(blocked_plan(
            request,
            workspace,
            revision,
            "SYU-WORK-001",
            "an exact seed is required",
        ));
    }
    let mut criteria = BTreeSet::new();
    for requested in &request.requested_targets {
        if index.target(requested).is_none() {
            return Ok(blocked_plan(
                request,
                workspace,
                revision,
                "SYU-WORK-001",
                format!("requested target {requested} does not resolve"),
            ));
        }
    }
    for seed in &request.seeds {
        match seed {
            WorkSeed::Anchor(a) => {
                if index.anchor(a).is_none() {
                    return Ok(blocked_plan(
                        request,
                        workspace,
                        revision,
                        "SYU-WORK-001",
                        format!("seed {a} does not resolve"),
                    ));
                }
                expand_seed(index, a, &mut criteria);
            }
            WorkSeed::Item(item) => {
                if let Some(anchors) = index.item_anchors.get(&item.0) {
                    for a in anchors {
                        if a.kind == LocalAnchorKind::Criterion {
                            criteria.insert(a.clone());
                        }
                    }
                }
            }
        }
    }
    if criteria.is_empty() && request.requested_targets.is_empty() {
        return Ok(blocked_plan(
            request,
            workspace,
            revision,
            "SYU-WORK-001",
            "seed does not select a criterion",
        ));
    }
    let mut slices = Vec::new();
    if request.requested_targets.is_empty() {
        for criterion in criteria {
            for implementation in primary_bindings(request, index, &criterion) {
                if !request.constraints.include_facets.is_empty()
                    && index.bindings.get(&implementation).is_some_and(|binding| {
                        !request.constraints.include_facets.contains(&binding.facet)
                    })
                {
                    continue;
                }
                slices.push(build_implementation_slice(
                    request,
                    workspace,
                    index,
                    &criterion,
                    &implementation,
                    None,
                )?);
            }
        }
    } else {
        for requested in &request.requested_targets {
            let binding = index
                .bindings
                .get(&requested.binding)
                .expect("indexed binding");
            if !request.constraints.include_facets.is_empty()
                && !request.constraints.include_facets.contains(&binding.facet)
            {
                continue;
            }
            match binding.role {
                syu_spec_model::BindingRole::Implementation => {
                    for criterion in &binding.satisfies {
                        slices.push(build_implementation_slice(
                            request,
                            workspace,
                            index,
                            criterion,
                            &requested.binding,
                            Some(requested),
                        )?);
                    }
                }
                syu_spec_model::BindingRole::Verification => {
                    for criterion in &binding.verifies {
                        slices.push(build_verification_slice(
                            request, workspace, index, criterion, requested,
                        )?);
                    }
                }
                syu_spec_model::BindingRole::Documentation => {
                    for criterion in &binding.documents {
                        slices.push(build_documentation_slice(
                            request,
                            workspace,
                            index,
                            criterion,
                            &requested.binding,
                            Some(requested),
                        )?);
                    }
                }
                _ => {
                    return Ok(blocked_plan(
                        request,
                        workspace,
                        revision,
                        "SYU-WORK-001",
                        format!(
                            "requested target {requested} uses unsupported binding role {}",
                            format!("{:?}", binding.role).to_ascii_lowercase()
                        ),
                    ));
                }
            }
        }
    }
    let mut expanded_slices = Vec::new();
    for slice in slices {
        expanded_slices.extend(split_slice_if_needed(request, workspace, &slice)?);
    }
    let mut slices = expanded_slices;
    slices.sort_by(|a, b| a.id.cmp(&b.id));
    if let Some(max) = request.constraints.max_slices
        && slices.len() > max
    {
        let d = Diagnostic::error(
            "SYU-WORK-003",
            format!("{} slices exceed requested maximum {max}", slices.len()),
            "work-request",
        );
        return Ok(finalize_plan(WorkPlan {
            schema: WORK_PLAN_SCHEMA.into(),
            id: plan_id(request, revision),
            basis: basis(workspace, revision),
            request: request.clone(),
            canonical_digest: String::new(),
            status: PlanStatus::Blocked,
            slices,
            diagnostics: vec![d],
        }));
    }
    if slices.is_empty() {
        return Ok(blocked_plan(
            request,
            workspace,
            revision,
            "SYU-WORK-002",
            "request produced no execution slices",
        ));
    }
    let status = if slices.iter().any(|s| !s.blockers.is_empty()) {
        PlanStatus::Blocked
    } else {
        PlanStatus::Ready
    };
    Ok(finalize_plan(WorkPlan {
        schema: WORK_PLAN_SCHEMA.into(),
        id: plan_id(request, revision),
        basis: basis(workspace, revision),
        request: request.clone(),
        canonical_digest: String::new(),
        status,
        slices,
        diagnostics: vec![],
    }))
}

fn expand_seed(index: &SpecIndex, seed: &SpecAnchor, criteria: &mut BTreeSet<SpecAnchor>) {
    match seed.kind {
        LocalAnchorKind::Criterion => {
            criteria.insert(seed.clone());
        }
        LocalAnchorKind::Binding => {
            if let Some(b) = index.bindings.get(seed) {
                criteria.extend(b.satisfies.iter().chain(&b.verifies).cloned());
            }
        }
        LocalAnchorKind::Contract => {
            if let Some(c) = index.contracts.get(seed) {
                for p in &c.participants {
                    if let Some(b) = index.bindings.get(&p.binding) {
                        criteria.extend(b.satisfies.iter().cloned());
                    }
                }
            }
        }
        _ => {}
    }
}

fn primary_bindings(
    request: &WorkRequest,
    index: &SpecIndex,
    criterion: &SpecAnchor,
) -> Vec<SpecAnchor> {
    let mut bindings = match request.operation {
        WorkOperation::Document => index
            .bindings
            .iter()
            .filter(|(_, binding)| {
                binding.role == syu_spec_model::BindingRole::Documentation
                    && binding.documents.contains(criterion)
            })
            .map(|(anchor, _)| anchor.clone())
            .collect::<Vec<_>>(),
        _ => index
            .criteria_to_implementations
            .get(criterion)
            .cloned()
            .unwrap_or_default(),
    };
    bindings.sort();
    bindings.dedup();
    bindings
}

fn build_implementation_slice(
    request: &WorkRequest,
    workspace: &SpecWorkspace,
    index: &SpecIndex,
    criterion: &SpecAnchor,
    implementation: &SpecAnchor,
    exact_target: Option<&BoundTargetRef>,
) -> Result<ExecutionSlice> {
    let binding = index.bindings.get(implementation).expect("indexed binding");
    let mut blockers = vec![];
    let mut editable = if let Some(target) = exact_target {
        exact_target_plan(
            workspace,
            index,
            target,
            TargetAccessMode::Editable,
            "Requested implementation target.",
            &mut blockers,
        )
    } else {
        targets(
            workspace,
            implementation,
            binding,
            TargetAccessMode::Editable,
            "Primary implementation satisfying the selected criterion.",
            &mut blockers,
        )
    };
    let mut verification =
        criterion_verification_targets(request, workspace, index, criterion, None, &mut blockers);
    let (mut readonly, contracts) =
        contract_readonly_context(workspace, index, implementation, &mut blockers);
    dedup(&mut editable);
    dedup(&mut verification);
    dedup(&mut readonly);
    for values in [&mut editable, &mut verification, &mut readonly] {
        values.retain(|target| {
            !request
                .constraints
                .exclude_paths
                .iter()
                .any(|pattern| path_matches(pattern, &target.resolved_path))
        });
    }
    let mut anchors = vec![criterion.clone(), implementation.clone()];
    anchors.extend(contracts.clone());
    finalize_slice(
        request,
        workspace,
        index,
        criterion,
        &format!("{}-{}", criterion.local_id, implementation.local_id),
        format!("{}: {}", request.summary, binding.responsibility),
        anchors,
        editable,
        verification,
        readonly,
        contracts,
        blockers,
    )
}

fn build_documentation_slice(
    request: &WorkRequest,
    workspace: &SpecWorkspace,
    index: &SpecIndex,
    criterion: &SpecAnchor,
    documentation: &SpecAnchor,
    exact_target: Option<&BoundTargetRef>,
) -> Result<ExecutionSlice> {
    let binding = index.bindings.get(documentation).expect("indexed binding");
    let mut blockers = vec![];
    let mut editable = if let Some(target) = exact_target {
        exact_target_plan(
            workspace,
            index,
            target,
            TargetAccessMode::Editable,
            "Requested documentation target.",
            &mut blockers,
        )
    } else {
        targets(
            workspace,
            documentation,
            binding,
            TargetAccessMode::Editable,
            "Primary documentation target for the selected criterion.",
            &mut blockers,
        )
    };
    let verification = Vec::new();
    let mut readonly = Vec::new();
    let mut contracts = Vec::new();
    let implementations = index
        .criteria_to_implementations
        .get(criterion)
        .cloned()
        .unwrap_or_default();
    for implementation in implementations {
        if let Some(other) = index.bindings.get(&implementation) {
            readonly.extend(targets(
                workspace,
                &implementation,
                other,
                TargetAccessMode::Readonly,
                "Implementation context referenced by the selected documentation target.",
                &mut blockers,
            ));
        }
        let (more_readonly, more_contracts) =
            contract_readonly_context(workspace, index, &implementation, &mut blockers);
        readonly.extend(more_readonly);
        contracts.extend(more_contracts);
    }
    dedup(&mut editable);
    dedup(&mut readonly);
    finalize_slice(
        request,
        workspace,
        index,
        criterion,
        &format!("{}-{}", criterion.local_id, documentation.local_id),
        format!("{}: {}", request.summary, binding.responsibility),
        vec![criterion.clone(), documentation.clone()],
        editable,
        verification,
        readonly,
        contracts,
        blockers,
    )
}

fn build_verification_slice(
    request: &WorkRequest,
    workspace: &SpecWorkspace,
    index: &SpecIndex,
    criterion: &SpecAnchor,
    requested: &BoundTargetRef,
) -> Result<ExecutionSlice> {
    let binding = index
        .bindings
        .get(&requested.binding)
        .expect("indexed binding");
    let mut blockers = vec![];
    let editable = Vec::new();
    let mut verification = criterion_verification_targets(
        request,
        workspace,
        index,
        criterion,
        Some(requested),
        &mut blockers,
    );
    let mut readonly = Vec::new();
    let mut anchors = vec![criterion.clone(), requested.binding.clone()];
    let implementations = index
        .criteria_to_implementations
        .get(criterion)
        .cloned()
        .unwrap_or_default();
    let mut contracts = Vec::new();
    for implementation in implementations {
        anchors.push(implementation.clone());
        if let Some(other) = index.bindings.get(&implementation) {
            readonly.extend(targets(
                workspace,
                &implementation,
                other,
                TargetAccessMode::Readonly,
                "Implementation context for the selected verification target.",
                &mut blockers,
            ));
        }
        let (more_readonly, more_contracts) =
            contract_readonly_context(workspace, index, &implementation, &mut blockers);
        readonly.extend(more_readonly);
        contracts.extend(more_contracts);
    }
    dedup(&mut verification);
    dedup(&mut readonly);
    finalize_slice(
        request,
        workspace,
        index,
        criterion,
        &format!("{}-verify-{}", criterion.local_id, requested.target_id),
        format!("{}: {}", request.summary, binding.responsibility),
        anchors,
        editable,
        verification,
        readonly,
        contracts,
        blockers,
    )
}
fn completion_checks(
    request: &WorkRequest,
    editable: &[PlannedTarget],
    verification: &[PlannedTarget],
    contracts: &[SpecAnchor],
) -> Vec<CompletionCheck> {
    let mut checks = verification
        .iter()
        .filter_map(|target| {
            target.resolved_selector.symbols.first().and_then(|symbol| {
                if !symbol
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
                {
                    return None;
                }
                let (program, args) = match target.adapter.as_str() {
                    "rust" => ("cargo", vec!["test".into(), symbol.clone()]),
                    "typescript" => ("npm", vec!["test".into(), "--".into(), symbol.clone()]),
                    "python" => ("pytest", vec!["-k".into(), symbol.clone()]),
                    "go" => (
                        "go",
                        vec!["test".into(), "./...".into(), "-run".into(), symbol.clone()],
                    ),
                    "shell" => ("bash", vec!["-n".into(), target.resolved_path.clone()]),
                    _ => return None,
                };
                Some(CompletionCheck::Command {
                    program: program.into(),
                    args,
                    cwd: None,
                })
            })
        })
        .collect::<Vec<_>>();
    match request.operation {
        WorkOperation::Add => {
            for target in editable {
                checks.push(CompletionCheck::TargetExists {
                    target: target.reference.clone(),
                });
            }
            checks.push(CompletionCheck::DiffWithinScope);
        }
        WorkOperation::Remove => {
            checks.push(CompletionCheck::DiffWithinScope);
        }
        WorkOperation::Refactor => {
            checks.push(CompletionCheck::DiffWithinScope);
            for contract in contracts {
                checks.push(CompletionCheck::ContractConsistent {
                    contract: contract.clone(),
                });
            }
        }
        WorkOperation::Document => {
            checks.push(CompletionCheck::DiffWithinScope);
        }
        WorkOperation::Investigate => {}
        WorkOperation::Modify => {}
    }
    checks.push(CompletionCheck::Validate {
        preset: "agent-ready".into(),
    });
    checks.sort_by(|a, b| format!("{a:?}").cmp(&format!("{b:?}")));
    checks.dedup();
    checks
}

#[allow(clippy::too_many_arguments)]
fn finalize_slice(
    request: &WorkRequest,
    workspace: &SpecWorkspace,
    index: &SpecIndex,
    criterion: &SpecAnchor,
    id: &str,
    goal: String,
    mut anchors: Vec<SpecAnchor>,
    mut editable: Vec<PlannedTarget>,
    mut verification: Vec<PlannedTarget>,
    mut readonly: Vec<PlannedTarget>,
    mut contracts: Vec<SpecAnchor>,
    mut blockers: Vec<Diagnostic>,
) -> Result<ExecutionSlice> {
    for values in [&mut editable, &mut verification, &mut readonly] {
        values.retain(|target| {
            !request
                .constraints
                .exclude_paths
                .iter()
                .any(|pattern| path_matches(pattern, &target.resolved_path))
        });
    }
    if request.operation == WorkOperation::Investigate {
        for target in &mut editable {
            target.access = TargetAccessMode::Readonly;
        }
        for target in &mut verification {
            target.access = TargetAccessMode::Readonly;
        }
        readonly.append(&mut editable);
        readonly.append(&mut verification);
        dedup(&mut readonly);
    }
    anchors.extend(contracts.clone());
    if workspace.config.work.context.include_parent_rules {
        for rule in index.criteria_to_rules.get(criterion).into_iter().flatten() {
            anchors.push(rule.clone());
            if workspace.config.work.context.include_parent_principles {
                anchors.extend(
                    index
                        .rules_to_principles
                        .get(rule)
                        .into_iter()
                        .flatten()
                        .cloned(),
                );
            }
        }
    }
    anchors.sort();
    anchors.dedup();
    contracts.sort();
    contracts.dedup();
    let criterion_value = match index.anchor(criterion) {
        Some(AnchorValue::Criterion(c)) => c,
        _ => unreachable!(),
    };
    let editable_scope = editable
        .iter()
        .chain(verification.iter())
        .filter(|target| target.access == TargetAccessMode::Editable)
        .collect::<Vec<_>>();
    if editable_scope.is_empty() && request.operation != WorkOperation::Investigate {
        blockers.push(Diagnostic::error(
            "SYU-WORK-004",
            "slice has no editable target after exact target selection",
            "work-plan",
        ));
    }
    let editable_files = editable_scope
        .iter()
        .map(|t| t.resolved_path.as_str())
        .collect::<BTreeSet<_>>()
        .len();
    let editable_symbols = editable_scope
        .iter()
        .map(|t| t.resolved_selector.symbols.len())
        .sum();
    let total_bytes = editable
        .iter()
        .chain(&verification)
        .chain(&readonly)
        .map(|t| t.byte_end.saturating_sub(t.byte_start))
        .sum();
    let budget = SliceBudgetUsage {
        editable_files,
        editable_symbols,
        verification_targets: verification.len(),
        readonly_targets: readonly.len(),
        total_bytes,
    };
    let limits = &workspace.config.work.slicing;
    if editable_files > limits.max_editable_files
        || editable_symbols > limits.max_editable_symbols
        || verification.len() > limits.max_verification_targets
        || readonly.len() > limits.max_readonly_targets
        || total_bytes > limits.max_total_bytes
    {
        blockers.push(Diagnostic::error(
            "SYU-WORK-003",
            "slice exceeds configured budget",
            "work-plan",
        ));
    }
    let completion = completion_checks(request, &editable, &verification, &contracts);
    Ok(ExecutionSlice {
        id: id.into(),
        goal,
        anchors,
        editable_targets: editable,
        verification_targets: verification.clone(),
        readonly_context: readonly,
        acceptance: vec![AcceptanceRef {
            anchor: criterion.clone(),
            statement: criterion_value.statement.clone(),
        }],
        contracts,
        non_goals: default_non_goals(request),
        completion,
        budget,
        confidence: PlanConfidence::Exact,
        blockers,
    })
}

fn default_non_goals(request: &WorkRequest) -> Vec<NonGoal> {
    let mut non_goals = vec![NonGoal {
        code: "readonly-siblings".into(),
        statement: "Do not modify readonly contract counterparts or unrelated sibling bindings."
            .into(),
    }];
    match request.operation {
        WorkOperation::Refactor => non_goals.push(NonGoal {
            code: "preserve-behavior".into(),
            statement: "Preserve externally visible behavior and contract guarantees.".into(),
        }),
        WorkOperation::Add => non_goals.push(NonGoal {
            code: "no-regression-removal".into(),
            statement: "Do not remove existing required behaviors while adding the new change."
                .into(),
        }),
        WorkOperation::Remove => non_goals.push(NonGoal {
            code: "remove-only".into(),
            statement: "Do not introduce replacement behavior unless the request explicitly requires it."
                .into(),
        }),
        WorkOperation::Document => non_goals.push(NonGoal {
            code: "no-code-drift".into(),
            statement: "Do not change implementation behavior unless the request explicitly includes executable targets.".into(),
        }),
        WorkOperation::Investigate => non_goals.push(NonGoal {
            code: "no-executable-edits".into(),
            statement: "Do not make executable changes while investigating.".into(),
        }),
        _ => {}
    }
    non_goals
}

fn criterion_verification_targets(
    request: &WorkRequest,
    workspace: &SpecWorkspace,
    index: &SpecIndex,
    criterion: &SpecAnchor,
    requested: Option<&BoundTargetRef>,
    blockers: &mut Vec<Diagnostic>,
) -> Vec<PlannedTarget> {
    let mut verification = vec![];
    for anchor in index
        .criteria_to_verifications
        .get(criterion)
        .into_iter()
        .flatten()
    {
        if let Some(binding) = index.bindings.get(anchor) {
            for target in &binding.targets {
                let reference = BoundTargetRef {
                    binding: anchor.clone(),
                    target_id: target.id.clone(),
                };
                let access = if requested == Some(&reference) {
                    TargetAccessMode::Editable
                } else {
                    match request.operation {
                        WorkOperation::Add | WorkOperation::Remove => TargetAccessMode::Editable,
                        _ => TargetAccessMode::RunOnly,
                    }
                };
                verification.extend(one_target(
                    workspace,
                    &reference,
                    binding,
                    target,
                    access,
                    "Direct verification of the selected criterion.",
                    blockers,
                ));
            }
        }
    }
    verification
}

fn exact_target_plan(
    workspace: &SpecWorkspace,
    index: &SpecIndex,
    requested: &BoundTargetRef,
    access: TargetAccessMode,
    reason: &str,
    blockers: &mut Vec<Diagnostic>,
) -> Vec<PlannedTarget> {
    let Some(binding) = index.bindings.get(&requested.binding) else {
        return vec![];
    };
    let Some(target) = index.target(requested) else {
        return vec![];
    };
    one_target(
        workspace, requested, binding, target, access, reason, blockers,
    )
}

fn contract_readonly_context(
    workspace: &SpecWorkspace,
    index: &SpecIndex,
    implementation: &SpecAnchor,
    blockers: &mut Vec<Diagnostic>,
) -> (Vec<PlannedTarget>, Vec<SpecAnchor>) {
    let mut readonly = vec![];
    let mut contracts = vec![];
    for contract_anchor in index
        .binding_to_contracts
        .get(implementation)
        .into_iter()
        .flatten()
    {
        contracts.push(contract_anchor.clone());
        if let Some(contract) = index.contracts.get(contract_anchor) {
            if let Some(target) = index.target(&contract.source)
                && let Some(source_binding) = index.bindings.get(&contract.source.binding)
            {
                readonly.extend(one_target(
                    workspace,
                    &contract.source,
                    source_binding,
                    target,
                    TargetAccessMode::Readonly,
                    "Contract source constraining this implementation.",
                    blockers,
                ));
            }
            for participant in &contract.participants {
                if &participant.binding != implementation
                    && let Some(other) = index.bindings.get(&participant.binding)
                {
                    readonly.extend(targets(
                        workspace,
                        &participant.binding,
                        other,
                        TargetAccessMode::Readonly,
                        "Contract counterpart; readonly in this slice.",
                        blockers,
                    ));
                }
            }
        }
    }
    (readonly, contracts)
}
fn path_matches(pattern: &str, path: &str) -> bool {
    pattern
        .strip_suffix("/**")
        .map_or(pattern == path, |prefix| {
            path == prefix || path.starts_with(&format!("{prefix}/"))
        })
}
fn targets(
    workspace: &SpecWorkspace,
    anchor: &SpecAnchor,
    binding: &ArtifactBinding,
    access: TargetAccessMode,
    reason: &str,
    blockers: &mut Vec<Diagnostic>,
) -> Vec<PlannedTarget> {
    binding
        .targets
        .iter()
        .flat_map(|t| {
            one_target(
                workspace,
                &BoundTargetRef {
                    binding: anchor.clone(),
                    target_id: t.id.clone(),
                },
                binding,
                t,
                access,
                reason,
                blockers,
            )
        })
        .collect()
}
fn one_target(
    workspace: &SpecWorkspace,
    reference: &BoundTargetRef,
    binding: &ArtifactBinding,
    target: &syu_spec_model::ArtifactTarget,
    access: TargetAccessMode,
    reason: &str,
    blockers: &mut Vec<Diagnostic>,
) -> Vec<PlannedTarget> {
    match resolve_target_with_adapters(&workspace.root, target, &workspace.config.adapters.enabled)
    {
        Ok(r) => vec![PlannedTarget {
            reference: reference.clone(),
            access,
            resolved_path: r.path.to_string_lossy().into_owned(),
            resolved_selector: ResolvedSelector {
                description: r.description,
                symbols: r.symbols,
            },
            content_hash: r.content_hash,
            excerpt_hash: r.excerpt_hash,
            adapter: target.adapter.clone(),
            facet: binding.facet.clone(),
            role: binding.role,
            byte_start: r.byte_start,
            byte_end: r.byte_end,
            line_start: r.line_start,
            line_end: r.line_end,
            reason: reason.into(),
        }],
        Err(e) => {
            let mut d = Diagnostic::error(
                "SYU-TARGET-002",
                e.to_string(),
                target.path.to_string_lossy(),
            );
            d.target = Some(reference.clone());
            blockers.push(d);
            vec![]
        }
    }
}
fn dedup(values: &mut Vec<PlannedTarget>) {
    values.sort_by(|a, b| a.reference.cmp(&b.reference));
    values.dedup_by(|a, b| a.reference == b.reference);
}
fn basis(workspace: &SpecWorkspace, revision: &str) -> PlanBasis {
    PlanBasis {
        revision: revision.into(),
        workspace_fingerprint: workspace.fingerprint(),
    }
}
fn plan_id(r: &WorkRequest, revision: &str) -> String {
    format!(
        "PLAN-{}-{}",
        r.id.trim_start_matches("WORK-"),
        &revision.chars().take(8).collect::<String>()
    )
}
fn blocked_plan(
    request: &WorkRequest,
    workspace: &SpecWorkspace,
    revision: &str,
    rule: &str,
    message: impl Into<String>,
) -> WorkPlan {
    finalize_plan(WorkPlan {
        schema: WORK_PLAN_SCHEMA.into(),
        id: plan_id(request, revision),
        basis: basis(workspace, revision),
        request: request.clone(),
        canonical_digest: String::new(),
        status: PlanStatus::Blocked,
        slices: vec![],
        diagnostics: vec![Diagnostic::error(rule, message, "work-request")],
    })
}

fn split_slice_if_needed(
    request: &WorkRequest,
    workspace: &SpecWorkspace,
    slice: &ExecutionSlice,
) -> Result<Vec<ExecutionSlice>> {
    if !slice_exceeds_limits(slice, &workspace.config.work.slicing) {
        return Ok(vec![slice.clone()]);
    }
    let criterion = slice
        .acceptance
        .first()
        .map(|acceptance| acceptance.anchor.clone())
        .context("slice is missing acceptance criterion")?;
    let Some(groups) = split_groups(slice) else {
        return Ok(vec![slice.clone()]);
    };
    let mut out = Vec::new();
    for (index, group) in groups.into_iter().enumerate() {
        let candidate =
            rebuild_split_slice(request, workspace, &criterion, slice, group, index + 1)?;
        let mut nested = split_slice_if_needed(request, workspace, &candidate)?;
        out.append(&mut nested);
    }
    Ok(out)
}

enum SliceGroup {
    Editable(Vec<PlannedTarget>),
    Verification(Vec<PlannedTarget>),
    Readonly(Vec<PlannedTarget>),
}

fn split_groups(slice: &ExecutionSlice) -> Option<Vec<SliceGroup>> {
    if let Some(groups) = target_groups(&slice.editable_targets) {
        return Some(groups.into_iter().map(SliceGroup::Editable).collect());
    }
    if let Some(groups) = target_groups(&slice.verification_targets) {
        return Some(groups.into_iter().map(SliceGroup::Verification).collect());
    }
    if let Some(groups) = target_groups(&slice.readonly_context) {
        return Some(groups.into_iter().map(SliceGroup::Readonly).collect());
    }
    None
}

fn target_groups(targets: &[PlannedTarget]) -> Option<Vec<Vec<PlannedTarget>>> {
    [
        group_targets(targets, |target| target.facet.clone()),
        group_targets(targets, |target| target.reference.binding.to_string()),
        group_targets(targets, |target| target.resolved_path.clone()),
        group_targets(targets, |target| target.reference.to_string()),
    ]
    .into_iter()
    .find(|groups| groups.len() > 1)
}

fn group_targets(
    targets: &[PlannedTarget],
    key: impl Fn(&PlannedTarget) -> String,
) -> Vec<Vec<PlannedTarget>> {
    let mut grouped = BTreeMap::<String, Vec<PlannedTarget>>::new();
    for target in targets {
        grouped.entry(key(target)).or_default().push(target.clone());
    }
    grouped.into_values().collect()
}

fn rebuild_split_slice(
    request: &WorkRequest,
    workspace: &SpecWorkspace,
    _criterion: &SpecAnchor,
    original: &ExecutionSlice,
    group: SliceGroup,
    part: usize,
) -> Result<ExecutionSlice> {
    let blockers = original
        .blockers
        .iter()
        .filter(|diagnostic| diagnostic.rule_id != "SYU-WORK-003")
        .cloned()
        .collect::<Vec<_>>();
    let (editable_targets, verification_targets, readonly_context) = match group {
        SliceGroup::Editable(editable) => (
            editable,
            original.verification_targets.clone(),
            original.readonly_context.clone(),
        ),
        SliceGroup::Verification(verification) => (
            original.editable_targets.clone(),
            verification,
            original.readonly_context.clone(),
        ),
        SliceGroup::Readonly(readonly) => (
            original.editable_targets.clone(),
            original.verification_targets.clone(),
            readonly,
        ),
    };
    let contracts = original.contracts.clone();
    let completion = completion_checks(
        request,
        &editable_targets,
        &verification_targets,
        &contracts,
    );
    let budget = slice_budget(&editable_targets, &verification_targets, &readonly_context);
    let mut blockers = blockers;
    if editable_targets
        .iter()
        .chain(verification_targets.iter())
        .all(|target| target.access != TargetAccessMode::Editable)
        && request.operation != WorkOperation::Investigate
    {
        blockers.push(Diagnostic::error(
            "SYU-WORK-004",
            "slice has no editable target after exact target selection",
            "work-plan",
        ));
    }
    if slice_budget_exceeds(&budget, &workspace.config.work.slicing) {
        blockers.push(Diagnostic::error(
            "SYU-WORK-003",
            "slice exceeds configured budget",
            "work-plan",
        ));
    }
    Ok(ExecutionSlice {
        id: format!("{}-part{:02}", original.id, part),
        goal: original.goal.clone(),
        anchors: original.anchors.clone(),
        editable_targets,
        verification_targets,
        readonly_context,
        acceptance: original.acceptance.clone(),
        contracts,
        non_goals: default_non_goals(request),
        completion,
        budget,
        confidence: original.confidence,
        blockers,
    })
}

fn slice_budget(
    editable_targets: &[PlannedTarget],
    verification_targets: &[PlannedTarget],
    readonly_context: &[PlannedTarget],
) -> SliceBudgetUsage {
    let editable_scope = editable_targets
        .iter()
        .chain(verification_targets.iter())
        .filter(|target| target.access == TargetAccessMode::Editable)
        .collect::<Vec<_>>();
    SliceBudgetUsage {
        editable_files: editable_scope
            .iter()
            .map(|target| target.resolved_path.as_str())
            .collect::<BTreeSet<_>>()
            .len(),
        editable_symbols: editable_scope
            .iter()
            .map(|target| target.resolved_selector.symbols.len())
            .sum(),
        verification_targets: verification_targets.len(),
        readonly_targets: readonly_context.len(),
        total_bytes: editable_targets
            .iter()
            .chain(verification_targets)
            .chain(readonly_context)
            .map(|target| target.byte_end.saturating_sub(target.byte_start))
            .sum(),
    }
}

fn slice_exceeds_limits(slice: &ExecutionSlice, limits: &syu_project_model::SliceLimits) -> bool {
    slice_budget_exceeds(&slice.budget, limits)
}

fn slice_budget_exceeds(
    budget: &SliceBudgetUsage,
    limits: &syu_project_model::SliceLimits,
) -> bool {
    budget.editable_files > limits.max_editable_files
        || budget.editable_symbols > limits.max_editable_symbols
        || budget.verification_targets > limits.max_verification_targets
        || budget.readonly_targets > limits.max_readonly_targets
        || budget.total_bytes > limits.max_total_bytes
}

pub fn export_context(
    work_plan: &WorkPlan,
    slice_id: &str,
    workspace: &SpecWorkspace,
    index: &SpecIndex,
    current_revision: &str,
) -> Result<ContextPack> {
    if work_plan.basis.revision != current_revision {
        bail!("work plan revision is stale");
    }
    let canonical = plan(&work_plan.request, workspace, index, current_revision)?;
    if canonical.basis != work_plan.basis {
        bail!("work plan basis is stale");
    }
    if canonical.canonical_digest != work_plan.canonical_digest
        || canonical.status != work_plan.status
    {
        bail!("work plan structure does not match the canonical plan");
    }
    if canonical.slices != work_plan.slices || canonical.diagnostics != work_plan.diagnostics {
        bail!("work plan content is tampered");
    }
    if canonical.status != PlanStatus::Ready {
        bail!("only ready work plans can be exported");
    }
    let selected = canonical
        .slices
        .iter()
        .find(|candidate| candidate.id == slice_id)
        .context("selected slice is missing from the canonical plan")?;
    if !selected.blockers.is_empty() {
        bail!("cannot export a blocked slice");
    }
    let mut spec_context = Vec::new();
    for anchor in &selected.anchors {
        match index.anchor(anchor) {
            Some(AnchorValue::Principle(v)) => {
                spec_context.push(SpecContextEntry::Statement {
                    anchor: anchor.clone(),
                    text: v.statement.clone(),
                });
                continue;
            }
            Some(AnchorValue::Rule(v)) => {
                spec_context.push(SpecContextEntry::Statement {
                    anchor: anchor.clone(),
                    text: v.statement.clone(),
                });
                continue;
            }
            Some(AnchorValue::Criterion(v)) => {
                spec_context.push(SpecContextEntry::Statement {
                    anchor: anchor.clone(),
                    text: v.statement.clone(),
                });
                continue;
            }
            Some(AnchorValue::Binding(v)) => {
                spec_context.push(SpecContextEntry::Statement {
                    anchor: anchor.clone(),
                    text: v.responsibility.clone(),
                });
                continue;
            }
            Some(AnchorValue::Contract(v)) => {
                spec_context.push(SpecContextEntry::Contract {
                    anchor: anchor.clone(),
                    kind: v.kind,
                    source: v.source.clone(),
                    guarantees: v.guarantees.clone(),
                    participants: v
                        .participants
                        .iter()
                        .map(|participant| ContractParticipantContext {
                            binding: participant.binding.clone(),
                            role: participant.role.clone(),
                        })
                        .collect(),
                });
                continue;
            }
            None => continue,
        };
    }
    let mut artifact_context = Vec::new();
    let mut included = BTreeSet::new();
    for (mode, targets) in [
        (ContextMode::Editable, &selected.editable_targets),
        (ContextMode::Verification, &selected.verification_targets),
        (ContextMode::Readonly, &selected.readonly_context),
    ] {
        for target in targets {
            if !included.insert(target.reference.clone()) {
                continue;
            }
            let resolved = index
                .target(&target.reference)
                .and_then(|declared| {
                    resolve_target_with_adapters(
                        &workspace.root,
                        declared,
                        &workspace.config.adapters.enabled,
                    )
                    .ok()
                })
                .context("target resolution failed while exporting context")?;
            if resolved.excerpt.is_empty() {
                bail!("target excerpt is empty for {}", target.reference);
            }
            artifact_context.push(ArtifactExcerpt {
                reference: target.reference.clone(),
                mode,
                access: target.access,
                path: target.resolved_path.clone(),
                selector: target.resolved_selector.clone(),
                line_start: target.line_start,
                line_end: target.line_end,
                byte_start: target.byte_start,
                byte_end: target.byte_end,
                adapter: target.adapter.clone(),
                facet: target.facet.clone(),
                role: target.role,
                content_hash: target.content_hash.clone(),
                excerpt_hash: target.excerpt_hash.clone(),
                reason: target.reason.clone(),
                excerpt: resolved.excerpt,
            });
        }
    }
    let pack = ContextPack {
        schema: CONTEXT_PACK_SCHEMA.into(),
        plan: canonical.id.clone(),
        slice: selected.id.clone(),
        basis: canonical.basis.clone(),
        instructions: ContextInstructions {
            goal: selected.goal.clone(),
            non_goals: selected.non_goals.clone(),
        },
        spec_context,
        artifact_context,
        completion: selected.completion.clone(),
    };
    let serialized = serde_yaml::to_string(&pack)?;
    if serialized.len() > workspace.config.work.slicing.max_total_bytes {
        bail!("context pack exceeds serialized budget");
    }
    Ok(pack)
}

fn finalize_plan(mut plan: WorkPlan) -> WorkPlan {
    plan.canonical_digest = work_plan_digest(&plan);
    plan
}
