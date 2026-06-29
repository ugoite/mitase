use crate::i18n::{Locale, UiCopy, copy};
use syu_workbench::{
    WorkbenchActionId, WorkbenchActionRegistry, WorkbenchApiPayload, WorkbenchState,
};

mod commands;
mod navigation;
mod scope;

pub use commands::cli_command_catalog;
pub use navigation::*;
pub use scope::*;

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
            Self::Browse => "navigate",
            Self::Check => "check",
            Self::Plan => "plan",
            Self::Change => "change",
            Self::Operate => "operate",
            Self::Generate => "generate",
        }
    }
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
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceSettingsState {
    pub workspace_root: String,
    pub spec_root: String,
    pub bind: String,
    pub port: u16,
    pub strict_review: bool,
    pub raw_yaml: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WorkbenchUiState {
    pub payload: WorkbenchApiPayload,
    pub command_query: String,
    pub spec_query: String,
    pub spec_kind: String,
    pub spec_browser: Option<SpecBrowserModel>,
    pub item_edit_preview: Option<ItemEditPreview>,
    pub settings: Option<WorkspaceSettingsState>,
    pub locale: Locale,
}

impl WorkbenchUiState {
    pub fn from_state(state: WorkbenchState) -> Self {
        Self::with_registry(state, WorkbenchActionRegistry::default())
    }

    pub fn with_registry(state: WorkbenchState, registry: WorkbenchActionRegistry) -> Self {
        let availability = registry.availability(&state);
        Self {
            payload: WorkbenchApiPayload {
                state,
                actions: registry.actions().to_vec(),
                availability,
            },
            command_query: String::new(),
            spec_query: String::new(),
            spec_kind: String::new(),
            spec_browser: None,
            item_edit_preview: None,
            settings: None,
            locale: Locale::En,
        }
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
    pub fn set_locale(&mut self, locale: Locale) {
        self.locale = locale;
    }
    pub fn copy(&self) -> &'static dyn UiCopy {
        copy(self.locale)
    }

    pub fn pulse_summary(&self) -> WorkspacePulseSummary {
        let workspace = self
            .payload
            .state
            .workspace
            .as_ref()
            .map(|item| item.workspace_root.display().to_string())
            .unwrap_or_else(|| {
                localized(
                    self.locale,
                    "workspace not loaded",
                    "ワークスペース未読み込み",
                )
                .to_string()
            });
        let branch = self
            .payload
            .state
            .workspace
            .as_ref()
            .and_then(|item| item.branch.clone())
            .unwrap_or_else(|| {
                localized(self.locale, "branch unavailable", "ブランチ未取得").to_string()
            });
        let health = self
            .payload
            .state
            .workspace
            .as_ref()
            .and_then(|item| item.validation_summary.clone())
            .unwrap_or_else(|| {
                localized(self.locale, "validation pending", "検証待ち").to_string()
            });
        WorkspacePulseSummary {
            workspace,
            branch,
            health,
        }
    }
}

pub fn workbench_action_category(id: WorkbenchActionId) -> CommandCategory {
    use WorkbenchActionId::*;
    match id {
        ValidationRun | GoalCheck => CommandCategory::Check,
        RequestClassify | RequestScope | RequestPlan | GoalTestSelect | BranchScope
        | BranchInferGoal | SpecImpact | TraceRange | RelateRange | AssignmentPreview => {
            CommandCategory::Plan
        }
        RequestNew
        | RequestScaffold
        | AssignmentCreate
        | AssignmentCancel
        | AssignmentRecordManual => CommandCategory::Change,
        AssignmentRunDry | AssignmentRun | AgentRun | AssignmentCollectEvidence => {
            CommandCategory::Operate
        }
        HistoryShow => CommandCategory::Browse,
    }
}

pub fn workbench_action_title(locale: Locale, id: WorkbenchActionId) -> &'static str {
    use WorkbenchActionId::*;
    match (locale, id) {
        (Locale::Ja, RequestNew) => "新しい依頼を作成",
        (Locale::Ja, RequestClassify) => "依頼を分類",
        (Locale::Ja, RequestScope) => "依頼の範囲を抽出",
        (Locale::Ja, RequestScaffold) => "仕様変更を準備",
        (Locale::Ja, RequestPlan) => "Goal Plan を作成",
        (Locale::Ja, GoalTestSelect) => "必須テストを選択",
        (Locale::Ja, GoalCheck) => "Goal Plan を検証",
        (Locale::Ja, BranchScope) => "ブランチ範囲を分析",
        (Locale::Ja, BranchInferGoal) => "差分から Goal を推論",
        (Locale::Ja, SpecImpact) => "仕様影響を確認",
        (Locale::Ja, TraceRange) => "範囲をトレース",
        (Locale::Ja, RelateRange) => "関連を確認",
        (Locale::Ja, ValidationRun) => "ワークスペースを検証",
        (Locale::Ja, HistoryShow) => "実行履歴を表示",
        (Locale::Ja, AssignmentCreate) => "割り当てを作成",
        (Locale::Ja, AssignmentPreview) => "割り当てを確認",
        (Locale::Ja, AssignmentRunDry) => "ドライラン",
        (Locale::Ja, AssignmentRun) => "割り当てを実行",
        (Locale::Ja, AssignmentCancel) => "割り当てを取消",
        (Locale::Ja, AssignmentRecordManual) => "手動作業を記録",
        (Locale::Ja, AssignmentCollectEvidence) => "証拠を収集",
        (Locale::Ja, AgentRun) => "エージェントを実行",
        (_, RequestNew) => "Create request",
        (_, RequestClassify) => "Classify request",
        (_, RequestScope) => "Scope request",
        (_, RequestScaffold) => "Prepare spec changes",
        (_, RequestPlan) => "Create Goal Plan",
        (_, GoalTestSelect) => "Select required tests",
        (_, GoalCheck) => "Check Goal Plan",
        (_, BranchScope) => "Analyze branch scope",
        (_, BranchInferGoal) => "Infer Goal from diff",
        (_, SpecImpact) => "Inspect spec impact",
        (_, TraceRange) => "Trace range",
        (_, RelateRange) => "Inspect relationships",
        (_, ValidationRun) => "Validate workspace",
        (_, HistoryShow) => "Show execution history",
        (_, AssignmentCreate) => "Create assignment",
        (_, AssignmentPreview) => "Review assignment",
        (_, AssignmentRunDry) => "Run dry",
        (_, AssignmentRun) => "Run assignment",
        (_, AssignmentCancel) => "Cancel assignment",
        (_, AssignmentRecordManual) => "Record manual work",
        (_, AssignmentCollectEvidence) => "Collect evidence",
        (_, AgentRun) => "Run agent",
    }
}

