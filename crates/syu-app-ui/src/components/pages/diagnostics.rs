use crate::components::explorer::{PageHeader, page_href};
use crate::components::indicators::{IndicatorStatus, StatusCircle};
use crate::i18n::Locale;
use crate::model::{PageSection, WorkbenchPage, WorkbenchUiState};
use dioxus::prelude::*;
use syu_workbench::JobStatus;

const TABS: [PageSection; 4] = [
    PageSection::Workspace,
    PageSection::GoalPlan,
    PageSection::Trace,
    PageSection::Repository,
];

#[derive(Clone, PartialEq)]
struct DiagnosticCheck {
    id: &'static str,
    title_en: &'static str,
    title_ja: &'static str,
    summary_en: String,
    summary_ja: String,
    status: IndicatorStatus,
    detail_en: String,
    detail_ja: String,
    raw: Option<String>,
}

#[component]
pub fn DiagnosticsPage(
    ui: WorkbenchUiState,
    section: Option<PageSection>,
    entity: Option<String>,
    focus_anchor: Option<String>,
) -> Element {
    let copy = ui.copy();
    let tab = section
        .filter(|tab| TABS.contains(tab))
        .unwrap_or(PageSection::Workspace);
    let checks = checks_for(&ui, tab);
    let selected_id = entity
        .filter(|id| checks.iter().any(|check| check.id == *id))
        .or_else(|| checks.first().map(|check| check.id.to_string()));
    let selected = selected_id
        .as_ref()
        .and_then(|id| checks.iter().find(|check| check.id == *id))
        .cloned();
    let focused = focus_anchor.as_deref() == Some("diagnostics-run");
    rsx! {
        PageHeader { kicker: "Workbench".to_string(), title: copy.page_title(WorkbenchPage::Diagnostics).to_string(), description: copy.page_summary(WorkbenchPage::Diagnostics).to_string(), actions: rsx! { button { id: "diagnostics-run", class: if focused { "rounded-lg border-2 border-red-500 bg-slate-950 px-4 py-2 text-sm font-semibold text-white" } else { "rounded-lg bg-slate-950 px-4 py-2 text-sm font-semibold text-white" }, type: "button", "data-command-target": "diagnostics-run", "data-run-diagnostics": "true", "data-running-label": if ui.locale == Locale::Ja { "診断中…" } else { "Running…" }, "{copy.run_diagnostics()}" } } }
        div { class: "mb-4 flex gap-2", input { class: "min-w-0 flex-1 rounded-lg border border-slate-300 px-3 py-2 text-sm", placeholder: if ui.locale == Locale::Ja { "診断項目を検索" } else { "Search diagnostic checks" }, "data-diagnostics-filter": "true" } }
        nav { class: "mb-3 flex flex-wrap gap-1 border-b border-slate-200", "aria-label": "Diagnostic groups", for item in TABS { a { class: tab_class(item == tab), href: page_href(WorkbenchPage::Diagnostics, ui.locale, Some(item), None, None), "{tab_icon(item)} {copy.section_title(item)} " StatusCircle { status: aggregate_status(&ui, item), label: aggregate_label(ui.locale, item).to_string(), count: Some(error_count(&ui, item)) } } } }
        div { class: "grid items-start gap-3 lg:grid-cols-[18rem_minmax(0,1fr)]",
            aside { class: "rounded-lg border border-slate-200 bg-slate-50 p-2", "aria-label": "Diagnostic checks",
                div { class: "flex items-center justify-between px-2 py-2", span { class: "text-xs font-medium uppercase text-slate-500", "{copy.section_title(tab)} checks" } span { class: "rounded-full border bg-white px-2 py-0.5 text-xs", "{checks.len()}" } }
                for check in &checks { a { "data-diagnostic-check": "true", class: if selected_id.as_deref() == Some(check.id) { "mb-1 block rounded-lg bg-slate-950 p-3 text-white" } else { "mb-1 block rounded-lg p-3 hover:bg-white" }, href: page_href(WorkbenchPage::Diagnostics, ui.locale, Some(tab), Some(check.id), None), div { class: "flex items-start gap-2", StatusCircle { status: check.status, label: local(ui.locale, check.title_en, check.title_ja).to_string(), count: None } div { strong { class: "block text-sm", "{local(ui.locale, check.title_en, check.title_ja)}" } span { class: "mt-1 block text-xs opacity-70", "{local_owned(ui.locale, &check.summary_en, &check.summary_ja)}" } } } } }
            }
            section { class: "rounded-lg border border-slate-200 bg-slate-50 p-4",
                if let Some(check) = selected { DiagnosticDetail { ui: ui.clone(), check } }
            }
        }
    }
}

