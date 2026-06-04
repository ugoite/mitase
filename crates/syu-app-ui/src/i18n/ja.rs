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

    fn palette_hint_active(&self) -> &'static str {
        "選択して確認"
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

    fn close_label(&self) -> &'static str {
        "閉じる"
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
            WorkbenchPane::Pulse => "ワークスペース",
            WorkbenchPane::Commands => "コマンドパレット",
            WorkbenchPane::Goals => "ゴール計画",
            WorkbenchPane::Request => "受付",
            WorkbenchPane::Branch => "ブランチ",
            WorkbenchPane::Assignment => "割り当て",
            WorkbenchPane::Graph => "仕様グラフ",
            WorkbenchPane::Evidence => "証跡",
        }
    }

    fn pane_summary(&self, pane: WorkbenchPane) -> &'static str {
        match pane {
            WorkbenchPane::Pulse => "ワークスペース、ブランチ、開くものをまとめて見る",
            WorkbenchPane::Commands => "上部の入力欄から操作を呼ぶ",
            WorkbenchPane::Goals => "いまのゴールと、その計画を見る",
            WorkbenchPane::Request => "受付中の依頼を見る",
            WorkbenchPane::Branch => "このブランチで変わったものを見る",
            WorkbenchPane::Assignment => "引き継ぎ先を見る",
            WorkbenchPane::Graph => "仕様・コード・テストのつながりを見る",
            WorkbenchPane::Evidence => "最新のイベントと出力を見る",
        }
    }

    fn help_title(&self, topic: HelpTopic) -> &'static str {
        match topic {
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

    fn command_surface_title(&self) -> &'static str {
        "コマンドパレット"
    }

    fn command_surface_body(&self) -> &'static str {
        "上部の入力欄にフォーカスすると候補が出ます。候補を選んで進めます。"
    }

    fn command_surface_chip_one(&self) -> &'static str {
        "フォーカス"
    }

    fn command_surface_chip_two(&self) -> &'static str {
        "絞り込み"
    }

    fn command_surface_chip_three(&self) -> &'static str {
        "実行"
    }
}
