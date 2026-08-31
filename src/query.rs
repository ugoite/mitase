use anyhow::{Result, bail};
use clap::ValueEnum;
use mitase_spec_model::{
    ArtifactBinding, ArtifactTargetLifecycle, BindingRole, BoundTargetRef, ItemStatus,
    LocalAnchorKind, SpecAnchor, SpecDocument, SpecId, TargetClaim,
};
use mitase_validation::{
    VerificationAssessment, VerificationAssessmentReason, assess_verification_claim,
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
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ListItem {
    pub id: SpecId,
    pub kind: SpecKind,
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
    pub authored_relations: Vec<RelationView>,
    pub derived_relations: Vec<RelationView>,
    pub bindings: Vec<BindingView>,
    pub verification_claims: Vec<VerificationView>,
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
) -> ListResult {
    let items = item_records(workspace, index)
        .into_iter()
        .filter(|item| kind.is_none_or(|expected| expected == item.kind))
        .filter(|item| status.is_none_or(|expected| expected.matches(item.status)))
        .map(|item| ListItem {
            id: item.id,
            kind: item.kind,
            title: item.title,
            status: item.status,
            source: item.source,
        })
        .collect();
    ListResult { items }
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
        authored_relations,
        derived_relations,
        bindings,
        verification_claims,
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
            "{} {} [{}] — {} ({})",
            item.kind.label(),
            item.id,
            status,
            item.title,
            item.source
        );
    }
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

fn item_records(workspace: &SpecWorkspace, index: &SpecIndex) -> Vec<ItemRecord> {
    let mut records = Vec::new();
    for loaded in &workspace.documents {
        match &loaded.document {
            SpecDocument::Philosophies { philosophies, .. } => {
                for item in philosophies {
                    records.push(item_record(
                        workspace,
                        index,
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
            SpecDocument::Policies { policies, .. } => {
                for item in policies {
                    records.push(item_record(
                        workspace,
                        index,
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
            SpecDocument::Requirements { requirements, .. } => {
                for item in requirements {
                    records.push(item_record(
                        workspace,
                        index,
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
            SpecDocument::Features { features, .. } => {
                for item in features {
                    records.push(item_record(
                        workspace,
                        index,
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
        title,
        summary,
        description,
        status,
        source: relative_path(workspace, source),
        bindings: binding_views,
    }
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
