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
    assert!(html.contains("navigation"));
    assert!(html.contains(">Items</span>"));
    assert!(html.contains(">Diagnostics</span>"));
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
    assert!(html.contains("href=\"?pane=diagnostics&#38;lang=ja&#38;cli=cli.validate\""));
    assert!(html.contains("data-command-id=\"cli.validate\""));
    assert!(!html.contains("sidebar=0"));
}

#[test]
fn diagnostics_overview_preserves_selected_locale_on_validation_links() {
    let mut ui = build_demo_state();
    ui.set_locale(Locale::Ja);

    let html = render_element(rsx! {
        AppShell { ui, active_pane: WorkbenchPane::Diagnostics, sidebar_open: true }
    });

    assert!(html.contains("href=\"?pane=diagnostics&#38;lang=ja&#38;cli=cli.validate\""));
    assert!(html.contains("href=\"?pane=diagnostics&#38;lang=ja&#38;cli=cli.doctor\""));
    assert!(html.contains("href=\"?pane=diagnostics&#38;lang=ja&#38;cli=cli.audit\""));
}

#[test]
fn browse_and_action_links_preserve_selected_locale() {
    let mut ui = build_demo_state();
    ui.set_locale(Locale::Ja);
    let mut linked_item = test_spec_item("REQ-ONLY", "Only item", None);
    linked_item
        .linked_requirements
        .push("REQ-RELATED".to_string());
    ui.spec_browser = Some(SpecBrowserModel {
        sections: vec![SpecBrowserSection {
            label: "Requirements".to_string(),
            documents: vec![SpecBrowserDocument {
                path: "requirements.yaml".to_string(),
                title: "Requirements".to_string(),
                folder_segments: Vec::new(),
                items: vec![linked_item],
            }],
        }],
        selected_item_id: Some("REQ-ONLY".to_string()),
    });

    let browse_html = render_element(rsx! {
        AppShell { ui: ui.clone(), active_pane: WorkbenchPane::Items, sidebar_open: true }
    });

    assert!(
        browse_html.contains(
            "href=\"?pane=items&#38;lang=ja&#38;cli=cli.show&#38;spec_item=REQ-RELATED\""
        )
    );

    let action_html = render_element(rsx! {
        FlowActionButton {
            label: "Validate".to_string(),
            action_id: WorkbenchActionId::ValidationRun,
            ui,
            onclick: None,
        }
    });

    assert!(action_html.contains("href=\"?pane=commands&#38;lang=ja&#38;action=validation.run\""));
}

#[test]
fn language_switch_keeps_the_full_sidebar() {
    let html = render_element(rsx! {
        AppShell { ui: build_demo_state(), active_pane: WorkbenchPane::Request, sidebar_open: false }
    });

    assert!(html.contains("navigation"));
    assert!(html.contains(">Items</span>"));
    assert!(html.contains("?pane=request&#38;lang=ja"));
    assert!(!html.contains("sidebar=0"));
    assert!(!html.contains("show sidebar"));
}

#[test]
fn navigation_links_clear_command_context_but_keep_locale() {
    let mut ui = build_demo_state();
    ui.set_locale(Locale::Ja);
    ui.set_query("show");
    ui.set_spec_query("workbench");
    ui.select_cli_command("cli.show");

    let html = render_element(rsx! {
        AppShell { ui, active_pane: WorkbenchPane::Items, sidebar_open: true }
    });

    assert!(html.contains("href=\"?pane=request&#38;lang=ja\""));
    assert!(html.contains("href=\"?pane=branch&#38;lang=ja\""));
    assert!(html.contains("href=\"?pane=diagnostics&#38;lang=ja\""));
    assert!(!html.contains("sidebar="));
    assert!(!html.contains("href=\"?pane=request&#38;lang=ja&#38;cli=cli.show"));
    assert!(!html.contains("href=\"?pane=branch&#38;lang=ja&#38;cli=cli.show"));
    assert!(!html.contains("href=\"?pane=diagnostics&#38;lang=ja&#38;cli=cli.show"));
    assert!(!html.contains("href=\"?pane=pulse"));
}

