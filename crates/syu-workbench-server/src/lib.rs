#![forbid(unsafe_code)]
use anyhow::Result;
use serde::Serialize;
use syu_diagnostics::ValidationResult;
use syu_planner::plan;
use syu_validation::{ValidationContext, validate};
use syu_work_model::{WorkPlan, WorkRequest};
use syu_workspace::SpecWorkspace;

#[derive(Debug, Clone, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceProjection {
    pub plan: Option<WorkPlan>,
    pub validation: ValidationResult,
}
pub fn project(
    workspace: &SpecWorkspace,
    request: Option<&WorkRequest>,
    revision: &str,
) -> Result<WorkspaceProjection> {
    let index = workspace.index()?;
    let plan = request
        .map(|r| plan(r, workspace, &index, revision))
        .transpose()?;
    let validation = validate(&ValidationContext {
        config: &workspace.config,
        workspace,
        index: &index,
        changed_paths: None,
        work_plan: plan.as_ref(),
        selected_slice: None,
        preset: workspace.config.validation.preset,
        revision,
    });
    Ok(WorkspaceProjection { plan, validation })
}
