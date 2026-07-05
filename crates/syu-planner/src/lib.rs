#![forbid(unsafe_code)]
use anyhow::{Context, Result, bail};
use globset::{Glob, GlobSet, GlobSetBuilder};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use syu_diagnostics::Diagnostic;
use syu_spec_model::{
    ArtifactBinding, BoundTargetRef, LocalAnchorKind, RepoPath, Selector, SpecAnchor,
};
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
    if !request.seeds.is_empty() && !request.requested_targets.is_empty() {
        return Ok(blocked_plan(
            request,
            workspace,
            revision,
            "SYU-WORK-001",
            "request cannot combine seeds and requested targets",
        ));
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
    let exclude_matcher = compile_exclude_matcher(&request.constraints.exclude_paths)?;
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
                    exclude_matcher.as_ref(),
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
                            exclude_matcher.as_ref(),
                        )?);
                    }
                }
                syu_spec_model::BindingRole::Verification => {
                    for criterion in &binding.verifies {
                        slices.push(build_verification_slice(
                            request,
                            workspace,
                            index,
                            criterion,
                            requested,
                            exclude_matcher.as_ref(),
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
                            exclude_matcher.as_ref(),
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
            execution: PlanExecution::IsolatedSlices,
            request: request.clone(),
            canonical_digest: String::new(),
            status: PlanStatus::Blocked,
            slices,
            diagnostics: vec![d],
        }));
    }
    let plan_id = plan_id(request, revision);
    let plan_basis = basis(workspace, revision);
    for slice in &mut slices {
        if slice.blockers.is_empty()
            && let Err(error) =
                validate_context_pack_budget(&plan_id, &plan_basis, slice, workspace, index)
        {
            slice.blockers.push(Diagnostic::error(
                "SYU-WORK-003",
                format!("context pack exceeds configured budget: {error:#}"),
                "work-plan",
            ));
        }
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
        id: plan_id,
        basis: plan_basis,
        execution: PlanExecution::IsolatedSlices,
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
    exclude_matcher: Option<&GlobSet>,
) -> Result<ExecutionSlice> {
    let binding = index.bindings.get(implementation).expect("indexed binding");
    let add_budget_bytes = request.constraints.max_added_bytes_per_target;
    let add_budget_lines = request.constraints.max_added_lines_per_target;
    let mut blockers = vec![];
    let mut editable = if let Some(target) = exact_target {
        exact_target_plan(
            workspace,
            index,
            target,
            TargetAccessMode::Editable,
            "Requested implementation target.",
            request.operation,
            true,
            add_budget_bytes,
            add_budget_lines,
            exclude_matcher,
            &mut blockers,
        )
    } else {
        targets(
            workspace,
            implementation,
            binding,
            TargetAccessMode::Editable,
            "Primary implementation satisfying the selected criterion.",
            request.operation,
            false,
            add_budget_bytes,
            add_budget_lines,
            exclude_matcher,
            &mut blockers,
        )
    };
    let mut verification = criterion_verification_targets(
        request,
        workspace,
        index,
        criterion,
        None,
        exclude_matcher,
        &mut blockers,
    );
    let (mut readonly, contracts) = contract_readonly_context(
        workspace,
        index,
        implementation,
        exclude_matcher,
        &mut blockers,
    );
    dedup(&mut editable);
    dedup(&mut verification);
    dedup(&mut readonly);
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
    exclude_matcher: Option<&GlobSet>,
) -> Result<ExecutionSlice> {
    let binding = index.bindings.get(documentation).expect("indexed binding");
    let add_budget_bytes = request.constraints.max_added_bytes_per_target;
    let add_budget_lines = request.constraints.max_added_lines_per_target;
    let mut blockers = vec![];
    let mut editable = if let Some(target) = exact_target {
        exact_target_plan(
            workspace,
            index,
            target,
            TargetAccessMode::Editable,
            "Requested documentation target.",
            request.operation,
            true,
            add_budget_bytes,
            add_budget_lines,
            exclude_matcher,
            &mut blockers,
        )
    } else {
        targets(
            workspace,
            documentation,
            binding,
            TargetAccessMode::Editable,
            "Primary documentation target for the selected criterion.",
            request.operation,
            false,
            add_budget_bytes,
            add_budget_lines,
            exclude_matcher,
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
                request.operation,
                false,
                None,
                None,
                exclude_matcher,
                &mut blockers,
            ));
        }
        let (more_readonly, more_contracts) = contract_readonly_context(
            workspace,
            index,
            &implementation,
            exclude_matcher,
            &mut blockers,
        );
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
    exclude_matcher: Option<&GlobSet>,
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
        exclude_matcher,
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
                request.operation,
                false,
                None,
                None,
                exclude_matcher,
                &mut blockers,
            ));
        }
        let (more_readonly, more_contracts) = contract_readonly_context(
            workspace,
            index,
            &implementation,
            exclude_matcher,
            &mut blockers,
        );
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
            for target in editable {
                checks.push(CompletionCheck::TargetAbsent {
                    target: target.reference.clone(),
                });
            }
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
    if request.operation == WorkOperation::Add
        && !editable_scope
            .iter()
            .any(|target| target.lifecycle == TargetLifecycle::EnsurePresent)
    {
        blockers.push(Diagnostic::error(
            "SYU-WORK-001",
            "add request does not introduce any new target",
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
        .map(target_budget_bytes)
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
    exclude_matcher: Option<&GlobSet>,
    blockers: &mut Vec<Diagnostic>,
) -> Vec<PlannedTarget> {
    let mut verification = vec![];
    let add_budget_bytes = request.constraints.max_added_bytes_per_target;
    let add_budget_lines = request.constraints.max_added_lines_per_target;
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
                    TargetAccessMode::RunOnly
                };
                verification.extend(one_target(
                    workspace,
                    &reference,
                    binding,
                    target,
                    TargetPlanOptions {
                        access,
                        reason: "Direct verification of the selected criterion.",
                        operation: request.operation,
                        exact_target: requested == Some(&reference),
                        add_budget_bytes,
                        add_budget_lines,
                    },
                    exclude_matcher,
                    blockers,
                ));
            }
        }
    }
    verification
}

