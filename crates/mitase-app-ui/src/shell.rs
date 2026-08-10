use mitase_workbench_server::WorkspaceProjection;

pub(crate) fn render(template: &str, projection: &WorkspaceProjection) -> String {
    template
        .replace(
            "ugoite / mitase",
            &super::components::escape(&projection.snapshot.root),
        )
        .replace(
            "issue-762 · 8954b70",
            &format!(
                "{} · {}",
                super::components::escape(
                    &projection
                        .snapshot
                        .revision
                        .chars()
                        .take(9)
                        .collect::<String>()
                ),
                super::components::escape(
                    &projection
                        .snapshot
                        .fingerprint
                        .chars()
                        .take(9)
                        .collect::<String>()
                )
            ),
        )
}
