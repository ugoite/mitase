use anyhow::{Context, Result};
use serde::Serialize;
use std::process::Command;
use syu_project_model::ReadinessLevel;
use syu_spec_model::{BindingRole, ItemStatus, TargetClaim};
use syu_workspace::{SpecIndex, SpecWorkspace};

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ReadinessAxis {
    pub required: usize,
    pub ready: usize,
    pub blockers: Vec<String>,
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
}

impl ReadinessAxis {
    fn empty(message: impl Into<String>) -> Self {
        Self {
            required: 0,
            ready: 0,
            blockers: vec![message.into()],
        }
    }
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
            )
        })
        .collect::<Vec<_>>();
    let inventory = if let Some(error) = &index.inventory_error {
        ReadinessAxis {
            required: 1,
            ready: 0,
            blockers: vec![error.clone()],
        }
    } else if active.is_empty() {
        ReadinessAxis::empty("active inventory has no subjects")
    } else {
        ReadinessAxis {
            required: active.len(),
            ready: active.len(),
            blockers: vec![],
        }
    };

    let claimed = index
        .target_to_artifact
        .keys()
        .filter(|target| {
            index
                .bindings
                .get(&target.binding)
                .is_some_and(|binding| binding.role == BindingRole::Implementation)
        })
        .cloned()
        .collect::<Vec<_>>();
    let ownership_blockers = claimed
        .iter()
        .filter_map(|target| {
            let identity = index.target_to_artifact.get(target)?;
            let owners = index
                .artifact_owners
                .get(identity)
                .map(Vec::len)
                .unwrap_or(0);
            (owners != 1).then(|| format!("{identity} has {owners} owners"))
        })
        .collect::<Vec<_>>();
    let ownership = if claimed.is_empty() {
        ReadinessAxis::empty("no declared artifact subjects")
    } else {
        ReadinessAxis {
            required: claimed.len(),
            ready: claimed.len().saturating_sub(ownership_blockers.len()),
            blockers: ownership_blockers,
        }
    };

    let criteria = index
        .criterion_status
        .iter()
        .filter(|(_, status)| **status == ItemStatus::Implemented)
        .map(|(anchor, _)| anchor.clone())
        .filter(|criterion| scope_level(workspace, criterion, index) >= ReadinessLevel::Seedable)
        .collect::<Vec<_>>();
    let mut seed_blockers = Vec::new();
    let mut work_blockers = Vec::new();
    for criterion in &criteria {
        let request = syu_work_model::WorkRequest {
            schema: syu_work_model::WORK_REQUEST_SCHEMA.into(),
            id: format!("readiness-{}", criterion.local_id),
            summary: "canonical readiness probe".into(),
            operation: syu_work_model::WorkOperation::Modify,
            seeds: vec![syu_work_model::WorkSeed::Anchor(criterion.clone())],
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
        };
        match syu_planner::plan(&request, workspace, index, revision) {
            Ok(plan)
                if matches!(plan.status, syu_work_model::PlanStatus::Ready)
                    && !plan.slices.is_empty() =>
            {
                if plan.slices.len()
                    > workspace
                        .config
                        .validation
                        .readiness
                        .limits
                        .max_slices_per_seed
                {
                    work_blockers.push(format!("{criterion} exceeds max_slices_per_seed"));
                }
            }
            Ok(plan) => {
                seed_blockers.push(format!("{criterion}: {:?}", plan.status));
                work_blockers.push(format!("{criterion}: no canonical work slice"));
            }
            Err(error) => {
                seed_blockers.push(format!("{criterion}: {error}"));
                work_blockers.push(format!("{criterion}: {error}"));
            }
        }
    }
    let seedability = axis_for(criteria.len(), seed_blockers);
    let workability = axis_for(criteria.len(), work_blockers);

    let mut verification_blockers = Vec::new();
    let mut closed_loop_blockers = Vec::new();
    let mut verification_ready = 0;
    let mut closed_loop_ready = 0;
    for criterion in &criteria {
        let implementations = index
            .criteria_to_implementation_targets
            .get(criterion)
            .cloned()
            .unwrap_or_default();
        let verifications = index
            .criteria_to_verification_targets
            .get(criterion)
            .cloned()
            .unwrap_or_default();
        let mut criterion_ready = !implementations.is_empty() && !verifications.is_empty();
        let mut criterion_executed = criterion_ready;
        if !criterion_ready {
            verification_blockers.push(format!(
                "{criterion}: missing implementation or verification target"
            ));
        }
        for implementation in &implementations {
            let covered = verifications.iter().filter(|verification| {
                let Some(binding) = index.bindings.get(&verification.binding) else { return false; };
                binding.role == BindingRole::Verification && binding.targets.iter().find(|target| target.id == verification.target_id).is_some_and(|target| target.claims.iter().any(|claim| matches!(claim, TargetClaim::Verifies { criterion: actual, covers, .. } if actual == criterion && covers.contains(implementation))))
            }).collect::<Vec<_>>();
            if covered.is_empty() {
                criterion_ready = false;
                criterion_executed = false;
                verification_blockers.push(format!(
                    "{criterion}: no exact verification covers {implementation}"
                ));
            }
            for verification in covered {
                let target = index
                    .target(verification)
                    .context("verification target disappeared from index")?;
                let claim = target
                    .claims
                    .iter()
                    .find_map(|claim| match claim {
                        TargetClaim::Verifies {
                            criterion: actual,
                            runner,
                            ..
                        } if *actual == *criterion => Some(runner),
                        _ => None,
                    })
                    .context("verification target has no verification claim")?;
                let runner = workspace.config.verification.runners.get(&claim.runner);
                if runner.is_none() {
                    criterion_ready = false;
                    criterion_executed = false;
                    verification_blockers.push(format!(
                        "{criterion}: runner {} is not configured",
                        claim.runner
                    ));
                } else if execute_verification && let Some(runner) = runner {
                    let args = runner
                        .arguments
                        .iter()
                        .map(|argument| expand(argument, &claim.arguments))
                        .collect::<Vec<_>>();
                    let output = probe_runner(&runner.executable, &args, &workspace.root);
                    if !output.is_ok_and(|output| output.status.success()) {
                        criterion_executed = false;
                        closed_loop_blockers.push(format!(
                            "{criterion}: verification runner {} failed",
                            claim.runner
                        ));
                    }
                }
            }
        }
        if criterion_ready {
            verification_ready += 1;
        }
        if criterion_ready && criterion_executed {
            closed_loop_ready += 1;
        }
    }
    let verification = axis_for(criteria.len(), verification_blockers);
    let closed_loop = axis_for(criteria.len(), closed_loop_blockers);
    Ok(ReadinessReport {
        target: readiness_label(workspace.config.validation.readiness.target).into(),
        inventory,
        ownership,
        seedability,
        workability,
        verification: ReadinessAxis {
            ready: verification_ready,
            ..verification
        },
        closed_loop: ReadinessAxis {
            ready: closed_loop_ready,
            ..closed_loop
        },
    })
}

fn probe_runner(
    executable: &str,
    _arguments: &[String],
    root: &std::path::Path,
) -> std::io::Result<std::process::Output> {
    Command::new(executable)
        .arg("--version")
        .current_dir(root)
        .output()
}

fn axis_for(required: usize, blockers: Vec<String>) -> ReadinessAxis {
    ReadinessAxis {
        required,
        ready: required.saturating_sub(blockers.len()),
        blockers,
    }
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
    criterion: &syu_spec_model::SpecAnchor,
    index: &SpecIndex,
) -> ReadinessLevel {
    let facet = index
        .criteria_to_implementation_targets
        .get(criterion)
        .and_then(|targets| targets.first())
        .and_then(|target| index.bindings.get(&target.binding))
        .map(|binding| binding.facet.as_str());
    facet
        .and_then(|facet| {
            workspace
                .config
                .validation
                .readiness
                .scopes
                .get(facet)
                .copied()
        })
        .unwrap_or(workspace.config.validation.readiness.target)
}

fn expand(template: &str, values: &std::collections::BTreeMap<String, String>) -> String {
    values
        .iter()
        .fold(template.to_owned(), |value, (key, replacement)| {
            value.replace(&format!("{{{key}}}"), replacement)
        })
}
