use std::process::Command;
use syu_app_ui::{WORKBENCH_CSS, WORKBENCH_PROJECTION_JS, WorkbenchView};
use syu_spec_model::SpecDocument;
use syu_workbench_server::project;
use syu_workspace::SpecWorkspace;

#[test]
fn workbench_projection_exposes_explicit_run_state_and_exact_anchors() {
    let output = Command::new(env!("CARGO_BIN_EXE_syu"))
        .args([
            "workbench",
            "project",
            "--workspace",
            "fixtures/v1/valid-web-app",
            "--format",
            "json",
        ])
        .output()
        .expect("run workbench projection");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let projection: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(projection["validation"]["state"], "not_run");
    assert!(projection["validation"]["phases"].is_array());
    assert!(
        projection["items"]
            .as_array()
            .is_some_and(|items| items.iter().any(|item| item["anchors"]
                .as_array()
                .is_some_and(|anchors| !anchors.is_empty())))
    );
    assert_eq!(projection["requested_work"]["id"], "WORK-AUTH-FAILURE");
    assert!(projection["plan"].is_object());
}

#[test]
fn items_initial_tab_is_all_and_projection_lists_every_spec_item() {
    let workspace = SpecWorkspace::load("fixtures/v1/valid-web-app").unwrap();
    let projection = project(&workspace, None, "test").unwrap();
    let expected = workspace
        .documents
        .iter()
        .map(|doc| match &doc.document {
            SpecDocument::Philosophies { philosophies, .. } => philosophies.len(),
            SpecDocument::Policies { policies, .. } => policies.len(),
            SpecDocument::Requirements { requirements, .. } => requirements.len(),
            SpecDocument::Features { features, .. } => features.len(),
        })
        .sum::<usize>();
    assert_eq!(projection.items.len(), expected);
    let html = WorkbenchView::new(&projection).render_html();
    assert!(html.contains("data-tab=\"all\""));
}

#[test]
fn normal_ui_does_not_expose_raw_yaml_editors() {
    let html = include_str!("../crates/syu-app-ui/assets/workbench.html");
    assert!(!html.contains("data-settings-page=\"yaml\""));
    assert!(!html.contains("Raw YAML"));
    assert!(!WORKBENCH_PROJECTION_JS.contains("openItemEditor(path, id"));
    assert!(!WORKBENCH_PROJECTION_JS.contains("textarea code"));
}

#[test]
fn workbench_removes_inert_controls_and_uses_structured_baseline_inputs() {
    let html = include_str!("../crates/syu-app-ui/assets/workbench.html");
    assert!(!html.contains("quick-pill"));
    assert!(!html.contains("class=\"help\""));
    assert!(!html.contains("data-config-baseline "));
    assert!(html.contains("data-config-baseline-strategy"));
    assert!(html.contains("data-config-baseline-ref"));
    assert!(html.contains("active-request-label"));
    assert!(!html.contains(
        "data-work-plan-label data-i18n=\"common.request\">Request</span><span>⌄</span>"
    ));
}

#[test]
fn workbench_item_editor_covers_traceability_fields() {
    assert!(WORKBENCH_PROJECTION_JS.contains("bindingEditor"));
    assert!(WORKBENCH_PROJECTION_JS.contains("contractEditor"));
    for field in [
        "applies_to",
        "governed_by",
        "generated_from",
        "guarantees",
        "selector_kind",
    ] {
        assert!(WORKBENCH_PROJECTION_JS.contains(field));
    }
}

#[test]
fn tabs_never_scroll_vertically() {
    assert!(WORKBENCH_CSS.contains("overflow-y: hidden"));
    assert!(WORKBENCH_CSS.contains("max-height: 51px"));
}

#[test]
fn diagnostics_phase_tabs_are_real_phase_controls() {
    let html = include_str!("../crates/syu-app-ui/assets/workbench.html");
    for phase in ["all", "config", "graph", "targets", "scope", "plan"] {
        assert!(html.contains(&format!("data-diagnostic-phase=\"{phase}\"")));
    }
    assert!(WORKBENCH_PROJECTION_JS.contains("selectedDiagnosticPhase"));
}

#[test]
fn work_context_has_stateful_rail_selection() {
    assert!(WORKBENCH_PROJECTION_JS.contains("selectedContextGroup"));
    assert!(WORKBENCH_PROJECTION_JS.contains("selectedContextEntry"));
    assert!(WORKBENCH_PROJECTION_JS.contains("renderWorkContextDetail"));
}

#[test]
fn workbench_bootstrap_declares_selected_anchor_before_default_request() {
    let selected_anchor = WORKBENCH_PROJECTION_JS
        .find("let selectedAnchor = null;")
        .expect("selectedAnchor declaration");
    let default_request = WORKBENCH_PROJECTION_JS
        .find("let draftWorkRequest = requestedWork ? clone(requestedWork) : defaultWorkRequest();")
        .expect("default work request bootstrap");
    assert!(
        selected_anchor < default_request,
        "selectedAnchor must be initialized before defaultWorkRequest() can read it"
    );
}

#[test]
fn work_seed_button_opens_picker_without_global_selected_anchor_dependency() {
    let seed_block = WORKBENCH_PROJECTION_JS
        .split("one('[data-work-seed]')")
        .nth(1)
        .expect("seed action")
        .split("one('[data-work-plan]')")
        .next()
        .expect("seed action end");
    assert!(seed_block.contains("openWorkPage('overview')"));
    assert!(seed_block.contains("renderWorkRequestEditor"));
    assert!(!seed_block.contains("selectedAnchor"));
    assert!(!seed_block.contains("location.assign"));
}

#[test]
fn workbench_starts_work_from_user_facing_origins_and_hides_expert_fields() {
    assert!(WORKBENCH_PROJECTION_JS.contains("renderWorkStart"));
    for marker in [
        "work.start.branch",
        "work.start.specification",
        "work.start.describe",
        "advancedDetails(advanced)",
        "advancedDetails(metadata)",
    ] {
        assert!(WORKBENCH_PROJECTION_JS.contains(marker), "missing {marker}");
    }
    assert!(WORKBENCH_CSS.contains(".toolbar .btn.compact .btn-label { display: none; }"));
    assert!(
        WORKBENCH_CSS
            .contains(".advanced-editor:not([open]) > .advanced-editor-body { display: none; }")
    );
}

#[test]
fn diagnostics_primary_action_runs_validation_without_duplicate_result_action() {
    assert!(WORKBENCH_PROJECTION_JS.contains("validateButton?.addEventListener('click'"));
    assert!(WORKBENCH_PROJECTION_JS.contains("await runValidationFromCurrentControl()"));
    assert!(!WORKBENCH_PROJECTION_JS.contains("renderValidationStats"));
    let summary = WORKBENCH_PROJECTION_JS
        .split("function renderDiagnosticSummary")
        .nth(1)
        .expect("diagnostic summary")
        .split("function renderDiagnosticIssues")
        .next()
        .expect("diagnostic summary end");
    assert!(!summary.contains("runValidationFromCurrentControl"));
}
