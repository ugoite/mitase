use crate::i18n::{Locale, UiCopy, copy};
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

pub fn command_category_label(locale: Locale, category: CommandCategory) -> &'static str {
    match locale {
        Locale::En => category.label(),
        Locale::Ja => match category {
            CommandCategory::Browse => "閲覧",
            CommandCategory::Check => "検証",
            CommandCategory::Plan => "計画",
            CommandCategory::Change => "変更",
            CommandCategory::Operate => "操作",
            CommandCategory::Generate => "生成",
        },
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

pub fn command_effect_label(locale: Locale, effect: CommandEffect) -> &'static str {
    match locale {
        Locale::En => effect.label(),
        Locale::Ja => match effect {
            CommandEffect::ReadOnly => "読み取り専用",
            CommandEffect::MutatesState => "状態を変更",
            CommandEffect::MutatesFiles => "ファイルを変更",
            CommandEffect::RuntimeProcess => "実行プロセス",
        },
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

pub fn command_result_status_label(locale: Locale, status: CommandResultStatus) -> &'static str {
    match locale {
        Locale::En => status.label(),
        Locale::Ja => match status {
            CommandResultStatus::Ready => "準備完了",
            CommandResultStatus::Pass => "成功",
            CommandResultStatus::Warn => "警告",
            CommandResultStatus::Fail => "失敗",
            CommandResultStatus::Pending => "保留",
        },
    }
}

pub fn workbench_action_title(locale: Locale, action_id: WorkbenchActionId) -> &'static str {
    match locale {
        Locale::En => match action_id {
            WorkbenchActionId::RequestNew => "New request",
            WorkbenchActionId::RequestClassify => "Classify request",
            WorkbenchActionId::RequestScope => "Scope request",
            WorkbenchActionId::RequestScaffold => "Scaffold request",
            WorkbenchActionId::RequestPlan => "Plan request",
            WorkbenchActionId::GoalTestSelect => "Select goal tests",
            WorkbenchActionId::GoalCheck => "Check goal",
            WorkbenchActionId::BranchScope => "Load branch scope",
            WorkbenchActionId::BranchInferGoal => "Infer goal from branch",
            WorkbenchActionId::SpecImpact => "Show spec impact",
            WorkbenchActionId::TraceRange => "Trace range",
            WorkbenchActionId::RelateRange => "Relate range",
            WorkbenchActionId::ValidationRun => "Run validation",
            WorkbenchActionId::HistoryShow => "Show history",
            WorkbenchActionId::AssignmentCreate => "Create assignment",
            WorkbenchActionId::AssignmentPreview => "Preview assignment",
            WorkbenchActionId::AssignmentRunDry => "Dry-run assignment",
            WorkbenchActionId::AssignmentRun => "Run assignment",
            WorkbenchActionId::AssignmentCancel => "Cancel assignment",
            WorkbenchActionId::AssignmentRecordManual => "Record manual assignment",
            WorkbenchActionId::AssignmentCollectEvidence => "Collect assignment evidence",
            WorkbenchActionId::AgentRun => "Run agent",
        },
        Locale::Ja => match action_id {
            WorkbenchActionId::RequestNew => "依頼を作成",
            WorkbenchActionId::RequestClassify => "依頼を分類",
            WorkbenchActionId::RequestScope => "依頼をスコープ",
            WorkbenchActionId::RequestScaffold => "依頼を雛形化",
            WorkbenchActionId::RequestPlan => "依頼を計画化",
            WorkbenchActionId::GoalTestSelect => "テストを選択",
            WorkbenchActionId::GoalCheck => "ゴールを検証",
            WorkbenchActionId::BranchScope => "ブランチ範囲を読む",
            WorkbenchActionId::BranchInferGoal => "ブランチからゴールを推定",
            WorkbenchActionId::SpecImpact => "仕様への影響を見る",
            WorkbenchActionId::TraceRange => "範囲を追跡",
            WorkbenchActionId::RelateRange => "範囲の関連を見る",
            WorkbenchActionId::ValidationRun => "検証を実行",
            WorkbenchActionId::HistoryShow => "履歴を見る",
            WorkbenchActionId::AssignmentCreate => "割り当てを作成",
            WorkbenchActionId::AssignmentPreview => "割り当てをプレビュー",
            WorkbenchActionId::AssignmentRunDry => "割り当てをドライラン",
            WorkbenchActionId::AssignmentRun => "割り当てを実行",
            WorkbenchActionId::AssignmentCancel => "割り当てを取消",
            WorkbenchActionId::AssignmentRecordManual => "手動割り当てを記録",
            WorkbenchActionId::AssignmentCollectEvidence => "割り当て証跡を収集",
            WorkbenchActionId::AgentRun => "エージェントを実行",
        },
    }
}

