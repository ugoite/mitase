use super::*;
use syu_workbench::{WorkbenchActionId, WorkbenchActionRegistry};

#[test]
fn filters_actions_by_query() {
    let mut ui = WorkbenchUiState::from_state(WorkbenchState::default());
    ui.payload = WorkbenchApiPayload::new(WorkbenchState::default());
    ui.command_query = "history".to_string();

    let visible = ui.visible_actions();

    assert!(!visible.is_empty());
    assert!(
        visible
            .iter()
            .all(|entry| entry.action.id.label().contains("history")
                || entry.action.title.to_lowercase().contains("history"))
    );
}

#[test]
fn read_only_action_returns_placeholder_preview() {
    let ui = build_demo_state();

    let preview = ui.action_preview(WorkbenchActionId::HistoryShow).unwrap();

    assert!(preview.result_summary.contains("Preview opened"));
    assert_eq!(preview.evidence_summary, "Ready to review");
}

#[test]
fn registry_loaded_from_server_payload() {
    let state = WorkbenchState::default();
    let payload = WorkbenchApiPayload::new(state);
    assert_eq!(
        payload.actions.len(),
        WorkbenchActionRegistry::standard().actions().len()
    );
}

#[test]
fn cli_catalog_exposes_top_level_and_task_commands() {
    let ids = cli_command_catalog()
        .iter()
        .map(|command| command.id)
        .collect::<Vec<_>>();

    assert_eq!(ids.len(), 24);
    assert!(!ids.contains(&"cli.workbench"));
    assert!(ids.contains(&"cli.validate"));
    assert!(ids.contains(&"cli.task.check"));
    assert!(ids.contains(&"cli.add"));
    assert!(
        cli_command_catalog()
            .iter()
            .any(|command| command.id == "cli.list" && command.opens_spec_browser)
    );
}

#[test]
fn filters_cli_commands_by_query_and_previews_invocation() {
    let mut ui = WorkbenchUiState::from_state(WorkbenchState::default());
    ui.set_query("validate");

    let visible = ui.visible_cli_commands();
    let preview = ui.cli_command_preview("cli.validate").unwrap();

    assert!(visible.iter().any(|command| command.id == "cli.validate"));
    assert!(preview.invocation.contains("syu validate ."));
    assert_eq!(preview.evidence_summary, "read-only");
}
