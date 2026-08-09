#![forbid(unsafe_code)]
use anyhow::{Context, Result, bail};
use globset::{Glob, GlobSet, GlobSetBuilder};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
};
use syu_diagnostics::Diagnostic;
use syu_project_model::ValidationPreset;
use syu_spec_model::format_sha256;
use syu_spec_model::{
    ArtifactBinding, ArtifactTargetLifecycle, BindingRole, BoundTargetRef, ItemStatus, RepoPath,
    Selector, SpecAnchor, TargetClaim,
};
use syu_work_model::*;
use syu_workspace::{
    AnchorValue, SpecIndex, SpecWorkspace, resolve_indexed_target, resolve_target_in_workspace,
    selector_supports_editable,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SuggestionConfidence {
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TargetSuggestion {
    pub id: String,
    pub rank: usize,
    #[serde(rename = "ref")]
    pub reference: BoundTargetRef,
    pub role: BindingRole,
    pub transition: TargetTransition,
    pub lifecycle: TargetLifecycle,
    pub path: String,
    pub selector: String,
    pub existing_file: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub budget_bytes: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub budget_lines: Option<usize>,
    pub confidence: SuggestionConfidence,
    pub evidence: Vec<String>,
    pub evidence_fingerprint: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SplitWorkRecommendation {
    pub reason: String,
    pub suggested_groups: Vec<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TargetSuggestionSet {
    pub criterion: SpecAnchor,
    pub suggestions: Vec<TargetSuggestion>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub split_recommendation: Option<SplitWorkRecommendation>,
    pub suggestion_token: String,
}

/// Readiness and governance probes use this intentionally non-serializable
/// input instead of manufacturing a public WorkRequest.
#[derive(Debug, Clone)]
pub struct PlanProbe {
    pub criterion: SpecAnchor,
    pub requested_targets: Vec<BoundTargetRef>,
    pub max_slices: usize,
}

/// Validate the exact origin before a request is expanded into a plan.
///
/// This is deliberately owned by the canonical planner rather than by a UI
/// server. CLI planning, execution re-planning, and Workbench all need the
/// same closed-world origin boundary; otherwise an apparently exact Feature
/// or target selection could silently broaden when it crosses a process
/// boundary.
pub fn validate_work_origin(index: &SpecIndex, origin: &WorkOrigin) -> Result<()> {
    let criterion = origin.criterion();
    if criterion.kind != syu_spec_model::LocalAnchorKind::Criterion {
        bail!("Work must start from an exact requirement criterion");
    }
    if !matches!(index.anchor(criterion), Some(AnchorValue::Criterion(_))) {
        bail!("Work criterion anchor does not resolve to an exact requirement criterion");
    }
    if index.criterion_status.get(criterion) != Some(&ItemStatus::Implemented) {
        bail!("origin criterion {criterion} is not implemented");
    }
    match origin {
        WorkOrigin::RequirementCriterion { criterion } => {
            validate_requirement_origin(index, criterion)
        }
        WorkOrigin::FeatureImplementationBinding {
            binding,
            criterion,
            targets,
        } => {
            let artifact_binding = index
                .bindings
                .get(binding)
                .ok_or_else(|| anyhow::anyhow!("implementation binding {binding} is unknown"))?;
            if artifact_binding.role != BindingRole::Implementation {
                bail!("origin binding {binding} is not an implementation binding");
            }
            if index.item_status.get(&binding.item) != Some(&ItemStatus::Implemented) {
                bail!("origin binding {binding} is not implemented");
            }
            let mut expected = artifact_binding
                .targets
                .iter()
                .filter(|target| !matches!(target.lifecycle, ArtifactTargetLifecycle::Absent))
                .map(|target| BoundTargetRef {
                    binding: binding.clone(),
                    target_id: target.id.clone(),
                })
                .collect::<Vec<_>>();
            expected.sort();
            if targets.windows(2).any(|window| window[0] >= window[1]) {
                bail!("implementation binding origin target list is not canonically sorted");
            }
            let mut actual = targets.clone();
            actual.sort();
            if expected != actual {
                bail!("implementation binding origin must contain its complete active target set");
            }
            if targets.is_empty() {
                bail!("implementation binding origin has no active implementation target");
            }
            let binding_criteria = targets
                .iter()
                .filter_map(|target| index.target(target))
                .flat_map(|artifact| artifact.claims.iter())
                .filter_map(|claim| match claim {
                    TargetClaim::Satisfies { criterion } => Some(criterion.clone()),
                    _ => None,
                })
                .collect::<BTreeSet<_>>();
            if binding_criteria.len() != 1 {
                bail!("implementation binding origin has an ambiguous criterion");
            }
            if binding_criteria.iter().next() != Some(criterion) {
                bail!("implementation binding origin has no exact satisfies criterion");
            }
            for target in targets {
                validate_origin_target(index, target, binding, criterion)?;
            }
            validate_origin_contract_closure(index, criterion, targets)
        }
        WorkOrigin::FeatureImplementationTarget {
            target,
            binding,
            criterion,
        } => validate_origin_target(index, target, binding, criterion),
    }
}

/// Validate a request after it has crossed a serialization boundary.
///
/// Requested targets deliberately do not repeat the origin criterion on the
/// wire. That criterion is derived here from the authoritative origin, and
/// every requested target is checked against the resulting executable or
/// contextual closure. A target's own claims are never allowed to silently
/// replace the selected origin.
pub fn validate_work_request(index: &SpecIndex, request: &WorkRequest) -> Result<()> {
    if request.schema != WORK_REQUEST_SCHEMA {
        bail!("Work request schema must be {WORK_REQUEST_SCHEMA}");
    }
    let criterion = request.origin.criterion();
    if criterion.kind != syu_spec_model::LocalAnchorKind::Criterion
        || !matches!(index.anchor(criterion), Some(AnchorValue::Criterion(_)))
    {
        bail!("Work origin must name one exact requirement criterion");
    }
    for requested in &request.requested_targets {
        if let Some(requested_criterion) = requested.criterion()
            && requested_criterion != criterion
        {
            bail!(
                "requested target {} is outside the exact origin criterion {}",
                requested.reference(),
                criterion
            );
        }
    }
    // The request may have crossed a process boundary, so the origin itself
    // must be revalidated for every origin kind.  In particular, a
    // Requirement origin is not a permission to infer a broader binding or
    // substitute a target's own claims for the selected criterion.
    validate_work_origin(index, &request.origin)?;

    let boundary = request_target_boundary(index, &request.origin, request.operation);
    if !request.constraints.exact_scope
        && (!request.constraints.exact_generated_targets.is_empty()
            || !request.constraints.exact_contracts.is_empty())
    {
        bail!("exact selected-slice closure is only valid with exact_scope");
    }
    for target in &request.requested_targets {
        let allowed = match target.transition(request_default_transition(request.operation)) {
            TargetTransition::Add | TargetTransition::Modify | TargetTransition::Remove => {
                &boundary.editable
            }
            TargetTransition::RunOnly => &boundary.run_only,
            TargetTransition::Readonly => &boundary.readonly,
        };
        let allowed_add = matches!(
            target.transition(request_default_transition(request.operation)),
            TargetTransition::Add
        ) && requirement_add_target_is_in_origin_binding(
            index,
            &request.origin,
            target.reference(),
        );
        let allowed_declared_editable = matches!(
            target.transition(request_default_transition(request.operation)),
            TargetTransition::Modify | TargetTransition::Remove
        ) && requirement_declared_target_is_in_origin_binding(
            index,
            &request.origin,
            target.reference(),
        );
        if !allowed.contains(target.reference()) && !allowed_add && !allowed_declared_editable {
            bail!(
                "requested target {} is outside the exact {} origin closure",
                target.reference(),
                match target.transition(request_default_transition(request.operation)) {
                    TargetTransition::Add | TargetTransition::Modify | TargetTransition::Remove =>
                        "editable",
                    TargetTransition::RunOnly => "verification",
                    TargetTransition::Readonly => "readonly",
                }
            );
        }
    }
    let generated = request
        .constraints
        .exact_generated_targets
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    if generated.iter().any(|target| {
        !boundary.readonly.contains(target) || !index.generated_from.contains_key(target)
    }) {
        bail!("exact generated target closure is outside the selected origin");
    }
    if request.constraints.exact_scope {
        let roots = request
            .requested_targets
            .iter()
            .map(|target| target.reference.clone())
            .collect::<Vec<_>>();
        let (related_targets, _, expected_contracts) = dependency_closure(index, &roots);
        let expected_generated = related_targets
            .iter()
            .filter(|target| index.generated_from.contains_key(*target))
            .cloned()
            .collect::<BTreeSet<_>>();
        if generated != expected_generated {
            bail!(
                "exact generated target closure is incomplete or broader than the selected slice"
            );
        }
        if request
            .constraints
            .exact_contracts
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>()
            != expected_contracts
        {
            bail!("exact contract closure is incomplete or broader than the selected slice");
        }
    } else if request
        .constraints
        .exact_contracts
        .iter()
        .any(|contract| !boundary.contracts.contains(contract))
    {
        bail!("exact contract closure is outside the selected origin");
    }
    Ok(())
}

fn validate_planning_origin(request: &WorkRequest, index: &SpecIndex) -> Result<()> {
    validate_work_request(index, request)
}

#[derive(Default)]
struct RequestTargetBoundary {
    editable: BTreeSet<BoundTargetRef>,
    run_only: BTreeSet<BoundTargetRef>,
    readonly: BTreeSet<BoundTargetRef>,
    contracts: BTreeSet<SpecAnchor>,
}

fn request_default_transition(operation: WorkOperation) -> TargetTransition {
    match operation {
        WorkOperation::Add => TargetTransition::Add,
        WorkOperation::Remove => TargetTransition::Remove,
        _ => TargetTransition::Modify,
    }
}

fn request_target_boundary(
    index: &SpecIndex,
    origin: &WorkOrigin,
    operation: WorkOperation,
) -> RequestTargetBoundary {
    let criterion = origin.criterion();
    let implementation_roots = match origin {
        WorkOrigin::RequirementCriterion { .. } => index
            .criteria_to_implementation_targets
            .get(criterion)
            .into_iter()
            .flatten()
            .filter(|target| {
                index.bindings.get(&target.binding).is_some_and(|binding| {
                    binding.role == BindingRole::Implementation
                        && index.item_status.get(&target.binding.item)
                            == Some(&ItemStatus::Implemented)
                        && index.target(target).is_some_and(|artifact| {
                            !matches!(artifact.lifecycle, ArtifactTargetLifecycle::Absent)
                        })
                        && index.target_to_artifact.contains_key(*target)
                })
            })
            .cloned()
            .collect(),
        WorkOrigin::FeatureImplementationBinding { targets, .. } => targets.clone(),
        WorkOrigin::FeatureImplementationTarget { target, .. } => vec![target.clone()],
    };
    let documentation_targets = if matches!(origin, WorkOrigin::RequirementCriterion { .. }) {
        index
            .bindings
            .iter()
            .flat_map(|(binding_anchor, binding)| {
                binding
                    .targets
                    .iter()
                    .filter(|target| {
                        binding.role == BindingRole::Documentation
                            && target.claims.iter().any(|claim| {
                                matches!(claim, TargetClaim::Documents { anchor } if anchor == criterion)
                            })
                    })
                    .map(|target| BoundTargetRef {
                        binding: binding_anchor.clone(),
                        target_id: target.id.clone(),
                    })
            })
            .collect::<BTreeSet<_>>()
    } else {
        BTreeSet::new()
    };
    let exposed_implementation_targets =
        if matches!(origin, WorkOrigin::RequirementCriterion { .. }) {
            index
                .exposes_by_target
                .iter()
                .filter(|(_, exposed)| implementation_roots.contains(exposed))
                .map(|(public, _)| public.clone())
                .collect::<BTreeSet<_>>()
        } else {
            BTreeSet::new()
        };
    let editable = if matches!(origin, WorkOrigin::RequirementCriterion { .. })
        && operation == WorkOperation::Document
    {
        documentation_targets.clone()
    } else if operation == WorkOperation::Investigate {
        BTreeSet::new()
    } else if matches!(origin, WorkOrigin::RequirementCriterion { .. }) {
        implementation_roots
            .iter()
            .cloned()
            .chain(exposed_implementation_targets.iter().cloned())
            .collect()
    } else {
        implementation_roots.iter().cloned().collect()
    };
    let mut boundary = RequestTargetBoundary {
        editable,
        ..RequestTargetBoundary::default()
    };
    boundary.run_only.extend(boundary.editable.iter().cloned());
    boundary.readonly.extend(boundary.editable.iter().cloned());
    for implementation in &implementation_roots {
        for verification in index
            .criteria_to_verification_targets
            .get(criterion)
            .into_iter()
            .flatten()
        {
            if index
                .verification_by_target
                .get(implementation)
                .is_some_and(|covered| covered.contains(verification))
            {
                boundary.run_only.insert(verification.clone());
            }
        }
    }
    if matches!(origin, WorkOrigin::RequirementCriterion { .. }) {
        boundary
            .readonly
            .extend(implementation_roots.iter().cloned());
    }
    if operation == WorkOperation::Document {
        boundary
            .readonly
            .extend(implementation_roots.iter().cloned());
    }

    let mut queue = implementation_roots;
    queue.extend(documentation_targets);
    extend_request_context(index, &mut boundary, queue);
    boundary
}

fn requirement_add_target_is_in_origin_binding(
    index: &SpecIndex,
    origin: &WorkOrigin,
    target: &BoundTargetRef,
) -> bool {
    let WorkOrigin::RequirementCriterion { criterion } = origin else {
        return false;
    };
    let Some(binding) = index.bindings.get(&target.binding) else {
        return false;
    };
    if !matches!(
        binding.role,
        BindingRole::Implementation | BindingRole::Verification
    ) || !matches!(
        index.item_status.get(&target.binding.item),
        Some(ItemStatus::Implemented | ItemStatus::Planned)
    ) {
        return false;
    }
    let Some(declared) = index.target(target) else {
        return false;
    };
    // `absent` is a removal declaration. It is never an authority for an
    // Add request, even when the target is otherwise linked to the criterion.
    if matches!(
        declared.lifecycle,
        syu_spec_model::ArtifactTargetLifecycle::Absent
    ) {
        return false;
    }
    let has_exact_binding = match binding.role {
        BindingRole::Implementation => {
            let target_claims_criterion = declared.claims.iter().any(|claim| {
                matches!(claim, TargetClaim::Satisfies { criterion: actual } if actual == criterion)
            });
            target_claims_criterion
                && index
                    .criteria_to_implementation_targets
                    .get(criterion)
                    .into_iter()
                    .flatten()
                    .any(|root| root.binding == target.binding)
        }
        BindingRole::Verification => declared.claims.iter().any(|claim| {
            matches!(claim, TargetClaim::Verifies { criterion: actual, .. } if actual == criterion)
        }),
        _ => false,
    };
    let has_other_criterion_claim = declared.claims.iter().any(|claim| match claim {
        TargetClaim::Satisfies { criterion: actual }
        | TargetClaim::Verifies {
            criterion: actual, ..
        } => actual != criterion,
        _ => false,
    });
    let is_new_verification_post_state = binding.role == BindingRole::Verification
        && index.item_status.get(&target.binding.item) == Some(&ItemStatus::Planned);
    has_exact_binding
        && !has_other_criterion_claim
        && (!index.target_to_artifact.contains_key(target) || is_new_verification_post_state)
}

fn requirement_declared_target_is_in_origin_binding(
    index: &SpecIndex,
    origin: &WorkOrigin,
    target: &BoundTargetRef,
) -> bool {
    let WorkOrigin::RequirementCriterion { criterion } = origin else {
        return false;
    };
    let Some(binding) = index.bindings.get(&target.binding) else {
        return false;
    };
    if binding.role != BindingRole::Implementation
        || !matches!(
            index.item_status.get(&target.binding.item),
            Some(ItemStatus::Implemented | ItemStatus::Planned)
        )
    {
        return false;
    }
    let Some(declared) = index.target(target) else {
        return false;
    };
    declared.claims.iter().any(|claim| {
        matches!(claim, TargetClaim::Satisfies { criterion: actual } if actual == criterion)
    }) && !declared.claims.iter().any(|claim| match claim {
        TargetClaim::Satisfies { criterion: actual }
        | TargetClaim::Verifies {
            criterion: actual, ..
        } => actual != criterion,
        _ => false,
    })
}

fn extend_request_context(
    index: &SpecIndex,
    boundary: &mut RequestTargetBoundary,
    mut queue: Vec<BoundTargetRef>,
) {
    let mut seen = BTreeSet::new();
    while let Some(target) = queue.pop() {
        if !seen.insert(target.clone()) {
            continue;
        }
        for generated in index.generated_by_source.get(&target).into_iter().flatten() {
            boundary.readonly.insert(generated.clone());
            queue.push(generated.clone());
        }
        for source in index.generated_from.get(&target).into_iter().flatten() {
            boundary.readonly.insert(source.clone());
            queue.push(source.clone());
        }
        for contract_anchor in index.contracts_by_target.get(&target).into_iter().flatten() {
            boundary.contracts.insert(contract_anchor.clone());
            let Some(contract) = index.contracts.get(contract_anchor) else {
                continue;
            };
            let related = std::iter::once(&contract.source).chain(
                contract
                    .participants
                    .iter()
                    .map(|participant| &participant.target),
            );
            for related in related {
                boundary.readonly.insert(related.clone());
                queue.push(related.clone());
            }
        }
    }
}

fn validate_requirement_origin(index: &SpecIndex, criterion: &SpecAnchor) -> Result<()> {
    let implementations = index
        .criteria_to_implementation_targets
        .get(criterion)
        .into_iter()
        .flatten()
        .filter(|target| {
            index.bindings.get(&target.binding).is_some_and(|binding| {
                binding.role == BindingRole::Implementation
                    && index.item_status.get(&target.binding.item) == Some(&ItemStatus::Implemented)
                    && binding.targets.iter().any(|candidate| {
                        candidate.id == target.target_id
                            && !matches!(candidate.lifecycle, ArtifactTargetLifecycle::Absent)
                    })
            })
        })
        .cloned()
        .collect::<Vec<_>>();
    if implementations.is_empty() {
        bail!("origin criterion {criterion} has no active implemented implementation target");
    }
    validate_origin_contract_closure(index, criterion, &implementations)
}

fn validate_origin_target(
    index: &SpecIndex,
    target: &BoundTargetRef,
    binding: &SpecAnchor,
    criterion: &SpecAnchor,
) -> Result<()> {
    if &target.binding != binding {
        bail!("origin target {target} does not belong to binding {binding}");
    }
    let artifact_binding = index
        .bindings
        .get(binding)
        .ok_or_else(|| anyhow::anyhow!("origin binding {binding} is unknown"))?;
    if artifact_binding.role != BindingRole::Implementation {
        bail!("origin binding {binding} is not an implementation binding");
    }
    if index.item_status.get(&binding.item) != Some(&ItemStatus::Implemented) {
        bail!("origin binding {binding} is not implemented");
    }
    let artifact = index
        .target(target)
        .ok_or_else(|| anyhow::anyhow!("origin target {target} is unknown"))?;
    if matches!(artifact.lifecycle, ArtifactTargetLifecycle::Absent) {
        bail!("origin target {target} is absent");
    }
    if !index.target_to_artifact.contains_key(target) {
        bail!("origin target {target} does not resolve to an active inventory artifact");
    }
    let claims = artifact
        .claims
        .iter()
        .filter_map(|claim| match claim {
            TargetClaim::Satisfies { criterion: actual } => Some(actual),
            _ => None,
        })
        .collect::<Vec<_>>();
    if claims.len() != 1 || claims[0] != criterion {
        bail!("origin target {target} must have exactly one matching satisfies criterion");
    }
    let verification_targets = index
        .criteria_to_verification_targets
        .get(criterion)
        .into_iter()
        .flatten();
    if !verification_targets.clone().any(|verification| {
        index
            .verification_by_target
            .get(target)
            .is_some_and(|covered| covered.contains(verification))
    }) {
        bail!("origin target {target} has no exact verification coverage");
    }
    validate_origin_contract_closure(index, criterion, std::slice::from_ref(target))
}

fn validate_origin_contract_closure(
    index: &SpecIndex,
    criterion: &SpecAnchor,
    roots: &[BoundTargetRef],
) -> Result<()> {
    let mut pending = roots.to_vec();
    let mut visited_targets = BTreeSet::new();
    let mut visited_contracts = BTreeSet::new();
    while let Some(target) = pending.pop() {
        if !visited_targets.insert(target.clone()) {
            continue;
        }
        let Some(contracts) = index.contracts_by_target.get(&target) else {
            continue;
        };
        for contract_anchor in contracts {
            if !visited_contracts.insert(contract_anchor.clone()) {
                continue;
            }
            let contract = index.contracts.get(contract_anchor).ok_or_else(|| {
                anyhow::anyhow!("origin target {target} has an unresolved contract closure")
            })?;
            if contract.guarantees.is_empty()
                || contract
                    .guarantees
                    .iter()
                    .any(|guarantee| guarantee != criterion)
            {
                bail!(
                    "origin target {target} has a contract without the exact criterion guarantee"
                );
            }
            let mut related = vec![contract.source.clone()];
            related.extend(
                contract
                    .participants
                    .iter()
                    .map(|participant| participant.target.clone()),
            );
            for related_target in related {
                let artifact = index.target(&related_target).ok_or_else(|| {
                    anyhow::anyhow!("origin target {target} has an unresolved contract participant")
                })?;
                if matches!(artifact.lifecycle, ArtifactTargetLifecycle::Absent) {
                    bail!("origin target {related_target} has an absent contract participant");
                }
                if !index.target_to_artifact.contains_key(&related_target) {
                    bail!(
                        "origin target {related_target} does not resolve to an active contract artifact"
                    );
                }
                pending.push(related_target);
            }
        }
    }
    Ok(())
}

pub fn suggest_targets(
    criterion: &SpecAnchor,
    workspace: &SpecWorkspace,
    index: &SpecIndex,
) -> Result<TargetSuggestionSet> {
    let statement = match index.anchor(criterion) {
        Some(AnchorValue::Criterion(value)) => value.statement.as_str(),
        _ => bail!("target suggestions require an exact criterion anchor"),
    };
    let workspace_fingerprint = workspace.try_fingerprint()?;
    let criterion_terms = significant_terms(statement);
    let governing_rules = index
        .criteria_to_rules
        .get(criterion)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .collect::<BTreeSet<_>>();
    let contract_targets = index
        .contracts
        .values()
        .filter(|contract| contract.guarantees.iter().any(|anchor| anchor == criterion))
        .flat_map(|contract| {
            std::iter::once(contract.source.clone())
                .chain(contract.participants.iter().map(|item| item.target.clone()))
        })
        .collect::<BTreeSet<_>>();
    let mut ranked = Vec::new();
    for (binding_anchor, binding) in &index.bindings {
        if index.item_status.get(&binding_anchor.item) == Some(&ItemStatus::Deprecated) {
            continue;
        }
        for target in &binding.targets {
            let reference = BoundTargetRef {
                binding: binding_anchor.clone(),
                target_id: target.id.clone(),
            };
            let current_target = index.target_to_artifact.contains_key(&reference);
            let artifact_exists = index.all_target_to_artifact.contains_key(&reference);
            let planned_item =
                index.item_status.get(&binding_anchor.item) == Some(&ItemStatus::Planned);
            let planned_missing_target = !current_target
                && target.lifecycle != ArtifactTargetLifecycle::Absent
                && planned_item;
            let mut score = 0usize;
            let mut evidence = Vec::new();
            let directly_claims = target.claims.iter().any(|claim| {
                !matches!(claim, TargetClaim::Enforces { .. })
                    && claim_anchor(claim).is_some_and(|actual| actual == criterion)
            });
            let planned_remove_target = target.lifecycle == ArtifactTargetLifecycle::Absent
                && artifact_exists
                && planned_item
                && directly_claims;
            if target.lifecycle == ArtifactTargetLifecycle::Absent && !planned_remove_target {
                continue;
            }
            if !current_target && !planned_missing_target && !planned_remove_target {
                continue;
            }
            if directly_claims {
                score += 100;
                evidence.push("The target explicitly claims this criterion.".into());
                if !current_target {
                    evidence.push(
                        "The target is declared in the specification but is not present in the current inventory; review it as an Add candidate."
                            .into(),
                    );
                }
            }
            if planned_missing_target {
                score += 20;
                evidence.push(
                    "This planned target is missing from the current inventory and requires explicit Add approval."
                        .into(),
                );
            }
            if planned_remove_target {
                score += 20;
                evidence.push(
                    "This planned absent target resolves to a current artifact and requires explicit Remove approval (ensure-absent)."
                        .into(),
                );
            }
            if binding_anchor.item == criterion.item {
                score += 40;
                evidence.push("The target belongs to the same specification item.".into());
            }
            let enforces_governing_rule = target.claims.iter().any(
                |claim| matches!(claim, TargetClaim::Enforces { rule } if governing_rules.contains(rule)),
            );
            if enforces_governing_rule {
                score += 75;
                evidence.push("The target enforces a rule governing this criterion.".into());
            }
            let supports_contract = contract_targets.contains(&reference);
            if supports_contract {
                score += 75;
                evidence.push(
                    "The target participates in a contract guaranteeing this criterion.".into(),
                );
            }
            // Suggestions may rank evidence, but executable scope starts from
            // an exact relation to the selected criterion. A shared
            // requirement item or coincidental terminology is not authority
            // to propose another criterion's target.
            if !directly_claims && !enforces_governing_rule && !supports_contract {
                continue;
            }
            let searchable = format!(
                "{} {} {} {} {}",
                binding.facet,
                binding.responsibility,
                target.path.display(),
                selector_text(&target.selector),
                index
                    .target_to_artifact
                    .get(&reference)
                    .map(String::as_str)
                    .unwrap_or_default()
            );
            let matched_terms = significant_terms(&searchable)
                .intersection(&criterion_terms)
                .cloned()
                .collect::<Vec<_>>();
            if !matched_terms.is_empty() {
                score += 10 + matched_terms.len().min(5) * 3;
                evidence.push(format!(
                    "Specification and target names share: {}.",
                    matched_terms.join(", ")
                ));
            }
            if !supports_contract && index.contracts_by_target.contains_key(&reference) {
                score += 8;
                evidence.push("The target participates in an explicit contract.".into());
            }
            if index.verification_by_target.contains_key(&reference) {
                score += 8;
                evidence.push("An exact verification target covers this target.".into());
            }
            if score == 0 {
                continue;
            }
            let transition = if planned_remove_target {
                TargetTransition::Remove
            } else if planned_missing_target {
                TargetTransition::Add
            } else {
                transition_for_role(binding.role)
            };
            let lifecycle = match transition {
                TargetTransition::Add => TargetLifecycle::EnsurePresent,
                TargetTransition::Remove => TargetLifecycle::EnsureAbsent,
                _ => TargetLifecycle::Stable,
            };
            let path = target.path.to_string_lossy().into_owned();
            let selector = selector_text(&target.selector);
            let existing_file = workspace.root.join(target.path.as_path()).is_file();
            let budget_bytes = (transition == TargetTransition::Add).then_some(512);
            let budget_lines = (transition == TargetTransition::Add).then_some(32);
            let confidence = if directly_claims || enforces_governing_rule || supports_contract {
                SuggestionConfidence::High
            } else if score >= 40 || matched_terms.len() >= 2 {
                SuggestionConfidence::Medium
            } else {
                SuggestionConfidence::Low
            };
            let artifact_digest = index
                .target_to_artifact
                .get(&reference)
                .or_else(|| index.all_target_to_artifact.get(&reference))
                .and_then(|identity| {
                    index
                        .artifact_units
                        .iter()
                        .find(|unit| &unit.identity == identity)
                })
                .map(|unit| unit.digest.as_str())
                .unwrap_or_default();
            let id = suggestion_id(&reference);
            let evidence_fingerprint = suggestion_digest(
                criterion,
                &workspace_fingerprint,
                &id,
                &reference,
                binding.role,
                transition,
                lifecycle,
                &path,
                &selector,
                existing_file,
                budget_bytes,
                budget_lines,
                confidence,
                statement,
                artifact_digest,
                &evidence,
            );
            ranked.push((
                score,
                id,
                reference,
                binding.role,
                transition,
                lifecycle,
                path,
                selector,
                existing_file,
                budget_bytes,
                budget_lines,
                confidence,
                evidence,
                evidence_fingerprint,
            ));
        }
    }
    ranked.sort_by(|left, right| right.0.cmp(&left.0).then_with(|| left.1.cmp(&right.1)));
    let suggestions = ranked
        .into_iter()
        .enumerate()
        .map(
            |(
                offset,
                (
                    _,
                    id,
                    reference,
                    role,
                    transition,
                    lifecycle,
                    path,
                    selector,
                    existing_file,
                    budget_bytes,
                    budget_lines,
                    confidence,
                    evidence,
                    evidence_fingerprint,
                ),
            )| {
                TargetSuggestion {
                    id,
                    rank: offset + 1,
                    reference,
                    role,
                    transition,
                    lifecycle,
                    path,
                    selector,
                    existing_file,
                    budget_bytes,
                    budget_lines,
                    confidence,
                    evidence,
                    evidence_fingerprint,
                }
            },
        )
        .collect::<Vec<_>>();
    let split_recommendation = split_work_recommendation(&suggestions, workspace, index);
    let suggestion_token = suggestion_set_token(criterion, workspace, &suggestions)?;
    Ok(TargetSuggestionSet {
        criterion: criterion.clone(),
        suggestions,
        split_recommendation,
        suggestion_token,
    })
}

fn claim_anchor(claim: &TargetClaim) -> Option<&SpecAnchor> {
    match claim {
        TargetClaim::Satisfies { criterion } | TargetClaim::Verifies { criterion, .. } => {
            Some(criterion)
        }
        TargetClaim::Documents { anchor } | TargetClaim::Evidences { anchor } => Some(anchor),
        TargetClaim::Enforces { rule } => Some(rule),
        TargetClaim::GeneratedFrom { .. } | TargetClaim::Exposes { .. } => None,
    }
}

fn transition_for_role(role: BindingRole) -> TargetTransition {
    match role {
        BindingRole::Verification => TargetTransition::RunOnly,
        BindingRole::Generated | BindingRole::Evidence => TargetTransition::Readonly,
        _ => TargetTransition::Modify,
    }
}

fn selector_text(selector: &Selector) -> String {
    match selector {
        Selector::File => "file".into(),
        Selector::Symbol { name } => name.clone(),
        Selector::Operation { method, path } => format!("{method} {path}"),
        Selector::Heading { value }
        | Selector::JsonPointer { value }
        | Selector::Marker { value } => value.clone(),
    }
}

fn significant_terms(value: &str) -> BTreeSet<String> {
    const STOP: &[&str] = &[
        "the", "a", "an", "and", "or", "to", "of", "for", "in", "on", "with", "is", "are", "be",
        "this", "that", "target", "behavior",
    ];
    value
        .split(|character: char| !character.is_ascii_alphanumeric())
        .map(str::to_ascii_lowercase)
        .filter(|term| term.len() >= 3 && !STOP.contains(&term.as_str()))
        .collect()
}

fn suggestion_id(reference: &BoundTargetRef) -> String {
    let digest = Sha256::digest(reference.to_string().as_bytes());
    format!("target-{}", &hex_digest(&digest)[..16])
}

#[allow(clippy::too_many_arguments)]
fn suggestion_digest(
    criterion: &SpecAnchor,
    workspace_fingerprint: &str,
    id: &str,
    reference: &BoundTargetRef,
    role: BindingRole,
    transition: TargetTransition,
    lifecycle: TargetLifecycle,
    path: &str,
    selector: &str,
    existing_file: bool,
    budget_bytes: Option<usize>,
    budget_lines: Option<usize>,
    confidence: SuggestionConfidence,
    criterion_statement: &str,
    artifact_digest: &str,
    evidence: &[String],
) -> String {
    let authority = serde_json::json!({
        "schema": "syu/work-target-suggestion-authority/v1",
        "criterion": criterion,
        "workspace_fingerprint": workspace_fingerprint,
        "id": id,
        "reference": reference,
        "role": role,
        "transition": transition,
        "lifecycle": lifecycle,
        "path": path,
        "selector": selector,
        "existing_file": existing_file,
        "budget_bytes": budget_bytes,
        "budget_lines": budget_lines,
        "confidence": confidence,
        "criterion_statement": criterion_statement,
        "artifact_digest": artifact_digest,
        "evidence": evidence,
    });
    let mut hash = Sha256::new();
    hash.update(syu_work_model::canonical_json_bytes(authority));
    hex_digest(&hash.finalize())
}

fn suggestion_set_token(
    criterion: &SpecAnchor,
    workspace: &SpecWorkspace,
    suggestions: &[TargetSuggestion],
) -> Result<String> {
    let mut hash = Sha256::new();
    hash.update(criterion.to_string().as_bytes());
    hash.update(workspace.try_fingerprint()?.as_bytes());
    for suggestion in suggestions {
        hash.update(suggestion.id.as_bytes());
        hash.update(suggestion.evidence_fingerprint.as_bytes());
    }
    Ok(hex_digest(&hash.finalize()))
}

fn hex_digest(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

pub fn split_work_recommendation(
    suggestions: &[TargetSuggestion],
    workspace: &SpecWorkspace,
    index: &SpecIndex,
) -> Option<SplitWorkRecommendation> {
    let limits = &workspace.config.work.slicing;
    let budget = suggestion_budget(suggestions, index);
    if !suggestion_budget_exceeds(&budget, limits) {
        return None;
    }
    let mut groups = Vec::<Vec<TargetSuggestion>>::new();
    for candidate in suggestions {
        let can_append = groups.last().is_some_and(|group| {
            let mut combined = group.clone();
            combined.push(candidate.clone());
            !suggestion_budget_exceeds(&suggestion_budget(&combined, index), limits)
        });
        if can_append {
            groups
                .last_mut()
                .expect("group exists")
                .push(candidate.clone());
        } else {
            groups.push(vec![candidate.clone()]);
        }
    }
    let suggested_groups = groups
        .into_iter()
        .map(|group| group.into_iter().map(|candidate| candidate.id).collect())
        .collect();
    Some(SplitWorkRecommendation {
        reason: format!(
            "The candidate set exceeds configured slicing limits (editable files {}/{}, editable symbols {}/{}, verification targets {}/{}, readonly targets {}/{}, bytes {}/{}). Review and approve smaller groups.",
            budget.editable_files,
            limits.max_editable_files,
            budget.editable_symbols,
            limits.max_editable_symbols,
            budget.verification_targets,
            limits.max_verification_targets,
            budget.readonly_targets,
            limits.max_readonly_targets,
            budget.total_bytes,
            limits.max_total_bytes,
        ),
        suggested_groups,
    })
}

#[derive(Debug, Default)]
struct SuggestionBudget {
    editable_files: usize,
    editable_symbols: usize,
    verification_targets: usize,
    readonly_targets: usize,
    total_bytes: usize,
}

fn suggestion_budget(suggestions: &[TargetSuggestion], index: &SpecIndex) -> SuggestionBudget {
    let mut editable_paths = BTreeSet::new();
    let mut budget = SuggestionBudget::default();
    for candidate in suggestions {
        match candidate.transition {
            TargetTransition::Add => {
                if candidate.role == BindingRole::Verification {
                    // A planned verification Add is represented in the plan
                    // twice: once for the post-write target and once for the
                    // RunOnly verification phase.
                    budget.verification_targets += 1;
                }
                if let Some(target) = index.target(&candidate.reference) {
                    editable_paths.insert(target.path.clone());
                    budget.editable_symbols += match target.selector {
                        Selector::Symbol { .. } => 1,
                        _ => 0,
                    };
                }
            }
            TargetTransition::Modify | TargetTransition::Remove => {
                if let Some(target) = index.target(&candidate.reference) {
                    editable_paths.insert(target.path.clone());
                    budget.editable_symbols += match target.selector {
                        Selector::Symbol { .. } => 1,
                        _ => 0,
                    };
                }
            }
            TargetTransition::RunOnly => budget.verification_targets += 1,
            TargetTransition::Readonly => budget.readonly_targets += 1,
        }
        budget.total_bytes += suggestion_budget_bytes(candidate, index);
    }
    budget.editable_files = editable_paths.len();
    budget
}

fn suggestion_budget_bytes(candidate: &TargetSuggestion, index: &SpecIndex) -> usize {
    if candidate.transition == TargetTransition::Add {
        return candidate.budget_bytes.unwrap_or_default();
    }
    let identity = match candidate.transition {
        TargetTransition::Remove => index.all_target_to_artifact.get(&candidate.reference),
        TargetTransition::Modify | TargetTransition::RunOnly | TargetTransition::Readonly => {
            index.target_to_artifact.get(&candidate.reference)
        }
        TargetTransition::Add => unreachable!("Add budgets return above"),
    };
    identity
        .and_then(|identity| {
            index
                .artifact_units
                .iter()
                .find(|unit| &unit.identity == identity)
        })
        .map(|unit| unit.span.byte_end.saturating_sub(unit.span.byte_start))
        .unwrap_or_default()
}

fn suggestion_budget_exceeds(
    budget: &SuggestionBudget,
    limits: &syu_project_model::SliceLimits,
) -> bool {
    budget.editable_files > limits.max_editable_files
        || budget.editable_symbols > limits.max_editable_symbols
        || budget.verification_targets > limits.max_verification_targets
        || budget.readonly_targets > limits.max_readonly_targets
        || budget.total_bytes > limits.max_total_bytes
}

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
    if let Some(error) = &index.inventory_error {
        bail!("inventory failed; plan generation is refused: {error}");
    }
    if let Err(error) = validate_planning_origin(request, index) {
        return Ok(blocked_plan(
            request,
            workspace,
            index,
            revision,
            "SYU-WORK-001",
            format!("exact Work origin is invalid: {error}"),
        ));
    }
    // Force the inventory-backed fingerprint before any blocked/ready plan is
    // serialized. A fallback UI fingerprint must never become an execution
    // basis.
    let _ = workspace.try_fingerprint()?;
    if request.constraints.max_added_bytes_per_target == Some(0)
        || request.constraints.max_added_lines_per_target == Some(0)
    {
        return Ok(blocked_plan(
            request,
            workspace,
            index,
            revision,
            "SYU-WORK-001",
            "add budgets must be greater than zero",
        ));
    }
    let exclude_matcher = compile_exclude_matcher(&request.constraints.exclude_paths)?;
    let mut criteria = BTreeSet::new();
    let origin_criterion = request.origin.criterion().clone();
    if index.anchor(&origin_criterion).is_none() {
        return Ok(blocked_plan(
            request,
            workspace,
            index,
            revision,
            "SYU-WORK-001",
            format!("origin criterion {origin_criterion} does not resolve"),
        ));
    }
    criteria.insert(origin_criterion);
    for requested in &request.requested_targets {
        let reference = requested.reference();
        let transition = requested.transition(default_transition(request.operation));
        if index.target(reference).is_none() {
            return Ok(blocked_plan(
                request,
                workspace,
                index,
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
                index,
                revision,
                "SYU-WORK-001",
                "investigate requests only permit run-only or readonly requested targets",
            ));
        }
    }
    if criteria.is_empty() && request.requested_targets.is_empty() {
        return Ok(blocked_plan(
            request,
            workspace,
            index,
            revision,
            "SYU-WORK-001",
            "origin does not select a criterion",
        ));
    }
    let mut slices = Vec::new();
    if request.requested_targets.is_empty() {
        for criterion in criteria {
            let selected = primary_targets(request, index, &criterion);
            for component in target_components(index, selected) {
                if !request.constraints.include_facets.is_empty()
                    && component.iter().all(|target| {
                        index.bindings.get(&target.binding).is_some_and(|binding| {
                            !request.constraints.include_facets.contains(&binding.facet)
                        })
                    })
                {
                    continue;
                }
                let requested = component
                    .iter()
                    .cloned()
                    .map(|reference| RequestedTarget {
                        reference,
                        criterion: Some(criterion.clone()),
                        transition: default_transition(request.operation),
                    })
                    .collect::<Vec<_>>();
                let slice = build_requested_criterion_slice(
                    request,
                    workspace,
                    index,
                    &criterion,
                    &requested,
                    exclude_matcher.as_ref(),
                )?;
                slices.push(slice);
            }
        }
    } else {
        let grouped = match group_requested_targets(request, index) {
            Ok(grouped) => grouped,
            Err(error) => {
                return Ok(blocked_plan(
                    request,
                    workspace,
                    index,
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
                Some(criterion) => {
                    let slice = build_requested_criterion_slice(
                        request,
                        workspace,
                        index,
                        &criterion,
                        &group.requested,
                        exclude_matcher.as_ref(),
                    )?;
                    slices.push(slice);
                }
                None => {
                    for requested in group.requested {
                        let slice = build_requested_target_slice(
                            request,
                            workspace,
                            index,
                            &requested,
                            exclude_matcher.as_ref(),
                        )?;
                        slices.push(slice);
                    }
                }
            }
        }
    }
    let mut expanded_slices = Vec::new();
    for slice in slices {
        expanded_slices.extend(split_slice_if_needed(request, workspace, index, &slice)?);
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
            basis: basis(workspace, index, revision, &slices),
            execution: PlanExecution::IsolatedSlices,
            request: request.clone(),
            origin_closure: origin_closure(request, index, &slices),
            origin_closure_digest: String::new(),
            canonical_digest: String::new(),
            status: PlanStatus::Blocked,
            slices,
            diagnostics: vec![d],
        }));
    }
    let plan_id = plan_id(request, revision);
    let plan_basis = basis(workspace, index, revision, &slices);
    let preflight = finalize_plan(WorkPlan {
        schema: WORK_PLAN_SCHEMA.into(),
        id: plan_id.clone(),
        basis: plan_basis.clone(),
        execution: PlanExecution::IsolatedSlices,
        request: request.clone(),
        origin_closure: origin_closure(request, index, &slices),
        origin_closure_digest: String::new(),
        canonical_digest: String::new(),
        status: PlanStatus::Ready,
        slices: slices.clone(),
        diagnostics: vec![],
    });
    for slice in &mut slices {
        if slice.blockers.is_empty()
            && let Err(error) = validate_context_pack_budget(
                &preflight.canonical_digest,
                &plan_basis,
                slice,
                workspace,
                index,
            )
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
            index,
            revision,
            "SYU-WORK-002",
            "request produced no execution slices",
        ));
    }
    for slice in &mut slices {
        if request.operation != WorkOperation::Investigate
            && !slice_has_verification_coverage(index, request, slice)
        {
            slice.blockers.push(Diagnostic::error(
                "SYU-WORK-015",
                "every editable target requires exact active verification coverage; approve an exact verification Add target before proceeding",
                "work-plan",
            ));
        }
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
        origin_closure: origin_closure(request, index, &slices),
        origin_closure_digest: String::new(),
        canonical_digest: String::new(),
        status,
        slices,
        diagnostics: vec![],
    }))
}

