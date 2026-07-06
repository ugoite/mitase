#![forbid(unsafe_code)]
use anyhow::Result;
use serde::Serialize;
use syu_diagnostics::ValidationResult;
use syu_planner::plan;
use syu_spec_model::LocalAnchorKind;
use syu_validation::{PlanValidationMode, ValidationContext, validate};
use syu_work_model::{WorkPlan, WorkRequest};
use syu_workspace::SpecWorkspace;

#[derive(Debug, Clone, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceProjection {
    pub workspace: WorkspaceSummary,
    pub items: Vec<ItemSummary>,
    pub plan: Option<WorkPlan>,
    pub validation: ValidationResult,
}

#[derive(Debug, Clone, Serialize)]
pub struct WorkspaceSummary {
    pub root: String,
    pub revision: String,
    pub fingerprint: String,
    pub config_schema: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ItemSummary {
    pub id: String,
    pub path: String,
    pub principles: usize,
    pub rules: usize,
    pub criteria: usize,
    pub bindings: usize,
    pub contracts: usize,
}
pub fn project(
    workspace: &SpecWorkspace,
    request: Option<&WorkRequest>,
    revision: &str,
) -> Result<WorkspaceProjection> {
    let index = workspace.index()?;
    let items = index
        .item_paths
        .iter()
        .map(|(id, path)| {
            let anchors = index.item_anchors.get(id).cloned().unwrap_or_default();
            let count = |kind| anchors.iter().filter(|anchor| anchor.kind == kind).count();
            ItemSummary {
                id: id.to_string(),
                path: path
                    .strip_prefix(&workspace.root)
                    .unwrap_or(path)
                    .display()
                    .to_string(),
                principles: count(LocalAnchorKind::Principle),
                rules: count(LocalAnchorKind::Rule),
                criteria: count(LocalAnchorKind::Criterion),
                bindings: count(LocalAnchorKind::Binding),
                contracts: count(LocalAnchorKind::Contract),
            }
        })
        .collect();
    let plan = request
        .map(|r| plan(r, workspace, &index, revision))
        .transpose()?;
    let validation = validate(&ValidationContext {
        config: &workspace.config,
        workspace,
        index: &index,
        changed_files: None,
        reported_changed_files: None,
        work_plan: plan.as_ref(),
        selected_slice: None,
        plan_mode: PlanValidationMode::PreState,
        preset: workspace.config.validation.preset,
        revision: Some(revision),
        change_base_revision: None,
    });
    Ok(WorkspaceProjection {
        workspace: WorkspaceSummary {
            root: workspace.root.display().to_string(),
            revision: revision.to_string(),
            fingerprint: workspace.fingerprint(),
            config_schema: workspace.config.schema.clone(),
        },
        items,
        plan,
        validation,
    })
}
