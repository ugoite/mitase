use syu_app_ui::WorkbenchView;
use syu_workbench_server::project;
use syu_workspace::SpecWorkspace;

#[test]
fn workbench_rendered_dom_uses_projection_driven_placeholders() {
    let workspace = SpecWorkspace::load("fixtures/v1/valid-web-app").expect("workspace");
    let projection = project(&workspace, None, "test-revision").expect("projection");
    let html = WorkbenchView::new(&projection).render_html();

    for token in [
        "REQ-WORKBENCH",
        "SLICE-01",
        "PLAN-WORKBENCH",
        "UI-VISUAL-CONTRACT",
        "No issues found",
        "just now",
    ] {
        assert!(!html.contains(token), "static demo content leaked: {token}");
    }

    for marker in [
        "data-work-overview",
        "data-work-slices-rail",
        "data-work-context-rail",
        "data-work-validation-rail",
        "data-items-rail",
        "data-diagnostic-result",
        "data-settings-layer-panel=\"application\"",
        "data-settings-layer-panel=\"workspace\"",
    ] {
        assert!(html.contains(marker), "missing projection marker: {marker}");
    }
}