pub fn plan_probe(
    probe: &PlanProbe,
    workspace: &SpecWorkspace,
    index: &SpecIndex,
    revision: &str,
) -> Result<WorkPlan> {
    let request = WorkRequest {
        schema: WORK_REQUEST_SCHEMA.into(),
        id: format!("probe-{}", probe.criterion.local_id),
        title: "internal readiness probe".into(),
        operation: WorkOperation::Modify,
        origin: WorkOrigin::RequirementCriterion {
            criterion: probe.criterion.clone(),
        },
        constraints: WorkConstraints {
            max_slices: Some(probe.max_slices),
            ..WorkConstraints::default()
        },
        requested_targets: probe
            .requested_targets
            .iter()
            .cloned()
            .map(|reference| RequestedTarget {
                reference,
                criterion: Some(probe.criterion.clone()),
                transition: TargetTransition::Modify,
            })
            .collect(),
    };
    plan(&request, workspace, index, revision)
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
            syu_spec_model::TargetClaim::GeneratedFrom { .. }
            | syu_spec_model::TargetClaim::Exposes { .. } => None,
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
    let mut targets = match &request.origin {
        WorkOrigin::FeatureImplementationBinding { targets, .. } => targets.clone(),
        WorkOrigin::FeatureImplementationTarget { target, .. } => vec![target.clone()],
        WorkOrigin::RequirementCriterion { .. } => match request.operation {
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
        },
    };
    targets.sort();
    targets.dedup();
    targets
}

