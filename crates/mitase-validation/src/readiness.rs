use anyhow::{Context, Result, bail};
use mitase_project_model::{ProjectConfig, ReadinessLevel};
use mitase_spec_model::{BoundTargetRef, ItemStatus, OwnershipSelector, SpecAnchor, format_sha256};
use mitase_work_model::{
    CompletionAttempt, CompletionStatus, FINALIZATION_RECEIPT_SCHEMA, FinalizationReceipt,
    PLAN_APPROVAL_SCHEMA, PlanApproval, PlanStatus, TargetLifecycle, TargetTransition,
    VERIFICATION_RECEIPT_SCHEMA, VerificationAttemptStatus, VerificationClaimRef,
    VerificationReceipt, work_plan_digest,
};
use mitase_workspace::{SpecIndex, SpecWorkspace};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::process::Command;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

/// A readiness count is a count of these subjects, never a subtraction of
/// blocker strings. This keeps one subject with several blockers from being
/// counted several times.
#[derive(Debug, Clone, Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct ReadinessSubject {
    pub id: String,
    /// Canonical configuration identity for the subject. Readiness gates
    /// compare this identity as well as the level, so equal levels assigned
    /// to different probes cannot satisfy each other accidentally.
    pub scope_id: String,
    pub required_level: ReadinessLevel,
    pub ready: bool,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct ReadinessAxis {
    pub required: usize,
    pub ready: usize,
    pub blockers: Vec<String>,
    pub subjects: Vec<ReadinessSubject>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ReadinessReport {
    pub target: ReadinessLevel,
    pub inventory: ReadinessAxis,
    pub ownership: ReadinessAxis,
    pub seedability: ReadinessAxis,
    pub workability: ReadinessAxis,
    pub verification: ReadinessAxis,
    pub closed_loop: ReadinessAxis,
    pub execution_state: String,
    pub receipts: Vec<VerificationReceipt>,
}

impl ReadinessAxis {
    pub fn empty(message: impl Into<String>) -> Self {
        axis_from_subjects(vec![ReadinessSubject {
            id: "axis-empty".into(),
            scope_id: repository_scope(),
            required_level: ReadinessLevel::Traceable,
            ready: false,
            blockers: vec![message.into()],
        }])
    }

    pub fn is_ready(&self) -> bool {
        self.required > 0 && self.ready == self.required && self.blockers.is_empty()
    }
}

impl ReadinessReport {
    pub fn meets(&self, level: ReadinessLevel) -> bool {
        required_axes(level).iter().all(|axis| match axis {
            ReadinessAxisId::Inventory => self.inventory.is_ready(),
            ReadinessAxisId::Ownership => self.ownership.is_ready(),
            ReadinessAxisId::Seedability => self.seedability.is_ready(),
            ReadinessAxisId::Workability => self.workability.is_ready(),
            ReadinessAxisId::Verification => self.verification.is_ready(),
            ReadinessAxisId::ClosedLoop => self.closed_loop.is_ready(),
        })
    }

    /// Evaluate the workspace-wide target and every typed probe by its exact
    /// configuration identity. A subject from another criterion or probe can
    /// never satisfy the configured requirement merely because its level is
    /// the same.
    pub fn meets_configured(&self, config: &ProjectConfig) -> bool {
        if !self.meets(config.validation.readiness.target) {
            return false;
        }
        let readiness = &config.validation.readiness;
        readiness
            .probes
            .implemented_criteria
            .iter()
            .all(|probe| self.meets_scope(&criterion_scope(&probe.criterion), probe.level))
            && readiness
                .probes
                .public_entrypoints
                .as_ref()
                .is_none_or(|probe| {
                    matches!(
                        probe.level,
                        ReadinessLevel::Off
                            | ReadinessLevel::Seedable
                            | ReadinessLevel::WorkReady
                            | ReadinessLevel::Verifiable
                            | ReadinessLevel::ClosedLoop
                    ) && self.meets_scope(public_entrypoints_scope(), probe.level)
                })
            && readiness
                .probes
                .contracts
                .as_ref()
                .is_none_or(|probe| self.meets_scope(contracts_scope(), probe.level))
    }

    fn meets_scope(&self, scope_id: &str, level: ReadinessLevel) -> bool {
        if level == ReadinessLevel::Off {
            return true;
        }
        required_axes(level).iter().all(|axis| match axis {
            // Structural invariants are repository-wide by design and must
            // never be reduced to the maturity probe denominator.
            ReadinessAxisId::Inventory => self.inventory.is_ready(),
            ReadinessAxisId::Ownership => self.ownership.is_ready(),
            ReadinessAxisId::Seedability => {
                scoped_axis_is_ready(&self.seedability, scope_id, level)
            }
            ReadinessAxisId::Workability => {
                scoped_axis_is_ready(&self.workability, scope_id, level)
            }
            ReadinessAxisId::Verification => {
                scoped_axis_is_ready(&self.verification, scope_id, level)
            }
            ReadinessAxisId::ClosedLoop => scoped_axis_is_ready(&self.closed_loop, scope_id, level),
        })
    }
}

fn scoped_axis_is_ready(axis: &ReadinessAxis, scope_id: &str, level: ReadinessLevel) -> bool {
    let subjects = axis
        .subjects
        .iter()
        .filter(|subject| subject.scope_id == scope_id && subject.required_level == level)
        .collect::<Vec<_>>();
    !subjects.is_empty()
        && subjects
            .iter()
            .all(|subject| subject.ready && subject.blockers.is_empty())
}

fn repository_scope() -> String {
    "repository".into()
}

fn criterion_scope(criterion: &SpecAnchor) -> String {
    format!("criterion:{criterion}")
}

fn public_entrypoints_scope() -> &'static str {
    "public-entrypoints"
}

fn contracts_scope() -> &'static str {
    "contracts"
}

fn changed_units_scope() -> &'static str {
    "changed-units"
}

