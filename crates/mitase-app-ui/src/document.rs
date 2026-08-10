use mitase_workbench_server::WorkspaceProjection;

pub(crate) fn render(projection: &WorkspaceProjection) -> String {
    let shell = super::shell::render(include_str!("../assets/workbench.html"), projection);
    super::pages::attach_projection(shell, projection)
}