/// Partition targets by the explicit dependency relations that are allowed to
/// make one execution boundary: a contract, generated-from edge, or a direct
/// generated-by edge.  Unrelated implementations of the same criterion remain
/// independent slices.
fn target_components(
    index: &SpecIndex,
    mut targets: Vec<BoundTargetRef>,
) -> Vec<Vec<BoundTargetRef>> {
    targets.sort();
    targets.dedup();
    let mut components = Vec::new();
    while let Some(frontier_target) = targets.pop() {
        let mut component = vec![frontier_target];
        let mut cursor = 0;
        while cursor < component.len() {
            let current = component[cursor].clone();
            cursor += 1;
            let mut neighbor = 0;
            while neighbor < targets.len() {
                if targets_connected(index, &current, &targets[neighbor]) {
                    component.push(targets.swap_remove(neighbor));
                } else {
                    neighbor += 1;
                }
            }
        }
        component.sort();
        components.push(component);
    }
    components.sort_by(|left, right| left.first().cmp(&right.first()));
    components
}

fn targets_connected(index: &SpecIndex, left: &BoundTargetRef, right: &BoundTargetRef) -> bool {
    let left_contracts = index.contracts_by_target.get(left);
    let right_contracts = index.contracts_by_target.get(right);
    left_contracts.is_some_and(|contracts| {
        right_contracts
            .is_some_and(|other| contracts.iter().any(|contract| other.contains(contract)))
    }) || index
        .generated_from
        .get(left)
        .is_some_and(|sources| sources.contains(right))
        || index
            .generated_from
            .get(right)
            .is_some_and(|sources| sources.contains(left))
        || index
            .generated_by_source
            .get(left)
            .is_some_and(|outputs| outputs.contains(right))
        || index
            .generated_by_source
            .get(right)
            .is_some_and(|outputs| outputs.contains(left))
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

#[allow(dead_code, clippy::too_many_arguments)]
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
            index,
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
        format!("{}: {}", request.title, binding.responsibility),
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
            index,
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
    let implementations = if index
        .criterion_status
        .get(criterion)
        .is_some_and(|status| *status != ItemStatus::Implemented)
    {
        // Planned public-entrypoint contracts are intentionally probed one
        // exact target at a time. Pulling every public symbol into each
        // requested slice would turn a traceability probe into a repository
        // sized readonly closure.
        Vec::new()
    } else {
        index
            .criteria_to_implementation_targets
            .get(criterion)
            .cloned()
            .unwrap_or_default()
    };
    for implementation in implementations {
        if let Some(other) = index.bindings.get(&implementation.binding)
            && let Some(target) = index.target(&implementation)
        {
            readonly.extend(one_target(
                workspace,
                index,
                &implementation,
                other,
                target,
                TargetPlanOptions {
                    policy: target_policy(TargetTransition::Readonly),
                    reason: "Exact implementation context referenced by the selected documentation target.",
                    operation: WorkOperation::Modify,
                    add_budget_bytes: None,
                    add_budget_lines: None,
                    exclude_matcher,
                },
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
        format!("{}: {}", request.title, binding.responsibility),
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
            TargetAccessMode::Readonly | TargetAccessMode::Generated => readonly.push(target),
        }
    }
    let mut contracts = Vec::new();
    let mut anchors = vec![reference.binding.clone()];
    // Dependency closure is a property of the exact requested target, not of
    // whether its binding happens to carry a Satisfies claim. A configuration
    // source may legitimately generate output without itself implementing a
    // requirement criterion.
    let (more_readonly, more_contracts) = contract_readonly_context_for_target(
        workspace,
        index,
        reference,
        exclude_matcher,
        &mut blockers,
    );
    readonly.extend(more_readonly);
    contracts.extend(more_contracts);
    if let Some(criterion) = match requested.criterion().cloned() {
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
    } {
        anchors.push(criterion.clone());
        for planned in &mut verification {
            planned.verification_claim = Some(VerificationClaimRef {
                target: planned.reference.clone(),
                criterion: criterion.clone(),
            });
        }
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
    }
    finalize_requested_slice(
        request,
        workspace,
        index,
        &requested_target_slice_id("requested", std::slice::from_ref(requested)),
        format!("{}: {}", request.title, binding.responsibility),
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
    let mut goal = request.title.clone();
    let requested_transitions = transition_map(requested_targets);
    let exact_scope = request.constraints.exact_scope
        || matches!(
            request.origin,
            WorkOrigin::FeatureImplementationBinding { .. }
                | WorkOrigin::FeatureImplementationTarget { .. }
        );
    for requested in requested_targets {
        let reference = requested.reference();
        let binding = index
            .bindings
            .get(&reference.binding)
            .expect("indexed binding");
        goal = format!("{}: {}", request.title, binding.responsibility);
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
        for mut target in planned {
            if target.access == TargetAccessMode::RunOnly {
                target.verification_claim =
                    planned_criterion
                        .as_ref()
                        .map(|criterion| VerificationClaimRef {
                            target: target.reference.clone(),
                            criterion: criterion.clone(),
                        });
            }
            match target.access {
                TargetAccessMode::Editable => editable.push(target),
                TargetAccessMode::RunOnly => verification.push(target),
                TargetAccessMode::Readonly | TargetAccessMode::Generated => readonly.push(target),
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
    let criterion_is_implemented = index
        .criterion_status
        .get(criterion)
        .is_none_or(|status| *status == ItemStatus::Implemented);
    if criterion_is_implemented && !exact_scope {
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
    }
    let implementations = if criterion_is_implemented && !exact_scope {
        index
            .criteria_to_implementation_targets
            .get(criterion)
            .cloned()
            .unwrap_or_default()
    } else {
        // A planned public-entrypoint criterion is probed per exact requested
        // target. Expanding every public symbol into readonly context would
        // make the probe repository-sized and would not establish an exact
        // public-artifact plan.
        Vec::new()
    };
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
                index,
                &implementation,
                other,
                target,
                TargetPlanOptions {
                    policy: target_policy(TargetTransition::Readonly),
                    reason: "Exact implementation context referenced by the selected criterion.",
                    operation: WorkOperation::Modify,
                    add_budget_bytes: None,
                    add_budget_lines: None,
                    exclude_matcher,
                },
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
    normalize_generated_access(index, &editable, &mut readonly);
    if request.constraints.exact_scope {
        let exact_generated = request
            .constraints
            .exact_generated_targets
            .iter()
            .collect::<BTreeSet<_>>();
        for target in &mut readonly {
            if exact_generated.contains(&target.reference) {
                target.access = TargetAccessMode::Generated;
            }
        }
        if !request.constraints.exact_contracts.is_empty() {
            contracts = request.constraints.exact_contracts.clone();
        }
    }
    drop_readonly_overlaps(&mut readonly, &editable, &verification);
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
    let completion = completion_checks(
        request,
        &editable,
        &verification,
        &contracts,
        workspace.config.validation.preset,
    );
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
                index,
                &implementation,
                other,
                target,
                TargetPlanOptions {
                    policy: target_policy(TargetTransition::Readonly),
                    reason: "Exact implementation context for the selected verification target.",
                    operation: WorkOperation::Modify,
                    add_budget_bytes: None,
                    add_budget_lines: None,
                    exclude_matcher,
                },
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
        format!("{}: {}", request.title, binding.responsibility),
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
    preset: ValidationPreset,
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
            // Verification commands are selected only from the target's
            // explicit runner claim and the configured runner registry. A
            // symbol name is never a command-line API.
            TargetTransition::RunOnly => {}
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
        preset: match preset {
            ValidationPreset::Standard => "standard",
            ValidationPreset::Strict => "strict",
            ValidationPreset::AgentReady => "agent-ready",
        }
        .into(),
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
    drop_readonly_overlaps(&mut readonly, &editable, &verification);
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
    normalize_generated_access(index, &editable, &mut readonly);
    if request.constraints.exact_scope {
        let exact_generated = request
            .constraints
            .exact_generated_targets
            .iter()
            .collect::<BTreeSet<_>>();
        for target in &mut readonly {
            if exact_generated.contains(&target.reference) {
                target.access = TargetAccessMode::Generated;
            }
        }
        if !request.constraints.exact_contracts.is_empty() {
            contracts = request.constraints.exact_contracts.clone();
        }
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
    let completion = completion_checks(
        request,
        &editable,
        &verification,
        &contracts,
        workspace.config.validation.preset,
    );
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

fn normalize_generated_access(
    index: &SpecIndex,
    editable: &[PlannedTarget],
    readonly: &mut [PlannedTarget],
) {
    let editable_sources = editable
        .iter()
        .filter(|target| target.access == TargetAccessMode::Editable)
        .map(|target| target.reference.clone())
        .collect::<BTreeSet<_>>();
    for generated in readonly
        .iter_mut()
        .filter(|target| target.access == TargetAccessMode::Generated)
    {
        if !index
            .generated_from
            .get(&generated.reference)
            .is_some_and(|sources| {
                sources
                    .iter()
                    .any(|source| editable_sources.contains(source))
            })
        {
            generated.access = TargetAccessMode::Readonly;
        }
    }
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

fn slice_has_verification_coverage(
    index: &SpecIndex,
    request: &WorkRequest,
    slice: &ExecutionSlice,
) -> bool {
    if slice.editable_targets.is_empty() {
        return true;
    }
    let criterion = request.origin.criterion();
    let explicit_verification_adds = request
        .requested_targets
        .iter()
        .filter(|requested| {
            requested
                .criterion()
                .is_none_or(|requested| requested == criterion)
                && requested.transition(default_transition(request.operation))
                    == TargetTransition::Add
                && index
                    .bindings
                    .get(&requested.reference().binding)
                    .is_some_and(|binding| binding.role == BindingRole::Verification)
        })
        .map(|requested| requested.reference())
        .collect::<BTreeSet<_>>();
    slice.editable_targets.iter().all(|editable| {
        if explicit_verification_adds.contains(&editable.reference) {
            return true;
        }
        slice.verification_targets.iter().any(|verification| {
            verification
                .verification_claim
                .as_ref()
                .is_some_and(|claim| {
                    claim.criterion == *criterion
                        && claim.target == verification.reference
                        && (index
                            .verification_by_target
                            .get(&editable.reference)
                            .is_some_and(|covered| covered.contains(&verification.reference))
                            || (explicit_verification_adds.contains(&verification.reference)
                                && index
                                    .all_verification_by_target
                                    .get(&editable.reference)
                                    .is_some_and(|covered| {
                                        covered.contains(&verification.reference)
                                    })))
                })
        })
    })
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
    // Resolve the exact verification targets claimed by this criterion. The
    // older binding-level index is intentionally not used here: a verification
    // binding may contain tests for several criteria, and expanding the whole
    // binding would make an unrelated test part of this slice.
    let requested_add = requested_transitions
        .into_iter()
        .flat_map(|transitions| transitions.values())
        .any(|transition| *transition == TargetTransition::Add)
        || requested.is_some_and(|value| {
            requested_transitions.and_then(|transitions| transitions.get(value.reference()))
                == Some(&TargetTransition::Add)
        });
    let verification_refs = if requested_add {
        index
            .all_criteria_to_verification_targets
            .get(criterion)
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
    } else {
        index
            .criteria_to_verification_targets
            .get(criterion)
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
    };
    for reference in verification_refs {
        let Some(binding) = index.bindings.get(&reference.binding) else {
            continue;
        };
        let Some(target) = index.target(reference) else {
            continue;
        };
        let requested_ref = requested.map(|value| value.reference());
        let requested_transition = requested_transitions.and_then(|map| map.get(reference));
        let exact_target = requested_ref == Some(reference)
            || requested_transitions.is_some_and(|map| map.contains_key(reference));
        let requested_verification_add = exact_target
            && requested_transition == Some(&TargetTransition::Add)
            && binding.role == BindingRole::Verification;
        if requested_transition.is_some() && !requested_verification_add {
            continue;
        }
        let missing_target = !index.target_to_artifact.contains_key(reference)
            && target.lifecycle != ArtifactTargetLifecycle::Absent;
        if requested_add && missing_target && !requested_verification_add {
            blockers.push(Diagnostic::error(
                "SYU-WORK-014",
                format!(
                    "verification target {reference} is planned but missing; approve its exact Add target before including it in the slice"
                ),
                target.path.to_string_lossy(),
            ));
            continue;
        }
        let policy = if requested_verification_add {
            target_policy(TargetTransition::RunOnly)
        } else if exact_target {
            requested_policy
        } else {
            target_policy(TargetTransition::RunOnly)
        };
        let claim = VerificationClaimRef {
            target: reference.clone(),
            criterion: criterion.clone(),
        };
        for mut planned in one_target(
            workspace,
            index,
            reference,
            binding,
            target,
            TargetPlanOptions {
                policy,
                reason: "Direct verification of the selected criterion.",
                operation: request.operation,
                add_budget_bytes,
                add_budget_lines,
                exclude_matcher,
            },
            blockers,
        ) {
            planned.verification_claim = Some(claim.clone());
            verification.push(planned);
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
        index,
        requested,
        binding,
        target,
        TargetPlanOptions {
            policy,
            reason,
            operation,
            add_budget_bytes,
            add_budget_lines,
            exclude_matcher,
        },
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
    let (related_targets, generated_targets, contracts) =
        dependency_closure(index, std::slice::from_ref(implementation));
    let mut readonly = Vec::new();
    for reference in related_targets {
        if reference == *implementation {
            continue;
        }
        let Some(binding) = index.bindings.get(&reference.binding) else {
            continue;
        };
        let Some(target) = index.target(&reference) else {
            continue;
        };
        let mut planned = one_target(
            workspace,
            index,
            &reference,
            binding,
            target,
            TargetPlanOptions {
                policy: target_policy(TargetTransition::Readonly),
                reason: if generated_targets.contains(&reference) {
                    "Generated output in the complete derived closure; tools may not write it directly."
                } else if index.generated_from.contains_key(&reference) {
                    "Exact source of a generated artifact in the complete closure."
                } else {
                    "Readonly contract or dependency context in the complete closure."
                },
                operation: WorkOperation::Modify,
                add_budget_bytes: None,
                add_budget_lines: None,
                exclude_matcher,
            },
            blockers,
        );
        if generated_targets.contains(&reference) {
            for target in &mut planned {
                target.access = TargetAccessMode::Generated;
            }
        }
        readonly.extend(planned);
    }
    (readonly, contracts.into_iter().collect())
}

/// Resolve the complete fixed-point dependency closure of exact targets. A
/// generated chain or a contract participant can introduce another generated
/// edge or contract, so one-hop collection is not an executable boundary.
fn dependency_closure(
    index: &SpecIndex,
    roots: &[BoundTargetRef],
) -> (
    BTreeSet<BoundTargetRef>,
    BTreeSet<BoundTargetRef>,
    BTreeSet<SpecAnchor>,
) {
    let mut related_targets = roots.iter().cloned().collect::<BTreeSet<_>>();
    let mut generated_targets = BTreeSet::new();
    let mut contracts = BTreeSet::new();
    let mut queue = roots.to_vec();
    while let Some(current) = queue.pop() {
        for generated in index
            .generated_by_source
            .get(&current)
            .into_iter()
            .flatten()
        {
            if generated_targets.insert(generated.clone()) {
                related_targets.insert(generated.clone());
                queue.push(generated.clone());
            }
        }
        for source in index.generated_from.get(&current).into_iter().flatten() {
            if related_targets.insert(source.clone()) {
                queue.push(source.clone());
            }
        }
        for contract_anchor in index
            .contracts_by_target
            .get(&current)
            .into_iter()
            .flatten()
        {
            if !contracts.insert(contract_anchor.clone()) {
                continue;
            }
            let Some(contract) = index.contracts.get(contract_anchor) else {
                continue;
            };
            for related in std::iter::once(&contract.source).chain(
                contract
                    .participants
                    .iter()
                    .map(|participant| &participant.target),
            ) {
                if related_targets.insert(related.clone()) {
                    queue.push(related.clone());
                }
            }
        }
    }
    (related_targets, generated_targets, contracts)
}
#[allow(clippy::too_many_arguments)]
fn targets(
    workspace: &SpecWorkspace,
    index: &SpecIndex,
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
        exclude_matcher,
    };
    binding
        .targets
        .iter()
        .flat_map(|t| {
            one_target(
                workspace,
                index,
                &BoundTargetRef {
                    binding: anchor.clone(),
                    target_id: t.id.clone(),
                },
                binding,
                t,
                options,
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
    exclude_matcher: Option<&'a GlobSet>,
}

fn one_target(
    workspace: &SpecWorkspace,
    index: &SpecIndex,
    reference: &BoundTargetRef,
    binding: &ArtifactBinding,
    target: &syu_spec_model::ArtifactTarget,
    options: TargetPlanOptions<'_>,
    blockers: &mut Vec<Diagnostic>,
) -> Vec<PlannedTarget> {
    if options
        .exclude_matcher
        .is_some_and(|matcher| matcher.is_match(&target.path))
    {
        return vec![];
    }
    if binding.role == BindingRole::Generated
        && matches!(options.policy.access, TargetAccessMode::Editable)
    {
        let mut diagnostic = Diagnostic::error(
            "SYU-WORK-013",
            format!(
                "generated target {reference} is not directly editable; request one of its generated-from source targets"
            ),
            target.path.to_string_lossy(),
        );
        diagnostic.target = Some(reference.clone());
        blockers.push(diagnostic);
        return vec![];
    }
    if options.policy.transition == TargetTransition::Add
        && matches!(target.lifecycle, ArtifactTargetLifecycle::Absent)
    {
        let mut diagnostic = Diagnostic::error(
            "SYU-TARGET-002",
            format!("target {reference} is declared absent and cannot be planned as an Add"),
            target.path.to_string_lossy(),
        );
        diagnostic.target = Some(reference.clone());
        blockers.push(diagnostic);
        return vec![];
    }
    let has_active_artifact = index.target_to_artifact.contains_key(reference)
        || (target.lifecycle == ArtifactTargetLifecycle::Absent
            && (index.all_target_to_artifact.contains_key(reference)
                || resolve_target_in_workspace(workspace, target).is_ok()));
    let planned_missing_target = !has_active_artifact
        && target.lifecycle != ArtifactTargetLifecycle::Absent
        && index.item_status.get(&reference.binding.item) == Some(&ItemStatus::Planned);
    if !(matches!(options.policy.transition, TargetTransition::Add)
        || has_active_artifact
        || (options.policy.transition == TargetTransition::RunOnly && planned_missing_target))
    {
        let mut d = Diagnostic::error(
            "SYU-TARGET-002",
            format!(
                "target {} does not resolve to one active inventory artifact",
                reference
            ),
            target.path.to_string_lossy(),
        );
        d.target = Some(reference.clone());
        blockers.push(d);
        return vec![];
    }
    if matches!(options.policy.access, TargetAccessMode::Editable)
        && (!selector_supports_editable(&target.selector)
            // A missing semantic node has no current exact span. Creation is
            // intentionally file-scoped; existing operations/pointers may be
            // modified through their exact spans below.
            || (matches!(options.policy.transition, TargetTransition::Add)
                && matches!(target.selector, Selector::Operation { .. } | Selector::JsonPointer { .. })))
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
    let resolved = if enabled_adapters(workspace).contains(&target.adapter) {
        match index
            .target_to_artifact
            .get(reference)
            .and_then(|identity| {
                index
                    .artifact_units
                    .iter()
                    .find(|unit| &unit.identity == identity)
            })
            .map_or_else(
                || Ok(None),
                |unit| resolve_indexed_target(workspace, target, unit),
            ) {
            Ok(Some(resolved)) => Ok(resolved),
            Ok(None) => resolve_target_in_workspace(workspace, target),
            Err(error) => Err(error),
        }
    } else {
        Err(anyhow::anyhow!("adapter {} is disabled", target.adapter))
    };
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
                verification_claim: None,
                artifact_identity: None,
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
                container_content_hash: None,
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
        Err(error) => match options.policy.transition {
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
                let container_content_hash = match approved_container_hash(workspace, target) {
                    Ok(hash) => hash,
                    Err(error) => {
                        let mut d = Diagnostic::error(
                            "SYU-TARGET-003",
                            format!("cannot inspect add target container: {error}"),
                            target.path.to_string_lossy(),
                        );
                        d.target = Some(reference.clone());
                        blockers.push(d);
                        return vec![];
                    }
                };
                vec![declared_target_plan(
                    reference,
                    binding,
                    target,
                    DeclaredTargetPlanOptions {
                        policy: options.policy,
                        reason: options.reason,
                        add_budget_bytes,
                        add_budget_lines,
                        container_content_hash,
                    },
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
            TargetTransition::RunOnly if planned_missing_target => {
                vec![declared_target_plan(
                    reference,
                    binding,
                    target,
                    DeclaredTargetPlanOptions {
                        policy: options.policy,
                        reason: options.reason,
                        add_budget_bytes: 0,
                        add_budget_lines: 0,
                        container_content_hash: None,
                    },
                )]
            }
            TargetTransition::Modify | TargetTransition::RunOnly | TargetTransition::Readonly => {
                let mut d = Diagnostic::error(
                    "SYU-TARGET-002",
                    format!(
                        "target does not resolve: {} ({error})",
                        target.path.to_string_lossy()
                    ),
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
    options: DeclaredTargetPlanOptions<'_>,
) -> PlannedTarget {
    PlannedTarget {
        reference: reference.clone(),
        verification_claim: None,
        artifact_identity: None,
        transition: options.policy.transition,
        lifecycle: options.policy.lifecycle,
        access: options.policy.access,
        resolved_path: target.path.to_string_lossy().into_owned(),
        resolved_selector: ResolvedSelector {
            description: declared_selector(&target.selector).0,
            symbols: declared_selector(&target.selector).1,
        },
        content_hash: String::new(),
        excerpt_hash: String::new(),
        container_content_hash: options.container_content_hash,
        adapter: target.adapter.clone(),
        facet: binding.facet.clone(),
        role: binding.role,
        byte_start: 0,
        byte_end: 0,
        line_start: 0,
        line_end: 0,
        budget_bytes: options.add_budget_bytes,
        budget_lines: Some(options.add_budget_lines),
        reason: options.reason.into(),
    }
}

struct DeclaredTargetPlanOptions<'a> {
    policy: TargetPolicy,
    reason: &'a str,
    add_budget_bytes: usize,
    add_budget_lines: usize,
    container_content_hash: Option<String>,
}

/// A missing semantic target is added to the file state that was reviewed
/// with the plan. A missing file deliberately has no container snapshot: the
/// agent must create it with a no-overwrite precondition instead.
fn approved_container_hash(
    workspace: &SpecWorkspace,
    target: &syu_spec_model::ArtifactTarget,
) -> Result<Option<String>> {
    if matches!(target.selector, Selector::File) {
        return Ok(None);
    }
    let canonical_root = workspace.root.canonicalize()?;
    let path = workspace.root.join(target.path.as_path());
    let ancestor = path
        .ancestors()
        .find(|candidate| candidate.exists())
        .ok_or_else(|| anyhow::anyhow!("target path has no existing workspace ancestor"))?;
    if !ancestor.canonicalize()?.starts_with(&canonical_root) {
        bail!(
            "target path escapes workspace through a symlink: {}",
            path.display()
        );
    }
    let bytes = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(error).with_context(|| format!("read target container {}", path.display()));
        }
    };
    let mut hash = Sha256::new();
    hash.update(bytes);
    Ok(Some(format_sha256(hash.finalize())))
}

fn declared_selector(selector: &Selector) -> (String, Vec<String>) {
    match selector {
        Selector::File => ("file".into(), Vec::new()),
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
    values.sort_by(|a, b| {
        a.reference
            .cmp(&b.reference)
            .then(a.artifact_identity.cmp(&b.artifact_identity))
            .then(a.verification_claim.cmp(&b.verification_claim))
    });
    values.dedup_by(|a, b| {
        a.reference == b.reference
            && a.artifact_identity == b.artifact_identity
            && a.verification_claim == b.verification_claim
    });
}

fn validate_target_access_uniqueness(
    blockers: &mut Vec<Diagnostic>,
    editable: &[PlannedTarget],
    verification: &[PlannedTarget],
    readonly: &[PlannedTarget],
) {
    for target in verification {
        if editable.iter().any(|editable| {
            editable.reference == target.reference
                && editable.transition == TargetTransition::Add
                && target.transition == TargetTransition::RunOnly
                && target.verification_claim.is_some()
        }) {
            // A newly added exact verification target is intentionally both
            // editable (the post-state write) and run-only (the test that
            // executes after that write). These are two phases of one
            // approved target identity, not an ambiguous scope expansion.
            continue;
        }
        if editable
            .iter()
            .any(|editable| editable.reference == target.reference)
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
    for target in readonly {
        if editable
            .iter()
            .chain(verification)
            .any(|other| other.reference == target.reference)
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

fn drop_readonly_overlaps(
    readonly: &mut Vec<PlannedTarget>,
    editable: &[PlannedTarget],
    verification: &[PlannedTarget],
) {
    let active = editable
        .iter()
        .chain(verification)
        .map(|target| &target.reference)
        .collect::<BTreeSet<_>>();
    readonly.retain(|target| !active.contains(&target.reference));
}

fn basis(
    workspace: &SpecWorkspace,
    index: &SpecIndex,
    revision: &str,
    slices: &[ExecutionSlice],
) -> PlanBasis {
    PlanBasis {
        revision: revision.into(),
        workspace_fingerprint: workspace
            .try_fingerprint()
            .expect("plan refuses inventory failures before creating a basis"),
        spec_fingerprint: workspace
            .spec_fingerprint()
            .expect("plan basis requires readable specification inputs"),
        ownership_fingerprint: index.ownership_fingerprint_excluding(
            &slices
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
        ),
        readonly_fingerprint: readonly_targets_fingerprint_for_execution(slices),
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
    index: &SpecIndex,
    revision: &str,
    rule: &str,
    message: impl Into<String>,
) -> WorkPlan {
    finalize_plan(WorkPlan {
        schema: WORK_PLAN_SCHEMA.into(),
        id: plan_id(request, revision),
        basis: basis(workspace, index, revision, &[]),
        execution: PlanExecution::IsolatedSlices,
        request: request.clone(),
        origin_closure: origin_closure(request, index, &[]),
        origin_closure_digest: String::new(),
        canonical_digest: String::new(),
        status: PlanStatus::Blocked,
        slices: vec![],
        diagnostics: vec![Diagnostic::error(rule, message, "work-request")],
    })
}

fn split_slice_if_needed(
    request: &WorkRequest,
    workspace: &SpecWorkspace,
    index: &SpecIndex,
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
    for (part, group) in groups.into_iter().enumerate() {
        let candidate = rebuild_split_slice(
            request,
            workspace,
            index,
            &criterion,
            slice,
            group,
            part + 1,
        )?;
        let mut nested = split_slice_if_needed(request, workspace, index, &candidate)?;
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
    let has_mixed_editable_transitions = slice
        .editable_targets
        .iter()
        .map(|target| format!("{:?}", target.transition))
        .collect::<BTreeSet<_>>()
        .len()
        > 1;
    if slice.acceptance.len() == 1
        && slice.editable_targets.len() > 1
        && has_mixed_editable_transitions
    {
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
    // Verification is assigned per implementation component when the slice
    // is rebuilt. A shared criterion-level test may appear in multiple focused
    // slices, but unrelated verification targets must not keep every slice
    // over the limit.
    if slice.budget.verification_targets > limits.max_verification_targets {
        return true;
    }
    if slice.budget.readonly_targets > limits.max_readonly_targets {
        return true;
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
    index: &SpecIndex,
    criterion: &SpecAnchor,
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
    let (editable_targets, verification_targets, readonly_context, contracts, anchors) = match group
    {
        SliceGroup::Editable(editable) => {
            let verification_targets: Vec<PlannedTarget> = original
                .verification_targets
                .iter()
                .filter(|verification| {
                    editable.iter().any(|target| {
                        index
                            .verification_by_target
                            .get(&target.reference)
                            .is_some_and(|covered| covered.contains(&verification.reference))
                    })
                })
                .cloned()
                .collect();
            let (readonly_context, contracts, anchors) =
                focused_split_closure(index, criterion, original, &editable);
            (
                editable,
                verification_targets,
                readonly_context,
                contracts,
                anchors,
            )
        }
    };
    let completion = completion_checks(
        request,
        &editable_targets,
        &verification_targets,
        &contracts,
        workspace.config.validation.preset,
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
        anchors,
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

/// Keep split candidates semantically focused. The unsplit criterion slice
/// carries the complete origin closure, but a selectable child only needs the
/// exact contract, generated-artifact, and same-binding readonly context that
/// constrains its editable targets.
fn focused_split_closure(
    index: &SpecIndex,
    criterion: &SpecAnchor,
    original: &ExecutionSlice,
    editable: &[PlannedTarget],
) -> (Vec<PlannedTarget>, Vec<SpecAnchor>, Vec<SpecAnchor>) {
    let roots = editable
        .iter()
        .map(|target| target.reference.clone())
        .collect::<Vec<_>>();
    let (related_targets, _generated_targets, related_contracts) =
        dependency_closure(index, &roots);
    let child_editable = editable
        .iter()
        .map(|target| target.reference.clone())
        .collect::<BTreeSet<_>>();
    let mut readonly = original
        .editable_targets
        .iter()
        .filter(|target| {
            related_targets.contains(&target.reference)
                && !child_editable.contains(&target.reference)
        })
        .map(|target| {
            let mut context = target.clone();
            context.access = TargetAccessMode::Readonly;
            context.transition = TargetTransition::Readonly;
            context.lifecycle = TargetLifecycle::Stable;
            context
        })
        .collect::<Vec<_>>();
    readonly.extend(
        original
            .readonly_context
            .iter()
            .filter(|target| related_targets.contains(&target.reference))
            .cloned()
            .collect::<Vec<_>>(),
    );
    readonly.sort_by(|left, right| left.reference.cmp(&right.reference));
    readonly.dedup_by(|left, right| left.reference == right.reference);
    let contracts = original
        .contracts
        .iter()
        .filter(|anchor| related_contracts.contains(*anchor))
        .cloned()
        .collect::<Vec<_>>();
    let mut anchors = vec![criterion.clone()];
    anchors.extend(
        editable
            .iter()
            .map(|target| target.reference.binding.clone()),
    );
    anchors.extend(contracts.iter().cloned());
    anchors.sort();
    anchors.dedup();
    (readonly, contracts, anchors)
}

/// Return the exact closure carried by one selectable execution slice. The
/// plan-level closure may contain sibling slices; a split candidate must show
/// only the boundary that will be replayed when that candidate is selected.
pub fn origin_closure_for_slice(index: &SpecIndex, slice: &ExecutionSlice) -> OriginClosure {
    let mut closure = OriginClosure {
        implementation_targets: slice
            .editable_targets
            .iter()
            .map(|target| target.reference.clone())
            .collect(),
        verification_targets: slice
            .verification_targets
            .iter()
            .map(|target| target.reference.clone())
            .collect(),
        readonly_targets: slice
            .readonly_context
            .iter()
            .map(|target| target.reference.clone())
            .collect(),
        contracts: slice.contracts.clone(),
    };
    loop {
        let before = closure.contracts.len();
        for target in closure
            .implementation_targets
            .iter()
            .chain(closure.verification_targets.iter())
            .chain(closure.readonly_targets.iter())
        {
            if let Some(contracts) = index.contracts_by_target.get(target) {
                closure.contracts.extend(contracts.iter().cloned());
            }
        }
        closure.contracts.sort();
        closure.contracts.dedup();
        for anchor in &closure.contracts {
            let Some(contract) = index.contracts.get(anchor) else {
                continue;
            };
            closure.readonly_targets.push(contract.source.clone());
            closure.readonly_targets.extend(
                contract
                    .participants
                    .iter()
                    .map(|participant| participant.target.clone()),
            );
        }
        closure.readonly_targets.sort();
        closure.readonly_targets.dedup();
        if closure.contracts.len() == before {
            break;
        }
    }
    closure.implementation_targets.sort();
    closure.implementation_targets.dedup();
    closure.verification_targets.sort();
    closure.verification_targets.dedup();
    closure.readonly_targets.sort();
    closure.readonly_targets.dedup();
    closure.contracts.sort();
    closure.contracts.dedup();
    closure
}

pub fn origin_closure_digest(closure: &OriginClosure) -> String {
    digest_json(closure)
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
        &canonical.canonical_digest,
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
    plan_digest: &str,
    basis: &PlanBasis,
    slice: &ExecutionSlice,
    workspace: &SpecWorkspace,
    index: &SpecIndex,
) -> Result<()> {
    let spec_context = slice_spec_context(slice, index);
    let pack = build_context_pack(plan_digest, basis, slice, workspace, index, spec_context)?;
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
    plan_digest: &str,
    basis: &PlanBasis,
    slice: &ExecutionSlice,
    workspace: &SpecWorkspace,
    index: &SpecIndex,
    spec_context: Vec<SpecContextEntry>,
) -> Result<ContextPack> {
    let mut artifact_context = Vec::new();
    // A target can be present in more than one plan section when the exact
    // implementation target is also included as readonly context. Context is
    // keyed by target identity, not by the section that requested it; the
    // section order below intentionally keeps the strongest mode (editable,
    // then verification, then readonly) and avoids serializing the same
    // excerpt twice.
    let mut included: BTreeSet<BoundTargetRef> = BTreeSet::new();
    let mut included_supports: BTreeSet<String> = BTreeSet::new();
    for (mode, targets) in [
        (ContextMode::Editable, &slice.editable_targets),
        (ContextMode::Verification, &slice.verification_targets),
        (ContextMode::Readonly, &slice.readonly_context),
    ] {
        for target in targets {
            if !included.insert(target.reference.clone()) {
                continue;
            }
            let resolved = index
                .target(&target.reference)
                .and_then(|declared| resolve_target_in_workspace(workspace, declared).ok());
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
                        TargetTransition::Add | TargetTransition::RunOnly => {
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
                        && let Some(container) = resolve_target_in_workspace(
                            workspace,
                            &syu_spec_model::ArtifactTarget {
                                id: target.reference.target_id.clone(),
                                adapter: target.adapter.clone(),
                                path: RepoPath::new(target.resolved_path.clone())
                                    .expect("resolved path is a valid repo path"),
                                selector: syu_spec_model::Selector::Marker {
                                    value: "crate".into(),
                                },
                                lifecycle: syu_spec_model::ArtifactTargetLifecycle::Present,
                                claims: vec![],
                            },
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
                        let bytes = workspace.read_bytes(&path).unwrap_or_default();
                        if !bytes.is_empty() {
                            let excerpt = String::from_utf8_lossy(&bytes).into_owned();
                            let mut hash = Sha256::new();
                            hash.update(&bytes);
                            let digest = format_sha256(hash.finalize());
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
        plan_digest: plan_digest.into(),
        slice_id: slice.id.clone(),
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
        bail!(
            "context pack exceeds serialized budget ({} > {})",
            serialized.len(),
            workspace.config.work.slicing.max_total_bytes
        );
    }
    Ok(())
}

fn finalize_plan(mut plan: WorkPlan) -> WorkPlan {
    if plan.origin_closure_digest.is_empty() {
        plan.origin_closure_digest = digest_json(&plan.origin_closure);
    }
    plan.canonical_digest = work_plan_digest(&plan);
    plan
}

fn origin_closure(
    request: &WorkRequest,
    index: &SpecIndex,
    slices: &[ExecutionSlice],
) -> OriginClosure {
    let mut closure = OriginClosure::default();
    match &request.origin {
        WorkOrigin::RequirementCriterion { criterion } => {
            closure.implementation_targets.extend(
                index
                    .criteria_to_implementation_targets
                    .get(criterion)
                    .into_iter()
                    .flatten()
                    .filter(|target| {
                        index
                            .bindings
                            .get(&target.binding)
                            .is_some_and(|binding| binding.role == BindingRole::Implementation)
                    })
                    .cloned(),
            );
            closure.verification_targets.extend(
                index
                    .criteria_to_verification_targets
                    .get(criterion)
                    .into_iter()
                    .flatten()
                    .filter(|target| {
                        index
                            .bindings
                            .get(&target.binding)
                            .is_some_and(|binding| binding.role == BindingRole::Verification)
                    })
                    .cloned(),
            );
        }
        WorkOrigin::FeatureImplementationBinding { targets, .. } => {
            closure
                .implementation_targets
                .extend(targets.iter().cloned());
        }
        WorkOrigin::FeatureImplementationTarget { target, .. } => {
            closure.implementation_targets.push(target.clone());
        }
    }
    for slice in slices {
        closure.implementation_targets.extend(
            slice
                .editable_targets
                .iter()
                .map(|target| target.reference.clone()),
        );
        closure.verification_targets.extend(
            slice
                .verification_targets
                .iter()
                .map(|target| target.reference.clone()),
        );
        closure.readonly_targets.extend(
            slice
                .readonly_context
                .iter()
                .map(|target| target.reference.clone()),
        );
        closure.contracts.extend(slice.contracts.iter().cloned());
    }
    loop {
        let before = closure.contracts.len();
        for target in closure
            .implementation_targets
            .iter()
            .chain(closure.verification_targets.iter())
            .chain(closure.readonly_targets.iter())
        {
            if let Some(contracts) = index.contracts_by_target.get(target) {
                closure.contracts.extend(contracts.iter().cloned());
            }
        }
        closure.contracts.sort();
        closure.contracts.dedup();
        for anchor in &closure.contracts {
            let Some(contract) = index.contracts.get(anchor) else {
                continue;
            };
            closure.readonly_targets.push(contract.source.clone());
            closure.readonly_targets.extend(
                contract
                    .participants
                    .iter()
                    .map(|participant| participant.target.clone()),
            );
        }
        closure.readonly_targets.sort();
        closure.readonly_targets.dedup();
        if closure.contracts.len() == before {
            break;
        }
    }
    closure.implementation_targets.sort();
    closure.implementation_targets.dedup();
    closure.verification_targets.sort();
    closure.verification_targets.dedup();
    closure.readonly_targets.sort();
    closure.readonly_targets.dedup();
    closure.contracts.sort();
    closure.contracts.dedup();
    closure
}

fn digest_json<T: Serialize>(value: &T) -> String {
    let bytes = syu_work_model::canonical_json_bytes(
        serde_json::to_value(value).expect("serialize canonical work digest input"),
    );
    let mut hash = Sha256::new();
    hash.update(b"syu/work-origin-closure-digest/v1\0");
    hash.update(bytes);
    format_sha256(hash.finalize())
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
                "      max_slices_per_origin: 4\n",
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
    fn exact_scope_replan_keeps_one_candidate_boundary() {
        let tempdir = tempdir().expect("tempdir");
        write_minimal_workspace(tempdir.path());
        let feature_path = tempdir.path().join("spec/feature.yaml");
        let mut feature = fs::read_to_string(&feature_path).expect("feature spec");
        feature.push_str(concat!(
            "      - id: impl-two\n",
            "        role: implementation\n",
            "        facet: frontend\n",
            "        responsibility: Implement the second target.\n",
            "        targets:\n",
            "          - id: other\n",
            "            adapter: rust\n",
            "            path: src/other.rs\n",
            "            selector: { kind: symbol, name: other }\n",
            "            claims:\n",
            "              - kind: satisfies\n",
            "                criterion: REQ-TEST-001#criterion.test\n",
        ));
        fs::write(feature_path, feature).expect("two implementation targets");
        fs::write(
            tempdir.path().join("src/handler.rs"),
            "pub fn handler() {}\n",
        )
        .expect("first target");
        fs::write(tempdir.path().join("src/other.rs"), "pub fn other() {}\n")
            .expect("second target");

        let workspace = SpecWorkspace::load(tempdir.path()).expect("workspace");
        let index = workspace.index().expect("index");
        let criterion: SpecAnchor = "REQ-TEST-001#criterion.test".parse().unwrap();
        let request = WorkRequest {
            schema: WORK_REQUEST_SCHEMA.into(),
            id: "WORK-EXACT-SCOPE".into(),
            title: "select one implementation boundary".into(),
            operation: WorkOperation::Modify,
            origin: WorkOrigin::RequirementCriterion {
                criterion: criterion.clone(),
            },
            constraints: WorkConstraints::default(),
            requested_targets: vec![],
        };
        let candidate_plan =
            plan(&request, &workspace, &index, "revision").expect("candidate plan");
        assert_eq!(candidate_plan.status, PlanStatus::Blocked);
        assert_eq!(candidate_plan.slices.len(), 2);
        let candidate = candidate_plan.slices.first().expect("candidate slice");

        let requested_targets = candidate
            .editable_targets
            .iter()
            .chain(candidate.verification_targets.iter())
            .chain(candidate.readonly_context.iter())
            .map(|target| RequestedTarget {
                reference: target.reference.clone(),
                criterion: Some(criterion.clone()),
                transition: target.transition,
            })
            .collect();
        let selected_request = WorkRequest {
            constraints: WorkConstraints {
                exact_scope: true,
                max_slices: Some(1),
                ..request.constraints.clone()
            },
            requested_targets,
            ..request
        };
        let selected_plan =
            plan(&selected_request, &workspace, &index, "revision").expect("selected plan");
        assert_eq!(selected_plan.status, PlanStatus::Blocked);
        assert_eq!(selected_plan.slices.len(), 1);
        assert_eq!(
            selected_plan.slices[0]
                .editable_targets
                .iter()
                .map(|target| target.reference.clone())
                .collect::<Vec<_>>(),
            candidate
                .editable_targets
                .iter()
                .map(|target| target.reference.clone())
                .collect::<Vec<_>>()
        );
        assert_eq!(
            selected_plan.slices[0]
                .readonly_context
                .iter()
                .map(|target| target.reference.clone())
                .collect::<Vec<_>>(),
            candidate
                .readonly_context
                .iter()
                .map(|target| target.reference.clone())
                .collect::<Vec<_>>()
        );
        assert!(
            selected_plan.slices[0]
                .editable_targets
                .iter()
                .all(|target| target.reference == candidate.editable_targets[0].reference)
        );
    }

    #[test]
    fn serialized_request_cannot_cross_requirement_origin() {
        let tempdir = tempdir().expect("tempdir");
        write_minimal_workspace(tempdir.path());
        let feature_path = tempdir.path().join("spec/feature.yaml");
        let mut feature = fs::read_to_string(&feature_path).expect("feature spec");
        feature.push_str(concat!(
            "      - id: other-impl\n",
            "        role: implementation\n",
            "        facet: unrelated\n",
            "        responsibility: Implement another criterion.\n",
            "        targets:\n",
            "          - id: other\n",
            "            adapter: rust\n",
            "            path: src/other.rs\n",
            "            selector: { kind: symbol, name: other }\n",
            "            claims:\n",
            "              - kind: satisfies\n",
            "                criterion: REQ-OTHER-001#criterion.other\n",
        ));
        fs::write(feature_path, feature).expect("feature with unrelated binding");
        let requirement_path = tempdir.path().join("spec/requirement.yaml");
        let mut requirement = fs::read_to_string(&requirement_path).expect("requirements");
        requirement.push_str(concat!(
            "  - id: REQ-OTHER-001\n",
            "    title: Other requirement\n",
            "    description: Other requirement.\n",
            "    priority: low\n",
            "    status: implemented\n",
            "    criteria:\n",
            "      - id: other\n",
            "        kind: behavior\n",
            "        statement: Other behavior.\n",
            "        governed_by: []\n",
        ));
        fs::write(requirement_path, requirement).expect("requirements with unrelated criterion");
        fs::write(tempdir.path().join("src/other.rs"), "pub fn other() {}\n")
            .expect("unrelated source");

        let workspace = SpecWorkspace::load(tempdir.path()).expect("workspace");
        let index = workspace.index().expect("index");
        let request = WorkRequest {
            schema: WORK_REQUEST_SCHEMA.into(),
            id: "WORK-CROSS-ORIGIN".into(),
            title: "Cross origin request".into(),
            operation: WorkOperation::Modify,
            origin: WorkOrigin::RequirementCriterion {
                criterion: "REQ-TEST-001#criterion.test".parse().unwrap(),
            },
            constraints: WorkConstraints::default(),
            requested_targets: vec![RequestedTarget {
                reference: "FEAT-TEST-001#binding.other-impl/target.other"
                    .parse()
                    .unwrap(),
                criterion: Some("REQ-OTHER-001#criterion.other".parse().unwrap()),
                transition: TargetTransition::Modify,
            }],
        };
        let wire = serde_yaml::to_string(&request).expect("serialize request");
        let roundtrip: WorkRequest = serde_yaml::from_str(&wire).expect("deserialize request");
        assert!(roundtrip.requested_targets[0].criterion.is_none());
        let plan = plan(&roundtrip, &workspace, &index, "cross-origin").expect("plan");
        assert_eq!(plan.status, PlanStatus::Blocked);
        assert!(plan.diagnostics.iter().any(|diagnostic| {
            diagnostic
                .message
                .contains("outside the exact editable origin closure")
        }));
    }

    #[test]
    fn target_suggestions_rank_exact_claims_with_reviewable_evidence() {
        let tempdir = tempdir().expect("tempdir");
        write_minimal_workspace(tempdir.path());
        fs::write(
            tempdir.path().join("src/handler.rs"),
            "pub fn handler() {}\n",
        )
        .expect("handler file");
        let workspace = SpecWorkspace::load(tempdir.path()).expect("workspace");
        let index = workspace.index().expect("index");
        let criterion: SpecAnchor = "REQ-TEST-001#criterion.test".parse().unwrap();

        let suggestions = suggest_targets(&criterion, &workspace, &index).expect("suggestions");

        assert_eq!(suggestions.suggestions.len(), 1);
        let candidate = &suggestions.suggestions[0];
        assert_eq!(candidate.rank, 1);
        assert_eq!(
            candidate.reference.to_string(),
            "FEAT-TEST-001#binding.impl/target.handler-present"
        );
        assert_eq!(candidate.confidence, SuggestionConfidence::High);
        assert!(
            candidate
                .evidence
                .iter()
                .any(|item| item.contains("explicitly claims"))
        );
        assert!(!candidate.evidence_fingerprint.is_empty());
        assert!(!suggestions.suggestion_token.is_empty());
        assert!(suggestions.split_recommendation.is_none());
    }

    #[test]
    fn planned_missing_exact_target_is_an_add_suggestion_with_scope_metadata() {
        let tempdir = tempdir().expect("tempdir");
        write_minimal_workspace(tempdir.path());
        let feature_path = tempdir.path().join("spec/feature.yaml");
        let feature = fs::read_to_string(&feature_path)
            .expect("feature spec")
            .replace("status: implemented", "status: planned")
            .replace("name: handler", "name: handler_missing");
        fs::write(feature_path, feature).expect("planned feature spec");
        fs::write(
            tempdir.path().join("src/handler.rs"),
            "pub fn existing_handler() {}\n",
        )
        .expect("existing container");
        let workspace = SpecWorkspace::load(tempdir.path()).expect("workspace");
        let index = workspace.index().expect("index");
        let criterion: SpecAnchor = "REQ-TEST-001#criterion.test".parse().unwrap();

        let suggestions = suggest_targets(&criterion, &workspace, &index).expect("suggestions");
        let candidate = suggestions.suggestions.first().expect("add suggestion");

        assert_eq!(candidate.transition, TargetTransition::Add);
        assert_eq!(candidate.lifecycle, TargetLifecycle::EnsurePresent);
        assert_eq!(candidate.path, "src/handler.rs");
        assert_eq!(candidate.selector, "handler_missing");
        assert!(candidate.existing_file);
        assert_eq!(candidate.budget_bytes, Some(512));
        assert_eq!(candidate.budget_lines, Some(32));
        assert!(
            candidate
                .evidence
                .iter()
                .any(|item| item.contains("planned target"))
        );
    }

    #[test]
    fn planned_absent_exact_target_is_a_remove_suggestion_when_artifact_exists() {
        let tempdir = tempdir().expect("tempdir");
        write_minimal_workspace(tempdir.path());
        let feature_path = tempdir.path().join("spec/feature.yaml");
        let feature = fs::read_to_string(&feature_path)
            .expect("feature spec")
            .replace(
                "          - id: handler-missing\n",
                concat!(
                    "          - id: handler-remove\n",
                    "            adapter: rust\n",
                    "            path: src/handler.rs\n",
                    "            selector: { kind: symbol, name: handler }\n",
                    "            lifecycle: absent\n",
                    "            claims:\n",
                    "              - kind: satisfies\n",
                    "                criterion: REQ-TEST-001#criterion.test\n",
                    "          - id: handler-missing\n",
                ),
            )
            .replace("status: implemented", "status: planned");
        fs::write(feature_path, feature).expect("remove target spec");
        fs::write(
            tempdir.path().join("src/handler.rs"),
            "pub fn handler() {}\n",
        )
        .expect("handler file");
        let workspace = SpecWorkspace::load(tempdir.path()).expect("workspace");
        let index = workspace.index().expect("index");
        let criterion: SpecAnchor = "REQ-TEST-001#criterion.test".parse().unwrap();

        let suggestions = suggest_targets(&criterion, &workspace, &index).expect("suggestions");
        let candidate = suggestions
            .suggestions
            .iter()
            .find(|candidate| candidate.reference.target_id.to_string() == "handler-remove")
            .expect("planned remove suggestion");
        assert_eq!(candidate.transition, TargetTransition::Remove);
        assert_eq!(candidate.lifecycle, TargetLifecycle::EnsureAbsent);
        assert_eq!(candidate.path, "src/handler.rs");
        assert!(candidate.existing_file);
        assert!(
            candidate
                .evidence
                .iter()
                .any(|item| item.contains("ensure-absent"))
        );
    }

    #[test]
    fn requested_add_verification_target_is_repeated_as_post_state_run_only() {
        let tempdir = tempdir().expect("tempdir");
        write_minimal_workspace(tempdir.path());
        let feature_path = tempdir.path().join("spec/feature.yaml");
        let feature = fs::read_to_string(&feature_path).expect("feature spec")
            + concat!(
                "\n  - id: FEAT-TEST-VERIFICATION-001\n",
                "    title: Planned verification\n",
                "    summary: Add an exact verification target after approval.\n",
                "    status: planned\n",
                "    bindings:\n",
                "      - id: verification\n",
                "        role: verification\n",
                "        facet: verification\n",
                "        responsibility: Verify the test criterion.\n",
                "        targets:\n",
                "          - id: missing-test\n",
                "            adapter: rust\n",
                "            path: src/handler.rs\n",
                "            selector: { kind: symbol, name: added_handler }\n",
                "            claims:\n",
                "              - kind: verifies\n",
                "                criterion: REQ-TEST-001#criterion.test\n",
                "                covers: []\n",
                "                runner: { runner: cargo-test, arguments: { package: sample, test: added_handler } }\n",
            );
        fs::write(feature_path, feature).expect("verification target spec");
        fs::write(
            tempdir.path().join("src/handler.rs"),
            "pub fn handler() {}\n",
        )
        .expect("handler file");
        let workspace = SpecWorkspace::load(tempdir.path()).expect("workspace");
        let index = workspace.index().expect("index");
        let criterion: SpecAnchor = "REQ-TEST-001#criterion.test".parse().unwrap();
        let reference: BoundTargetRef =
            "FEAT-TEST-VERIFICATION-001#binding.verification/target.missing-test"
                .parse()
                .unwrap();
        let request = WorkRequest {
            schema: WORK_REQUEST_SCHEMA.into(),
            id: "WORK-TEST-VERIFICATION-ADD".into(),
            title: "Add a verification test".into(),
            operation: WorkOperation::Add,
            origin: WorkOrigin::RequirementCriterion {
                criterion: "REQ-TEST-001#criterion.test".parse().unwrap(),
            },
            constraints: WorkConstraints {
                max_added_bytes_per_target: Some(512),
                max_added_lines_per_target: Some(32),
                ..Default::default()
            },
            requested_targets: vec![RequestedTarget {
                reference,
                criterion: Some(criterion),
                transition: TargetTransition::Add,
            }],
        };

        let plan = plan(&request, &workspace, &index, "rev-verification-add").expect("plan");
        assert_eq!(plan.status, PlanStatus::Ready, "{plan:?}");
        let slice = plan.slices.first().expect("verification add slice");
        assert!(slice.editable_targets.iter().any(|target| {
            target.reference.target_id.to_string() == "missing-test"
                && target.transition == TargetTransition::Add
                && target.access == TargetAccessMode::Editable
        }));
        let verification = slice
            .verification_targets
            .iter()
            .find(|target| target.reference.target_id.to_string() == "missing-test")
            .expect("new test is also a verification target");
        assert_eq!(verification.access, TargetAccessMode::RunOnly);
        assert_eq!(verification.transition, TargetTransition::RunOnly);
        assert_eq!(
            verification.verification_claim.as_ref().unwrap().criterion,
            "REQ-TEST-001#criterion.test".parse().unwrap()
        );
    }

    #[test]
    fn implementation_add_does_not_pull_unapproved_missing_verification_target() {
        let tempdir = tempdir().expect("tempdir");
        write_minimal_workspace(tempdir.path());
        let feature_path = tempdir.path().join("spec/feature.yaml");
        let mut feature = fs::read_to_string(&feature_path).expect("feature spec");
        feature.push_str(concat!(
            "\n  - id: FEAT-TEST-VERIFICATION-001\n",
            "    title: Planned verification\n",
            "    summary: Add a verification target only after its exact approval.\n",
            "    status: planned\n",
            "    bindings:\n",
            "      - id: verification\n",
            "        role: verification\n",
            "        facet: verification\n",
            "        responsibility: Verify the test criterion.\n",
            "        targets:\n",
            "          - id: missing-test\n",
            "            adapter: rust\n",
            "            path: tests/behavior.rs\n",
            "            selector: { kind: symbol, name: missing_test }\n",
            "            claims:\n",
            "              - kind: verifies\n",
            "                criterion: REQ-TEST-001#criterion.test\n",
            "                covers: [FEAT-TEST-001#binding.impl/target.handler-present]\n",
            "                runner: { runner: cargo-test, arguments: { package: sample, test: missing_test } }\n",
        ));
        fs::write(feature_path, feature).expect("verification feature spec");
        fs::write(
            tempdir.path().join("src/handler.rs"),
            "pub fn existing_handler() {}\n",
        )
        .expect("existing container");
        let workspace = SpecWorkspace::load(tempdir.path()).expect("workspace");
        let index = workspace.index().expect("index");
        let criterion: SpecAnchor = "REQ-TEST-001#criterion.test".parse().unwrap();
        let request = WorkRequest {
            schema: WORK_REQUEST_SCHEMA.into(),
            id: "WORK-TEST-UNAPPROVED-VERIFICATION".into(),
            title: "Add the implementation target".into(),
            operation: WorkOperation::Add,
            origin: WorkOrigin::RequirementCriterion {
                criterion: "REQ-TEST-001#criterion.test".parse().unwrap(),
            },
            constraints: WorkConstraints {
                max_added_bytes_per_target: Some(512),
                max_added_lines_per_target: Some(32),
                ..Default::default()
            },
            requested_targets: vec![RequestedTarget {
                reference: "FEAT-TEST-001#binding.impl/target.handler-missing"
                    .parse()
                    .unwrap(),
                criterion: Some(criterion),
                transition: TargetTransition::Add,
            }],
        };

        let plan = plan(&request, &workspace, &index, "rev-unapproved-verification").expect("plan");
        assert_eq!(plan.status, PlanStatus::Blocked, "{plan:?}");
        assert!(plan.slices.iter().all(|slice| {
            slice
                .verification_targets
                .iter()
                .all(|target| target.reference.target_id.to_string() != "missing-test")
        }));
        assert!(
            plan.diagnostics
                .iter()
                .any(|diagnostic| diagnostic.rule_id == "SYU-WORK-001")
        );
    }

    #[test]
    fn implemented_missing_exact_target_is_not_reframed_as_add() {
        let tempdir = tempdir().expect("tempdir");
        write_minimal_workspace(tempdir.path());
        fs::write(tempdir.path().join("src/handler.rs"), "pub fn other() {}\n")
            .expect("drifted artifact");
        let workspace = SpecWorkspace::load(tempdir.path()).expect("workspace");
        let index = workspace.index().expect("workspace index");
        let criterion: SpecAnchor = "REQ-TEST-001#criterion.test".parse().unwrap();

        let suggestions = suggest_targets(&criterion, &workspace, &index).expect("suggestions");
        assert!(suggestions.suggestions.is_empty());
    }

    #[test]
    fn target_suggestions_recommend_split_when_candidate_budget_overflows() {
        let tempdir = tempdir().expect("tempdir");
        write_minimal_workspace(tempdir.path());
        fs::write(
            tempdir.path().join("src/handler.rs"),
            "pub fn handler() {}\n",
        )
        .expect("handler file");
        let mut workspace = SpecWorkspace::load(tempdir.path()).expect("workspace");
        workspace.config.work.slicing.max_editable_symbols = 0;
        let index = workspace.index().expect("index");
        let criterion: SpecAnchor = "REQ-TEST-001#criterion.test".parse().unwrap();

        let suggestions = suggest_targets(&criterion, &workspace, &index).expect("suggestions");

        let split = suggestions
            .split_recommendation
            .expect("split recommendation");
        assert!(split.reason.contains("exceeds configured slicing limits"));
        assert_eq!(split.suggested_groups.len(), 1);
    }

    #[test]
    fn missing_add_targets_split_by_declared_budget_and_file_limit() {
        let tempdir = tempdir().expect("tempdir");
        write_minimal_workspace(tempdir.path());
        let feature_path = tempdir.path().join("spec/feature.yaml");
        let feature = fs::read_to_string(&feature_path)
            .expect("feature spec")
            .replace("status: implemented", "status: planned")
            .replace(
                "            selector: { kind: symbol, name: handler_missing }\n            claims: []\n",
                "            selector: { kind: symbol, name: handler_missing }\n            claims:\n              - kind: satisfies\n                criterion: REQ-TEST-001#criterion.test\n          - id: handler-missing-two\n            adapter: rust\n            path: src/other.rs\n            selector: { kind: symbol, name: handler_missing_two }\n            claims:\n              - kind: satisfies\n                criterion: REQ-TEST-001#criterion.test\n",
            );
        fs::write(feature_path, feature).expect("two missing targets");
        fs::write(
            tempdir.path().join("src/handler.rs"),
            "pub fn handler() {}\n",
        )
        .expect("handler file");
        let mut workspace = SpecWorkspace::load(tempdir.path()).expect("workspace");
        workspace.config.work.slicing.max_total_bytes = 700;
        workspace.config.work.slicing.max_editable_files = 1;
        let index = workspace.index().expect("index");
        let criterion: SpecAnchor = "REQ-TEST-001#criterion.test".parse().unwrap();
        let suggestions = suggest_targets(&criterion, &workspace, &index).expect("suggestions");
        let adds = suggestions
            .suggestions
            .iter()
            .filter(|candidate| candidate.transition == TargetTransition::Add)
            .cloned()
            .collect::<Vec<_>>();

        assert_eq!(adds.len(), 2);
        assert_eq!(adds[0].budget_bytes, Some(512));
        assert_eq!(adds[1].budget_bytes, Some(512));
        let split = split_work_recommendation(&adds, &workspace, &index)
            .expect("lifecycle candidates require a split");
        assert_eq!(split.suggested_groups.len(), 2);
        assert!(split.suggested_groups.iter().all(|group| group.len() == 1));
    }

    #[test]
    fn verification_add_targets_count_the_post_write_run_only_phase() {
        let tempdir = tempdir().expect("tempdir");
        write_minimal_workspace(tempdir.path());
        let feature_path = tempdir.path().join("spec/feature.yaml");
        let feature = fs::read_to_string(&feature_path)
            .expect("feature spec")
            .replace("status: implemented", "status: planned")
            .replace(
                "            claims: []\n",
                "            claims: []\n      - id: verification\n        role: verification\n        facet: checks\n        responsibility: Verify the planned targets.\n        targets:\n          - id: verify-one\n            adapter: rust\n            path: tests/verify.rs\n            selector: { kind: symbol, name: verify_one }\n            claims:\n              - kind: satisfies\n                criterion: REQ-TEST-001#criterion.test\n          - id: verify-two\n            adapter: rust\n            path: tests/verify.rs\n            selector: { kind: symbol, name: verify_two }\n            claims:\n              - kind: satisfies\n                criterion: REQ-TEST-001#criterion.test\n",
            );
        fs::write(feature_path, feature).expect("verification Add targets");
        fs::write(
            tempdir.path().join("src/handler.rs"),
            "pub fn handler() {}\n",
        )
        .expect("handler file");
        let mut workspace = SpecWorkspace::load(tempdir.path()).expect("workspace");
        workspace.config.work.slicing.max_verification_targets = 1;
        let index = workspace.index().expect("index");
        let criterion: SpecAnchor = "REQ-TEST-001#criterion.test".parse().unwrap();
        let suggestions = suggest_targets(&criterion, &workspace, &index).expect("suggestions");
        let verification_adds = suggestions
            .suggestions
            .iter()
            .filter(|candidate| {
                candidate.transition == TargetTransition::Add
                    && candidate.role == BindingRole::Verification
            })
            .cloned()
            .collect::<Vec<_>>();

        assert_eq!(verification_adds.len(), 2);
        let split = split_work_recommendation(&verification_adds, &workspace, &index)
            .expect("verification Add phases require a split");
        assert_eq!(split.suggested_groups.len(), 2);
        assert!(split.suggested_groups.iter().all(|group| group.len() == 1));
    }

    #[test]
    fn zero_add_budgets_are_rejected() {
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
            id: "WORK-TEST-003".into(),
            title: "Reject empty budgets".into(),
            operation: WorkOperation::Add,
            origin: WorkOrigin::RequirementCriterion {
                criterion: "REQ-TEST-001#criterion.test".parse().unwrap(),
            },
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
            title: "Add a new target".into(),
            operation: WorkOperation::Add,
            origin: WorkOrigin::RequirementCriterion {
                criterion: "REQ-TEST-001#criterion.test".parse().unwrap(),
            },
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
        assert_eq!(plan.status, PlanStatus::Blocked);
        assert!(plan.slices.iter().any(|slice| {
            slice.editable_targets.iter().any(|target| {
                target.transition == TargetTransition::Add
                    && target.lifecycle == TargetLifecycle::EnsurePresent
                    && target.reference.target_id.to_string() == "handler-missing"
            })
        }));
    }

    #[test]
    fn explicit_add_rejects_a_target_declared_absent() {
        let tempdir = tempdir().expect("tempdir");
        write_minimal_workspace(tempdir.path());
        let feature_path = tempdir.path().join("spec/feature.yaml");
        let feature = fs::read_to_string(&feature_path)
            .expect("feature spec")
            .replace(
                "            selector: { kind: symbol, name: handler_missing }\n            claims: []",
                concat!(
                    "            selector: { kind: symbol, name: handler_missing }\n",
                    "            lifecycle: absent\n",
                    "            claims:\n",
                    "              - kind: satisfies\n",
                    "                criterion: REQ-TEST-001#criterion.test",
                ),
            );
        fs::write(feature_path, feature).expect("absent add target");
        fs::write(
            tempdir.path().join("src/handler.rs"),
            "pub fn handler() {}\n",
        )
        .expect("handler file");
        let workspace = SpecWorkspace::load(tempdir.path()).expect("workspace");
        let index = workspace.index().expect("index");
        let request = WorkRequest {
            schema: WORK_REQUEST_SCHEMA.into(),
            id: "WORK-ABSENT-ADD".into(),
            title: "reject absent declaration as add".into(),
            operation: WorkOperation::Add,
            origin: WorkOrigin::RequirementCriterion {
                criterion: "REQ-TEST-001#criterion.test".parse().unwrap(),
            },
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

        let plan = plan(&request, &workspace, &index, "rev-absent-add").expect("plan");
        assert_eq!(plan.status, PlanStatus::Blocked);
        assert!(plan.diagnostics.iter().any(|diagnostic| {
            diagnostic.rule_id == "SYU-WORK-001"
                && diagnostic
                    .message
                    .contains("outside the exact editable origin closure")
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
            tempdir.path().join("src/placeholder.rs"),
            "pub fn placeholder() {}\n",
        )
        .expect("placeholder file");
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
                title: "Add a new target".into(),
                operation: WorkOperation::Add,
                origin: WorkOrigin::RequirementCriterion {
                    criterion: "REQ-TEST-001#criterion.test".parse().unwrap(),
                },
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
            title: "Mixed transition closure".into(),
            operation: WorkOperation::Modify,
            origin: WorkOrigin::RequirementCriterion {
                criterion: "REQ-TEST-001#criterion.test".parse().unwrap(),
            },
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
            title: "Check explicit access modes".into(),
            operation: WorkOperation::Modify,
            origin: WorkOrigin::RequirementCriterion {
                criterion: "REQ-TEST-001#criterion.test".parse().unwrap(),
            },
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
            title: "Check explicit access modes".into(),
            operation: WorkOperation::Modify,
            origin: WorkOrigin::RequirementCriterion {
                criterion: "REQ-TEST-001#criterion.test".parse().unwrap(),
            },
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
            title: "Add a new target".into(),
            operation: WorkOperation::Add,
            origin: WorkOrigin::RequirementCriterion {
                criterion: "REQ-TEST-001#criterion.test".parse().unwrap(),
            },
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
        assert_eq!(plan.status, PlanStatus::Blocked);
        assert!(
            plan.slices
                .iter()
                .flat_map(|slice| &slice.blockers)
                .any(|diagnostic| diagnostic.rule_id == "SYU-WORK-015")
        );
        let slice = plan.slices.first().expect("slice");
        assert!(export_context(&plan, &slice.id, &workspace, &index, "rev-2").is_err());
    }

    fn write_dependency_workspace(root: &Path) {
        fs::create_dir_all(root.join("spec")).expect("spec dir");
        fs::create_dir_all(root.join("src")).expect("src dir");
        fs::create_dir_all(root.join("web")).expect("web dir");
        fs::write(
            root.join("syu.yaml"),
            concat!(
                "schema: syu/config/v1\n",
                "workspace: { spec_roots: [spec], excludes: [] }\n",
                "inventory:\n",
                "  active_profile: default\n",
                "  profiles:\n",
                "    - id: default\n",
                "      providers:\n",
                "        rust: {}\n",
                "        javascript: { roots: [web] }\n",
                "        declared: { roots: [generated.txt] }\n",
                "validation:\n",
                "  preset: standard\n",
                "  readiness:\n",
                "    target: off\n",
                "    limits: { max_ownership_scope_units: 64, max_targets_per_binding: 12, max_slices_per_origin: 4 }\n",
                "  changed: { require_owned_changes: false, require_plan: false }\n",
                "verification: { runners: {} }\n",
                "work:\n",
                "  slicing: { max_editable_files: 4, max_editable_symbols: 8, max_verification_targets: 4, max_readonly_targets: 8, max_total_bytes: 8192 }\n",
            ),
        )
        .expect("config");
        fs::write(
            root.join("spec/requirement.yaml"),
            concat!(
                "schema: syu/spec/v1\n",
                "kind: requirements\n",
                "namespace: dependency\n",
                "category: Dependency\n",
                "requirements:\n",
                "  - id: REQ-DEPENDENCY-001\n",
                "    title: Dependency\n",
                "    description: Keep provider and consumer coherent.\n",
                "    priority: high\n",
                "    status: implemented\n",
                "    criteria:\n",
                "      - id: coherent\n",
                "        kind: behavior\n",
                "        statement: Provider and consumer change coherently.\n",
                "        governed_by: []\n",
            ),
        )
        .expect("requirement");
        fs::write(
            root.join("spec/feature.yaml"),
            concat!(
                "schema: syu/spec/v1\n",
                "kind: features\n",
                "namespace: dependency\n",
                "category: Dependency\n",
                "features:\n",
                "  - id: FEAT-DEPENDENCY-001\n",
                "    title: Dependency planning\n",
                "    summary: Plan cross-language and generated dependencies.\n",
                "    status: implemented\n",
                "    bindings:\n",
                "      - id: provider\n",
                "        role: implementation\n",
                "        facet: backend\n",
                "        responsibility: Provide the API.\n",
                "        targets:\n",
                "          - { id: provider, adapter: rust, path: src/lib.rs, selector: { kind: symbol, name: provider }, claims: [{ kind: satisfies, criterion: REQ-DEPENDENCY-001#criterion.coherent }] }\n",
                "      - id: consumer\n",
                "        role: implementation\n",
                "        facet: frontend\n",
                "        responsibility: Consume the API.\n",
                "        targets:\n",
                "          - { id: consumer, adapter: javascript, path: web/client.js, selector: { kind: symbol, name: consumer }, claims: [{ kind: satisfies, criterion: REQ-DEPENDENCY-001#criterion.coherent }] }\n",
                "      - id: contract-source\n",
                "        role: contract-source\n",
                "        facet: api\n",
                "        responsibility: Define the API contract.\n",
                "        targets:\n",
                "          - { id: contract, adapter: javascript, path: web/client.js, selector: { kind: symbol, name: contract }, claims: [] }\n",
                "      - id: generator\n",
                "        role: implementation\n",
                "        facet: generation\n",
                "        responsibility: Generate the checked-in artifact.\n",
                "        targets:\n",
                "          - { id: source, adapter: rust, path: src/lib.rs, selector: { kind: symbol, name: generate }, claims: [] }\n",
                "      - id: generated\n",
                "        role: generated\n",
                "        facet: generation\n",
                "        responsibility: Record generated output.\n",
                "        targets:\n",
                "          - id: output\n",
                "            adapter: declared\n",
                "            path: generated.txt\n",
                "            selector: { kind: file }\n",
                "            claims:\n",
                "              - kind: generated-from\n",
                "                targets: [FEAT-DEPENDENCY-001#binding.generator/target.source]\n",
                "    contracts:\n",
                "      - id: provider-consumer\n",
                "        kind: function\n",
                "        source: FEAT-DEPENDENCY-001#binding.contract-source/target.contract\n",
                "        participants:\n",
                "          - { target: FEAT-DEPENDENCY-001#binding.provider/target.provider, role: provider }\n",
                "          - { target: FEAT-DEPENDENCY-001#binding.consumer/target.consumer, role: consumer }\n",
                "        guarantees: [REQ-DEPENDENCY-001#criterion.coherent]\n",
            ),
        )
        .expect("feature");
        fs::write(
            root.join("src/lib.rs"),
            "pub fn provider() {}\npub fn generate() {}\n",
        )
        .expect("rust");
        fs::write(
            root.join("web/client.js"),
            "export function consumer() {}\nexport function contract() {}\n",
        )
        .expect("javascript");
        fs::write(root.join("generated.txt"), "generated\n").expect("generated");
    }

    #[test]
    fn cross_language_provider_and_consumer_share_one_contract_slice() {
        let tempdir = tempdir().expect("tempdir");
        write_dependency_workspace(tempdir.path());
        let workspace = SpecWorkspace::load(tempdir.path()).expect("workspace");
        let index = workspace.index().expect("index");
        let request = WorkRequest {
            schema: WORK_REQUEST_SCHEMA.into(),
            id: "WORK-DEPENDENCY-001".into(),
            title: "Change provider and consumer".into(),
            operation: WorkOperation::Modify,
            origin: WorkOrigin::RequirementCriterion {
                criterion: "REQ-DEPENDENCY-001#criterion.coherent".parse().unwrap(),
            },
            constraints: WorkConstraints::default(),
            requested_targets: vec![
                RequestedTarget {
                    reference: "FEAT-DEPENDENCY-001#binding.provider/target.provider"
                        .parse()
                        .unwrap(),
                    criterion: None,
                    transition: TargetTransition::Modify,
                },
                RequestedTarget {
                    reference: "FEAT-DEPENDENCY-001#binding.consumer/target.consumer"
                        .parse()
                        .unwrap(),
                    criterion: None,
                    transition: TargetTransition::Modify,
                },
            ],
        };
        let combined_plan = plan(&request, &workspace, &index, "rev-contract").expect("plan");
        assert_eq!(combined_plan.status, PlanStatus::Blocked);
        assert_eq!(combined_plan.slices.len(), 1);
        let slice = &combined_plan.slices[0];
        assert_eq!(
            slice
                .editable_targets
                .iter()
                .map(|target| target.adapter.as_str())
                .collect::<BTreeSet<_>>(),
            BTreeSet::from(["javascript", "rust"])
        );
        assert!(slice.contracts.iter().any(|contract| {
            contract.to_string() == "FEAT-DEPENDENCY-001#contract.provider-consumer"
        }));
        assert!(slice.readonly_context.iter().any(|target| {
            target.reference.to_string()
                == "FEAT-DEPENDENCY-001#binding.contract-source/target.contract"
        }));
        let editable_refs = slice
            .editable_targets
            .iter()
            .map(|target| &target.reference)
            .collect::<BTreeSet<_>>();
        assert!(
            slice
                .readonly_context
                .iter()
                .all(|target| !editable_refs.contains(&target.reference))
        );

        let provider_only = WorkRequest {
            schema: WORK_REQUEST_SCHEMA.into(),
            id: "WORK-DEPENDENCY-PROVIDER".into(),
            title: "Change only the provider".into(),
            operation: WorkOperation::Modify,
            origin: WorkOrigin::RequirementCriterion {
                criterion: "REQ-DEPENDENCY-001#criterion.coherent".parse().unwrap(),
            },
            constraints: WorkConstraints::default(),
            requested_targets: vec![RequestedTarget {
                reference: "FEAT-DEPENDENCY-001#binding.provider/target.provider"
                    .parse()
                    .unwrap(),
                criterion: None,
                transition: TargetTransition::Modify,
            }],
        };
        let provider_plan =
            plan(&provider_only, &workspace, &index, "rev-provider").expect("provider plan");
        assert_eq!(provider_plan.status, PlanStatus::Blocked);
        assert!(
            provider_plan.slices[0]
                .readonly_context
                .iter()
                .any(|target| {
                    target.reference.to_string()
                        == "FEAT-DEPENDENCY-001#binding.consumer/target.consumer"
                        && target.access == TargetAccessMode::Readonly
                })
        );
    }

    #[test]
    fn criterion_origin_preserves_one_contract_component() {
        let tempdir = tempdir().expect("tempdir");
        write_dependency_workspace(tempdir.path());
        let workspace = SpecWorkspace::load(tempdir.path()).expect("workspace");
        let index = workspace.index().expect("index");
        let criterion_seed = WorkRequest {
            schema: WORK_REQUEST_SCHEMA.into(),
            id: "WORK-CONTRACT-CRITERION".into(),
            title: "Change contract participants".into(),
            operation: WorkOperation::Modify,
            origin: WorkOrigin::RequirementCriterion {
                criterion: "REQ-DEPENDENCY-001#criterion.coherent".parse().unwrap(),
            },
            constraints: WorkConstraints::default(),
            requested_targets: vec![],
        };
        let criterion_plan = plan(&criterion_seed, &workspace, &index, "criterion").unwrap();
        assert_eq!(criterion_plan.status, PlanStatus::Blocked);
        assert_eq!(criterion_plan.slices.len(), 1);
        assert_eq!(criterion_plan.slices[0].editable_targets.len(), 2);
        let provider: BoundTargetRef = "FEAT-DEPENDENCY-001#binding.provider/target.provider"
            .parse()
            .unwrap();
        let consumer: BoundTargetRef = "FEAT-DEPENDENCY-001#binding.consumer/target.consumer"
            .parse()
            .unwrap();

        let changed_seed = WorkRequest {
            schema: WORK_REQUEST_SCHEMA.into(),
            id: "WORK-CONTRACT-CHANGED".into(),
            title: "Change contract participants".into(),
            operation: WorkOperation::Modify,
            origin: WorkOrigin::RequirementCriterion {
                criterion: "REQ-DEPENDENCY-001#criterion.coherent".parse().unwrap(),
            },
            constraints: WorkConstraints::default(),
            requested_targets: vec![],
        };
        let changed_plan = plan(&changed_seed, &workspace, &index, "changed").unwrap();
        assert_eq!(
            changed_plan.status,
            PlanStatus::Blocked,
            "{changed_plan:#?}"
        );
        assert_eq!(changed_plan.slices.len(), 1);
        let editable = &changed_plan.slices[0].editable_targets;
        assert_eq!(editable.len(), 2);
        assert_eq!(
            editable
                .iter()
                .map(|target| target.reference.clone())
                .collect::<BTreeSet<_>>(),
            BTreeSet::from([provider.clone(), consumer.clone()]),
        );
        assert!(
            changed_plan.slices[0]
                .readonly_context
                .iter()
                .all(|readonly| {
                    !editable
                        .iter()
                        .any(|editable| editable.reference == readonly.reference)
                })
        );
    }

    #[test]
    fn changed_unit_rejects_inactive_and_generated_artifacts() {
        let inactive_dir = tempdir().expect("tempdir");
        write_minimal_workspace(inactive_dir.path());
        fs::write(
            inactive_dir.path().join("src/handler.rs"),
            "#[cfg(feature = \"enterprise\")]\npub fn handler() {}\n",
        )
        .unwrap();
        let workspace = SpecWorkspace::load(inactive_dir.path()).expect("workspace");
        let index = workspace.index().expect("index");
        let request = WorkRequest {
            schema: WORK_REQUEST_SCHEMA.into(),
            id: "WORK-INACTIVE-CHANGED".into(),
            title: "Change inactive artifact".into(),
            operation: WorkOperation::Modify,
            origin: WorkOrigin::RequirementCriterion {
                criterion: "REQ-TEST-001#criterion.test".parse().unwrap(),
            },
            constraints: WorkConstraints::default(),
            requested_targets: vec![],
        };
        assert_eq!(
            plan(&request, &workspace, &index, "inactive")
                .unwrap()
                .status,
            PlanStatus::Blocked
        );

        let generated = tempdir().expect("generated tempdir");
        write_dependency_workspace(generated.path());
        let feature = fs::read_to_string(generated.path().join("spec/feature.yaml")).unwrap()
            .replacen("claims:\n              - kind: generated-from", "claims:\n              - { kind: satisfies, criterion: REQ-DEPENDENCY-001#criterion.coherent }\n              - kind: generated-from", 1);
        fs::write(generated.path().join("spec/feature.yaml"), feature).unwrap();
        let workspace = SpecWorkspace::load(generated.path()).expect("workspace");
        let index = workspace.index().expect("index");
        let request = WorkRequest {
            schema: WORK_REQUEST_SCHEMA.into(),
            id: "WORK-GENERATED-CHANGED".into(),
            title: "Change generated artifact".into(),
            operation: WorkOperation::Modify,
            origin: WorkOrigin::RequirementCriterion {
                criterion: "REQ-DEPENDENCY-001#criterion.coherent".parse().unwrap(),
            },
            constraints: WorkConstraints::default(),
            requested_targets: vec![],
        };
        let plan = plan(&request, &workspace, &index, "generated").unwrap();
        assert_eq!(plan.status, PlanStatus::Blocked);
        assert!(
            plan.slices
                .iter()
                .flat_map(|slice| &slice.blockers)
                .any(|diagnostic| diagnostic.rule_id == "SYU-WORK-013")
        );
    }

    #[test]
    fn generated_outputs_are_derived_context_and_never_directly_editable() {
        let tempdir = tempdir().expect("tempdir");
        write_dependency_workspace(tempdir.path());
        let feature = fs::read_to_string(tempdir.path().join("spec/feature.yaml"))
            .expect("feature spec")
            .replacen(
                "          - { id: source, adapter: rust, path: src/lib.rs, selector: { kind: symbol, name: generate }, claims: [] }",
                "          - { id: source, adapter: rust, path: src/lib.rs, selector: { kind: symbol, name: generate }, claims: [{ kind: satisfies, criterion: REQ-DEPENDENCY-001#criterion.coherent }] }",
                1,
            );
        fs::write(tempdir.path().join("spec/feature.yaml"), feature).expect("linked feature");
        let workspace = SpecWorkspace::load(tempdir.path()).expect("workspace");
        let index = workspace.index().expect("index");
        let source_request = WorkRequest {
            schema: WORK_REQUEST_SCHEMA.into(),
            id: "WORK-GENERATED-SOURCE".into(),
            title: "Change generator".into(),
            operation: WorkOperation::Modify,
            origin: WorkOrigin::RequirementCriterion {
                criterion: "REQ-DEPENDENCY-001#criterion.coherent".parse().unwrap(),
            },
            constraints: WorkConstraints::default(),
            requested_targets: vec![RequestedTarget {
                reference: "FEAT-DEPENDENCY-001#binding.generator/target.source"
                    .parse()
                    .unwrap(),
                criterion: None,
                transition: TargetTransition::Modify,
            }],
        };
        let source_plan =
            plan(&source_request, &workspace, &index, "rev-generated").expect("source plan");
        assert_eq!(source_plan.status, PlanStatus::Blocked);
        assert!(source_plan.slices[0].readonly_context.iter().any(|target| {
            target.reference.to_string() == "FEAT-DEPENDENCY-001#binding.generated/target.output"
                && target.access == TargetAccessMode::Generated
        }));
        let output: BoundTargetRef = "FEAT-DEPENDENCY-001#binding.generated/target.output"
            .parse()
            .unwrap();
        let mut exact_request = source_request.clone();
        exact_request.id = "WORK-GENERATED-EXACT".into();
        exact_request.constraints.exact_scope = true;
        exact_request.constraints.exact_generated_targets = vec![output.clone()];
        exact_request.requested_targets.push(RequestedTarget {
            reference: output,
            criterion: None,
            transition: TargetTransition::Readonly,
        });
        let exact_plan = plan(&exact_request, &workspace, &index, "rev-generated-exact")
            .expect("exact generated replan");
        assert_eq!(exact_plan.status, PlanStatus::Blocked);
        assert!(exact_plan.slices[0].readonly_context.iter().any(|target| {
            target.reference.to_string() == "FEAT-DEPENDENCY-001#binding.generated/target.output"
                && target.access == TargetAccessMode::Generated
        }));

        let direct_request = WorkRequest {
            schema: WORK_REQUEST_SCHEMA.into(),
            id: "WORK-GENERATED-DIRECT".into(),
            title: "Change generated output".into(),
            operation: WorkOperation::Modify,
            origin: WorkOrigin::RequirementCriterion {
                criterion: "REQ-DEPENDENCY-001#criterion.coherent".parse().unwrap(),
            },
            constraints: WorkConstraints::default(),
            requested_targets: vec![RequestedTarget {
                reference: "FEAT-DEPENDENCY-001#binding.generated/target.output"
                    .parse()
                    .unwrap(),
                criterion: Some("REQ-DEPENDENCY-001#criterion.coherent".parse().unwrap()),
                transition: TargetTransition::Modify,
            }],
        };
        let direct_plan =
            plan(&direct_request, &workspace, &index, "rev-generated").expect("direct plan");
        assert_eq!(direct_plan.status, PlanStatus::Blocked);
        assert!(direct_plan.diagnostics.iter().any(|diagnostic| {
            diagnostic
                .message
                .contains("outside the exact editable origin closure")
        }));
    }

    #[test]
    fn inactive_build_profile_target_never_enters_executable_scope() {
        let tempdir = tempdir().expect("tempdir");
        write_minimal_workspace(tempdir.path());
        fs::write(
            tempdir.path().join("src/handler.rs"),
            "#[cfg(feature = \"enterprise\")]\npub fn handler() {}\n",
        )
        .expect("handler");
        let workspace = SpecWorkspace::load(tempdir.path()).expect("workspace");
        let index = workspace.index().expect("index");
        assert!(index.artifact_units.iter().any(|unit| {
            unit.identity.contains("handler")
                && matches!(
                    unit.reachability,
                    syu_inventory::ArtifactReachability::Conditional { .. }
                )
        }));
        let request = WorkRequest {
            schema: WORK_REQUEST_SCHEMA.into(),
            id: "WORK-INACTIVE".into(),
            title: "Change inactive target".into(),
            operation: WorkOperation::Modify,
            origin: WorkOrigin::RequirementCriterion {
                criterion: "REQ-TEST-001#criterion.test".parse().unwrap(),
            },
            constraints: WorkConstraints::default(),
            requested_targets: vec![RequestedTarget {
                reference: "FEAT-TEST-001#binding.impl/target.handler-present"
                    .parse()
                    .unwrap(),
                criterion: None,
                transition: TargetTransition::Modify,
            }],
        };
        let plan = plan(&request, &workspace, &index, "rev-inactive").expect("plan");
        assert_eq!(plan.status, PlanStatus::Blocked);
        assert!(
            plan.slices[0]
                .blockers
                .iter()
                .any(|diagnostic| diagnostic.rule_id == "SYU-TARGET-002")
        );
        assert!(plan.slices[0].editable_targets.is_empty());
    }
}
