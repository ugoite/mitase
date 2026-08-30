use anyhow::{Result, bail};
use mitase_project_model::{ProjectConfig, ReadinessLevel};
use mitase_spec_model::{BoundTargetRef, ItemStatus, OwnershipSelector, SpecAnchor};
use mitase_workspace::{SpecIndex, SpecWorkspace};
use serde::Serialize;
use std::collections::BTreeSet;
use std::process::Command;

/// Readiness counts typed subjects rather than subtracting blocker strings.
/// This keeps one subject with several blockers counted exactly once.
#[derive(Debug, Clone, Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct ReadinessSubject {
    pub id: String,
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
    pub verification: ReadinessAxis,
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
            ReadinessAxisId::Verification => self.verification.is_ready(),
        })
    }

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
                .is_none_or(|probe| self.meets_scope(public_entrypoints_scope(), probe.level))
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
            ReadinessAxisId::Inventory => self.inventory.is_ready(),
            ReadinessAxisId::Ownership => self.ownership.is_ready(),
            ReadinessAxisId::Seedability => {
                scoped_axis_is_ready(&self.seedability, scope_id, level)
            }
            ReadinessAxisId::Verification => {
                scoped_axis_is_ready(&self.verification, scope_id, level)
            }
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
        ReadinessLevel::Verifiable => &[Inventory, Ownership, Seedability, Verification],
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadinessAxisId {
    Inventory,
    Ownership,
    Seedability,
    Verification,
}

/// Inspect specification and repository evidence without executing a runner.
/// External tools may execute the selected Verification Claims separately.
pub fn evaluate(
    workspace: &SpecWorkspace,
    index: &SpecIndex,
    revision: &str,
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

    let inventory = if let Some(error) = &index.inventory_error {
        axis_from_subjects(vec![ReadinessSubject {
            id: "inventory:error".into(),
            scope_id: repository_scope(),
            required_level: ReadinessLevel::Traceable,
            ready: false,
            blockers: vec![error.clone()],
        }])
    } else {
        axis_from_subjects(
            active
                .iter()
                .map(|unit| ReadinessSubject {
                    id: format!("inventory:{}", unit.identity),
                    scope_id: repository_scope(),
                    required_level: ReadinessLevel::Traceable,
                    ready: true,
                    blockers: vec![],
                })
                .collect(),
        )
    };

    let criteria = implemented_criteria(workspace, index)?;
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

    let mut ownership_subjects = active
        .iter()
        .filter_map(|unit| {
            let owners = index
                .artifact_owners
                .get(&unit.identity)
                .cloned()
                .unwrap_or_default();
            let required_for_maturity = ownership_required.contains(&unit.identity);
            let blockers = if owners.len() > 1 || (required_for_maturity && owners.len() != 1) {
                vec![format!("{} has {} owners", unit.identity, owners.len())]
            } else {
                vec![]
            };
            (required_for_maturity || !blockers.is_empty()).then(|| ReadinessSubject {
                id: format!("ownership:{}", unit.identity),
                scope_id: repository_scope(),
                required_level: ReadinessLevel::Traceable,
                ready: blockers.is_empty(),
                blockers,
            })
        })
        .collect::<Vec<_>>();
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
            let limit = workspace
                .config
                .validation
                .readiness
                .limits
                .max_ownership_scope_units;
            if matched > limit {
                ownership_subjects.push(ReadinessSubject {
                    id: format!("ownership-scope:{binding_anchor}/{}", scope.id),
                    scope_id: repository_scope(),
                    required_level: ReadinessLevel::Traceable,
                    ready: false,
                    blockers: vec![format!(
                        "scope covers {matched} active units; max_ownership_scope_units is {limit}"
                    )],
                });
            }
        }
    }
    let ownership = axis_from_subjects(ownership_subjects);

    let mut seed_subjects = Vec::new();
    let mut verification_subjects = Vec::new();
    for criterion in &criteria {
        let scope_id = criterion_scope(criterion);
        let implementations = implementation_obligations(index, criterion);
        let verifications = index
            .criteria_to_verification_targets
            .get(criterion)
            .cloned()
            .unwrap_or_default();
        let required_level = scope_level(workspace, criterion);

        if implementations.is_empty() {
            seed_subjects.push(subject(
                format!("criterion:{criterion}/implementation"),
                scope_id.clone(),
                required_level,
                false,
                "criterion has no exact implementation target",
            ));
        }

        for implementation in &implementations {
            let target_ready = target_is_present_or_declared_absent(index, implementation);
            let mut target_blockers = Vec::new();
            if !target_ready {
                target_blockers
                    .push("implementation target does not resolve to one exact artifact".into());
            }
            seed_subjects.push(ReadinessSubject {
                id: format!("criterion:{criterion}/target:{implementation}"),
                scope_id: scope_id.clone(),
                required_level,
                ready: target_blockers.is_empty(),
                blockers: target_blockers,
            });

            let verification = verification_subject(
                workspace,
                index,
                criterion,
                std::slice::from_ref(implementation),
                &verifications,
                required_level,
            );
            if required_level >= ReadinessLevel::Verifiable {
                verification_subjects.push(ReadinessSubject {
                    id: format!("criterion:{criterion}/target:{implementation}/verification"),
                    ..verification
                });
            }
        }

        if implementations.is_empty() {
            let verification = verification_subject(
                workspace,
                index,
                criterion,
                &implementations,
                &verifications,
                required_level,
            );
            if required_level >= ReadinessLevel::Verifiable {
                verification_subjects.push(verification);
            }
        }
    }

    seed_subjects.extend(implemented_feature_subjects(workspace, index));

    if let Some(level) = workspace
        .config
        .validation
        .readiness
        .probes
        .public_entrypoints
        .as_ref()
        .map(|probe| probe.level)
    {
        let subjects = public_entrypoint_subjects(workspace, index, level);
        if level >= ReadinessLevel::Seedable {
            seed_subjects.extend(subjects.clone());
        }
        if level >= ReadinessLevel::Verifiable {
            verification_subjects.extend(subjects);
        }
    }

    if let Some(level) = workspace
        .config
        .validation
        .readiness
        .probes
        .contracts
        .as_ref()
        .map(|probe| probe.level)
    {
        let subjects = contract_subjects(index, level);
        seed_subjects.extend(subjects.clone());
        if level >= ReadinessLevel::Verifiable {
            verification_subjects.extend(subjects);
        }
    }

    if workspace.config.validation.readiness.probes.changed_units {
        for path in changed_artifact_paths(workspace, revision)? {
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
                        binding.owns.iter().filter_map(move |scope| {
                            (scope.adapter == "declared"
                                && scope.path.to_string_lossy() == path_for_owner
                                && matches!(scope.selector, OwnershipSelector::File))
                            .then(|| format!("{binding_anchor}#scope.{}", scope.id))
                        })
                    })
                    .collect::<Vec<_>>();
                let ready = owners.len() == 1;
                seed_subjects.push(subject(
                    format!("changed:{path}"),
                    changed_units_scope(),
                    ReadinessLevel::Seedable,
                    ready,
                    if ready {
                        ""
                    } else {
                        "changed file is absent from active inventory and does not have one exact owner"
                    },
                ));
            } else {
                for unit in units {
                    let owners = index
                        .artifact_owners
                        .get(&unit.identity)
                        .cloned()
                        .unwrap_or_default();
                    let ready = owners.len() == 1;
                    seed_subjects.push(subject(
                        format!("changed:{}", unit.identity),
                        changed_units_scope(),
                        ReadinessLevel::Seedable,
                        ready,
                        if ready {
                            ""
                        } else {
                            "changed artifact requires exactly one owner"
                        },
                    ));
                }
            }
        }
    }

    Ok(ReadinessReport {
        target: workspace.config.validation.readiness.target,
        inventory,
        ownership,
        seedability: axis_from_subjects(seed_subjects),
        verification: axis_from_subjects(verification_subjects),
    })
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

