use syu_workbench::{
    ActiveGoalState, ActiveRequestState, EvidenceEntry, GoalListState, WorkbenchAction,
    WorkbenchActionAvailability, WorkbenchActionId, WorkbenchActionMutability,
    WorkbenchActionRegistry, WorkbenchApiPayload, WorkbenchEvidenceKind, WorkbenchState,
};

#[derive(Debug, Clone, PartialEq)]
pub struct CommandPaletteEntry {
    pub action: WorkbenchAction,
    pub availability: WorkbenchActionAvailability,
    pub disabled_reason: Option<String>,
    pub matched_query: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WorkbenchActionRunPreview {
    pub action_id: WorkbenchActionId,
    pub title: String,
    pub result_summary: String,
    pub evidence_summary: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WorkspacePulseSummary {
    pub workspace: String,
    pub branch: String,
    pub health: String,
    pub available_actions: usize,
    pub recent_evidence: String,
    pub next_action: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WorkbenchUiState {
    pub payload: WorkbenchApiPayload,
    pub command_palette_open: bool,
    pub command_query: String,
    pub selected_action_id: Option<WorkbenchActionId>,
    pub preview: Option<WorkbenchActionRunPreview>,
}

impl WorkbenchUiState {
    pub fn from_state(state: WorkbenchState) -> Self {
        Self {
            payload: WorkbenchApiPayload::new(state),
            command_palette_open: true,
            command_query: String::new(),
            selected_action_id: None,
            preview: None,
        }
    }

    pub fn with_registry(state: WorkbenchState, registry: WorkbenchActionRegistry) -> Self {
        let availability = registry.availability(&state);
        Self {
            payload: WorkbenchApiPayload {
                state,
                actions: registry.actions().to_vec(),
                availability,
            },
            command_palette_open: true,
            command_query: String::new(),
            selected_action_id: None,
            preview: None,
        }
    }

    pub fn open_command_palette(&mut self) {
        self.command_palette_open = true;
    }

    pub fn close_command_palette(&mut self) {
        self.command_palette_open = false;
    }

    pub fn set_query(&mut self, query: impl Into<String>) {
        self.command_query = query.into();
    }

    pub fn select_action(
        &mut self,
        action_id: WorkbenchActionId,
    ) -> Option<WorkbenchActionRunPreview> {
        self.selected_action_id = Some(action_id);
        let preview = self.action_preview(action_id);
        self.preview = preview.clone();
        preview
    }

    pub fn visible_actions(&self) -> Vec<CommandPaletteEntry> {
        let query = self.command_query.trim().to_lowercase();
        self.payload
            .actions
            .iter()
            .cloned()
            .zip(self.payload.availability.iter().cloned())
            .filter_map(|(action, availability)| {
                let haystack = format!(
                    "{} {} {}",
                    action.id.label(),
                    action.title.to_lowercase(),
                    action.description.to_lowercase()
                );
                let matched_query = query.is_empty() || haystack.contains(&query);
                matched_query.then(|| CommandPaletteEntry {
                    disabled_reason: (!availability.available)
                        .then(|| availability_reason(&availability)),
                    action,
                    availability,
                    matched_query,
                })
            })
            .collect()
    }

    pub fn action_preview(
        &self,
        action_id: WorkbenchActionId,
    ) -> Option<WorkbenchActionRunPreview> {
        let action = self
            .payload
            .actions
            .iter()
            .find(|candidate| candidate.id == action_id)?;
        if action.mutability != WorkbenchActionMutability::ReadOnly {
            return None;
        }

        Some(WorkbenchActionRunPreview {
            action_id,
            title: action.title.clone(),
            result_summary: format!("Read-only action placeholder for {}", action.title),
            evidence_summary: format!(
                "Evidence placeholder for {} ({})",
                action.title,
                action.evidence_kind.label()
            ),
        })
    }

    pub fn run_read_only_action(
        &mut self,
        action_id: WorkbenchActionId,
    ) -> Option<WorkbenchActionRunPreview> {
        let preview = self.action_preview(action_id)?;
        self.selected_action_id = Some(action_id);
        self.preview = Some(preview.clone());
        Some(preview)
    }

    pub fn selected_action(&self) -> Option<&WorkbenchAction> {
        let selected = self.selected_action_id?;
        self.payload
            .actions
            .iter()
            .find(|action| action.id == selected)
    }

    pub fn disabled_reason(&self, action_id: WorkbenchActionId) -> Option<String> {
        self.payload
            .availability
            .iter()
            .find(|availability| availability.id == action_id)
            .and_then(|availability| {
                (!availability.available).then(|| availability_reason(availability))
            })
    }

    pub fn pulse_summary(&self) -> WorkspacePulseSummary {
        let workspace = self
            .payload
            .state
            .workspace
            .as_ref()
            .map(|workspace| workspace.workspace_root.display().to_string())
            .unwrap_or_else(|| "workspace not loaded".to_string());
        let branch = self
            .payload
            .state
            .workspace
            .as_ref()
            .and_then(|workspace| workspace.branch.clone())
            .unwrap_or_else(|| "no branch loaded".to_string());
        let health = self
            .payload
            .state
            .workspace
            .as_ref()
            .and_then(|workspace| workspace.validation_summary.clone())
            .unwrap_or_else(|| "health pending".to_string());
        let available_actions = self
            .payload
            .availability
            .iter()
            .filter(|availability| availability.available)
            .count();
        let recent_evidence = self
            .payload
            .state
            .evidence_timeline
            .entries
            .last()
            .map(evidence_summary)
            .unwrap_or_else(|| "evidence placeholder".to_string());
        let next_action = self
            .visible_actions()
            .into_iter()
            .find(|entry| entry.availability.available)
            .map(|entry| entry.action.title)
            .unwrap_or_else(|| "no suggested action".to_string());

        WorkspacePulseSummary {
            workspace,
            branch,
            health,
            available_actions,
            recent_evidence,
            next_action,
        }
    }
}

pub fn build_demo_state() -> WorkbenchUiState {
    let mut state = WorkbenchState {
        workspace: Some(syu_workbench::WorkspaceSnapshot {
            workspace_root: std::path::PathBuf::from("/workspace/syu"),
            spec_root: std::path::PathBuf::from("/workspace/syu/docs/syu"),
            branch: Some("issue-738-workbench-ui".to_string()),
            validation_summary: Some("green".to_string()),
        }),
        request: Some(ActiveRequestState::default()),
        goals: GoalListState {
            active: vec![ActiveGoalState {
                goal_id: "goal-1".to_string(),
                ..ActiveGoalState::default()
            }],
            selected_goal_id: Some("goal-1".to_string()),
        },
        ..WorkbenchState::default()
    };
    state.evidence_timeline.entries.push(EvidenceEntry {
        kind: WorkbenchEvidenceKind::ValidationReport,
        summary: "validation passed".to_string(),
        action_id: None,
    });
    let mut ui = WorkbenchUiState::from_state(state);
    ui.command_palette_open = true;
    ui.command_query = "goal".to_string();
    ui
}

fn availability_reason(availability: &WorkbenchActionAvailability) -> String {
    let missing = availability
        .missing_state
        .iter()
        .map(|state| state.label())
        .collect::<Vec<_>>();
    if missing.is_empty() {
        "available".to_string()
    } else {
        format!("disabled: missing {}", missing.join(", "))
    }
}

fn evidence_summary(entry: &EvidenceEntry) -> String {
    format!("{}: {}", entry.kind.label(), entry.summary)
}

#[cfg(test)]
mod tests {
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
}
