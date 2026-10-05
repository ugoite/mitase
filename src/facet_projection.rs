//! Read-only facet projection over the canonical specification graph.
//!
//! The projection groups current `role: implementation` exact targets with
//! direct `TargetClaim::Satisfies` claims by their opaque binding facet. It
//! never infers required facets, never promotes `Exposes` relations into
//! coverage, and never executes a verification runner. Verification output
//! is declared structural verification only.

use anyhow::{Result, bail};
use mitase_spec_model::{
    ArtifactTargetLifecycle, BindingRole, BoundTargetRef, ItemStatus, RepoPath, Selector,
    SpecAnchor, SpecDocument, SpecId, TargetClaim,
};
use mitase_validation::{VerificationAssessmentStatus, assess_verification_claim};
use mitase_workspace::{AnchorValue, SpecIndex, SpecWorkspace};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

pub const FACET_PROJECTION_CONTRACT_VERSION: &str = "mitase/facet-projection-report/v1";
const CLI_SCHEMA_VERSION: &str = "mitase/cli/v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum FacetSourceKind {
    Feature,
    Criterion,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum DeclaredStatus {
    Verified,
    Partial,
    Unverified,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FacetBindingEntry {
    pub id: SpecAnchor,
    pub role: BindingRole,
    pub facet: String,
    pub responsibility: String,
    pub targets: Vec<FacetTargetEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FacetTargetEntry {
    pub id: BoundTargetRef,
    pub adapter: String,
    pub path: RepoPath,
    pub selector: Selector,
    pub lifecycle: ArtifactTargetLifecycle,
    pub current: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DeclaredVerification {
    pub status: DeclaredStatus,
    pub verification_targets: Vec<BoundTargetRef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FacetRow {
    pub facet: String,
    pub feature: SpecId,
    pub binding: SpecAnchor,
    pub implementation_targets: Vec<BoundTargetRef>,
    pub declared_verification: DeclaredVerification,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FacetCriterionEntry {
    pub criterion: SpecAnchor,
    pub facets: Vec<FacetRow>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NonSemanticTarget {
    pub id: BoundTargetRef,
    pub role: BindingRole,
    pub facet: String,
    pub adapter: String,
    pub path: RepoPath,
    pub selector: Selector,
    pub exposes: Vec<BoundTargetRef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FacetProjectionReport {
    pub schema_version: String,
    pub contract_version: String,
    pub source: String,
    pub source_kind: FacetSourceKind,
    pub feature_status: ItemStatus,
    pub bindings: Vec<FacetBindingEntry>,
    pub criteria: Vec<FacetCriterionEntry>,
    pub non_semantic_targets: Vec<NonSemanticTarget>,
}

struct SemanticTarget {
    feature: SpecId,
    binding: SpecAnchor,
    facet: String,
    target: BoundTargetRef,
}

type FacetRowKey = (String, SpecId, SpecAnchor);
type FacetRowTargets = BTreeMap<FacetRowKey, BTreeSet<BoundTargetRef>>;
type CriterionGrouping = BTreeMap<SpecAnchor, FacetRowTargets>;

/// Build a deterministic facet projection for one feature or criterion.
///
/// Accepted sources are a feature ID (`FEAT-*`) or a requirement criterion
/// anchor (`REQ-*#criterion.*`). Anything else is a top-level error.
pub fn build_facet_projection(
    workspace: &SpecWorkspace,
    index: &SpecIndex,
    source: &str,
) -> Result<FacetProjectionReport> {
    if source.contains('#') {
        let anchor: SpecAnchor = source
            .parse()
            .map_err(|_| anyhow::anyhow!("facet projection source {source} was not found"))?;
        if !matches!(index.anchor(&anchor), Some(AnchorValue::Criterion(_))) {
            bail!("facet projection source {source} is not a feature or criterion");
        }
        build_criterion_report(workspace, index, source, &anchor)
    } else {
        let id = SpecId::from(source);
        if feature_record(workspace, &id).is_none() {
            bail!("facet projection source {source} was not found");
        }
        build_feature_report(workspace, index, source, &id)
    }
}

fn feature_record(workspace: &SpecWorkspace, id: &SpecId) -> Option<ItemStatus> {
    for loaded in &workspace.documents {
        let SpecDocument::Features { features, .. } = &loaded.document else {
            continue;
        };
        for feature in features {
            if feature.id == *id {
                return Some(feature.status);
            }
        }
    }
    None
}

/// Aggregate declared verification without executing any runner.
///
/// Every implementation target of the facet row is checked for at least one
/// structurally valid exact verification claim. The result describes
/// declarations only; it never means a test ran or passed.
fn aggregate_declared_verification(covered: usize, total: usize) -> DeclaredStatus {
    if total == 0 || covered == 0 {
        DeclaredStatus::Unverified
    } else if covered == total {
        DeclaredStatus::Verified
    } else {
        DeclaredStatus::Partial
    }
}

fn valid_verifications_for(
    workspace: &SpecWorkspace,
    index: &SpecIndex,
    criterion: &SpecAnchor,
    implementation: &BoundTargetRef,
) -> Vec<BoundTargetRef> {
    let mut verifications = index
        .verification_by_target
        .get(implementation)
        .into_iter()
        .flatten()
        .filter(|verification| {
            assess_verification_claim(&workspace.config, index, verification, criterion).status
                == VerificationAssessmentStatus::Valid
        })
        .cloned()
        .collect::<Vec<_>>();
    verifications.sort();
    verifications.dedup();
    verifications
}

fn direct_satisfies(target: &mitase_spec_model::ArtifactTarget) -> Vec<SpecAnchor> {
    let mut criteria = target
        .claims
        .iter()
        .filter_map(|claim| match claim {
            TargetClaim::Satisfies { criterion } => Some(criterion.clone()),
            _ => None,
        })
        .collect::<Vec<_>>();
    criteria.sort();
    criteria.dedup();
    criteria
}

fn exposes_of(target: &mitase_spec_model::ArtifactTarget) -> Vec<BoundTargetRef> {
    let mut exposed = target
        .claims
        .iter()
        .filter_map(|claim| match claim {
            TargetClaim::Exposes { target } => Some(target.clone()),
            _ => None,
        })
        .collect::<Vec<_>>();
    exposed.sort();
    exposed.dedup();
    exposed
}

fn binding_entry(index: &SpecIndex, anchor: &SpecAnchor) -> Option<FacetBindingEntry> {
    let binding = index.bindings.get(anchor)?;
    let mut targets = binding
        .targets
        .iter()
        .map(|target| {
            let reference = BoundTargetRef {
                binding: anchor.clone(),
                target_id: target.id.clone(),
            };
            FacetTargetEntry {
                current: index.target_to_artifact.contains_key(&reference),
                id: reference,
                adapter: target.adapter.clone(),
                path: target.path.clone(),
                selector: target.selector.clone(),
                lifecycle: target.lifecycle,
            }
        })
        .collect::<Vec<_>>();
    targets.sort_by(|left, right| left.id.cmp(&right.id));
    Some(FacetBindingEntry {
        id: anchor.clone(),
        role: binding.role,
        facet: binding.facet.clone(),
        responsibility: binding.responsibility.clone(),
        targets,
    })
}

/// Collect the current semantic implementation targets owned by one feature.
///
/// A target joins the matrix only when its owning feature is not planned,
/// its binding role is implementation, it resolves to exactly one current
/// artifact, and it carries a direct satisfies claim.
fn semantic_targets_of_feature(index: &SpecIndex, feature: &SpecId) -> Vec<SemanticTarget> {
    let mut semantic = Vec::new();
    for (anchor, binding) in &index.bindings {
        if anchor.item != *feature || binding.role != BindingRole::Implementation {
            continue;
        }
        if index.item_status.get(&anchor.item) == Some(&ItemStatus::Planned) {
            continue;
        }
        for target in &binding.targets {
            let reference = BoundTargetRef {
                binding: anchor.clone(),
                target_id: target.id.clone(),
            };
            if !index.target_to_artifact.contains_key(&reference) {
                continue;
            }
            if direct_satisfies(target).is_empty() {
                continue;
            }
            semantic.push(SemanticTarget {
                feature: feature.clone(),
                binding: anchor.clone(),
                facet: binding.facet.clone(),
                target: reference,
            });
        }
    }
    semantic
}

/// Collect the current semantic implementation targets of one criterion.
///
/// The index already restricts this relation to current exact targets behind
/// implementation-role bindings with direct satisfies claims; the binding
/// identity is expanded here so facet rows never lose their owner.
fn semantic_targets_of_criterion(index: &SpecIndex, criterion: &SpecAnchor) -> Vec<SemanticTarget> {
    let mut semantic = Vec::new();
    for implementation in index
        .criteria_to_implementation_targets
        .get(criterion)
        .into_iter()
        .flatten()
    {
        let Some(binding) = index.bindings.get(&implementation.binding) else {
            continue;
        };
        semantic.push(SemanticTarget {
            feature: implementation.binding.item.clone(),
            binding: implementation.binding.clone(),
            facet: binding.facet.clone(),
            target: implementation.clone(),
        });
    }
    semantic
}

fn criterion_entries(
    workspace: &SpecWorkspace,
    index: &SpecIndex,
    semantic: &[SemanticTarget],
) -> Vec<FacetCriterionEntry> {
    let mut grouped: CriterionGrouping = BTreeMap::new();
    for entry in semantic {
        let Some(target) = index.target(&entry.target) else {
            continue;
        };
        for criterion in direct_satisfies(target) {
            grouped
                .entry(criterion)
                .or_default()
                .entry((
                    entry.facet.clone(),
                    entry.feature.clone(),
                    entry.binding.clone(),
                ))
                .or_default()
                .insert(entry.target.clone());
        }
    }
    grouped
        .into_iter()
        .map(|(criterion, rows)| {
            let facets = rows
                .into_iter()
                .map(|((facet, feature, binding), targets)| {
                    let implementation_targets = targets.into_iter().collect::<Vec<_>>();
                    let mut verification_targets = implementation_targets
                        .iter()
                        .flat_map(|implementation| {
                            valid_verifications_for(workspace, index, &criterion, implementation)
                        })
                        .collect::<Vec<_>>();
                    verification_targets.sort();
                    verification_targets.dedup();
                    let covered = implementation_targets
                        .iter()
                        .filter(|implementation| {
                            !valid_verifications_for(workspace, index, &criterion, implementation)
                                .is_empty()
                        })
                        .count();
                    let status =
                        aggregate_declared_verification(covered, implementation_targets.len());
                    FacetRow {
                        facet,
                        feature,
                        binding,
                        implementation_targets,
                        declared_verification: DeclaredVerification {
                            status,
                            verification_targets,
                        },
                    }
                })
                .collect::<Vec<_>>();
            FacetCriterionEntry { criterion, facets }
        })
        .collect()
}

fn build_feature_report(
    workspace: &SpecWorkspace,
    index: &SpecIndex,
    source: &str,
    feature: &SpecId,
) -> Result<FacetProjectionReport> {
    let status = feature_record(workspace, feature)
        .ok_or_else(|| anyhow::anyhow!("facet projection source {source} was not found"))?;
    let semantic = semantic_targets_of_feature(index, feature);
    let matrix: BTreeSet<BoundTargetRef> =
        semantic.iter().map(|entry| entry.target.clone()).collect();
    let mut bindings = index
        .bindings
        .keys()
        .filter(|anchor| anchor.item == *feature)
        .filter_map(|anchor| binding_entry(index, anchor))
        .collect::<Vec<_>>();
    bindings.sort_by(|left, right| left.id.cmp(&right.id));
    let mut non_semantic_targets = Vec::new();
    for binding in &bindings {
        let role = index
            .bindings
            .get(&binding.id)
            .map(|entry| entry.role)
            .unwrap_or(BindingRole::Operation);
        for target in &binding.targets {
            if !target.current || matrix.contains(&target.id) {
                continue;
            }
            let exposes = index.target(&target.id).map(exposes_of).unwrap_or_default();
            non_semantic_targets.push(NonSemanticTarget {
                id: target.id.clone(),
                role,
                facet: binding.facet.clone(),
                adapter: target.adapter.clone(),
                path: target.path.clone(),
                selector: target.selector.clone(),
                exposes,
            });
        }
    }
    non_semantic_targets.sort_by(|left, right| left.id.cmp(&right.id));
    Ok(FacetProjectionReport {
        schema_version: CLI_SCHEMA_VERSION.into(),
        contract_version: FACET_PROJECTION_CONTRACT_VERSION.into(),
        source: source.into(),
        source_kind: FacetSourceKind::Feature,
        feature_status: status,
        bindings,
        criteria: criterion_entries(workspace, index, &semantic),
        non_semantic_targets,
    })
}

fn build_criterion_report(
    workspace: &SpecWorkspace,
    index: &SpecIndex,
    source: &str,
    criterion: &SpecAnchor,
) -> Result<FacetProjectionReport> {
    let status = index
        .criterion_status
        .get(criterion)
        .copied()
        .unwrap_or(ItemStatus::Implemented);
    let semantic = semantic_targets_of_criterion(index, criterion);
    let mut binding_anchors = semantic
        .iter()
        .map(|entry| entry.binding.clone())
        .collect::<Vec<_>>();
    binding_anchors.sort();
    binding_anchors.dedup();
    let bindings = binding_anchors
        .iter()
        .filter_map(|anchor| binding_entry(index, anchor))
        .collect::<Vec<_>>();
    Ok(FacetProjectionReport {
        schema_version: CLI_SCHEMA_VERSION.into(),
        contract_version: FACET_PROJECTION_CONTRACT_VERSION.into(),
        source: source.into(),
        source_kind: FacetSourceKind::Criterion,
        feature_status: status,
        bindings,
        criteria: criterion_entries(workspace, index, &semantic),
        non_semantic_targets: Vec::new(),
    })
}

fn cell_value(row: &FacetRow) -> String {
    let status = match row.declared_verification.status {
        DeclaredStatus::Verified => "verified",
        DeclaredStatus::Partial => "partial",
        DeclaredStatus::Unverified => "unverified",
    };
    format!("{} / {status}", row.implementation_targets.len())
}

/// Render the deterministic human-readable projection.
///
/// JSON stays canonical; this presentation never adds meaning.
pub fn render_markdown(report: &FacetProjectionReport) -> String {
    let mut out = String::new();
    out.push_str(&format!("# Facet projection: {}\n\n", report.source));
    out.push_str(&format!(
        "Status: {}\n",
        match report.feature_status {
            ItemStatus::Planned => "planned",
            ItemStatus::Implemented => "implemented",
            ItemStatus::Deprecated => "deprecated",
        }
    ));
    out.push_str(
        "Note: facets are project-defined. Missing facets are not validation failures.\n\n",
    );
    let mut facets = report
        .criteria
        .iter()
        .flat_map(|entry| entry.facets.iter().map(|row| row.facet.clone()))
        .collect::<Vec<_>>();
    facets.sort();
    facets.dedup();
    if report.criteria.is_empty() {
        out.push_str("No current semantic implementation targets.\n\n");
    } else {
        out.push_str("| Criterion |");
        for facet in &facets {
            out.push_str(&format!(" {facet} |"));
        }
        out.push('\n');
        out.push_str("| --- |");
        for _ in &facets {
            out.push_str(" --- |");
        }
        out.push('\n');
        for entry in &report.criteria {
            let cells: BTreeMap<&str, &FacetRow> = entry
                .facets
                .iter()
                .map(|row| (row.facet.as_str(), row))
                .collect();
            out.push_str(&format!("| {} |", entry.criterion));
            for facet in &facets {
                match cells.get(facet.as_str()) {
                    Some(row) => out.push_str(&format!(" {} |", cell_value(row))),
                    None => out.push_str(" - |"),
                }
            }
            out.push('\n');
        }
        out.push('\n');
    }
    let mut rows = report
        .criteria
        .iter()
        .flat_map(|entry| {
            entry
                .facets
                .iter()
                .map(|row| (entry.criterion.clone(), row))
        })
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| {
        left.1
            .facet
            .cmp(&right.1.facet)
            .then_with(|| left.0.cmp(&right.0))
            .then_with(|| left.1.feature.cmp(&right.1.feature))
            .then_with(|| left.1.binding.cmp(&right.1.binding))
    });
    for (criterion, row) in rows {
        out.push_str(&format!("## {} ({criterion})\n\n", row.facet));
        out.push_str(&format!("- feature: {}\n", row.feature));
        out.push_str(&format!("- binding: {}\n", row.binding));
        for target in &row.implementation_targets {
            out.push_str(&format!("- implementation: {target}\n"));
        }
        if row.declared_verification.verification_targets.is_empty() {
            out.push_str("- declared verification: none [unverified]\n");
        } else {
            for verification in &row.declared_verification.verification_targets {
                out.push_str(&format!(
                    "- declared verification: {verification} [valid]\n"
                ));
            }
        }
        out.push('\n');
    }
    out.push_str("## Non-semantic targets\n\n");
    if report.non_semantic_targets.is_empty() {
        out.push_str("None.\n");
    } else {
        for target in &report.non_semantic_targets {
            let role = serde_json::to_value(target.role)
                .ok()
                .and_then(|value| value.as_str().map(ToOwned::to_owned))
                .unwrap_or_else(|| "unknown".into());
            out.push_str(&format!(
                "- facet={} role={} {}\n",
                target.facet, role, target.id
            ));
            for exposed in &target.exposes {
                out.push_str(&format!("  - exposes {exposed}\n"));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn facet_fixture() -> (SpecWorkspace, SpecIndex) {
        let root =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/acceptance/facet-oriented-v2");
        let workspace = SpecWorkspace::load(&root).expect("facet fixture workspace");
        let index = workspace.index().expect("facet fixture index");
        (workspace, index)
    }

    fn creation(report: &FacetProjectionReport) -> &FacetCriterionEntry {
        report
            .criteria
            .iter()
            .find(|entry| entry.criterion.to_string() == "REQ-FACET-001#criterion.creation")
            .expect("creation criterion entry")
    }

    #[test]
    fn projects_multiple_facets_for_one_criterion() {
        let (workspace, index) = facet_fixture();
        let report = build_facet_projection(&workspace, &index, "FEAT-FACET-001")
            .expect("feature projection");
        assert_eq!(report.schema_version, "mitase/cli/v1");
        assert_eq!(report.contract_version, FACET_PROJECTION_CONTRACT_VERSION);
        assert_eq!(report.source_kind, FacetSourceKind::Feature);
        assert_eq!(report.feature_status, ItemStatus::Implemented);
        assert_eq!(report.criteria.len(), 2);
        let entry = creation(&report);
        let facets = entry
            .facets
            .iter()
            .map(|row| row.facet.clone())
            .collect::<Vec<_>>();
        assert_eq!(
            facets,
            vec!["core", "frontend", "mcp", "weird-project-specific-name"]
        );
        for row in &entry.facets {
            assert_eq!(row.feature.to_string(), "FEAT-FACET-001");
            assert_eq!(row.declared_verification.status, DeclaredStatus::Verified);
            assert!(!row.declared_verification.verification_targets.is_empty());
        }
        let recovery = report
            .criteria
            .iter()
            .find(|entry| entry.criterion.to_string() == "REQ-FACET-001#criterion.recovery")
            .expect("recovery criterion entry");
        assert_eq!(recovery.facets.len(), 1);
        assert_eq!(recovery.facets[0].facet, "core");
    }

    #[test]
    fn keeps_opaque_facets_and_lists_non_semantic_targets() {
        let (workspace, index) = facet_fixture();
        let report = build_facet_projection(&workspace, &index, "FEAT-FACET-001")
            .expect("feature projection");
        assert_eq!(report.bindings.len(), 7);
        let entry = creation(&report);
        let weird = entry
            .facets
            .iter()
            .find(|row| row.facet == "weird-project-specific-name")
            .expect("opaque facet row");
        assert_eq!(
            weird.implementation_targets,
            vec![
                "FEAT-FACET-001#binding.weird/target.batch-tool"
                    .parse()
                    .unwrap()
            ]
        );
        let semantic: BTreeSet<BoundTargetRef> = report
            .criteria
            .iter()
            .flat_map(|entry| entry.facets.iter())
            .flat_map(|row| row.implementation_targets.iter().cloned())
            .collect();
        assert!(
            !semantic.contains(&"FEAT-FACET-001#binding.ops/target.router".parse().unwrap()),
            "operation targets must not join the semantic matrix"
        );
        let legacy = report
            .non_semantic_targets
            .iter()
            .find(|target| {
                target.id.to_string() == "FEAT-FACET-001#binding.legacy/target.legacy-entry"
            })
            .expect("exposes-only target stays visible");
        assert_eq!(
            legacy.exposes,
            vec![
                "FEAT-FACET-001#binding.core/target.create-entry"
                    .parse()
                    .unwrap()
            ]
        );
        assert_eq!(report.non_semantic_targets.len(), 3);
    }

    #[test]
    fn invalid_runner_metadata_aggregates_to_unverified_without_passed_language() {
        let (mut workspace, _) = facet_fixture();
        workspace.config.verification.runners.clear();
        let index = workspace.index().expect("facet fixture index");
        let report = build_facet_projection(&workspace, &index, "FEAT-FACET-001")
            .expect("feature projection");
        let entry = creation(&report);
        for row in &entry.facets {
            assert_eq!(row.declared_verification.status, DeclaredStatus::Unverified);
            assert!(row.declared_verification.verification_targets.is_empty());
        }
        let rendered = render_markdown(&report);
        assert!(!rendered.contains("passed"));
        assert!(rendered.contains("[unverified]"));
    }

    #[test]
    fn aggregation_reports_verified_partial_and_unverified() {
        assert_eq!(
            aggregate_declared_verification(2, 2),
            DeclaredStatus::Verified
        );
        assert_eq!(
            aggregate_declared_verification(1, 2),
            DeclaredStatus::Partial
        );
        assert_eq!(
            aggregate_declared_verification(0, 2),
            DeclaredStatus::Unverified
        );
        assert_eq!(
            aggregate_declared_verification(0, 0),
            DeclaredStatus::Unverified
        );
    }

    #[test]
    fn criterion_source_expands_owners_without_losing_identity() {
        let (workspace, index) = facet_fixture();
        let report = build_facet_projection(&workspace, &index, "REQ-FACET-001#criterion.creation")
            .expect("criterion projection");
        assert_eq!(report.source_kind, FacetSourceKind::Criterion);
        assert_eq!(report.criteria.len(), 1);
        assert_eq!(report.non_semantic_targets.len(), 0);
        let rows = &report.criteria[0].facets;
        assert_eq!(rows.len(), 4);
        for row in rows {
            assert_eq!(row.feature.to_string(), "FEAT-FACET-001");
            assert!(
                row.binding
                    .to_string()
                    .starts_with("FEAT-FACET-001#binding.")
            );
        }
    }

    #[test]
    fn unsupported_sources_are_top_level_errors() {
        let (workspace, index) = facet_fixture();
        for source in [
            "REQ-FACET-001",
            "POL-FACET-001",
            "FEAT-FACET-001#binding.core",
            "FEAT-FACET-001#binding.core/target.create-entry",
            "REQ-DOES-NOT-EXIST#criterion.missing",
            "FEAT-DOES-NOT-EXIST",
            "crates/facet-core/src/lib.rs",
        ] {
            assert!(
                build_facet_projection(&workspace, &index, source).is_err(),
                "{source} must be rejected"
            );
        }
    }

    #[test]
    fn markdown_is_deterministic_and_marks_missing_facets_as_blank() {
        let (workspace, index) = facet_fixture();
        let report = build_facet_projection(&workspace, &index, "FEAT-FACET-001")
            .expect("feature projection");
        let first = render_markdown(&report);
        let second = render_markdown(
            &build_facet_projection(&workspace, &index, "FEAT-FACET-001").expect("repeat"),
        );
        assert_eq!(first, second);
        assert!(first.starts_with("# Facet projection: FEAT-FACET-001\n"));
        assert!(first.contains("Missing facets are not validation failures."));
        assert!(first.contains("weird-project-specific-name"));
        assert!(first.contains("## Non-semantic targets"));
        assert!(first.contains("exposes FEAT-FACET-001#binding.core/target.create-entry"));
    }
}