fn axis_from_subjects(mut subjects: Vec<ReadinessSubject>) -> ReadinessAxis {
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
            bail!(
                "readiness criterion {} is configured more than once",
                probe.criterion
            );
        }
        if index.criterion_status.get(&probe.criterion) != Some(&ItemStatus::Implemented) {
            bail!(
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

fn target_is_present_or_declared_absent(index: &SpecIndex, target: &BoundTargetRef) -> bool {
    if index.target_to_artifact.contains_key(target) {
        return true;
    }
    index
        .bindings
        .get(&target.binding)
        .and_then(|binding| {
            binding
                .targets
                .iter()
                .find(|candidate| candidate.id == target.target_id)
        })
        .is_some_and(|target| {
            target.lifecycle == mitase_spec_model::ArtifactTargetLifecycle::Absent
        })
}

fn implemented_feature_subjects(
    workspace: &SpecWorkspace,
    index: &SpecIndex,
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
                    if !target_is_present_or_declared_absent(index, &reference) {
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
                &crate::VerificationClaimRef {
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
        let claim = crate::VerificationClaimRef {
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
        if !workspace
            .config
            .verification
            .runners
            .contains_key(&runner_ref.runner)
        {
            blockers.push(format!(
                "verification runner {} is not configured",
                runner_ref.runner
            ));
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
    _workspace: &SpecWorkspace,
    index: &SpecIndex,
    required_level: ReadinessLevel,
) -> Vec<ReadinessSubject> {
    index
        .exposes_by_target
        .iter()
        .map(|(target_ref, exposed_target)| {
            let mut blockers = Vec::new();
            let Some(identity) = index.target_to_artifact.get(target_ref) else {
                return subject(
                    format!("public:{target_ref}"),
                    public_entrypoints_scope(),
                    required_level,
                    false,
                    "explicit public entrypoint target does not resolve to one active semantic artifact",
                );
            };
            let exact_owner = index
                .artifact_owners
                .get(identity)
                .is_some_and(|owners| {
                    owners.iter().any(|owner| {
                        owner.binding == target_ref.binding
                            && owner.target_id.as_ref() == Some(&target_ref.target_id)
                    })
                });
            if !exact_owner {
                blockers.push(
                    "public artifact requires its exact ArtifactTarget owner".into(),
                );
            }
            let connected = index
                .criteria_to_implementation_targets
                .values()
                .any(|targets| targets.contains(exposed_target));
            if !connected {
                blockers.push("exposed capability target is not connected to a current criterion".into());
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

fn contract_subjects(index: &SpecIndex, required_level: ReadinessLevel) -> Vec<ReadinessSubject> {
    if index.contracts.is_empty() {
        return vec![subject(
            "contracts:active".into(),
            contracts_scope(),
            required_level,
            false,
            "contracts: all was requested but no active contract is declared",
        )];
    }
    index
        .contracts
        .iter()
        .map(|(anchor, contract)| {
            let expected = std::iter::once(&contract.source)
                .chain(
                    contract
                        .participants
                        .iter()
                        .map(|participant| &participant.target),
                )
                .collect::<BTreeSet<_>>();
            let blockers = expected
                .iter()
                .filter(|target| !index.target_to_artifact.contains_key(**target))
                .map(|target| format!("{target} is not an exact participant target"))
                .collect::<Vec<_>>();
            ReadinessSubject {
                id: format!("contract:{anchor}"),
                scope_id: contracts_scope().into(),
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

fn changed_artifact_paths(workspace: &SpecWorkspace, revision: &str) -> Result<BTreeSet<String>> {
    let baseline = readiness_change_baseline(workspace, revision)?;
    let mut paths = BTreeSet::new();
    collect_git_paths(
        &workspace.root,
        &["diff", "--name-only", &baseline],
        &mut paths,
    )?;
    collect_git_paths(
        &workspace.root,
        &["diff", "--name-only", "--cached"],
        &mut paths,
    )?;
    collect_git_paths(&workspace.root, &["diff", "--name-only"], &mut paths)?;
    collect_git_paths(
        &workspace.root,
        &["ls-files", "--others", "--exclude-standard"],
        &mut paths,
    )?;
    Ok(paths)
}

fn readiness_change_baseline(workspace: &SpecWorkspace, revision: &str) -> Result<String> {
    match workspace.config.validation.changed.baseline.as_ref() {
        Some(mitase_project_model::ChangeBaseline::MergeBase { against }) => git_text(
            &workspace.root,
            &["merge-base", revision, against.0.as_str()],
        )
        .or_else(|_| git_text(&workspace.root, &["rev-parse", "HEAD^"]))
        .or_else(|_| Ok(revision.to_owned())),
        Some(mitase_project_model::ChangeBaseline::Revision { revision }) => Ok(revision.0.clone()),
        Some(mitase_project_model::ChangeBaseline::Parent) => {
            git_text(&workspace.root, &["rev-parse", &format!("{revision}^")])
        }
        None => git_text(&workspace.root, &["merge-base", revision, "origin/main"])
            .or_else(|_| git_text(&workspace.root, &["rev-parse", &format!("{revision}^")]))
            .or_else(|_| Ok(revision.to_owned())),
    }
}

fn git_text(root: &std::path::Path, args: &[&str]) -> Result<String> {
    let output = Command::new("git")
        .args(["-C", root.to_string_lossy().as_ref()])
        .args(args)
        .output()?;
    if !output.status.success() {
        bail!("git command failed: git {}", args.join(" "));
    }
    let value = String::from_utf8(output.stdout)?.trim().to_owned();
    if value.is_empty() {
        bail!("git command returned no value: git {}", args.join(" "));
    }
    Ok(value)
}

fn collect_git_paths(
    root: &std::path::Path,
    args: &[&str],
    paths: &mut BTreeSet<String>,
) -> Result<()> {
    let output = Command::new("git")
        .args(["-C", root.to_string_lossy().as_ref()])
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

fn scope_level(workspace: &SpecWorkspace, criterion: &SpecAnchor) -> ReadinessLevel {
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
        assert_eq!(required_axes(ReadinessLevel::Verifiable).len(), 4);
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

    #[test]
    fn current_public_entrypoints_have_exact_governance() {
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(std::path::Path::parent)
            .expect("workspace root")
            .to_path_buf();
        let workspace = SpecWorkspace::load(root).expect("workspace");
        let index = workspace.index().expect("index");
        let blockers = public_entrypoint_subjects(&workspace, &index, ReadinessLevel::Seedable)
            .into_iter()
            .filter(|subject| !subject.ready || !subject.blockers.is_empty())
            .flat_map(|subject| {
                subject
                    .blockers
                    .into_iter()
                    .map(move |blocker| format!("{}: {blocker}", subject.id))
            })
            .collect::<Vec<_>>();
        assert!(
            blockers.is_empty(),
            "public entrypoint blockers: {blockers:?}"
        );
    }

    #[test]
    fn readiness_inspection_does_not_execute_configured_runners() {
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(std::path::Path::parent)
            .expect("workspace root")
            .to_path_buf();
        let mut workspace = SpecWorkspace::load(root).expect("workspace");
        for runner in workspace.config.verification.runners.values_mut() {
            runner.executable = "/definitely-not-an-executable".into();
        }
        let index = workspace.index().expect("index");
        evaluate(&workspace, &index, "readiness-test").expect("readiness");
    }
}
