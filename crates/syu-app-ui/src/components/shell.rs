use crate::components::explorer::page_href;
use crate::components::pages::{DiagnosticsPage, ItemsPage, ScopePage, SettingsPage, WorkPage};
use crate::i18n::Locale;
use crate::model::{
    CommandCategory, FocusIntent, PageSection, WorkbenchPage, WorkbenchUiState,
    cli_command_catalog, localized_target_title, target_for_action, target_for_command,
    workbench_action_category, workbench_action_description, workbench_action_title,
};
use dioxus::prelude::*;

#[component]
pub fn AppShell(
    ui: WorkbenchUiState,
    active_page: WorkbenchPage,
    #[props(default)] section: Option<PageSection>,
    #[props(default)] entity: Option<String>,
    #[props(default)] focus: Option<FocusIntent>,
    #[props(default)] sidebar_open: bool,
) -> Element {
    let _ = sidebar_open;
    let anchor = focus.map(focus_anchor).map(str::to_string);
    rsx! {
        div { class: "min-h-screen bg-slate-50 text-slate-950",
            StatusBar { ui: ui.clone(), active_page }
            div { class: "workbench-layout",
                WorkbenchSidebar { ui: ui.clone(), active_page }
                main { class: "min-w-0 p-4 lg:ml-64 lg:p-8",
                    div { class: "mx-auto max-w-[82rem] rounded-xl border border-slate-200 bg-white p-4 shadow-sm sm:p-5",
                        WorkbenchStage { ui, active_page, section, entity, focus_anchor: anchor }
                    }
                }
            }
        }
    }
}

#[component]
pub fn StatusBar(ui: WorkbenchUiState, active_page: WorkbenchPage) -> Element {
    let copy = ui.copy();
    rsx! {
        header { class: "sticky top-0 z-40 border-b border-slate-200 bg-white",
            div { class: "flex min-h-20 items-start gap-3 px-4 py-3 lg:ml-64 lg:px-8",
                div { class: "min-w-0 flex-1", CommandPalette { ui: ui.clone() } }
                a { class: "grid h-10 w-10 shrink-0 place-items-center rounded-full text-slate-500 hover:bg-slate-100", href: page_href(WorkbenchPage::Settings, ui.locale, Some(PageSection::SyuYaml), None, None), title: copy.page_title(WorkbenchPage::Settings), "aria-label": copy.page_title(WorkbenchPage::Settings), "⚙" }
            }
            div { class: "sr-only", "{copy.page_title(active_page)}" }
        }
    }
}

#[component]
pub fn WorkbenchSidebar(ui: WorkbenchUiState, active_page: WorkbenchPage) -> Element {
    let copy = ui.copy();
    let summary = ui.pulse_summary();
    rsx! {
        aside { class: "w-full border-b border-slate-200 bg-white lg:fixed lg:inset-y-0 lg:left-0 lg:z-50 lg:w-64 lg:border-b-0 lg:border-r",
            nav { class: "flex h-full flex-col px-4 py-4 lg:px-5 lg:py-5", "aria-label": copy.sidebar_title(),
                a { class: "mb-5 flex h-10 items-center px-3 text-sm font-semibold", href: page_href(WorkbenchPage::Work, ui.locale, None, None, None), "Syu" }
                ul { class: "space-y-1", for page in WorkbenchPage::ROLES { li { a { class: sidebar_class(page == active_page), href: page_href(page, ui.locale, None, None, None), span { class: "grid h-6 w-6 place-items-center text-base font-normal", "{page.icon()}" } span { "{copy.page_title(page)}" } } } } }
                div { class: "mt-auto hidden border-t border-slate-200 px-3 pt-4 text-xs text-slate-500 lg:block", strong { class: "block text-slate-800", "ugoite / syu" } p { class: "mt-1 truncate", "{copy.branch_label()}: {summary.branch}" } p { class: "mt-1 flex items-center gap-2", span { class: "h-2 w-2 rounded-full bg-emerald-500" } "workspace connected" } }
            }
        }
    }
}

#[component]
pub fn WorkbenchStage(
    ui: WorkbenchUiState,
    active_page: WorkbenchPage,
    section: Option<PageSection>,
    entity: Option<String>,
    focus_anchor: Option<String>,
) -> Element {
    match active_page {
        WorkbenchPage::Work => rsx! { WorkPage { ui, section, entity, focus_anchor } },
        WorkbenchPage::Scope => rsx! { ScopePage { ui, section, entity, focus_anchor } },
        WorkbenchPage::Items => rsx! { ItemsPage { ui, section, entity, focus_anchor } },
        WorkbenchPage::Diagnostics => {
            rsx! { DiagnosticsPage { ui, section, entity, focus_anchor } }
        }
        WorkbenchPage::Settings => rsx! { SettingsPage { ui, section, focus_anchor } },
    }
}

