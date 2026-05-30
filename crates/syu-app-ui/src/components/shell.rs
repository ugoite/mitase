use crate::components::{
    Button, CommandItem, DetailDrawer, EmptyState, EvidenceLogRow, GoalCard, IconButton, Panel,
    ScopeChip, StatusDot,
};
use crate::design::classes;
use crate::model::{WorkbenchUiState, WorkspacePulseSummary};
use dioxus::prelude::*;

#[component]
pub fn AppShell(ui: WorkbenchUiState) -> Element {
    rsx! {
        div { class: classes::APP_SHELL,
            div { class: classes::PAGE_FRAME,
                StatusBar { ui: ui.clone() }
                if ui.command_palette_open {
                    CommandPalette { ui: ui.clone() }
                }
                div { class: classes::MAIN_GRID,
                    GoalRail { ui: ui.clone() }
                    GoalCanvas { ui: ui.clone() }
                    EvidencePanel { ui: ui.clone() }
                }
            }
        }
    }
}

#[component]
pub fn StatusBar(ui: WorkbenchUiState) -> Element {
    let summary = ui.pulse_summary();
    rsx! {
        header { class: classes::CHROME_BAR,
            div { class: "flex items-center gap-3",
                Button { label: "Cmd+K".to_string(), active: ui.command_palette_open, disabled: false }
                div { class: classes::CHROME_META,
                    ScopeChip { label: summary.workspace.clone() }
                    ScopeChip { label: summary.branch.clone() }
                    ScopeChip { label: summary.health.clone() }
                }
            }
            div { class: "flex items-center gap-2",
                IconButton { label: "Workspace".to_string(), icon: "⌁".to_string() }
                StatusDot { tone_class: "bg-evidence-pass", label: format!("{} actions", summary.available_actions) }
            }
        }
    }
}

#[component]
pub fn WorkspacePulse(summary: WorkspacePulseSummary) -> Element {
    rsx! {
        Panel { class: classes::PANEL_MUTED,
            div { class: "flex flex-col gap-3 p-4",
                div { class: classes::SECTION_HEADER,
                    h2 { class: classes::SECTION_TITLE, "Workbench Pulse" }
                    ScopeChip { label: summary.health.clone() }
                }
                div { class: "grid gap-3 md:grid-cols-2",
                    div { class: "space-y-1",
                        p { class: "text-xs uppercase tracking-[0.18em] text-foreground/60", "workspace" }
                        p { class: "text-sm", "{summary.workspace}" }
                    }
                    div { class: "space-y-1",
                        p { class: "text-xs uppercase tracking-[0.18em] text-foreground/60", "branch" }
                        p { class: "text-sm", "{summary.branch}" }
                    }
                }
                div { class: "grid gap-3 md:grid-cols-3",
                    PulseMetric { label: "available actions".to_string(), value: summary.available_actions.to_string() }
                    PulseMetric { label: "recent evidence".to_string(), value: summary.recent_evidence.clone() }
                    PulseMetric { label: "next suggested".to_string(), value: summary.next_action.clone() }
                }
            }
        }
    }
}

#[component]
pub fn CommandPalette(ui: WorkbenchUiState) -> Element {
    let entries = ui.visible_actions();
    rsx! {
        Panel { class: classes::PANEL_MUTED,
            div { class: "flex flex-col gap-3 p-4",
                div { class: classes::SECTION_HEADER,
                    h2 { class: classes::SECTION_TITLE, "Command Palette" }
                    ScopeChip { label: if ui.command_palette_open { "open".to_string() } else { "closed".to_string() } }
                }
                input {
                    class: "w-full rounded-xl border border-border bg-background px-3 py-2 text-sm outline-none",
                    value: "{ui.command_query}",
                    placeholder: "Filter actions"
                }
                div { class: "space-y-2",
                    for entry in entries.iter().cloned() {
                        CommandItem { entry: entry, selected: false }
                    }
                }
                if entries.is_empty() {
                    EmptyState { title: "No matching actions".to_string(), body: "Try a different command palette filter.".to_string() }
                }
            }
        }
    }
}

