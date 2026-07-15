use syu_workbench_server::WorkspaceProjection;

pub(crate) fn attach_projection(mut html: String, projection: &WorkspaceProjection) -> String {
    let json = serde_json::to_string(projection)
        .unwrap_or_else(|_| "{\"error\":\"projection serialization failed\"}".into())
        .replace('<', "\\u003c");
    let state = format!("<script type=\"application/json\" id=\"syu-projection\">{json}</script>");
    html = html.replace(
        "<script type=\"application/json\" id=\"syu-projection\"></script>",
        &state,
    );
    html
}
