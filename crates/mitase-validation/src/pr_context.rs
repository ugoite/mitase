use crate::{VerificationAssessment, VerificationAssessmentStatus, assess_verification_claim};
use globset::{Glob, GlobSetBuilder};
use mitase_inventory::{ArtifactUnitKind, SemanticChange, SemanticChangeKind};
use mitase_spec_model::{ExactSelector, LocalAnchorKind, SpecAnchor, SpecDocument, TargetClaim};
use mitase_workspace::{AnchorValue, SpecIndex, SpecWorkspace};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

pub const PR_CONTEXT_REPORT_VERSION: &str = "mitase/pr-context-report/v1";

#[derive(Debug, Clone, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "kebab-case")]
pub enum ChangeKind {
    Added,
    Modified,
    Deleted,
    Renamed,
    Unknown,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChangedArtifact {
    pub path: String,
    pub status: ChangeKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub old_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_identity: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub head_identity: Option<String>,
    pub binding_state: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RevisionContext {
    pub base_sha: String,
    pub head_sha: String,
    pub inventory_profile: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "kebab-case")]
pub enum DisplayReason {
    Direct,
    Upstream,
    Always,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct ReferencePath {
    pub changed_path: String,
    pub relation: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rule_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RelatedSpecification {
    pub id: String,
    pub kind: String,
    pub title: String,
    pub reasons: Vec<DisplayReason>,
    pub evidence: Vec<ReferencePath>,
}

#[derive(Debug, Clone, Serialize)]
pub struct VerificationEvidence {
    pub criterion: String,
    pub claim: TargetClaim,
    pub target: String,
    pub runner: Option<VerificationRunnerMetadata>,
    pub base_assessment: Option<VerificationAssessment>,
    pub head_assessment: Option<VerificationAssessment>,
    pub evidence_lost: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct VerificationRunnerMetadata {
    pub id: String,
    pub claim_arguments: BTreeMap<String, String>,
    pub base: Option<ConfiguredRunner>,
    pub head: Option<ConfiguredRunner>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ConfiguredRunner {
    pub executable: String,
    pub argument_template: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct EvidenceGap {
    pub kind: String,
    pub path: Option<String>,
    pub id: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct PrContextReport {
    pub schema_version: String,
    pub contract_version: String,
    pub revision: RevisionContext,
    pub changed_artifacts: Vec<ChangedArtifact>,
    pub related_specifications: Vec<RelatedSpecification>,
    pub verification_evidence: Vec<VerificationEvidence>,
    pub evidence_gaps: Vec<EvidenceGap>,
}

#[derive(Debug, Clone)]
pub struct ChangedPath {
    pub path: String,
    pub old_path: Option<String>,
    pub status: ChangeKind,
}

pub struct PrContextSide<'a> {
    pub revision: &'a str,
    pub workspace: &'a SpecWorkspace,
    pub index: &'a SpecIndex,
}

struct SpecEntry {
    kind: String,
    title: String,
}

/// Build a deterministic report from two already-loaded repository snapshots.
/// This function only reads indexes and never invokes a verifier.
pub fn build_pr_context_report(
    base: PrContextSide<'_>,
    head: PrContextSide<'_>,
    changes: &[ChangedPath],
    semantic_changes: &[SemanticChange],
) -> PrContextReport {
    let base_sha = base.revision.to_owned();
    let head_sha = head.revision.to_owned();
    let base_index = base.index;
    let head_index = head.index;
    let base = base.workspace;
    let head = head.workspace;
    let mut changed_symbol_identities: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for change in semantic_changes {
        let identity = change
            .after_identity
            .as_deref()
            .or(change.before_identity.as_deref())
            .unwrap_or_default();
        if base_index
            .artifact_units
            .iter()
            .chain(&head_index.artifact_units)
            .any(|unit| unit.identity == identity && unit.kind == ArtifactUnitKind::Symbol)
        {
            let path = change.path.to_string_lossy().into_owned();
            for identity in [&change.before_identity, &change.after_identity]
                .into_iter()
                .flatten()
            {
                changed_symbol_identities
                    .entry(path.clone())
                    .or_default()
                    .insert(identity.clone());
            }
        }
    }
    let mut related: BTreeMap<
        String,
        (SpecEntry, BTreeSet<DisplayReason>, BTreeSet<ReferencePath>),
    > = BTreeMap::new();
    let mut gaps = Vec::new();
    let mut artifacts = Vec::new();
    let mut affected_criteria = BTreeSet::new();
    let mut criterion_paths: BTreeMap<SpecAnchor, BTreeSet<String>> = BTreeMap::new();

    for change in changes {
        let paths: Vec<&str> = change
            .old_path
            .as_deref()
            .into_iter()
            .chain([change.path.as_str()])
            .collect();
        let mut owners = BTreeSet::new();
        let mut resolved_targets = BTreeSet::new();
        for (side, index, side_name) in [(base, base_index, "base"), (head, head_index, "head")] {
            for path in &paths {
                if let Some(targets) = index.path_to_targets.get(*path) {
                    for target in targets {
                        let changed_symbols = changed_symbol_identities.get(*path);
                        let symbol_target = index.target(target).is_some_and(|artifact| {
                            matches!(
                                artifact.selector,
                                ExactSelector::Symbol { .. } | ExactSelector::Test { .. }
                            )
                        });
                        if symbol_target
                            && changed_symbols.is_some_and(|identities| {
                                !index
                                    .target_to_artifact
                                    .get(target)
                                    .is_some_and(|identity| identities.contains(identity))
                            })
                        {
                            continue;
                        }
                        if index.target_to_artifact.contains_key(target) {
                            resolved_targets.insert(target.clone());
                        } else {
                            gaps.push(EvidenceGap {
                                kind: "unresolved-binding-target".into(),
                                path: Some((*path).into()),
                                id: Some(target.to_string()),
                                message: format!("The declared target {target} is not a current uniquely resolved artifact in the {side_name} snapshot."),
                            });
                        }
                        owners.insert(target.binding.clone());
                        add_item(
                            &mut related,
                            side,
                            &target.binding.item.0,
                            DisplayReason::Direct,
                            ReferencePath {
                                changed_path: (*path).into(),
                                relation: vec![
                                    format!("{side_name}:{}", target),
                                    "binding-owner".into(),
                                ],
                                rule_id: None,
                            },
                        );
                        if let Some(binding) = index.bindings.get(&target.binding) {
                            for artifact in &binding.targets {
                                if artifact.path.to_string_lossy() == *path {
                                    for claim in &artifact.claims {
                                        if let mitase_spec_model::TargetClaim::Satisfies {
                                            criterion,
                                        } = claim
                                        {
                                            affected_criteria.insert(criterion.clone());
                                            criterion_paths
                                                .entry(criterion.clone())
                                                .or_default()
                                                .insert((*path).into());
                                        }
                                    }
                                }
                            }
                        }
                        for contract_anchor in index
                            .binding_to_contracts
                            .get(&target.binding)
                            .into_iter()
                            .flatten()
                        {
                            if let Some(contract) = index.contracts.get(contract_anchor) {
                                for criterion in &contract.guarantees {
                                    affected_criteria.insert(criterion.clone());
                                    criterion_paths
                                        .entry(criterion.clone())
                                        .or_default()
                                        .insert((*path).into());
                                }
                            }
                        }
                    }
                }
            }
        }
        if resolved_targets.len() > 1 {
            gaps.push(EvidenceGap {
                kind: "ambiguous-binding".into(),
                path: Some(change.path.clone()),
                id: None,
                message: format!("{} current exact Binding targets match this changed artifact; ownership is ambiguous.", resolved_targets.len()),
            });
        }
        let specification_change = [base, head].iter().any(|workspace| {
            workspace.config.workspace.spec_roots.iter().any(|root| {
                let root = root.as_path().to_string_lossy();
                change.path.starts_with(&format!("{root}/")) || change.path == root
            })
        });
        if owners.is_empty() && !specification_change {
            gaps.push(EvidenceGap {
                kind: "unbound-artifact".into(),
                path: Some(change.path.clone()),
                id: None,
                message: "No exact Binding target resolves this changed path in either snapshot."
                    .into(),
            });
        }
        let base_declared = paths
            .iter()
            .any(|path| base_index.path_to_targets.contains_key(*path));
        let head_declared = paths
            .iter()
            .any(|path| head_index.path_to_targets.contains_key(*path));
        let base_bound = paths.iter().any(|path| {
            base_index
                .path_to_targets
                .get(*path)
                .is_some_and(|targets| {
                    targets
                        .iter()
                        .any(|target| base_index.target_to_artifact.contains_key(target))
                })
        });
        let head_bound = paths.iter().any(|path| {
            head_index
                .path_to_targets
                .get(*path)
                .is_some_and(|targets| {
                    targets
                        .iter()
                        .any(|target| head_index.target_to_artifact.contains_key(target))
                })
        });
        let state = match (base_bound, head_bound, base_declared, head_declared) {
            (true, true, _, _) => "bound-in-base-and-head",
            (false, true, _, _) => "bound-in-head",
            (true, false, _, _) => "bound-in-base",
            (false, false, true, true) => "declared-but-unresolved-in-base-and-head",
            (false, false, true, false) => "declared-but-unresolved-in-base",
            (false, false, false, true) => "declared-but-unresolved-in-head",
            (false, false, false, false) => "unbound",
        }
        .to_owned();
        artifacts.push(ChangedArtifact {
            path: change.path.clone(),
            status: change.status.clone(),
            old_path: change.old_path.clone(),
            symbol: None,
            base_identity: None,
            head_identity: None,
            binding_state: if owners.is_empty() && specification_change {
                "specification-change".into()
            } else {
                state
            },
        });
    }

    for change in semantic_changes {
        let identity = change
            .after_identity
            .as_deref()
            .or(change.before_identity.as_deref())
            .unwrap_or_default();
        let is_symbol = base_index
            .artifact_units
            .iter()
            .chain(&head_index.artifact_units)
            .any(|unit| unit.identity == identity && unit.kind == ArtifactUnitKind::Symbol);
        if !is_symbol {
            continue;
        }
        let symbol = identity.rsplit("::").next().unwrap_or(identity).to_owned();
        let status = match change.kind {
            SemanticChangeKind::Rename => ChangeKind::Renamed,
            SemanticChangeKind::Deletion | SemanticChangeKind::PublicRemoval => ChangeKind::Deleted,
            SemanticChangeKind::Addition | SemanticChangeKind::PublicAddition => ChangeKind::Added,
            _ => ChangeKind::Modified,
        };
        let base_bound = change.before_identity.as_ref().is_some_and(|identity| {
            base_index
                .target_to_artifact
                .values()
                .any(|value| value == identity)
        });
        let head_bound = change.after_identity.as_ref().is_some_and(|identity| {
            head_index
                .target_to_artifact
                .values()
                .any(|value| value == identity)
        });
        let binding_state = match (base_bound, head_bound) {
            (true, true) => "symbol-bound-in-base-and-head",
            (true, false) => "symbol-bound-in-base",
            (false, true) => "symbol-bound-in-head",
            (false, false) => "unbound-symbol",
        };
        let item = ChangedArtifact {
            path: change.path.to_string_lossy().into_owned(),
            status,
            old_path: change
                .before_identity
                .as_deref()
                .and_then(identity_path)
                .map(str::to_owned),
            symbol: Some(symbol),
            base_identity: change.before_identity.clone(),
            head_identity: change.after_identity.clone(),
            binding_state: binding_state.into(),
        };
        if !artifacts.iter().any(|present: &ChangedArtifact| {
            present.path == item.path && present.symbol == item.symbol
        }) {
            artifacts.push(item);
        }
    }

    // Specification-only changes can remove a verification declaration
    // without touching any source artifact. Keep those Criteria in scope.
    for (workspace, index) in [(base, base_index), (head, head_index)] {
        for (item, document_path) in &index.item_paths {
            let document_path = document_path.to_string_lossy().replace('\\', "/");
            let Some(path) = changes.iter().find_map(|change| {
                let matches_document = |candidate: &str| {
                    document_path == candidate || document_path.ends_with(&format!("/{candidate}"))
                };
                if matches_document(&change.path) {
                    Some(change.path.clone())
                } else if change.old_path.as_deref().is_some_and(matches_document) {
                    change.old_path.clone()
                } else {
                    None
                }
            }) else {
                continue;
            };
            add_item(
                &mut related,
                workspace,
                &item.0,
                DisplayReason::Direct,
                ReferencePath {
                    changed_path: path.clone(),
                    relation: vec![format!("specification-document:{item}")],
                    rule_id: None,
                },
            );
            for anchor in index.item_anchors.get(item).into_iter().flatten() {
                if anchor.kind == LocalAnchorKind::Criterion {
                    affected_criteria.insert(anchor.clone());
                    criterion_paths
                        .entry(anchor.clone())
                        .or_default()
                        .insert(path.clone());
                }
            }
            for (criterion, targets) in &index.all_criteria_to_verification_targets {
                if targets.iter().any(|target| &target.binding.item == item) {
                    affected_criteria.insert(criterion.clone());
                    criterion_paths
                        .entry(criterion.clone())
                        .or_default()
                        .insert(path.clone());
                }
            }
        }
    }

    // Follow the canonical reverse governance chain from directly changed
    // implementation bindings through Criterion, Policy, and Philosophy.
    for index in [base_index, head_index] {
        for (criterion, implementations) in &index.all_criteria_to_implementation_targets {
            if implementations.iter().any(|target| {
                changes.iter().any(|change| {
                    let paths = [Some(change.path.as_str()), change.old_path.as_deref()];
                    paths.into_iter().flatten().any(|path| {
                        index
                            .target(target)
                            .is_some_and(|t| t.path.to_string_lossy() == path)
                    })
                })
            }) {
                affected_criteria.insert(criterion.clone());
            }
        }
    }
    for criterion in &affected_criteria {
        for (side, index, side_name) in [(base, base_index, "base"), (head, head_index, "head")] {
            let paths = criterion_paths.get(criterion).cloned().unwrap_or_default();
            let paths = if paths.is_empty() {
                BTreeSet::from([String::new()])
            } else {
                paths
            };
            for path in paths {
                add_item(
                    &mut related,
                    side,
                    &criterion.item.0,
                    DisplayReason::Upstream,
                    ReferencePath {
                        changed_path: path.clone(),
                        relation: vec!["artifact".into(), "binding".into(), criterion.to_string()],
                        rule_id: None,
                    },
                );
                if let Some(AnchorValue::Criterion(value)) = index.anchor(criterion) {
                    for rule in &value.governed_by {
                        add_item(
                            &mut related,
                            side,
                            &rule.item.0,
                            DisplayReason::Upstream,
                            ReferencePath {
                                changed_path: path.clone(),
                                relation: vec![criterion.to_string(), rule.to_string()],
                                rule_id: None,
                            },
                        );
                        for principle in index.rules_to_principles.get(rule).into_iter().flatten() {
                            add_item(
                                &mut related,
                                side,
                                &principle.item.0,
                                DisplayReason::Upstream,
                                ReferencePath {
                                    changed_path: path.clone(),
                                    relation: vec![
                                        criterion.to_string(),
                                        rule.to_string(),
                                        principle.to_string(),
                                    ],
                                    rule_id: None,
                                },
                            );
                        }
                    }
                }
            }
            let _ = side_name;
        }
    }

    for (side, index, label) in [(base, base_index, "base"), (head, head_index, "head")] {
        add_always_review(&mut related, &mut gaps, side, index, changes, label);
    }

    let mut verifications = BTreeMap::new();
    for criterion in affected_criteria {
        let base_claims = claim_assessments(&base.config, base_index, &criterion);
        let head_claims = claim_assessments(&head.config, head_index, &criterion);
        let claims: BTreeSet<_> = base_claims
            .keys()
            .chain(head_claims.keys())
            .cloned()
            .collect();
        for claim in claims {
            let before = base_claims.get(&claim).cloned();
            let after = head_claims.get(&claim).cloned();
            let declaration = claim_declaration(base_index, &criterion, &claim)
                .or_else(|| claim_declaration(head_index, &criterion, &claim))
                .expect("indexed verification target retains its declaration");
            let runner = runner_metadata(base, head, &declaration);
            verifications.insert(
                (criterion.to_string(), claim.clone()),
                VerificationEvidence {
                    criterion: criterion.to_string(),
                    claim: declaration,
                    target: claim,
                    runner,
                    evidence_lost: before
                        .as_ref()
                        .is_some_and(|a| a.status == VerificationAssessmentStatus::Valid)
                        && !after
                            .as_ref()
                            .is_some_and(|a| a.status == VerificationAssessmentStatus::Valid),
                    base_assessment: before,
                    head_assessment: after,
                },
            );
        }
        if base_claims.is_empty() && head_claims.is_empty() {
            gaps.push(EvidenceGap {
                kind: "missing-verification-claim".into(),
                path: None,
                id: Some(criterion.to_string()),
                message: "No verification claim is declared for this affected Criterion.".into(),
            });
        }
    }

    artifacts.sort_by(|a, b| (&a.path, &a.symbol).cmp(&(&b.path, &b.symbol)));
    gaps.sort_by(|a, b| {
        (&a.kind, &a.path, &a.id, &a.message).cmp(&(&b.kind, &b.path, &b.id, &b.message))
    });
    gaps.dedup_by(|a, b| {
        a.kind == b.kind && a.path == b.path && a.id == b.id && a.message == b.message
    });
    let related_specifications = related
        .into_iter()
        .map(|(id, (entry, reasons, evidence))| RelatedSpecification {
            id,
            kind: entry.kind,
            title: entry.title,
            reasons: reasons.into_iter().collect(),
            evidence: evidence.into_iter().collect(),
        })
        .collect();
    PrContextReport {
        schema_version: "mitase/cli/v1".into(),
        contract_version: PR_CONTEXT_REPORT_VERSION.into(),
        revision: RevisionContext {
            base_sha,
            head_sha,
            inventory_profile: head.config.inventory.active_profile.clone(),
        },
        changed_artifacts: artifacts,
        related_specifications,
        verification_evidence: verifications.into_values().collect(),
        evidence_gaps: gaps,
    }
}

fn identity_path(identity: &str) -> Option<&str> {
    let (_, rest) = identity.split_once(':')?;
    Some(rest.split("::").next().unwrap_or(rest))
}

fn claim_declaration(
    index: &SpecIndex,
    criterion: &SpecAnchor,
    target_reference: &str,
) -> Option<TargetClaim> {
    let target = index
        .all_criteria_to_verification_targets
        .get(criterion)?
        .iter()
        .find(|target| target.to_string() == target_reference)?;
    index
        .target(target)?
        .claims
        .iter()
        .find_map(|claim| match claim {
            TargetClaim::Verifies {
                criterion: actual, ..
            } if actual == criterion => Some(claim.clone()),
            _ => None,
        })
}

fn runner_metadata(
    base: &SpecWorkspace,
    head: &SpecWorkspace,
    claim: &TargetClaim,
) -> Option<VerificationRunnerMetadata> {
    let TargetClaim::Verifies { runner, .. } = claim else {
        return None;
    };
    let configured = |workspace: &SpecWorkspace| {
        workspace
            .config
            .verification
            .runners
            .get(&runner.runner)
            .map(|runner| ConfiguredRunner {
                executable: runner.executable.clone(),
                argument_template: runner.arguments.clone(),
            })
    };
    Some(VerificationRunnerMetadata {
        id: runner.runner.clone(),
        claim_arguments: runner.arguments.clone(),
        base: configured(base),
        head: configured(head),
    })
}

fn claim_assessments(
    config: &mitase_project_model::ProjectConfig,
    index: &SpecIndex,
    criterion: &SpecAnchor,
) -> BTreeMap<String, VerificationAssessment> {
    let mut out = BTreeMap::new();
    for target in index
        .all_criteria_to_verification_targets
        .get(criterion)
        .into_iter()
        .flatten()
    {
        let assessment = assess_verification_claim(config, index, target, criterion);
        out.insert(target.to_string(), assessment);
    }
    out
}

fn add_always_review(
    related: &mut BTreeMap<String, (SpecEntry, BTreeSet<DisplayReason>, BTreeSet<ReferencePath>)>,
    gaps: &mut Vec<EvidenceGap>,
    workspace: &SpecWorkspace,
    index: &SpecIndex,
    changes: &[ChangedPath],
    _side: &str,
) {
    for rule in &workspace.config.review.always {
        let mut builder = GlobSetBuilder::new();
        let mut invalid = false;
        for pattern in &rule.paths {
            match Glob::new(&pattern.0) {
                Ok(glob) => {
                    builder.add(glob);
                }
                Err(error) => {
                    invalid = true;
                    gaps.push(EvidenceGap {
                        kind: "invalid-review-pattern".into(),
                        path: None,
                        id: Some(rule.id.clone()),
                        message: error.to_string(),
                    });
                }
            }
        }
        if invalid {
            continue;
        }
        let Ok(globs) = builder.build() else {
            continue;
        };
        for change in changes {
            for path in [Some(change.path.as_str()), change.old_path.as_deref()]
                .into_iter()
                .flatten()
            {
                if !globs.is_match(path) {
                    continue;
                }
                for item in &rule.items {
                    if !index
                        .item_anchors
                        .contains_key(&mitase_spec_model::SpecId(item.clone()))
                    {
                        gaps.push(EvidenceGap {
                            kind: "unknown-review-item".into(),
                            path: Some(path.into()),
                            id: Some(item.clone()),
                            message: format!(
                                "review rule {} refers to an unknown specification item",
                                rule.id
                            ),
                        });
                        continue;
                    }
                    add_item(
                        related,
                        workspace,
                        item,
                        DisplayReason::Always,
                        ReferencePath {
                            changed_path: path.into(),
                            relation: vec![format!("review:{}", rule.id)],
                            rule_id: Some(rule.id.clone()),
                        },
                    );
                }
            }
        }
    }
}

fn add_item(
    related: &mut BTreeMap<String, (SpecEntry, BTreeSet<DisplayReason>, BTreeSet<ReferencePath>)>,
    workspace: &SpecWorkspace,
    item: &str,
    reason: DisplayReason,
    path: ReferencePath,
) {
    let Some(entry) = item_entry(workspace, item) else {
        return;
    };
    let data = related
        .entry(item.into())
        .or_insert_with(|| (entry, BTreeSet::new(), BTreeSet::new()));
    data.1.insert(reason);
    data.2.insert(path);
}

fn item_entry(workspace: &SpecWorkspace, id: &str) -> Option<SpecEntry> {
    for loaded in &workspace.documents {
        match &loaded.document {
            SpecDocument::Philosophies { philosophies, .. } => {
                if let Some(x) = philosophies.iter().find(|x| x.id.0 == id) {
                    return Some(SpecEntry {
                        kind: "philosophy".into(),
                        title: x.title.clone(),
                    });
                }
            }
            SpecDocument::Policies { policies, .. } => {
                if let Some(x) = policies.iter().find(|x| x.id.0 == id) {
                    return Some(SpecEntry {
                        kind: "policy".into(),
                        title: x.title.clone(),
                    });
                }
            }
            SpecDocument::Requirements { requirements, .. } => {
                if let Some(x) = requirements.iter().find(|x| x.id.0 == id) {
                    return Some(SpecEntry {
                        kind: "requirement".into(),
                        title: x.title.clone(),
                    });
                }
            }
            SpecDocument::Features { features, .. } => {
                if let Some(x) = features.iter().find(|x| x.id.0 == id) {
                    return Some(SpecEntry {
                        kind: "feature".into(),
                        title: x.title.clone(),
                    });
                }
            }
        }
    }
    None
}
