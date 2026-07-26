use anyhow::{Context, Result};
use serde::Serialize;
use std::collections::BTreeSet;
use std::process::Command;
use syu_project_model::{ProjectConfig, ReadinessLevel};
use syu_spec_model::{BoundTargetRef, ItemStatus, OwnershipSelector, SpecAnchor, TargetClaim};
use syu_work_model::{
    PlanStatus, RequestedTarget, TargetTransition, VerificationReceipt, WORK_REQUEST_SCHEMA,
    WorkOperation, WorkRequest, WorkSeed,
};
use syu_workspace::{SpecIndex, SpecWorkspace};

/// A readiness count is a count of these subjects, never a subtraction of
/// blocker strings. This keeps one subject with several blockers from being
/// counted several times.
#[derive(Debug, Clone, Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct ReadinessSubject {
    pub id: String,
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
    pub target: String,
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

    /// Evaluate both the workspace-wide readiness target and every explicitly
    /// configured facet scope. A repository can therefore remain traceable
    /// overall while requiring a bounded Workbench facet to be closed-loop.
    pub fn meets_configured(&self, config: &ProjectConfig) -> bool {
        if !self.meets(config.validation.readiness.target) {
            return false;
        }
        config
            .validation
            .readiness
            .scopes
            .values()
            .copied()
            .filter(|level| *level != ReadinessLevel::Off)
            .all(|level| {
                required_axes(level).iter().all(|axis| match axis {
                    ReadinessAxisId::Inventory => self.inventory.is_ready(),
                    ReadinessAxisId::Ownership => self.ownership.is_ready(),
                    ReadinessAxisId::Seedability => scoped_axis_is_ready(&self.seedability, level),
                    ReadinessAxisId::Workability => scoped_axis_is_ready(&self.workability, level),
                    ReadinessAxisId::Verification => {
                        scoped_axis_is_ready(&self.verification, level)
                    }
                    ReadinessAxisId::ClosedLoop => scoped_axis_is_ready(&self.closed_loop, level),
                })
            })
    }
}

