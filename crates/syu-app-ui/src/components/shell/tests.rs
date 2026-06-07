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
    ui.set_query("goal");
    ui.select_cli_command("cli.task.check");

    let html = render_element(rsx! {
        AppShell { ui, active_pane: WorkbenchPane::Commands, sidebar_open: false }
    });

    assert!(html.contains("name=\"lang\" value=\"ja\""));
    assert!(html.contains("name=\"query\" value=\"goal\""));
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
fn command_palette_renders_category_filters_and_badges() {
    let html = render_element(rsx! {
        AppShell { ui: build_demo_state(), active_pane: WorkbenchPane::Commands, sidebar_open: false }
    });

    assert!(html.contains("aria-label=\"Command categories\""));
    assert!(html.contains("data-command-category=\"check\""));
    assert!(html.contains(">Check</a>"));
}

#[test]
fn check_result_renders_summary_list_and_detail_layout() {
    let mut ui = build_demo_state();
    ui.select_cli_command("cli.validate");

    let html = render_element(rsx! {
        AppShell { ui, active_pane: WorkbenchPane::Commands, sidebar_open: false }
    });

    assert!(html.contains("data-result-kind=\"CheckDetail\""));
    assert!(html.contains("data-check-summary=\"true\""));
    assert!(html.contains("aria-label=\"Result items\""));
    assert!(html.contains("aria-current=\"page\""));
    assert!(html.contains("data-result-detail=\"pending\""));
    assert!(html.contains("style=\"max-height: 36rem\""));
    assert!(!html.contains("Focus the top box, type to filter, and pick a result."));
}

#[test]
fn change_result_is_rendered_below_its_execution_form() {
    let mut ui = build_demo_state();
    ui.select_cli_command("cli.init");

    let html = render_element(rsx! {
        AppShell { ui, active_pane: WorkbenchPane::Commands, sidebar_open: false }
    });

    let run_form = html.find("name=\"run\" value=\"1\"").expect("run form");
    let result = html
        .find("data-result-kind=\"ChangeDetail\"")
        .expect("typed result");
    assert!(run_form < result);
}

#[test]
fn every_result_category_has_a_distinct_summary_surface() {
    let cases = [
        ("cli.log", "browse", "data-browse-context"),
        ("cli.validate", "check", "data-check-summary"),
        ("cli.task.plan", "plan", "data-plan-summary"),
        ("cli.init", "change", "data-change-summary"),
        ("cli.lsp", "operate", "data-operation-summary"),
        ("cli.report", "generate", "data-generated-summary"),
    ];

    for (command_id, category, summary_marker) in cases {
        let mut ui = build_demo_state();
        ui.select_cli_command(command_id);
        let html = render_element(rsx! {
            AppShell { ui, active_pane: WorkbenchPane::Commands, sidebar_open: false }
        });

        assert!(html.contains(&format!("data-category-layout=\"{category}\"")));
        assert!(html.contains(summary_marker));
    }
}

#[test]
fn spec_browse_commands_render_search_list_and_detail_without_run_form() {
    let mut ui = build_demo_state();
    ui.set_query("REQ-WORKBENCH");
    ui.set_spec_query("browser");
    ui.select_cli_command("cli.list");
    ui.spec_browser = Some(SpecBrowserModel {
        sections: Vec::new(),
        selected_item_id: None,
    });

    let html = render_element(rsx! {
        AppShell { ui, active_pane: WorkbenchPane::Commands, sidebar_open: false }
    });

    assert!(html.contains("data-category-layout=\"browse\""));
    assert!(html.contains("Search specs"));
    assert!(html.contains("aria-label=\"Spec tree\""));
    assert!(html.contains("style=\"max-height: 30rem\""));
    assert!(html.contains("data-spec-search=\"true\""));
    assert!(html.contains("data-spec-detail=\"true\""));
    assert!(html.contains("name=\"query\" value=\"REQ-WORKBENCH\""));
    assert!(html.contains("name=\"spec_query\" value=\"browser\""));
    assert!(!html.contains("name=\"run\" value=\"1\""));
}

#[test]
fn spec_browser_filters_tree_without_changing_palette_query() {
    let mut ui = build_demo_state();
    ui.set_query("show");
    ui.set_spec_query("matching summary");
    ui.select_cli_command("cli.show");
    ui.spec_browser = Some(SpecBrowserModel {
        sections: vec![SpecBrowserSection {
            label: "Requirements".to_string(),
            documents: vec![SpecBrowserDocument {
                path: "requirements.yaml".to_string(),
                title: "Requirements".to_string(),
                folder_segments: Vec::new(),
                items: vec![
                    test_spec_item("REQ-MATCH", "Matching item", Some("matching summary")),
                    test_spec_item("REQ-HIDDEN", "Hidden item", Some("unrelated")),
                ],
            }],
        }],
        selected_item_id: Some("REQ-HIDDEN".to_string()),
    });

    let html = render_element(rsx! {
        AppShell { ui, active_pane: WorkbenchPane::Commands, sidebar_open: false }
    });

    assert!(html.contains("name=\"query\" value=\"show\""));
    assert!(html.contains("name=\"spec_query\" value=\"matching summary\""));
    assert!(html.contains("REQ-MATCH"));
    assert!(!html.contains("REQ-HIDDEN"));
}

#[test]
fn spec_browser_renders_empty_state_for_no_matches() {
    let mut ui = build_demo_state();
    ui.set_spec_query("missing");
    ui.select_cli_command("cli.search");
    ui.spec_browser = Some(SpecBrowserModel {
        sections: vec![SpecBrowserSection {
            label: "Requirements".to_string(),
            documents: vec![SpecBrowserDocument {
                path: "requirements.yaml".to_string(),
                title: "Requirements".to_string(),
                folder_segments: Vec::new(),
                items: vec![test_spec_item("REQ-ONLY", "Only item", None)],
            }],
        }],
        selected_item_id: Some("REQ-ONLY".to_string()),
    });

    let html = render_element(rsx! {
        AppShell { ui, active_pane: WorkbenchPane::Commands, sidebar_open: false }
    });

    assert!(html.contains("No matching spec items"));
    assert!(!html.contains("REQ-ONLY"));
}

fn test_spec_item(id: &str, title: &str, summary: Option<&str>) -> SpecBrowserItem {
    SpecBrowserItem {
        kind: "requirement".to_string(),
        id: id.to_string(),
        title: title.to_string(),
        summary: summary.map(str::to_string),
        description: None,
        product_design_principle: None,
        coding_guideline: None,
        priority: None,
        status: None,
        linked_philosophies: Vec::new(),
        linked_policies: Vec::new(),
        linked_requirements: Vec::new(),
        linked_features: Vec::new(),
        tests: Vec::new(),
        implementations: Vec::new(),
    }
}

#[test]
fn plan_commands_do_not_append_the_browse_surface() {
    let mut ui = build_demo_state();
    ui.select_cli_command("cli.task.scope");
    ui.spec_browser = Some(SpecBrowserModel {
        sections: Vec::new(),
        selected_item_id: None,
    });

    let html = render_element(rsx! {
        AppShell { ui, active_pane: WorkbenchPane::Commands, sidebar_open: false }
    });

    assert!(html.contains("data-category-layout=\"plan\""));
    assert!(!html.contains("Search specs"));
    assert!(!html.contains("aria-label=\"Spec tree\""));
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
