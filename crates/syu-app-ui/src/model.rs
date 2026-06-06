use crate::i18n::{HelpTopic, Locale, UiCopy, copy};
use std::collections::BTreeMap;
use syu_task_model::{
    GoalPlanArtifact, GoalPlanCompletion, GoalPlanConfidence, GoalPlanCoverage,
    GoalPlanCoverageMode, GoalPlanGoal, GoalPlanImplementationPlan, GoalPlanPersistentItem,
    GoalPlanPersistentItems, GoalPlanScope, GoalPlanScopeInclude, GoalPlanSource,
    GoalPlanSpecMapping, GoalPlanSpecUpdates, GoalPlanTestPlan, RequestArtifact,
    RequestArtifactContext, RequestClassification, ScaffoldAction, ScaffoldPlan, ScaffoldUpdate,
    ScaffoldUpdateKind, ScopeFeatureCandidate, ScopeOutcome, ScopeSignals, SearchResult,
    TaskTestSelectionCommand, TaskTestSelectionEscalation, TaskTestSelectionPlan,
};
use syu_workbench::{
    ActiveGoalState, ActiveRequestState, AffectedSpecItem, BranchScopeEvidence, BranchScopeReport,
    ChangedFileReport, EvidenceCommand, EvidenceEntry, EvidenceRecord, EvidenceSeverity,
    EvidenceSource, EvidenceStatus, EvidenceSubject, GoalListState, OutOfScopeChange,
    OwnershipStatus, WorkbenchAction, WorkbenchActionAvailability, WorkbenchActionId,
    WorkbenchActionMutability, WorkbenchActionRegistry, WorkbenchApiPayload, WorkbenchEvidenceKind,
    WorkbenchState,
};

mod commands;
mod demo;