fn scoped_axis_is_ready(axis: &ReadinessAxis, level: ReadinessLevel) -> bool {
    let subjects = axis
        .subjects
        .iter()
        .filter(|subject| subject.required_level == level)
        .collect::<Vec<_>>();
    !subjects.is_empty()
        && subjects
            .iter()
            .all(|subject| subject.ready && subject.blockers.is_empty())
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
                syu_inventory::ArtifactReachability::Active
            ) && unit.exposure != syu_inventory::ArtifactExposure::Support
        })
        .collect::<Vec<_>>();

    let inventory_subjects = if let Some(error) = &index.inventory_error {
        vec![ReadinessSubject {
            id: "inventory:error".into(),
            required_level: ReadinessLevel::Traceable,
            ready: false,
            blockers: vec![error.clone()],
        }]
    } else {
        active
            .iter()
            .map(|unit| ReadinessSubject {
                id: format!("inventory:{}", unit.identity),
                required_level: ReadinessLevel::Traceable,
                ready: true,
                blockers: vec![],
            })
            .collect()
    };
    let inventory = axis_from_subjects(inventory_subjects);

    // When readiness is configured for a bounded criterion set, ownership is
    // evaluated for the exact implementation/verification artifacts selected
    // by those criteria. This keeps a repository-wide planned self-hosting
    // catalog from becoming an artificial ownership denominator.
    let criteria = implemented_criteria(workspace, index);
    let ownership_focus = workspace
        .config
        .validation
        .readiness
        .probes
        .implemented_criteria
        .as_deref()
        .filter(|selection| *selection != "all")
        .map(|_| {
            criteria
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
                .collect::<BTreeSet<_>>()
        });

    let ownership_subjects = active
        .iter()
        .filter(|unit| {
            ownership_focus
                .as_ref()
                .is_none_or(|focused| focused.contains(&unit.identity))
        })
        .map(|unit| {
            let owners = index
                .artifact_owners
                .get(&unit.identity)
                .cloned()
                .unwrap_or_default();
            let mut blockers = if owners.len() == 1 {
                vec![]
            } else {
                vec![format!("{} has {} owners", unit.identity, owners.len())]
            };
            for owner in &owners {
                if let Some(binding) = index.bindings.get(&owner.binding)
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
            ReadinessSubject {
                id: format!("ownership:{}", unit.identity),
                required_level: ReadinessLevel::Traceable,
                ready: blockers.is_empty(),
                blockers,
            }
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
                .filter(|unit| {
                    ownership_focus
                        .as_ref()
                        .is_none_or(|focused| focused.contains(&unit.identity))
                })
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
        .as_deref()
        .is_some_and(|value| value == "all");
    let contracts_probe = workspace
        .config
        .validation
        .readiness
        .probes
        .contracts
        .as_deref()
        .is_some_and(|value| value == "all");

    let mut seed_subjects = Vec::new();
    let mut work_subjects = Vec::new();
    let mut verification_subjects = Vec::new();
    let mut closed_subjects = Vec::new();
    let mut execution_jobs = Vec::new();

    for criterion in &criteria {
        let implementation_targets = index
            .criteria_to_implementation_targets
            .get(criterion)
            .cloned()
            .unwrap_or_default();
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
                required_level,
                false,
                "criterion has no exact implementation target",
            ));
        } else {
            for target_ref in &implementation_targets {
                let exact = index.target_to_artifact.contains_key(target_ref);
                let target_plan_ready = plan_ready
                    && plan.as_ref().is_ok_and(|plan| {
                        plan.slices.iter().any(|slice| {
                            slice
                                .editable_targets
                                .iter()
                                .any(|target| &target.reference == target_ref)
                                && !slice.verification_targets.is_empty()
                        })
                    });
                let mut seed_blockers = Vec::new();
                if !exact {
                    seed_blockers.push(
                        "implementation target does not resolve to one exact artifact".into(),
                    );
                }
                if !target_plan_ready {
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
                    let mut closed = subject(
                        closed_id.clone(),
                        required_level,
                        false,
                        if !execute_verification {
                            "verification execution was not run"
                        } else if !work_ready {
                            "structural verification closure is not ready"
                        } else {
                            "canonical receipt and post-state validation have not passed"
                        },
                    );
                    if execute_verification
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
    // This keeps the Syu capability catalog in the denominator once a feature
    // is declared implemented, even when the feature has no requirement
    // criterion of its own.
    seed_subjects.extend(implemented_feature_subjects(workspace, index));

    if public_probe {
        let public_subjects = public_entrypoint_subjects(workspace, index, &active, revision);
        seed_subjects.extend(public_subjects);
    }

    if contracts_probe {
        if index.contracts.is_empty() {
            let empty = subject(
                "contracts:active".into(),
                ReadinessLevel::Seedable,
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
                                    "canonical contract seed produced no contract closure".into(),
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
                            blockers.push(format!("canonical contract seed failed: {error}"))
                        }
                    }
                }
                let ready = blockers.is_empty();
                seed_subjects.push(ReadinessSubject {
                    id: format!("contract:{anchor}"),
                    required_level: ReadinessLevel::Seedable,
                    ready,
                    blockers: blockers.clone(),
                });
                work_subjects.push(ReadinessSubject {
                    id: format!("contract:{anchor}/plan"),
                    required_level: ReadinessLevel::WorkReady,
                    ready,
                    blockers,
                });
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
                                matches!(diagnostic.severity, syu_diagnostics::Severity::Error)
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
        target: readiness_label(workspace.config.validation.readiness.target).into(),
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
) -> Result<syu_work_model::WorkPlan> {
    syu_planner::plan(
        &WorkRequest {
            schema: WORK_REQUEST_SCHEMA.into(),
            id: format!("readiness-contract-{}", contract.local_id),
            summary: "canonical contract closure plan probe".into(),
            operation: WorkOperation::Modify,
            seeds: vec![WorkSeed::Anchor(contract.clone())],
            constraints: syu_work_model::WorkConstraints {
                max_slices: Some(
                    workspace
                        .config
                        .validation
                        .readiness
                        .limits
                        .max_slices_per_seed,
                ),
                ..Default::default()
            },
            requested_targets: vec![],
        },
        workspace,
        index,
        revision,
    )
}

fn subject(
    id: String,
    required_level: ReadinessLevel,
    ready: bool,
    blocker: &str,
) -> ReadinessSubject {
    ReadinessSubject {
        id,
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

fn implemented_criteria(workspace: &SpecWorkspace, index: &SpecIndex) -> Vec<SpecAnchor> {
    let selection = workspace
        .config
        .validation
        .readiness
        .probes
        .implemented_criteria
        .as_deref();
    index
        .criterion_status
        .iter()
        .filter(|(anchor, status)| {
            **status == ItemStatus::Implemented
                && selection.is_none_or(|selection| {
                    selection == "all"
                        || selection
                            .split(',')
                            .map(str::trim)
                            .any(|candidate| candidate == anchor.to_string())
                })
        })
        .map(|(anchor, _)| anchor.clone())
        .collect()
}

fn implemented_feature_subjects(
    workspace: &SpecWorkspace,
    index: &SpecIndex,
) -> Vec<ReadinessSubject> {
    workspace
        .documents
        .iter()
        .flat_map(|loaded| match &loaded.document {
            syu_spec_model::SpecDocument::Features { features, .. } => features.clone(),
            _ => Vec::new(),
        })
        .filter(|feature| feature.status == ItemStatus::Implemented)
        .map(|feature| {
            let bindings = index
                .bindings
                .iter()
                .filter(|(anchor, binding)| {
                    anchor.item == feature.id
                        && binding.role == syu_spec_model::BindingRole::Implementation
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
) -> Result<syu_work_model::WorkPlan> {
    syu_planner::plan(
        &WorkRequest {
            schema: WORK_REQUEST_SCHEMA.into(),
            id: format!("readiness-{}", criterion.local_id),
            summary: "canonical readiness plan probe".into(),
            operation: WorkOperation::Modify,
            seeds: vec![WorkSeed::Anchor(criterion.clone())],
            constraints: syu_work_model::WorkConstraints {
                max_slices: Some(
                    workspace
                        .config
                        .validation
                        .readiness
                        .limits
                        .max_slices_per_seed,
                ),
                ..Default::default()
            },
            requested_targets: vec![],
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
            index.target(verification).is_some_and(|target| {
                target.claims.iter().any(|claim| {
                    matches!(claim, TargetClaim::Verifies { criterion: actual, covers, .. } if actual == criterion && !covers.is_empty() && covers.contains(implementation))
                })
            })
        });
        if !covered {
            blockers.push(format!(
                "no exact verification target covers {implementation}"
            ));
        }
    }
    for verification in verifications {
        let Some(target) = index.target(verification) else {
            blockers.push(format!("verification target {verification} is unresolved"));
            continue;
        };
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
        let claims = target
            .claims
            .iter()
            .filter_map(|claim| match claim {
                TargetClaim::Verifies {
                    criterion: actual,
                    runner,
                    covers,
                } if actual == criterion => Some((runner, covers)),
                _ => None,
            })
            .collect::<Vec<_>>();
        if claims.len() != 1 {
            blockers.push(format!(
                "verification target {verification} must have one exact claim"
            ));
            continue;
        }
        let runner = &claims[0].0.runner;
        if !workspace.config.verification.runners.contains_key(runner) {
            blockers.push(format!("verification runner {runner} is not configured"));
        }
        if claims[0]
            .0
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
        required_level,
        ready: blockers.is_empty(),
        blockers,
    }
}

fn public_entrypoint_subjects(
    workspace: &SpecWorkspace,
    index: &SpecIndex,
    active: &[&syu_inventory::ArtifactUnit],
    revision: &str,
) -> Vec<ReadinessSubject> {
    active
        .iter()
        .filter(|unit| unit.exposure == syu_inventory::ArtifactExposure::Public)
        .map(|unit| {
            let mut blockers = Vec::new();
            let owners = index
                .artifact_owners
                .get(&unit.identity)
                .cloned()
                .unwrap_or_default();
            let exact = owners.iter().find_map(|owner| {
                let id = owner.target_id.as_ref()?;
                let reference = BoundTargetRef {
                    binding: owner.binding.clone(),
                    target_id: id.clone(),
                };
                (index.target_to_artifact.get(&reference) == Some(&unit.identity))
                    .then_some(reference)
            });
            let Some(target_ref) = exact else {
                return subject(
                    format!("public:{}", unit.identity),
                    ReadinessLevel::Seedable,
                    false,
                    "public artifact requires an exact ArtifactTarget owner",
                );
            };
            if index.target_to_artifact.get(&target_ref) != Some(&unit.identity) {
                blockers
                    .push("exact ArtifactTarget does not resolve to the public artifact".into());
            }
            let criteria = index
                .criteria_to_implementation_targets
                .iter()
                .filter(|(_, targets)| targets.contains(&target_ref))
                .map(|(criterion, _)| criterion.clone())
                .collect::<Vec<_>>();
            if criteria.is_empty() {
                blockers.push("public artifact is not connected to a criterion".into());
            }
            for criterion in criteria {
                match canonical_public_target_plan(
                    workspace,
                    index,
                    &target_ref,
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
                id: format!("public:{}", unit.identity),
                required_level: ReadinessLevel::Seedable,
                ready: blockers.is_empty(),
                blockers,
            }
        })
        .collect()
}

fn scope_matches(
    scope: &syu_spec_model::OwnershipScope,
    unit: &syu_inventory::ArtifactUnit,
) -> bool {
    if scope.adapter != unit.adapter {
        return false;
    }
    match &scope.selector {
        OwnershipSelector::File => {
            scope.path == unit.path && unit.kind == syu_inventory::ArtifactUnitKind::File
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
) -> Result<syu_work_model::WorkPlan> {
    syu_planner::plan(
        &WorkRequest {
            schema: WORK_REQUEST_SCHEMA.into(),
            id: format!("readiness-public-{}", target.target_id),
            summary: "canonical public entrypoint plan probe".into(),
            operation: WorkOperation::Modify,
            seeds: vec![],
            constraints: syu_work_model::WorkConstraints {
                max_slices: Some(
                    workspace
                        .config
                        .validation
                        .readiness
                        .limits
                        .max_slices_per_seed,
                ),
                ..Default::default()
            },
            requested_targets: vec![RequestedTarget {
                reference: target.clone(),
                criterion: Some(criterion.clone()),
                transition: TargetTransition::Modify,
            }],
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
        Some(syu_project_model::ChangeBaseline::MergeBase { against }) => git_text(
            &workspace.root,
            &["merge-base", revision, against.0.as_str()],
        )
        .or_else(|_| git_text(&workspace.root, &["rev-parse", "HEAD^"]))
        .or_else(|_| Ok::<String, anyhow::Error>(revision.to_owned()))
        .context("resolve configured readiness merge-base"),
        Some(syu_project_model::ChangeBaseline::Revision { revision }) => Ok(revision.0.clone()),
        Some(syu_project_model::ChangeBaseline::Parent) => {
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

fn readiness_label(level: ReadinessLevel) -> &'static str {
    match level {
        ReadinessLevel::Off => "off",
        ReadinessLevel::Traceable => "traceable",
        ReadinessLevel::Seedable => "seedable",
        ReadinessLevel::WorkReady => "work-ready",
        ReadinessLevel::Verifiable => "verifiable",
        ReadinessLevel::ClosedLoop => "closed-loop",
    }
}

fn scope_level(
    workspace: &SpecWorkspace,
    criterion: &SpecAnchor,
    index: &SpecIndex,
) -> ReadinessLevel {
    let default = workspace.config.validation.readiness.target;
    index
        .criteria_to_implementation_targets
        .get(criterion)
        .into_iter()
        .flatten()
        .filter_map(|target| {
            let facet = index.bindings.get(&target.binding)?.facet.as_str();
            Some(
                workspace
                    .config
                    .validation
                    .readiness
                    .scopes
                    .get(facet)
                    .copied()
                    .unwrap_or(default),
            )
        })
        .max()
        .unwrap_or(default)
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
            let active = index
                .artifact_units
                .iter()
                .filter(|unit| {
                    matches!(
                        unit.reachability,
                        syu_inventory::ArtifactReachability::Active
                    ) && unit.exposure != syu_inventory::ArtifactExposure::Support
                })
                .collect::<Vec<_>>();
            public_entrypoint_subjects(&workspace, &index, &active, "readiness-test")
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

    fn assert_current_public_entrypoints_have_canonical_plans() {
        let blockers = current_public_entrypoint_blockers();
        assert!(
            blockers.is_empty(),
            "public entrypoint blockers: {blockers:?}",
        );
    }

    #[test]
    fn workbench_client_transport_public_entrypoints_have_canonical_plans() {
        assert_current_public_entrypoints_have_canonical_plans();
    }

    #[test]
    fn workbench_client_actions_public_entrypoints_have_canonical_plans() {
        assert_current_public_entrypoints_have_canonical_plans();
    }

    #[test]
    fn workbench_components_public_entrypoints_have_canonical_plans() {
        assert_current_public_entrypoints_have_canonical_plans();
    }

    #[test]
    fn workbench_pages_public_entrypoints_have_canonical_plans() {
        assert_current_public_entrypoints_have_canonical_plans();
    }

    #[test]
    fn workbench_navigation_public_entrypoints_have_canonical_plans() {
        assert_current_public_entrypoints_have_canonical_plans();
    }

    #[test]
    fn workbench_rendering_public_entrypoints_have_canonical_plans() {
        assert_current_public_entrypoints_have_canonical_plans();
    }

    #[test]
    fn code_diagnostics_public_entrypoints_have_canonical_plans() {
        assert_current_public_entrypoints_have_canonical_plans();
    }

    #[test]
    fn inventory_discovery_public_entrypoints_have_canonical_plans() {
        assert_current_public_entrypoints_have_canonical_plans();
    }

    #[test]
    fn specification_model_public_entrypoints_have_canonical_plans() {
        assert_current_public_entrypoints_have_canonical_plans();
    }

    #[test]
    fn validation_engine_public_entrypoints_have_canonical_plans() {
        assert_current_public_entrypoints_have_canonical_plans();
    }

    #[test]
    fn work_planning_public_entrypoints_have_canonical_plans() {
        assert_current_public_entrypoints_have_canonical_plans();
    }

    #[test]
    fn workbench_server_lifecycle_public_entrypoints_have_canonical_plans() {
        assert_current_public_entrypoints_have_canonical_plans();
    }

    #[test]
    fn workbench_server_validation_public_entrypoints_have_canonical_plans() {
        assert_current_public_entrypoints_have_canonical_plans();
    }

    #[test]
    fn workspace_loading_public_entrypoints_have_canonical_plans() {
        assert_current_public_entrypoints_have_canonical_plans();
    }

    #[test]
    fn workspace_resolution_public_entrypoints_have_canonical_plans() {
        assert_current_public_entrypoints_have_canonical_plans();
    }

    #[test]
    fn agent_delivery_public_entrypoints_have_canonical_plans() {
        assert_current_public_entrypoints_have_canonical_plans();
    }
}
