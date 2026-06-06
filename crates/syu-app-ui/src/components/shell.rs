use crate::components::{
    AgentEvidenceView, CommandItem, CommandOutputView, DetailDrawer, EmptyState, EvidenceBadge,
    EvidenceDetailDrawer, EvidenceRecordCard, GoalCard, ManualDecisionEvidenceView, Panel,
    ScopeChip, ScopeEvidenceView, StatusDot, TestEvidenceView, ValidationEvidenceView,
};
use crate::design::classes;
use crate::i18n::{HelpTopic, Locale};
use crate::model::{
    CliCommandEntry, CliCommandPreview, CommandCategory, CommandResultItem, CommandResultStatus,
    SpecBrowserItem, SpecBrowserModel, TypedCommandResult, WorkbenchUiState, WorkspacePulseSummary,
    workbench_action_category,
};
use dioxus::prelude::*;
use std::collections::HashMap;
use syu_task_model::{
    GoalPlanArtifact, GoalPlanConfidence, GoalPlanPersistentItem, GoalPlanScopeInclude,
    ScaffoldAction, ScaffoldUpdateKind,
};
use syu_workbench::{
    AgentRun, Assignee, AssigneeKind, Assignment, AssignmentStatus, EvidenceRecord, EvidenceSource,
    OwnershipStatus, ScopeGuardResult, ScopeGuardStatus, WorkbenchAction, WorkbenchActionId,
};

mod overview;
pub use overview::*;
mod command_palette;
pub use command_palette::*;
mod goals;
pub use goals::*;
mod assignment;
pub use assignment::*;
mod branch_scope;
pub use branch_scope::*;
mod request;
pub use request::*;
mod evidence;
pub use evidence::*;

const GRAPH_COLUMNS: usize = 4;
const GRAPH_COLUMN_WIDTH: i32 = 210;
const GRAPH_ROW_HEIGHT: i32 = 86;
const GRAPH_NODE_X: i32 = 95;
const GRAPH_NODE_Y: i32 = 56;
const GRAPH_NODE_WIDTH: i32 = 172;
const GRAPH_NODE_HEIGHT: i32 = 44;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkbenchPane {
    Pulse,
    Commands,
    Goals,
    Request,
    Branch,
    Assignment,
    Graph,
    Evidence,
}

#[allow(dead_code)]
impl WorkbenchPane {
    const ALL: [WorkbenchPane; 7] = [
        Self::Pulse,
        Self::Goals,
        Self::Request,
        Self::Branch,
        Self::Assignment,
        Self::Graph,
        Self::Evidence,
    ];

    pub fn slug(self) -> &'static str {
        match self {
            Self::Pulse => "pulse",
            Self::Commands => "commands",
            Self::Goals => "goals",
            Self::Request => "request",
            Self::Branch => "branch",
            Self::Assignment => "assignment",
            Self::Graph => "graph",
            Self::Evidence => "evidence",
        }
    }

    pub fn from_slug(value: &str) -> Option<Self> {
        match value {
            "pulse" => Some(Self::Pulse),
            "commands" | "palette" => Some(Self::Commands),
            "goals" => Some(Self::Goals),
            "request" => Some(Self::Request),
            "branch" => Some(Self::Branch),
            "assignment" => Some(Self::Assignment),
            "graph" => Some(Self::Graph),
            "evidence" => Some(Self::Evidence),
            _ => None,
        }
    }

    fn icon(self) -> &'static str {
        match self {
            Self::Pulse => "◌",
            Self::Commands => "⌘",
            Self::Goals => "◎",
            Self::Request => "↻",
            Self::Branch => "↗",
            Self::Assignment => "✦",
            Self::Graph => "◈",
            Self::Evidence => "⟡",
        }
    }
}

#[component]
pub fn AppShell(ui: WorkbenchUiState, active_pane: WorkbenchPane, sidebar_open: bool) -> Element {
    let _ = sidebar_open;
    rsx! {
        div { class: classes::APP_SHELL,
        div { class: classes::PAGE_FRAME,
                StatusBar {
                    ui: ui.clone(),
                    active_pane: active_pane,
                    sidebar_open: false,
                    palette: rsx! { CommandPalette { ui: ui.clone(), active_pane: active_pane } },
                }
                if let Some(help_topic) = ui.help_topic {
                    HelpPanel {
                        ui: ui.clone(),
                        active_pane: active_pane,
                        sidebar_open: false,
                        help_topic: help_topic,
                    }
                }
                div { class: classes::MAIN_GRID,
                    WorkbenchStage {
                        ui: ui.clone(),
                        active_pane: active_pane,
                    }
                }
            }
        }
    }
}

#[component]
pub fn StatusBar(
    ui: WorkbenchUiState,
    active_pane: WorkbenchPane,
    sidebar_open: bool,
    palette: Element,
) -> Element {
    let _ = sidebar_open;
    let summary = ui.pulse_summary();
    let copy = ui.copy();
    rsx! {
        header { class: "border-b border-border bg-panel",
            nav { class: "mx-auto flex max-w-7xl items-center justify-between gap-4 py-3", "aria-label": "Global",
                div { class: "flex lg:flex-1",
                    a { class: "-m-1.5 p-1.5 text-base font-semibold text-foreground", href: view_href(&ui, WorkbenchPane::Pulse, false, ui.locale, None),
                        span { class: "sr-only", "{copy.app_title()}" }
                        "Syu"
                    }
                }
                div { class: "hidden min-w-0 flex-1 lg:block", {palette.clone()} }
                div { class: "flex flex-1 justify-end",
                    details { class: "relative",
                        summary { class: "flex h-10 w-10 cursor-pointer list-none items-center justify-center rounded-full border border-border bg-background text-sm text-foreground/70 hover:bg-panel-muted", title: copy.language_label(),
                            "⚙"
                        }
                        div { class: "absolute right-0 z-30 mt-2 w-80 rounded-lg border border-border bg-panel p-3 shadow-lg",
                            div { class: "space-y-3",
                                div { class: "grid grid-cols-2 gap-2", "aria-label": copy.language_label(),
                                    a { class: language_button_class(ui.locale == Locale::En), href: view_href(&ui, active_pane, false, Locale::En, ui.help_topic), "EN" }
                                    a { class: language_button_class(ui.locale == Locale::Ja), href: view_href(&ui, active_pane, false, Locale::Ja, ui.help_topic), "日本語" }
                                }
                                SettingsRow { label: copy.workspace_label().to_string(), value: summary.workspace.clone() }
                                SettingsRow { label: copy.branch_label().to_string(), value: summary.branch.clone() }
                                SettingsRow { label: copy.health_label().to_string(), value: summary.health.clone() }
                                div { class: "text-[10px] uppercase tracking-[0.18em] text-foreground/45",
                                    "{copy.language_label()}: {copy.language_name(ui.locale)}"
                                }
                            }
                        }
                    }
                }
            }
            div { class: "pb-4 lg:hidden", {palette} }
            div { class: "sr-only",
                ScopeChip { label: format!("{} {}", summary.available_actions, copy.actions_label()) }
            }
        }
    }
}