#[component]
fn DiagnosticDetail(ui: WorkbenchUiState, check: DiagnosticCheck) -> Element {
    rsx! {
        div { class: "flex flex-wrap items-start justify-between gap-3", div { p { class: "text-[10px] uppercase tracking-[0.2em] text-slate-400", "Diagnostic check · {check.id}" } h2 { class: "mt-1 text-lg font-semibold", "{local(ui.locale, check.title_en, check.title_ja)}" } } StatusCircle { status: check.status, label: local_owned(ui.locale, &check.summary_en, &check.summary_ja).to_string(), count: None } }
        div { class: status_callout(check.status), p { class: "text-sm leading-6", "{local_owned(ui.locale, &check.detail_en, &check.detail_ja)}" } }
        div { class: "mt-3 grid gap-3 md:grid-cols-2", section { class: "rounded-lg border border-slate-200 bg-white p-4", h3 { class: "text-[10px] uppercase tracking-[0.18em] text-slate-500", if ui.locale == Locale::Ja { "推奨する修正" } else { "Recommended action" } } ol { class: "mt-2 list-decimal space-y-1 pl-5 text-sm", li { if ui.locale == Locale::Ja { "対象と根拠を確認する" } else { "Review the affected subject and evidence" } } li { if ui.locale == Locale::Ja { "必要な修正を適用する" } else { "Apply the required correction" } } li { if ui.locale == Locale::Ja { "診断を再実行する" } else { "Run diagnostics again" } } } } section { class: "rounded-lg border border-slate-200 bg-white p-4", h3 { class: "text-[10px] uppercase tracking-[0.18em] text-slate-500", if ui.locale == Locale::Ja { "診断の根拠" } else { "Diagnostic evidence" } } p { class: "mt-2 text-sm text-slate-700", "{local_owned(ui.locale, &check.summary_en, &check.summary_ja)}" } } }
        if let Some(raw) = check.raw { details { class: "mt-3 rounded-lg border border-slate-200 bg-white p-4", summary { class: "cursor-pointer text-sm font-semibold", if ui.locale == Locale::Ja { "raw 診断を表示" } else { "Show raw diagnostics" } } pre { class: "mt-3 max-h-64 overflow-auto rounded bg-slate-950 p-3 text-xs text-slate-100", "{raw}" } } }
    }
}

