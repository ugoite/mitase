#![allow(unused_braces)]

use crate::components::explorer::{PageHeader, page_href};
use crate::components::indicators::{IndicatorStatus, StatusCircle};
use crate::i18n::Locale;
use crate::model::{PageSection, WorkbenchPage, WorkbenchUiState};
use dioxus::prelude::*;

const TABS: [PageSection; 3] = [
    PageSection::General,
    PageSection::SyuYaml,
    PageSection::Integrations,
];

#[component]
pub fn SettingsPage(
    ui: WorkbenchUiState,
    section: Option<PageSection>,
    focus_anchor: Option<String>,
) -> Element {
    let copy = ui.copy();
    let tab = section
        .filter(|item| TABS.contains(item))
        .unwrap_or(PageSection::SyuYaml);
    let workspace = ui.payload.state.workspace.as_ref();
    let root = workspace
        .map(|item| item.workspace_root.display().to_string())
        .unwrap_or_else(|| ".".to_string());
    let spec_root = workspace
        .map(|item| item.spec_root.display().to_string())
        .unwrap_or_else(|| "docs/syu".to_string());
    let settings = ui.settings.clone();
    let bind = settings
        .as_ref()
        .map(|item| item.bind.clone())
        .unwrap_or_else(|| "127.0.0.1".to_string());
    let port = settings.as_ref().map(|item| item.port).unwrap_or(3000);
    let strict_review = settings.as_ref().is_some_and(|item| item.strict_review);
    let raw_yaml = settings
        .as_ref()
        .map(|item| item.raw_yaml.clone())
        .unwrap_or_default();
    let focused = focus_anchor.as_deref() == Some("workspace-configuration");
    rsx! {
        PageHeader { kicker: "Workbench utility".to_string(), title: copy.page_title(WorkbenchPage::Settings).to_string(), description: copy.page_summary(WorkbenchPage::Settings).to_string(), actions: rsx! { button { class: "rounded-lg border border-slate-300 bg-white px-3 py-2 text-sm font-semibold", type: "button", "data-settings-validate": "true", if ui.locale == Locale::Ja { "検証" } else { "Validate" } } button { class: "rounded-lg bg-slate-950 px-4 py-2 text-sm font-semibold text-white", type: "button", "data-settings-apply": "true", if ui.locale == Locale::Ja { "適用" } else { "Apply" } } } }
        select { class: "mb-4 min-w-80 rounded-lg border border-slate-300 bg-white px-3 py-2 text-sm", option { "Workspace: {root}" } }
        nav { class: "mb-3 flex flex-wrap gap-1 border-b border-slate-200", "aria-label": "Settings sections", for item in TABS { a { class: tab_class(item == tab), href: page_href(WorkbenchPage::Settings, ui.locale, Some(item), None, None), "{tab_icon(item)} {copy.section_title(item)}" } } }
        section { id: "workspace-configuration", class: if focused { "rounded-lg border-2 border-red-500 bg-slate-50 p-4" } else { "rounded-lg border border-slate-200 bg-slate-50 p-4" }, "data-command-target": "workspace-configuration", tabindex: "-1",
            match tab {
                PageSection::SyuYaml => rsx! { SyuYamlSettings { locale: ui.locale, spec_root, bind, port, strict_review, raw_yaml } },
                PageSection::General => rsx! { GeneralSettings { locale: ui.locale, root } },
                PageSection::Integrations => rsx! { IntegrationSettings { locale: ui.locale } },
                _ => rsx! {},
            }
        }
    }
}

