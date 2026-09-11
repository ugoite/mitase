use anyhow::{Result, bail};
use clap::ValueEnum;
use mitase_spec_model::{
    ArtifactBinding, ArtifactTargetLifecycle, BindingRole, BoundTargetRef, ItemStatus,
    LocalAnchorKind, SpecAnchor, SpecDocument, SpecId, TargetClaim,
};
use mitase_validation::{
    VerificationAssessment, VerificationAssessmentReason, VerificationAssessmentStatus,
    assess_verification_claim,
};
use mitase_workspace::{AnchorValue, SpecIndex, SpecWorkspace};
use serde::Serialize;
use std::{fmt::Write as _, path::Path};

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum SpecKind {
    Philosophy,
    Policy,
    Requirement,
    Feature,
}

impl SpecKind {
    fn label(self) -> &'static str {
        match self {
            Self::Philosophy => "philosophy",
            Self::Policy => "policy",
            Self::Requirement => "requirement",
            Self::Feature => "feature",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum StatusFilter {
    Planned,
    Implemented,
    Deprecated,
}

impl StatusFilter {
    fn matches(self, status: Option<ItemStatus>) -> bool {
        status.is_some_and(|actual| {
            matches!(
                (self, actual),
                (Self::Planned, ItemStatus::Planned)
                    | (Self::Implemented, ItemStatus::Implemented)
                    | (Self::Deprecated, ItemStatus::Deprecated)
            )
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ListResult {
    pub items: Vec<ListItem>,
    pub unverified_criteria: Vec<CriterionView>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ListItem {
    pub id: SpecId,
    pub kind: SpecKind,
    pub namespace: String,
    pub category: String,
    pub title: String,
    pub status: Option<ItemStatus>,
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ShowResult {
    pub id: SpecId,
    pub kind: SpecKind,
    pub title: String,
    pub summary: String,
    pub description: String,
    pub status: Option<ItemStatus>,
    pub source: String,
    pub anchors: Vec<SpecAnchor>,
    pub criteria: Vec<CriterionView>,
    pub authored_relations: Vec<RelationView>,
    pub derived_relations: Vec<RelationView>,
    pub bindings: Vec<BindingView>,
    pub verification_claims: Vec<VerificationView>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CriterionView {
    pub id: SpecAnchor,
    pub kind: mitase_spec_model::CriterionKind,
    pub statement: String,
    pub implementation_targets: Vec<BoundTargetRef>,
    pub verification_targets: Vec<BoundTargetRef>,
    pub verification: CriterionVerification,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum CriterionVerification {
    Verified,
    Unverified,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RelationView {
    pub relation: String,
    pub source: String,
    pub targets: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct QueryResult {
    pub source: String,
    pub relations: Vec<RelationView>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BindingView {
    pub id: SpecAnchor,
    pub role: BindingRole,
    pub facet: String,
    pub responsibility: String,
    pub owns: Vec<mitase_spec_model::OwnershipScope>,
    pub targets: Vec<TargetView>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TargetView {
    pub id: BoundTargetRef,
    pub adapter: String,
    pub path: mitase_spec_model::RepoPath,
    pub selector: mitase_spec_model::Selector,
    pub lifecycle: ArtifactTargetLifecycle,
    pub current: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artifact: Option<String>,
    pub claims: Vec<ClaimView>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum ClaimView {
    Satisfies {
        criterion: SpecAnchor,
    },
    Verifies {
        criterion: SpecAnchor,
        covers: Vec<BoundTargetRef>,
        runner: String,
    },
    Documents {
        anchor: SpecAnchor,
    },
    Enforces {
        rule: SpecAnchor,
    },
    GeneratedFrom {
        targets: Vec<BoundTargetRef>,
    },
    Exposes {
        target: BoundTargetRef,
    },
    Evidences {
        anchor: SpecAnchor,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct VerificationView {
    pub verification: BoundTargetRef,
    pub criterion: SpecAnchor,
    pub covers: Vec<BoundTargetRef>,
    pub runner: String,
    pub assessment: VerificationAssessment,
}

#[derive(Debug, Clone)]
struct ItemRecord {
    id: SpecId,
    kind: SpecKind,
    namespace: String,
    category: String,
    title: String,
    summary: String,
    description: String,
    status: Option<ItemStatus>,
    source: String,
    bindings: Vec<(SpecAnchor, ArtifactBinding)>,
}

pub fn list(
    workspace: &SpecWorkspace,
    index: &SpecIndex,
    kind: Option<SpecKind>,
    status: Option<StatusFilter>,
    namespace: Option<&str>,
    category: Option<&str>,
    include_unverified_criteria: bool,
) -> ListResult {
    let records = item_records(workspace, index);
    let items = records
        .iter()
        .filter(|item| kind.is_none_or(|expected| expected == item.kind))
        .filter(|item| status.is_none_or(|expected| expected.matches(item.status)))
        .filter(|item| namespace.is_none_or(|expected| expected == item.namespace))
        .filter(|item| category.is_none_or(|expected| expected == item.category))
        .map(|item| ListItem {
            id: item.id.clone(),
            kind: item.kind,
            namespace: item.namespace.clone(),
            category: item.category.clone(),
            title: item.title.clone(),
            status: item.status,
            source: item.source.clone(),
        })
        .collect();
    let unverified_criteria = if include_unverified_criteria {
        records
            .iter()
            .filter(|item| kind.is_none_or(|expected| expected == item.kind))
            .filter(|item| status.is_none_or(|expected| expected.matches(item.status)))
            .filter(|item| namespace.is_none_or(|expected| expected == item.namespace))
            .filter(|item| category.is_none_or(|expected| expected == item.category))
            .filter(|item| item.status == Some(ItemStatus::Implemented))
            .flat_map(|item| criterion_views(&workspace.config, index, item))
            .filter(|criterion| criterion.verification == CriterionVerification::Unverified)
            .collect()
    } else {
        Vec::new()
    };
    ListResult {
        items,
        unverified_criteria,
    }
}

pub fn show(workspace: &SpecWorkspace, index: &SpecIndex, id: &str) -> Result<ShowResult> {
    let records = item_records(workspace, index);
    let matches = records
        .iter()
        .filter(|item| item.id.0 == id)
        .collect::<Vec<_>>();
    let item = match matches.as_slice() {
        [] => bail!("specification {id} was not found"),
        [item] => *item,
        _ => bail!("specification {id} is ambiguous"),
    };

    let anchors = index
        .item_anchors
        .get(&item.id)
        .cloned()
        .unwrap_or_default();
    let criteria = criterion_views(&workspace.config, index, item);
    let mut authored_relations = Vec::new();
    let mut derived_relations = Vec::new();
    for anchor in &anchors {
        add_authored_relations(index, anchor, &mut authored_relations);
        add_derived_relations(index, anchor, &mut derived_relations);
    }
    authored_relations.sort();
    derived_relations.sort();

    let bindings = item
        .bindings
        .iter()
        .map(|(anchor, binding)| binding_view(index, anchor, binding))
        .collect();
    let verification_claims = verification_claims_for(index, &workspace.config, &item.id);

    Ok(ShowResult {
        id: item.id.clone(),
        kind: item.kind,
        title: item.title.clone(),
        summary: item.summary.clone(),
        description: item.description.clone(),
        status: item.status,
        source: item.source.clone(),
        anchors,
        criteria,
        authored_relations,
        derived_relations,
        bindings,
        verification_claims,
    })
}

/// Query explicit authored and derived relations for one canonical graph source.
///
/// A source may be a specification ID, a local specification anchor, or an
/// exact bound target reference. The result is deliberately limited to
/// relations already represented by `SpecIndex`; it does not infer impact from
/// source-language dependencies or call graphs.
pub fn query(
    workspace: &SpecWorkspace,
    index: &SpecIndex,
    source: &str,
    relation: Option<&str>,
) -> Result<QueryResult> {
    let mut relations = Vec::new();
    if let Ok(reference) = source.parse::<BoundTargetRef>() {
        if index.target(&reference).is_none() {
            bail!("query source {source} was not found");
        }
        add_authored_relations(index, &reference.binding, &mut relations);
        add_derived_relations(index, &reference.binding, &mut relations);
        relations.retain(|entry| entry.source == source);
    } else if let Ok(anchor) = source.parse::<SpecAnchor>() {
        if index.anchor(&anchor).is_none() {
            bail!("query source {source} was not found");
        }
        add_authored_relations(index, &anchor, &mut relations);
        add_derived_relations(index, &anchor, &mut relations);
    } else {
        let matches = item_records(workspace, index)
            .into_iter()
            .filter(|item| item.id.0 == source)
            .collect::<Vec<_>>();
        match matches.as_slice() {
            [] => bail!("query source {source} was not found"),
            [_] => {
                let anchors = index
                    .item_anchors
                    .get(&matches[0].id)
                    .cloned()
                    .unwrap_or_default();
                for anchor in anchors {
                    add_authored_relations(index, &anchor, &mut relations);
                    add_derived_relations(index, &anchor, &mut relations);
                }
            }
            _ => bail!("query source {source} is ambiguous"),
        }
    }

    relations.sort();
    relations.dedup();
    if let Some(expected) = relation {
        relations.retain(|entry| entry.relation == expected);
    }
    Ok(QueryResult {
        source: source.into(),
        relations,
    })
}

pub fn render_list_text(result: &ListResult) -> String {
    let mut output = String::new();
    for item in &result.items {
        let status = item
            .status
            .map(item_status_label)
            .unwrap_or("unstatus-bearing");
        let _ = writeln!(
            output,
            "{} {} [{}] {}/{} — {} ({})",
            item.kind.label(),
            item.id,
            status,
            item.namespace,
            item.category,
            item.title,
            item.source
        );
    }
    if !result.unverified_criteria.is_empty() {
        output.push_str("Unverified criteria:\n");
        for criterion in &result.unverified_criteria {
            let _ = writeln!(output, "  {} — {}", criterion.id, criterion.statement);
        }
    }
    output
}

pub fn render_query_text(result: &QueryResult) -> String {
    let mut output = String::new();
    let _ = writeln!(output, "Query source: {}", result.source);
    write_relations(&mut output, "Relations", &result.relations);
    output
}

pub fn render_show_text(result: &ShowResult) -> String {
    let mut output = String::new();
    let _ = writeln!(output, "{} {}", result.kind.label(), result.id);
    let _ = writeln!(output, "Title: {}", result.title);
    let _ = writeln!(output, "Source: {}", result.source);
    if let Some(status) = result.status {
        let _ = writeln!(output, "Status: {}", item_status_label(status));
    }
    if !result.summary.is_empty() {
        let _ = writeln!(output, "Summary: {}", result.summary);
    }
    if !result.description.is_empty() {
        let _ = writeln!(output, "Description: {}", result.description);
    }
    render_evidence_trace(&mut output, result);
    write_relations(
        &mut output,
        "Authored relations",
        &result.authored_relations,
    );
    write_relations(&mut output, "Derived relations", &result.derived_relations);
    if !result.bindings.is_empty() {
        output.push_str("Bindings:\n");
        for binding in &result.bindings {
            let _ = writeln!(
                output,
                "  {} [{}] {}",
                binding.id,
                serde_json::to_value(binding.role)
                    .ok()
                    .and_then(|value| value.as_str().map(ToOwned::to_owned))
                    .unwrap_or_else(|| "unknown".into()),
                binding.responsibility
            );
            for target in &binding.targets {
                let current = if target.current {
                    "current"
                } else {
                    "catalog-only"
                };
                let artifact = target
                    .artifact
                    .as_deref()
                    .map(|value| format!(" -> {value}"))
                    .unwrap_or_default();
                let _ = writeln!(
                    output,
                    "    {} [{}] {}{}",
                    target.id,
                    current,
                    target.path.display(),
                    artifact
                );
            }
        }
    }
    if !result.verification_claims.is_empty() {
        output.push_str("Verification claims:\n");
        for claim in &result.verification_claims {
            let assessment = serde_json::to_value(claim.assessment.status)
                .ok()
                .and_then(|value| value.as_str().map(ToOwned::to_owned))
                .unwrap_or_else(|| "unknown".into());
            let reason = claim
                .assessment
                .reason
                .map(verification_reason_label)
                .map(|value| format!(" ({value})"))
                .unwrap_or_default();
            let _ = writeln!(
                output,
                "  {} verifies {} [{}]{}",
                claim.verification, claim.criterion, assessment, reason
            );
        }
    }
    output
}

fn render_evidence_trace(output: &mut String, result: &ShowResult) {
    if result.criteria.is_empty() {
        return;
    }
    output.push_str("Evidence trace:\n");
    for criterion in &result.criteria {
        let verification_targets = verification_targets_for_trace(result, criterion);
        let status = match criterion.verification {
            CriterionVerification::Verified => "verified",
            CriterionVerification::Unverified => "unverified",
        };
        let verification_count = verification_targets
            .iter()
            .filter(|target| verification_status_for(result, &criterion.id, target) == "valid")
            .count();
        let _ = writeln!(output, "  {} [{}]", criterion.id, status);
        let _ = writeln!(
            output,
            "    implements {} · verifies {}/{}",
            criterion.implementation_targets.len(),
            verification_count,
            verification_targets.len()
        );
        let _ = writeln!(output, "    statement: {}", criterion.statement);
        if criterion.implementation_targets.is_empty() {
            output.push_str("    implements: none\n");
        } else {
            output.push_str("    implements:\n");
            for target in &criterion.implementation_targets {
                let _ = writeln!(output, "      - {target}");
            }
        }
        if verification_targets.is_empty() {
            output.push_str("    verifies: none\n");
        } else {
            output.push_str("    verifies:\n");
            for target in &verification_targets {
                let _ = writeln!(
                    output,
                    "      - {} [{}]",
                    target,
                    verification_status_for(result, &criterion.id, target)
                );
            }
        }
    }
}

fn verification_targets_for_trace(
    result: &ShowResult,
    criterion: &CriterionView,
) -> Vec<BoundTargetRef> {
    let mut targets = criterion.verification_targets.clone();
    targets.extend(
        result
            .verification_claims
            .iter()
            .filter(|claim| claim.criterion == criterion.id)
            .map(|claim| claim.verification.clone()),
    );
    targets.sort();
    targets.dedup();
    targets
}

fn verification_status_for(
    result: &ShowResult,
    criterion: &SpecAnchor,
    target: &BoundTargetRef,
) -> &'static str {
    let mut claims = result
        .verification_claims
        .iter()
        .filter(|claim| claim.criterion == *criterion && claim.verification == *target);
    if claims
        .clone()
        .any(|claim| claim.assessment.status == VerificationAssessmentStatus::Valid)
    {
        "valid"
    } else if claims
        .clone()
        .any(|claim| claim.assessment.status == VerificationAssessmentStatus::Invalid)
    {
        "invalid"
    } else if claims
        .any(|claim| claim.assessment.status == VerificationAssessmentStatus::CatalogOnly)
    {
        "catalog-only"
    } else {
        "unresolved"
    }
}

fn item_records(workspace: &SpecWorkspace, index: &SpecIndex) -> Vec<ItemRecord> {
    let mut records = Vec::new();
    for loaded in &workspace.documents {
        match &loaded.document {
            SpecDocument::Philosophies {
                namespace,
                category,
                philosophies,
                ..
            } => {
                for item in philosophies {
                    records.push(item_record(
                        workspace,
                        index,
                        namespace,
                        category,
                        item.id.clone(),
                        SpecKind::Philosophy,
                        item.title.clone(),
                        item.summary.clone(),
                        String::new(),
                        None,
                        item.bindings.clone(),
                        &loaded.path,
                    ));
                }
            }
            SpecDocument::Policies {
                namespace,
                category,
                policies,
                ..
            } => {
                for item in policies {
                    records.push(item_record(
                        workspace,
                        index,
                        namespace,
                        category,
                        item.id.clone(),
                        SpecKind::Policy,
                        item.title.clone(),
                        item.summary.clone(),
                        item.description.clone(),
                        None,
                        item.bindings.clone(),
                        &loaded.path,
                    ));
                }
            }
            SpecDocument::Requirements {
                namespace,
                category,
                requirements,
                ..
            } => {
                for item in requirements {
                    records.push(item_record(
                        workspace,
                        index,
                        namespace,
                        category,
                        item.id.clone(),
                        SpecKind::Requirement,
                        item.title.clone(),
                        String::new(),
                        String::new(),
                        Some(item.status),
                        item.bindings.clone(),
                        &loaded.path,
                    ));
                }
            }
            SpecDocument::Features {
                namespace,
                category,
                features,
                ..
            } => {
                for item in features {
                    records.push(item_record(
                        workspace,
                        index,
                        namespace,
                        category,
                        item.id.clone(),
                        SpecKind::Feature,
                        item.title.clone(),
                        item.summary.clone(),
                        String::new(),
                        Some(item.status),
                        item.bindings.clone(),
                        &loaded.path,
                    ));
                }
            }
        }
    }
    records.sort_by(|left, right| left.id.cmp(&right.id));
    records
}

#[allow(clippy::too_many_arguments)]
fn item_record(
    workspace: &SpecWorkspace,
    index: &SpecIndex,
    namespace: &str,
    category: &str,
    id: SpecId,
    kind: SpecKind,
    title: String,
    summary: String,
    description: String,
    status: Option<ItemStatus>,
    bindings: Vec<ArtifactBinding>,
    source: &Path,
) -> ItemRecord {
    let binding_views = bindings
        .into_iter()
        .map(|binding| {
            let anchor = SpecAnchor {
                item: id.clone(),
                kind: LocalAnchorKind::Binding,
                local_id: binding.id.clone(),
            };
            (anchor, binding)
        })
        .filter(|(anchor, _)| index.bindings.contains_key(anchor))
        .collect();
    ItemRecord {
        id,
        kind,
        namespace: namespace.into(),
        category: category.into(),
        title,
        summary,
        description,
        status,
        source: relative_path(workspace, source),
        bindings: binding_views,
    }
}

fn criterion_views(
    config: &mitase_project_model::ProjectConfig,
    index: &SpecIndex,
    item: &ItemRecord,
) -> Vec<CriterionView> {
    let mut criteria = Vec::new();
    for anchor in index
        .item_anchors
        .get(&item.id)
        .into_iter()
        .flatten()
        .filter(|anchor| anchor.kind == LocalAnchorKind::Criterion)
    {
        let Some(AnchorValue::Criterion(criterion)) = index.anchor(anchor) else {
            continue;
        };
        let implementation_targets = index
            .criteria_to_implementation_targets
            .get(anchor)
            .cloned()
            .unwrap_or_default();
        let verification_targets = index
            .criteria_to_verification_targets
            .get(anchor)
            .cloned()
            .unwrap_or_default();
        let verified = !implementation_targets.is_empty()
            && implementation_targets.iter().all(|implementation| {
                index
                    .verification_by_target
                    .get(implementation)
                    .into_iter()
                    .flatten()
                    .any(|verification| {
                        assess_verification_claim(config, index, verification, anchor).status
                            == VerificationAssessmentStatus::Valid
                    })
            });
        criteria.push(CriterionView {
            id: anchor.clone(),
            kind: criterion.kind,
            statement: criterion.statement.clone(),
            implementation_targets,
            verification_targets,
            verification: if verified {
                CriterionVerification::Verified
            } else {
                CriterionVerification::Unverified
            },
        });
    }
    criteria.sort_by(|left, right| left.id.cmp(&right.id));
    criteria
}

fn relative_path(workspace: &SpecWorkspace, path: &Path) -> String {
    path.strip_prefix(&workspace.root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn add_authored_relations(index: &SpecIndex, anchor: &SpecAnchor, output: &mut Vec<RelationView>) {
    match index.anchor(anchor) {
        Some(AnchorValue::Rule(_)) => {
            if let Some(targets) = index.rules_to_principles.get(anchor) {
                add_relation(
                    output,
                    "governed-by",
                    anchor,
                    targets.iter().map(ToString::to_string),
                );
            }
        }
        Some(AnchorValue::Criterion(_)) => {
            if let Some(targets) = index.criteria_to_rules.get(anchor) {
                add_relation(
                    output,
                    "governed-by",
                    anchor,
                    targets.iter().map(ToString::to_string),
                );
            }
        }
        Some(AnchorValue::Binding(binding)) => {
            for target in &binding.targets {
                let target_ref = BoundTargetRef {
                    binding: anchor.clone(),
                    target_id: target.id.clone(),
                };
                for claim in &target.claims {
                    match claim {
                        TargetClaim::Satisfies { criterion } => {
                            add_relation(output, "satisfies", &target_ref, [criterion.to_string()])
                        }
                        TargetClaim::Verifies {
                            criterion, covers, ..
                        } => {
                            add_relation(output, "verifies", &target_ref, [criterion.to_string()]);
                            add_relation(
                                output,
                                "covers",
                                &target_ref,
                                covers.iter().map(ToString::to_string),
                            );
                        }
                        TargetClaim::Documents { anchor } => {
                            add_relation(output, "documents", &target_ref, [anchor.to_string()]);
                        }
                        TargetClaim::Enforces { rule } => {
                            add_relation(output, "enforces", &target_ref, [rule.to_string()]);
                        }
                        TargetClaim::GeneratedFrom { targets } => add_relation(
                            output,
                            "generated-from",
                            &target_ref,
                            targets.iter().map(ToString::to_string),
                        ),
                        TargetClaim::Exposes { target } => {
                            add_relation(output, "exposes", &target_ref, [target.to_string()]);
                        }
                        TargetClaim::Evidences { anchor } => {
                            add_relation(output, "evidences", &target_ref, [anchor.to_string()]);
                        }
                    }
                }
            }
        }
        Some(AnchorValue::Contract(contract)) => {
            add_relation(output, "source", anchor, [contract.source.to_string()]);
            add_relation(
                output,
                "participants",
                anchor,
                contract
                    .participants
                    .iter()
                    .map(|participant| format!("{} ({})", participant.target, participant.role)),
            );
            add_relation(
                output,
                "guarantees",
                anchor,
                contract.guarantees.iter().map(ToString::to_string),
            );
        }
        Some(AnchorValue::Principle(_)) | None => {}
    }
}

fn add_derived_relations(index: &SpecIndex, anchor: &SpecAnchor, output: &mut Vec<RelationView>) {
    match anchor.kind {
        LocalAnchorKind::Principle => {
            if let Some(targets) = index.principles_to_rules.get(anchor) {
                add_relation(
                    output,
                    "reverse-governed-by",
                    anchor,
                    targets.iter().map(ToString::to_string),
                );
            }
        }
        LocalAnchorKind::Criterion => {
            if let Some(targets) = index.criteria_to_implementation_targets.get(anchor) {
                add_relation(
                    output,
                    "implementation-targets",
                    anchor,
                    targets.iter().map(ToString::to_string),
                );
            }
            if let Some(targets) = index.criteria_to_verification_targets.get(anchor) {
                add_relation(
                    output,
                    "verification-targets",
                    anchor,
                    targets.iter().map(ToString::to_string),
                );
            }
            if let Some(targets) = index.all_criteria_to_implementation_targets.get(anchor) {
                add_relation(
                    output,
                    "catalog-implementation-targets",
                    anchor,
                    targets.iter().map(ToString::to_string),
                );
            }
            if let Some(targets) = index.all_criteria_to_verification_targets.get(anchor) {
                add_relation(
                    output,
                    "catalog-verification-targets",
                    anchor,
                    targets.iter().map(ToString::to_string),
                );
            }
        }
        LocalAnchorKind::Binding => {
            for target in index
                .bindings
                .get(anchor)
                .into_iter()
                .flat_map(|binding| binding.targets.iter())
            {
                let target_ref = BoundTargetRef {
                    binding: anchor.clone(),
                    target_id: target.id.clone(),
                };
                if let Some(contracts) = index.contracts_by_target.get(&target_ref) {
                    add_relation(
                        output,
                        "contracts",
                        &target_ref,
                        contracts.iter().map(ToString::to_string),
                    );
                }
                if let Some(generated) = index.generated_by_source.get(&target_ref) {
                    add_relation(
                        output,
                        "generated-targets",
                        &target_ref,
                        generated.iter().map(ToString::to_string),
                    );
                }
            }
        }
        LocalAnchorKind::Rule | LocalAnchorKind::Contract => {}
    }
}

fn add_relation(
    output: &mut Vec<RelationView>,
    relation: &str,
    source: &impl ToString,
    targets: impl IntoIterator<Item = String>,
) {
    let mut targets = targets.into_iter().collect::<Vec<_>>();
    if targets.is_empty() {
        return;
    }
    targets.sort();
    targets.dedup();
    output.push(RelationView {
        relation: relation.into(),
        source: source.to_string(),
        targets,
    });
}

fn binding_view(index: &SpecIndex, anchor: &SpecAnchor, binding: &ArtifactBinding) -> BindingView {
    let targets = binding
        .targets
        .iter()
        .map(|target| {
            let reference = BoundTargetRef {
                binding: anchor.clone(),
                target_id: target.id.clone(),
            };
            let mut claims = target.claims.iter().map(claim_view).collect::<Vec<_>>();
            claims.sort_by_key(|claim| serde_json::to_string(claim).unwrap_or_default());
            TargetView {
                id: reference.clone(),
                adapter: target.adapter.clone(),
                path: target.path.clone(),
                selector: target.selector.clone(),
                lifecycle: target.lifecycle,
                current: index.target_to_artifact.contains_key(&reference),
                artifact: index.target_to_artifact.get(&reference).cloned(),
                claims,
            }
        })
        .collect();
    BindingView {
        id: anchor.clone(),
        role: binding.role,
        facet: binding.facet.clone(),
        responsibility: binding.responsibility.clone(),
        owns: binding.owns.clone(),
        targets,
    }
}

fn claim_view(claim: &TargetClaim) -> ClaimView {
    match claim {
        TargetClaim::Satisfies { criterion } => ClaimView::Satisfies {
            criterion: criterion.clone(),
        },
        TargetClaim::Verifies {
            criterion,
            covers,
            runner,
        } => ClaimView::Verifies {
            criterion: criterion.clone(),
            covers: covers.clone(),
            runner: runner.runner.clone(),
        },
        TargetClaim::Documents { anchor } => ClaimView::Documents {
            anchor: anchor.clone(),
        },
        TargetClaim::Enforces { rule } => ClaimView::Enforces { rule: rule.clone() },
        TargetClaim::GeneratedFrom { targets } => ClaimView::GeneratedFrom {
            targets: targets.clone(),
        },
        TargetClaim::Exposes { target } => ClaimView::Exposes {
            target: target.clone(),
        },
        TargetClaim::Evidences { anchor } => ClaimView::Evidences {
            anchor: anchor.clone(),
        },
    }
}

fn verification_claims_for(
    index: &SpecIndex,
    config: &mitase_project_model::ProjectConfig,
    item: &SpecId,
) -> Vec<VerificationView> {
    let mut claims = Vec::new();
    for (binding_anchor, binding) in &index.bindings {
        if binding.role != BindingRole::Verification {
            continue;
        }
        for target in &binding.targets {
            let verification = BoundTargetRef {
                binding: binding_anchor.clone(),
                target_id: target.id.clone(),
            };
            for claim in &target.claims {
                let TargetClaim::Verifies {
                    criterion,
                    covers,
                    runner,
                } = claim
                else {
                    continue;
                };
                if criterion.item != *item && verification.binding.item != *item {
                    continue;
                }
                claims.push(VerificationView {
                    verification: verification.clone(),
                    criterion: criterion.clone(),
                    covers: covers.clone(),
                    runner: runner.runner.clone(),
                    assessment: assess_verification_claim(config, index, &verification, criterion),
                });
            }
        }
    }
    claims.sort_by(|left, right| {
        left.criterion
            .cmp(&right.criterion)
            .then_with(|| left.verification.cmp(&right.verification))
    });
    claims
}

fn write_relations(output: &mut String, title: &str, relations: &[RelationView]) {
    if relations.is_empty() {
        return;
    }
    let _ = writeln!(output, "{title}:");
    for relation in relations {
        let _ = writeln!(
            output,
            "  {} {} -> {}",
            relation.source,
            relation.relation,
            relation.targets.join(", ")
        );
    }
}

fn item_status_label(status: ItemStatus) -> &'static str {
    match status {
        ItemStatus::Planned => "planned",
        ItemStatus::Implemented => "implemented",
        ItemStatus::Deprecated => "deprecated",
    }
}

fn verification_reason_label(reason: VerificationAssessmentReason) -> String {
    serde_json::to_value(reason)
        .ok()
        .and_then(|value| value.as_str().map(ToOwned::to_owned))
        .unwrap_or_else(|| "unknown".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn show_text_renders_a_single_evidence_trace() {
        let criterion: SpecAnchor = "REQ-SEARCH-004#criterion.advanced".parse().unwrap();
        let implementation: BoundTargetRef = "FEAT-SEARCH-001#binding.implementation/target.search"
            .parse()
            .unwrap();
        let verification: BoundTargetRef = "FEAT-SEARCH-001#binding.verification/target.contract"
            .parse()
            .unwrap();
        let result = ShowResult {
            id: SpecId::from("REQ-SEARCH-004"),
            kind: SpecKind::Requirement,
            title: "Advanced search".into(),
            summary: "Shared search semantics".into(),
            description: String::new(),
            status: Some(ItemStatus::Implemented),
            source: "spec/requirements.yaml".into(),
            anchors: vec![criterion.clone()],
            criteria: vec![CriterionView {
                id: criterion.clone(),
                kind: mitase_spec_model::CriterionKind::Behavior,
                statement: "Search uses shared semantics.".into(),
                implementation_targets: vec![implementation.clone()],
                verification_targets: vec![verification.clone()],
                verification: CriterionVerification::Verified,
            }],
            authored_relations: Vec::new(),
            derived_relations: Vec::new(),
            bindings: Vec::new(),
            verification_claims: vec![VerificationView {
                verification: verification.clone(),
                criterion: criterion.clone(),
                covers: vec![implementation],
                runner: "cargo-test".into(),
                assessment: VerificationAssessment {
                    status: VerificationAssessmentStatus::Valid,
                    verification,
                    criterion,
                    covers: Vec::new(),
                    reason: None,
                },
            }],
        };

        let rendered = render_show_text(&result);

        assert!(rendered.contains("Evidence trace:"));
        assert!(rendered.contains("implements 1 · verifies 1/1"));
        assert!(rendered.contains(
            "    implements:\n      - FEAT-SEARCH-001#binding.implementation/target.search"
        ));
        assert!(rendered.contains(
            "    verifies:\n      - FEAT-SEARCH-001#binding.verification/target.contract [valid]"
        ));
        assert!(!rendered.contains("Criteria:"));
    }

    #[test]
    fn show_text_marks_missing_verification_as_unresolved() {
        let criterion: SpecAnchor = "REQ-SEARCH-004#criterion.advanced".parse().unwrap();
        let implementation: BoundTargetRef = "FEAT-SEARCH-001#binding.implementation/target.search"
            .parse()
            .unwrap();
        let verification: BoundTargetRef = "FEAT-SEARCH-001#binding.verification/target.contract"
            .parse()
            .unwrap();
        let result = ShowResult {
            id: SpecId::from("REQ-SEARCH-004"),
            kind: SpecKind::Requirement,
            title: "Advanced search".into(),
            summary: String::new(),
            description: String::new(),
            status: Some(ItemStatus::Implemented),
            source: "spec/requirements.yaml".into(),
            anchors: vec![criterion.clone()],
            criteria: vec![CriterionView {
                id: criterion,
                kind: mitase_spec_model::CriterionKind::Behavior,
                statement: "Search uses shared semantics.".into(),
                implementation_targets: vec![implementation],
                verification_targets: vec![verification],
                verification: CriterionVerification::Unverified,
            }],
            authored_relations: Vec::new(),
            derived_relations: Vec::new(),
            bindings: Vec::new(),
            verification_claims: Vec::new(),
        };

        let rendered = render_show_text(&result);

        assert!(rendered.contains("[unverified]"));
        assert!(rendered.contains("verifies 0/1"));
        assert!(rendered.contains(" [unresolved]"));
    }

    #[test]
    fn show_text_includes_catalog_only_verification_in_evidence_trace() {
        let criterion: SpecAnchor = "REQ-SEARCH-004#criterion.advanced".parse().unwrap();
        let verification: BoundTargetRef = "FEAT-SEARCH-001#binding.verification/target.contract"
            .parse()
            .unwrap();
        let result = ShowResult {
            id: SpecId::from("REQ-SEARCH-004"),
            kind: SpecKind::Requirement,
            title: "Advanced search".into(),
            summary: String::new(),
            description: String::new(),
            status: Some(ItemStatus::Planned),
            source: "spec/requirements.yaml".into(),
            anchors: vec![criterion.clone()],
            criteria: vec![CriterionView {
                id: criterion.clone(),
                kind: mitase_spec_model::CriterionKind::Behavior,
                statement: "Search uses shared semantics.".into(),
                implementation_targets: Vec::new(),
                verification_targets: Vec::new(),
                verification: CriterionVerification::Unverified,
            }],
            authored_relations: Vec::new(),
            derived_relations: Vec::new(),
            bindings: Vec::new(),
            verification_claims: vec![VerificationView {
                verification: verification.clone(),
                criterion: criterion.clone(),
                covers: Vec::new(),
                runner: "cargo-test".into(),
                assessment: VerificationAssessment {
                    status: VerificationAssessmentStatus::CatalogOnly,
                    verification,
                    criterion,
                    covers: Vec::new(),
                    reason: Some(VerificationAssessmentReason::CatalogPlanned),
                },
            }],
        };

        let rendered = render_show_text(&result);

        assert!(rendered.contains("verifies 0/1"));
        assert!(rendered.contains(
            "    verifies:\n      - FEAT-SEARCH-001#binding.verification/target.contract [catalog-only]"
        ));
    }
}