fn checks_for(ui: &WorkbenchUiState, tab: PageSection) -> Vec<DiagnosticCheck> {
    let job = ui.payload.state.job.status;
    let running = matches!(job, JobStatus::Queued | JobStatus::Running);
    match tab {
        PageSection::Workspace => {
            let summary = ui
                .payload
                .state
                .workspace
                .as_ref()
                .and_then(|item| item.validation_summary.clone());
            let status = if running {
                IndicatorStatus::Running
            } else if summary.is_some() {
                IndicatorStatus::Success
            } else {
                IndicatorStatus::Disabled
            };
            vec![
                DiagnosticCheck { id: "reciprocal-links", title_en: "Reciprocal links", title_ja: "相互リンク", summary_en: summary.clone().unwrap_or_else(|| "not run".to_string()), summary_ja: summary.clone().unwrap_or_else(|| "未実行".to_string()), status, detail_en: "Checks adjacent specification links in both directions so Item-driven scope stays reliable.".to_string(), detail_ja: "隣接する仕様リンクを双方向に検査し、Item 起点のスコープを信頼できる状態にします。".to_string(), raw: None },
                DiagnosticCheck { id: "document-schema", title_en: "Document schema", title_ja: "文書スキーマ", summary_en: summary.clone().unwrap_or_else(|| "not run".to_string()), summary_ja: summary.clone().unwrap_or_else(|| "未実行".to_string()), status, detail_en: "Validates specification document structure and field types.".to_string(), detail_ja: "仕様文書の構造とフィールド型を検証します。".to_string(), raw: None },
                DiagnosticCheck { id: "generated-docs", title_en: "Generated docs", title_ja: "生成文書", summary_en: "freshness is checked by repository validation".to_string(), summary_ja: "リポジトリ検証で鮮度を確認".to_string(), status, detail_en: "Checks whether generated specification and report files match their sources.".to_string(), detail_ja: "生成された仕様文書とレポートがソースと一致するか確認します。".to_string(), raw: None },
                DiagnosticCheck { id: "workspace-validation", title_en: "Workspace validation", title_ja: "ワークスペース検証", summary_en: summary.clone().unwrap_or_else(|| "not run".to_string()), summary_ja: summary.unwrap_or_else(|| "未実行".to_string()), status, detail_en: "Checks the specification workspace and reports structural or semantic failures.".to_string(), detail_ja: "仕様ワークスペースを検査し、構造・意味上の問題を報告します。".to_string(), raw: None },
            ]
        }
        PageSection::GoalPlan => {
            let goal = ui.payload.state.goals.active_goal();
            let report = goal.and_then(|goal| goal.check_report.as_ref());
            vec![DiagnosticCheck { id: "goal-plan-check", title_en: "Goal Plan check", title_ja: "Goal Plan 検証", summary_en: report.map(|report| format!("{} errors, {} warnings", report.error_count(), report.warning_count())).unwrap_or_else(|| "no active result".to_string()), summary_ja: report.map(|report| format!("エラー {} 件、警告 {} 件", report.error_count(), report.warning_count())).unwrap_or_else(|| "結果なし".to_string()), status: if running { IndicatorStatus::Running } else if report.is_some_and(|report| report.error_count() > 0) { IndicatorStatus::Error } else if report.is_some_and(|report| report.warning_count() > 0) { IndicatorStatus::Warning } else if report.is_some() { IndicatorStatus::Success } else { IndicatorStatus::Disabled }, detail_en: "Compares the active Goal Plan with its branch range, required tests, and completion commands.".to_string(), detail_ja: "アクティブな Goal Plan をブランチ範囲、必須テスト、完了コマンドと照合します。".to_string(), raw: report.and_then(|report| serde_yaml::to_string(report).ok()) }]
        }
        PageSection::Trace => {
            let report = ui
                .payload
                .state
                .branch_scope
                .as_ref()
                .and_then(|state| state.report.as_ref());
            vec![DiagnosticCheck {
                id: "trace-ownership",
                title_en: "Trace ownership",
                title_ja: "トレース所有",
                summary_en: report
                    .map(|item| format!("{} unowned files", item.trace_ownership.unowned_files))
                    .unwrap_or_else(|| "not analyzed".to_string()),
                summary_ja: report
                    .map(|item| {
                        format!("所有未確認 {} ファイル", item.trace_ownership.unowned_files)
                    })
                    .unwrap_or_else(|| "未分析".to_string()),
                status: if report.is_some_and(|item| item.trace_ownership.unowned_files > 0) {
                    IndicatorStatus::Warning
                } else if report.is_some() {
                    IndicatorStatus::Success
                } else {
                    IndicatorStatus::Disabled
                },
                detail_en: "Explains whether changed code is owned by traced specification Items."
                    .to_string(),
                detail_ja: "変更コードが仕様 Item によって所有されているかを説明します。"
                    .to_string(),
                raw: report.and_then(|item| serde_yaml::to_string(item).ok()),
            }]
        }
        PageSection::Repository => vec![DiagnosticCheck {
            id: "repository-state",
            title_en: "Repository state",
            title_ja: "リポジトリ状態",
            summary_en: ui
                .payload
                .state
                .workspace
                .as_ref()
                .and_then(|item| item.branch.clone())
                .unwrap_or_else(|| "branch unavailable".to_string()),
            summary_ja: ui
                .payload
                .state
                .workspace
                .as_ref()
                .and_then(|item| item.branch.clone())
                .unwrap_or_else(|| "ブランチ未取得".to_string()),
            status: if ui.payload.state.workspace.is_some() {
                IndicatorStatus::Success
            } else {
                IndicatorStatus::Disabled
            },
            detail_en:
                "Checks repository connectivity, current branch, and generated documentation state."
                    .to_string(),
            detail_ja: "リポジトリ接続、現在のブランチ、生成文書の状態を確認します。".to_string(),
            raw: None,
        }],
        _ => Vec::new(),
    }
}

