use anyhow::{Result, bail};
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
    ScopeOutcome, TaskTestSelectionPlan, WorkIntent, WorkKind, WorkMode, WorkOperation, WorkPlan,
    WorkSeed, WorkSurface,
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

/// Build a UI-independent, typed Work plan from a natural-language request.
///
/// Explicit axes take precedence over inference. Related spec items are expanded through the
/// existing workspace graph and annotated according to the selected WorkKind profile.
pub fn plan_request_work_with_constraints(
    workspace: impl AsRef<Path>,
    request: &RequestArtifact,
    explicit_kind: Option<WorkKind>,
    explicit_operation: Option<WorkOperation>,
    explicit_mode: Option<WorkMode>,
    constraints: syu_task_model::WorkConstraints,
) -> Result<WorkPlan> {
    use syu_task_model::{SourceRole, WorkGraphNode, WorkPlanningInput, plan_work};

    let workspace = syu::workspace::load_workspace(workspace.as_ref())?;
    let mut seeds = request
        .explicit_ids()
        .into_iter()
        .map(|id| WorkSeed {
            surface: surface_for_id(&id),
            id,
            source_role: SourceRole::Seed,
        })
        .collect::<Vec<_>>();
    let scope = syu::command::task::scope_request(&workspace, request)?;
    let search_candidates = scope
        .classification
        .related_items
        .iter()
        .filter_map(|item| {
            surface_for_label(&item.kind).map(|surface| syu_task_model::WorkCandidate {
                id: item.id.clone(),
                surface,
                score: 20,
                match_reasons: vec!["scoped related item".to_string()],
            })
        })
        .collect::<Vec<_>>();
    let mut diagnostics = Vec::new();
    if seeds.is_empty() && search_candidates.len() == 1 {
        let candidate = &search_candidates[0];
        seeds.push(WorkSeed {
            id: candidate.id.clone(),
            surface: candidate.surface,
            source_role: SourceRole::Inferred,
        });
    } else if seeds.is_empty() && !search_candidates.is_empty() {
        diagnostics.push(syu_task_model::WorkDiagnostic {
            rule: "WORK_AMBIGUOUS_SEED".to_string(),
            subject: syu_task_model::work_request_text(request),
            message: "request matched multiple candidate Items without a confident unique winner"
                .to_string(),
        });
    }
    let nodes = workspace
        .philosophies
        .iter()
        .map(|item| WorkGraphNode {
            id: item.id.clone(),
            surface: WorkSurface::Philosophy,
            document_path: None,
            status: None,
            linked_ids: item.linked_policies.clone(),
            implementations: Vec::new(),
            tests: Vec::new(),
        })
        .chain(workspace.policies.iter().map(|item| WorkGraphNode {
            id: item.id.clone(),
            surface: WorkSurface::Policy,
            document_path: None,
            status: None,
            linked_ids: item.linked_requirements.clone(),
            implementations: Vec::new(),
            tests: Vec::new(),
        }))
        .chain(workspace.requirements.iter().map(|item| {
            WorkGraphNode {
                id: item.id.clone(),
                surface: WorkSurface::Requirement,
                document_path: None,
                status: Some(item.status.clone()),
                linked_ids: item
                    .linked_policies
                    .iter()
                    .chain(&item.linked_features)
                    .cloned()
                    .collect(),
                implementations: Vec::new(),
                tests: trace_targets(&item.tests),
            }
        }))
        .chain(workspace.features.iter().map(|item| WorkGraphNode {
            id: item.id.clone(),
            surface: WorkSurface::Feature,
            document_path: None,
            status: Some(item.status.clone()),
            linked_ids: item.linked_requirements.clone(),
            implementations: trace_targets(&item.implementations),
            tests: Vec::new(),
        }))
        .collect();
    Ok(plan_work(WorkPlanningInput {
        request: syu_task_model::work_request_text(request),
        explicit_kind,
        explicit_operation,
        explicit_mode,
        seeds,
        search_candidates,
        nodes,
        constraints,
        diagnostics,
        ..Default::default()
    }))
}

pub fn plan_request_work(
    workspace: impl AsRef<Path>,
    request: &RequestArtifact,
    explicit_kind: Option<WorkKind>,
    explicit_operation: Option<WorkOperation>,
    explicit_mode: Option<WorkMode>,
) -> Result<WorkPlan> {
    plan_request_work_with_constraints(
        workspace,
        request,
        explicit_kind,
        explicit_operation,
        explicit_mode,
        Default::default(),
    )
}

/// Plan from an Item seed through the same shared engine used for requests.
pub fn plan_item_work(
    workspace: impl AsRef<Path>,
    item_id: &str,
    explicit_kind: Option<WorkKind>,
    explicit_operation: Option<WorkOperation>,
    explicit_mode: Option<WorkMode>,
) -> Result<WorkPlan> {
    plan_request_work(
        workspace,
        &RequestArtifact {
            version: 1,
            request: format!("Plan work for {item_id}"),
            context: syu_task_model::RequestArtifactContext {
                linked_ids: vec![item_id.to_string()],
                ..Default::default()
            },
        },
        explicit_kind,
        explicit_operation,
        explicit_mode,
    )
}