pub fn workbench_action_description(locale: Locale, action_id: WorkbenchActionId) -> &'static str {
    match locale {
        Locale::En => match action_id {
            WorkbenchActionId::RequestNew => {
                "Capture a new request artifact for the active workspace."
            }
            WorkbenchActionId::RequestClassify => {
                "Determine whether the active request is a create, change, or delete."
            }
            WorkbenchActionId::RequestScope => {
                "Map the active request to the relevant specification graph and impact area."
            }
            WorkbenchActionId::RequestScaffold => {
                "Preview the spec and file updates needed to realize the active request."
            }
            WorkbenchActionId::RequestPlan => "Turn the scoped request into a temporary Goal Plan.",
            WorkbenchActionId::GoalTestSelect => {
                "Choose the narrowest tests that cover the active Goal Plan."
            }
            WorkbenchActionId::GoalCheck => {
                "Compare the active Goal Plan against the current branch range."
            }
            WorkbenchActionId::BranchScope => {
                "Summarize the current branch scope and visible impact surface."
            }
            WorkbenchActionId::BranchInferGoal => {
                "Infer a Goal Plan from the current branch diff and tracked scope."
            }
            WorkbenchActionId::SpecImpact => {
                "Explain the likely specification impact of the current branch scope."
            }
            WorkbenchActionId::TraceRange => {
                "Trace changed files and symbols for the selected branch range."
            }
            WorkbenchActionId::RelateRange => {
                "Relate the selected branch range to affected specs and tests."
            }
            WorkbenchActionId::ValidationRun => {
                "Run the repository validation pass for the active workspace."
            }
            WorkbenchActionId::HistoryShow => {
                "Show the evidence trail for the active request or goal."
            }
            WorkbenchActionId::AssignmentCreate => {
                "Assign the active goal to a human or AI with explicit scope and evidence."
            }
            WorkbenchActionId::AssignmentPreview => {
                "Inspect scoped assignment constraints, blockers, and prompt context."
            }
            WorkbenchActionId::AssignmentRunDry => {
                "Run the configured command adapter in dry-run mode and capture evidence."
            }
            WorkbenchActionId::AssignmentRun => {
                "Execute a scoped command adapter when full execution is explicitly enabled."
            }
            WorkbenchActionId::AssignmentCancel => {
                "Cancel the active assignment without running commands."
            }
            WorkbenchActionId::AssignmentRecordManual => {
                "Record a human assignment decision as evidence."
            }
            WorkbenchActionId::AssignmentCollectEvidence => {
                "Append runner output and scope guard results to the evidence timeline."
            }
            WorkbenchActionId::AgentRun => {
                "Launch an AI run against a bounded goal scope and assignment."
            }
        },
        Locale::Ja => match action_id {
            WorkbenchActionId::RequestNew => {
                "現在のワークスペースに新しい依頼アーティファクトを作成します。"
            }
            WorkbenchActionId::RequestClassify => {
                "現在の依頼が新規作成・変更・削除のどれかを判定します。"
            }
            WorkbenchActionId::RequestScope => {
                "現在の依頼を関連する仕様グラフと影響範囲に対応付けます。"
            }
            WorkbenchActionId::RequestScaffold => {
                "依頼を実現するために必要な仕様更新とファイル更新をプレビューします。"
            }
            WorkbenchActionId::RequestPlan => {
                "スコープ済み依頼から一時的な Goal Plan を作成します。"
            }
            WorkbenchActionId::GoalTestSelect => {
                "現在の Goal Plan を覆う最小限のテストを選びます。"
            }
            WorkbenchActionId::GoalCheck => "現在の Goal Plan をブランチ範囲と照合します。",
            WorkbenchActionId::BranchScope => "現在のブランチ範囲と見えている影響面を要約します。",
            WorkbenchActionId::BranchInferGoal => {
                "現在の diff と追跡済みスコープから Goal Plan を推定します。"
            }
            WorkbenchActionId::SpecImpact => {
                "現在のブランチ範囲が仕様に与えそうな影響を説明します。"
            }
            WorkbenchActionId::TraceRange => {
                "選択したブランチ範囲の変更ファイルとシンボルを追跡します。"
            }
            WorkbenchActionId::RelateRange => {
                "選択したブランチ範囲を影響する仕様とテストへ関連付けます。"
            }
            WorkbenchActionId::ValidationRun => {
                "現在のワークスペースに対してリポジトリ検証を実行します。"
            }
            WorkbenchActionId::HistoryShow => "現在の依頼やゴールの証跡を表示します。",
            WorkbenchActionId::AssignmentCreate => {
                "明示的なスコープと証跡を付けて、現在のゴールを人または AI に割り当てます。"
            }
            WorkbenchActionId::AssignmentPreview => {
                "スコープ済みの割り当て条件、障害、プロンプト文脈を確認します。"
            }
            WorkbenchActionId::AssignmentRunDry => {
                "設定済みのコマンドアダプタをドライランし、証跡を記録します。"
            }
            WorkbenchActionId::AssignmentRun => {
                "完全実行が明示的に有効なときに、スコープ済みコマンドアダプタを実行します。"
            }
            WorkbenchActionId::AssignmentCancel => {
                "コマンドを実行せずに現在の割り当てを取り消します。"
            }
            WorkbenchActionId::AssignmentRecordManual => {
                "人間による割り当て判断を証跡として記録します。"
            }
            WorkbenchActionId::AssignmentCollectEvidence => {
                "ランナーの出力とスコープガード結果を証跡タイムラインへ追加します。"
            }
            WorkbenchActionId::AgentRun => {
                "境界づけられたゴールスコープと割り当てに対して AI 実行を開始します。"
            }
        },
    }
}