#[allow(clippy::too_many_arguments)]
fn exact_target_plan(
    workspace: &SpecWorkspace,
    index: &SpecIndex,
    requested: &BoundTargetRef,
    access: TargetAccessMode,
    reason: &str,
    operation: WorkOperation,
    exact_target: bool,
    add_budget_bytes: Option<usize>,
    add_budget_lines: Option<usize>,
    exclude_matcher: Option<&GlobSet>,
    blockers: &mut Vec<Diagnostic>,
) -> Vec<PlannedTarget> {
    let Some(binding) = index.bindings.get(&requested.binding) else {
        return vec![];
    };
    let Some(target) = index.target(requested) else {
        return vec![];
    };
    one_target(
        workspace,
        requested,
        binding,
        target,
        TargetPlanOptions {
            access,
            reason,
            operation,
            exact_target,
            add_budget_bytes,
            add_budget_lines,
        },
        exclude_matcher,
        blockers,
    )
}

#[allow(clippy::too_many_arguments)]
fn contract_readonly_context(
    workspace: &SpecWorkspace,
    index: &SpecIndex,
    implementation: &SpecAnchor,
    exclude_matcher: Option<&GlobSet>,
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
                    TargetPlanOptions {
                        access: TargetAccessMode::Readonly,
                        reason: "Contract source constraining this implementation.",
                        operation: WorkOperation::Modify,
                        exact_target: false,
                        add_budget_bytes: None,
                        add_budget_lines: None,
                    },
                    exclude_matcher,
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
                        WorkOperation::Modify,
                        false,
                        None,
                        None,
                        exclude_matcher,
                        blockers,
                    ));
                }
            }
        }
    }
    (readonly, contracts)
}
#[allow(clippy::too_many_arguments)]
fn targets(
    workspace: &SpecWorkspace,
    anchor: &SpecAnchor,
    binding: &ArtifactBinding,
    access: TargetAccessMode,
    reason: &str,
    operation: WorkOperation,
    exact_target: bool,
    add_budget_bytes: Option<usize>,
    add_budget_lines: Option<usize>,
    exclude_matcher: Option<&GlobSet>,
    blockers: &mut Vec<Diagnostic>,
) -> Vec<PlannedTarget> {
    let options = TargetPlanOptions {
        access,
        reason,
        operation,
        exact_target,
        add_budget_bytes,
        add_budget_lines,
    };
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
                options,
                exclude_matcher,
                blockers,
            )
        })
        .collect()
}