fn aggregate_status(ui: &WorkbenchUiState, tab: PageSection) -> IndicatorStatus {
    let checks = checks_for(ui, tab);
    if checks
        .iter()
        .any(|item| item.status == IndicatorStatus::Error)
    {
        IndicatorStatus::Error
    } else if checks
        .iter()
        .any(|item| item.status == IndicatorStatus::Warning)
    {
        IndicatorStatus::Warning
    } else if checks
        .iter()
        .any(|item| item.status == IndicatorStatus::Running)
    {
        IndicatorStatus::Running
    } else if checks
        .iter()
        .any(|item| item.status == IndicatorStatus::Success)
    {
        IndicatorStatus::Success
    } else {
        IndicatorStatus::Disabled
    }
}
fn error_count(ui: &WorkbenchUiState, tab: PageSection) -> usize {
    checks_for(ui, tab)
        .iter()
        .filter(|item| {
            matches!(
                item.status,
                IndicatorStatus::Error | IndicatorStatus::Warning
            )
        })
        .count()
}
fn aggregate_label(locale: Locale, tab: PageSection) -> &'static str {
    match (locale, tab) {
        (Locale::Ja, PageSection::Workspace) => "ワークスペース診断",
        (Locale::Ja, PageSection::GoalPlan) => "Goal Plan 診断",
        (Locale::Ja, PageSection::Trace) => "トレース診断",
        (Locale::Ja, PageSection::Repository) => "リポジトリ診断",
        (_, PageSection::Workspace) => "workspace diagnostics",
        (_, PageSection::GoalPlan) => "Goal Plan diagnostics",
        (_, PageSection::Trace) => "trace diagnostics",
        _ => "repository diagnostics",
    }
}
fn tab_class(active: bool) -> &'static str {
    if active {
        "flex items-center gap-2 border-b-2 border-slate-950 px-3 py-2 text-sm font-semibold"
    } else {
        "flex items-center gap-2 px-3 py-2 text-sm font-semibold text-slate-500"
    }
}
fn tab_icon(tab: PageSection) -> &'static str {
    match tab {
        PageSection::Workspace => "▣",
        PageSection::GoalPlan => "◎",
        PageSection::Trace => "↗",
        PageSection::Repository => "⌘",
        _ => "",
    }
}
fn status_callout(status: IndicatorStatus) -> &'static str {
    match status {
        IndicatorStatus::Error => "mt-4 border-l-2 border-red-500 bg-red-50 p-4 text-red-800",
        IndicatorStatus::Warning | IndicatorStatus::Inferred => {
            "mt-4 border-l-2 border-amber-500 bg-amber-50 p-4 text-amber-800"
        }
        IndicatorStatus::Success => {
            "mt-4 border-l-2 border-emerald-500 bg-emerald-50 p-4 text-emerald-800"
        }
        IndicatorStatus::Running => "mt-4 border-l-2 border-blue-500 bg-blue-50 p-4 text-blue-800",
        IndicatorStatus::Disabled => {
            "mt-4 border-l-2 border-slate-400 bg-slate-100 p-4 text-slate-700"
        }
    }
}
fn local<'a>(locale: Locale, en: &'a str, ja: &'a str) -> &'a str {
    if locale == Locale::Ja { ja } else { en }
}
fn local_owned<'a>(locale: Locale, en: &'a str, ja: &'a str) -> &'a str {
    if locale == Locale::Ja { ja } else { en }
}
