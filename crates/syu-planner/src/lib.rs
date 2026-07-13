#![forbid(unsafe_code)]
use anyhow::{Context, Result, bail};
use globset::{Glob, GlobSet, GlobSetBuilder};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use syu_diagnostics::Diagnostic;
use syu_spec_model::{
    ArtifactBinding, BoundTargetRef, LocalAnchorKind, RepoPath, Selector, SpecAnchor, TargetClaim,
};
use syu_work_model::*;
use syu_workspace::{
    AnchorValue, SpecIndex, SpecWorkspace, resolve_target_with_adapters, selector_supports_editable,
};

fn enabled_adapters(workspace: &SpecWorkspace) -> Vec<String> {
    workspace
        .config
        .inventory
        .profiles
        .iter()
        .find(|profile| profile.id == workspace.config.inventory.active_profile)
        .map(|profile| profile.providers.keys().cloned().collect())
        .unwrap_or_else(|| {
            vec![
                "rust".into(),
                "javascript".into(),
                "typescript".into(),
                "markdown".into(),
                "openapi".into(),
                "yaml".into(),
                "json".into(),
            ]
        })
}

pub fn plan(
    request: &WorkRequest,
    workspace: &SpecWorkspace,
    index: &SpecIndex,
    revision: &str,
) -> Result<WorkPlan> {
    if request.schema != WORK_REQUEST_SCHEMA {
        bail!("request schema must be {WORK_REQUEST_SCHEMA}");
    }
    if request.constraints.max_added_bytes_per_target == Some(0)
        || request.constraints.max_added_lines_per_target == Some(0)
    {
        return Ok(blocked_plan(
            request,
            workspace,
            revision,
            "SYU-WORK-001",
            "add budgets must be greater than zero",
        ));
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
        let reference = requested.reference();
        let transition = requested.transition(default_transition(request.operation));
        if index.target(reference).is_none() {
            return Ok(blocked_plan(
                request,
                workspace,
                revision,
                "SYU-WORK-001",
                format!("requested target {reference} does not resolve"),
            ));
        }
        if request.operation == WorkOperation::Investigate
            && matches!(
                transition,
                TargetTransition::Add | TargetTransition::Modify | TargetTransition::Remove
            )
        {
            return Ok(blocked_plan(
                request,
                workspace,
                revision,
                "SYU-WORK-001",
                "investigate requests only permit run-only or readonly requested targets",
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
            WorkSeed::ArtifactIdentity { artifact_identity }
            | WorkSeed::ChangedUnit {
                changed_unit: artifact_identity,
            } => {
                let targets = index
                    .target_to_artifact
                    .iter()
                    .filter(|(_, identity)| *identity == artifact_identity)
                    .map(|(target, _)| target)
                    .collect::<Vec<_>>();
                if targets.is_empty() {
                    if let Some(owners) = index.artifact_owners.get(artifact_identity) {
                        for owner in owners {
                            if let Some(binding) = index.bindings.get(&owner.binding) {
                                criteria.extend(binding_criteria(binding));
                            }
                        }
                    }
                    if criteria.is_empty() {
                        return Ok(blocked_plan(
                            request,
                            workspace,
                            revision,
                            "SYU-WORK-001",
                            format!("artifact identity {artifact_identity} does not resolve"),
                        ));
                    }
                }
                for target in targets {
                    if let Some(declared) = index.target(target) {
                        for claim in &declared.claims {
                            if let syu_spec_model::TargetClaim::Satisfies { criterion } = claim {
                                criteria.insert(criterion.clone());
                            }
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
            for implementation in primary_targets(request, index, &criterion) {
                if !request.constraints.include_facets.is_empty()
                    && index
                        .bindings
                        .get(&implementation.binding)
                        .is_some_and(|binding| {
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
                    &implementation.binding,
                    Some(&implementation),
                    target_policy(default_transition(request.operation)),
                    exclude_matcher.as_ref(),
                )?);
            }
        }
    } else {
        let grouped = match group_requested_targets(request, index) {
            Ok(grouped) => grouped,
            Err(error) => {
                return Ok(blocked_plan(
                    request,
                    workspace,
                    revision,
                    "SYU-WORK-001",
                    error.to_string(),
                ));
            }
        };
        for group in grouped {
            if !request.constraints.include_facets.is_empty()
                && group.requested.iter().all(|requested| {
                    index
                        .bindings
                        .get(&requested.reference().binding)
                        .is_some_and(|binding| {
                            !request.constraints.include_facets.contains(&binding.facet)
                        })
                })
            {
                continue;
            }
            match group.criterion {
                Some(criterion) => slices.push(build_requested_criterion_slice(
                    request,
                    workspace,
                    index,
                    &criterion,
                    &group.requested,
                    exclude_matcher.as_ref(),
                )?),
                None => {
                    for requested in group.requested {
                        slices.push(build_requested_target_slice(
                            request,
                            workspace,
                            index,
                            &requested,
                            exclude_matcher.as_ref(),
                        )?);
                    }
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
                criteria.extend(binding_criteria(b));
            }
        }
        LocalAnchorKind::Contract => {
            if let Some(c) = index.contracts.get(seed) {
                for p in &c.participants {
                    if let Some(b) = index.bindings.get(&p.target.binding) {
                        criteria.extend(binding_criteria(b));
                    }
                }
            }
        }
        _ => {}
    }
}

struct RequestedGroup {
    criterion: Option<SpecAnchor>,
    requested: Vec<RequestedTarget>,
}

fn group_requested_targets(
    request: &WorkRequest,
    index: &SpecIndex,
) -> Result<Vec<RequestedGroup>> {
    let mut by_criterion = BTreeMap::<SpecAnchor, Vec<RequestedTarget>>::new();
    let mut standalone = Vec::new();
    let mut normalized = Vec::new();
    let mut seen = BTreeMap::<BoundTargetRef, RequestedTarget>::new();
    for requested in &request.requested_targets {
        if let Some(previous) = seen.get(requested.reference()) {
            if previous.transition == requested.transition
                && previous.criterion == requested.criterion
            {
                continue;
            }
            bail!(
                "conflicting requested target transitions for {}",
                requested.reference()
            );
        }
        seen.insert(requested.reference.clone(), requested.clone());
        normalized.push(requested.clone());
    }
    for requested in &normalized {
        let binding = index
            .bindings
            .get(&requested.reference().binding)
            .ok_or_else(|| anyhow::anyhow!("indexed binding missing for requested target"))?;
        let criterion = match requested.criterion() {
            Some(criterion) => {
                if !matches!(
                    index.anchor(criterion),
                    Some(syu_workspace::AnchorValue::Criterion(_))
                ) {
                    bail!("requested target criterion {criterion} does not resolve to a criterion");
                }
                Some(criterion.clone())
            }
            None => requested_target_criterion(binding)?,
        };
        if let Some(criterion) = criterion {
            by_criterion
                .entry(criterion)
                .or_default()
                .push(requested.clone());
        } else {
            standalone.push(requested.clone());
        }
    }
    let mut groups = by_criterion
        .into_iter()
        .map(|(criterion, requested)| RequestedGroup {
            criterion: Some(criterion),
            requested,
        })
        .collect::<Vec<_>>();
    groups.extend(standalone.into_iter().map(|requested| RequestedGroup {
        criterion: None,
        requested: vec![requested],
    }));
    Ok(groups)
}

fn binding_criteria(binding: &ArtifactBinding) -> Vec<SpecAnchor> {
    binding
        .targets
        .iter()
        .flat_map(|target| target.claims.iter())
        .filter_map(|claim| match claim {
            syu_spec_model::TargetClaim::Satisfies { criterion }
            | syu_spec_model::TargetClaim::Verifies { criterion, .. } => Some(criterion.clone()),
            syu_spec_model::TargetClaim::Documents { anchor }
            | syu_spec_model::TargetClaim::Evidences { anchor } => Some(anchor.clone()),
            syu_spec_model::TargetClaim::Enforces { rule } => Some(rule.clone()),
            syu_spec_model::TargetClaim::GeneratedFrom { .. } => None,
        })
        .collect()
}

fn requested_target_criterion(binding: &ArtifactBinding) -> Result<Option<SpecAnchor>> {
    let mut criteria = binding_criteria(binding);
    criteria.sort();
    criteria.dedup();
    match criteria.len() {
        0 => Ok(None),
        1 => Ok(criteria.into_iter().next()),
        _ => bail!("requested target relates to multiple criteria; specify a criterion"),
    }
}

fn primary_targets(
    request: &WorkRequest,
    index: &SpecIndex,
    criterion: &SpecAnchor,
) -> Vec<BoundTargetRef> {
    let mut targets = match request.operation {
        WorkOperation::Document => index
            .bindings
            .iter()
            .flat_map(|(anchor, binding)| {
                binding
                    .targets
                    .iter()
                    .filter(move |target| {
                        binding.role == syu_spec_model::BindingRole::Documentation
                            && target.claims.iter().any(|claim| {
                                matches!(claim, syu_spec_model::TargetClaim::Documents { anchor: actual } if actual == criterion)
                            })
                    })
                    .map(move |target| BoundTargetRef {
                        binding: anchor.clone(),
                        target_id: target.id.clone(),
                    })
            })
            .collect::<Vec<_>>(),
        _ => index
            .criteria_to_implementation_targets
            .get(criterion)
            .cloned()
            .unwrap_or_default(),
    };
    targets.sort();
    targets.dedup();
    targets
}

fn requested_target_slice_id(prefix: &str, requested: &[RequestedTarget]) -> String {
    let mut parts = requested
        .iter()
        .map(|requested| requested.reference().to_string())
        .collect::<Vec<_>>();
    parts.sort();
    format!("{}-{}", prefix, parts.join("+"))
}

fn default_transition(operation: WorkOperation) -> TargetTransition {
    match operation {
        WorkOperation::Add => TargetTransition::Add,
        WorkOperation::Remove => TargetTransition::Remove,
        _ => TargetTransition::Modify,
    }
}

#[derive(Clone, Copy)]
struct TargetPolicy {
    transition: TargetTransition,
    access: TargetAccessMode,
    lifecycle: TargetLifecycle,
}

fn target_policy(transition: TargetTransition) -> TargetPolicy {
    let access = match transition {
        TargetTransition::Add | TargetTransition::Modify | TargetTransition::Remove => {
            TargetAccessMode::Editable
        }
        TargetTransition::RunOnly => TargetAccessMode::RunOnly,
        TargetTransition::Readonly => TargetAccessMode::Readonly,
    };
    let lifecycle = match transition {
        TargetTransition::Add => TargetLifecycle::EnsurePresent,
        TargetTransition::Remove => TargetLifecycle::EnsureAbsent,
        TargetTransition::Modify | TargetTransition::RunOnly | TargetTransition::Readonly => {
            TargetLifecycle::Stable
        }
    };
    TargetPolicy {
        transition,
        access,
        lifecycle,
    }
}

fn transition_map(
    requested_targets: &[RequestedTarget],
) -> BTreeMap<BoundTargetRef, TargetTransition> {
    let mut map = BTreeMap::new();
    for requested in requested_targets {
        map.entry(requested.reference.clone())
            .or_insert(requested.transition);
    }
    map
}

#[allow(clippy::too_many_arguments)]
fn build_implementation_slice(
    request: &WorkRequest,
    workspace: &SpecWorkspace,
    index: &SpecIndex,
    criterion: &SpecAnchor,
    implementation: &SpecAnchor,
    exact_target: Option<&BoundTargetRef>,
    policy: TargetPolicy,
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
            policy,
            "Requested implementation target.",
            request.operation,
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
            policy,
            "Primary implementation satisfying the selected criterion.",
            request.operation,
            add_budget_bytes,
            add_budget_lines,
            exclude_matcher,
            &mut blockers,
        )
    };
    let editable_refs = editable
        .iter()
        .map(|target| target.reference.clone())
        .collect::<BTreeSet<_>>();
    let mut verification = criterion_verification_targets(
        request,
        workspace,
        index,
        criterion,
        None,
        None,
        target_policy(TargetTransition::RunOnly),
        exclude_matcher,
        &mut blockers,
    )
    .into_iter()
    .filter(|planned| {
        index.target(&planned.reference).is_some_and(|target| {
            target.claims.iter().any(|claim| {
                matches!(
                    claim,
                    TargetClaim::Verifies {
                        criterion: actual,
                        covers,
                        ..
                    } if actual == criterion && covers.iter().any(|covered| editable_refs.contains(covered))
                )
            })
        })
    })
    .collect();
    let (mut readonly, contracts) = contract_readonly_context_for_target(
        workspace,
        index,
        exact_target.expect("implementation target is exact"),
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

#[allow(dead_code)]
#[allow(clippy::too_many_arguments)]
fn build_documentation_slice(
    request: &WorkRequest,
    workspace: &SpecWorkspace,
    index: &SpecIndex,
    criterion: &SpecAnchor,
    documentation: &SpecAnchor,
    exact_target: Option<&BoundTargetRef>,
    policy: TargetPolicy,
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
            policy,
            "Requested documentation target.",
            request.operation,
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
            policy,
            "Primary documentation target for the selected criterion.",
            request.operation,
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
        .criteria_to_implementation_targets
        .get(criterion)
        .cloned()
        .unwrap_or_default();
    for implementation in implementations {
        if let Some(other) = index.bindings.get(&implementation.binding)
            && let Some(target) = index.target(&implementation)
        {
            readonly.extend(one_target(
                workspace,
                &implementation,
                other,
                target,
                TargetPlanOptions {
                    policy: target_policy(TargetTransition::Readonly),
                    reason: "Exact implementation context referenced by the selected documentation target.",
                    operation: WorkOperation::Modify,
                    add_budget_bytes: None,
                    add_budget_lines: None,
                },
                exclude_matcher,
                &mut blockers,
            ));
        }
        let (more_readonly, more_contracts) = contract_readonly_context_for_target(
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

#[allow(clippy::too_many_arguments)]
fn build_requested_target_slice(
    request: &WorkRequest,
    workspace: &SpecWorkspace,
    index: &SpecIndex,
    requested: &RequestedTarget,
    exclude_matcher: Option<&GlobSet>,
) -> Result<ExecutionSlice> {
    let reference = requested.reference();
    let binding = index
        .bindings
        .get(&reference.binding)
        .expect("indexed binding");
    let policy = target_policy(requested.transition(default_transition(request.operation)));
    let mut blockers = vec![];
    let planned = exact_target_plan(
        workspace,
        index,
        reference,
        policy,
        "Requested target.",
        request.operation,
        request.constraints.max_added_bytes_per_target,
        request.constraints.max_added_lines_per_target,
        exclude_matcher,
        &mut blockers,
    );
    let mut editable = Vec::new();
    let mut verification = Vec::new();
    let mut readonly = Vec::new();
    for target in planned {
        match target.access {
            TargetAccessMode::Editable => editable.push(target),
            TargetAccessMode::RunOnly => verification.push(target),
            TargetAccessMode::Readonly => readonly.push(target),
        }
    }
    let mut contracts = Vec::new();
    let mut anchors = vec![reference.binding.clone()];
    if let Some(criterion) = match requested_target_criterion(binding) {
        Ok(criterion) => criterion,
        Err(error) => {
            blockers.push(Diagnostic::error(
                "SYU-WORK-001",
                error.to_string(),
                "work-plan",
            ));
            None
        }
    } {
        anchors.push(criterion.clone());
        verification.extend(criterion_verification_targets(
            request,
            workspace,
            index,
            &criterion,
            Some(requested),
            None,
            policy,
            exclude_matcher,
            &mut blockers,
        ));
        let (more_readonly, more_contracts) = contract_readonly_context_for_target(
            workspace,
            index,
            reference,
            exclude_matcher,
            &mut blockers,
        );
        readonly.extend(more_readonly);
        contracts.extend(more_contracts);
    }
    finalize_requested_slice(
        request,
        workspace,
        index,
        &requested_target_slice_id("requested", std::slice::from_ref(requested)),
        format!("{}: {}", request.summary, binding.responsibility),
        anchors,
        editable,
        verification,
        readonly,
        contracts,
        blockers,
        None,
    )
}

#[allow(clippy::too_many_arguments)]
fn build_requested_criterion_slice(
    request: &WorkRequest,
    workspace: &SpecWorkspace,
    index: &SpecIndex,
    criterion: &SpecAnchor,
    requested_targets: &[RequestedTarget],
    exclude_matcher: Option<&GlobSet>,
) -> Result<ExecutionSlice> {
    let mut editable = Vec::new();
    let mut verification = Vec::new();
    let mut readonly = Vec::new();
    let mut contracts = Vec::new();
    let mut blockers = vec![];
    let mut anchors = vec![criterion.clone()];
    let mut goal = request.summary.clone();
    let requested_transitions = transition_map(requested_targets);
    for requested in requested_targets {
        let reference = requested.reference();
        let binding = index
            .bindings
            .get(&reference.binding)
            .expect("indexed binding");
        goal = format!("{}: {}", request.summary, binding.responsibility);
        anchors.push(reference.binding.clone());
        let policy = target_policy(requested.transition(default_transition(request.operation)));
        let planned_criterion = match requested.criterion().cloned() {
            Some(criterion) => Some(criterion),
            None => match requested_target_criterion(binding) {
                Ok(criterion) => criterion,
                Err(error) => {
                    blockers.push(Diagnostic::error(
                        "SYU-WORK-001",
                        error.to_string(),
                        "work-plan",
                    ));
                    None
                }
            },
        };
        let planned = exact_target_plan(
            workspace,
            index,
            reference,
            policy,
            "Requested target.",
            request.operation,
            request.constraints.max_added_bytes_per_target,
            request.constraints.max_added_lines_per_target,
            exclude_matcher,
            &mut blockers,
        );
        for target in planned {
            match target.access {
                TargetAccessMode::Editable => editable.push(target),
                TargetAccessMode::RunOnly => verification.push(target),
                TargetAccessMode::Readonly => readonly.push(target),
            }
        }
        let (more_readonly, more_contracts) = contract_readonly_context_for_target(
            workspace,
            index,
            reference,
            exclude_matcher,
            &mut blockers,
        );
        readonly.extend(more_readonly);
        contracts.extend(more_contracts);
        if let Some(criterion_for_target) = planned_criterion {
            anchors.push(criterion_for_target);
        }
    }
    verification.extend(criterion_verification_targets(
        request,
        workspace,
        index,
        criterion,
        None,
        Some(&requested_transitions),
        target_policy(TargetTransition::RunOnly),
        exclude_matcher,
        &mut blockers,
    ));
    let implementations = index
        .criteria_to_implementation_targets
        .get(criterion)
        .cloned()
        .unwrap_or_default();
    for implementation in implementations {
        if requested_targets
            .iter()
            .any(|requested| requested.reference() == &implementation)
        {
            continue;
        }
        anchors.push(implementation.binding.clone());
        if let Some(other) = index.bindings.get(&implementation.binding)
            && let Some(target) = index.target(&implementation)
        {
            readonly.extend(one_target(
                workspace,
                &implementation,
                other,
                target,
                TargetPlanOptions {
                    policy: target_policy(TargetTransition::Readonly),
                    reason: "Exact implementation context referenced by the selected criterion.",
                    operation: WorkOperation::Modify,
                    add_budget_bytes: None,
                    add_budget_lines: None,
                },
                exclude_matcher,
                &mut blockers,
            ));
        }
        let (more_readonly, more_contracts) = contract_readonly_context_for_target(
            workspace,
            index,
            &implementation,
            exclude_matcher,
            &mut blockers,
        );
        readonly.extend(more_readonly);
        contracts.extend(more_contracts);
    }
    anchors.extend(contracts.clone());
    let slice_id = requested_targets
        .iter()
        .find(|requested| {
            requested.transition(default_transition(request.operation)) == TargetTransition::RunOnly
        })
        .map(|requested| {
            format!(
                "{}-verify-{}",
                criterion.local_id,
                requested.reference().target_id
            )
        })
        .unwrap_or_else(|| requested_target_slice_id("criterion", requested_targets));
    finalize_requested_slice(
        request,
        workspace,
        index,
        &slice_id,
        goal,
        anchors,
        editable,
        verification,
        readonly,
        contracts,
        blockers,
        Some(criterion.clone()),
    )
}

#[allow(clippy::too_many_arguments)]
fn finalize_requested_slice(
    request: &WorkRequest,
    workspace: &SpecWorkspace,
    index: &SpecIndex,
    id: &str,
    goal: String,
    mut anchors: Vec<SpecAnchor>,
    mut editable: Vec<PlannedTarget>,
    mut verification: Vec<PlannedTarget>,
    mut readonly: Vec<PlannedTarget>,
    mut contracts: Vec<SpecAnchor>,
    mut blockers: Vec<Diagnostic>,
    criterion: Option<SpecAnchor>,
) -> Result<ExecutionSlice> {
    validate_target_access_uniqueness(
        &mut blockers,
        editable.as_slice(),
        verification.as_slice(),
        readonly.as_slice(),
    );
    dedup(&mut editable);
    dedup(&mut verification);
    dedup(&mut readonly);
    anchors.sort();
    anchors.dedup();
    contracts.sort();
    contracts.dedup();
    let completion = completion_checks(request, &editable, &verification, &contracts);
    let budget = slice_budget(&editable, &verification, &readonly);
    if editable.is_empty()
        && verification.is_empty()
        && readonly.is_empty()
        && request.operation != WorkOperation::Investigate
    {
        blockers.push(Diagnostic::error(
            "SYU-WORK-004",
            "request produced no executable or contextual targets",
            "work-plan",
        ));
    }
    let limits = &workspace.config.work.slicing;
    if slice_budget_exceeds(&budget, limits) {
        blockers.push(Diagnostic::error(
            "SYU-WORK-003",
            "slice exceeds configured budget",
            "work-plan",
        ));
    }
    Ok(ExecutionSlice {
        id: id.into(),
        goal,
        anchors,
        editable_targets: editable,
        verification_targets: verification,
        readonly_context: readonly,
        acceptance: criterion
            .and_then(|criterion| match index.anchor(&criterion) {
                Some(AnchorValue::Criterion(value)) => Some(AcceptanceRef {
                    anchor: criterion,
                    statement: value.statement.clone(),
                }),
                _ => None,
            })
            .into_iter()
            .collect(),
        contracts,
        non_goals: default_non_goals(request),
        completion,
        budget,
        confidence: PlanConfidence::Exact,
        blockers,
    })
}

#[allow(dead_code)]
fn build_verification_slice(
    request: &WorkRequest,
    workspace: &SpecWorkspace,
    index: &SpecIndex,
    criterion: &SpecAnchor,
    requested: &RequestedTarget,
    exclude_matcher: Option<&GlobSet>,
) -> Result<ExecutionSlice> {
    let requested_ref = requested.reference();
    let binding = index
        .bindings
        .get(&requested_ref.binding)
        .expect("indexed binding");
    let mut blockers = vec![];
    let editable = Vec::new();
    let mut verification = criterion_verification_targets(
        request,
        workspace,
        index,
        criterion,
        Some(requested),
        None,
        target_policy(requested.transition(default_transition(request.operation))),
        exclude_matcher,
        &mut blockers,
    );
    let mut readonly = Vec::new();
    let mut anchors = vec![criterion.clone(), requested_ref.binding.clone()];
    let implementations = index
        .criteria_to_implementation_targets
        .get(criterion)
        .cloned()
        .unwrap_or_default();
    let mut contracts = Vec::new();
    for implementation in implementations {
        anchors.push(implementation.binding.clone());
        if let Some(other) = index.bindings.get(&implementation.binding)
            && let Some(target) = index.target(&implementation)
        {
            readonly.extend(one_target(
                workspace,
                &implementation,
                other,
                target,
                TargetPlanOptions {
                    policy: target_policy(TargetTransition::Readonly),
                    reason: "Exact implementation context for the selected verification target.",
                    operation: WorkOperation::Modify,
                    add_budget_bytes: None,
                    add_budget_lines: None,
                },
                exclude_matcher,
                &mut blockers,
            ));
        }
        let (more_readonly, more_contracts) = contract_readonly_context_for_target(
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
        &format!("{}-verify-{}", criterion.local_id, requested_ref.target_id),
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
    let mut checks = Vec::new();
    for target in editable.iter().chain(verification.iter()) {
        match target.transition {
            TargetTransition::Add => checks.push(CompletionCheck::TargetExists {
                target: target.reference.clone(),
            }),
            TargetTransition::Remove => checks.push(CompletionCheck::TargetAbsent {
                target: target.reference.clone(),
            }),
            TargetTransition::RunOnly => {
                if let Some(symbol) = target.resolved_selector.symbols.first()
                    && symbol
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
                {
                    let (program, args) = match target.adapter.as_str() {
                        "rust" => ("cargo", vec!["test".into(), symbol.clone()]),
                        "typescript" => ("npm", vec!["test".into(), "--".into(), symbol.clone()]),
                        "python" => ("pytest", vec!["-k".into(), symbol.clone()]),
                        "go" => (
                            "go",
                            vec!["test".into(), "./...".into(), "-run".into(), symbol.clone()],
                        ),
                        "shell" => ("bash", vec!["-n".into(), target.resolved_path.clone()]),
                        _ => continue,
                    };
                    checks.push(CompletionCheck::Command {
                        program: program.into(),
                        args,
                        cwd: None,
                    });
                }
            }
            TargetTransition::Readonly | TargetTransition::Modify => {}
        }
        if target.access == TargetAccessMode::Editable
            && !matches!(
                target.transition,
                TargetTransition::Add | TargetTransition::Remove
            )
        {
            checks.push(CompletionCheck::DiffWithinScope);
        }
    }
    if editable.iter().any(|target| {
        matches!(
            target.transition,
            TargetTransition::Add | TargetTransition::Remove
        )
    }) || request.operation == WorkOperation::Document
        || request.operation == WorkOperation::Refactor
    {
        checks.push(CompletionCheck::DiffWithinScope);
    }
    if request.operation == WorkOperation::Refactor || !contracts.is_empty() {
        for contract in contracts {
            checks.push(CompletionCheck::ContractConsistent {
                contract: contract.clone(),
            });
        }
        checks.push(CompletionCheck::DiffWithinScope);
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
    validate_target_access_uniqueness(
        &mut blockers,
        editable.as_slice(),
        verification.as_slice(),
        readonly.as_slice(),
    );
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
    if true {
        for rule in index.criteria_to_rules.get(criterion).into_iter().flatten() {
            anchors.push(rule.clone());
            if true {
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

#[allow(clippy::too_many_arguments)]
fn criterion_verification_targets(
    request: &WorkRequest,
    workspace: &SpecWorkspace,
    index: &SpecIndex,
    criterion: &SpecAnchor,
    requested: Option<&RequestedTarget>,
    requested_transitions: Option<&BTreeMap<BoundTargetRef, TargetTransition>>,
    requested_policy: TargetPolicy,
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
                if requested_transitions
                    .and_then(|map| map.get(&reference))
                    .is_some()
                {
                    continue;
                }
                let requested_ref = requested.map(|value| value.reference());
                let exact_target = requested_ref == Some(&reference);
                let policy = if exact_target {
                    requested_policy
                } else {
                    target_policy(TargetTransition::RunOnly)
                };
                verification.extend(one_target(
                    workspace,
                    &reference,
                    binding,
                    target,
                    TargetPlanOptions {
                        policy,
                        reason: "Direct verification of the selected criterion.",
                        operation: request.operation,
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
    policy: TargetPolicy,
    reason: &str,
    operation: WorkOperation,
    add_budget_bytes: Option<usize>,
    add_budget_lines: Option<usize>,
    exclude_matcher: Option<&GlobSet>,
    blockers: &mut Vec<Diagnostic>,
) -> Vec<PlannedTarget> {
    let Some(binding) = index.bindings.get(&requested.binding) else {
        return vec![];
    };
    let Some(target) = binding
        .targets
        .iter()
        .find(|candidate| candidate.id == requested.target_id)
    else {
        return vec![];
    };
    one_target(
        workspace,
        requested,
        binding,
        target,
        TargetPlanOptions {
            policy,
            reason,
            operation,
            add_budget_bytes,
            add_budget_lines,
        },
        exclude_matcher,
        blockers,
    )
}

#[allow(clippy::too_many_arguments)]
fn contract_readonly_context_for_target(
    workspace: &SpecWorkspace,
    index: &SpecIndex,
    implementation: &BoundTargetRef,
    exclude_matcher: Option<&GlobSet>,
    blockers: &mut Vec<Diagnostic>,
) -> (Vec<PlannedTarget>, Vec<SpecAnchor>) {
    let mut readonly = Vec::new();
    let mut contracts = Vec::new();
    for contract_anchor in index
        .contracts_by_target
        .get(implementation)
        .cloned()
        .unwrap_or_default()
    {
        contracts.push(contract_anchor.clone());
        let Some(contract) = index.contracts.get(&contract_anchor) else {
            continue;
        };
        if let Some(target) = index.target(&contract.source)
            && let Some(binding) = index.bindings.get(&contract.source.binding)
        {
            readonly.extend(one_target(
                workspace,
                &contract.source,
                binding,
                target,
                TargetPlanOptions {
                    policy: target_policy(TargetTransition::Readonly),
                    reason: "Contract source constraining this implementation target.",
                    operation: WorkOperation::Modify,
                    add_budget_bytes: None,
                    add_budget_lines: None,
                },
                exclude_matcher,
                blockers,
            ));
        }
        for participant in &contract.participants {
            if participant.target == *implementation {
                continue;
            }
            if let Some(binding) = index.bindings.get(&participant.target.binding)
                && let Some(target) = index.target(&participant.target)
            {
                readonly.extend(one_target(
                    workspace,
                    &participant.target,
                    binding,
                    target,
                    TargetPlanOptions {
                        policy: target_policy(TargetTransition::Readonly),
                        reason: "Contract counterpart; readonly in this slice.",
                        operation: WorkOperation::Modify,
                        add_budget_bytes: None,
                        add_budget_lines: None,
                    },
                    exclude_matcher,
                    blockers,
                ));
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
    policy: TargetPolicy,
    reason: &str,
    operation: WorkOperation,
    add_budget_bytes: Option<usize>,
    add_budget_lines: Option<usize>,
    exclude_matcher: Option<&GlobSet>,
    blockers: &mut Vec<Diagnostic>,
) -> Vec<PlannedTarget> {
    let options = TargetPlanOptions {
        policy,
        reason,
        operation,
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
    policy: TargetPolicy,
    reason: &'a str,
    #[allow(dead_code)]
    operation: WorkOperation,
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
    if matches!(options.policy.access, TargetAccessMode::Editable)
        && !selector_supports_editable(&target.selector)
    {
        let mut d = Diagnostic::error(
            "SYU-TARGET-004",
            format!(
                "editable target requires an exact selector: {}",
                target.path.display()
            ),
            target.path.to_string_lossy(),
        );
        d.target = Some(reference.clone());
        blockers.push(d);
        return vec![];
    }
    let resolved =
        resolve_target_with_adapters(&workspace.root, target, &enabled_adapters(workspace));
    match resolved {
        Ok(r) => {
            if matches!(options.policy.transition, TargetTransition::Add) {
                let mut d = Diagnostic::error(
                    "SYU-WORK-001",
                    format!("add target already exists: {reference}"),
                    target.path.to_string_lossy(),
                );
                d.target = Some(reference.clone());
                blockers.push(d);
                return vec![];
            }
            vec![PlannedTarget {
                reference: reference.clone(),
                transition: options.policy.transition,
                lifecycle: options.policy.lifecycle,
                access: options.policy.access,
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
            }]
        }
        Err(_) => match options.policy.transition {
            TargetTransition::Add => {
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
                    reference,
                    binding,
                    target,
                    options.policy,
                    options.reason,
                    add_budget_bytes,
                    add_budget_lines,
                )]
            }
            TargetTransition::Remove => {
                let mut d = Diagnostic::error(
                    "SYU-WORK-001",
                    format!("remove target does not exist: {reference}"),
                    target.path.to_string_lossy(),
                );
                d.target = Some(reference.clone());
                blockers.push(d);
                vec![]
            }
            TargetTransition::Modify | TargetTransition::RunOnly | TargetTransition::Readonly => {
                let mut d = Diagnostic::error(
                    "SYU-TARGET-002",
                    format!("target does not resolve: {}", target.path.to_string_lossy()),
                    target.path.to_string_lossy(),
                );
                d.target = Some(reference.clone());
                blockers.push(d);
                vec![]
            }
        },
    }
}

fn declared_target_plan(
    reference: &BoundTargetRef,
    binding: &ArtifactBinding,
    target: &syu_spec_model::ArtifactTarget,
    policy: TargetPolicy,
    reason: &str,
    add_budget_bytes: usize,
    add_budget_lines: usize,
) -> PlannedTarget {
    PlannedTarget {
        reference: reference.clone(),
        transition: policy.transition,
        lifecycle: policy.lifecycle,
        access: policy.access,
        resolved_path: target.path.to_string_lossy().into_owned(),
        resolved_selector: ResolvedSelector {
            description: declared_selector(&target.selector).0,
            symbols: declared_selector(&target.selector).1,
        },
        content_hash: String::new(),
        excerpt_hash: String::new(),
        adapter: target.adapter.clone(),
        facet: binding.facet.clone(),
        role: binding.role,
        byte_start: 0,
        byte_end: 0,
        line_start: 0,
        line_end: 0,
        budget_bytes: add_budget_bytes,
        budget_lines: Some(add_budget_lines),
        reason: reason.into(),
    }
}

fn declared_selector(selector: &Selector) -> (String, Vec<String>) {
    match selector {
        Selector::Symbol { name } => (format!("symbol {name}"), vec![name.clone()]),
        Selector::Operation { method, path } => (
            format!("operation {} {path}", method.to_ascii_uppercase()),
            Vec::new(),
        ),
        Selector::Heading { value } => (format!("heading {value}"), Vec::new()),
        Selector::JsonPointer { value } => (format!("json-pointer {value}"), Vec::new()),
        Selector::Marker { value } => (format!("marker {value}"), Vec::new()),
    }
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

fn validate_target_access_uniqueness(
    blockers: &mut Vec<Diagnostic>,
    editable: &[PlannedTarget],
    verification: &[PlannedTarget],
    readonly: &[PlannedTarget],
) {
    let mut seen = BTreeMap::<BoundTargetRef, TargetAccessMode>::new();
    for target in editable.iter().chain(verification).chain(readonly) {
        if let Some(access) = seen.insert(target.reference.clone(), target.access)
            && access != target.access
        {
            let mut d = Diagnostic::error(
                "SYU-WORK-001",
                format!(
                    "requested target appears with multiple access modes: {}",
                    target.reference
                ),
                "work-plan",
            );
            d.target = Some(target.reference.clone());
            blockers.push(d);
        }
    }
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
        revision.chars().take(8).collect::<String>()
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
    if slice.acceptance.len() == 1 && slice.editable_targets.len() > 1 {
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
                            target: participant.target.clone(),
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
                            target: participant.target.clone(),
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
    let mut included_supports: BTreeSet<String> = BTreeSet::new();
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
                    &enabled_adapters(workspace),
                )
                .ok()
            });
            match resolved {
                Some(resolved) => {
                    let excerpt = resolved.excerpt.clone();
                    if excerpt.is_empty() {
                        bail!("target resolution failed while exporting context");
                    }
                    artifact_context.push(ArtifactContextEntry::Target(TargetContext {
                        reference: target.reference.clone(),
                        transition: target.transition,
                        lifecycle: target.lifecycle,
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
                    }));
                }
                None => {
                    match target.transition {
                        TargetTransition::Add => {
                            artifact_context.push(ArtifactContextEntry::IntendedTarget(
                                IntendedTargetContext {
                                    reference: target.reference.clone(),
                                    transition: target.transition,
                                    lifecycle: target.lifecycle,
                                    mode,
                                    access: target.access,
                                    path: target.resolved_path.clone(),
                                    selector: target.resolved_selector.clone(),
                                    budget_bytes: Some(target.budget_bytes),
                                    budget_lines: target.budget_lines,
                                    reason: target.reason.clone(),
                                },
                            ));
                        }
                        _ => {
                            bail!("target resolution failed while exporting context");
                        }
                    }
                    if matches!(target.lifecycle, TargetLifecycle::EnsurePresent)
                        && let Some(container) = resolve_target_with_adapters(
                            &workspace.root,
                            &syu_spec_model::ArtifactTarget {
                                id: target.reference.target_id.clone(),
                                adapter: target.adapter.clone(),
                                path: RepoPath::new(target.resolved_path.clone())
                                    .expect("resolved path is a valid repo path"),
                                selector: syu_spec_model::Selector::Marker {
                                    value: "crate".into(),
                                },
                                claims: vec![],
                            },
                            &enabled_adapters(workspace),
                        )
                        .ok()
                    {
                        let support_id = format!("support:{}", target.reference);
                        if included_supports.insert(support_id.clone()) {
                            artifact_context.push(ArtifactContextEntry::Support(SupportContext {
                                support_id,
                                supports: target.reference.clone(),
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
                            }));
                        }
                    } else if matches!(target.lifecycle, TargetLifecycle::EnsurePresent) {
                        // Ownership/module scopes can be wider than an exact
                        // selector. When no explicit marker resolves, include
                        // the existing file as bounded readonly support rather
                        // than dropping context for a new private target.
                        let path = workspace.root.join(&target.resolved_path);
                        let bytes = fs::read(&path).unwrap_or_default();
                        if !bytes.is_empty() {
                            let excerpt = String::from_utf8_lossy(&bytes).into_owned();
                            let mut hash = Sha256::new();
                            hash.update(&bytes);
                            let digest = format!("sha256:{:x}", hash.finalize());
                            let support_id = format!("support:{}", target.reference);
                            if included_supports.insert(support_id.clone()) {
                                artifact_context.push(ArtifactContextEntry::Support(
                                    SupportContext {
                                        support_id,
                                        supports: target.reference.clone(),
                                        mode: ContextMode::Readonly,
                                        access: TargetAccessMode::Readonly,
                                        path: target.resolved_path.clone(),
                                        selector: ResolvedSelector {
                                            description: "ownership container".into(),
                                            symbols: vec![],
                                        },
                                        line_start: 1,
                                        line_end: excerpt.lines().count().max(1),
                                        byte_start: 0,
                                        byte_end: bytes.len(),
                                        adapter: target.adapter.clone(),
                                        facet: target.facet.clone(),
                                        role: target.role,
                                        content_hash: digest.clone(),
                                        excerpt_hash: digest,
                                        reason: "Container context for new target.".into(),
                                        excerpt,
                                    },
                                ));
                            }
                        }
                    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, path::Path};
    use tempfile::tempdir;

    fn write_minimal_workspace(root: &Path) {
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
                "    limits:\n",
                "      max_ownership_scope_units: 64\n",
                "      max_targets_per_binding: 12\n",
                "      max_slices_per_seed: 4\n",
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
                "      - id: impl\n",
                "        role: implementation\n",
                "        facet: backend\n",
                "        responsibility: Implement the target.\n",
                "        owns:\n",
                "          - id: handler-module\n",
                "            adapter: rust\n",
                "            path: src/handler.rs\n",
                "            selector: { kind: module, name: crate }\n",
                "        targets:\n",
                "          - id: handler-present\n",
                "            adapter: rust\n",
                "            path: src/handler.rs\n",
                "            selector: { kind: symbol, name: handler }\n",
                "            claims:\n",
                "              - kind: satisfies\n",
                "                criterion: REQ-TEST-001#criterion.test\n",
                "          - id: handler-missing\n",
                "            adapter: rust\n",
                "            path: src/handler.rs\n",
                "            selector: { kind: symbol, name: handler_missing }\n",
                "            claims: []\n",
            ),
        )
        .expect("feature spec");
        fs::write(
            root.join("spec/requirement.yaml"),
            concat!(
                "schema: syu/spec/v1\n",
                "kind: requirements\n",
                "namespace: sample\n",
                "category: Sample\n",
                "requirements:\n",
                "  - id: REQ-TEST-001\n",
                "    title: Test requirement\n",
                "    description: Test requirement.\n",
                "    priority: high\n",
                "    status: implemented\n",
                "    criteria:\n",
                "      - id: test\n",
                "        kind: behavior\n",
                "        statement: Test criterion.\n",
                "        governed_by: []\n",
            ),
        )
        .expect("requirement spec");
    }

    #[test]
    fn zero_add_budgets_are_rejected() {
        let tempdir = tempdir().expect("tempdir");
        write_minimal_workspace(tempdir.path());
        let workspace = SpecWorkspace::load(tempdir.path()).expect("workspace");
        let index = workspace.index().expect("index");
        let request = WorkRequest {
            schema: WORK_REQUEST_SCHEMA.into(),
            id: "WORK-TEST-003".into(),
            summary: "Reject empty budgets".into(),
            operation: WorkOperation::Add,
            seeds: vec![],
            constraints: WorkConstraints {
                max_added_bytes_per_target: Some(0),
                max_added_lines_per_target: Some(0),
                ..Default::default()
            },
            requested_targets: vec![RequestedTarget {
                reference: "FEAT-TEST-001#binding.impl/target.handler-missing"
                    .parse()
                    .unwrap(),
                criterion: None,
                transition: TargetTransition::Add,
            }],
        };
        let plan = plan(&request, &workspace, &index, "rev-3").expect("plan");
        assert_eq!(plan.status, PlanStatus::Blocked);
        assert!(
            plan.diagnostics
                .iter()
                .any(|d| d.message.contains("greater than zero"))
        );
    }

    #[test]
    fn explicit_add_transition_plans_missing_target_as_ensure_present() {
        let tempdir = tempdir().expect("tempdir");
        write_minimal_workspace(tempdir.path());
        fs::write(
            tempdir.path().join("src/handler.rs"),
            "pub fn handler() {}\n",
        )
        .expect("handler file");
        let workspace = SpecWorkspace::load(tempdir.path()).expect("workspace");
        let index = workspace.index().expect("index");
        let request = WorkRequest {
            schema: WORK_REQUEST_SCHEMA.into(),
            id: "WORK-TEST-001".into(),
            summary: "Add a new target".into(),
            operation: WorkOperation::Add,
            seeds: vec![],
            constraints: WorkConstraints {
                max_added_bytes_per_target: Some(256),
                max_added_lines_per_target: Some(8),
                ..Default::default()
            },
            requested_targets: vec![RequestedTarget {
                reference: "FEAT-TEST-001#binding.impl/target.handler-missing"
                    .parse()
                    .unwrap(),
                criterion: None,
                transition: TargetTransition::Add,
            }],
        };
        let plan = plan(&request, &workspace, &index, "rev-1").expect("plan");
        assert_eq!(plan.status, PlanStatus::Ready);
        assert!(plan.slices.iter().any(|slice| {
            slice.editable_targets.iter().any(|target| {
                target.transition == TargetTransition::Add
                    && target.lifecycle == TargetLifecycle::EnsurePresent
                    && target.reference.target_id.to_string() == "handler-missing"
            })
        }));
    }

    #[test]
    fn add_transition_blocks_missing_exact_selector_targets() {
        let tempdir = tempdir().expect("tempdir");
        write_minimal_workspace(tempdir.path());
        fs::write(
            tempdir.path().join("src/api.yaml"),
            "paths:\n  /existing:\n    get:\n      responses: {}\n",
        )
        .expect("api file");
        fs::write(
            tempdir.path().join("spec/feature.yaml"),
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
                "      - id: impl\n",
                "        role: implementation\n",
                "        facet: backend\n",
                "        responsibility: Implement the target.\n",
                "        targets:\n",
                "          - id: operation-missing\n",
                "            adapter: openapi\n",
                "            path: src/api.yaml\n",
                "            selector: { kind: operation, method: post, path: /new }\n",
                "            claims: [{ kind: satisfies, criterion: REQ-TEST-001#criterion.test }]\n",
                "          - id: pointer-missing\n",
                "            adapter: yaml\n",
                "            path: src/api.yaml\n",
                "            selector: { kind: json-pointer, value: /paths/~1new }\n",
                "            claims: []\n",
            ),
        )
        .expect("feature spec");
        let workspace = SpecWorkspace::load(tempdir.path()).expect("workspace");
        let index = workspace.index().expect("index");
        for target_id in ["operation-missing", "pointer-missing"] {
            let request = WorkRequest {
                schema: WORK_REQUEST_SCHEMA.into(),
                id: format!("WORK-TEST-{target_id}"),
                summary: "Add a new target".into(),
                operation: WorkOperation::Add,
                seeds: vec![],
                constraints: WorkConstraints {
                    max_added_bytes_per_target: Some(256),
                    max_added_lines_per_target: Some(8),
                    ..Default::default()
                },
                requested_targets: vec![RequestedTarget {
                    reference: format!("FEAT-TEST-001#binding.impl/target.{target_id}")
                        .parse()
                        .unwrap(),
                    criterion: None,
                    transition: TargetTransition::Add,
                }],
            };
            let plan = plan(&request, &workspace, &index, "rev-add").expect("plan");
            assert_eq!(plan.status, PlanStatus::Blocked);
            assert!(plan.slices.iter().any(|slice| {
                slice
                    .blockers
                    .iter()
                    .any(|diagnostic| diagnostic.rule_id == "SYU-TARGET-004")
            }));
        }
    }

    #[test]
    fn oversized_mixed_transition_closure_is_not_split_into_ready_slices() {
        let tempdir = tempdir().expect("tempdir");
        write_minimal_workspace(tempdir.path());
        fs::write(
            tempdir.path().join("syu.yaml"),
            fs::read_to_string(tempdir.path().join("syu.yaml"))
                .expect("config")
                .replacen("max_editable_files: 2", "max_editable_files: 1", 1)
                .replacen("max_editable_symbols: 4", "max_editable_symbols: 1", 1),
        )
        .expect("config");
        fs::write(
            tempdir.path().join("src/handler.rs"),
            "pub fn handler() {}\n",
        )
        .expect("handler file");
        fs::write(
            tempdir.path().join("src/registry.rs"),
            "pub fn registry() {}\n",
        )
        .expect("registry file");
        fs::write(
            tempdir.path().join("spec/feature.yaml"),
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
                "      - id: impl\n",
                "        role: implementation\n",
                "        facet: backend\n",
                "        responsibility: Implement the target.\n",
                "        targets:\n",
                "          - { id: handler-present, adapter: rust, path: src/handler.rs, selector: { kind: symbol, name: handler }, claims: [{ kind: satisfies, criterion: REQ-TEST-001#criterion.test }] }\n",
                "          - { id: handler-missing, adapter: rust, path: src/handler.rs, selector: { kind: symbol, name: handler_missing }, claims: [] }\n",
                "          - { id: registry-missing, adapter: rust, path: src/registry.rs, selector: { kind: symbol, name: registry_missing }, claims: [] }\n",
            ),
        )
        .expect("feature spec");
        let workspace = SpecWorkspace::load(tempdir.path()).expect("workspace");
        let index = workspace.index().expect("index");
        let request = WorkRequest {
            schema: WORK_REQUEST_SCHEMA.into(),
            id: "WORK-TEST-006".into(),
            summary: "Mixed transition closure".into(),
            operation: WorkOperation::Modify,
            seeds: vec![],
            constraints: WorkConstraints {
                max_added_bytes_per_target: Some(256),
                max_added_lines_per_target: Some(8),
                ..Default::default()
            },
            requested_targets: vec![
                RequestedTarget {
                    reference: "FEAT-TEST-001#binding.impl/target.handler-present"
                        .parse()
                        .unwrap(),
                    criterion: None,
                    transition: TargetTransition::Modify,
                },
                RequestedTarget {
                    reference: "FEAT-TEST-001#binding.impl/target.registry-missing"
                        .parse()
                        .unwrap(),
                    criterion: None,
                    transition: TargetTransition::Add,
                },
            ],
        };
        let plan = plan(&request, &workspace, &index, "rev-6").expect("plan");
        assert_eq!(plan.status, PlanStatus::Blocked);
        assert_eq!(plan.slices.len(), 1);
        assert!(
            plan.slices[0]
                .blockers
                .iter()
                .any(|diagnostic| diagnostic.rule_id == "SYU-WORK-003")
        );
    }

    #[test]
    fn explicit_run_only_transition_preserves_access_mode() {
        let tempdir = tempdir().expect("tempdir");
        write_minimal_workspace(tempdir.path());
        fs::write(
            tempdir.path().join("src/handler.rs"),
            "pub fn handler() {}\n",
        )
        .expect("handler file");
        let workspace = SpecWorkspace::load(tempdir.path()).expect("workspace");
        let index = workspace.index().expect("index");
        let request = WorkRequest {
            schema: WORK_REQUEST_SCHEMA.into(),
            id: "WORK-TEST-004".into(),
            summary: "Check explicit access modes".into(),
            operation: WorkOperation::Modify,
            seeds: vec![],
            constraints: WorkConstraints::default(),
            requested_targets: vec![RequestedTarget {
                reference: "FEAT-TEST-001#binding.impl/target.handler-present"
                    .parse()
                    .unwrap(),
                criterion: None,
                transition: TargetTransition::RunOnly,
            }],
        };
        let plan = plan(&request, &workspace, &index, "rev-4").expect("plan");
        assert_eq!(plan.status, PlanStatus::Ready);
        let accesses = plan
            .slices
            .iter()
            .flat_map(|slice| slice.verification_targets.iter())
            .map(|target| target.access)
            .collect::<Vec<_>>();
        assert!(accesses.contains(&TargetAccessMode::RunOnly));
    }

    #[test]
    fn explicit_readonly_transition_preserves_access_mode() {
        let tempdir = tempdir().expect("tempdir");
        write_minimal_workspace(tempdir.path());
        fs::write(
            tempdir.path().join("src/handler.rs"),
            "pub fn handler() {}\n",
        )
        .expect("handler file");
        let workspace = SpecWorkspace::load(tempdir.path()).expect("workspace");
        let index = workspace.index().expect("index");
        let request = WorkRequest {
            schema: WORK_REQUEST_SCHEMA.into(),
            id: "WORK-TEST-005".into(),
            summary: "Check explicit access modes".into(),
            operation: WorkOperation::Modify,
            seeds: vec![],
            constraints: WorkConstraints::default(),
            requested_targets: vec![RequestedTarget {
                reference: "FEAT-TEST-001#binding.impl/target.handler-present"
                    .parse()
                    .unwrap(),
                criterion: None,
                transition: TargetTransition::Readonly,
            }],
        };
        let plan = plan(&request, &workspace, &index, "rev-5").expect("plan");
        assert_eq!(plan.status, PlanStatus::Ready);
        let accesses = plan
            .slices
            .iter()
            .flat_map(|slice| slice.readonly_context.iter())
            .map(|target| target.access)
            .collect::<Vec<_>>();
        assert!(accesses.contains(&TargetAccessMode::Readonly));
    }

    #[test]
    fn context_pack_distinguishes_target_and_support_entries_for_missing_targets() {
        let tempdir = tempdir().expect("tempdir");
        write_minimal_workspace(tempdir.path());
        fs::write(
            tempdir.path().join("src/handler.rs"),
            "pub fn handler() {}\n",
        )
        .expect("handler file");
        let workspace = SpecWorkspace::load(tempdir.path()).expect("workspace");
        let index = workspace.index().expect("index");
        let request = WorkRequest {
            schema: WORK_REQUEST_SCHEMA.into(),
            id: "WORK-TEST-002".into(),
            summary: "Add a new target".into(),
            operation: WorkOperation::Add,
            seeds: vec![],
            constraints: WorkConstraints {
                max_added_bytes_per_target: Some(256),
                max_added_lines_per_target: Some(8),
                ..Default::default()
            },
            requested_targets: vec![RequestedTarget {
                reference: "FEAT-TEST-001#binding.impl/target.handler-missing"
                    .parse()
                    .unwrap(),
                criterion: None,
                transition: TargetTransition::Add,
            }],
        };
        let plan = plan(&request, &workspace, &index, "rev-2").expect("plan");
        let slice = plan.slices.first().expect("slice");
        let pack = export_context(&plan, &slice.id, &workspace, &index, "rev-2").expect("pack");
        let intended_entries = pack
            .artifact_context
            .iter()
            .filter(|entry| matches!(entry, ArtifactContextEntry::IntendedTarget(_)))
            .count();
        let support_entries = pack
            .artifact_context
            .iter()
            .filter(|entry| matches!(entry, ArtifactContextEntry::Support(_)))
            .count();
        assert_eq!(intended_entries, 1);
        assert_eq!(support_entries, 1);
        let support = pack
            .artifact_context
            .iter()
            .find_map(|entry| match entry {
                ArtifactContextEntry::Support(value) => Some(value),
                _ => None,
            })
            .expect("support entry");
        assert_eq!(support.supports.target_id.to_string(), "handler-missing");
        assert!(support.support_id.starts_with("support:"));
    }
}
