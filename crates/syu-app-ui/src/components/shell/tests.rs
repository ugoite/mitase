use super::*;
use crate::model::build_demo_state;
use dioxus_ssr::render_element;
use syu_workbench::{
    AgentRunMode, Assignee, Assignment, AssignmentScope, AssignmentStatus, ScopeGuardResult,
    ScopeGuardStatus, WorkbenchActionId, WorkbenchState,
};

#[test]
fn app_shell_renders_command_palette_first_shell() {
    let html = render_element(rsx! {
        AppShell { ui: build_demo_state(), active_pane: WorkbenchPane::Commands, sidebar_open: true }
    });

    assert!(html.contains("Syu"));
    assert!(html.contains("Type a command"));
    assert!(html.contains("Command palette"));
    assert!(html.contains("data-command-palette"));
    assert!(!html.contains("navigation"));
    assert!(!html.contains("<select"));
    assert!(!html.contains(">focus</span>"));
    assert!(!html.contains(">filter</span>"));
    assert!(!html.contains(">run</span>"));
}

#[test]
fn command_palette_renders_disabled_reason_for_unavailable_actions() {
    let mut ui = WorkbenchUiState::from_state(WorkbenchState::default());
    ui.set_query("goal");

    let html = render_element(rsx! {
        AppShell { ui, active_pane: WorkbenchPane::Commands, sidebar_open: true }
    });

    assert!(html.contains("goal.check"));
}

#[test]
fn command_palette_preserves_selected_locale_across_search_links_and_run_forms() {
    let mut ui = build_demo_state();
    ui.set_locale(Locale::Ja);
    ui.set_query("validate");

    let html = render_element(rsx! {
        AppShell { ui, active_pane: WorkbenchPane::Commands, sidebar_open: false }
    });

    assert!(html.contains("name=\"lang\" value=\"ja\""));
    assert!(
        html.contains("href=\"?pane=commands&#38;sidebar=0&#38;lang=ja&#38;cli=cli.validate\"")
    );
    assert!(html.contains("data-command-id=\"cli.validate\""));
}

#[test]
fn runnable_cli_forms_preserve_selected_locale() {
    let mut ui = build_demo_state();
    ui.set_locale(Locale::Ja);
    ui.select_cli_command("cli.task.check");

    let html = render_element(rsx! {
        AppShell { ui, active_pane: WorkbenchPane::Commands, sidebar_open: false }
    });

    assert!(html.contains("name=\"lang\" value=\"ja\""));
    assert!(html.contains("name=\"cli\" value=\"cli.task.check\""));
}

#[test]
fn cli_result_uses_explicit_run_form_without_log_shortcut() {
    let mut ui = build_demo_state();
    ui.set_locale(Locale::Ja);
    ui.select_cli_command("cli.validate");

    let html = render_element(rsx! {
        AppShell { ui, active_pane: WorkbenchPane::Commands, sidebar_open: false }
    });

    assert!(!html.contains("Show log"));
    assert!(html.contains("name=\"run\" value=\"1\""));
    assert!(html.contains("name=\"cli\" value=\"cli.validate\""));
}

#[test]
fn human_assignments_hide_the_dry_run_action() {
    let human_assignment = Assignment {
        assignee: Some(Assignee::human("Manual Reviewer")),
        run_mode: AgentRunMode::Manual,
        status: AssignmentStatus::AssignmentReady,
        scope_guard: ScopeGuardResult {
            status: ScopeGuardStatus::ScopeValid,
            blockers: Vec::new(),
            out_of_scope_files: Vec::new(),
        },
        scope: AssignmentScope::default(),
        ..Assignment::default()
    };
    let automated_assignment = Assignment {
        assignee: Some(Assignee::local_command("local-coder", "Local coder")),
        ..human_assignment.clone()
    };

    assert!(!assignment_has_automated_assignee(&human_assignment));
    assert!(assignment_has_automated_assignee(&automated_assignment));
}

#[test]
fn goal_canvas_renders_a_read_only_action_preview_placeholder() {
    let mut ui = build_demo_state();
    ui.run_read_only_action(WorkbenchActionId::HistoryShow);

    let html = render_element(rsx! {
        GoalCanvas {
            ui,
            on_run_action: None
        }
    });

    let pulse = html.find("workspace").expect("workspace should render");
    let preview = html
        .find("Preview opened for")
        .expect("preview should render");

    assert!(pulse < preview);
    assert!(html.contains("Preview opened for"));
    assert!(html.contains("Ready to review"));
}

#[test]
fn goal_plan_yaml_preview_roundtrips_through_the_task_model_schema() {
    let ui = build_demo_state();
    let plan = ui
        .payload
        .state
        .goals
        .active_goal()
        .and_then(|goal| goal.goal_plan.as_ref())
        .expect("demo state should include an active goal plan");

    let yaml = goal_plan_yaml_preview(plan);
    let parsed: GoalPlanArtifact =
        serde_yaml::from_str(&yaml).expect("preview should deserialize as a GoalPlanArtifact");

    assert_eq!(&parsed, plan);
    assert!(yaml.contains("test_plan:"));
    assert!(yaml.contains("coverage:"));
}

#[test]
fn evidence_panel_renders_placeholder_when_empty() {
    let ui = WorkbenchUiState::from_state(WorkbenchState::default());

    let html = render_element(rsx! {
        EvidencePanel { ui }
    });

    assert!(html.contains("Append evidence by running goal checks"));
}