fn surface_for_id(id: &str) -> WorkSurface {
    if id.starts_with("PHIL-") {
        WorkSurface::Philosophy
    } else if id.starts_with("POL-") {
        WorkSurface::Policy
    } else if id.starts_with("FEAT-") {
        WorkSurface::Feature
    } else {
        WorkSurface::Requirement
    }
}

fn surface_for_label(label: &str) -> Option<WorkSurface> {
    match label {
        "philosophy" => Some(WorkSurface::Philosophy),
        "policy" => Some(WorkSurface::Policy),
        "requirement" => Some(WorkSurface::Requirement),
        "feature" => Some(WorkSurface::Feature),
        _ => None,
    }
}

fn trace_targets(
    map: &BTreeMap<String, Vec<syu_domain::TraceReference>>,
) -> Vec<syu_task_model::TraceTarget> {
    map.iter()
        .flat_map(|(language, references)| {
            references
                .iter()
                .map(move |reference| syu_task_model::TraceTarget {
                    language: language.clone(),
                    file: reference.file.display().to_string(),
                    symbols: reference.symbols.clone(),
                })
        })
        .collect()
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
    let query = query.trim();
    if query.is_empty() {
        bail!("search query must not be empty or whitespace");
    }
    let normalized_query = query.to_lowercase();
    let results = match kind {
        Some(LookupKind::Philosophy) => workspace
            .philosophies
            .iter()
            .filter(|item| {
                matches_search(
                    &normalized_query,
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
                    &normalized_query,
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
                    &normalized_query,
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
                    &normalized_query,
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
                            &normalized_query,
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
                            &normalized_query,
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
                            &normalized_query,
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
                            &normalized_query,
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

#[cfg(test)]
mod tests {
    use super::{
        LookupKind, RequestArtifact, WorkKind, WorkMode, WorkOperation, plan_item_work,
        plan_request_work, search_items,
    };
    use std::path::PathBuf;
    use syu_task_model::{RequestArtifactContext, SourceRole};

    fn fixture_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/workspaces")
            .join(name)
    }

    #[test]
    fn search_items_preserves_trimmed_query_and_matches_case_insensitively() {
        let report = search_items(fixture_path("passing"), "  Trace  ", None)
            .expect("search should succeed");

        assert_eq!(report.query, "Trace");
        assert!(
            report
                .results
                .iter()
                .any(|item| item.id == "FEAT-TRACE-002" && item.kind == "feature")
        );
    }

    #[test]
    fn search_items_rejects_blank_queries() {
        let error = search_items(fixture_path("passing"), "   ", Some(LookupKind::Feature))
            .expect_err("blank query should fail");

        assert!(
            error
                .to_string()
                .contains("search query must not be empty or whitespace")
        );
    }

    #[test]
    fn request_planner_expands_item_seed_through_the_spec_graph() {
        let request = RequestArtifact {
            version: 1,
            request: "Implement FEAT-TRACE-002".to_string(),
            context: RequestArtifactContext {
                linked_ids: vec!["FEAT-TRACE-002".to_string()],
                ..RequestArtifactContext::default()
            },
        };

        let plan = plan_request_work(fixture_path("passing"), &request, None, None, None)
            .expect("work plan");

        assert_eq!(plan.intent.kind, WorkKind::Deliver);
        assert_eq!(plan.intent.operation, WorkOperation::Modify);
        assert!(
            plan.impact.items.iter().any(|item| {
                item.id == "FEAT-TRACE-002" && item.source_role == SourceRole::Seed
            })
        );
        assert!(
            plan.impact
                .items
                .iter()
                .any(|item| item.id.starts_with("REQ-"))
        );
        assert!(
            !plan
                .impact
                .items
                .iter()
                .any(|item| item.id == "FEAT-TRACE-001")
        );
        assert!(plan.verification.cargo_test_fallback);
    }

    #[test]
    fn review_override_produces_evidence_only_plan() {
        let request = RequestArtifact {
            version: 1,
            request: "Inspect FEAT-TRACE-002".to_string(),
            context: RequestArtifactContext {
                linked_ids: vec!["FEAT-TRACE-002".to_string()],
                ..RequestArtifactContext::default()
            },
        };

        let plan = plan_request_work(
            fixture_path("passing"),
            &request,
            Some(WorkKind::Review),
            None,
            Some(WorkMode::ReviewOnly),
        )
        .expect("review plan");

        assert!(plan.mutations.is_empty());
        assert!(plan.verification.mutation_forbidden);
        assert!(!plan.verification.cargo_test_fallback);
    }

    #[test]
    fn item_and_request_entry_points_have_planner_parity() {
        let request = RequestArtifact {
            version: 1,
            request: "Plan work for FEAT-TRACE-002".to_string(),
            context: RequestArtifactContext {
                linked_ids: vec!["FEAT-TRACE-002".to_string()],
                ..RequestArtifactContext::default()
            },
        };
        let request_plan = plan_request_work(
            fixture_path("passing"),
            &request,
            Some(WorkKind::Deliver),
            Some(WorkOperation::Modify),
            None,
        )
        .expect("request plan");
        let item_plan = plan_item_work(
            fixture_path("passing"),
            "FEAT-TRACE-002",
            Some(WorkKind::Deliver),
            Some(WorkOperation::Modify),
            None,
        )
        .expect("item plan");
        assert_eq!(request_plan, item_plan);
    }
}