#[test]
fn role_subview_links_clear_command_context() {
    let mut ui = build_demo_state();
    ui.set_query("show");
    ui.select_cli_command("cli.show");

    let html = render_element(rsx! {
        AppShell { ui, active_pane: WorkbenchPane::Request, sidebar_open: true }
    });

    assert!(html.contains("href=\"?pane=request&#38;lang=en\""));
    assert!(html.contains("href=\"?pane=goals&#38;lang=en\""));
    assert!(html.contains("href=\"?pane=assignment&#38;lang=en\""));
    assert!(html.contains("href=\"?pane=evidence&#38;lang=en\""));
    assert!(!html.contains("sidebar="));
    assert!(!html.contains("href=\"?pane=request&#38;lang=en&#38;cli=cli.show"));
    assert!(!html.contains("href=\"?pane=goals&#38;lang=en&#38;cli=cli.show"));
    assert!(!html.contains("href=\"?pane=pulse"));
}

#[test]
fn stage_help_uses_active_pane_copy_without_collapsing_sidebar() {
    let mut ui = build_demo_state();
    ui.set_locale(Locale::En);
    let request_html = render_element(rsx! {
        WorkbenchStage { ui: ui.clone(), active_pane: WorkbenchPane::Request }
    });

    assert!(request_html.contains("help-tooltip-request"));
    assert!(
        request_html
            .contains("Shows the request text, its classification, and the scope it opens.")
    );
    assert!(!request_html.contains("href=\"?pane=request&#38;sidebar=0"));

    let commands_html = render_element(rsx! {
        WorkbenchStage { ui, active_pane: WorkbenchPane::Commands }
    });

    assert!(commands_html.contains("help-tooltip-palette"));
    assert!(commands_html.contains("Focus the top box, type a few letters, then choose a result."));
    assert!(!commands_html.contains("href=\"?pane=commands&#38;sidebar=0"));
}

#[test]
fn help_tooltip_is_visible_on_keyboard_focus() {
    let html = render_element(rsx! {
        WorkbenchStage { ui: build_demo_state(), active_pane: WorkbenchPane::Request }
    });

    assert!(html.contains("group-focus-within:opacity-100"));
    assert!(html.contains("group-focus-within:translate-y-0"));
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
    assert!(html.contains("data-command-run-form=\"true\""));
    assert!(html.contains("data-command-run-button=\"true\""));
    assert!(html.contains("data-command-run-status=\"true\""));
    assert!(html.contains("aria-live=\"polite\""));
    assert!(html.contains("data-running-label=\"実行中...\""));
}

#[test]
fn workbench_action_run_form_exposes_shared_running_ui() {
    let mut ui = build_demo_state();
    ui.select_action(WorkbenchActionId::ValidationRun);

    let html = render_element(rsx! {
        AppShell { ui, active_pane: WorkbenchPane::Commands, sidebar_open: false }
    });

    assert!(html.contains("name=\"action\" value=\"validation.run\""));
    assert!(html.contains("data-command-run-form=\"true\""));
    assert!(html.contains("data-command-run-button=\"true\""));
    assert!(html.contains("data-command-run-status=\"true\""));
    assert!(html.contains("aria-live=\"polite\""));
    assert!(html.contains("data-running-label=\"Running...\""));
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
    assert!(html.contains("aria-label=\"Spec kind tabs\""));
    assert!(html.contains("data-spec-kind-tab=\"philosophy\""));
    assert!(html.contains("data-spec-kind-tab=\"policy\""));
    assert!(html.contains("data-spec-kind-tab=\"requirement\""));
    assert!(html.contains("data-spec-kind-tab=\"feature\""));
    assert!(html.contains("name=\"query\" value=\"REQ-WORKBENCH\""));
    assert!(html.contains("name=\"spec_query\" value=\"browser\""));
    assert!(!html.contains("name=\"run\" value=\"1\""));
}

