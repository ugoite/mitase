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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandCategory {
    Browse,
    Check,
    Plan,
    Change,
    Operate,
    Generate,
}

impl CommandCategory {
    pub const ALL: [Self; 6] = [
        Self::Browse,
        Self::Check,
        Self::Plan,
        Self::Change,
        Self::Operate,
        Self::Generate,
    ];

    pub const fn slug(self) -> &'static str {
        match self {
            Self::Browse => "browse",
            Self::Check => "check",
            Self::Plan => "plan",
            Self::Change => "change",
            Self::Operate => "operate",
            Self::Generate => "generate",
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Browse => "Browse",
            Self::Check => "Check",
            Self::Plan => "Plan",
            Self::Change => "Change",
            Self::Operate => "Operate",
            Self::Generate => "Generate",
        }
    }

    pub fn from_slug(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|category| category.slug() == value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandEffect {
    ReadOnly,
    MutatesState,
    MutatesFiles,
    RuntimeProcess,
}

impl CommandEffect {
    pub const fn label(self) -> &'static str {
        match self {
            Self::ReadOnly => "read only",
            Self::MutatesState => "changes state",
            Self::MutatesFiles => "changes files",
            Self::RuntimeProcess => "runtime process",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandResultKind {
    ListDetail,
    CheckDetail,
    PlanDetail,
    ChangeDetail,
    OperationDetail,
    GeneratedArtifact,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandResultStatus {
    Ready,
    Pass,
    Warn,
    Fail,
    Pending,
}

impl CommandResultStatus {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::Pass => "pass",
            Self::Warn => "warn",
            Self::Fail => "fail",
            Self::Pending => "pending",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandResultItem {
    pub id: String,
    pub title: String,
    pub summary: String,
    pub detail: String,
    pub status: CommandResultStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypedCommandResult {
    pub kind: CommandResultKind,
    pub status: CommandResultStatus,
    pub summary: String,
    pub items: Vec<CommandResultItem>,
    pub diagnostics: Option<String>,
}

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
    pub category: CommandCategory,
    pub effect: CommandEffect,
    pub result: TypedCommandResult,
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

impl CliCommandEntry {
    pub fn category(self) -> CommandCategory {
        match self.id {
            "cli.audit" | "cli.doctor" | "cli.validate" | "cli.task.check" => {
                CommandCategory::Check
            }
            "cli.task.classify"
            | "cli.task.scope"
            | "cli.task.scaffold"
            | "cli.task.plan"
            | "cli.task.test_select"
            | "cli.task.infer" => CommandCategory::Plan,
            "cli.init" | "cli.add" => CommandCategory::Change,
            "cli.lsp" => CommandCategory::Operate,
            "cli.report" | "cli.completion" => CommandCategory::Generate,
            _ => CommandCategory::Browse,
        }
    }

    pub fn effect(self) -> CommandEffect {
        if self.id == "cli.lsp" {
            CommandEffect::RuntimeProcess
        } else if self.mutates_files {
            CommandEffect::MutatesFiles
        } else {
            CommandEffect::ReadOnly
        }
    }

    pub fn result_kind(self) -> CommandResultKind {
        category_result_kind(self.category())
    }
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
    pub category: CommandCategory,
    pub effect: CommandEffect,
    pub result: TypedCommandResult,
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
    pub spec_query: String,
    pub command_category: Option<CommandCategory>,
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
            spec_query: String::new(),
            command_category: None,
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
            spec_query: String::new(),
            command_category: None,
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

    pub fn set_spec_query(&mut self, query: impl Into<String>) {
        self.spec_query = query.into();
    }

    pub fn set_command_category(&mut self, category: Option<CommandCategory>) {
        self.command_category = category;
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
            .filter(|entry| {
                self.command_category
                    .is_none_or(|category| workbench_action_category(entry.action.id) == category)
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
            .filter(|command| {
                self.command_category
                    .is_none_or(|category| command.category() == category)
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
            format!("Review the input for {}, then run it.", command.invocation)
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
            "input ready".to_string()
        } else {
            "read-only".to_string()
        };
        Some(CliCommandPreview {
            id: command.id.to_string(),
            title: command.title.to_string(),
            invocation: command.invocation.to_string(),
            result_summary: result_summary.clone(),
            evidence_summary: evidence_summary.clone(),
            requires_input: command.requires_input,
            mutates_files: command.mutates_files,
            category: command.category(),
            effect: command.effect(),
            result: pending_typed_result(
                command.result_kind(),
                result_summary,
                evidence_summary.clone(),
            ),
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
            category: workbench_action_category(action_id),
            effect: workbench_action_effect(action),
            result: pending_typed_result(
                workbench_action_result_kind(action_id),
                format!("Preview opened for {}", action.title),
                "Ready to review".to_string(),
            ),
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

pub fn category_result_kind(category: CommandCategory) -> CommandResultKind {
    match category {
        CommandCategory::Browse => CommandResultKind::ListDetail,
        CommandCategory::Check => CommandResultKind::CheckDetail,
        CommandCategory::Plan => CommandResultKind::PlanDetail,
        CommandCategory::Change => CommandResultKind::ChangeDetail,
        CommandCategory::Operate => CommandResultKind::OperationDetail,
        CommandCategory::Generate => CommandResultKind::GeneratedArtifact,
    }
}

pub fn pending_typed_result(
    kind: CommandResultKind,
    summary: String,
    detail: String,
) -> TypedCommandResult {
    TypedCommandResult {
        kind,
        status: CommandResultStatus::Pending,
        summary: summary.clone(),
        items: vec![CommandResultItem {
            id: "pending".to_string(),
            title: "Not run yet".to_string(),
            summary,
            detail,
            status: CommandResultStatus::Pending,
        }],
        diagnostics: None,
    }
}

pub fn workbench_action_category(action_id: WorkbenchActionId) -> CommandCategory {
    match action_id {
        WorkbenchActionId::GoalCheck | WorkbenchActionId::ValidationRun => CommandCategory::Check,
        WorkbenchActionId::RequestClassify
        | WorkbenchActionId::RequestScope
        | WorkbenchActionId::RequestScaffold
        | WorkbenchActionId::RequestPlan
        | WorkbenchActionId::GoalTestSelect
        | WorkbenchActionId::BranchInferGoal => CommandCategory::Plan,
        WorkbenchActionId::RequestNew
        | WorkbenchActionId::AssignmentCreate
        | WorkbenchActionId::AssignmentRecordManual => CommandCategory::Change,
        WorkbenchActionId::AssignmentRunDry
        | WorkbenchActionId::AssignmentRun
        | WorkbenchActionId::AssignmentCancel
        | WorkbenchActionId::AssignmentCollectEvidence
        | WorkbenchActionId::AgentRun => CommandCategory::Operate,
        _ => CommandCategory::Browse,
    }
}

pub fn workbench_action_effect(action: &WorkbenchAction) -> CommandEffect {
    match action.mutability {
        WorkbenchActionMutability::ReadOnly => CommandEffect::ReadOnly,
        WorkbenchActionMutability::MutatesState => CommandEffect::MutatesState,
        WorkbenchActionMutability::MutatesFiles
        | WorkbenchActionMutability::MutatesStateAndFiles => CommandEffect::MutatesFiles,
    }
}

pub fn workbench_action_result_kind(action_id: WorkbenchActionId) -> CommandResultKind {
    category_result_kind(workbench_action_category(action_id))
}

#[cfg(test)]
mod tests;