#[component]
fn SyuYamlSettings(
    locale: Locale,
    spec_root: String,
    bind: String,
    port: u16,
    strict_review: bool,
    raw_yaml: String,
) -> Element {
    rsx! {
        div { class: "flex flex-wrap items-center justify-between gap-3", div { p { class: "text-[10px] uppercase tracking-[0.2em] text-slate-400", "Utility page" } h2 { class: "mt-1 text-lg font-semibold", "syu.yaml" } } div { class: "flex items-center gap-2", StatusCircle { status: IndicatorStatus::Warning, label: if locale == Locale::Ja { "未適用の変更".to_string() } else { "unapplied changes".to_string() }, count: None } span { class: "rounded-full border bg-white px-2.5 py-1 text-xs", "source preserving" } } }
        div { class: "mt-4 grid gap-3 lg:grid-cols-2",
            form { class: "rounded-lg border border-slate-200 bg-white p-4", "data-settings-form": "true", h3 { class: "text-xs font-medium uppercase tracking-wide text-slate-500", if locale == Locale::Ja { "構造化設定" } else { "Structured settings" } } div { class: "mt-3 grid gap-3", SettingField { label: "Spec root".to_string(), name: "spec_root".to_string(), value: spec_root } SettingField { label: "Workbench bind".to_string(), name: "bind".to_string(), value: bind } div { class: "grid gap-3 sm:grid-cols-2", SettingField { label: "Port".to_string(), name: "port".to_string(), value: port.to_string() } SettingField { label: "Strict review".to_string(), name: "strict_review".to_string(), value: strict_review.to_string() } } SettingField { label: "Agent profile".to_string(), name: "agent".to_string(), value: "local-codex (dry run first)".to_string() } } details { class: "mt-3", summary { class: "cursor-pointer text-xs", if locale == Locale::Ja { "YAML を直接編集" } else { "Edit YAML directly" } } textarea { class: "mt-2 min-h-40 w-full rounded-lg border border-slate-300 p-3 font-mono text-xs", name: "raw_yaml", readonly: true, "{raw_yaml}" } } }
            div { class: "rounded-lg border border-slate-200 bg-white p-4", h3 { class: "text-xs font-medium uppercase tracking-wide text-slate-500", if locale == Locale::Ja { "差分プレビュー" } else { "Diff preview" } } pre { class: "mt-3 min-h-32 rounded-lg bg-slate-950 p-4 text-xs leading-5 text-slate-100", "data-settings-diff": "true", if locale == Locale::Ja { "検証すると source-preserving diff を表示します。" } else { "Validate to show the source-preserving diff." } } div { class: "mt-3 border-l-2 border-emerald-500 bg-emerald-50 p-3 text-sm text-emerald-800", "data-settings-message": "true", if locale == Locale::Ja { "既存コメントと未知のフィールドは保持されます。" } else { "Existing comments and unknown fields will be preserved." } } input { type: "hidden", "data-settings-source-hash": "true" } }
        }
    }
}

#[component]
fn GeneralSettings(locale: Locale, root: String) -> Element {
    rsx! { h2 { class: "text-lg font-semibold", if locale == Locale::Ja { "ワークスペース情報" } else { "Workspace information" } } div { class: "mt-4 grid gap-3 md:grid-cols-2", SettingSummary { label: if locale == Locale::Ja { "ルート".to_string() } else { "Root".to_string() }, value: root } SettingSummary { label: if locale == Locale::Ja { "接続".to_string() } else { "Connection".to_string() }, value: if locale == Locale::Ja { "接続済み".to_string() } else { "connected".to_string() } } } }
}
#[component]
fn IntegrationSettings(locale: Locale) -> Element {
    rsx! { h2 { class: "text-lg font-semibold", if locale == Locale::Ja { "リポジトリ・エージェント連携" } else { "Repository and agent integrations" } } p { class: "mt-2 text-sm text-slate-600", if locale == Locale::Ja { "安全に利用できるローカル連携だけを表示します。" } else { "Only integrations that can be configured safely are shown." } } div { class: "mt-4 rounded-lg border border-slate-200 bg-white p-4", StatusCircle { status: IndicatorStatus::Success, label: "local repository".to_string(), count: None } } }
}
#[component]
fn SettingField(label: String, name: String, value: String) -> Element {
    rsx! { label { class: "grid gap-1 text-[10px] uppercase tracking-[0.18em] text-slate-500", "{label}" input { class: "rounded-lg border border-slate-300 px-3 py-2 text-sm normal-case tracking-normal text-slate-900", name: "{name}", value: "{value}" } } }
}
#[component]
fn SettingSummary(label: String, value: String) -> Element {
    rsx! { div { class: "rounded-lg border border-slate-200 bg-white p-4", p { class: "text-[10px] uppercase tracking-[0.18em] text-slate-500", "{label}" } p { class: "mt-2 text-sm", "{value}" } } }
}
fn tab_class(active: bool) -> &'static str {
    if active {
        "border-b-2 border-slate-950 px-3 py-2 text-sm font-semibold"
    } else {
        "px-3 py-2 text-sm font-semibold text-slate-500"
    }
}
fn tab_icon(tab: PageSection) -> &'static str {
    match tab {
        PageSection::General => "⚙",
        PageSection::SyuYaml => "⌘",
        PageSection::Integrations => "↗",
        _ => "",
    }
}
