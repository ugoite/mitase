use super::{HelpTopic, Locale, UiCopy};
use crate::model::{PageSection, WorkbenchPage};

pub struct Japanese;
pub static JA: Japanese = Japanese;

impl UiCopy for Japanese {
    fn workspace_label(&self) -> &'static str {
        "ワークスペース"
    }
    fn branch_label(&self) -> &'static str {
        "ブランチ"
    }
    fn health_label(&self) -> &'static str {
        "状態"
    }
    fn palette_placeholder(&self) -> &'static str {
        "コマンド、タスク、Item を検索"
    }
    fn sidebar_title(&self) -> &'static str {
        "Workbench ナビゲーション"
    }
    fn help_label(&self) -> &'static str {
        "ヘルプ"
    }
    fn language_label(&self) -> &'static str {
        "言語"
    }
    fn language_name(&self, locale: Locale) -> &'static str {
        match locale {
            Locale::En => "英語",
            Locale::Ja => "日本語",
        }
    }
    fn page_title(&self, page: WorkbenchPage) -> &'static str {
        match page {
            WorkbenchPage::Work => "Work",
            WorkbenchPage::Scope => "Scope",
            WorkbenchPage::Items => "Items",
            WorkbenchPage::Diagnostics => "Diagnostics",
            WorkbenchPage::Settings => "設定",
        }
    }
    fn page_summary(&self, page: WorkbenchPage) -> &'static str {
        match page {
            WorkbenchPage::Work => "実装作業を理解し、割り当て、証拠を確認します",
            WorkbenchPage::Scope => "コード・仕様・テストの境界と根拠を確認します",
            WorkbenchPage::Items => "仕様の正本を閲覧・編集します",
            WorkbenchPage::Diagnostics => "スコープと実行を信頼できるか診断します",
            WorkbenchPage::Settings => "ワークスペースを安全に設定します",
        }
    }
    fn section_title(&self, section: PageSection) -> &'static str {
        match section {
            PageSection::Brief => "概要",
            PageSection::WorkScope => "スコープ",
            PageSection::Delivery => "引き渡し",
            PageSection::Evidence => "証拠",
            PageSection::CodeTests => "コードとテスト",
            PageSection::Feature => "機能",
            PageSection::Requirement => "要件",
            PageSection::Policy => "ポリシー",
            PageSection::Philosophy => "理念",
            PageSection::Workspace => "ワークスペース",
            PageSection::GoalPlan => "Goal Plan",
            PageSection::Trace => "トレース",
            PageSection::Repository => "リポジトリ",
            PageSection::General => "一般",
            PageSection::App => "アプリ",
            PageSection::SyuYaml => "syu.yaml",
            PageSection::Integrations => "連携",
        }
    }
    fn new_work(&self) -> &'static str {
        "+ 新しい Work"
    }
    fn search(&self) -> &'static str {
        "検索"
    }
    fn run_diagnostics(&self) -> &'static str {
        "診断"
    }
    fn help_body(&self, topic: HelpTopic) -> &'static str {
        match topic {
            HelpTopic::Palette => "候補を選ぶと該当ページへ移動し、必要な操作にフォーカスします。",
            HelpTopic::Sidebar => "4つの固定された Workbench ページを切り替えます。",
            HelpTopic::Work => "目的を先に読み、スコープ、引き渡し、証拠を確認します。",
            HelpTopic::Scope => "Implementation Slice と推論した境界の根拠を確認します。",
            HelpTopic::Items => "仕様 Item を管理し、Item 起点の Work を開始します。",
            HelpTopic::Diagnostics => "全診断を開始し、グループ別の構造化された結果を確認します。",
            HelpTopic::Settings => "適用前にワークスペース設定を検証し、差分を確認します。",
        }
    }
}