#[component]
pub fn GoalRail(ui: WorkbenchUiState) -> Element {
    rsx! {
        Panel { class: classes::PANEL,
            div { class: classes::PANEL_INNER,
                div { class: classes::SECTION_HEADER,
                    h2 { class: classes::SECTION_TITLE, "Goal Rail" }
                    ScopeChip { label: format!("{} goals", ui.payload.state.goals.active.len()) }
                }
                div { class: classes::SECTION_BODY,
                    if ui.payload.state.goals.active.is_empty() {
                        EmptyState { title: "No active goals".to_string(), body: "The Workbench will anchor the first goal here.".to_string() }
                    } else {
                        for goal in &ui.payload.state.goals.active {
                            GoalCard {
                                goal_id: goal.goal_id.clone(),
                                title: goal_title(goal),
                                selected: ui.payload.state.goals.selected_goal_id.as_ref() == Some(&goal.goal_id),
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub fn GoalCanvas(ui: WorkbenchUiState) -> Element {
    let summary = ui.pulse_summary();
    rsx! {
        Panel { class: classes::PANEL,
            div { class: classes::PANEL_INNER,
                WorkspacePulse { summary: summary.clone() }
                if let Some(preview) = ui.preview.clone().or_else(|| ui.selected_action_id.and_then(|id| ui.action_preview(id))) {
                    DetailDrawer {
                        title: preview.title.clone(),
                        body: preview.result_summary.clone(),
                        evidence: preview.evidence_summary.clone(),
                    }
                } else if let Some(action) = ui.selected_action() {
                    DetailDrawer {
                        title: action.title.clone(),
                        body: action.description.clone(),
                        evidence: format!("ready for {}", action.evidence_kind.label()),
                    }
                } else {
                    EmptyState { title: "No action selected".to_string(), body: "Open the palette to run a read-only action or inspect the next suggested step.".to_string() }
                }
            }
        }
    }
}

#[component]
pub fn EvidencePanel(ui: WorkbenchUiState) -> Element {
    rsx! {
        Panel { class: classes::PANEL,
            div { class: classes::PANEL_INNER,
                div { class: classes::SECTION_HEADER,
                    h2 { class: classes::SECTION_TITLE, "Evidence" }
                    ScopeChip { label: format!("{} entries", ui.payload.state.evidence_timeline.entries.len()) }
                }
                div { class: classes::SECTION_BODY,
                    if ui.payload.state.evidence_timeline.entries.is_empty() {
                        EmptyState { title: "Evidence placeholder".to_string(), body: "The first implementation keeps proof visible here.".to_string() }
                    } else {
                        for entry in &ui.payload.state.evidence_timeline.entries {
                            EvidenceLogRow { entry: entry.clone() }
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub fn PulseMetric(label: String, value: String) -> Element {
    rsx! {
        div { class: "rounded-xl border border-border bg-panel p-3",
            p { class: "text-xs uppercase tracking-[0.18em] text-foreground/60", "{label}" }
            p { class: "mt-1 text-sm", "{value}" }
        }
    }
}

fn goal_title(goal: &syu_workbench::ActiveGoalState) -> String {
    goal.goal_plan
        .as_ref()
        .map(|plan| plan.goal.title.clone())
        .unwrap_or_else(|| "Untitled goal".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::build_demo_state;
    use dioxus_ssr::render_element;
    use syu_workbench::{WorkbenchActionId, WorkbenchState};

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
            CommandPalette { ui: ui.clone() }
        });

        assert!(html.contains("disabled: missing active_goal_plan"));
        assert!(html.contains("goal.check"));
    }

    #[test]
    fn goal_canvas_renders_a_read_only_action_preview_placeholder() {
        let mut ui = build_demo_state();
        ui.run_read_only_action(WorkbenchActionId::HistoryShow);

        let html = render_element(rsx! {
            GoalCanvas { ui }
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

        assert!(html.contains("Evidence placeholder"));
    }
}