pub fn workbench_state_requirement_label(
    locale: Locale,
    requirement: syu_workbench::WorkbenchStateRequirement,
) -> &'static str {
    match locale {
        Locale::En => requirement.label(),
        Locale::Ja => match requirement {
            syu_workbench::WorkbenchStateRequirement::WorkspaceLoaded => "workspace 読み込み済み",
            syu_workbench::WorkbenchStateRequirement::ActiveRequest => "アクティブな依頼",
            syu_workbench::WorkbenchStateRequirement::ActiveGoalPlan => "アクティブな Goal Plan",
            syu_workbench::WorkbenchStateRequirement::BranchScopeLoaded => {
                "ブランチ範囲読み込み済み"
            }
            syu_workbench::WorkbenchStateRequirement::AssignmentLoaded => "割り当て読み込み済み",
            syu_workbench::WorkbenchStateRequirement::ConfirmationMetadata => "確認メタデータ",
            syu_workbench::WorkbenchStateRequirement::BoundedScope => "境界づけられたスコープ",
        },
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemEditPreview {
    pub item_id: String,
    pub diff: String,
    pub apply_payload: String,
    pub applied: bool,
    pub message: String,
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
    pub spec_kind: String,
    pub command_category: Option<CommandCategory>,
    pub selected_action_id: Option<WorkbenchActionId>,
    pub selected_cli_command_id: Option<String>,
    pub preview: Option<WorkbenchActionRunPreview>,
    pub cli_preview: Option<CliCommandPreview>,
    pub spec_browser: Option<SpecBrowserModel>,
    pub item_edit_preview: Option<ItemEditPreview>,
    pub locale: Locale,
}

impl WorkbenchUiState {
    pub fn from_state(state: WorkbenchState) -> Self {
        Self {
            payload: WorkbenchApiPayload::new(state),
            command_palette_open: true,
            command_query: String::new(),
            spec_query: String::new(),
            spec_kind: String::new(),
            command_category: None,
            selected_action_id: None,
            selected_cli_command_id: None,
            preview: None,
            cli_preview: None,
            spec_browser: None,
            item_edit_preview: None,
            locale: Locale::En,
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
            spec_kind: String::new(),
            command_category: None,
            selected_action_id: None,
            selected_cli_command_id: None,
            preview: None,
            cli_preview: None,
            spec_browser: None,
            item_edit_preview: None,
            locale: Locale::En,
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

    pub fn set_spec_kind(&mut self, kind: impl Into<String>) {
        self.spec_kind = kind.into();
    }

    pub fn set_command_category(&mut self, category: Option<CommandCategory>) {
        self.command_category = category;
    }

    pub fn set_locale(&mut self, locale: Locale) {
        self.locale = locale;
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
        let locale = self.locale;
        let mut actions = self
            .payload
            .actions
            .iter()
            .cloned()
            .zip(self.payload.availability.iter().cloned())
            .filter_map(|(action, availability)| {
                let title = workbench_action_title(locale, action.id);
                let description = workbench_action_description(locale, action.id);
                let haystack = format!(
                    "{} {} {}",
                    action.id.label(),
                    title.to_lowercase(),
                    description.to_lowercase()
                );
                let matched_query = query.is_empty() || haystack.contains(&query);
                matched_query.then(|| CommandPaletteEntry {
                    disabled_reason: (!availability.available)
                        .then(|| availability_reason(locale, &availability)),
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
                workbench_action_title(locale, entry.action.id).to_string(),
            )
        });
        actions
    }

    pub fn suggested_actions(&self, limit: usize) -> Vec<CommandPaletteEntry> {
        let query = self.command_query.trim().to_lowercase();
        let mut actions = self.visible_actions();
        if query.is_empty() {
            actions.sort_by_key(|entry| {
                (
                    !entry.availability.available,
                    workbench_action_title(self.locale, entry.action.id).to_string(),
                )
            });
        }
        actions.into_iter().take(limit).collect()
    }

    pub fn visible_cli_commands(&self) -> Vec<CliCommandEntry> {
        let query = self.command_query.trim().to_lowercase();
        let mut commands = cli_command_catalog(self.locale)
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
        let command = cli_command_catalog(self.locale)
            .iter()
            .find(|command| command.id == command_id)?;
        let result_summary = cli_command_result_summary(self.locale, command);
        let evidence_summary = cli_command_evidence_summary(self.locale, command);
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
                self.locale,
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
        let title = workbench_action_title(self.locale, action_id).to_string();

        Some(WorkbenchActionRunPreview {
            action_id,
            title: title.clone(),
            result_summary: action_preview_summary(self.locale, &title),
            evidence_summary: action_preview_evidence_summary(self.locale).to_string(),
            category: workbench_action_category(action_id),
            effect: workbench_action_effect(action),
            result: pending_typed_result(
                self.locale,
                workbench_action_result_kind(action_id),
                action_preview_summary(self.locale, &title),
                action_preview_evidence_summary(self.locale).to_string(),
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
                (!availability.available).then(|| availability_reason(self.locale, availability))
            })
    }

    pub fn pulse_summary(&self) -> WorkspacePulseSummary {
        let workspace = self
            .payload
            .state
            .workspace
            .as_ref()
            .map(|workspace| workspace.workspace_root.display().to_string())
            .unwrap_or_else(|| workspace_not_loaded(self.locale).to_string());
        let branch = self
            .payload
            .state
            .workspace
            .as_ref()
            .and_then(|workspace| workspace.branch.clone())
            .unwrap_or_else(|| no_branch_loaded(self.locale).to_string());
        let health = self
            .payload
            .state
            .workspace
            .as_ref()
            .and_then(|workspace| workspace.validation_summary.clone())
            .unwrap_or_else(|| health_pending(self.locale).to_string());
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
            .map(|entry| evidence_summary(self.locale, entry))
            .unwrap_or_else(|| no_evidence_yet(self.locale).to_string());
        let next_action = self
            .visible_actions()
            .into_iter()
            .find(|entry| entry.availability.available)
            .map(|entry| workbench_action_title(self.locale, entry.action.id).to_string())
            .unwrap_or_else(|| nothing_to_open(self.locale).to_string());

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

fn availability_reason(locale: Locale, availability: &WorkbenchActionAvailability) -> String {
    let missing = availability
        .missing_state
        .iter()
        .map(|state| workbench_state_requirement_label(locale, *state))
        .collect::<Vec<_>>();
    if missing.is_empty() {
        available_label(locale).to_string()
    } else {
        disabled_missing_label(locale, &missing)
    }
}

fn evidence_summary(locale: Locale, entry: &EvidenceEntry) -> String {
    let kind = match locale {
        Locale::En => entry.kind.label(),
        Locale::Ja => match entry.kind {
            syu_workbench::WorkbenchEvidenceKind::RequestArtifact => "依頼アーティファクト",
            syu_workbench::WorkbenchEvidenceKind::ClassificationOutcome => "分類結果",
            syu_workbench::WorkbenchEvidenceKind::ScopeOutcome => "スコープ結果",
            syu_workbench::WorkbenchEvidenceKind::ScaffoldPlan => "雛形計画",
            syu_workbench::WorkbenchEvidenceKind::GoalPlanArtifact => "Goal Plan",
            syu_workbench::WorkbenchEvidenceKind::TaskTestSelectionPlan => "テスト選択計画",
            syu_workbench::WorkbenchEvidenceKind::GoalPlanCheckReport => "Goal Plan 検証",
            syu_workbench::WorkbenchEvidenceKind::BranchScopeReport => "ブランチ範囲",
            syu_workbench::WorkbenchEvidenceKind::SpecImpactReport => "仕様影響",
            syu_workbench::WorkbenchEvidenceKind::ValidationReport => "検証レポート",
            syu_workbench::WorkbenchEvidenceKind::HistoryResponse => "履歴応答",
            syu_workbench::WorkbenchEvidenceKind::AssignmentState => "割り当て状態",
            syu_workbench::WorkbenchEvidenceKind::JobState => "ジョブ状態",
            syu_workbench::WorkbenchEvidenceKind::AgentRun => "エージェント実行",
        },
    };
    format!("{kind}: {}", entry.summary)
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
    locale: Locale,
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
            title: not_run_yet(locale).to_string(),
            summary,
            detail,
            status: CommandResultStatus::Pending,
        }],
        diagnostics: None,
    }
}

fn action_preview_summary(locale: Locale, title: &str) -> String {
    match locale {
        Locale::En => format!("Preview opened for {}", title),
        Locale::Ja => format!("{title} のプレビューを開きました"),
    }
}

fn action_preview_evidence_summary(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Ready to review",
        Locale::Ja => "レビュー準備済み",
    }
}

fn cli_command_result_summary(locale: Locale, command: &CliCommandEntry) -> String {
    match locale {
        Locale::En => {
            if command.requires_input {
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
            }
        }
        Locale::Ja => {
            if command.requires_input {
                format!("{} の入力を確認してから実行します。", command.invocation)
            } else if command.mutates_files {
                format!(
                    "{} はファイルを書き込む前に確認が必要です。",
                    command.invocation
                )
            } else if command.opens_spec_browser {
                "下の構造化仕様を確認するか、コマンドを実行して端末出力を見てください。".to_string()
            } else {
                format!("{} 実行待ち", command.invocation)
            }
        }
    }
}

fn cli_command_evidence_summary(locale: Locale, command: &CliCommandEntry) -> String {
    match locale {
        Locale::En => {
            if command.mutates_files {
                "writes files".to_string()
            } else if command.requires_input {
                "input ready".to_string()
            } else {
                "read-only".to_string()
            }
        }
        Locale::Ja => {
            if command.mutates_files {
                "ファイルを書き込む".to_string()
            } else if command.requires_input {
                "入力待ち".to_string()
            } else {
                "読み取り専用".to_string()
            }
        }
    }
}

fn available_label(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "available",
        Locale::Ja => "利用可能",
    }
}

fn disabled_missing_label(locale: Locale, missing: &[&'static str]) -> String {
    match locale {
        Locale::En => format!("disabled: missing {}", missing.join(", ")),
        Locale::Ja => format!("無効: {} が不足", missing.join("、")),
    }
}

fn no_evidence_yet(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "no evidence yet",
        Locale::Ja => "まだ証跡はありません",
    }
}

fn no_branch_loaded(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "no branch loaded",
        Locale::Ja => "ブランチ未読み込み",
    }
}

fn workspace_not_loaded(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "workspace not loaded",
        Locale::Ja => "ワークスペース未読み込み",
    }
}

fn health_pending(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "health pending",
        Locale::Ja => "状態確認待ち",
    }
}

fn nothing_to_open(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "nothing to open",
        Locale::Ja => "開くものがありません",
    }
}

fn not_run_yet(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Not run yet",
        Locale::Ja => "未実行",
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