pub fn workbench_action_description(locale: Locale, id: WorkbenchActionId) -> &'static str {
    use WorkbenchActionId::*;
    match (locale, id) {
        (Locale::Ja, HistoryShow) => "Work の Evidence に移動して対象タイムラインを表示します。",
        (Locale::Ja, ValidationRun | GoalCheck) => "Diagnostics に移動して診断を開始します。",
        (Locale::Ja, BranchScope | BranchInferGoal | SpecImpact | TraceRange | RelateRange) => {
            "Scope に移動して Implementation Slice と根拠を確認します。"
        }
        (
            Locale::Ja,
            AssignmentCreate
            | AssignmentPreview
            | AssignmentRunDry
            | AssignmentRun
            | AssignmentCancel
            | AssignmentRecordManual
            | AssignmentCollectEvidence
            | AgentRun,
        ) => "Work の引き渡し画面でスコープ、テスト、証拠を確認します。",
        (Locale::Ja, _) => "Work の該当コンポーネントへ移動して入力内容を確認します。",
        (_, HistoryShow) => "Move to Work Evidence and focus the relevant timeline.",
        (_, ValidationRun | GoalCheck) => "Move to Diagnostics and prepare the relevant check.",
        (_, BranchScope | BranchInferGoal | SpecImpact | TraceRange | RelateRange) => {
            "Move to Scope and review implementation slices with their evidence."
        }
        (
            _,
            AssignmentCreate
            | AssignmentPreview
            | AssignmentRunDry
            | AssignmentRun
            | AssignmentCancel
            | AssignmentRecordManual
            | AssignmentCollectEvidence
            | AgentRun,
        ) => "Move to Work Delivery to review scope, tests, and evidence.",
        (_, _) => "Move to the relevant Work component and review its input.",
    }
}

fn localized<'a>(locale: Locale, en: &'a str, ja: &'a str) -> &'a str {
    if locale == Locale::Ja { ja } else { en }
}
