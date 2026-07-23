use syu_app_ui::WorkbenchView;
use syu_workbench_server::project;
use syu_workspace::SpecWorkspace;

mod support;

#[test]
fn workbench_rendered_dom_uses_projection_driven_placeholders() {
    let fixture = support::isolated_fixture("valid-web-app");
    let workspace = SpecWorkspace::load(fixture.path()).expect("workspace");
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
        "data-specifications-rail",
        "data-diagnostic-result",
        "data-settings-layer-panel=\"application\"",
        "data-settings-layer-panel=\"workspace\"",
        "data-tab=\"all\"",
        "data-diagnostics-filters",
        "data-scope-mode-button=\"branch\"",
    ] {
        assert!(html.contains(marker), "missing projection marker: {marker}");
    }

    assert!(!html.contains("data-settings-page=\"yaml\""));
}