#[component]
fn SettingsRow(label: String, value: String) -> Element {
    rsx! {
        div { class: "grid gap-1 text-xs text-foreground/55",
            span { class: "uppercase tracking-[0.18em]", "{label}" }
            p { class: "truncate rounded-lg border border-border bg-background px-3 py-2 text-sm text-foreground", "{value}" }
        }
    }
}

fn language_button_class(active: bool) -> &'static str {
    if active {
        "rounded-lg border border-foreground bg-foreground px-3 py-2 text-center text-xs font-medium text-background"
    } else {
        "rounded-lg border border-border bg-background px-3 py-2 text-center text-xs font-medium text-foreground/75 hover:bg-panel-muted"
    }
}

#[component]
fn HelpLink(
    ui: WorkbenchUiState,
    active_pane: WorkbenchPane,
    sidebar_open: bool,
    topic: HelpTopic,
) -> Element {
    let copy = ui.copy();
    rsx! {
        a {
            class: "inline-flex h-9 w-9 items-center justify-center rounded-full border border-border bg-panel-muted text-xs text-foreground/70 hover:bg-background",
            href: view_href(&ui, active_pane, sidebar_open, ui.locale, Some(topic)),
            title: copy.help_label(),
            "?"
        }
    }
}

#[component]
fn HelpPanel(
    ui: WorkbenchUiState,
    active_pane: WorkbenchPane,
    sidebar_open: bool,
    help_topic: HelpTopic,
) -> Element {
    let copy = ui.copy();
    rsx! {
        section { class: "rounded-2xl border border-border bg-panel p-4 shadow-[0_1px_2px_rgba(15,23,42,0.04)]",
            div { class: "flex items-start justify-between gap-3",
                div { class: "space-y-1",
                    p { class: "text-[10px] uppercase tracking-[0.24em] text-foreground/45", "{copy.help_label()}" }
                    h3 { class: "text-sm font-semibold text-foreground", "{copy.help_title(help_topic)}" }
                }
                a {
                    class: "inline-flex h-8 w-8 items-center justify-center rounded-full border border-border bg-panel-muted text-foreground/60 hover:bg-background",
                    href: view_href(&ui, active_pane, sidebar_open, ui.locale, None),
                    title: copy.close_label(),
                    "×"
                }
            }
            p { class: "mt-3 text-sm text-foreground/75", "{copy.help_body(help_topic)}" }
        }
    }
}

fn pane_help_topic(pane: WorkbenchPane) -> HelpTopic {
    match pane {
        WorkbenchPane::Pulse => HelpTopic::Pulse,
        WorkbenchPane::Commands => HelpTopic::Palette,
        WorkbenchPane::Goals => HelpTopic::Goals,
        WorkbenchPane::Request => HelpTopic::Request,
        WorkbenchPane::Branch => HelpTopic::Branch,
        WorkbenchPane::Assignment => HelpTopic::Assignment,
        WorkbenchPane::Graph => HelpTopic::Graph,
        WorkbenchPane::Evidence => HelpTopic::Evidence,
    }
}

fn view_href(
    ui: &WorkbenchUiState,
    pane: WorkbenchPane,
    sidebar_open: bool,
    locale: Locale,
    help_topic: Option<HelpTopic>,
) -> String {
    let mut params = vec![
        format!("pane={}", pane.slug()),
        format!("sidebar={}", if sidebar_open { "1" } else { "0" }),
        format!("lang={}", locale.slug()),
    ];
    if !ui.command_query.trim().is_empty() {
        params.push(format!("query={}", urlencoding::encode(&ui.command_query)));
    }
    if let Some(category) = ui.command_category {
        params.push(format!("category={}", category.slug()));
    }
    if let Some(action_id) = ui.selected_action_id {
        params.push(format!("action={}", action_id.label()));
    }
    if let Some(command_id) = ui.selected_cli_command_id.as_ref() {
        params.push(format!("cli={}", urlencoding::encode(command_id)));
    }
    if let Some(goal_id) = ui.payload.state.goals.selected_goal_id.as_ref() {
        params.push(format!("goal={}", urlencoding::encode(goal_id)));
    }
    if let Some(help_topic) = help_topic.or(ui.help_topic) {
        params.push(format!("help={}", help_topic.slug()));
    }
    format!("?{}", params.join("&"))
}

