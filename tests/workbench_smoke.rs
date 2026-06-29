use dioxus::prelude::*;
use syu_app_ui::model::{
    SpecBrowserDocument, SpecBrowserItem, SpecBrowserModel, SpecBrowserSection,
};
use syu_app_ui::{AppShell, FocusIntent, Locale, PageSection, WorkbenchPage, WorkbenchUiState};
use syu_workbench::WorkbenchState;

fn render(
    ui: WorkbenchUiState,
    page: WorkbenchPage,
    section: Option<PageSection>,
    entity: Option<String>,
    focus: Option<FocusIntent>,
) -> String {
    dioxus_ssr::render_element(rsx! {
        AppShell { ui, active_page: page, section, entity, focus, sidebar_open: true }
    })
}

#[test]
fn sidebar_has_only_four_roles_in_required_order() {
    let html = render(
        WorkbenchUiState::from_state(WorkbenchState::default()),
        WorkbenchPage::Work,
        None,
        None,
        None,
    );
    let work = html.find(">Work<").unwrap();
    let scope = html.find(">Scope<").unwrap();
    let items = html.find(">Items<").unwrap();
    let diagnostics = html.find(">Diagnostics<").unwrap();
    assert!(work < scope && scope < items && items < diagnostics);
    assert!(!html.contains(">Settings<"));
    for legacy in [
        "Command palette</",
        "Goal plan</",
        "Request intake</",
        "Assignment</",
        "Spec graph</",
    ] {
        assert!(!html.contains(legacy));
    }
}

#[test]
fn work_is_human_readable_default_page() {
    let html = render(
        WorkbenchUiState::from_state(WorkbenchState::default()),
        WorkbenchPage::Work,
        None,
        None,
        None,
    );
    assert!(html.contains("Understand, assign, and verify implementation work"));
    assert!(html.contains("Brief"));
    assert!(html.contains("No work yet"));
}

#[test]
fn item_draft_uses_the_same_detail_canvas() {
    let mut ui = WorkbenchUiState::from_state(WorkbenchState::default());
    ui.spec_browser = Some(SpecBrowserModel {
        selected_item_id: Some("REQ-EXISTING-001".to_string()),
        sections: vec![SpecBrowserSection {
            label: "requirements".to_string(),
            documents: vec![SpecBrowserDocument {
                path: "requirements/core.yaml".to_string(),
                title: "Core".to_string(),
                folder_segments: vec![],
                items: vec![SpecBrowserItem {
                    kind: "requirements".to_string(),
                    id: "REQ-EXISTING-001".to_string(),
                    title: "Existing".to_string(),
                    summary: None,
                    description: None,
                    product_design_principle: None,
                    coding_guideline: None,
                    priority: None,
                    status: Some("active".to_string()),
                    linked_philosophies: vec![],
                    linked_policies: vec![],
                    linked_requirements: vec![],
                    linked_features: vec![],
                    tests: vec![],
                    implementations: vec![],
                }],
            }],
        }],
    });
    let html = render(
        ui,
        WorkbenchPage::Items,
        Some(PageSection::Requirement),
        Some("draft".to_string()),
        Some(FocusIntent::Create),
    );
    assert!(html.contains("Create in the same Detail Canvas"));
    assert!(html.contains("data-command-target=\"item-editor\""));
    assert!(html.contains("Preview changes"));
    assert!(!html.contains("name=\"cli\""));
}

#[test]
fn palette_history_targets_work_evidence() {
    let html = render(
        WorkbenchUiState::from_state(WorkbenchState::default()),
        WorkbenchPage::Work,
        None,
        None,
        None,
    );
    assert!(html.contains("Show history"));
    assert!(html.contains("page=work"));
    assert!(html.contains("section=evidence"));
    assert!(html.contains("focus=timeline"));
    assert!(!html.contains("page=commands"));
}

#[test]
fn diagnostics_and_settings_have_localized_japanese_copy() {
    let mut ui = WorkbenchUiState::from_state(WorkbenchState::default());
    ui.set_locale(Locale::Ja);
    let diagnostics = render(
        ui.clone(),
        WorkbenchPage::Diagnostics,
        None,
        None,
        Some(FocusIntent::DiagnosticsRun),
    );
    assert!(diagnostics.contains("診断項目を検索"));
    assert!(diagnostics.contains("aria-label"));
    assert!(diagnostics.contains("border-red-500"));
    let settings = render(
        ui,
        WorkbenchPage::Settings,
        Some(PageSection::SyuYaml),
        None,
        None,
    );
    assert!(settings.contains("構造化設定"));
    assert!(settings.contains("差分プレビュー"));
    assert!(settings.contains("既存コメントと未知のフィールドは保持"));
}
