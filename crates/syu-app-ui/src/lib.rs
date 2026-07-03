#![forbid(unsafe_code)]
use syu_diagnostics::Diagnostic;
use syu_work_model::{ExecutionSlice, PlannedTarget};
use syu_workbench_server::WorkspaceProjection;

/// Read-only UI view over the canonical server projection. The UI does not
/// parse specification YAML or infer ownership, contracts, or edit scope.
pub struct WorkbenchView<'a> {
    projection: &'a WorkspaceProjection,
}
impl<'a> WorkbenchView<'a> {
    pub fn new(projection: &'a WorkspaceProjection) -> Self {
        Self { projection }
    }
    pub fn slices(&self) -> impl Iterator<Item = &'a ExecutionSlice> {
        self.projection.plan.iter().flat_map(|p| p.slices.iter())
    }
    pub fn diagnostics(&self) -> impl Iterator<Item = &'a Diagnostic> {
        self.projection.validation.diagnostics.iter()
    }
    pub fn editable(slice: &'a ExecutionSlice) -> impl Iterator<Item = &'a PlannedTarget> {
        slice.editable_targets.iter()
    }
    pub fn verification(slice: &'a ExecutionSlice) -> impl Iterator<Item = &'a PlannedTarget> {
        slice.verification_targets.iter()
    }
    pub fn readonly(slice: &'a ExecutionSlice) -> impl Iterator<Item = &'a PlannedTarget> {
        slice.readonly_context.iter()
    }
}