#[component]
pub fn WorkbenchSidebar(ui: WorkbenchUiState, active_pane: WorkbenchPane) -> Element {
    let copy = ui.copy();
    rsx! {
        aside { class: "w-full shrink-0 lg:w-72",
            nav { class: "rounded-2xl border border-border bg-panel p-3 shadow-[0_1px_2px_rgba(15,23,42,0.04),0_18px_36px_rgba(15,23,42,0.06)]",
                div { class: "flex items-center justify-between gap-3 px-1 pb-3",
                    p { class: "text-xs font-medium uppercase tracking-[0.24em] text-foreground/50", "{copy.sidebar_title()}" }
                    div { class: "flex items-center gap-2",
                        HelpLink { ui: ui.clone(), active_pane: active_pane, sidebar_open: true, topic: HelpTopic::Sidebar }
                        a {
                            class: "inline-flex h-8 w-8 items-center justify-center rounded-full border border-border bg-panel-muted text-foreground/60 hover:bg-background",
                            href: view_href(&ui, active_pane, true, ui.locale, ui.help_topic),
                            title: copy.sidebar_toggle_close(),
                            "◱"
                        }
                    }
                }
                ul { class: "space-y-1",
                    for pane in WorkbenchPane::ALL {
                        li {
                            SidebarPaneButton {
                                ui: ui.clone(),
                                pane,
                                active: pane == active_pane,
                                collapsed: false,
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub fn WorkbenchSidebarRail(ui: WorkbenchUiState, active_pane: WorkbenchPane) -> Element {
    let copy = ui.copy();
    rsx! {
        aside { class: "w-full shrink-0 lg:w-16",
            nav { class: "rounded-2xl border border-border bg-panel p-2 shadow-[0_1px_2px_rgba(15,23,42,0.04),0_18px_36px_rgba(15,23,42,0.06)]",
                a {
                    class: "mb-2 inline-flex h-10 w-10 items-center justify-center rounded-full border border-border bg-panel-muted text-foreground/60 hover:bg-background",
                    href: view_href(&ui, active_pane, false, ui.locale, ui.help_topic),
                    title: copy.sidebar_toggle_open(),
                    "☰"
                }
                ul { class: "space-y-1",
                    for pane in WorkbenchPane::ALL {
                        li {
                            SidebarPaneButton {
                                ui: ui.clone(),
                                pane,
                                active: pane == active_pane,
                                collapsed: true,
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn SidebarPaneButton(
    ui: WorkbenchUiState,
    pane: WorkbenchPane,
    active: bool,
    collapsed: bool,
) -> Element {
    let copy = ui.copy();
    let base = if active {
        "group flex w-full items-start gap-3 rounded-2xl border border-border bg-foreground/5 px-3 py-3 text-left text-foreground shadow-[0_0_0_1px_rgba(15,23,42,0.02)]"
    } else {
        "group flex w-full items-start gap-3 rounded-2xl border border-transparent bg-panel-muted px-3 py-3 text-left text-foreground/72 hover:border-border hover:bg-background"
    };
    rsx! {
        a {
            class: base,
            href: view_href(&ui, pane, !collapsed, ui.locale, ui.help_topic),
            title: copy.pane_summary(pane),
            span { class: "grid h-9 w-9 shrink-0 place-items-center rounded-full border border-border bg-background text-xs text-foreground/75 transition group-hover:bg-panel",
                "{pane.icon()}"
            }
            if !collapsed {
                span { class: "flex min-w-0 flex-1 flex-col",
                    span { class: "text-sm font-medium text-foreground", "{copy.pane_title(pane)}" }
                    span { class: "text-[10px] leading-4 tracking-[0.18em] text-foreground/45", "{copy.pane_summary(pane)}" }
                }
            }
        }
    }
}

#[component]
pub fn WorkbenchStage(ui: WorkbenchUiState, active_pane: WorkbenchPane) -> Element {
    let detail = selected_pane_detail(ui.clone(), active_pane);
    let action_preview = ui.preview.clone();
    let cli_preview = ui.cli_preview.clone().or_else(|| {
        ui.selected_cli_command_id
            .as_deref()
            .and_then(|id| ui.cli_command_preview(id))
    });
    let selected_action = ui.selected_action().cloned();
    let show_pane_detail = active_pane != WorkbenchPane::Commands
        || (ui.selected_cli_command_id.is_none() && ui.selected_action_id.is_none());
    rsx! {
        main { class: "min-w-0 flex-1",
            section { class: "rounded-lg border border-border bg-panel p-4 shadow-sm",
                div { class: "mb-4 flex items-center justify-between gap-3",
                    div { class: "min-w-0",
                        p { class: "text-xs uppercase text-foreground/45", "result" }
                        h1 { class: "truncate text-lg font-semibold text-foreground", "{cli_preview.as_ref().map(|command| command.title.as_str()).or_else(|| selected_action.as_ref().map(|action| action.title.as_str())).unwrap_or(ui.copy().pane_title(active_pane))}" }
                    }
                    a {
                        class: "inline-flex h-9 w-9 items-center justify-center rounded-lg border border-border bg-background text-xs text-foreground/70 hover:bg-panel-muted",
                        href: view_href(&ui, active_pane, false, ui.locale, Some(pane_help_topic(active_pane))),
                        title: ui.copy().help_label(),
                        "?"
                    }
                }
                if show_pane_detail {
                    {detail}
                }
                if let Some(preview) = cli_preview {
                    div { class: "mt-4",
                        if preview.category == CommandCategory::Browse && cli_command_opens_spec_browser(&preview.id) {
                            if let Some(browser) = ui.spec_browser.clone() {
                                SpecInfoBrowser {
                                    browser,
                                    query: ui.command_query.clone(),
                                    locale: ui.locale,
                                    command_id: preview.id.clone(),
                                    category: ui.command_category,
                                }
                            }
                        } else {
                            CliCommandResult { preview: preview.clone(), locale: ui.locale, query: ui.command_query.clone() }
                        }
                    }
                } else if let Some(preview) = action_preview {
                    div { class: "mt-4",
                        if let Some(action) = selected_action {
                            WorkbenchActionResult { action, locale: ui.locale, result: Some(preview.result.clone()), category: ui.command_category, query: ui.command_query.clone() }
                        } else {
                            TypedResultView { result: preview.result.clone(), category: preview.category }
                        }
                    }
                } else if let Some(action) = selected_action {
                    div { class: "mt-4",
                        WorkbenchActionResult { action, locale: ui.locale, result: None, category: ui.command_category, query: ui.command_query.clone() }
                    }
                }
            }
        }
    }
}

#[component]
fn WorkbenchActionResult(
    action: WorkbenchAction,
    locale: Locale,
    result: Option<TypedCommandResult>,
    category: Option<CommandCategory>,
    query: String,
) -> Element {
    let needs_input = workbench_action_needs_text_input(action.id.label());
    let needs_confirmation = action.mutability.requires_confirmation();
    let action_category = workbench_action_category(action.id);
    let category_param = category.map_or_else(String::new, |value| value.slug().to_string());
    rsx! {
        section { class: "space-y-4",
          div { class: classes::DRAWER,
            div { class: "flex items-center justify-between gap-3",
                h3 { class: "text-sm font-semibold", "{action.title}" }
                div { class: "flex gap-2",
                    ScopeChip { label: action_category.label().to_string() }
                    ScopeChip { label: if needs_confirmation { "confirm".to_string() } else { "ready".to_string() } }
                }
            }
            p { class: "mt-2 text-sm text-foreground/75", "{action.description}" }
            form { class: "mt-3 grid gap-2 sm:grid-cols-[minmax(0,1fr)_auto]", action: "/", method: "get",
                input { type: "hidden", name: "pane", value: "commands" }
                input { type: "hidden", name: "sidebar", value: "0" }
                input { type: "hidden", name: "lang", value: "{locale.slug()}" }
                input { type: "hidden", name: "action", value: "{action.id.label()}" }
                input { type: "hidden", name: "run", value: "1" }
                input { type: "hidden", name: "category", value: "{category_param}" }
                input { type: "hidden", name: "query", value: "{query}" }
                if needs_input {
                    input {
                        class: "min-w-0 rounded-lg border border-border bg-background px-3 py-2 text-sm outline-none focus:border-foreground/20",
                        name: "action_input",
                        placeholder: "request",
                        autocomplete: "off",
                    }
                } else {
                    input { type: "hidden", name: "action_input", value: "" }
                }
                if needs_confirmation {
                    label { class: "inline-flex items-center gap-2 rounded-lg border border-border bg-background px-3 py-2 text-xs text-foreground/70",
                        input { type: "checkbox", name: "action_confirm", value: "1" }
                        span { "confirm" }
                    }
                }
                button {
                    class: "rounded-lg border border-border bg-foreground px-3 py-2 text-sm font-medium text-background hover:bg-foreground/90",
                    type: "submit",
                    "Run"
                }
            }
          }
          if let Some(result) = result {
              TypedResultView { result, category: action_category }
          }
        }
    }
}

fn workbench_action_needs_text_input(action_id: &str) -> bool {
    matches!(
        action_id,
        "request.new"
            | "request.classify"
            | "request.scope"
            | "request.scaffold"
            | "request.plan"
            | "assignment.create"
    )
}

#[component]
fn CliCommandResult(preview: CliCommandPreview, locale: Locale, query: String) -> Element {
    let default_cli_arg = cli_input_placeholder(&preview.id);
    let needs_confirmation = preview.mutates_files;

    rsx! {
        section { class: "space-y-4",
          div { class: classes::DRAWER,
            div { class: "flex items-center justify-between gap-3",
                p { class: "text-[10px] uppercase tracking-[0.24em] text-foreground/45", "command" }
                div { class: "flex gap-2",
                    ScopeChip { label: preview.category.label().to_string() }
                    ScopeChip { label: preview.effect.label().to_string() }
                    ScopeChip { label: if needs_confirmation { "confirm".to_string() } else if preview.requires_input { "input".to_string() } else { "ready".to_string() } }
                }
            }
            div { class: "mt-3 rounded-lg border border-border bg-background p-3",
                p { class: "text-[10px] uppercase tracking-[0.24em] text-foreground/45", "execution" }
                p { class: "mt-1 break-all text-sm font-medium text-foreground", "{preview.invocation}" }
            }
            form { class: "mt-3 grid gap-2 sm:grid-cols-[minmax(0,1fr)_auto]", action: "/", method: "get",
                input { type: "hidden", name: "pane", value: "commands" }
                input { type: "hidden", name: "sidebar", value: "0" }
                input { type: "hidden", name: "lang", value: "{locale.slug()}" }
                input { type: "hidden", name: "cli", value: "{preview.id}" }
                input { type: "hidden", name: "run", value: "1" }
                input { type: "hidden", name: "category", value: "{preview.category.slug()}" }
                input { type: "hidden", name: "query", value: "{query}" }
                if preview.requires_input {
                    input {
                        class: "min-w-0 rounded-lg border border-border bg-background px-3 py-2 text-sm outline-none focus:border-foreground/20",
                        name: "cli_arg",
                        placeholder: "{default_cli_arg}",
                        value: "{default_cli_arg}",
                        autocomplete: "off",
                    }
                } else {
                    input { type: "hidden", name: "cli_arg", value: "" }
                }
                if preview.mutates_files {
                    label { class: "inline-flex items-center gap-2 rounded-lg border border-border bg-background px-3 py-2 text-xs text-foreground/70",
                        input { type: "checkbox", name: "cli_confirm", value: "1" }
                        span { "confirm" }
                    }
                }
                button {
                    class: "rounded-lg border border-border bg-foreground px-3 py-2 text-sm font-medium text-background hover:bg-foreground/90",
                    type: "submit",
                    "Run"
                }
            }
          }
          TypedResultView { result: preview.result.clone(), category: preview.category }
        }
    }
}

#[component]
fn TypedResultView(result: TypedCommandResult, category: CommandCategory) -> Element {
    let pass_count = result
        .items
        .iter()
        .filter(|item| item.status == CommandResultStatus::Pass)
        .count();
    let warn_count = result
        .items
        .iter()
        .filter(|item| item.status == CommandResultStatus::Warn)
        .count();
    let fail_count = result
        .items
        .iter()
        .filter(|item| item.status == CommandResultStatus::Fail)
        .count();
    rsx! {
        section {
            class: "rounded-lg border border-border bg-panel p-4 shadow-sm",
            "data-result-kind": format!("{:?}", result.kind),
            "data-category-layout": category.slug(),
            div { class: "mb-4 flex flex-wrap items-start justify-between gap-3",
                div { class: "min-w-0",
                    p { class: "text-[10px] uppercase tracking-[0.24em] text-foreground/45", "{category.label()} result" }
                    h3 { class: "mt-1 text-base font-semibold text-foreground", "{result.summary}" }
                }
                ScopeChip { label: result.status.label().to_string() }
            }
            ResultCategorySummary {
                category,
                result: result.clone(),
                pass_count,
                warn_count,
                fail_count,
            }
            div { class: "grid gap-3 lg:grid-cols-3", "data-result-grid": "true",
                nav {
                    class: "overflow-auto rounded-lg border border-border bg-background p-2",
                    style: "max-height: 30rem",
                    "aria-label": "Result items",
                    p { class: "px-2 pb-2 text-[10px] uppercase tracking-[0.2em] text-foreground/45", "{result_list_label(category)}" }
                    for (index, item) in result.items.iter().enumerate() {
                        ResultListItem { item: item.clone(), selected: index == 0 }
                    }
                }
                div {
                    class: "min-w-0 overflow-auto rounded-lg border border-border bg-background p-4",
                    style: "max-height: 36rem",
                    "data-result-detail-panel": "true",
                    if result.items.is_empty() {
                        EmptyState { title: "No result items".to_string(), body: result.summary.clone() }
                    } else {
                        for (index, item) in result.items.iter().enumerate() {
                            article {
                                "data-result-detail": item.id.clone(),
                                hidden: index != 0,
                                div { class: "flex items-start justify-between gap-3",
                                    div {
                                        p { class: "text-[10px] uppercase tracking-[0.24em] text-foreground/45", "{item.id}" }
                                        h4 { class: "mt-1 text-sm font-semibold text-foreground", "{item.title}" }
                                    }
                                    ScopeChip { label: item.status.label().to_string() }
                                }
                                p { class: "mt-3 text-sm text-foreground/70", "{item.summary}" }
                                pre { class: "mt-3 whitespace-pre-wrap break-words rounded-lg border border-border bg-panel-muted p-3 text-xs text-foreground/70", "{item.detail}" }
                            }
                        }
                    }
                    if let Some(diagnostics) = result.diagnostics {
                        details { class: "mt-4",
                            summary { class: "cursor-pointer text-xs font-medium text-foreground/60", "Diagnostics" }
                            pre { class: "mt-2 max-h-80 overflow-auto whitespace-pre-wrap break-words rounded-lg border border-border bg-panel-muted p-3 text-xs text-foreground/65", "{diagnostics}" }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn ResultCategorySummary(
    category: CommandCategory,
    result: TypedCommandResult,
    pass_count: usize,
    warn_count: usize,
    fail_count: usize,
) -> Element {
    match category {
        CommandCategory::Browse => rsx! {
            div { class: "mb-4 rounded-lg border border-border bg-background p-3", "data-browse-context": "true",
                p { class: "mb-1 text-[10px] uppercase tracking-[0.2em] text-foreground/45", "Search and context" }
                input {
                    class: "w-full rounded-lg border border-border bg-panel-muted px-3 py-2 text-sm text-foreground/70",
                    value: "{result.summary}",
                    readonly: true,
                    aria_label: "Browse result context",
                }
            }
        },
        CommandCategory::Check => rsx! {
            div { class: "mb-4 grid gap-2 sm:grid-cols-3", "data-check-summary": "true",
                MetricTile { label: "pass".to_string(), value: pass_count.to_string() }
                MetricTile { label: "warn".to_string(), value: warn_count.to_string() }
                MetricTile { label: "fail".to_string(), value: fail_count.to_string() }
            }
        },
        CommandCategory::Plan => rsx! {
            div { class: "mb-4 grid gap-2 sm:grid-cols-2", "data-plan-summary": "true",
                MetricTile { label: "proposal status".to_string(), value: result.status.label().to_string() }
                MetricTile { label: "generated proposals".to_string(), value: result.items.len().to_string() }
            }
        },
        CommandCategory::Change => rsx! {
            div { class: "mb-4 rounded-lg border border-border bg-background p-3", "data-change-summary": "true",
                p { class: "text-[10px] uppercase tracking-[0.2em] text-foreground/45", "Execution result" }
                p { class: "mt-1 text-sm text-foreground/70", "Review the applied workspace or Workbench state changes below." }
            }
        },
        CommandCategory::Operate => rsx! {
            div { class: "mb-4 grid gap-2 sm:grid-cols-2", "data-operation-summary": "true",
                MetricTile { label: "runtime status".to_string(), value: result.status.label().to_string() }
                MetricTile { label: "events".to_string(), value: result.items.len().to_string() }
            }
        },
        CommandCategory::Generate => rsx! {
            div { class: "mb-4 grid gap-2 sm:grid-cols-2", "data-generated-summary": "true",
                MetricTile { label: "artifact status".to_string(), value: result.status.label().to_string() }
                MetricTile { label: "generated artifacts".to_string(), value: result.items.len().to_string() }
            }
        },
    }
}

fn result_list_label(category: CommandCategory) -> &'static str {
    match category {
        CommandCategory::Browse => "Items",
        CommandCategory::Check => "Checks",
        CommandCategory::Plan => "Proposals",
        CommandCategory::Change => "Changes",
        CommandCategory::Operate => "Events",
        CommandCategory::Generate => "Artifacts",
    }
}

#[component]
fn ResultListItem(item: CommandResultItem, selected: bool) -> Element {
    rsx! {
        a {
            class: "mb-1 grid gap-1 rounded-md border border-transparent px-3 py-2 text-foreground hover:border-border hover:bg-panel-muted",
            href: "#",
            aria_current: if selected { "page" } else { "false" },
            "data-result-item": item.id.clone(),
            span { class: "text-xs font-medium", "{item.title}" }
            span { class: "text-[10px] uppercase tracking-[0.16em] opacity-65", "{item.status.label()}" }
        }
    }
}

fn cli_command_opens_spec_browser(command_id: &str) -> bool {
    crate::model::cli_command_catalog()
        .iter()
        .any(|command| command.id == command_id && command.opens_spec_browser)
}

#[component]
fn SpecInfoBrowser(
    browser: SpecBrowserModel,
    query: String,
    locale: Locale,
    command_id: String,
    category: Option<CommandCategory>,
) -> Element {
    let selected = selected_spec_item(&browser, &query);
    let category_value = category.map_or("browse", CommandCategory::slug);
    rsx! {
        section { class: "rounded-lg border border-border bg-panel p-4 shadow-sm", "data-category-layout": "browse",
            div { class: "mb-3 grid gap-3 lg:grid-cols-3", "data-spec-browser-grid": "true",
                form {
                    class: "flex items-end gap-2",
                    "data-spec-search": "true",
                    action: "/",
                    method: "get",
                    input { type: "hidden", name: "pane", value: "commands" }
                    input { type: "hidden", name: "sidebar", value: "0" }
                    input { type: "hidden", name: "lang", value: "{locale.slug()}" }
                    input { type: "hidden", name: "cli", value: "{command_id}" }
                    input { type: "hidden", name: "category", value: "{category_value}" }
                    div { class: "min-w-0 flex-1",
                    p { class: "mb-1 text-[10px] uppercase tracking-[0.24em] text-foreground/45", "Search specs" }
                    input {
                        class: "w-full rounded-lg border border-border bg-background px-3 py-2 text-sm outline-none focus:border-foreground/20",
                        name: "query",
                        value: "{query}",
                        placeholder: "Search specs",
                    }
                    }
                    button {
                        class: "rounded-lg border border-border bg-foreground px-3 py-2 text-sm font-medium text-background hover:bg-foreground/90",
                        type: "submit",
                        "Search"
                    }
                }
                div { class: "rounded-lg border border-border bg-background p-2",
                    p { class: "px-2 pb-2 text-[10px] uppercase tracking-[0.24em] text-foreground/45", "Spec tree" }
                    nav { class: "overflow-auto", style: "max-height: 30rem", "aria-label": "Spec tree",
                        for section in &browser.sections {
                            div { class: "mb-3 last:mb-0",
                                p { class: "px-2 pb-1 text-[10px] uppercase tracking-[0.24em] text-foreground/45", "{section.label}" }
                                for document in &section.documents {
                                    details { class: "group", open: true,
                                        summary { class: "cursor-pointer list-none rounded-md px-2 py-1 text-xs font-medium text-foreground/65 hover:bg-panel-muted",
                                            "{document.title}"
                                        }
                                        div { class: "ml-3 border-l border-border pl-2",
                                            for item in &document.items {
                                                a {
                                                    class: spec_tree_item_class(selected.as_ref().map(|selected| selected.id.as_str()) == Some(item.id.as_str())),
                                                    href: spec_item_href(&command_id, locale, category_value, &query, &item.id),
                                                    title: "{item.title}",
                                                    "data-spec-tree-item": "true",
                                                    "data-spec-text": format!("{} {} {}", item.id, item.title, item.description.clone().unwrap_or_default()),
                                                    span { class: "truncate text-xs font-medium", "{item.id}" }
                                                    span { class: "truncate text-[11px] text-foreground/55", "{item.title}" }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                div { class: "min-w-0", "data-spec-detail": "true",
                    if let Some(item) = selected {
                        SpecModelCard { item }
                    } else {
                        EmptyState { title: "No spec item".to_string(), body: "The workspace spec tree is empty or still loading." }
                    }
                }
            }
        }
    }
}

fn spec_item_href(
    command_id: &str,
    locale: Locale,
    category: &str,
    query: &str,
    item_id: &str,
) -> String {
    format!(
        "?pane=commands&sidebar=0&lang={}&cli={}&category={}&query={}&spec_item={}",
        locale.slug(),
        urlencoding::encode(command_id),
        urlencoding::encode(category),
        urlencoding::encode(query),
        urlencoding::encode(item_id),
    )
}

fn spec_tree_item_class(active: bool) -> &'static str {
    if active {
        "grid gap-0.5 rounded-md border border-foreground bg-foreground px-2 py-2 text-background"
    } else {
        "grid gap-0.5 rounded-md border border-transparent px-2 py-2 text-foreground hover:border-border hover:bg-panel-muted"
    }
}

fn selected_spec_item(browser: &SpecBrowserModel, query: &str) -> Option<SpecBrowserItem> {
    let needle = query.trim().to_lowercase();
    browser
        .sections
        .iter()
        .flat_map(|section| section.documents.iter())
        .flat_map(|document| document.items.iter())
        .find(|item| {
            browser.selected_item_id.as_deref() == Some(item.id.as_str())
                || (!needle.is_empty()
                    && format!(
                        "{} {} {}",
                        item.id,
                        item.title,
                        item.description.clone().unwrap_or_default()
                    )
                    .to_lowercase()
                    .contains(&needle))
        })
        .cloned()
        .or_else(|| {
            browser
                .sections
                .iter()
                .flat_map(|section| section.documents.iter())
                .flat_map(|document| document.items.iter())
                .next()
                .cloned()
        })
}

#[component]
fn SpecModelCard(item: SpecBrowserItem) -> Element {
    rsx! {
        article { class: "rounded-lg border border-border bg-background p-4",
            div { class: "flex flex-wrap items-start justify-between gap-3",
                div { class: "min-w-0",
                    p { class: "text-[10px] uppercase tracking-[0.24em] text-foreground/45", "{item.kind}" }
                    h3 { class: "mt-1 text-base font-semibold text-foreground", "{item.title}" }
                    p { class: "mt-1 break-all text-xs font-medium text-foreground/55", "{item.id}" }
                }
                if let Some(status) = item.status.clone() {
                    ScopeChip { label: status }
                }
            }
            if let Some(summary) = item.summary.clone().or(item.description.clone()) {
                p { class: "mt-4 text-sm leading-6 text-foreground/75", "{summary}" }
            }
            div { class: "mt-4 grid gap-3 md:grid-cols-2",
                if let Some(priority) = item.priority.clone() {
                    MetricTile { label: "priority".to_string(), value: priority }
                }
                if let Some(status) = item.status.clone() {
                    MetricTile { label: "status".to_string(), value: status }
                }
            }
            if let Some(principle) = item.product_design_principle.clone() {
                ModelCardSection { title: "Product principle".to_string(), body: principle }
            }
            if let Some(guideline) = item.coding_guideline.clone() {
                ModelCardSection { title: "Coding guideline".to_string(), body: guideline }
            }
            LinkList { label: "philosophies".to_string(), values: item.linked_philosophies.clone() }
            LinkList { label: "policies".to_string(), values: item.linked_policies.clone() }
            LinkList { label: "requirements".to_string(), values: item.linked_requirements.clone() }
            LinkList { label: "features".to_string(), values: item.linked_features.clone() }
            TraceGroups { label: "tests".to_string(), groups: item.tests.clone() }
            TraceGroups { label: "implementations".to_string(), groups: item.implementations.clone() }
        }
    }
}

#[component]
fn ModelCardSection(title: String, body: String) -> Element {
    rsx! {
        section { class: "mt-4 rounded-lg border border-border bg-panel p-3",
            p { class: "text-[10px] uppercase tracking-[0.24em] text-foreground/45", "{title}" }
            p { class: "mt-1 text-sm leading-6 text-foreground/75", "{body}" }
        }
    }
}

#[component]
fn LinkList(label: String, values: Vec<String>) -> Element {
    if values.is_empty() {
        return rsx! {};
    }
    rsx! {
        div { class: "mt-4",
            p { class: "text-[10px] uppercase tracking-[0.24em] text-foreground/45", "{label}" }
            div { class: "mt-2 flex flex-wrap gap-2",
                for value in values {
                    span { class: "rounded-md border border-border bg-panel px-2 py-1 text-xs font-medium text-foreground/70", "{value}" }
                }
            }
        }
    }
}

#[component]
fn TraceGroups(label: String, groups: Vec<crate::model::SpecBrowserTraceGroup>) -> Element {
    if groups.is_empty() {
        return rsx! {};
    }
    rsx! {
        div { class: "mt-4",
            p { class: "text-[10px] uppercase tracking-[0.24em] text-foreground/45", "{label}" }
            div { class: "mt-2 grid gap-2",
                for group in groups {
                    div { class: "rounded-lg border border-border bg-panel p-3",
                        p { class: "text-xs font-medium uppercase tracking-[0.18em] text-foreground/55", "{group.language}" }
                        for reference in group.references {
                            p { class: "mt-1 break-all text-xs text-foreground/70", "{reference.file}" }
                        }
                    }
                }
            }
        }
    }
}

fn cli_input_placeholder(command_id: &str) -> &'static str {
    match command_id {
        "cli.show" | "cli.log" => "REQ-WORKBENCH-001",
        "cli.search" => "workbench",
        "cli.explain" | "cli.relate" => "REQ-WORKBENCH-001",
        "cli.trace" => "src/main.rs",
        "cli.completion" => "zsh",
        "cli.add" => "requirement REQ-NEW-001",
        "cli.task.classify" | "cli.task.scope" | "cli.task.scaffold" | "cli.task.plan" => {
            "target/syu/workbench/request.yaml"
        }
        "cli.task.test_select" | "cli.task.check" => "target/syu/workbench/goal.yaml",
        _ => "value",
    }
}

fn selected_pane_detail(ui: WorkbenchUiState, active_pane: WorkbenchPane) -> Element {
    match active_pane {
        WorkbenchPane::Pulse => rsx! { WorkbenchPulse { summary: ui.pulse_summary() } },
        WorkbenchPane::Commands => rsx! { CommandSurfaceOverview { ui } },
        WorkbenchPane::Goals => rsx! { GoalsOverview { ui } },
        WorkbenchPane::Request => rsx! { RequestOverview { ui } },
        WorkbenchPane::Branch => rsx! { BranchOverview { ui } },
        WorkbenchPane::Assignment => rsx! { AssignmentOverview { ui } },
        WorkbenchPane::Graph => rsx! { GraphOverview { ui } },
        WorkbenchPane::Evidence => rsx! { EvidenceOverview { ui } },
    }
}

#[component]
pub fn PulseMetric(label: String, value: String) -> Element {
    rsx! {
        div { class: "rounded-xl border border-border bg-panel p-3",
            p { class: "text-xs uppercase tracking-[0.18em] text-foreground/60", "{label}" }
            p { class: "mt-1 text-sm", "{value}" }
        }
    }
}

fn goal_title(goal: &syu_workbench::ActiveGoalState) -> String {
    goal.goal_plan
        .as_ref()
        .map(|plan| plan.goal.title.clone())
        .unwrap_or_else(|| "Untitled goal".to_string())
}

#[component]
fn FlowActionButton(
    label: String,
    action_id: WorkbenchActionId,
    ui: WorkbenchUiState,
    onclick: Option<EventHandler<WorkbenchActionId>>,
) -> Element {
    let available = ui
        .payload
        .availability
        .iter()
        .find(|availability| availability.id == action_id)
        .is_some_and(|availability| availability.available);
    let class = if available {
        "rounded-xl border border-command-active bg-command-active px-3 py-2 text-sm font-medium text-background"
    } else {
        "rounded-xl border border-border bg-panel-muted px-3 py-2 text-sm font-medium text-foreground/55"
    };
    let _ = onclick;
    rsx! {
        a {
            class: class,
            href: format!("?pane=commands&sidebar=1&action={}", action_id.label()),
            aria_disabled: if available { "false" } else { "true" },
            span { "{label}" }
            span { class: "ml-2 text-xs opacity-75", "{action_id.label()}" }
        }
    }
}

fn temporary_artifact_label(ui: &WorkbenchUiState) -> String {
    ui.payload
        .state
        .request
        .as_ref()
        .and_then(|request| request.request_path.as_ref())
        .map(|path| format!("temporary: {}", path.display()))
        .unwrap_or_else(|| "temporary planning artifact".to_string())
}

fn scaffold_action_label(action: ScaffoldAction) -> &'static str {
    action.label()
}

fn scaffold_kind_label(kind: ScaffoldUpdateKind) -> &'static str {
    kind.label()
}

fn confidence_label(confidence: Option<GoalPlanConfidence>) -> String {
    match confidence {
        Some(GoalPlanConfidence::High) => "confidence: high",
        Some(GoalPlanConfidence::Medium) => "confidence: medium",
        Some(GoalPlanConfidence::Low) => "confidence: low",
        None => "confidence: pending",
    }
    .to_string()
}

fn persistent_item_labels(plan: &GoalPlanArtifact) -> Vec<String> {
    let mut labels = Vec::new();
    labels.extend(
        plan.spec_mapping
            .persistent_items
            .philosophies
            .iter()
            .map(persistent_item_id),
    );
    labels.extend(
        plan.spec_mapping
            .persistent_items
            .policies
            .iter()
            .map(persistent_item_id),
    );
    labels.extend(
        plan.spec_mapping
            .persistent_items
            .requirements
            .iter()
            .map(persistent_item_id),
    );
    labels.extend(
        plan.spec_mapping
            .persistent_items
            .features
            .iter()
            .map(persistent_item_id),
    );
    labels
}

fn persistent_item_id(item: &GoalPlanPersistentItem) -> String {
    item.id().to_string()
}

fn include_pattern(include: &GoalPlanScopeInclude) -> String {
    include.pattern().to_string()
}

fn path_matches_goal_pattern(path: &str, pattern: &str) -> bool {
    if path == pattern {
        return true;
    }

    if let Some(prefix) = pattern.strip_suffix("/**") {
        return path.starts_with(prefix);
    }

    if let Some(prefix) = pattern.strip_suffix("/*") {
        return path.starts_with(prefix);
    }

    if let Some((prefix, suffix)) = pattern.split_once('*') {
        return path.starts_with(prefix) && path.ends_with(suffix);
    }

    false
}

fn goal_plan_yaml_preview(plan: &GoalPlanArtifact) -> String {
    serde_yaml::to_string(plan)
        .unwrap_or_else(|err| format!("# failed to render Goal Plan YAML: {err}"))
}

fn graph_state_class(state: &str) -> &'static str {
    match state {
        "spec-linked" => "text-spec-linked",
        "code-linked" => "text-code-linked",
        "test-linked" => "text-test-linked",
        "scope-in" => "text-scope-in",
        "scope-out" => "text-scope-out",
        "scope-ambiguous" => "text-scope-ambiguous",
        "ownership-known" => "text-ownership-known",
        "ownership-missing" => "text-ownership-missing",
        "ownership-ambiguous" => "text-ownership-ambiguous",
        "evidence-pass" => "text-evidence-pass",
        "evidence-warn" => "text-evidence-warn",
        "evidence-fail" => "text-evidence-fail",
        "evidence-pending" => "text-evidence-pending",
        _ => "text-foreground/70",
    }
}

fn assignment_status_tone(status: AssignmentStatus) -> &'static str {
    match status {
        AssignmentStatus::AssignmentReady
        | AssignmentStatus::AssignmentComplete
        | AssignmentStatus::AssignmentDryRun => "bg-evidence-pass",
        AssignmentStatus::AssignmentActive => "bg-evidence-pending",
        AssignmentStatus::AssignmentBlocked | AssignmentStatus::AssignmentFailed => {
            "bg-evidence-fail"
        }
    }
}

fn scope_guard_tone(status: ScopeGuardStatus) -> &'static str {
    match status {
        ScopeGuardStatus::ScopeValid => "bg-evidence-pass",
        ScopeGuardStatus::ScopeAmbiguous => "bg-scope-ambiguous",
        ScopeGuardStatus::ScopeInvalid => "bg-evidence-fail",
    }
}

fn agent_run_tone(status: syu_workbench::AgentRunStatus) -> &'static str {
    match status {
        syu_workbench::AgentRunStatus::RunComplete => "bg-evidence-pass",
        syu_workbench::AgentRunStatus::RunDry | syu_workbench::AgentRunStatus::RunActive => {
            "bg-evidence-pending"
        }
        syu_workbench::AgentRunStatus::RunFailed | syu_workbench::AgentRunStatus::Blocked => {
            "bg-evidence-fail"
        }
    }
}

fn truncate_label(label: &str, max_chars: usize) -> String {
    if label.chars().count() <= max_chars {
        return label.to_string();
    }

    let mut truncated = label
        .chars()
        .take(max_chars.saturating_sub(3))
        .collect::<String>();
    truncated.push_str("...");
    truncated
}

fn assignment_has_automated_assignee(assignment: &Assignment) -> bool {
    assignment
        .assignee
        .as_ref()
        .is_some_and(|assignee| assignee.kind.is_automated())
}

#[cfg(test)]
mod tests;