#[test]
fn spec_browser_kind_tabs_filter_to_selected_layer() {
    let mut ui = build_demo_state();
    ui.set_spec_kind("requirement");
    ui.select_cli_command("cli.show");
    ui.spec_browser = Some(SpecBrowserModel {
        sections: vec![
            SpecBrowserSection {
                label: "Philosophy".to_string(),
                documents: vec![SpecBrowserDocument {
                    path: "philosophy.yaml".to_string(),
                    title: "Philosophy".to_string(),
                    folder_segments: Vec::new(),
                    items: vec![test_spec_item("PHIL-ONLY", "Hidden philosophy", None)],
                }],
            },
            SpecBrowserSection {
                label: "Requirements".to_string(),
                documents: vec![SpecBrowserDocument {
                    path: "requirements.yaml".to_string(),
                    title: "Requirements".to_string(),
                    folder_segments: Vec::new(),
                    items: vec![test_spec_item("REQ-ONLY", "Visible requirement", None)],
                }],
            },
        ],
        selected_item_id: Some("REQ-ONLY".to_string()),
    });

    let html = render_element(rsx! {
        AppShell { ui, active_pane: WorkbenchPane::Items, sidebar_open: true }
    });

    assert!(html.contains("data-spec-kind-panel=\"requirement\""));
    assert!(html.contains("data-spec-kind-tab=\"requirement\""));
    assert!(html.contains("aria-current=\"page\""));
    assert!(html.contains("aria-[current=page]:bg-foreground"));
    assert!(html.contains("name=\"spec_kind\" value=\"requirement\""));
    assert!(html.contains("REQ-ONLY"));
    assert!(!html.contains("PHIL-ONLY"));
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
fn spec_browser_groups_documents_into_explorer_folders() {
    let mut ui = build_demo_state();
    ui.select_cli_command("cli.show");
    ui.spec_browser = Some(SpecBrowserModel {
        sections: vec![SpecBrowserSection {
            label: "Features".to_string(),
            documents: vec![
                SpecBrowserDocument {
                    path: "features/cli/commands.yaml".to_string(),
                    title: "Commands".to_string(),
                    folder_segments: vec!["features".to_string(), "cli".to_string()],
                    items: vec![test_spec_item("FEAT-CLI-001", "Command palette", None)],
                },
                SpecBrowserDocument {
                    path: "features/cli/navigation.yaml".to_string(),
                    title: "Navigation".to_string(),
                    folder_segments: vec!["features".to_string(), "cli".to_string()],
                    items: vec![test_spec_item("FEAT-CLI-002", "Navigation", None)],
                },
                SpecBrowserDocument {
                    path: "features/workbench/items.yaml".to_string(),
                    title: "Items".to_string(),
                    folder_segments: vec!["features".to_string(), "workbench".to_string()],
                    items: vec![test_spec_item("FEAT-WB-001", "Items tree", None)],
                },
            ],
        }],
        selected_item_id: Some("FEAT-CLI-001".to_string()),
    });

    let html = render_element(rsx! {
        AppShell { ui, active_pane: WorkbenchPane::Items, sidebar_open: true }
    });

    assert_eq!(
        html.matches("data-spec-folder-path=\"cli\"").count(),
        1,
        "{html}"
    );
    assert_eq!(
        html.matches("data-spec-folder-path=\"workbench\"").count(),
        1,
        "{html}"
    );
    assert!(html.contains("data-spec-folder-icon=\"true\""));
    assert!(html.contains("data-spec-folder-toggle=\"true\""));
    assert!(html.contains("folder"));
    assert!(!html.contains("features / cli"));
    assert!(html.contains("data-spec-document-path=\"features/cli/commands.yaml\""));
    assert!(html.contains("data-spec-document-path=\"features/cli/navigation.yaml\""));
    assert!(html.contains("data-spec-item-target=\"FEAT-CLI-001\""));
    assert!(html.contains("data-spec-detail-card=\"FEAT-CLI-001\""));
    assert!(html.contains("FEAT-CLI-001"));
    assert!(html.contains("FEAT-CLI-002"));
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

    let preview = html
        .find("Preview opened for")
        .expect("preview should render");

    assert!(!html.contains("workspace"));
    assert!(html.contains("Preview opened for"));
    assert!(html.contains("Ready to review"));
    assert!(preview > 0);
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
