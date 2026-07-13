use anyhow::{Context, Result};
use serde::Serialize;
use std::process::Command;
use syu_project_model::ReadinessLevel;
use syu_spec_model::{BindingRole, ItemStatus, OwnershipSelector, TargetClaim};
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
    pub fn empty(message: impl Into<String>) -> Self {
        Self {
            required: 0,
            ready: 0,
            blockers: vec![message.into()],
        }
    }

    pub fn is_ready(&self) -> bool {
        self.ready == self.required && self.blockers.is_empty()
    }
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

    let active_identities = active
        .iter()
        .map(|unit| unit.identity.as_str())
        .collect::<Vec<_>>();
    let ownership_blockers = active_identities
        .iter()
        .filter_map(|identity| {
            let owners = index
                .artifact_owners
                .get(*identity)
                .map(Vec::len)
                .unwrap_or(0);
            (owners != 1).then(|| format!("{identity} has {owners} owners"))
        })
        .collect::<Vec<_>>();
    let mut ownership_blockers = ownership_blockers;
    for (binding_anchor, binding) in &index.bindings {
        if binding.targets.len()
            > workspace
                .config
                .validation
                .readiness
                .limits
                .max_targets_per_binding
        {
            ownership_blockers.push(format!(
                "{binding_anchor} has {} targets, exceeding max_targets_per_binding",
                binding.targets.len()
            ));
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
                ownership_blockers.push(format!(
                    "{binding_anchor}/scope.{} covers {matched} active units, exceeding max_ownership_scope_units",
                    scope.id
                ));
            }
        }
    }
    let ownership = if active.is_empty() {
        ReadinessAxis::empty("no declared artifact subjects")
    } else {
        ReadinessAxis {
            required: active.len(),
            ready: active.len().saturating_sub(ownership_blockers.len()),
            blockers: ownership_blockers,
        }
    };

    let configured_criteria = workspace
        .config
        .validation
        .readiness
        .probes
        .implemented_criteria
        .as_deref();
    let criteria = index
        .criterion_status
        .iter()
        .filter(|(anchor, status)| {
            **status == ItemStatus::Implemented
                && configured_criteria.is_none_or(|selection| {
                    selection == "all"
                        || selection
                            .split(',')
                            .map(str::trim)
                            .any(|candidate| candidate == anchor.to_string())
                })
        })
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
        let implementation_facets = index
            .criteria_to_implementation_targets
            .get(criterion)
            .into_iter()
            .flatten()
            .filter_map(|target| {
                index
                    .bindings
                    .get(&target.binding)
                    .map(|binding| binding.facet.as_str())
            })
            .collect::<std::collections::BTreeSet<_>>();
        if workspace
            .config
            .validation
            .readiness
            .probes
            .contracts
            .as_deref()
            .is_some_and(|selection| selection == "all")
            && implementation_facets.len() > 1
            && index
                .criteria_to_implementation_targets
                .get(criterion)
                .into_iter()
                .flatten()
                .any(|target| !index.contracts_by_target.contains_key(target))
        {
            work_blockers.push(format!(
                "{criterion}: implementation target is not closed by a contract"
            ));
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
                    if args.iter().any(|argument| argument.contains('{')) {
                        criterion_executed = false;
                        closed_loop_blockers.push(format!(
                            "{criterion}: runner {} has unresolved arguments",
                            claim.runner
                        ));
                        continue;
                    }
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
                && (unit.identity.contains(&format!("::{name}::"))
                    || unit.identity.ends_with(&format!("::{name}")))
        }
        OwnershipSelector::PathPrefix { value } => unit.path.as_path().starts_with(value.as_path()),
    }
}

fn probe_runner(
    executable: &str,
    arguments: &[String],
    root: &std::path::Path,
) -> std::io::Result<std::process::Output> {
    let mut command = Command::new(executable);
    command.args(arguments).current_dir(root);
    // Readiness may itself run from inside Cargo's test harness. Give nested
    // Cargo probes an isolated target directory so the probe exercises the
    // configured command without contending for the parent Cargo lock.
    if executable == "cargo" {
        command.env(
            "CARGO_TARGET_DIR",
            std::env::temp_dir().join(format!("syu-readiness-{}", std::process::id())),
        );
    }
    command.output()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_axis_with_blocker_is_not_ready() {
        assert!(!ReadinessAxis::empty("no subjects").is_ready());
    }

    #[test]
    fn readiness_levels_add_axes_monotonically() {
        assert_eq!(required_axes(ReadinessLevel::Traceable).len(), 2);
        assert_eq!(required_axes(ReadinessLevel::Seedable).len(), 3);
        assert_eq!(required_axes(ReadinessLevel::WorkReady).len(), 4);
        assert_eq!(required_axes(ReadinessLevel::Verifiable).len(), 5);
        assert_eq!(required_axes(ReadinessLevel::ClosedLoop).len(), 6);
    }
}
