use anyhow::{Context, Result};
use serde::Serialize;
use std::process::Command;
use syu_project_model::ReadinessLevel;
use syu_spec_model::{BindingRole, ItemStatus, OwnershipSelector, TargetClaim};
use syu_workspace::{SpecIndex, SpecWorkspace};

#[derive(Debug, Clone, Serialize, serde::Deserialize, PartialEq, Eq)]
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
    pub receipts: Vec<ReadinessReceipt>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ReadinessReceipt {
    pub criterion: String,
    pub plan_slice: Option<String>,
    pub runner: String,
    pub command_succeeded: bool,
    pub validation_passed: bool,
    pub poststate_fingerprint: String,
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
        .collect::<Vec<_>>();
    let mut seed_blockers = Vec::new();
    let mut work_blockers = Vec::new();
    if workspace
        .config
        .validation
        .readiness
        .probes
        .public_entrypoints
        .as_deref()
        .is_some_and(|selection| selection == "all")
    {
        for unit in active
            .iter()
            .filter(|unit| unit.exposure == syu_inventory::ArtifactExposure::Public)
        {
            if !index.artifact_owners.contains_key(&unit.identity) {
                seed_blockers.push(format!(
                    "{}: public entrypoint has no canonical owner",
                    unit.identity
                ));
            }
        }
    }
    if workspace
        .config
        .validation
        .readiness
        .probes
        .contracts
        .as_deref()
        .is_some_and(|selection| selection == "all")
    {
        for (anchor, contract) in &index.contracts {
            let mut participants = std::iter::once(&contract.source).chain(
                contract
                    .participants
                    .iter()
                    .map(|participant| &participant.target),
            );
            if participants.any(|target| !index.target_to_artifact.contains_key(target)) {
                work_blockers.push(format!(
                    "{anchor}: contract target is not exact and seedable"
                ));
            }
        }
    }
    if workspace.config.validation.readiness.probes.changed_units {
        for path in changed_artifact_paths(&workspace.root, revision)? {
            let changed_units = active
                .iter()
                .filter(|unit| unit.path.to_string_lossy() == path)
                .filter(|unit| !index.artifact_owners.contains_key(&unit.identity));
            for unit in changed_units {
                seed_blockers.push(format!(
                    "{}: changed unit has no canonical owner",
                    unit.identity
                ));
            }
        }
    }
    for criterion in &criteria {
        let level = scope_level(workspace, criterion, index);
        if level < ReadinessLevel::Seedable {
            continue;
        }
        let work_required = level >= ReadinessLevel::WorkReady;
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
                    && work_required
                {
                    work_blockers.push(format!("{criterion} exceeds max_slices_per_seed"));
                }
            }
            Ok(plan) => {
                seed_blockers.push(format!("{criterion}: {:?}", plan.status));
                if work_required {
                    work_blockers.push(format!("{criterion}: no canonical work slice"));
                }
            }
            Err(error) => {
                seed_blockers.push(format!("{criterion}: {error}"));
                if work_required {
                    work_blockers.push(format!("{criterion}: {error}"));
                }
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
            && work_required
        {
            work_blockers.push(format!(
                "{criterion}: implementation target is not closed by a contract"
            ));
        }
    }
    let seed_required = criteria
        .iter()
        .filter(|criterion| scope_level(workspace, criterion, index) >= ReadinessLevel::Seedable)
        .count();
    let work_required = criteria
        .iter()
        .filter(|criterion| scope_level(workspace, criterion, index) >= ReadinessLevel::WorkReady)
        .count();
    let seedability = axis_for(seed_required, seed_blockers);
    let workability = axis_for(work_required, work_blockers);

    let mut verification_blockers = Vec::new();
    let mut closed_loop_blockers = Vec::new();
    let mut receipts = Vec::new();
    let mut verification_ready = 0;
    let mut closed_loop_ready = 0;
    for criterion in &criteria {
        let level = scope_level(workspace, criterion, index);
        if level < ReadinessLevel::Verifiable {
            continue;
        }
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
        let plan_slice = syu_planner::plan(
            &syu_work_model::WorkRequest {
                schema: syu_work_model::WORK_REQUEST_SCHEMA.into(),
                id: format!("readiness-receipt-{}", criterion.local_id),
                summary: "canonical readiness receipt probe".into(),
                operation: syu_work_model::WorkOperation::Modify,
                seeds: vec![syu_work_model::WorkSeed::Anchor(criterion.clone())],
                constraints: Default::default(),
                requested_targets: vec![],
            },
            workspace,
            index,
            revision,
        )
        .ok()
        .and_then(|plan| plan.slices.first().map(|slice| slice.id.clone()));
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
                } else if execute_verification
                    && level >= ReadinessLevel::ClosedLoop
                    && let Some(runner) = runner
                {
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
                    let command_succeeded =
                        output.as_ref().is_ok_and(|output| output.status.success());
                    let validation = crate::validate_without_readiness(&crate::ValidationContext {
                        config: &workspace.config,
                        workspace,
                        index,
                        changed_files: None,
                        reported_changed_files: None,
                        work_plan: None,
                        selected_slice: None,
                        plan_mode: crate::PlanValidationMode::PostState,
                        preset: workspace.config.validation.preset,
                        revision: Some(revision),
                        change_base_revision: None,
                    });
                    let validation_passed = command_succeeded
                        && index.inventory_error.is_none()
                        && validation.diagnostics.iter().all(|diagnostic| {
                            !matches!(diagnostic.severity, syu_diagnostics::Severity::Error)
                        });
                    receipts.push(ReadinessReceipt {
                        criterion: criterion.to_string(),
                        plan_slice: plan_slice.clone(),
                        runner: claim.runner.clone(),
                        command_succeeded,
                        validation_passed,
                        poststate_fingerprint: workspace.fingerprint(),
                    });
                    if !validation_passed {
                        criterion_executed = false;
                        closed_loop_blockers.push(format!(
                            "{criterion}: closed-loop receipt for {} did not pass command, validation, and post-state checks",
                            claim.runner
                        ));
                    }
                }
            }
        }
        if criterion_ready {
            verification_ready += 1;
        }
        if level >= ReadinessLevel::ClosedLoop && criterion_ready && criterion_executed {
            closed_loop_ready += 1;
        }
    }
    let verification_required = criteria
        .iter()
        .filter(|criterion| scope_level(workspace, criterion, index) >= ReadinessLevel::Verifiable)
        .count();
    let closed_loop_required = criteria
        .iter()
        .filter(|criterion| scope_level(workspace, criterion, index) >= ReadinessLevel::ClosedLoop)
        .count();
    let verification = axis_for(verification_required, verification_blockers);
    let closed_loop = axis_for(closed_loop_required, closed_loop_blockers);
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
        receipts,
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

fn changed_artifact_paths(
    root: &std::path::Path,
    revision: &str,
) -> Result<std::collections::BTreeSet<String>> {
    let output = Command::new("git")
        .args(["-C"])
        .arg(root)
        .args(["diff", "--name-only", revision])
        .output()?;
    if !output.status.success() {
        return Ok(std::collections::BTreeSet::new());
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .filter(|path| !path.is_empty())
        .map(str::to_owned)
        .collect())
}

fn axis_for(required: usize, blockers: Vec<String>) -> ReadinessAxis {
    let blockers = if required == 0 && blockers.is_empty() {
        vec!["SYU-READINESS-EMPTY-SUBJECT: no subjects require this readiness axis".into()]
    } else {
        blockers
    };
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
    let default = workspace.config.validation.readiness.target;
    let levels = index
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
        .collect::<Vec<_>>();
    levels.into_iter().min().unwrap_or(default)
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
