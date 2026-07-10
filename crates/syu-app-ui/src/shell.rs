use syu_workbench_server::WorkspaceProjection;

pub(crate) fn render(template: &str, projection: &WorkspaceProjection) -> String {
    template
        .replace(
            "ugoite / syu",
            &super::components::escape(&projection.workspace.root),
        )
        .replace(
            "issue-762 · 8954b70",
            &format!(
                "{} · {}",
                super::components::escape(
                    &projection
                        .workspace
                        .revision
                        .chars()
                        .take(9)
                        .collect::<String>()
                ),
                super::components::escape(
                    &projection
                        .workspace
                        .fingerprint
                        .chars()
                        .take(9)
                        .collect::<String>()
                )
            ),
        )
}
