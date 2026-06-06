use super::*;

#[component]
pub fn EvidenceTimeline(entries: Vec<EvidenceRecord>, goal_id: Option<String>) -> Element {
    let filtered_entries = scoped_evidence_entries(entries, goal_id.as_deref());

    rsx! {
        div { class: "space-y-3",
            if filtered_entries.is_empty() {
                EmptyState {
                    title: "Evidence timeline".to_string(),
                    body: "Append evidence by running goal checks, test selection, or validation.".to_string()
                }
            } else {
                for record in filtered_entries {
                    { render_evidence_timeline_record(record) }
                }
            }
        }
    }
}

pub(super) fn render_evidence_timeline_record(record: EvidenceRecord) -> Element {
    match record.kind {
        syu_workbench::WorkbenchEvidenceKind::ValidationReport => {
            rsx! { ValidationEvidenceView { record } }
        }
        syu_workbench::WorkbenchEvidenceKind::TaskTestSelectionPlan => {
            rsx! { TestEvidenceView { record } }
        }
        syu_workbench::WorkbenchEvidenceKind::BranchScopeReport
        | syu_workbench::WorkbenchEvidenceKind::SpecImpactReport => {
            rsx! { ScopeEvidenceView { record } }
        }
        syu_workbench::WorkbenchEvidenceKind::AgentRun
        | syu_workbench::WorkbenchEvidenceKind::JobState => {
            rsx! { AgentEvidenceView { record } }
        }
        syu_workbench::WorkbenchEvidenceKind::AssignmentState => {
            rsx! { ManualDecisionEvidenceView { record } }
        }
        _ => rsx! { EvidenceRecordCard { record } },
    }
}

#[component]
pub fn EvidencePanel(ui: WorkbenchUiState) -> Element {
    let active_goal = ui.payload.state.goals.active_goal().cloned();
    let goal_id = active_goal.as_ref().map(|goal| goal.goal_id.clone());
    let latest = latest_scoped_evidence(
        &ui.payload.state.evidence_timeline.entries,
        goal_id.as_deref(),
    );
    rsx! {
        Panel { class: classes::PANEL,
            div { class: classes::PANEL_INNER,
                div { class: classes::SECTION_HEADER,
                    h2 { class: classes::SECTION_TITLE, "Evidence Timeline" }
                    if let Some(goal) = &active_goal {
                        ScopeChip { label: format!("goal {}", goal.goal_id) }
                    } else {
                        ScopeChip { label: "workspace".to_string() }
                    }
                }
                if let Some(record) = latest {
                    EvidenceDetailDrawer { record }
                }
                div { class: classes::SECTION_BODY,
                    EvidenceTimeline {
                        entries: ui.payload.state.evidence_timeline.entries.clone(),
                        goal_id: goal_id.clone(),
                    }
                }
            }
        }
    }
}

pub(super) fn scoped_evidence_entries(
    entries: Vec<EvidenceRecord>,
    goal_id: Option<&str>,
) -> Vec<EvidenceRecord> {
    match goal_id {
        Some(goal_id) => entries
            .into_iter()
            .filter(|entry| entry.goal_id.as_deref() == Some(goal_id))
            .collect(),
        None => entries,
    }
}

pub(super) fn latest_scoped_evidence(
    entries: &[EvidenceRecord],
    goal_id: Option<&str>,
) -> Option<EvidenceRecord> {
    match goal_id {
        Some(goal_id) => entries
            .iter()
            .rev()
            .find(|entry| entry.goal_id.as_deref() == Some(goal_id))
            .cloned(),
        None => entries.last().cloned(),
    }
}
