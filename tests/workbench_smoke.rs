use dioxus::prelude::*;
use dioxus_ssr::render_element;
use syu_app_ui::{
    AppShell, BranchScopeLens, EvidencePanel, GoalCanvas, GoalPlanExportPanel, SpecImpactGraph,
    WorkbenchUiState, build_demo_state,
};
use syu_workbench::{
    WorkbenchActionId, WorkbenchActionRegistry, WorkbenchApiPayload, WorkbenchState,
};

#[test]
fn app_shell_renders_workbench_pulse_before_the_side_panels() {
    let html = render_element(rsx! {
        AppShell { ui: build_demo_state() }
    });

    assert!(html.contains("Workbench Pulse"));
    assert!(html.contains("Goal Rail"));
    assert!(html.contains("Evidence"));
    assert!(html.contains("Command Palette"));
}

#[test]
fn command_palette_renders_disabled_reason_for_unavailable_actions() {
    let mut ui = WorkbenchUiState::from_state(WorkbenchState::default());
    ui.set_query("goal");

    let html = render_element(rsx! {
        AppShell { ui }
    });

    assert!(html.contains("disabled: missing active_goal_plan"));
    assert!(html.contains("goal.check"));
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

    let pulse = html.find("Workbench Pulse").expect("pulse should render");
    let preview = html
        .find("Read-only action placeholder")
        .expect("preview should render");

    assert!(pulse < preview);
    assert!(html.contains("Read-only action placeholder"));
    assert!(html.contains("Evidence placeholder"));
}

#[test]
fn evidence_panel_renders_placeholder_when_empty() {
    let ui = WorkbenchUiState::from_state(WorkbenchState::default());

    let html = render_element(rsx! {
        EvidencePanel { ui }
    });

    assert!(html.contains("Evidence timeline"));
    assert!(html.contains("Append evidence by running goal checks"));
}

#[test]
fn evidence_panel_renders_goal_scoped_timeline() {
    let ui = build_demo_state();

    let html = render_element(rsx! {
        EvidencePanel { ui }
    });

    assert!(html.contains("Evidence Timeline"));
    assert!(html.contains("goal GOAL-WB-REQUEST-001"));
    assert!(html.contains("goal.test_select"));
    assert!(html.contains("goal.check"));
    assert!(!html.contains("validation passed"));
}

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

    assert!(preview.result_summary.contains("placeholder"));
    assert!(preview.evidence_summary.contains("Evidence placeholder"));
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
fn request_intake_flow_renders_generated_goal_plan() {
    let ui = build_demo_state();

    let html = render_element(rsx! {
        GoalCanvas {
            ui,
            on_run_action: None
        }
    });

    assert!(html.contains("Request Intake"));
    assert!(html.contains("Change request"));
    assert!(html.contains("requirement_change"));
    assert!(html.contains("Scaffold Preview"));
    assert!(html.contains("Goal Plan"));
    assert!(html.contains("GOAL-WB-REQUEST-001"));
    assert!(html.contains("non-goal: Build a raw YAML editor"));
    assert!(html.contains("include: crates/syu-app-ui/src/model.rs"));
    assert!(html.contains("Export YAML"));
    assert!(html.contains("syu.goal_plan"));
}

#[test]
fn request_flow_actions_are_exposed_in_the_command_palette() {
    let mut ui = build_demo_state();
    ui.set_query("request.");

    let html = render_element(rsx! {
        AppShell { ui }
    });

    assert!(html.contains("request.classify"));
    assert!(html.contains("request.scope"));
    assert!(html.contains("request.scaffold"));
    assert!(html.contains("request.plan"));
}

#[test]
fn goal_plan_export_panel_marks_yaml_as_temporary_artifact() {
    let ui = build_demo_state();
    let plan = ui.payload.state.goals.active[0]
        .goal_plan
        .clone()
        .expect("demo state includes a goal plan");

    let html = render_element(rsx! {
        GoalPlanExportPanel { plan }
    });

    assert!(html.contains(".syu/workbench/goals/GOAL-WB-REQUEST-001.yaml"));
    assert!(html.contains("completion:"));
    assert!(html.contains("cargo test --test workbench_smoke"));
}

#[test]
fn branch_scope_lens_renders_scope_ownership_and_tests() {
    let ui = build_demo_state();

    let html = render_element(rsx! {
        BranchScopeLens {
            ui,
            on_run_action: None
        }
    });

    assert!(html.contains("Branch Scope Lens"));
    assert!(html.contains("origin/main...HEAD"));
    assert!(html.contains("Changed Files"));
    assert!(html.contains("ownership-known"));
    assert!(html.contains("ownership-missing"));
    assert!(html.contains("Out Of Scope"));
    assert!(html.contains("Affected Specs"));
    assert!(html.contains("FEAT-WORKBENCH-BRANCH-SCOPE-001"));
    assert!(html.contains("Suggested Goal Split"));
    assert!(html.contains("Goal Scope Comparison"));
    assert!(html.contains("files included by Goal"));
    assert!(html.contains("files excluded by Goal"));
    assert!(html.contains("changed files not covered by Goal"));
    assert!(html.contains("tests required by Goal"));
    assert!(html.contains("tests detected from code ownership"));
    assert!(html.contains("tests/workbench_smoke.rs"));
}

#[test]
fn spec_impact_graph_renders_typed_nodes_edges_and_legend() {
    let ui = build_demo_state();

    let html = render_element(rsx! {
        SpecImpactGraph { ui }
    });

    assert!(html.contains("Spec Impact Graph"));
    assert!(html.contains("spec-linked"));
    assert!(html.contains("code-linked"));
    assert!(html.contains("test-linked"));
    assert!(html.contains("scope-ambiguous"));
    assert!(html.contains("FEAT-WORKBENCH-SPEC-GRAPH-001"));
    assert!(html.contains("crates/syu-app-ui/src/components/shell.rs"));
    assert!(html.contains("role=\"button\""));
    assert!(html.contains("border-command-active"));
}