#[component]
pub fn CommandPalette(ui: WorkbenchUiState) -> Element {
    let copy = ui.copy();
    let mut entries = cli_command_catalog(ui.locale)
        .iter()
        .filter_map(|command| {
            let target = target_for_command(command.id)?;
            Some((
                command.id.to_string(),
                localized_target_title(ui.locale, command.id, command.title),
                command.description.to_string(),
                command.category(),
                target,
                None,
            ))
        })
        .collect::<Vec<_>>();
    for (action, availability) in ui
        .payload
        .actions
        .iter()
        .zip(ui.payload.availability.iter())
    {
        let disabled = (!availability.available).then(|| {
            if ui.locale == Locale::Ja {
                "必要な状態が不足しています".to_string()
            } else {
                "Required state is missing".to_string()
            }
        });
        entries.push((
            action.id.label().to_string(),
            workbench_action_title(ui.locale, action.id).to_string(),
            workbench_action_description(ui.locale, action.id).to_string(),
            workbench_action_category(action.id),
            target_for_action(action.id),
            disabled,
        ));
    }
    entries.sort_by(|left, right| left.1.cmp(&right.1));
    rsx! {
        div { class: "relative", "data-command-palette": "true",
            label { class: "sr-only", for: "command-palette-input", "{copy.palette_placeholder()}" }
            input { id: "command-palette-input", class: "w-full rounded-lg border border-slate-300 bg-white px-11 py-3 text-sm shadow-sm outline-none focus:border-slate-500 focus:ring-2 focus:ring-slate-200", value: "{ui.command_query}", placeholder: copy.palette_placeholder(), autocomplete: "off", "data-command-input": "true" }
            span { class: "pointer-events-none absolute left-4 top-3.5 text-slate-400", "⌘" }
            div { class: "mt-2 flex flex-wrap gap-1.5", for category in CommandCategory::ALL { span { class: if category == CommandCategory::Browse { "rounded-full bg-slate-950 px-3 py-1 text-[10px] uppercase tracking-[0.18em] text-white" } else { "rounded-full border border-slate-300 px-3 py-1 text-[10px] uppercase tracking-[0.18em] text-slate-500" }, "{category_label(category)}" } } }
            div { class: "command-palette-results absolute left-0 right-0 z-50 mt-2 hidden max-h-[28rem] overflow-auto rounded-xl border border-slate-200 bg-white p-2 shadow-2xl", role: "listbox",
                for (id, title, description, category, target, disabled) in entries {
                    if let Some(reason) = disabled { div { class: "rounded-lg px-3 py-3 opacity-50", "data-command-item": "true", "data-command-id": "{id}", "data-command-title": "{title}", "data-command-text": "{title} {description} {id}", strong { class: "block text-sm", "{title}" } p { class: "mt-1 text-xs text-slate-500", "{reason}" } } } else { a { class: "block rounded-lg px-3 py-3 hover:bg-slate-950 hover:text-white", href: target_href(&target, ui.locale), role: "option", "data-command-item": "true", "data-command-id": "{id}", "data-command-title": "{title}", "data-command-text": "{title} {description} {id}", div { class: "flex items-start justify-between gap-3", div { strong { class: "block text-sm", "{title}" } p { class: "mt-1 text-xs opacity-70", "{description}" } } span { class: "text-[10px] uppercase tracking-[0.18em] opacity-70", "{category_label(category)}" } } } }
                }
            }
        }
    }
}

fn target_href(target: &crate::model::CommandTarget, locale: Locale) -> String {
    page_href(
        target.page,
        locale,
        target.section,
        target.entity.as_deref(),
        Some(target.focus),
    )
}
fn focus_anchor(focus: FocusIntent) -> &'static str {
    match focus {
        FocusIntent::Search => "items-search",
        FocusIntent::Create => "item-editor",
        FocusIntent::Timeline => "evidence-timeline",
        FocusIntent::DiagnosticsRun => "diagnostics-run",
        FocusIntent::ScopeSelector => "scope-selector",
        FocusIntent::Assignment => "assignment",
        FocusIntent::Completion => "assignment",
        FocusIntent::Configuration => "workspace-configuration",
    }
}
fn sidebar_class(active: bool) -> &'static str {
    if active {
        "flex items-center gap-3 rounded-lg bg-slate-950 px-3 py-3 text-sm font-semibold text-white"
    } else {
        "flex items-center gap-3 rounded-lg px-3 py-3 text-sm font-semibold text-slate-700 hover:bg-slate-50"
    }
}
fn category_label(category: CommandCategory) -> &'static str {
    match category {
        CommandCategory::Browse => "navigate",
        CommandCategory::Check => "check",
        CommandCategory::Plan => "plan",
        CommandCategory::Change => "change",
        CommandCategory::Operate => "operate",
        CommandCategory::Generate => "generate",
    }
}
