#![allow(unused_braces)]

use crate::components::explorer::{EmptyDetail, PageHeader, page_href};
use crate::components::indicators::{IndicatorStatus, StatusCircle};
use crate::i18n::Locale;
use crate::model::{PageSection, SpecBrowserItem, WorkbenchPage, WorkbenchUiState};
use dioxus::prelude::*;

const TABS: [PageSection; 4] = [
    PageSection::Philosophy,
    PageSection::Policy,
    PageSection::Requirement,
    PageSection::Feature,
];

#[component]
pub fn ItemsPage(
    ui: WorkbenchUiState,
    section: Option<PageSection>,
    entity: Option<String>,
    focus_anchor: Option<String>,
) -> Element {
    let copy = ui.copy();
    let selected_tab = section
        .filter(|tab| TABS.contains(tab))
        .unwrap_or_else(|| section_from_kind(&ui.spec_kind).unwrap_or(PageSection::Requirement));
    let kind = kind_for_section(selected_tab);
    let query = ui.spec_query.trim().to_lowercase();
    let items = ui
        .spec_browser
        .as_ref()
        .map(|browser| {
            browser
                .sections
                .iter()
                .flat_map(|section| section.documents.iter())
                .flat_map(|document| document.items.iter())
                .filter(|item| item.kind == kind)
                .filter(|item| {
                    query.is_empty()
                        || item.id.to_lowercase().contains(&query)
                        || item.title.to_lowercase().contains(&query)
                        || item
                            .summary
                            .as_deref()
                            .is_some_and(|summary| summary.to_lowercase().contains(&query))
                })
                .cloned()
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let preview_draft_id = ui
        .item_edit_preview
        .as_ref()
        .map(|preview| preview.item_id.as_str());
    let draft = entity.as_deref() == Some("draft")
        || preview_draft_id.is_some_and(|id| entity.as_deref() == Some(id));
    let selected_id = if draft {
        Some("draft".to_string())
    } else {
        entity
            .or_else(|| {
                ui.spec_browser
                    .as_ref()
                    .and_then(|browser| browser.selected_item_id.clone())
            })
            .filter(|id| items.iter().any(|item| item.id == *id))
            .or_else(|| items.first().map(|item| item.id.clone()))
    };
    let selected_item = selected_id
        .as_ref()
        .and_then(|id| items.iter().find(|item| item.id == *id))
        .cloned();
    let focused_search = focus_anchor.as_deref() == Some("items-search");
    let focused_editor = focus_anchor.as_deref() == Some("item-editor");
    rsx! {
        PageHeader { kicker: "Workbench".to_string(), title: copy.page_title(WorkbenchPage::Items).to_string(), description: copy.page_summary(WorkbenchPage::Items).to_string(), actions: rsx! {} }
        form { class: "mb-4 flex gap-2", method: "get", action: "/", "data-command-target": "items-search", tabindex: "-1",
            input { type: "hidden", name: "page", value: "items" }
            input { type: "hidden", name: "section", value: "{selected_tab.slug()}" }
            input { type: "hidden", name: "lang", value: "{ui.locale.slug()}" }
            input { class: if focused_search { "min-w-0 flex-1 rounded-lg border-2 border-red-500 px-3 py-2 text-sm" } else { "min-w-0 flex-1 rounded-lg border border-slate-300 px-3 py-2 text-sm" }, name: "spec_query", value: "{ui.spec_query}", placeholder: if ui.locale == Locale::Ja { "Item を検索" } else { "Search Items" } }
            button { class: "rounded-lg bg-slate-950 px-4 py-2 text-sm font-semibold text-white", type: "submit", "{copy.search()}" }
        }
        nav { class: "mb-3 flex flex-wrap gap-1 border-b border-slate-200", "aria-label": "Item kinds", for tab in TABS { a { class: tab_class(tab == selected_tab), href: page_href(WorkbenchPage::Items, ui.locale, Some(tab), None, None), "{tab_icon(tab)} {copy.section_title(tab)}" } } }
        div { class: "grid items-start gap-3 lg:grid-cols-[18rem_minmax(0,1fr)]",
            aside { class: "rounded-lg border border-slate-200 bg-slate-50 p-2", "aria-label": "Items",
                div { class: "flex items-center justify-between px-2 py-2", span { class: "text-xs font-medium uppercase text-slate-500", "{copy.section_title(selected_tab)}" } a { class: "rounded-full border border-slate-300 bg-white px-2.5 py-1 text-xs", href: page_href(WorkbenchPage::Items, ui.locale, Some(selected_tab), Some("draft"), None), "+ Draft" } }
                if draft { div { class: "mb-1 rounded-lg border border-dashed border-slate-400 bg-white p-3", div { class: "flex items-center gap-2", StatusCircle { status: IndicatorStatus::Running, label: if ui.locale == Locale::Ja { "未保存の下書き".to_string() } else { "unsaved draft".to_string() }, count: None } strong { class: "text-sm", if ui.locale == Locale::Ja { "新しい Item" } else { "New Item" } } } } }
                for item in &items { a { class: if selected_id.as_deref() == Some(item.id.as_str()) { "mb-1 block rounded-lg bg-slate-950 p-3 text-white" } else { "mb-1 block rounded-lg p-3 hover:bg-white" }, href: page_href(WorkbenchPage::Items, ui.locale, Some(selected_tab), Some(&item.id), None), strong { class: "block text-sm", "{item.id}" } span { class: "mt-1 block text-xs opacity-70", "{item.title}" } } }
            }
            section { class: if focused_editor { "rounded-lg border-2 border-red-500 bg-slate-50 p-4" } else { "rounded-lg border border-slate-200 bg-slate-50 p-4" }, "data-command-target": "item-editor", tabindex: "-1",
                if draft { ItemEditor { ui: ui.clone(), section: selected_tab, item: None } } else if let Some(item) = selected_item { ItemDetail { ui: ui.clone(), section: selected_tab, item } } else { EmptyDetail { title: if ui.locale == Locale::Ja { "Item がありません".to_string() } else { "No Items in this layer".to_string() }, body: if ui.locale == Locale::Ja { "Draft を作成すると、既存 Item と同じ編集画面が開きます。".to_string() } else { "Create a draft to use the same editor as an existing Item.".to_string() } } }
            }
        }
    }
}

#[component]
fn ItemDetail(ui: WorkbenchUiState, section: PageSection, item: SpecBrowserItem) -> Element {
    rsx! {
        div { class: "flex flex-wrap items-start justify-between gap-3", div { p { class: "text-[10px] uppercase tracking-[0.2em] text-slate-400", "{ui.copy().section_title(section)} · source of truth" } h2 { class: "mt-1 text-lg font-semibold", "{item.id} · {item.title}" } } div { class: "flex flex-wrap gap-2", button { class: "rounded-lg border border-slate-300 bg-white px-3 py-2 text-xs font-semibold", type: "button", "data-create-work-from-item": "{item.id}", "data-work-lang": "{ui.locale.slug()}", if ui.locale == Locale::Ja { "この Item から Work を作成" } else { "Create Work from Item" } } a { class: "rounded-lg border border-slate-300 bg-white px-3 py-2 text-xs font-semibold", href: page_href(WorkbenchPage::Scope, ui.locale, Some(PageSection::CodeTests), Some(&item.id), None), if ui.locale == Locale::Ja { "実装範囲を調べる" } else { "Explore implementation scope" } } } }
        div { class: "mt-4 grid gap-3 md:grid-cols-2", ItemField { label: "ID".to_string(), value: item.id.clone() } ItemField { label: if ui.locale == Locale::Ja { "状態".to_string() } else { "Status".to_string() }, value: item.status.clone().unwrap_or_else(|| "—".to_string()) } ItemField { label: if ui.locale == Locale::Ja { "概要".to_string() } else { "Summary".to_string() }, value: item.summary.clone().unwrap_or_default() } ItemField { label: if ui.locale == Locale::Ja { "説明".to_string() } else { "Description".to_string() }, value: item.description.clone().unwrap_or_default() } }
        details { class: "mt-3 rounded-lg border border-slate-200 bg-white p-4", summary { class: "cursor-pointer text-sm font-semibold", if ui.locale == Locale::Ja { "編集" } else { "Edit Item" } } div { class: "mt-4", ItemEditor { ui: ui.clone(), section, item: Some(item.clone()) } } }
        details { class: "mt-3 rounded-lg border border-slate-200 bg-white p-4", summary { class: "cursor-pointer text-sm font-semibold", if ui.locale == Locale::Ja { "関連コード・テストと履歴" } else { "Related code, tests, and history" } } div { class: "mt-3 space-y-2 text-sm", for group in item.implementations.iter().chain(item.tests.iter()) { for reference in &group.references { p { "{reference.file}" } } } } }
    }
}

#[component]
fn ItemEditor(
    ui: WorkbenchUiState,
    section: PageSection,
    item: Option<SpecBrowserItem>,
) -> Element {
    let id = item
        .as_ref()
        .map(|item| item.id.clone())
        .unwrap_or_else(|| {
            ui.item_edit_preview
                .as_ref()
                .map(|preview| preview.item_id.clone())
                .unwrap_or_else(|| draft_id(section))
        });
    let title = item
        .as_ref()
        .map(|item| item.title.clone())
        .unwrap_or_default();
    let summary = item
        .as_ref()
        .and_then(|item| item.summary.clone())
        .unwrap_or_default();
    let description = item
        .as_ref()
        .and_then(|item| item.description.clone())
        .unwrap_or_default();
    let status = item
        .as_ref()
        .and_then(|item| item.status.clone())
        .unwrap_or_else(|| "planned".to_string());
    rsx! {
        div { class: "flex flex-wrap items-center justify-between gap-3", div { p { class: "text-[10px] uppercase tracking-[0.2em] text-slate-400", if item.is_none() { "New Item · editing" } else { "Item · editing" } } h2 { class: "mt-1 text-lg font-semibold", if item.is_none() { if ui.locale == Locale::Ja { "既存 Item と同じ Detail Canvas で新規作成" } else { "Create in the same Detail Canvas as existing Items" } } else { "{id}" } } } }
        form { class: "mt-4 grid gap-3", method: "post", action: "/run",
            input { type: "hidden", name: "page", value: "items" }
            input { type: "hidden", name: "section", value: "{section.slug()}" }
            input { type: "hidden", name: "entity", value: "{id}" }
            input { type: "hidden", name: "lang", value: "{ui.locale.slug()}" }
            if item.is_some() { input { type: "hidden", name: "item_edit", value: "{id}" } }
            div { class: "grid gap-3 md:grid-cols-2", label { class: "grid gap-1 text-xs uppercase tracking-wide text-slate-500", "ID" input { class: "rounded-lg border border-slate-300 bg-white px-3 py-2 text-sm normal-case text-slate-900", name: if item.is_none() { "item_edit" } else { "" }, value: "{id}", disabled: item.is_some(), required: true } } label { class: "grid gap-1 text-xs uppercase tracking-wide text-slate-500", if ui.locale == Locale::Ja { "状態" } else { "Status" } input { class: "rounded-lg border border-slate-300 bg-white px-3 py-2 text-sm normal-case text-slate-900", name: "status", value: "{status}" } } }
            label { class: "grid gap-1 text-xs uppercase tracking-wide text-slate-500", if ui.locale == Locale::Ja { "タイトル" } else { "Title" } input { class: "rounded-lg border border-slate-300 bg-white px-3 py-2 text-sm normal-case text-slate-900", name: "title", value: "{title}", required: true } }
            label { class: "grid gap-1 text-xs uppercase tracking-wide text-slate-500", if ui.locale == Locale::Ja { "概要" } else { "Summary" } textarea { class: "min-h-20 rounded-lg border border-slate-300 bg-white px-3 py-2 text-sm normal-case text-slate-900", name: "summary", "{summary}" } }
            label { class: "grid gap-1 text-xs uppercase tracking-wide text-slate-500", if ui.locale == Locale::Ja { "説明" } else { "Description" } textarea { class: "min-h-24 rounded-lg border border-slate-300 bg-white px-3 py-2 text-sm normal-case text-slate-900", name: "description", "{description}" } }
            div { class: "flex justify-end gap-2", button { class: "rounded-lg border border-slate-300 bg-white px-4 py-2 text-sm font-semibold", name: "item_edit_apply", value: "0", type: "submit", if ui.locale == Locale::Ja { "変更をプレビュー" } else { "Preview changes" } } }
        }
        if let Some(preview) = &ui.item_edit_preview { div { class: "mt-4 rounded-lg border border-slate-200 bg-white p-4", p { class: "text-sm", "{preview.message}" } pre { class: "mt-3 max-h-64 overflow-auto rounded bg-slate-950 p-3 text-xs text-slate-100", "{preview.diff}" } if !preview.apply_payload.is_empty() && !preview.applied { form { class: "mt-3 flex justify-end", method: "post", action: "/run", input { type: "hidden", name: "page", value: "items" } input { type: "hidden", name: "section", value: "{section.slug()}" } input { type: "hidden", name: "entity", value: "{id}" } input { type: "hidden", name: "lang", value: "{ui.locale.slug()}" } input { type: "hidden", name: "item_edit", value: "{id}" } input { type: "hidden", name: "item_edit_payload", value: "{preview.apply_payload}" } button { class: "rounded-lg bg-slate-950 px-4 py-2 text-sm font-semibold text-white", name: "item_edit_apply", value: "1", type: "submit", if ui.locale == Locale::Ja { "適用" } else { "Apply" } } } } } }
    }
}

#[component]
fn ItemField(label: String, value: String) -> Element {
    rsx! { div { class: "rounded-lg border border-slate-200 bg-white p-4", p { class: "text-[10px] uppercase tracking-[0.18em] text-slate-500", "{label}" } p { class: "mt-2 text-sm leading-6 text-slate-700", if value.is_empty() { "—" } else { "{value}" } } } }
}
fn section_from_kind(kind: &str) -> Option<PageSection> {
    match kind {
        "philosophy" => Some(PageSection::Philosophy),
        "policies" | "policy" => Some(PageSection::Policy),
        "requirements" | "requirement" => Some(PageSection::Requirement),
        "features" | "feature" => Some(PageSection::Feature),
        _ => None,
    }
}
fn kind_for_section(section: PageSection) -> &'static str {
    match section {
        PageSection::Philosophy => "philosophy",
        PageSection::Policy => "policies",
        PageSection::Requirement => "requirements",
        PageSection::Feature => "features",
        _ => "requirements",
    }
}
fn tab_icon(tab: PageSection) -> &'static str {
    match tab {
        PageSection::Philosophy => "○",
        PageSection::Policy => "◌",
        PageSection::Requirement => "□",
        PageSection::Feature => "◇",
        _ => "",
    }
}
fn tab_class(active: bool) -> &'static str {
    if active {
        "border-b-2 border-slate-950 px-3 py-2 text-sm font-semibold"
    } else {
        "px-3 py-2 text-sm font-semibold text-slate-500"
    }
}
fn draft_id(section: PageSection) -> String {
    match section {
        PageSection::Philosophy => "PHIL-NEW-001",
        PageSection::Policy => "POL-NEW-001",
        PageSection::Requirement => "REQ-NEW-001",
        PageSection::Feature => "FEAT-NEW-001",
        _ => "ITEM-NEW-001",
    }
    .to_string()
}
