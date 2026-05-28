use anyhow::Result;
use serde::Serialize;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

pub use syu::cli::LookupKind;
pub use syu::command::{
    doctor::DoctorReport,
    log::{HistoryRequest, HistoryResponse, HistoryScope},
    relate::{JsonRelateOutput, JsonRelateRangeOutput},
    task::{JsonTaskPlanOutput, JsonTaskPlanSourceEvidence},
    trace::{TraceLookupOutput, TraceRangeOutput},
};
pub use syu::model::CheckResult as ValidationReport;
pub use syu_task_model::{
    ClassificationOutcome, GoalPlanArtifact, GoalPlanCheckReport, RequestArtifact, ScaffoldPlan,
    ScopeOutcome, TaskTestSelectionPlan,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ItemSummary {
    pub kind: &'static str,
    pub id: String,
    pub title: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct BrowseSnapshot {
    pub workspace_root: String,
    pub spec_root: String,
    pub validation: ValidationReport,
    pub philosophies: Vec<ItemSummary>,
    pub policies: Vec<ItemSummary>,
    pub requirements: Vec<ItemSummary>,
    pub features: Vec<ItemSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ListReport {
    pub workspace_root: String,
    pub kind: Option<String>,
    pub items: Vec<ItemSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ShowReport {
    pub workspace_root: String,
    pub kind: String,
    pub id: String,
    pub title: String,
    pub details: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SearchReport {
    pub workspace_root: String,
    pub query: String,
    pub kind: Option<String>,
    pub results: Vec<ItemSummary>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AuditReport {
    pub workspace_root: String,
    pub validation: ValidationReport,
    pub counts: BTreeMap<String, usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ExplainReport {
    pub workspace_root: String,
    pub selector: String,
    pub matches: Vec<ItemSummary>,
    pub notes: Vec<String>,
}

pub fn validate_workspace(workspace: impl AsRef<Path>) -> ValidationReport {
    syu::command::check::collect_check_result(workspace.as_ref())
}

pub fn doctor_workspace(workspace: impl AsRef<Path>) -> Result<DoctorReport> {
    syu::command::doctor::build_doctor_report(workspace.as_ref())
}

pub fn classify_request(
    workspace: impl AsRef<Path>,
    request: &RequestArtifact,
) -> Result<ClassificationOutcome> {
    let workspace = syu::workspace::load_workspace(workspace.as_ref())?;
    syu::command::task::classify_request(&workspace, request)
}

pub fn scope_request(
    workspace: impl AsRef<Path>,
    request: &RequestArtifact,
) -> Result<ScopeOutcome> {
    let workspace = syu::workspace::load_workspace(workspace.as_ref())?;
    syu::command::task::scope_request(&workspace, request)
}

pub fn scaffold_request(
    workspace: impl AsRef<Path>,
    request: &RequestArtifact,
) -> Result<ScaffoldPlan> {
    let workspace = syu::workspace::load_workspace(workspace.as_ref())?;
    let classification = classify_request(&workspace.root, request)?;
    let explicit_ids = request.explicit_ids();
    syu::command::task::build_scaffold_plan(&workspace, &classification, &explicit_ids)
}

pub fn generate_goal_plan<P: AsRef<Path>>(
    workspace: impl AsRef<Path>,
    outcome: &ScopeOutcome,
    explicit_ids: &[String],
    request_path: impl AsRef<Path>,
    output_path: Option<P>,
) -> Result<JsonTaskPlanOutput> {
    let workspace = syu::workspace::load_workspace(workspace.as_ref())?;
    syu::command::task::build_goal_plan(
        &workspace,
        outcome,
        explicit_ids,
        request_path.as_ref(),
        output_path.as_ref().map(AsRef::as_ref),
    )
}

pub fn infer_goal_plan_from_diff<P: AsRef<Path>>(
    workspace: impl AsRef<Path>,
    range: &str,
    changed_files: &[PathBuf],
    output_path: Option<P>,
) -> Result<JsonTaskPlanOutput> {
    let workspace = syu::workspace::load_workspace(workspace.as_ref())?;
    syu::command::task::build_diff_inferred_goal_plan(
        &workspace,
        range,
        changed_files,
        output_path.as_ref().map(AsRef::as_ref),
    )
}

pub fn select_goal_tests(
    workspace: impl AsRef<Path>,
    artifact: &GoalPlanArtifact,
) -> Result<TaskTestSelectionPlan> {
    let workspace = syu::workspace::load_workspace(workspace.as_ref())?;
    syu::command::task::build_task_test_selection(&workspace, artifact)
}

pub fn check_goal_plan(
    workspace: impl AsRef<Path>,
    artifact: &GoalPlanArtifact,
    range: &str,
) -> Result<GoalPlanCheckReport> {
    let workspace = syu::workspace::load_workspace(workspace.as_ref())?;
    syu::command::task::check_goal_plan(&workspace, artifact, range)
}

pub fn history_for_item(request: HistoryRequest<'_>) -> Result<HistoryResponse> {
    syu::command::log::build_history_response(request)
}

pub fn relate_selector(workspace: impl AsRef<Path>, selector: &str) -> Result<JsonRelateOutput> {
    let workspace = syu::workspace::load_workspace(workspace.as_ref())?;
    syu::command::relate::build_relation_report(&workspace, selector)
}

pub fn relate_range(workspace: impl AsRef<Path>, range: &str) -> Result<JsonRelateRangeOutput> {
    let workspace = syu::workspace::load_workspace(workspace.as_ref())?;
    syu::command::relate::build_relation_range_report(&workspace, range)
}

pub fn trace_selector(
    workspace: impl AsRef<Path>,
    file: impl AsRef<Path>,
    symbol: Option<&str>,
) -> Result<TraceLookupOutput> {
    let workspace = syu::workspace::load_workspace(workspace.as_ref())?;
    syu::command::trace::trace_selector(&workspace, file.as_ref(), symbol)
}

pub fn trace_range(
    workspace: impl AsRef<Path>,
    range: &str,
    strict: bool,
    allowed_ids: &[String],
) -> Result<TraceRangeOutput> {
    let workspace = syu::workspace::load_workspace(workspace.as_ref())?;
    syu::command::trace::trace_range(&workspace, range, strict, allowed_ids)
}

pub fn browse_workspace(workspace: impl AsRef<Path>) -> Result<BrowseSnapshot> {
    let workspace = syu::workspace::load_workspace(workspace.as_ref())?;
    let validation = validate_workspace(&workspace.root);
    Ok(BrowseSnapshot {
        workspace_root: workspace.root.display().to_string(),
        spec_root: workspace.spec_root.display().to_string(),
        validation,
        philosophies: workspace
            .philosophies
            .iter()
            .map(|item| ItemSummary {
                kind: "philosophy",
                id: item.id.clone(),
                title: item.title.clone(),
            })
            .collect(),
        policies: workspace
            .policies
            .iter()
            .map(|item| ItemSummary {
                kind: "policy",
                id: item.id.clone(),
                title: item.title.clone(),
            })
            .collect(),
        requirements: workspace
            .requirements
            .iter()
            .map(|item| ItemSummary {
                kind: "requirement",
                id: item.id.clone(),
                title: item.title.clone(),
            })
            .collect(),
        features: workspace
            .features
            .iter()
            .map(|item| ItemSummary {
                kind: "feature",
                id: item.id.clone(),
                title: item.title.clone(),
            })
            .collect(),
    })
}

pub fn list_items(workspace: impl AsRef<Path>, kind: Option<LookupKind>) -> Result<ListReport> {
    let workspace = syu::workspace::load_workspace(workspace.as_ref())?;
    Ok(ListReport {
        workspace_root: workspace.root.display().to_string(),
        kind: kind.map(|value| value.label().to_string()),
        items: collect_items(&workspace, kind),
    })
}

pub fn show_item(workspace: impl AsRef<Path>, kind: LookupKind, id: &str) -> Result<ShowReport> {
    let workspace = syu::workspace::load_workspace(workspace.as_ref())?;
    let item = find_item(&workspace, kind, id)
        .ok_or_else(|| anyhow::anyhow!("`{id}` was not found in the workspace"))?;
    Ok(ShowReport {
        workspace_root: workspace.root.display().to_string(),
        kind: kind.label().to_string(),
        id: id.to_string(),
        title: item.title.clone(),
        details: item.details,
    })
}

pub fn search_items(
    workspace: impl AsRef<Path>,
    query: &str,
    kind: Option<LookupKind>,
) -> Result<SearchReport> {
    let workspace = syu::workspace::load_workspace(workspace.as_ref())?;
    let query = query.trim().to_lowercase();
    let results = match kind {
        Some(LookupKind::Philosophy) => workspace
            .philosophies
            .iter()
            .filter(|item| {
                matches_search(
                    &query,
                    &[
                        item.id.as_str(),
                        item.title.as_str(),
                        item.product_design_principle.as_str(),
                        item.coding_guideline.as_str(),
                    ],
                )
            })
            .map(|item| ItemSummary {
                kind: "philosophy",
                id: item.id.clone(),
                title: item.title.clone(),
            })
            .collect(),
        Some(LookupKind::Policy) => workspace
            .policies
            .iter()
            .filter(|item| {
                matches_search(
                    &query,
                    &[
                        item.id.as_str(),
                        item.title.as_str(),
                        item.summary.as_str(),
                        item.description.as_str(),
                    ],
                )
            })
            .map(|item| ItemSummary {
                kind: "policy",
                id: item.id.clone(),
                title: item.title.clone(),
            })
            .collect(),
        Some(LookupKind::Requirement) => workspace
            .requirements
            .iter()
            .filter(|item| {
                matches_search(
                    &query,
                    &[
                        item.id.as_str(),
                        item.title.as_str(),
                        item.description.as_str(),
                    ],
                )
            })
            .map(|item| ItemSummary {
                kind: "requirement",
                id: item.id.clone(),
                title: item.title.clone(),
            })
            .collect(),
        Some(LookupKind::Feature) => workspace
            .features
            .iter()
            .filter(|item| {
                matches_search(
                    &query,
                    &[item.id.as_str(), item.title.as_str(), item.summary.as_str()],
                )
            })
            .map(|item| ItemSummary {
                kind: "feature",
                id: item.id.clone(),
                title: item.title.clone(),
            })
            .collect(),
        None => {
            let mut results = Vec::new();
            results.extend(
                workspace
                    .philosophies
                    .iter()
                    .filter(|item| {
                        matches_search(
                            &query,
                            &[
                                item.id.as_str(),
                                item.title.as_str(),
                                item.product_design_principle.as_str(),
                                item.coding_guideline.as_str(),
                            ],
                        )
                    })
                    .map(|item| ItemSummary {
                        kind: "philosophy",
                        id: item.id.clone(),
                        title: item.title.clone(),
                    }),
            );
            results.extend(
                workspace
                    .policies
                    .iter()
                    .filter(|item| {
                        matches_search(
                            &query,
                            &[
                                item.id.as_str(),
                                item.title.as_str(),
                                item.summary.as_str(),
                                item.description.as_str(),
                            ],
                        )
                    })
                    .map(|item| ItemSummary {
                        kind: "policy",
                        id: item.id.clone(),
                        title: item.title.clone(),
                    }),
            );
            results.extend(
                workspace
                    .features
                    .iter()
                    .filter(|item| {
                        matches_search(
                            &query,
                            &[item.id.as_str(), item.title.as_str(), item.summary.as_str()],
                        )
                    })
                    .map(|item| ItemSummary {
                        kind: "feature",
                        id: item.id.clone(),
                        title: item.title.clone(),
                    }),
            );
            results.extend(
                workspace
                    .requirements
                    .iter()
                    .filter(|item| {
                        matches_search(
                            &query,
                            &[
                                item.id.as_str(),
                                item.title.as_str(),
                                item.description.as_str(),
                            ],
                        )
                    })
                    .map(|item| ItemSummary {
                        kind: "requirement",
                        id: item.id.clone(),
                        title: item.title.clone(),
                    }),
            );
            results
        }
    };

    Ok(SearchReport {
        workspace_root: workspace.root.display().to_string(),
        query: query.to_string(),
        kind: kind.map(|value| value.label().to_string()),
        results,
    })
}

pub fn audit_workspace(workspace: impl AsRef<Path>) -> Result<AuditReport> {
    let workspace = syu::workspace::load_workspace(workspace.as_ref())?;
    let validation = validate_workspace(&workspace.root);
    let mut counts = BTreeMap::new();
    counts.insert("philosophies".to_string(), workspace.philosophies.len());
    counts.insert("policies".to_string(), workspace.policies.len());
    counts.insert("requirements".to_string(), workspace.requirements.len());
    counts.insert("features".to_string(), workspace.features.len());

    Ok(AuditReport {
        workspace_root: workspace.root.display().to_string(),
        validation,
        counts,
    })
}

pub fn explain_selector(workspace: impl AsRef<Path>, selector: &str) -> Result<ExplainReport> {
    let workspace = syu::workspace::load_workspace(workspace.as_ref())?;
    let trimmed = selector.trim();
    let mut matches = Vec::new();
    let mut notes = Vec::new();

    if let Some(item) = find_any_item(&workspace, trimmed) {
        matches.push(item);
        notes.push("matched by exact id".to_string());
    } else {
        let search = search_items(&workspace.root, trimmed, None)?;
        matches.extend(search.results);
        notes.push("matched by id/title search".to_string());
    }

    Ok(ExplainReport {
        workspace_root: workspace.root.display().to_string(),
        selector: trimmed.to_string(),
        matches,
        notes,
    })
}

#[derive(Debug, Clone)]
struct ItemRecord {
    title: String,
    details: BTreeMap<String, String>,
}

fn collect_items(
    workspace: &syu::workspace::Workspace,
    kind: Option<LookupKind>,
) -> Vec<ItemSummary> {
    let mut items = Vec::new();
    if kind.is_none() || kind == Some(LookupKind::Philosophy) {
        items.extend(workspace.philosophies.iter().map(|item| ItemSummary {
            kind: "philosophy",
            id: item.id.clone(),
            title: item.title.clone(),
        }));
    }
    if kind.is_none() || kind == Some(LookupKind::Policy) {
        items.extend(workspace.policies.iter().map(|item| ItemSummary {
            kind: "policy",
            id: item.id.clone(),
            title: item.title.clone(),
        }));
    }
    if kind.is_none() || kind == Some(LookupKind::Requirement) {
        items.extend(workspace.requirements.iter().map(|item| ItemSummary {
            kind: "requirement",
            id: item.id.clone(),
            title: item.title.clone(),
        }));
    }
    if kind.is_none() || kind == Some(LookupKind::Feature) {
        items.extend(workspace.features.iter().map(|item| ItemSummary {
            kind: "feature",
            id: item.id.clone(),
            title: item.title.clone(),
        }));
    }
    items
}

fn find_any_item(workspace: &syu::workspace::Workspace, id: &str) -> Option<ItemSummary> {
    if let Some(item) = workspace.philosophies.iter().find(|item| item.id == id) {
        return Some(ItemSummary {
            kind: "philosophy",
            id: item.id.clone(),
            title: item.title.clone(),
        });
    }
    if let Some(item) = workspace.policies.iter().find(|item| item.id == id) {
        return Some(ItemSummary {
            kind: "policy",
            id: item.id.clone(),
            title: item.title.clone(),
        });
    }
    if let Some(item) = workspace.requirements.iter().find(|item| item.id == id) {
        return Some(ItemSummary {
            kind: "requirement",
            id: item.id.clone(),
            title: item.title.clone(),
        });
    }
    workspace
        .features
        .iter()
        .find(|item| item.id == id)
        .map(|item| ItemSummary {
            kind: "feature",
            id: item.id.clone(),
            title: item.title.clone(),
        })
}

fn matches_search(query: &str, fields: &[&str]) -> bool {
    fields
        .iter()
        .filter(|value| !value.is_empty())
        .any(|value| value.to_lowercase().contains(query))
}

fn find_item(
    workspace: &syu::workspace::Workspace,
    kind: LookupKind,
    id: &str,
) -> Option<ItemRecord> {
    match kind {
        LookupKind::Philosophy => workspace
            .philosophies
            .iter()
            .find(|item| item.id == id)
            .map(|item| ItemRecord {
                title: item.title.clone(),
                details: BTreeMap::from([
                    (
                        "product_design_principle".to_string(),
                        item.product_design_principle.clone(),
                    ),
                    (
                        "coding_guideline".to_string(),
                        item.coding_guideline.clone(),
                    ),
                ]),
            }),
        LookupKind::Policy => workspace
            .policies
            .iter()
            .find(|item| item.id == id)
            .map(|item| ItemRecord {
                title: item.title.clone(),
                details: BTreeMap::from([
                    ("summary".to_string(), item.summary.clone()),
                    ("description".to_string(), item.description.clone()),
                ]),
            }),
        LookupKind::Requirement => workspace
            .requirements
            .iter()
            .find(|item| item.id == id)
            .map(|item| ItemRecord {
                title: item.title.clone(),
                details: BTreeMap::from([
                    ("description".to_string(), item.description.clone()),
                    ("priority".to_string(), item.priority.clone()),
                    ("status".to_string(), item.status.clone()),
                ]),
            }),
        LookupKind::Feature => workspace
            .features
            .iter()
            .find(|item| item.id == id)
            .map(|item| ItemRecord {
                title: item.title.clone(),
                details: BTreeMap::from([
                    ("summary".to_string(), item.summary.clone()),
                    ("status".to_string(), item.status.clone()),
                ]),
            }),
    }
}