#[derive(Clone, Copy)]
struct TargetPlanOptions<'a> {
    access: TargetAccessMode,
    reason: &'a str,
    operation: WorkOperation,
    exact_target: bool,
    add_budget_bytes: Option<usize>,
    add_budget_lines: Option<usize>,
}

fn one_target(
    workspace: &SpecWorkspace,
    reference: &BoundTargetRef,
    binding: &ArtifactBinding,
    target: &syu_spec_model::ArtifactTarget,
    options: TargetPlanOptions<'_>,
    exclude_matcher: Option<&GlobSet>,
    blockers: &mut Vec<Diagnostic>,
) -> Vec<PlannedTarget> {
    if exclude_matcher.is_some_and(|matcher| matcher.is_match(&target.path)) {
        return vec![];
    }
    let resolved =
        resolve_target_with_adapters(&workspace.root, target, &workspace.config.adapters.enabled);
    match (
        options.operation,
        options.access,
        options.exact_target,
        resolved,
    ) {
        (WorkOperation::Add, TargetAccessMode::Editable, true, Ok(_)) => {
            let mut d = Diagnostic::error(
                "SYU-WORK-001",
                format!("add target already exists: {reference}"),
                target.path.to_string_lossy(),
            );
            d.target = Some(reference.clone());
            blockers.push(d);
            vec![]
        }
        (WorkOperation::Add, TargetAccessMode::Editable, _, Err(_)) => {
            let Some(add_budget_bytes) = options.add_budget_bytes else {
                let mut d = Diagnostic::error(
                    "SYU-WORK-001",
                    format!("add target {reference} requires explicit byte budget"),
                    target.path.to_string_lossy(),
                );
                d.target = Some(reference.clone());
                blockers.push(d);
                return vec![];
            };
            let Some(add_budget_lines) = options.add_budget_lines else {
                let mut d = Diagnostic::error(
                    "SYU-WORK-001",
                    format!("add target {reference} requires explicit line budget"),
                    target.path.to_string_lossy(),
                );
                d.target = Some(reference.clone());
                blockers.push(d);
                return vec![];
            };
            vec![declared_target_plan(
                workspace,
                reference,
                binding,
                target,
                options.access,
                options.reason,
                add_budget_bytes,
                add_budget_lines,
            )]
        }
        (WorkOperation::Add, _, _, Ok(r))
        | (WorkOperation::Modify, _, _, Ok(r))
        | (WorkOperation::Remove, _, _, Ok(r))
        | (WorkOperation::Document, _, _, Ok(r))
        | (WorkOperation::Investigate, _, _, Ok(r))
        | (WorkOperation::Refactor, _, _, Ok(r)) => vec![PlannedTarget {
            reference: reference.clone(),
            lifecycle: if matches!(options.operation, WorkOperation::Remove)
                && options.access == TargetAccessMode::Editable
            {
                TargetLifecycle::EnsureAbsent
            } else {
                TargetLifecycle::Stable
            },
            access: options.access,
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
            budget_bytes: r.byte_end.saturating_sub(r.byte_start),
            budget_lines: None,
            reason: options.reason.into(),
        }],
        (WorkOperation::Remove, TargetAccessMode::Editable, _, Err(_)) => {
            let mut d = Diagnostic::error(
                "SYU-WORK-001",
                format!("remove target does not exist: {reference}"),
                target.path.to_string_lossy(),
            );
            d.target = Some(reference.clone());
            blockers.push(d);
            vec![]
        }
        (WorkOperation::Add, _, _, Err(_))
        | (WorkOperation::Modify, _, _, Err(_))
        | (WorkOperation::Remove, _, _, Err(_))
        | (WorkOperation::Document, _, _, Err(_))
        | (WorkOperation::Investigate, _, _, Err(_))
        | (WorkOperation::Refactor, _, _, Err(_)) => {
            let mut d = Diagnostic::error(
                "SYU-TARGET-002",
                format!("target does not resolve: {}", target.path.to_string_lossy()),
                target.path.to_string_lossy(),
            );
            d.target = Some(reference.clone());
            blockers.push(d);
            vec![]
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn declared_target_plan(
    workspace: &SpecWorkspace,
    reference: &BoundTargetRef,
    binding: &ArtifactBinding,
    target: &syu_spec_model::ArtifactTarget,
    access: TargetAccessMode,
    reason: &str,
    add_budget_bytes: usize,
    add_budget_lines: usize,
) -> PlannedTarget {
    let (description, symbols) = declared_selector(&target.selector);
    let fallback = fallback_present_target_snapshot(workspace, target);
    PlannedTarget {
        reference: reference.clone(),
        lifecycle: TargetLifecycle::EnsurePresent,
        access,
        resolved_path: target.path.to_string_lossy().into_owned(),
        resolved_selector: ResolvedSelector {
            description,
            symbols,
        },
        content_hash: fallback.content_hash,
        excerpt_hash: fallback.excerpt_hash,
        adapter: target.adapter.clone(),
        facet: binding.facet.clone(),
        role: binding.role,
        byte_start: fallback.byte_start,
        byte_end: fallback.byte_end,
        line_start: fallback.line_start,
        line_end: fallback.line_end,
        budget_bytes: add_budget_bytes,
        budget_lines: Some(add_budget_lines),
        reason: reason.into(),
    }
}

fn declared_selector(selector: &Selector) -> (String, Vec<String>) {
    match selector {
        Selector::File => ("file".into(), Vec::new()),
        Selector::Symbol { names } => (format!("symbols {}", names.join(", ")), names.clone()),
        Selector::Operation { method, path } => (
            format!("operation {} {path}", method.to_ascii_uppercase()),
            Vec::new(),
        ),
        Selector::Heading { value } => (format!("heading {value}"), Vec::new()),
        Selector::JsonPointer { value } => (format!("json-pointer {value}"), Vec::new()),
        Selector::Marker { value } => (format!("marker {value}"), Vec::new()),
    }
}

struct FallbackPresentTargetSnapshot {
    content_hash: String,
    excerpt_hash: String,
    byte_start: usize,
    byte_end: usize,
    line_start: usize,
    line_end: usize,
}

fn fallback_present_target_snapshot(
    workspace: &SpecWorkspace,
    target: &syu_spec_model::ArtifactTarget,
) -> FallbackPresentTargetSnapshot {
    if let Ok(container) = resolve_target_with_adapters(
        &workspace.root,
        &syu_spec_model::ArtifactTarget {
            id: target.id.clone(),
            adapter: target.adapter.clone(),
            path: target.path.clone(),
            selector: Selector::File,
        },
        &workspace.config.adapters.enabled,
    ) {
        return FallbackPresentTargetSnapshot {
            content_hash: container.content_hash,
            excerpt_hash: container.excerpt_hash,
            byte_start: container.byte_start,
            byte_end: container.byte_end,
            line_start: container.line_start,
            line_end: container.line_end.max(1),
        };
    }
    let empty_hash = hash_bytes(&[]);
    FallbackPresentTargetSnapshot {
        content_hash: empty_hash.clone(),
        excerpt_hash: empty_hash,
        byte_start: 0,
        byte_end: 0,
        line_start: 1,
        line_end: 1,
    }
}

fn hash_bytes(bytes: &[u8]) -> String {
    let mut hash = Sha256::new();
    hash.update(bytes);
    format!("sha256:{:x}", hash.finalize())
}

fn compile_exclude_matcher(patterns: &[String]) -> Result<Option<GlobSet>> {
    if patterns.is_empty() {
        return Ok(None);
    }
    let mut builder = GlobSetBuilder::new();
    for pattern in patterns {
        builder
            .add(Glob::new(pattern).with_context(|| format!("invalid exclude path `{pattern}`"))?);
    }
    Ok(Some(builder.build()?))
}

fn target_budget_bytes(target: &PlannedTarget) -> usize {
    target
        .budget_bytes
        .max(target.byte_end.saturating_sub(target.byte_start))
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
        execution: PlanExecution::IsolatedSlices,
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
    let Some(groups) = split_groups(slice, &workspace.config.work.slicing) else {
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
}

fn split_groups(
    slice: &ExecutionSlice,
    limits: &syu_project_model::SliceLimits,
) -> Option<Vec<SliceGroup>> {
    if !slice_budget_can_shrink_with_editable_split(slice, limits) {
        return None;
    }
    if let Some(groups) = target_groups(&slice.editable_targets) {
        return Some(groups.into_iter().map(SliceGroup::Editable).collect());
    }
    None
}

fn slice_budget_can_shrink_with_editable_split(
    slice: &ExecutionSlice,
    limits: &syu_project_model::SliceLimits,
) -> bool {
    if slice.editable_targets.len() < 2 {
        return false;
    }
    if slice.budget.editable_files > limits.max_editable_files
        || slice.budget.editable_symbols > limits.max_editable_symbols
    {
        return true;
    }
    if slice.budget.verification_targets > limits.max_verification_targets
        || slice.budget.readonly_targets > limits.max_readonly_targets
    {
        return false;
    }
    if slice.budget.total_bytes <= limits.max_total_bytes {
        return false;
    }
    slice
        .editable_targets
        .iter()
        .any(|target| target_budget_bytes(target) > 0)
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
            .map(target_budget_bytes)
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
    let pack = build_context_pack(
        &canonical.id,
        &canonical.basis,
        selected,
        workspace,
        index,
        spec_context,
    )?;
    validate_serialized_context_pack_budget(&pack, workspace)?;
    Ok(pack)
}

fn validate_context_pack_budget(
    plan_id: &str,
    basis: &PlanBasis,
    slice: &ExecutionSlice,
    workspace: &SpecWorkspace,
    index: &SpecIndex,
) -> Result<()> {
    let spec_context = slice_spec_context(slice, index);
    let pack = build_context_pack(plan_id, basis, slice, workspace, index, spec_context)?;
    validate_serialized_context_pack_budget(&pack, workspace)
}

fn slice_spec_context(slice: &ExecutionSlice, index: &SpecIndex) -> Vec<SpecContextEntry> {
    let mut spec_context = Vec::new();
    for anchor in &slice.anchors {
        match index.anchor(anchor) {
            Some(AnchorValue::Principle(v)) => {
                spec_context.push(SpecContextEntry::Statement {
                    anchor: anchor.clone(),
                    text: v.statement.clone(),
                });
            }
            Some(AnchorValue::Rule(v)) => {
                spec_context.push(SpecContextEntry::Statement {
                    anchor: anchor.clone(),
                    text: v.statement.clone(),
                });
            }
            Some(AnchorValue::Criterion(v)) => {
                spec_context.push(SpecContextEntry::Statement {
                    anchor: anchor.clone(),
                    text: v.statement.clone(),
                });
            }
            Some(AnchorValue::Binding(v)) => {
                spec_context.push(SpecContextEntry::Statement {
                    anchor: anchor.clone(),
                    text: v.responsibility.clone(),
                });
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
            }
            None => {}
        }
    }
    spec_context
}

fn build_context_pack(
    plan_id: &str,
    basis: &PlanBasis,
    slice: &ExecutionSlice,
    workspace: &SpecWorkspace,
    index: &SpecIndex,
    spec_context: Vec<SpecContextEntry>,
) -> Result<ContextPack> {
    let mut artifact_context = Vec::new();
    let mut included: Vec<(BoundTargetRef, ContextMode)> = Vec::new();
    for (mode, targets) in [
        (ContextMode::Editable, &slice.editable_targets),
        (ContextMode::Verification, &slice.verification_targets),
        (ContextMode::Readonly, &slice.readonly_context),
    ] {
        for target in targets {
            if included
                .iter()
                .any(|(reference, seen_mode)| reference == &target.reference && *seen_mode == mode)
            {
                continue;
            }
            included.push((target.reference.clone(), mode));
            let resolved = index.target(&target.reference).and_then(|declared| {
                resolve_target_with_adapters(
                    &workspace.root,
                    declared,
                    &workspace.config.adapters.enabled,
                )
                .ok()
            });
            let excerpt = resolved
                .as_ref()
                .map(|resolved| resolved.excerpt.clone())
                .filter(|excerpt| !excerpt.is_empty())
                .or_else(|| match target.lifecycle {
                    TargetLifecycle::EnsurePresent => Some(format!(
                        "Target will be created: {} ({})",
                        target.resolved_path, target.resolved_selector.description
                    )),
                    TargetLifecycle::EnsureAbsent => Some(format!(
                        "Target will be removed: {} ({})",
                        target.resolved_path, target.resolved_selector.description
                    )),
                    TargetLifecycle::Stable => None,
                })
                .context("target resolution failed while exporting context")?;
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
                excerpt,
            });
            if matches!(target.lifecycle, TargetLifecycle::EnsurePresent)
                && resolved.is_none()
                && let Some(container) = resolve_target_with_adapters(
                    &workspace.root,
                    &syu_spec_model::ArtifactTarget {
                        id: target.reference.target_id.clone(),
                        adapter: target.adapter.clone(),
                        path: RepoPath::new(target.resolved_path.clone())
                            .expect("resolved path is a valid repo path"),
                        selector: syu_spec_model::Selector::File,
                    },
                    &workspace.config.adapters.enabled,
                )
                .ok()
            {
                if !included.iter().any(|(reference, seen_mode)| {
                    reference == &target.reference && *seen_mode == ContextMode::Readonly
                }) {
                    included.push((target.reference.clone(), ContextMode::Readonly));
                    artifact_context.push(ArtifactExcerpt {
                        reference: target.reference.clone(),
                        mode: ContextMode::Readonly,
                        access: TargetAccessMode::Readonly,
                        path: container.path.to_string_lossy().into_owned(),
                        selector: ResolvedSelector {
                            description: container.description,
                            symbols: container.symbols,
                        },
                        line_start: container.line_start,
                        line_end: container.line_end,
                        byte_start: container.byte_start,
                        byte_end: container.byte_end,
                        adapter: target.adapter.clone(),
                        facet: target.facet.clone(),
                        role: target.role,
                        content_hash: container.content_hash,
                        excerpt_hash: container.excerpt_hash,
                        reason: "Container context for new target.".into(),
                        excerpt: container.excerpt,
                    });
                }
            }
        }
    }
    Ok(ContextPack {
        schema: CONTEXT_PACK_SCHEMA.into(),
        plan: plan_id.into(),
        slice: slice.id.clone(),
        basis: basis.clone(),
        instructions: ContextInstructions {
            goal: slice.goal.clone(),
            non_goals: slice.non_goals.clone(),
        },
        spec_context,
        artifact_context,
        completion: slice.completion.clone(),
    })
}

fn validate_serialized_context_pack_budget(
    pack: &ContextPack,
    workspace: &SpecWorkspace,
) -> Result<()> {
    let serialized = serde_yaml::to_string(pack)?;
    if serialized.len() > workspace.config.work.slicing.max_total_bytes {
        bail!("context pack exceeds serialized budget");
    }
    Ok(())
}

fn finalize_plan(mut plan: WorkPlan) -> WorkPlan {
    plan.canonical_digest = work_plan_digest(&plan);
    plan
}
