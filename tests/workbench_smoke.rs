use mitase_app_ui::{WORKBENCH_CSS, WORKBENCH_MAIN_JS, WorkbenchView};
use mitase_workbench_server::project;
use mitase_workspace::SpecWorkspace;

mod support;

#[test]
fn workbench_projection_is_server_owned_and_starts_not_run() {
    let fixture = support::isolated_fixture("valid-web-app");
    let workspace = SpecWorkspace::load(fixture.path()).unwrap();
    let projection = project(&workspace, None, "test-revision").unwrap();
    let projection = serde_json::to_value(projection).unwrap();
    assert_eq!(projection["diagnostics"]["validation"]["state"], "not_run");
    assert!(projection["specifications"]["specifications"].is_array());
    assert!(
        projection["config"].is_null(),
        "raw config must not enter browser DTO"
    );
}

#[test]
fn rendered_workbench_uses_external_module_assets_and_specifications_route() {
    let fixture = support::isolated_fixture("valid-web-app");
    let workspace = SpecWorkspace::load(fixture.path()).unwrap();
    let projection = project(&workspace, None, "test-revision").unwrap();
    let html = WorkbenchView::new(&projection).render_html();
    assert!(html.contains("type=\"module\" src=\"/assets/js/main.js\""));
    assert!(!html.contains("/assets/projection.js"));
    assert!(html.contains("class=\"workspace-icon\""));
    assert!(!html.contains("data-workspace-branch"));
    assert!(html.contains("data-page=\"specifications\""));
    assert!(!html.contains("data-page=\"items\""));
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
}

#[test]
fn browser_modules_render_dtos_without_model_inference() {
    for banned in [
        "selector.names",
        "rawProjection",
        "binding-level",
        "mitase/config",
    ] {
        assert!(
            !WORKBENCH_MAIN_JS.contains(banned),
            "legacy browser semantic leaked: {banned}"
        );
    }
    for marker in [
        "./api.js",
        "./state.js",
        "./router.js",
        "./pages/work.js",
        "./pages/specifications.js",
        "./pages/readiness.js",
        "./pages/diagnostics.js",
    ] {
        assert!(
            WORKBENCH_MAIN_JS.contains(marker),
            "missing module: {marker}"
        );
    }
}

#[test]
fn workbench_tabs_are_keyboard_navigable() {
    assert!(WORKBENCH_CSS.contains("overflow-y: hidden"));
    assert!(WORKBENCH_CSS.contains("max-height: 51px"));
    let html = include_str!("../crates/mitase-app-ui/assets/workbench.html");
    assert!(html.contains("data-tab-group=\"specifications\""));
    assert!(!html.contains("data-tab-group=\"items\""));
}