pub use commands::cli_command_catalog;
pub use demo::build_demo_state;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CliCommandEntry {
    pub id: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    pub invocation: &'static str,
    pub requires_input: bool,
    pub mutates_files: bool,
    pub opens_spec_browser: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CliCommandPreview {
    pub id: String,
    pub title: String,
    pub invocation: String,
    pub result_summary: String,
    pub evidence_summary: String,
    pub requires_input: bool,
    pub mutates_files: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SpecBrowserModel {
    pub sections: Vec<SpecBrowserSection>,
    pub selected_item_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SpecBrowserSection {
    pub label: String,
    pub documents: Vec<SpecBrowserDocument>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SpecBrowserDocument {
    pub path: String,
    pub title: String,
    pub folder_segments: Vec<String>,
    pub items: Vec<SpecBrowserItem>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SpecBrowserItem {
    pub kind: String,
    pub id: String,
    pub title: String,
    pub summary: Option<String>,
    pub description: Option<String>,
    pub product_design_principle: Option<String>,
    pub coding_guideline: Option<String>,
    pub priority: Option<String>,
    pub status: Option<String>,
    pub linked_philosophies: Vec<String>,
    pub linked_policies: Vec<String>,
    pub linked_requirements: Vec<String>,
    pub linked_features: Vec<String>,
    pub tests: Vec<SpecBrowserTraceGroup>,
    pub implementations: Vec<SpecBrowserTraceGroup>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SpecBrowserTraceGroup {
    pub language: String,
    pub references: Vec<SpecBrowserTraceReference>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SpecBrowserTraceReference {
    pub file: String,
    pub symbols: Vec<String>,
    pub doc_contains: Vec<String>,
    pub method: Option<String>,
    pub path: Option<String>,
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
    pub selected_cli_command_id: Option<String>,
    pub preview: Option<WorkbenchActionRunPreview>,
    pub cli_preview: Option<CliCommandPreview>,
    pub spec_browser: Option<SpecBrowserModel>,
    pub locale: Locale,
    pub help_topic: Option<HelpTopic>,
}

impl WorkbenchUiState {
    pub fn from_state(state: WorkbenchState) -> Self {
        Self {
            payload: WorkbenchApiPayload::new(state),
            command_palette_open: true,
            command_query: String::new(),
            selected_action_id: None,
            selected_cli_command_id: None,
            preview: None,
            cli_preview: None,
            spec_browser: None,
            locale: Locale::En,
            help_topic: None,
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
            selected_cli_command_id: None,
            preview: None,
            cli_preview: None,
            spec_browser: None,
            locale: Locale::En,
            help_topic: None,
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

    pub fn set_locale(&mut self, locale: Locale) {
        self.locale = locale;
    }

    pub fn set_help_topic(&mut self, help_topic: Option<HelpTopic>) {
        self.help_topic = help_topic;
    }

    pub fn copy(&self) -> &'static dyn UiCopy {
        copy(self.locale)
    }

    pub fn select_action(
        &mut self,
        action_id: WorkbenchActionId,
    ) -> Option<WorkbenchActionRunPreview> {
        self.selected_action_id = Some(action_id);
        self.selected_cli_command_id = None;
        self.cli_preview = None;
        let preview = self.action_preview(action_id);
        self.preview = None;
        preview
    }

    pub fn select_cli_command(
        &mut self,
        command_id: impl Into<String>,
    ) -> Option<CliCommandPreview> {
        let command_id = command_id.into();
        self.selected_action_id = None;
        self.preview = None;
        self.selected_cli_command_id = Some(command_id.clone());
        let preview = self.cli_command_preview(&command_id);
        self.cli_preview = preview.clone();
        preview
    }

    pub fn visible_actions(&self) -> Vec<CommandPaletteEntry> {
        let query = self.command_query.trim().to_lowercase();
        let mut actions = self
            .payload
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
            .collect::<Vec<_>>();
        actions.sort_by_key(|entry| {
            (
                !entry.availability.available,
                !entry.action.id.label().contains(query.as_str()),
                entry.action.title.clone(),
            )
        });
        actions
    }

    pub fn suggested_actions(&self, limit: usize) -> Vec<CommandPaletteEntry> {
        let query = self.command_query.trim().to_lowercase();
        let mut actions = self.visible_actions();
        if query.is_empty() {
            actions
                .sort_by_key(|entry| (!entry.availability.available, entry.action.title.clone()));
        }
        actions.into_iter().take(limit).collect()
    }

    pub fn visible_cli_commands(&self) -> Vec<CliCommandEntry> {
        let query = self.command_query.trim().to_lowercase();
        let mut commands = cli_command_catalog()
            .iter()
            .copied()
            .filter(|command| {
                let haystack = format!(
                    "{} {} {} {}",
                    command.id, command.title, command.description, command.invocation
                )
                .to_lowercase();
                query.is_empty() || haystack.contains(&query)
            })
            .collect::<Vec<_>>();
        commands.sort_by_key(|command| {
            (
                command.mutates_files,
                !command.id.contains(query.as_str()),
                command.title,
            )
        });
        commands
    }

    pub fn cli_command_preview(&self, command_id: &str) -> Option<CliCommandPreview> {
        let command = cli_command_catalog()
            .iter()
            .find(|command| command.id == command_id)?;
        let result_summary = if command.requires_input {
            format!(
                "Provide input for {} before running it.",
                command.invocation
            )
        } else if command.mutates_files {
            format!(
                "{} requires confirmation before it writes files.",
                command.invocation
            )
        } else if command.opens_spec_browser {
            "Browse the structured spec information below, or run the command for terminal output."
                .to_string()
        } else {
            format!("{} is ready to run.", command.invocation)
        };
        let evidence_summary = if command.mutates_files {
            "writes files".to_string()
        } else if command.requires_input {
            "input required".to_string()
        } else {
            "read-only".to_string()
        };
        Some(CliCommandPreview {
            id: command.id.to_string(),
            title: command.title.to_string(),
            invocation: command.invocation.to_string(),
            result_summary,
            evidence_summary,
            requires_input: command.requires_input,
            mutates_files: command.mutates_files,
        })
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
            result_summary: format!("Preview opened for {}", action.title),
            evidence_summary: "Ready to review".to_string(),
        })
    }

    pub fn run_read_only_action(
        &mut self,
        action_id: WorkbenchActionId,
    ) -> Option<WorkbenchActionRunPreview> {
        let preview = self.action_preview(action_id)?;
        self.selected_action_id = Some(action_id);
        self.selected_cli_command_id = None;
        self.cli_preview = None;
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
            .unwrap_or_else(|| "no evidence yet".to_string());
        let next_action = self
            .visible_actions()
            .into_iter()
            .find(|entry| entry.availability.available)
            .map(|entry| entry.action.title)
            .unwrap_or_else(|| "nothing to open".to_string());

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
mod tests;