pub fn required_axes(level: ReadinessLevel) -> &'static [ReadinessAxisId] {
    use ReadinessAxisId::*;
    match level {
        ReadinessLevel::Off => &[],
        ReadinessLevel::Traceable => &[Inventory, Ownership],
        ReadinessLevel::Seedable => &[Inventory, Ownership, Seedability],
        ReadinessLevel::WorkReady => &[Inventory, Ownership, Seedability, Workability],
        ReadinessLevel::Verifiable => {
            &[Inventory, Ownership, Seedability, Workability, Verification]
        }
        ReadinessLevel::ClosedLoop => &[
            Inventory,
            Ownership,
            Seedability,
            Workability,
            Verification,
            ClosedLoop,
        ],
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadinessAxisId {
    Inventory,
    Ownership,
    Seedability,
    Workability,
    Verification,
    ClosedLoop,
}

pub fn evaluate(
    workspace: &SpecWorkspace,
    index: &SpecIndex,
    revision: &str,
    execute_verification: bool,
) -> Result<ReadinessReport> {
    let active = index
        .artifact_units
        .iter()
        .filter(|unit| {
            matches!(
                unit.reachability,
                mitase_inventory::ArtifactReachability::Active
            ) && unit.exposure != mitase_inventory::ArtifactExposure::Support
        })
        .collect::<Vec<_>>();

    let inventory_subjects = if let Some(error) = &index.inventory_error {
        vec![ReadinessSubject {
            id: "inventory:error".into(),
            scope_id: repository_scope(),
            required_level: ReadinessLevel::Traceable,
            ready: false,
            blockers: vec![error.clone()],
        }]
    } else {
        active
            .iter()
            .map(|unit| ReadinessSubject {
                id: format!("inventory:{}", unit.identity),
                scope_id: repository_scope(),
                required_level: ReadinessLevel::Traceable,
                ready: true,
                blockers: vec![],
            })
            .collect()
    };
    let inventory = axis_from_subjects(inventory_subjects);

    let criteria = implemented_criteria(workspace, index)?;
    let finalized_absent_targets = finalized_absent_targets(workspace, index, revision);
    let ownership_required = criteria
        .iter()
        .flat_map(|criterion| {
            index
                .criteria_to_implementation_targets
                .get(criterion)
                .into_iter()
                .flatten()
                .chain(
                    index
                        .criteria_to_verification_targets
                        .get(criterion)
                        .into_iter()
                        .flatten(),
                )
        })
        .filter_map(|target| index.target_to_artifact.get(target))
        .cloned()
        .collect::<BTreeSet<_>>();

    let ownership_subjects = active
        .iter()
        .filter_map(|unit| {
            let owners = index
                .artifact_owners
                .get(&unit.identity)
                .cloned()
                .unwrap_or_default();
            let required_for_maturity = ownership_required.contains(&unit.identity);
            let mut blockers = if owners.len() > 1 || (required_for_maturity && owners.len() != 1) {
                vec![format!("{} has {} owners", unit.identity, owners.len())]
            } else {
                vec![]
            };
            for owner in &owners {
                if required_for_maturity
                    && let Some(binding) = index.bindings.get(&owner.binding)
                    && binding.targets.len()
                        > workspace
                            .config
                            .validation
                            .readiness
                            .limits
                            .max_targets_per_binding
                {
                    blockers.push(format!(
                        "{} has {} targets, exceeding max_targets_per_binding",
                        owner.binding,
                        binding.targets.len()
                    ));
                }
            }
            (required_for_maturity || !blockers.is_empty()).then(|| ReadinessSubject {
                id: format!("ownership:{}", unit.identity),
                scope_id: repository_scope(),
                required_level: ReadinessLevel::Traceable,
                ready: blockers.is_empty(),
                blockers,
            })
        })
        .collect::<Vec<_>>();
    let mut ownership_subjects = ownership_subjects;
    for (binding_anchor, binding) in &index.bindings {
        if matches!(
            index.item_status.get(&binding_anchor.item),
            Some(ItemStatus::Planned)
        ) {
            continue;
        }
        for scope in &binding.owns {
            let matched = active
                .iter()
                .filter(|unit| scope_matches(scope, unit))
                .count();
            if matched
                > workspace
                    .config
                    .validation
                    .readiness
                    .limits
                    .max_ownership_scope_units
            {
                ownership_subjects.push(ReadinessSubject {
                    id: format!("ownership-scope:{binding_anchor}/{}", scope.id),
                    scope_id: repository_scope(),
                    required_level: ReadinessLevel::Traceable,
                    ready: false,
                    blockers: vec![format!(
                        "scope covers {matched} active units; max_ownership_scope_units is {}",
                        workspace
                            .config
                            .validation
                            .readiness
                            .limits
                            .max_ownership_scope_units
                    )],
                });
            }
        }
    }
    let ownership = axis_from_subjects(ownership_subjects);

    let public_probe = workspace
        .config
        .validation
        .readiness
        .probes
        .public_entrypoints
        .as_ref()
        .map(|probe| probe.level);
    let contracts_probe = workspace
        .config
        .validation
        .readiness
        .probes
        .contracts
        .as_ref()
        .map(|probe| probe.level);

    let mut seed_subjects = Vec::new();
    let mut work_subjects = Vec::new();
    let mut verification_subjects = Vec::new();
    let mut closed_subjects = Vec::new();
    let mut execution_jobs = Vec::new();

    for criterion in &criteria {
        let scope_id = criterion_scope(criterion);
        let implementation_targets = implementation_obligations(index, criterion);
        let verification_targets = index
            .criteria_to_verification_targets
            .get(criterion)
            .cloned()
            .unwrap_or_default();
        let required_level = scope_level(workspace, criterion, index);
        let plan = canonical_criterion_plan(workspace, index, criterion, revision);
        let plan_ready = plan
            .as_ref()
            .is_ok_and(|plan| matches!(plan.status, PlanStatus::Ready) && !plan.slices.is_empty());

        if implementation_targets.is_empty() {
            seed_subjects.push(subject(
                format!("criterion:{criterion}/implementation"),
                scope_id.clone(),
                required_level,
                false,
                "criterion has no exact implementation target",
            ));
        } else {
            for target_ref in &implementation_targets {
                let absent = target_is_absent(index, target_ref);
                let absence_ready = absent
                    && !index.all_target_to_artifact.contains_key(target_ref)
                    && finalized_absent_targets.contains(target_ref);
                let exact = index.target_to_artifact.contains_key(target_ref);
                let target_plan_ready = if absent {
                    absence_ready
                } else {
                    plan_ready
                        && plan.as_ref().is_ok_and(|plan| {
                            plan.slices.iter().any(|slice| {
                                slice
                                    .editable_targets
                                    .iter()
                                    .any(|target| &target.reference == target_ref)
                                    && !slice.verification_targets.is_empty()
                            })
                        })
                };
                let mut seed_blockers = Vec::new();
                if absent && !absence_ready {
                    seed_blockers.push(
                        "absent implementation target requires a finalized lifecycle proof".into(),
                    );
                } else if !absent && !exact {
                    seed_blockers.push(
                        "implementation target does not resolve to one exact artifact".into(),
                    );
                }
                if !target_plan_ready && !absence_ready {
                    seed_blockers.push(if !plan_ready {
                        plan.as_ref()
                            .err()
                            .map(ToString::to_string)
                            .unwrap_or_else(|| "criterion has no canonical ready plan".into())
                    } else {
                        "canonical plan has no exact target slice".into()
                    });
                }
                seed_subjects.push(ReadinessSubject {
                    id: format!("criterion:{criterion}/target:{}", target_ref),
                    scope_id: scope_id.clone(),
                    required_level,
                    ready: seed_blockers.is_empty(),
                    blockers: seed_blockers,
                });

                let verification_subject = verification_subject(
                    workspace,
                    index,
                    criterion,
                    std::slice::from_ref(target_ref),
                    &verification_targets,
                    required_level,
                );
                let verification_ready = verification_subject.ready;
                let verification_subject_id =
                    format!("criterion:{criterion}/target:{target_ref}/verification");
                if required_level >= ReadinessLevel::Verifiable {
                    verification_subjects.push(ReadinessSubject {
                        id: verification_subject_id,
                        ..verification_subject
                    });
                }

                let work_ready = target_plan_ready && verification_ready;
                let target_subject = format!("criterion:{criterion}/target:{target_ref}");
                if required_level >= ReadinessLevel::WorkReady {
                    work_subjects.push(subject(
                        format!("{target_subject}/work"),
                        scope_id.clone(),
                        required_level,
                        work_ready,
                        if work_ready {
                            ""
                        } else if !target_plan_ready {
                            "canonical target slice is not ready"
                        } else {
                            "verification closure is not exact"
                        },
                    ));
                }
                if required_level >= ReadinessLevel::ClosedLoop {
                    let closed_id = format!("{target_subject}/closed-loop");
                    let absence_closed = absence_ready && verification_ready;
                    let mut closed = subject(
                        closed_id.clone(),
                        scope_id.clone(),
                        required_level,
                        absence_closed,
                        if absence_closed {
                            ""
                        } else if absence_ready && !verification_ready {
                            "verification closure is not exact"
                        } else if !execute_verification {
                            "verification execution was not run"
                        } else if !work_ready {
                            "structural verification closure is not ready"
                        } else {
                            "canonical receipt and post-state validation have not passed"
                        },
                    );
                    if !absence_ready
                        && execute_verification
                        && work_ready
                        && let Ok(plan) = &plan
                    {
                        if let Some(slice) = plan.slices.iter().find(|slice| {
                            slice
                                .editable_targets
                                .iter()
                                .any(|target| &target.reference == target_ref)
                                && !slice.verification_targets.is_empty()
                        }) {
                            execution_jobs.push((
                                closed_id,
                                criterion.clone(),
                                plan.clone(),
                                slice.id.clone(),
                            ));
                        } else {
                            closed.blockers =
                                vec!["canonical plan has no verification slice for target".into()];
                        }
                    }
                    closed_subjects.push(closed);
                }
            }
        }
        if implementation_targets.is_empty() {
            let verification_subject = verification_subject(
                workspace,
                index,
                criterion,
                &implementation_targets,
                &verification_targets,
                required_level,
            );
            let verification_ready = verification_subject.ready;
            if required_level >= ReadinessLevel::Verifiable {
                verification_subjects.push(verification_subject);
            }
            let work_ready = plan_ready && verification_ready;
            if required_level >= ReadinessLevel::WorkReady {
                work_subjects.push(subject(
                    format!("criterion:{criterion}/work"),
                    scope_id.clone(),
                    required_level,
                    work_ready,
                    if work_ready {
                        ""
                    } else if !plan_ready {
                        "canonical plan is not ready"
                    } else {
                        "verification closure is not exact"
                    },
                ));
            }
            if required_level >= ReadinessLevel::ClosedLoop {
                closed_subjects.push(subject(
                    format!("criterion:{criterion}/closed-loop"),
                    scope_id,
                    required_level,
                    false,
                    if !execute_verification {
                        "verification execution was not run"
                    } else {
                        "structural verification closure is not ready"
                    },
                ));
            }
        }
    }

    // Features are first-class readiness subjects as well as their criteria.
    // This keeps the Mitase capability catalog in the denominator once a feature
    // is declared implemented, even when the feature has no requirement
    // criterion of its own.
    seed_subjects.extend(implemented_feature_subjects(
        workspace,
        index,
        &finalized_absent_targets,
    ));

    if let Some(required_level) = public_probe {
        let public_subjects =
            public_entrypoint_subjects(workspace, index, revision, required_level);
        // A public entrypoint probe is a scoped denominator for every axis it
        // asks Workbench to enforce.  Keeping these subjects only in
        // seedability made work-ready and above probes either impossible to
        // satisfy or silently unrelated to the entrypoint they advertised.
        if required_level >= ReadinessLevel::Seedable {
            seed_subjects.extend(public_subjects.clone());
        }
        if required_level >= ReadinessLevel::WorkReady {
            work_subjects.extend(public_subjects.clone());
        }
        if required_level >= ReadinessLevel::Verifiable {
            verification_subjects.extend(public_subjects.clone());
        }
        if required_level >= ReadinessLevel::ClosedLoop {
            closed_subjects.extend(public_subjects);
        }
    }

    if let Some(required_level) = contracts_probe {
        if index.contracts.is_empty() {
            let empty = subject(
                "contracts:active".into(),
                contracts_scope(),
                required_level,
                false,
                "contracts: all was requested but no active contract is declared",
            );
            seed_subjects.push(empty.clone());
            work_subjects.push(empty);
        } else {
            for (anchor, contract) in &index.contracts {
                let mut blockers = Vec::new();
                let expected = std::iter::once(&contract.source)
                    .chain(
                        contract
                            .participants
                            .iter()
                            .map(|participant| &participant.target),
                    )
                    .cloned()
                    .collect::<BTreeSet<_>>();
                for target in &expected {
                    if !index.target_to_artifact.contains_key(target) {
                        blockers.push(format!("{target} is not an exact participant target"));
                    }
                }
                if blockers.is_empty() {
                    match canonical_contract_plan(workspace, index, anchor, revision) {
                        Ok(plan) if matches!(plan.status, PlanStatus::Ready) => {
                            let contract_slices = plan
                                .slices
                                .iter()
                                .filter(|slice| slice.contracts.contains(anchor))
                                .collect::<Vec<_>>();
                            if contract_slices.is_empty() {
                                blockers.push(
                                    "canonical contract origin produced no contract closure".into(),
                                );
                            } else {
                                for slice in contract_slices {
                                    let visible = slice
                                        .editable_targets
                                        .iter()
                                        .chain(&slice.verification_targets)
                                        .chain(&slice.readonly_context)
                                        .map(|target| target.reference.clone())
                                        .collect::<BTreeSet<_>>();
                                    if !expected.is_subset(&visible) {
                                        blockers.push(
                                            "canonical contract plan omits an exact participant target"
                                                .into(),
                                        );
                                    }
                                    if slice
                                        .readonly_context
                                        .iter()
                                        .any(|target| !expected.contains(&target.reference))
                                    {
                                        blockers.push(
                                            "contract readonly closure contains a non-participant target"
                                                .into(),
                                        );
                                    }
                                }
                            }
                        }
                        Ok(plan) => {
                            blockers.push(format!("canonical contract plan is {:?}", plan.status))
                        }
                        Err(error) => {
                            blockers.push(format!("canonical contract origin failed: {error}"))
                        }
                    }
                }
                let ready = blockers.is_empty();
                seed_subjects.push(ReadinessSubject {
                    id: format!("contract:{anchor}"),
                    scope_id: contracts_scope().into(),
                    required_level,
                    ready,
                    blockers: blockers.clone(),
                });
                if required_level >= ReadinessLevel::WorkReady {
                    work_subjects.push(ReadinessSubject {
                        id: format!("contract:{anchor}/plan"),
                        scope_id: contracts_scope().into(),
                        required_level,
                        ready,
                        blockers,
                    });
                }
            }
        }
    }

    if workspace.config.validation.readiness.probes.changed_units {
        let changed_paths = changed_artifact_paths(workspace, revision)?;
        for path in changed_paths {
            let units = active
                .iter()
                .filter(|unit| unit.path.to_string_lossy() == path)
                .collect::<Vec<_>>();
            if units.is_empty() {
                let owners = index
                    .bindings
                    .iter()
                    .flat_map(|(binding_anchor, binding)| {
                        let path_for_owner = path.clone();
                        binding
                            .owns
                            .iter()
                            .filter(move |scope| {
                                scope.adapter == "declared"
                                    && scope.path.to_string_lossy() == path_for_owner
                                    && matches!(scope.selector, OwnershipSelector::File)
                            })
                            .map(move |scope| format!("{binding_anchor}#scope.{}", scope.id))
                    })
                    .collect::<Vec<_>>();
                let ready = owners.len() == 1;
                seed_subjects.push(ReadinessSubject {
                    id: format!("changed:{path}"),
                    scope_id: changed_units_scope().into(),
                    required_level: ReadinessLevel::Seedable,
                    ready,
                    blockers: if ready {
                        vec![]
                    } else {
                        vec![format!(
                            "changed file is absent from active inventory and has {} exact owners",
                            owners.len()
                        )]
                    },
                });
            } else {
                for unit in units {
                    let owners = index
                        .artifact_owners
                        .get(&unit.identity)
                        .cloned()
                        .unwrap_or_default();
                    let ready = owners.len() == 1;
                    seed_subjects.push(ReadinessSubject {
                        id: format!("changed:{}", unit.identity),
                        scope_id: changed_units_scope().into(),
                        required_level: ReadinessLevel::Seedable,
                        ready,
                        blockers: if ready {
                            vec![]
                        } else {
                            vec!["changed artifact requires exactly one owner".into()]
                        },
                    });
                }
            }
        }
    }

    let mut receipts = Vec::new();
    if execute_verification {
        for (subject_id, _criterion, plan, slice_id) in execution_jobs {
            match crate::execute_verification(workspace, index, &plan, &slice_id, revision) {
                Ok(receipt) => {
                    let slice = plan
                        .slices
                        .iter()
                        .find(|slice| slice.id == slice_id)
                        .context("readiness slice disappeared")?;
                    let post_state = crate::validate_without_readiness(&crate::ValidationContext {
                        config: &workspace.config,
                        workspace,
                        index,
                        changed_files: None,
                        reported_changed_files: None,
                        work_plan: Some(&plan),
                        selected_slice: Some(slice),
                        plan_mode: crate::PlanValidationMode::PostState,
                        preset: workspace.config.validation.preset,
                        revision: Some(revision),
                        change_base_revision: None,
                    });
                    if post_state.is_valid() {
                        receipts.push(receipt);
                        if let Some(subject) = closed_subjects
                            .iter_mut()
                            .find(|subject| subject.id == subject_id)
                        {
                            subject.ready = true;
                            subject.blockers.clear();
                        }
                    } else if let Some(subject) = closed_subjects
                        .iter_mut()
                        .find(|subject| subject.id == subject_id)
                    {
                        subject.blockers = post_state
                            .diagnostics
                            .iter()
                            .filter(|diagnostic| {
                                matches!(diagnostic.severity, mitase_diagnostics::Severity::Error)
                            })
                            .map(|diagnostic| diagnostic.message.clone())
                            .collect();
                    }
                }
                Err(error) => {
                    if let Some(subject) = closed_subjects
                        .iter_mut()
                        .find(|subject| subject.id == subject_id)
                    {
                        subject.blockers = vec![error.to_string()];
                    }
                }
            }
        }
    }

    let execution_state = if !execute_verification {
        "execution-not-run"
    } else if closed_subjects.iter().all(|subject| subject.ready) && !closed_subjects.is_empty() {
        "closed-loop-verified"
    } else {
        "structurally-verifiable"
    };
    Ok(ReadinessReport {
        target: workspace.config.validation.readiness.target,
        inventory,
        ownership,
        seedability: axis_from_subjects(seed_subjects),
        workability: axis_from_subjects(work_subjects),
        verification: axis_from_subjects(verification_subjects),
        closed_loop: axis_from_subjects(closed_subjects),
        execution_state: execution_state.into(),
        receipts,
    })
}

fn canonical_contract_plan(
    workspace: &SpecWorkspace,
    index: &SpecIndex,
    contract: &SpecAnchor,
    revision: &str,
) -> Result<mitase_work_model::WorkPlan> {
    let criterion = index
        .contracts
        .get(contract)
        .and_then(|value| value.guarantees.first())
        .cloned()
        .with_context(|| format!("contract {contract} has no criterion guarantee"))?;
    let requested_targets = index
        .contracts
        .get(contract)
        .map(|value| {
            // A contract source is readonly context, not an editable origin.
            // Probe the active implementation participants; the planner then
            // derives the source and the other contract participants through
            // the exact dependency closure.
            value
                .participants
                .iter()
                .filter(|participant| {
                    index
                        .bindings
                        .get(&participant.target.binding)
                        .is_some_and(|binding| {
                            binding.role == mitase_spec_model::BindingRole::Implementation
                        })
                })
                .filter(|participant| {
                    index.target(&participant.target).is_some_and(|target| {
                        target.lifecycle != mitase_spec_model::ArtifactTargetLifecycle::Absent
                    })
                })
                .map(|participant| participant.target.clone())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if requested_targets.is_empty() {
        bail!("contract {contract} has no active implementation participant");
    }
    mitase_planner::plan_probe(
        &mitase_planner::PlanProbe {
            criterion,
            requested_targets,
            max_slices: workspace
                .config
                .validation
                .readiness
                .limits
                .max_slices_per_origin,
        },
        workspace,
        index,
        revision,
    )
}

fn subject(
    id: String,
    scope_id: impl Into<String>,
    required_level: ReadinessLevel,
    ready: bool,
    blocker: &str,
) -> ReadinessSubject {
    ReadinessSubject {
        id,
        scope_id: scope_id.into(),
        required_level,
        ready,
        blockers: if ready || blocker.is_empty() {
            vec![]
        } else {
            vec![blocker.into()]
        },
    }
}

fn axis_from_subjects(subjects: Vec<ReadinessSubject>) -> ReadinessAxis {
    let mut subjects = subjects;
    subjects.sort_by(|a, b| a.id.cmp(&b.id));
    subjects.dedup_by(|a, b| {
        a.id == b.id && {
            b.blockers = a.blockers.clone();
            b.ready = a.ready;
            true
        }
    });
    let required = subjects.len();
    let ready = subjects
        .iter()
        .filter(|subject| subject.ready && subject.blockers.is_empty())
        .count();
    let blockers = subjects
        .iter()
        .flat_map(|subject| {
            subject
                .blockers
                .iter()
                .map(move |blocker| format!("{}: {blocker}", subject.id))
        })
        .collect();
    ReadinessAxis {
        required,
        ready,
        blockers,
        subjects,
    }
}

fn implemented_criteria(workspace: &SpecWorkspace, index: &SpecIndex) -> Result<Vec<SpecAnchor>> {
    let configured = &workspace
        .config
        .validation
        .readiness
        .probes
        .implemented_criteria;
    if configured.is_empty() {
        return Ok(index
            .criterion_status
            .iter()
            .filter(|(_, status)| **status == ItemStatus::Implemented)
            .map(|(anchor, _)| anchor.clone())
            .collect());
    }
    let mut criteria = BTreeSet::new();
    for probe in configured {
        if !criteria.insert(probe.criterion.clone()) {
            anyhow::bail!(
                "readiness criterion {} is configured more than once",
                probe.criterion
            );
        }
        if index.criterion_status.get(&probe.criterion) != Some(&ItemStatus::Implemented) {
            anyhow::bail!(
                "configured readiness criterion {} is missing or not implemented",
                probe.criterion
            );
        }
    }
    Ok(criteria.into_iter().collect())
}

fn implementation_obligations(index: &SpecIndex, criterion: &SpecAnchor) -> Vec<BoundTargetRef> {
    index
        .all_criteria_to_implementation_targets
        .get(criterion)
        .into_iter()
        .flatten()
        .filter(|target| {
            index.bindings.get(&target.binding).is_some_and(|binding| {
                index
                    .item_status
                    .get(&target.binding.item)
                    .is_none_or(|status| *status != ItemStatus::Planned)
                    && binding
                        .targets
                        .iter()
                        .any(|candidate| candidate.id == target.target_id)
            })
        })
        .cloned()
        .collect()
}

fn target_is_absent(index: &SpecIndex, target: &BoundTargetRef) -> bool {
    index
        .bindings
        .get(&target.binding)
        .and_then(|binding| {
            binding
                .targets
                .iter()
                .find(|candidate| candidate.id == target.target_id)
        })
        .is_some_and(|candidate| {
            candidate.lifecycle == mitase_spec_model::ArtifactTargetLifecycle::Absent
        })
}

fn json_files_recursive(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let Ok(entries) = fs::read_dir(root) else {
        return files;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            files.extend(json_files_recursive(&path));
        } else if path.extension().and_then(|value| value.to_str()) == Some("json") {
            files.push(path);
        }
    }
    files
}

fn durable_attempt_digest(attempt: &CompletionAttempt) -> Option<String> {
    let mut copy = attempt.clone();
    let expected = copy.attempt_digest.clone();
    copy.attempt_digest.clear();
    let bytes = mitase_work_model::canonical_json_bytes(serde_json::to_value(&copy).ok()?);
    let mut hash = Sha256::new();
    hash.update(mitase_work_model::VERIFICATION_RECEIPT_DIGEST_DOMAIN.as_bytes());
    hash.update(bytes);
    (expected == format_sha256(hash.finalize())).then_some(expected)
}

/// Read absence evidence only when the durable records form one validated
/// closure. A finalization JSON file is an index, not authority by itself:
/// it must point to a complete attempt, its canonical approval, the exact
/// Remove slice, and a verification receipt whose lifecycle proof agrees.
fn finalized_absent_targets(
    workspace: &SpecWorkspace,
    index: &SpecIndex,
    revision: &str,
) -> BTreeSet<BoundTargetRef> {
    let Ok(output) = Command::new("git")
        .args(["rev-parse", "--git-path", "mitase"])
        .current_dir(&workspace.root)
        .output()
    else {
        return BTreeSet::new();
    };
    if !output.status.success() {
        return BTreeSet::new();
    }
    let raw = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    let root = PathBuf::from(raw);
    let root = if root.is_absolute() {
        root
    } else {
        workspace.root.join(root)
    };
    let finalizations = root.join("completion/v1/finalizations");
    let attempts = json_files_recursive(&root.join("completion/v1/attempts"));
    let approvals = json_files_recursive(&root.join("completion/v1/approvals"));
    let finalizations = json_files_recursive(&finalizations);
    let mut targets = BTreeSet::new();
    for path in finalizations {
        let Ok(bytes) = fs::read(&path) else {
            continue;
        };
        let Ok(receipt) = serde_json::from_slice::<FinalizationReceipt>(&bytes) else {
            continue;
        };
        if receipt.schema != FINALIZATION_RECEIPT_SCHEMA
            || receipt.post_workspace_fingerprint.is_empty()
        {
            continue;
        }
        let mut finalization_without_digest = receipt.clone();
        let finalization_digest = finalization_without_digest.finalization_digest.clone();
        finalization_without_digest.finalization_digest.clear();
        let Ok(finalization_value) = serde_json::to_value(&finalization_without_digest) else {
            continue;
        };
        let finalization_bytes = mitase_work_model::canonical_json_bytes(finalization_value);
        let mut finalization_hash = Sha256::new();
        finalization_hash.update(mitase_work_model::FINALIZATION_RECEIPT_DIGEST_DOMAIN.as_bytes());
        finalization_hash.update(finalization_bytes);
        if finalization_digest != format_sha256(finalization_hash.finalize()) {
            continue;
        }
        let Some(attempt) = attempts.iter().find_map(|path| {
            let bytes = fs::read(path).ok()?;
            let attempt = serde_json::from_slice::<CompletionAttempt>(&bytes).ok()?;
            (attempt.attempt_id == receipt.attempt_id).then_some(attempt)
        }) else {
            continue;
        };
        if attempt.schema != mitase_work_model::COMPLETION_ATTEMPT_SCHEMA
            || durable_attempt_digest(&attempt).is_none()
            || attempt.attempt_digest != receipt.attempt_digest
            || attempt.plan_digest != receipt.plan_digest
            || attempt.slice_id != receipt.slice_id
            || attempt.approved_plan_digest != attempt.plan_digest
            || attempt.report.attempt_id != attempt.attempt_id
            || attempt.report.plan_digest != attempt.plan_digest
            || attempt.report.slice_id != attempt.slice_id
            || attempt.report.status != CompletionStatus::Complete
            || attempt.verification.status != VerificationAttemptStatus::Complete
        {
            continue;
        }
        let Some(verification) = attempt.receipt.as_ref() else {
            continue;
        };
        let Ok(verification_value) = serde_json::to_value(verification) else {
            continue;
        };
        let expected_receipt_digest = {
            let bytes = mitase_work_model::canonical_json_bytes(verification_value);
            let mut hash = Sha256::new();
            hash.update(mitase_work_model::VERIFICATION_RECEIPT_DIGEST_DOMAIN.as_bytes());
            hash.update(bytes);
            format_sha256(hash.finalize())
        };
        if attempt.report.receipt_digest.as_deref() != Some(expected_receipt_digest.as_str()) {
            continue;
        }
        if verification.schema != VERIFICATION_RECEIPT_SCHEMA
            || verification.plan_digest != receipt.plan_digest
            || verification.slice_id != receipt.slice_id
            || verification.workspace_fingerprint != receipt.pre_workspace_fingerprint
            || verification.lifecycle_proofs != receipt.lifecycle_proofs
            || verification
                .executions
                .iter()
                .any(|execution| execution.exit_code != 0 || execution.proof.matched_count != 1)
        {
            continue;
        }
        let Some(approval) = approvals.iter().find_map(|path| {
            let bytes = fs::read(path).ok()?;
            let approval = serde_json::from_slice::<PlanApproval>(&bytes).ok()?;
            (approval.plan_digest == receipt.plan_digest).then_some(approval)
        }) else {
            continue;
        };
        if approval.schema != PLAN_APPROVAL_SCHEMA
            || approval.plan_digest != approval.plan.canonical_digest
            || approval.plan_digest != work_plan_digest(&approval.plan)
            || approval.revision != approval.plan.basis.revision
            || approval.workspace_fingerprint != approval.plan.basis.workspace_fingerprint
            || approval.slice_id != receipt.slice_id
            || approval.plan.slices.len() != 1
            || approval.plan.slices[0].id != receipt.slice_id
            || verification.revision != approval.revision
            || !revision_is_ancestor(&workspace.root, &approval.revision, revision)
        {
            continue;
        }
        let Some(slice) = approval
            .plan
            .slices
            .iter()
            .find(|slice| slice.id == receipt.slice_id)
        else {
            continue;
        };
        let expected_claims = slice
            .verification_targets
            .iter()
            .filter_map(|target| target.verification_claim.clone())
            .collect::<BTreeSet<_>>();
        let actual_claims = verification
            .executions
            .iter()
            .filter_map(|execution| execution.claim.clone())
            .collect::<BTreeSet<_>>();
        if expected_claims.is_empty() || expected_claims != actual_claims {
            continue;
        }
        let Some(baseline) = crate::load_workspace_at_revision(&workspace.root, &approval.revision)
        else {
            continue;
        };
        if validate_durable_receipt_closure(
            &baseline.workspace,
            &baseline.index,
            &approval.plan,
            slice,
            &attempt,
            verification,
        )
        .is_err()
        {
            continue;
        }
        let mut valid = true;
        let mut finalized_targets = BTreeSet::new();
        let expected_remove_targets = slice
            .editable_targets
            .iter()
            .filter(|target| {
                target.transition == TargetTransition::Remove
                    && target.lifecycle == TargetLifecycle::EnsureAbsent
            })
            .map(|target| target.reference.clone())
            .collect::<BTreeSet<_>>();
        for proof in &receipt.lifecycle_proofs {
            let Some(target) = slice
                .editable_targets
                .iter()
                .find(|target| target.reference == proof.reference)
            else {
                valid = false;
                break;
            };
            if target.transition != TargetTransition::Remove
                || target.lifecycle != TargetLifecycle::EnsureAbsent
                || proof.transition != TargetTransition::Remove
                || proof.lifecycle != TargetLifecycle::EnsureAbsent
                || proof.before_content_hash != target.content_hash
                || !proof.after_content_hash.is_empty()
                || !current_absence_obligation_matches(index, target)
            {
                valid = false;
                break;
            }
            finalized_targets.insert(proof.reference.clone());
        }
        if finalized_targets.len() != receipt.lifecycle_proofs.len()
            || finalized_targets != expected_remove_targets
        {
            valid = false;
        }
        if valid && !receipt.lifecycle_proofs.is_empty() {
            targets.extend(finalized_targets);
        }
    }
    targets
}

fn validate_durable_receipt_closure(
    baseline_workspace: &SpecWorkspace,
    baseline_index: &SpecIndex,
    plan: &mitase_work_model::WorkPlan,
    slice: &mitase_work_model::ExecutionSlice,
    attempt: &CompletionAttempt,
    receipt: &VerificationReceipt,
) -> Result<()> {
    if receipt.schema != VERIFICATION_RECEIPT_SCHEMA
        || receipt.plan_digest != plan.canonical_digest
        || receipt.slice_id != slice.id
        || attempt.verification.status != VerificationAttemptStatus::Complete
        || attempt.verification.failure.is_some()
    {
        bail!("durable verification receipt identity is invalid");
    }

    let expected_claims = slice
        .verification_targets
        .iter()
        .map(|target| {
            target
                .verification_claim
                .clone()
                .ok_or_else(|| anyhow::anyhow!("verification target has no exact claim"))
        })
        .collect::<Result<BTreeSet<_>>>()?;
    let actual_claims = receipt
        .executions
        .iter()
        .map(|execution| {
            execution
                .claim
                .clone()
                .ok_or_else(|| anyhow::anyhow!("receipt execution has no exact claim"))
        })
        .collect::<Result<Vec<_>>>()?;
    if actual_claims.len() != expected_claims.len()
        || actual_claims.into_iter().collect::<BTreeSet<_>>() != expected_claims
        || attempt.verification.executions.len() != receipt.executions.len()
    {
        bail!("durable verification receipt execution set is not exact");
    }

    for execution in &receipt.executions {
        let claim = execution
            .claim
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("receipt execution has no exact claim"))?;
        if execution.target != claim.target
            || execution.exit_code != 0
            || execution.command.is_empty()
            || execution.proof.matched_count != 1
        {
            bail!("durable verification receipt execution is invalid");
        }
        let (_verification_target, runner_ref, covers) =
            crate::resolve_verification_claim(baseline_index, claim)?;
        let configured = baseline_workspace
            .config
            .verification
            .runners
            .get(&runner_ref.runner)
            .ok_or_else(|| anyhow::anyhow!("durable verification runner is not configured"))?;
        let arguments = configured
            .arguments
            .iter()
            .map(|argument| crate::expand_runner_argument(argument, &runner_ref.arguments))
            .collect::<Vec<_>>();
        let arguments = crate::canonical_runner_arguments(&configured.executable, arguments);
        let expected_command = std::iter::once(configured.executable.clone())
            .chain(arguments)
            .collect::<Vec<_>>();
        if execution.runner != runner_ref.runner || execution.command != expected_command {
            bail!("durable verification receipt command is stale");
        }
        let planned_verification = slice
            .verification_targets
            .iter()
            .find(|target| target.reference == execution.target)
            .ok_or_else(|| anyhow::anyhow!("receipt target is outside the selected slice"))?;
        let resolved_verification = crate::resolve_planned_target_for_workspace(
            baseline_workspace,
            baseline_index,
            planned_verification,
        )
        .ok_or_else(|| anyhow::anyhow!("durable verification target cannot be resolved"))?;
        if execution.verification_digest != resolved_verification.content_hash {
            bail!("durable verification target digest is stale");
        }

        let expected_implementation_digests = covers
            .iter()
            .map(|covered| {
                let digest = receipt
                    .lifecycle_proofs
                    .iter()
                    .find(|proof| proof.reference == *covered)
                    .map(|proof| proof.after_content_hash.clone())
                    .or_else(|| {
                        slice
                            .editable_targets
                            .iter()
                            .chain(slice.verification_targets.iter())
                            .chain(slice.readonly_context.iter())
                            .find(|target| target.reference == *covered)
                            .and_then(|target| {
                                crate::resolve_planned_target_for_workspace(
                                    baseline_workspace,
                                    baseline_index,
                                    target,
                                )
                                .map(|resolved| resolved.content_hash)
                            })
                    })
                    .ok_or_else(|| {
                        anyhow::anyhow!("durable implementation target cannot be resolved")
                    })?;
                Ok((covered.clone(), digest))
            })
            .collect::<Result<BTreeMap<_, _>>>()?;
        if execution.implementation_digests != expected_implementation_digests {
            bail!("durable implementation digest closure is stale");
        }
        if execution.proof.identity
            != runner_ref
                .arguments
                .get("test")
                .cloned()
                .unwrap_or_default()
            || execution.proof.identity.is_empty()
        {
            bail!("durable exact-test proof is stale");
        }
    }

    for execution in &receipt.executions {
        let mirrored = attempt.verification.executions.iter().find(|candidate| {
            candidate.target.as_ref() == Some(&execution.target)
                && candidate.claim == execution.claim
        });
        let Some(mirrored) = mirrored else {
            bail!("durable attempt does not mirror receipt executions");
        };
        if mirrored.runner != execution.runner
            || mirrored.command != execution.command
            || mirrored.exit_code != Some(execution.exit_code)
            || mirrored.stdout_digest.as_deref() != Some(execution.stdout_digest.as_str())
            || mirrored.stderr_digest.as_deref() != Some(execution.stderr_digest.as_str())
            || mirrored.proof.as_ref() != Some(&execution.proof)
            || mirrored.error.is_some()
        {
            bail!("durable attempt execution evidence is inconsistent");
        }
    }
    validate_durable_completion_report(attempt, slice, receipt)
}

fn validate_durable_completion_report(
    attempt: &CompletionAttempt,
    slice: &mitase_work_model::ExecutionSlice,
    receipt: &VerificationReceipt,
) -> Result<()> {
    if attempt.report.status != CompletionStatus::Complete || !attempt.report.blockers.is_empty() {
        bail!("durable completion report is not complete");
    }
    let expected_checks = slice
        .completion
        .iter()
        .map(serde_json::to_string)
        .collect::<Result<BTreeSet<_>, _>>()?;
    let actual_checks = attempt
        .report
        .checks
        .iter()
        .map(|check| serde_json::to_string(&check.check))
        .collect::<Result<BTreeSet<_>, _>>()?;
    if attempt.report.checks.len() != expected_checks.len()
        || actual_checks != expected_checks
        || attempt
            .report
            .checks
            .iter()
            .any(|check| !check.passed || check.evidence.is_empty())
    {
        bail!("durable completion checks are not exact");
    }
    let executed_claims = receipt
        .executions
        .iter()
        .filter_map(|execution| execution.claim.clone())
        .collect::<BTreeSet<_>>();
    let mut expected = BTreeMap::<String, (String, BTreeSet<String>)>::new();
    for acceptance in &slice.acceptance {
        let targets = slice
            .verification_targets
            .iter()
            .filter(|target| {
                target.verification_claim.as_ref().is_some_and(|claim| {
                    claim.criterion == acceptance.anchor && executed_claims.contains(claim)
                })
            })
            .map(|target| target.reference.to_string())
            .collect::<BTreeSet<_>>();
        if targets.is_empty()
            || expected
                .insert(
                    acceptance.anchor.to_string(),
                    (acceptance.statement.clone(), targets),
                )
                .is_some()
        {
            bail!("durable completion acceptance evidence is incomplete");
        }
    }
    let mut actual = BTreeMap::<String, (String, BTreeSet<String>)>::new();
    for evidence in &attempt.report.demonstrated {
        if actual
            .insert(
                evidence.anchor.to_string(),
                (
                    evidence.statement.clone(),
                    evidence
                        .verification_targets
                        .iter()
                        .map(ToString::to_string)
                        .collect(),
                ),
            )
            .is_some()
        {
            bail!("durable completion acceptance evidence is duplicated");
        }
    }
    if actual != expected {
        bail!("durable completion acceptance evidence is not exact");
    }
    Ok(())
}

/// A durable absence proof remains useful across commits and unrelated
/// workspace changes, but only while the current specification still carries
/// the same exact obligation. The target reference alone is not enough: a
/// reused reference with a different path or binding role must not inherit the
/// old proof.
fn current_absence_obligation_matches(
    index: &SpecIndex,
    approved_target: &mitase_work_model::PlannedTarget,
) -> bool {
    let Some(binding) = index.bindings.get(&approved_target.reference.binding) else {
        return false;
    };
    if binding.role != approved_target.role {
        return false;
    }
    let Some(target) = binding
        .targets
        .iter()
        .find(|target| target.id == approved_target.reference.target_id)
    else {
        return false;
    };
    target.lifecycle == mitase_spec_model::ArtifactTargetLifecycle::Absent
        && target.path.to_string_lossy() == approved_target.resolved_path
        && selector_matches_resolved_target(&target.selector, &approved_target.resolved_selector)
        && !index
            .all_target_to_artifact
            .contains_key(&approved_target.reference)
        && !index
            .target_to_artifact
            .contains_key(&approved_target.reference)
}

fn selector_matches_resolved_target(
    selector: &mitase_spec_model::ExactSelector,
    resolved: &mitase_work_model::ResolvedSelector,
) -> bool {
    match selector {
        mitase_spec_model::ExactSelector::File => {
            resolved.description == "file" && resolved.symbols.is_empty()
        }
        mitase_spec_model::ExactSelector::Symbol { name } => {
            resolved.description == format!("symbol {name}") && resolved.symbols == [name.clone()]
        }
        mitase_spec_model::ExactSelector::Operation { method, path } => {
            resolved.description == format!("operation {} {path}", method.to_ascii_uppercase())
                && resolved.symbols.is_empty()
        }
        mitase_spec_model::ExactSelector::Heading { value } => {
            resolved.description == format!("heading {value}") && resolved.symbols.is_empty()
        }
        mitase_spec_model::ExactSelector::JsonPointer { value } => {
            resolved.description == format!("json pointer {value}") && resolved.symbols.is_empty()
        }
        mitase_spec_model::ExactSelector::Marker { value } => {
            resolved.description == format!("marker {value}") && resolved.symbols.is_empty()
        }
    }
}

fn revision_is_ancestor(root: &Path, ancestor: &str, descendant: &str) -> bool {
    Command::new("git")
        .args([
            "-C",
            root.to_string_lossy().as_ref(),
            "merge-base",
            "--is-ancestor",
        ])
        .args([ancestor, descendant])
        .status()
        .is_ok_and(|status| status.success())
}

fn implemented_feature_subjects(
    workspace: &SpecWorkspace,
    index: &SpecIndex,
    finalized_absent_targets: &BTreeSet<BoundTargetRef>,
) -> Vec<ReadinessSubject> {
    workspace
        .documents
        .iter()
        .flat_map(|loaded| match &loaded.document {
            mitase_spec_model::SpecDocument::Features { features, .. } => features.clone(),
            _ => Vec::new(),
        })
        .filter(|feature| feature.status == ItemStatus::Implemented)
        .map(|feature| {
            let bindings = index
                .bindings
                .iter()
                .filter(|(anchor, binding)| {
                    anchor.item == feature.id
                        && binding.role == mitase_spec_model::BindingRole::Implementation
                })
                .collect::<Vec<_>>();
            let mut blockers = Vec::new();
            if bindings.is_empty() {
                blockers.push("implemented feature has no implementation binding".into());
            }
            for (anchor, binding) in bindings {
                if binding.targets.is_empty() {
                    blockers.push(format!("{anchor} has no exact implementation target"));
                }
                for target in &binding.targets {
                    let reference = BoundTargetRef {
                        binding: anchor.clone(),
                        target_id: target.id.clone(),
                    };
                    if target.lifecycle == mitase_spec_model::ArtifactTargetLifecycle::Absent {
                        if index.all_target_to_artifact.contains_key(&reference) {
                            blockers.push(format!(
                                "{reference} is declared absent but still resolves in the workspace"
                            ));
                        } else if !finalized_absent_targets.contains(&reference) {
                            blockers.push(format!(
                                "{reference} has no finalized absence lifecycle proof"
                            ));
                        }
                        continue;
                    }
                    if !index.target_to_artifact.contains_key(&reference) {
                        blockers.push(format!("{reference} is not an exact implementation target"));
                    }
                    let owners = index
                        .target_to_artifact
                        .get(&reference)
                        .and_then(|identity| index.artifact_owners.get(identity))
                        .cloned()
                        .unwrap_or_default();
                    if !owners.iter().any(|owner| owner.binding == *anchor) {
                        blockers.push(format!("{reference} has no canonical owner"));
                    }
                }
            }
            ReadinessSubject {
                id: format!("feature:{}", feature.id),
                scope_id: repository_scope(),
                required_level: ReadinessLevel::Traceable,
                ready: blockers.is_empty(),
                blockers,
            }
        })
        .collect()
}

fn canonical_criterion_plan(
    workspace: &SpecWorkspace,
    index: &SpecIndex,
    criterion: &SpecAnchor,
    revision: &str,
) -> Result<mitase_work_model::WorkPlan> {
    mitase_planner::plan_probe(
        &mitase_planner::PlanProbe {
            criterion: criterion.clone(),
            requested_targets: vec![],
            max_slices: workspace
                .config
                .validation
                .readiness
                .limits
                .max_slices_per_origin,
        },
        workspace,
        index,
        revision,
    )
}

fn verification_subject(
    workspace: &SpecWorkspace,
    index: &SpecIndex,
    criterion: &SpecAnchor,
    implementations: &[BoundTargetRef],
    verifications: &[BoundTargetRef],
    required_level: ReadinessLevel,
) -> ReadinessSubject {
    let mut blockers = Vec::new();
    if implementations.is_empty() || verifications.is_empty() {
        blockers.push("criterion must have implementation and verification targets".into());
    }
    for implementation in implementations {
        let covered = verifications.iter().any(|verification| {
            crate::resolve_verification_claim(
                index,
                &VerificationClaimRef {
                    target: verification.clone(),
                    criterion: criterion.clone(),
                },
            )
            .is_ok_and(|(_, _, covers)| covers.contains(implementation))
        });
        if !covered {
            blockers.push(format!(
                "no exact verification target covers {implementation}"
            ));
        }
    }
    for verification in verifications {
        if index.target(verification).is_none() {
            blockers.push(format!("verification target {verification} is unresolved"));
            continue;
        }
        let exact_owner = index
            .target_to_artifact
            .get(verification)
            .and_then(|identity| index.artifact_owners.get(identity))
            .is_some_and(|owners| {
                owners.iter().any(|owner| {
                    owner.binding == verification.binding
                        && owner.target_id.as_ref() == Some(&verification.target_id)
                })
            });
        if !exact_owner {
            blockers.push(format!(
                "verification target {verification} requires its own exact ownership"
            ));
        }
        let claim = VerificationClaimRef {
            target: verification.clone(),
            criterion: criterion.clone(),
        };
        let (_, runner_ref, _) = match crate::resolve_verification_claim(index, &claim) {
            Ok(resolved) => resolved,
            Err(error) => {
                blockers.push(error.to_string());
                continue;
            }
        };
        let runner = &runner_ref.runner;
        if !workspace.config.verification.runners.contains_key(runner) {
            blockers.push(format!("verification runner {runner} is not configured"));
        }
        if runner_ref
            .arguments
            .values()
            .any(|value| value.contains('{'))
        {
            blockers.push(format!(
                "verification target {verification} has unresolved runner arguments"
            ));
        }
    }
    ReadinessSubject {
        id: format!("criterion:{criterion}/verification"),
        scope_id: criterion_scope(criterion),
        required_level,
        ready: blockers.is_empty(),
        blockers,
    }
}

fn public_entrypoint_subjects(
    workspace: &SpecWorkspace,
    index: &SpecIndex,
    revision: &str,
    required_level: ReadinessLevel,
) -> Vec<ReadinessSubject> {
    // Language visibility is semantic inventory metadata, not an inferred
    // product contract. The readiness denominator must remain the explicit
    // `exposes` declarations: each one names an exact public entrypoint and
    // binds it to a capability target that can supply acceptance evidence.
    let unsupported_level = required_level > ReadinessLevel::WorkReady;
    index
        .exposes_by_target
        .iter()
        .map(|(target_ref, exposed_target)| {
            let mut blockers = Vec::new();
            if unsupported_level {
                blockers.push(
                    "public entrypoint probes above work-ready require explicit execution evidence"
                        .into(),
                );
            }
            let Some(identity) = index.target_to_artifact.get(target_ref) else {
                return subject(
                    format!("public:{target_ref}"),
                    public_entrypoints_scope(),
                    required_level,
                    false,
                    "explicit public entrypoint target does not resolve to one active semantic artifact",
                );
            };
            let owners = index
                .artifact_owners
                .get(identity)
                .cloned()
                .unwrap_or_default();
            let exact = owners
                .iter()
                .filter_map(|owner| {
                    let id = owner.target_id.as_ref()?;
                    let reference = BoundTargetRef {
                        binding: owner.binding.clone(),
                        target_id: id.clone(),
                    };
                    (index.target_to_artifact.get(&reference) == Some(identity))
                        .then_some(reference)
                })
                .collect::<Vec<_>>();
            if exact.len() != 1 {
                return subject(
                    format!("public:{target_ref}"),
                    public_entrypoints_scope(),
                    required_level,
                    false,
                    &format!(
                        "public artifact requires exactly one exact ArtifactTarget owner; found {}",
                        exact.len()
                    ),
                );
            }
            let target_ref = &exact[0];
            if index.target_to_artifact.get(target_ref) != Some(identity) {
                blockers
                    .push("exact ArtifactTarget does not resolve to the public artifact".into());
            }
            let criteria = index
                .criteria_to_implementation_targets
                .iter()
                .filter(|(_, targets)| targets.contains(exposed_target))
                .map(|(criterion, _)| criterion.clone())
                .collect::<Vec<_>>();
            if criteria.is_empty() {
                blockers.push(
                    "exposed capability target is not connected to a current criterion".into(),
                );
            }
            for criterion in criteria {
                match canonical_public_target_plan(
                    workspace,
                    index,
                    exposed_target,
                    &criterion,
                    revision,
                ) {
                    Ok(plan) if matches!(plan.status, PlanStatus::Ready) => {}
                    Ok(plan) => blockers.push(format!(
                        "{criterion} canonical public-entrypoint plan is {:?} with budgets {:?}: {}",
                        plan.status,
                        plan.slices
                            .iter()
                            .map(|slice| &slice.budget)
                            .collect::<Vec<_>>(),
                        plan.slices
                            .iter()
                            .flat_map(|slice| slice.blockers.iter())
                            .chain(plan.diagnostics.iter())
                            .map(|diagnostic| diagnostic.message.as_str())
                            .collect::<Vec<_>>()
                            .join("; ")
                    )),
                    Err(error) => blockers.push(format!(
                        "{criterion} canonical public-entrypoint plan failed: {error}"
                    )),
                }
            }
            ReadinessSubject {
                id: format!("public:{target_ref}"),
                scope_id: public_entrypoints_scope().into(),
                required_level,
                ready: blockers.is_empty(),
                blockers,
            }
        })
        .collect()
}

fn scope_matches(
    scope: &mitase_spec_model::OwnershipScope,
    unit: &mitase_inventory::ArtifactUnit,
) -> bool {
    if scope.adapter != unit.adapter {
        return false;
    }
    match &scope.selector {
        OwnershipSelector::File => {
            scope.path == unit.path && unit.kind == mitase_inventory::ArtifactUnitKind::File
        }
        OwnershipSelector::Module { name } => {
            scope.path == unit.path
                && (name == "*"
                    || unit.identity.contains(&format!("::{name}::"))
                    || unit.identity.ends_with(&format!("::{name}")))
        }
        OwnershipSelector::PathPrefix { value } => unit.path.as_path().starts_with(value.as_path()),
    }
}

fn canonical_public_target_plan(
    workspace: &SpecWorkspace,
    index: &SpecIndex,
    target: &BoundTargetRef,
    criterion: &SpecAnchor,
    revision: &str,
) -> Result<mitase_work_model::WorkPlan> {
    mitase_planner::plan_probe(
        &mitase_planner::PlanProbe {
            criterion: criterion.clone(),
            requested_targets: vec![target.clone()],
            max_slices: workspace
                .config
                .validation
                .readiness
                .limits
                .max_slices_per_origin,
        },
        workspace,
        index,
        revision,
    )
}

fn changed_artifact_paths(workspace: &SpecWorkspace, revision: &str) -> Result<BTreeSet<String>> {
    let root = &workspace.root;
    let baseline = readiness_change_baseline(workspace, revision)?;
    let mut paths = BTreeSet::new();
    // The same configured baseline is used for committed branch changes and
    // the two live working-tree views. This keeps readiness and workspace
    // validation aligned for committed, staged, unstaged, and untracked
    // changes.
    collect_git_paths(root, &["diff", "--name-only", &baseline], &mut paths)?;
    collect_git_paths(root, &["diff", "--name-only", "--cached"], &mut paths)?;
    collect_git_paths(root, &["diff", "--name-only"], &mut paths)?;
    collect_git_paths(
        root,
        &["ls-files", "--others", "--exclude-standard"],
        &mut paths,
    )?;
    Ok(paths)
}

fn readiness_change_baseline(workspace: &SpecWorkspace, revision: &str) -> Result<String> {
    let baseline = workspace.config.validation.changed.baseline.as_ref();
    match baseline {
        Some(mitase_project_model::ChangeBaseline::MergeBase { against }) => git_text(
            &workspace.root,
            &["merge-base", revision, against.0.as_str()],
        )
        .or_else(|_| git_text(&workspace.root, &["rev-parse", "HEAD^"]))
        .or_else(|_| Ok::<String, anyhow::Error>(revision.to_owned()))
        .context("resolve configured readiness merge-base"),
        Some(mitase_project_model::ChangeBaseline::Revision { revision }) => Ok(revision.0.clone()),
        Some(mitase_project_model::ChangeBaseline::Parent) => {
            git_text(&workspace.root, &["rev-parse", &format!("{revision}^")])
        }
        None => git_text(&workspace.root, &["merge-base", revision, "origin/main"])
            .or_else(|_| git_text(&workspace.root, &["rev-parse", &format!("{revision}^")]))
            .or_else(|_| Ok::<String, anyhow::Error>(revision.to_owned()))
            .context("resolve default readiness baseline"),
    }
}

fn git_text(root: &std::path::Path, args: &[&str]) -> Result<String> {
    let output = Command::new("git")
        .args(["-C", root.to_string_lossy().as_ref()])
        .args(args)
        .output()?;
    if !output.status.success() {
        anyhow::bail!("git command failed: git {}", args.join(" "));
    }
    let value = String::from_utf8(output.stdout)?.trim().to_owned();
    if value.is_empty() {
        anyhow::bail!("git command returned no value: git {}", args.join(" "));
    }
    Ok(value)
}

fn collect_git_paths(
    root: &std::path::Path,
    args: &[&str],
    paths: &mut BTreeSet<String>,
) -> Result<()> {
    let root_text = root.to_string_lossy().to_string();
    let output = Command::new("git")
        .args(["-C", root_text.as_str()])
        .args(args)
        .output()?;
    if output.status.success() {
        paths.extend(
            String::from_utf8_lossy(&output.stdout)
                .lines()
                .filter_map(|path| {
                    let path = path.trim();
                    (!path.is_empty()).then(|| path.to_owned())
                }),
        );
    }
    Ok(())
}

fn scope_level(
    workspace: &SpecWorkspace,
    criterion: &SpecAnchor,
    _index: &SpecIndex,
) -> ReadinessLevel {
    workspace
        .config
        .validation
        .readiness
        .probes
        .implemented_criteria
        .iter()
        .find(|probe| &probe.criterion == criterion)
        .map(|probe| probe.level)
        .unwrap_or(workspace.config.validation.readiness.target)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_axis_with_blocker_is_not_ready() {
        assert!(!ReadinessAxis::empty("no subjects").is_ready());
    }

    #[test]
    fn axis_counts_subjects_not_blockers() {
        let axis = axis_from_subjects(vec![ReadinessSubject {
            id: "one".into(),
            scope_id: repository_scope(),
            required_level: ReadinessLevel::Traceable,
            ready: false,
            blockers: vec!["a".into(), "b".into()],
        }]);
        assert_eq!(axis.required, 1);
        assert_eq!(axis.ready, 0);
        assert_eq!(axis.blockers.len(), 2);
    }

    #[test]
    fn readiness_levels_add_axes_monotonically() {
        assert_eq!(required_axes(ReadinessLevel::Traceable).len(), 2);
        assert_eq!(required_axes(ReadinessLevel::Seedable).len(), 3);
        assert_eq!(required_axes(ReadinessLevel::WorkReady).len(), 4);
        assert_eq!(required_axes(ReadinessLevel::Verifiable).len(), 5);
        assert_eq!(required_axes(ReadinessLevel::ClosedLoop).len(), 6);
    }

    #[test]
    fn scoped_readiness_requires_matching_scope_identity() {
        let axis = axis_from_subjects(vec![ReadinessSubject {
            id: "public".into(),
            scope_id: public_entrypoints_scope().into(),
            required_level: ReadinessLevel::Seedable,
            ready: true,
            blockers: vec![],
        }]);
        assert!(scoped_axis_is_ready(
            &axis,
            public_entrypoints_scope(),
            ReadinessLevel::Seedable
        ));
        assert!(!scoped_axis_is_ready(
            &axis,
            "typo-anything",
            ReadinessLevel::Seedable
        ));
    }

    fn current_public_entrypoint_blockers() -> &'static Vec<String> {
        static BLOCKERS: std::sync::OnceLock<Vec<String>> = std::sync::OnceLock::new();
        BLOCKERS.get_or_init(|| {
            let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .and_then(std::path::Path::parent)
                .expect("workspace root")
                .to_path_buf();
            let workspace = SpecWorkspace::load(root).expect("workspace");
            let index = workspace.index().expect("index");
            public_entrypoint_subjects(
                &workspace,
                &index,
                "readiness-test",
                ReadinessLevel::Seedable,
            )
            .into_iter()
            .filter(|subject| !subject.ready || !subject.blockers.is_empty())
            .flat_map(|subject| {
                subject
                    .blockers
                    .into_iter()
                    .map(move |blocker| format!("{}: {blocker}", subject.id))
            })
            .collect()
        })
    }

    #[test]
    fn all_current_public_entrypoints_have_exact_governance_and_canonical_plans() {
        let blockers = current_public_entrypoint_blockers();
        assert!(
            blockers.is_empty(),
            "public entrypoint blockers: {blockers:?}",
        );
    }
}
