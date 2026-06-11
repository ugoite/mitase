use super::*;

#[component]
pub fn GoalRail(ui: WorkbenchUiState) -> Element {
    let goal_empty_title = if ui.locale == Locale::Ja {
        "なし"
    } else {
        "None"
    };
    let goal_empty_body = if ui.locale == Locale::Ja {
        "最初のゴールはここに表示されます。"
    } else {
        "The first goal appears here."
    };
    rsx! {
        Panel { class: classes::PANEL,
            div { class: classes::PANEL_INNER,
                div { class: classes::SECTION_HEADER,
                    h2 { class: classes::SECTION_TITLE, if ui.locale == Locale::Ja { "ゴール" } else { "Goals" } }
                    ScopeChip { label: format!("{}", ui.payload.state.goals.active.len()) }
                }
                div { class: classes::SECTION_BODY,
                    if ui.payload.state.goals.active.is_empty() {
                        EmptyState { title: goal_empty_title.to_string(), body: goal_empty_body.to_string() }
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
pub fn GoalCanvas(
    ui: WorkbenchUiState,
    on_run_action: Option<EventHandler<WorkbenchActionId>>,
) -> Element {
    let locale = ui.locale;
    rsx! {
        Panel { class: classes::PANEL,
            div { class: classes::PANEL_INNER,
                RequestIntakeCanvas {
                    ui: ui.clone(),
                    on_run_action: on_run_action,
                }
                GoalPlanCanvas {
                    ui: ui.clone(),
                    on_run_action: on_run_action,
                }
                BranchScopeLens { ui: ui.clone(), on_run_action: on_run_action }
                AssignGoalDialog { ui: ui.clone(), on_run_action: on_run_action }
                SpecImpactGraph { ui: ui.clone() }
                if let Some(preview) = ui.preview.clone().or_else(|| ui.selected_action_id.and_then(|id| ui.action_preview(id))) {
                    DetailDrawer {
                        title: preview.title.clone(),
                        body: preview.result_summary.clone(),
                        evidence: preview.evidence_summary.clone(),
                    }
                } else if let Some(action) = ui.selected_action() {
                    DetailDrawer {
                        title: crate::model::workbench_action_title(locale, action.id).to_string(),
                        body: crate::model::workbench_action_description(locale, action.id).to_string(),
                        evidence: if locale == Locale::Ja {
                            format!("{} の準備完了", action.evidence_kind.label())
                        } else {
                            format!("ready for {}", action.evidence_kind.label())
                        },
                    }
                } else {
                    EmptyState {
                        title: (if locale == Locale::Ja { "プレビュー未選択" } else { "No preview selected" }).to_string(),
                        body: (if locale == Locale::Ja { "コマンドを確認するか、結果をプレビューするにはパレットを開いてください。" } else { "Open the palette to inspect a command or preview the result." }).to_string()
                    }
                }
            }
        }
    }
}
