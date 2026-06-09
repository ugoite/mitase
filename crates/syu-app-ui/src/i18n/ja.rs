use super::{HelpTopic, Locale, UiCopy};
use crate::WorkbenchPane;

pub struct Japanese;

pub static JA: Japanese = Japanese;

impl UiCopy for Japanese {
    fn app_title(&self) -> &'static str {
        "Syu Workbench"
    }

    fn app_tagline(&self) -> &'static str {
        "コマンドパレット中心の作業画面"
    }

    fn workspace_label(&self) -> &'static str {
        "ワークスペース"
    }

    fn branch_label(&self) -> &'static str {
        "ブランチ"
    }

    fn health_label(&self) -> &'static str {
        "状態"
    }

    fn actions_label(&self) -> &'static str {
        "候補"
    }

    fn palette_placeholder(&self) -> &'static str {
        "コマンドを入力"
    }

    fn palette_hint(&self) -> &'static str {
        "入力すると候補が出ます"
    }

    fn sidebar_title(&self) -> &'static str {
        "ナビゲーション"
    }

    fn sidebar_toggle_open(&self) -> &'static str {
        "サイドバーを表示"
    }

    fn sidebar_toggle_close(&self) -> &'static str {
        "サイドバーを隠す"
    }

    fn help_label(&self) -> &'static str {
        "ヘルプ"
    }

    fn language_label(&self) -> &'static str {
        "言語"
    }

    fn language_name(&self, locale: Locale) -> &'static str {
        match locale {
            Locale::En => "EN",
            Locale::Ja => "日本語",
        }
    }

    fn pane_title(&self, pane: WorkbenchPane) -> &'static str {
        match pane {
            WorkbenchPane::Items => "Item一覧",
            WorkbenchPane::Diagnostics => "診断",
            WorkbenchPane::Pulse => "作業",
            WorkbenchPane::Commands => "コマンドパレット",
            WorkbenchPane::Goals => "ゴール計画",
            WorkbenchPane::Request => "受付",
            WorkbenchPane::Branch => "スコープ",
            WorkbenchPane::Assignment => "割り当て",
            WorkbenchPane::Graph => "仕様グラフ",
            WorkbenchPane::Evidence => "証跡",
        }
    }

    fn pane_summary(&self, pane: WorkbenchPane) -> &'static str {
        match pane {
            WorkbenchPane::Items => "永続仕様を閲覧・編集する",
            WorkbenchPane::Diagnostics => "ワークスペースとゴールの検査を更新する",
            WorkbenchPane::Pulse => "依頼、ゴール、割り当て、証跡をまとめて見る",
            WorkbenchPane::Commands => "上部の入力欄から操作を呼ぶ",
            WorkbenchPane::Goals => "いまのゴールと、その計画を見る",
            WorkbenchPane::Request => "受付中の依頼を見る",
            WorkbenchPane::Branch => "ブランチ範囲と仕様への影響を見る",
            WorkbenchPane::Assignment => "引き継ぎ先を見る",
            WorkbenchPane::Graph => "仕様・コード・テストのつながりを見る",
            WorkbenchPane::Evidence => "最新のイベントと出力を見る",
        }
    }

    fn help_title(&self, topic: HelpTopic) -> &'static str {
        match topic {
            HelpTopic::Items => "Item一覧",
            HelpTopic::Diagnostics => "診断",
            HelpTopic::Palette => "コマンドパレット",
            HelpTopic::Sidebar => "サイドバー",
            HelpTopic::Pulse => "ワークスペース",
            HelpTopic::Goals => "ゴール計画",
            HelpTopic::Request => "受付",
            HelpTopic::Branch => "ブランチ",
            HelpTopic::Assignment => "割り当て",
            HelpTopic::Graph => "仕様グラフ",
            HelpTopic::Evidence => "証跡",
        }
    }

    fn help_body(&self, topic: HelpTopic) -> &'static str {
        match topic {
            HelpTopic::Items => {
                "階層ファイルツリーから仕様を閲覧し、リンクを辿って項目を管理します。"
            }
            HelpTopic::Diagnostics => {
                "ワークスペースとゴールの検査を実行し、問題のItemへ移動します。"
            }
            HelpTopic::Palette => "上部の入力欄にフォーカスして、少し入力すると候補が出ます。",
            HelpTopic::Sidebar => "左のサイドバーで画面を切り替えたり、折りたたんだりできます。",
            HelpTopic::Pulse => "ワークスペース、ブランチ、開くものをまとめて表示します。",
            HelpTopic::Goals => "現在のゴールと、そのための計画を表示します。",
            HelpTopic::Request => "受付テキスト、分類、開くスコープを表示します。",
            HelpTopic::Branch => "ブランチ範囲、変更ファイル、影響を表示します。",
            HelpTopic::Assignment => "引き継ぎ先と実行方法を表示します。",
            HelpTopic::Graph => "仕様・コード・テストのつながりを表示します。",
            HelpTopic::Evidence => "新しいイベントと出力を新しい順に表示します。",
        }
    }

    fn command_surface_body(&self) -> &'static str {
        "上部の入力欄にフォーカスすると候補が出ます。候補を選んで進めます。"
    }

    fn run_label(&self) -> &'static str {
        "実行"
    }

    fn running_label(&self) -> &'static str {
        "実行中..."
    }
}
