use crate::components::{
    AgentEvidenceView, CommandItem, CommandOutputView, DetailDrawer, EmptyState, EvidenceBadge,
    EvidenceDetailDrawer, EvidenceRecordCard, GoalCard, ManualDecisionEvidenceView, Panel,
    ScopeChip, ScopeEvidenceView, StatusDot, TestEvidenceView, ValidationEvidenceView,
};
use crate::design::classes;
use crate::i18n::{HelpTopic, Locale};
use crate::model::{
    CliCommandEntry, CliCommandPreview, CommandCategory, CommandResultItem, CommandResultStatus,
    SpecBrowserDocument, SpecBrowserItem, SpecBrowserModel, SpecBrowserSection, TypedCommandResult,
    WorkbenchUiState, workbench_action_category,
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
    Items,
    Diagnostics,
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
    const ALL: [WorkbenchPane; 4] = [Self::Items, Self::Pulse, Self::Branch, Self::Diagnostics];

    pub fn slug(self) -> &'static str {
        match self {
            Self::Items => "items",
            Self::Diagnostics => "diagnostics",
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
            "items" => Some(Self::Items),
            "diagnostics" => Some(Self::Diagnostics),
            "pulse" | "work" => Some(Self::Request),
            "goals" => Some(Self::Goals),
            "request" => Some(Self::Request),
            "assignment" => Some(Self::Assignment),
            "evidence" => Some(Self::Evidence),
            "branch" | "scope" => Some(Self::Branch),
            "graph" => Some(Self::Graph),
            "commands" | "palette" => Some(Self::Pulse),
            _ => None,
        }
    }

    fn icon(self) -> &'static str {
        match self {
            Self::Items => "▤",
            Self::Diagnostics => "✓",
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

    pub fn for_cli(command_id: &str) -> Self {
        match command_id {
            "cli.browse" | "cli.list" | "cli.show" | "cli.search" | "cli.explain"
            | "cli.relate" | "cli.log" | "cli.add" | "cli.init" | "cli.templates" => Self::Items,
            "cli.audit" | "cli.doctor" | "cli.validate" | "cli.report" | "cli.task.check"
            | "diagnostics.all" => Self::Diagnostics,
            "cli.trace" | "cli.task.scope" | "cli.task.infer" => Self::Branch,
            _ => Self::Pulse,
        }
    }

    pub fn for_action(action_id: WorkbenchActionId) -> Self {
        match action_id {
            WorkbenchActionId::ValidationRun | WorkbenchActionId::GoalCheck => Self::Diagnostics,
            WorkbenchActionId::BranchScope
            | WorkbenchActionId::SpecImpact
            | WorkbenchActionId::TraceRange
            | WorkbenchActionId::RelateRange => Self::Branch,
            WorkbenchActionId::HistoryShow => Self::Items,
            _ => Self::Pulse,
        }
    }

    pub fn role(self) -> Self {
        match self {
            Self::Goals | Self::Request | Self::Assignment | Self::Evidence | Self::Commands => {
                Self::Pulse
            }
            Self::Graph => Self::Branch,
            pane => pane,
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
                div { class: "workbench-layout",
                    if sidebar_open {
                        WorkbenchSidebar { ui: ui.clone(), active_pane }
                    } else {
                        WorkbenchSidebarRail { ui: ui.clone(), active_pane }
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
        header { class: "z-40 border-b border-border bg-panel/95", style: "position: sticky; top: 0",
            nav { class: "mx-auto flex max-w-7xl items-center justify-between gap-4 py-3", "aria-label": "Global",
                div { class: "flex lg:flex-1",
                    a { class: "-m-1.5 p-1.5 text-base font-semibold text-foreground", href: navigation_href(WorkbenchPane::Request, false, ui.locale),
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
                                    a { class: language_button_class(ui.locale == Locale::En), href: view_href(&ui, active_pane, false, Locale::En), "EN" }
                                    a { class: language_button_class(ui.locale == Locale::Ja), href: view_href(&ui, active_pane, false, Locale::Ja), "日本語" }
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
fn HelpLink(ui: WorkbenchUiState, topic: HelpTopic) -> Element {
    let copy = ui.copy();
    let tooltip_id = format!("help-tooltip-{}", topic.slug());
    rsx! {
        button {
            class: "group relative inline-flex h-9 w-9 cursor-help items-center justify-center rounded-full border border-border bg-panel-muted text-xs text-foreground/70 transition hover:bg-background focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-foreground/20",
            type: "button",
            title: copy.help_label(),
            "aria-describedby": tooltip_id.clone(),
            span { class: "pointer-events-none", "?" }
            span {
                id: tooltip_id,
                class: "pointer-events-none absolute right-0 top-full z-50 mt-2 w-72 translate-y-1 rounded-2xl border border-border bg-panel px-4 py-3 text-left opacity-0 shadow-[0_18px_36px_rgba(15,23,42,0.14)] transition duration-150 ease-out group-hover:translate-y-0 group-hover:opacity-100",
                span { class: "absolute -top-1 right-4 h-2 w-2 rotate-45 border-l border-t border-border bg-panel" }
                p { class: "text-[10px] uppercase tracking-[0.24em] text-foreground/45", "{copy.help_label()}" }
                h3 { class: "mt-1 text-sm font-semibold text-foreground", "{copy.help_title(topic)}" }
                p { class: "mt-2 text-sm leading-6 text-foreground/75", "{copy.help_body(topic)}" }
            }
        }
    }
}

fn view_href(
    ui: &WorkbenchUiState,
    pane: WorkbenchPane,
    sidebar_open: bool,
    locale: Locale,
) -> String {
    let mut params = vec![
        format!("pane={}", pane.slug()),
        format!("sidebar={}", if sidebar_open { "1" } else { "0" }),
        format!("lang={}", locale.slug()),
    ];
    if !ui.command_query.trim().is_empty() {
        params.push(format!("query={}", urlencoding::encode(&ui.command_query)));
    }
    if !ui.spec_query.trim().is_empty() {
        params.push(format!(
            "spec_query={}",
            urlencoding::encode(&ui.spec_query)
        ));
    }
    if !ui.spec_kind.trim().is_empty() {
        params.push(format!("spec_kind={}", urlencoding::encode(&ui.spec_kind)));
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
    format!("?{}", params.join("&"))
}

fn navigation_href(pane: WorkbenchPane, sidebar_open: bool, locale: Locale) -> String {
    format!(
        "?pane={}&sidebar={}&lang={}",
        route_pane_slug(pane),
        if sidebar_open { "1" } else { "0" },
        locale.slug()
    )
}

fn route_pane_slug(pane: WorkbenchPane) -> &'static str {
    if pane == WorkbenchPane::Pulse {
        WorkbenchPane::Request.slug()
    } else {
        pane.slug()
    }
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
                        HelpLink { ui: ui.clone(), topic: HelpTopic::Sidebar }
                        a {
                            class: "inline-flex h-8 w-8 items-center justify-center rounded-full border border-border bg-panel-muted text-foreground/60 hover:bg-background",
                            href: view_href(&ui, active_pane, true, ui.locale),
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
                                active: pane == active_pane.role(),
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
                    href: view_href(&ui, active_pane, false, ui.locale),
                    title: copy.sidebar_toggle_open(),
                    "☰"
                }
                ul { class: "space-y-1",
                    for pane in WorkbenchPane::ALL {
                        li {
                            SidebarPaneButton {
                                ui: ui.clone(),
                                pane,
                                active: pane == active_pane.role(),
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
            href: navigation_href(pane, !collapsed, ui.locale),
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
    let cli_preview = cli_preview.filter(|preview| command_matches_pane(&preview.id, active_pane));
    let selected_action = ui
        .selected_action()
        .filter(|action| action_matches_pane(action.id, active_pane))
        .cloned();
    let action_preview = action_preview.filter(|preview| {
        selected_action
            .as_ref()
            .is_some_and(|action| action.id == preview.action_id)
    });
    let item_edit_preview = ui.item_edit_preview.clone();
    let show_pane_detail = cli_preview.is_none() && selected_action.is_none();
    rsx! {
        main { class: "min-w-0 flex-1",
            section { class: "min-w-0 rounded-lg border border-border bg-panel p-4 shadow-sm",
                div { class: "mb-4 flex items-center justify-between gap-3",
                    div { class: "min-w-0",
                        p { class: "text-xs uppercase text-foreground/45", "result" }
                        h1 { class: "truncate text-lg font-semibold text-foreground", "{cli_preview.as_ref().map(|command| command.title.as_str()).or_else(|| selected_action.as_ref().map(|action| action.title.as_str())).unwrap_or(ui.copy().pane_title(active_pane))}" }
                    }
                    a {
                        class: "inline-flex h-9 w-9 items-center justify-center rounded-lg border border-border bg-background text-xs text-foreground/70 hover:bg-panel-muted",
                        href: view_href(&ui, active_pane, false, ui.locale),
                        title: ui.copy().help_label(),
                        "?"
                    }
                }
                RoleSubviewNav { ui: ui.clone(), active_pane }
                if show_pane_detail {
                    {detail}
                }
                if let Some(preview) = item_edit_preview {
                    ItemEditPreviewPanel { preview }
                }
                if let Some(preview) = cli_preview {
                    div { class: "mt-4",
                        if preview.category == CommandCategory::Browse && cli_command_opens_spec_browser(&preview.id) {
                            if let Some(browser) = ui.spec_browser.clone() {
                                SpecInfoBrowser {
                                    browser,
                                    command_query: ui.command_query.clone(),
                                    spec_query: ui.spec_query.clone(),
                                    spec_kind: ui.spec_kind.clone(),
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

fn command_matches_pane(command_id: &str, active_pane: WorkbenchPane) -> bool {
    if active_pane == WorkbenchPane::Commands {
        return true;
    }
    WorkbenchPane::for_cli(command_id).role() == active_pane.role()
}

fn action_matches_pane(action_id: WorkbenchActionId, active_pane: WorkbenchPane) -> bool {
    if active_pane == WorkbenchPane::Commands {
        return true;
    }
    WorkbenchPane::for_action(action_id).role() == active_pane.role()
}

#[component]
fn ItemEditPreviewPanel(preview: crate::model::ItemEditPreview) -> Element {
    rsx! {
        section { class: "mt-4 rounded-lg border border-border bg-background p-4", "data-item-edit-preview": "true",
            div { class: "flex flex-wrap items-start justify-between gap-3",
                div {
                    p { class: "text-[10px] uppercase tracking-[0.24em] text-foreground/45", "Item change" }
                    h3 { class: "mt-1 text-sm font-semibold", "{preview.item_id}" }
                    p { class: "mt-1 text-sm text-foreground/65", "{preview.message}" }
                }
                ScopeChip { label: if preview.applied { "applied".to_string() } else { "preview".to_string() } }
            }
            pre { class: "mt-3 max-h-96 overflow-auto whitespace-pre-wrap rounded-lg border border-border bg-panel-muted p-3 text-xs text-foreground/70", "{preview.diff}" }
            if !preview.applied && !preview.apply_payload.is_empty() {
                form { class: "mt-3", action: "/run", method: "post",
                    input { type: "hidden", name: "pane", value: "items" }
                    input { type: "hidden", name: "sidebar", value: "1" }
                    input { type: "hidden", name: "item_edit", value: "{preview.item_id}" }
                    input { type: "hidden", name: "item_edit_apply", value: "1" }
                    input { type: "hidden", name: "item_edit_payload", value: "{preview.apply_payload}" }
                    button { class: "rounded-lg border border-border bg-foreground px-3 py-2 text-sm font-medium text-background", type: "submit", "Apply reviewed change" }
                }
            }
        }
    }
}

#[component]
fn RoleSubviewNav(ui: WorkbenchUiState, active_pane: WorkbenchPane) -> Element {
    let panes: &[WorkbenchPane] = match active_pane.role() {
        WorkbenchPane::Pulse => &[
            WorkbenchPane::Request,
            WorkbenchPane::Goals,
            WorkbenchPane::Assignment,
            WorkbenchPane::Evidence,
        ],
        WorkbenchPane::Branch => &[WorkbenchPane::Branch, WorkbenchPane::Graph],
        _ => &[],
    };
    if panes.is_empty() {
        return rsx! {};
    }
    rsx! {
        nav { class: "mb-4 flex w-full min-w-0 max-w-full flex-nowrap gap-2 border-b border-border pb-3", style: "max-width: 100%; overflow-x: auto", "aria-label": "Role views", "data-role-subviews": "true",
            for pane in panes {
                a {
                    class: if *pane == active_pane { "whitespace-nowrap rounded-lg border border-foreground bg-foreground px-3 py-2 text-xs font-medium text-background" } else { "whitespace-nowrap rounded-lg border border-border bg-background px-3 py-2 text-xs font-medium text-foreground/70 hover:bg-panel-muted" },
                    href: navigation_href(*pane, true, ui.locale),
                    aria_current: if *pane == active_pane { "page" } else { "false" },
                    "{ui.copy().pane_title(*pane)}"
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
    let copy = crate::i18n::copy(locale);
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
            form { class: "mt-3 grid gap-2 sm:grid-cols-[minmax(0,1fr)_auto]", action: "/run", method: "post", "data-command-run-form": "true",
                input { type: "hidden", name: "pane", value: "{route_pane_slug(WorkbenchPane::for_action(action.id))}" }
                input { type: "hidden", name: "sidebar", value: "1" }
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
                    "data-command-run-button": "true",
                    "data-running-label": copy.running_label(),
                    "{copy.run_label()}"
                }
                p { class: "hidden text-xs text-foreground/60 sm:col-span-2", "aria-live": "polite", "data-command-run-status": "true" }
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
    let copy = crate::i18n::copy(locale);
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
            form { class: "mt-3 grid gap-2 sm:grid-cols-[minmax(0,1fr)_auto]", action: "/run", method: "post", "data-command-run-form": "true",
                input { type: "hidden", name: "pane", value: "{route_pane_slug(WorkbenchPane::for_cli(&preview.id))}" }
                input { type: "hidden", name: "sidebar", value: "1" }
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
                    "data-command-run-button": "true",
                    "data-running-label": copy.running_label(),
                    "{copy.run_label()}"
                }
                p { class: "hidden text-xs text-foreground/60 sm:col-span-2", "aria-live": "polite", "data-command-run-status": "true" }
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SpecKindTab {
    Philosophy,
    Policy,
    Requirement,
    Feature,
}

impl SpecKindTab {
    const ALL: [Self; 4] = [
        Self::Philosophy,
        Self::Policy,
        Self::Requirement,
        Self::Feature,
    ];

    const fn slug(self) -> &'static str {
        match self {
            Self::Philosophy => "philosophy",
            Self::Policy => "policy",
            Self::Requirement => "requirement",
            Self::Feature => "feature",
        }
    }

    const fn label(self) -> &'static str {
        match self {
            Self::Philosophy => "Philosophy",
            Self::Policy => "Policy",
            Self::Requirement => "Requirement",
            Self::Feature => "Feature",
        }
    }

    fn from_slug(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.slug() == value)
    }

    fn matches_section(self, label: &str) -> bool {
        matches!(
            (self, label.to_ascii_lowercase().as_str()),
            (Self::Philosophy, "philosophy")
                | (Self::Policy, "policies")
                | (Self::Requirement, "requirements")
                | (Self::Feature, "features")
        )
    }
}

fn active_spec_kind(browser: &SpecBrowserModel, spec_kind: &str) -> SpecKindTab {
    SpecKindTab::from_slug(spec_kind.trim())
        .filter(|kind| {
            browser
                .sections
                .iter()
                .any(|section| kind.matches_section(&section.label))
        })
        .unwrap_or_else(|| {
            SpecKindTab::ALL
                .into_iter()
                .find(|kind| {
                    browser
                        .sections
                        .iter()
                        .any(|section| kind.matches_section(&section.label))
                })
                .unwrap_or(SpecKindTab::Requirement)
        })
}

fn filter_spec_browser_by_kind(browser: &SpecBrowserModel, kind: SpecKindTab) -> SpecBrowserModel {
    SpecBrowserModel {
        sections: browser
            .sections
            .iter()
            .filter(|section| kind.matches_section(&section.label))
            .cloned()
            .collect(),
        selected_item_id: browser.selected_item_id.clone(),
    }
}

fn spec_browser_items(browser: &SpecBrowserModel) -> Vec<SpecBrowserItem> {
    browser
        .sections
        .iter()
        .flat_map(|section| section.documents.iter())
        .flat_map(|document| document.items.iter())
        .cloned()
        .collect()
}

fn spec_kind_tab_class(active: bool) -> &'static str {
    if active {
        "whitespace-nowrap rounded-lg border border-foreground bg-foreground px-3 py-2 text-xs font-medium text-background"
    } else {
        "whitespace-nowrap rounded-lg border border-border bg-background px-3 py-2 text-xs font-medium text-foreground/70 hover:bg-panel-muted"
    }
}

fn spec_kind_href(
    command_id: &str,
    locale: Locale,
    category: &str,
    command_query: &str,
    spec_query: &str,
    kind: SpecKindTab,
) -> String {
    format!(
        "?pane=items&sidebar=1&lang={}&cli={}&category={}&query={}&spec_query={}&spec_kind={}",
        locale.slug(),
        urlencoding::encode(command_id),
        urlencoding::encode(category),
        urlencoding::encode(command_query),
        urlencoding::encode(spec_query),
        kind.slug(),
    )
}

#[component]
fn SpecInfoBrowser(
    browser: SpecBrowserModel,
    command_query: String,
    spec_query: String,
    spec_kind: String,
    locale: Locale,
    command_id: String,
    category: Option<CommandCategory>,
) -> Element {
    let browser = filtered_spec_browser(&browser, &spec_query);
    let active_kind = active_spec_kind(&browser, &spec_kind);
    let browser = filter_spec_browser_by_kind(&browser, active_kind);
    let selected = selected_spec_item(&browser);
    let selected_id = selected.as_ref().map(|item| item.id.clone());
    let detail_items = spec_browser_items(&browser);
    let category_value = category.map_or("browse", CommandCategory::slug);
    rsx! {
        section { class: "rounded-lg border border-border bg-panel p-4 shadow-sm", "data-category-layout": "browse",
            div { class: "mb-4 flex flex-wrap items-center justify-between gap-2", "data-items-toolbar": "true",
                div {
                    p { class: "text-[10px] uppercase tracking-[0.24em] text-foreground/45", "Persistent specification" }
                    p { class: "text-sm text-foreground/65", "Browse the layered files, follow linked items, or create a new specification item." }
                }
                div { class: "flex flex-wrap gap-2",
                    a { class: "rounded-lg border border-border bg-background px-3 py-2 text-xs font-medium hover:bg-panel-muted", href: "?pane=items&sidebar=1&cli=cli.add", "New item" }
                    a { class: "rounded-lg border border-border bg-background px-3 py-2 text-xs font-medium hover:bg-panel-muted", href: "?pane=items&sidebar=1&cli=cli.init", "Initialize workspace" }
                }
            }
            form {
                class: "mb-3 flex items-end gap-2",
                "data-spec-search": "true",
                action: "/",
                method: "get",
                input { type: "hidden", name: "pane", value: "items" }
                input { type: "hidden", name: "sidebar", value: "0" }
                input { type: "hidden", name: "lang", value: "{locale.slug()}" }
                input { type: "hidden", name: "cli", value: "{command_id}" }
                input { type: "hidden", name: "category", value: "{category_value}" }
                input { type: "hidden", name: "query", value: "{command_query}" }
                input { type: "hidden", name: "spec_kind", value: "{active_kind.slug()}" }
                div { class: "min-w-0 flex-1",
                    p { class: "mb-1 text-[10px] uppercase tracking-[0.24em] text-foreground/45", "Search specs" }
                    input {
                        class: "w-full rounded-lg border border-border bg-background px-3 py-2 text-sm outline-none focus:border-foreground/20",
                        name: "spec_query",
                        value: "{spec_query}",
                        placeholder: "Search specs",
                    }
                }
                button {
                    class: "rounded-lg border border-border bg-foreground px-3 py-2 text-sm font-medium text-background hover:bg-foreground/90",
                    type: "submit",
                    "Search"
                }
            }
            nav { class: "mb-3 flex w-full min-w-0 flex-nowrap gap-2 overflow-x-auto border-b border-border pb-3", "aria-label": "Spec kind tabs",
                for kind in SpecKindTab::ALL {
                    a {
                        class: spec_kind_tab_class(kind == active_kind),
                        href: spec_kind_href(&command_id, locale, category_value, &command_query, &spec_query, kind),
                        aria_current: if kind == active_kind { "page" } else { "false" },
                        "data-spec-kind-tab": kind.slug(),
                        "{kind.label()}"
                    }
                }
            }
            div { class: "mb-3 grid gap-3 lg:grid-cols-3", "data-spec-browser-grid": "true", "data-spec-kind-panel": active_kind.slug(),
                div { class: "rounded-lg border border-border bg-background p-2",
                    p { class: "px-2 pb-2 text-[10px] uppercase tracking-[0.24em] text-foreground/45", "Spec tree" }
                    nav { class: "overflow-auto", style: "max-height: 30rem", "aria-label": "Spec tree",
                        for section in &browser.sections {
                            div { class: "mb-3 last:mb-0",
                                p { class: "px-2 pb-1 text-[10px] uppercase tracking-[0.24em] text-foreground/45", "{section.label}" }
                                SpecSectionTree {
                                    section: section.clone(),
                                    selected_id: selected.as_ref().map(|selected| selected.id.clone()),
                                    command_id: command_id.clone(),
                                    locale,
                                    category: category_value.to_string(),
                                    command_query: command_query.clone(),
                                    spec_query: spec_query.clone(),
                                    spec_kind: active_kind.slug().to_string(),
                                }
                            }
                        }
                    }
                }
                div { class: "min-w-0", "data-spec-detail": "true",
                    if !detail_items.is_empty() {
                        for item in detail_items {
                            div {
                                hidden: selected_id.as_deref() != Some(item.id.as_str()),
                                "data-spec-detail-card": item.id.clone(),
                                SpecModelCard { item: item.clone() }
                            }
                        }
                    } else if !spec_query.trim().is_empty() {
                        EmptyState { title: "No matching spec items".to_string(), body: "Try another ID, title, summary, or description.".to_string() }
                    } else {
                        EmptyState { title: "No spec item".to_string(), body: "The workspace spec tree is empty or still loading." }
                    }
                }
            }
        }
    }
}

#[component]
fn SpecSectionTree(
    section: SpecBrowserSection,
    selected_id: Option<String>,
    command_id: String,
    locale: Locale,
    category: String,
    command_query: String,
    spec_query: String,
    spec_kind: String,
) -> Element {
    let tree = SpecFolderNode::from_section(&section);
    rsx! {
        div { class: "grid gap-0.5", "data-spec-section-tree": section.label.clone(),
            for document in &tree.documents {
                SpecDocumentTree {
                    document: document.clone(),
                    selected_id: selected_id.clone(),
                    command_id: command_id.clone(),
                    locale,
                    category: category.clone(),
                    command_query: command_query.clone(),
                    spec_query: spec_query.clone(),
                    spec_kind: spec_kind.clone(),
                }
            }
            for folder in &tree.folders {
                SpecFolderTree {
                    folder: folder.clone(),
                    selected_id: selected_id.clone(),
                    command_id: command_id.clone(),
                    locale,
                    category: category.clone(),
                    command_query: command_query.clone(),
                    spec_query: spec_query.clone(),
                    spec_kind: spec_kind.clone(),
                }
            }
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
struct SpecFolderNode {
    name: String,
    path: String,
    folders: Vec<SpecFolderNode>,
    documents: Vec<SpecBrowserDocument>,
}

impl SpecFolderNode {
    fn from_section(section: &SpecBrowserSection) -> Self {
        let mut root = Self::default();
        for document in &section.documents {
            root.insert(
                normalized_folder_segments(section, document),
                document.clone(),
            );
        }
        root
    }

    fn insert(&mut self, segments: Vec<String>, document: SpecBrowserDocument) {
        if let Some((first, rest)) = segments.split_first() {
            let path = if self.path.is_empty() {
                first.clone()
            } else {
                format!("{}/{}", self.path, first)
            };
            let index = self
                .folders
                .iter()
                .position(|folder| folder.name == *first)
                .unwrap_or_else(|| {
                    self.folders.push(Self {
                        name: first.clone(),
                        path,
                        folders: Vec::new(),
                        documents: Vec::new(),
                    });
                    self.folders.len() - 1
                });
            self.folders[index].insert(rest.to_vec(), document);
        } else {
            self.documents.push(document);
        }
    }
}

fn normalized_folder_segments(
    section: &SpecBrowserSection,
    document: &SpecBrowserDocument,
) -> Vec<String> {
    let mut segments = document.folder_segments.clone();
    if let Some(root) = section_root_segment(&section.label)
        && segments
            .first()
            .is_some_and(|segment| segment.eq_ignore_ascii_case(root))
    {
        segments.remove(0);
    }
    segments
}

fn section_root_segment(label: &str) -> Option<&'static str> {
    match label.to_ascii_lowercase().as_str() {
        "philosophy" => Some("philosophy"),
        "policies" => Some("policies"),
        "requirements" => Some("requirements"),
        "features" => Some("features"),
        _ => None,
    }
}

#[component]
fn SpecFolderTree(
    folder: SpecFolderNode,
    selected_id: Option<String>,
    command_id: String,
    locale: Locale,
    category: String,
    command_query: String,
    spec_query: String,
    spec_kind: String,
) -> Element {
    rsx! {
        details {
            class: "group",
            open: true,
            "data-spec-folder": "true",
            "data-spec-folder-path": folder.path.clone(),
            summary { class: "flex cursor-pointer list-none items-center gap-1 rounded-md px-2 py-1 text-xs font-medium text-foreground/65 hover:bg-panel-muted",
                span { class: "w-3 text-[10px] text-foreground/40 group-open:rotate-90", "data-spec-folder-toggle": "true", ">" }
                span { class: "text-[10px] uppercase tracking-[0.12em] text-foreground/40", "data-spec-folder-icon": "true", "folder" }
                span { class: "truncate", "{folder.name}" }
            }
            div { class: "ml-3 grid gap-0.5 border-l border-border pl-2",
                for document in &folder.documents {
                    SpecDocumentTree {
                        document: document.clone(),
                        selected_id: selected_id.clone(),
                        command_id: command_id.clone(),
                        locale,
                        category: category.clone(),
                        command_query: command_query.clone(),
                        spec_query: spec_query.clone(),
                        spec_kind: spec_kind.clone(),
                    }
                }
                for child in &folder.folders {
                    SpecFolderTree {
                        folder: child.clone(),
                        selected_id: selected_id.clone(),
                        command_id: command_id.clone(),
                        locale,
                        category: category.clone(),
                        command_query: command_query.clone(),
                        spec_query: spec_query.clone(),
                        spec_kind: spec_kind.clone(),
                    }
                }
            }
        }
    }
}

#[component]
fn SpecDocumentTree(
    document: SpecBrowserDocument,
    selected_id: Option<String>,
    command_id: String,
    locale: Locale,
    category: String,
    command_query: String,
    spec_query: String,
    spec_kind: String,
) -> Element {
    rsx! {
        details {
            class: "group",
            open: true,
            "data-spec-document": "true",
            "data-spec-document-path": document.path.clone(),
            summary { class: "flex cursor-pointer list-none items-center gap-1 rounded-md px-2 py-1 text-xs font-medium text-foreground/65 hover:bg-panel-muted",
                span { class: "w-3 text-[10px] text-foreground/40 group-open:rotate-90", "data-spec-document-toggle": "true", ">" }
                span { class: "text-[10px] uppercase tracking-[0.12em] text-foreground/40", "doc" }
                span { class: "truncate", "{document.title}" }
            }
            div { class: "ml-3 grid gap-0.5 border-l border-border pl-2",
                for item in &document.items {
                    a {
                        class: spec_tree_item_class(selected_id.as_deref() == Some(item.id.as_str())),
                        href: spec_item_href(&command_id, locale, &category, &command_query, &spec_query, &spec_kind, &item.id),
                        title: "{item.title}",
                        "data-spec-tree-item": "true",
                        "data-spec-item-target": item.id.clone(),
                        "data-spec-text": format!("{} {} {}", item.id, item.title, item.description.clone().unwrap_or_default()),
                        span { class: "truncate text-xs font-medium", "{item.id}" }
                        span { class: "truncate text-[11px] text-foreground/55", "{item.title}" }
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
    command_query: &str,
    spec_query: &str,
    spec_kind: &str,
    item_id: &str,
) -> String {
    format!(
        "?pane=items&sidebar=1&lang={}&cli={}&category={}&query={}&spec_query={}&spec_kind={}&spec_item={}",
        locale.slug(),
        urlencoding::encode(command_id),
        urlencoding::encode(category),
        urlencoding::encode(command_query),
        urlencoding::encode(spec_query),
        urlencoding::encode(spec_kind),
        urlencoding::encode(item_id),
    )
}

fn spec_tree_item_class(active: bool) -> &'static str {
    if active {
        "grid gap-0.5 rounded-md border border-foreground bg-foreground px-2 py-1.5 text-background"
    } else {
        "grid gap-0.5 rounded-md border border-transparent px-2 py-1.5 text-foreground hover:border-border hover:bg-panel-muted"
    }
}

fn selected_spec_item(browser: &SpecBrowserModel) -> Option<SpecBrowserItem> {
    browser
        .sections
        .iter()
        .flat_map(|section| section.documents.iter())
        .flat_map(|document| document.items.iter())
        .find(|item| browser.selected_item_id.as_deref() == Some(item.id.as_str()))
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

fn filtered_spec_browser(browser: &SpecBrowserModel, query: &str) -> SpecBrowserModel {
    let needle = query.trim().to_lowercase();
    if needle.is_empty() {
        return browser.clone();
    }

    let sections = browser
        .sections
        .iter()
        .filter_map(|section| {
            let documents = section
                .documents
                .iter()
                .filter_map(|document| {
                    let items = document
                        .items
                        .iter()
                        .filter(|item| spec_item_matches(item, &needle))
                        .cloned()
                        .collect::<Vec<_>>();
                    (!items.is_empty()).then(|| SpecBrowserDocument {
                        path: document.path.clone(),
                        title: document.title.clone(),
                        folder_segments: document.folder_segments.clone(),
                        items,
                    })
                })
                .collect::<Vec<_>>();
            (!documents.is_empty()).then(|| SpecBrowserSection {
                label: section.label.clone(),
                documents,
            })
        })
        .collect();

    SpecBrowserModel {
        sections,
        selected_item_id: browser.selected_item_id.clone(),
    }
}

fn spec_item_matches(item: &SpecBrowserItem, needle: &str) -> bool {
    format!(
        "{} {} {} {}",
        item.id,
        item.title,
        item.summary.clone().unwrap_or_default(),
        item.description.clone().unwrap_or_default()
    )
    .to_lowercase()
    .contains(needle)
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
            details { class: "mt-4 rounded-lg border border-border bg-panel p-3", "data-item-editor": "true",
                summary { class: "cursor-pointer text-sm font-semibold", "Edit item" }
                form { class: "mt-3 grid gap-3", action: "/run", method: "post",
                    input { type: "hidden", name: "pane", value: "items" }
                    input { type: "hidden", name: "sidebar", value: "1" }
                    input { type: "hidden", name: "item_edit", value: "{item.id}" }
                    label { class: "grid gap-1 text-xs text-foreground/60",
                        "Title"
                        input { class: "rounded-lg border border-border bg-background px-3 py-2 text-sm text-foreground", value: "{item.title}", name: "title" }
                    }
                    label { class: "grid gap-1 text-xs text-foreground/60",
                        "Summary"
                        textarea { class: "min-h-20 rounded-lg border border-border bg-background px-3 py-2 text-sm text-foreground", name: "summary", "{item.summary.clone().unwrap_or_default()}" }
                    }
                    label { class: "grid gap-1 text-xs text-foreground/60",
                        "Description"
                        textarea { class: "min-h-28 rounded-lg border border-border bg-background px-3 py-2 text-sm text-foreground", name: "description", "{item.description.clone().unwrap_or_default()}" }
                    }
                    label { class: "grid gap-1 text-xs text-foreground/60",
                        "Product design principle"
                        textarea { class: "min-h-28 rounded-lg border border-border bg-background px-3 py-2 text-sm text-foreground", name: "product_design_principle", "{item.product_design_principle.clone().unwrap_or_default()}" }
                    }
                    label { class: "grid gap-1 text-xs text-foreground/60",
                        "Coding guideline"
                        textarea { class: "min-h-28 rounded-lg border border-border bg-background px-3 py-2 text-sm text-foreground", name: "coding_guideline", "{item.coding_guideline.clone().unwrap_or_default()}" }
                    }
                    div { class: "grid gap-3 sm:grid-cols-2",
                        label { class: "grid gap-1 text-xs text-foreground/60",
                            "Priority"
                            input { class: "rounded-lg border border-border bg-background px-3 py-2 text-sm text-foreground", value: "{item.priority.clone().unwrap_or_default()}", name: "priority" }
                        }
                        label { class: "grid gap-1 text-xs text-foreground/60",
                            "Status"
                            input { class: "rounded-lg border border-border bg-background px-3 py-2 text-sm text-foreground", value: "{item.status.clone().unwrap_or_default()}", name: "status" }
                        }
                    }
                    label { class: "grid gap-1 text-xs text-foreground/60",
                        "Linked philosophies"
                        input { class: "rounded-lg border border-border bg-background px-3 py-2 text-sm text-foreground", value: "{item.linked_philosophies.join(\", \")}", name: "linked_philosophies" }
                    }
                    label { class: "grid gap-1 text-xs text-foreground/60",
                        "Linked policies"
                        input { class: "rounded-lg border border-border bg-background px-3 py-2 text-sm text-foreground", value: "{item.linked_policies.join(\", \")}", name: "linked_policies" }
                    }
                    label { class: "grid gap-1 text-xs text-foreground/60",
                        "Linked requirements"
                        input { class: "rounded-lg border border-border bg-background px-3 py-2 text-sm text-foreground", value: "{item.linked_requirements.join(\", \")}", name: "linked_requirements" }
                    }
                    label { class: "grid gap-1 text-xs text-foreground/60",
                        "Linked features"
                        input { class: "rounded-lg border border-border bg-background px-3 py-2 text-sm text-foreground", value: "{item.linked_features.join(\", \")}", name: "linked_features" }
                    }
                    label { class: "grid gap-1 text-xs text-foreground/60",
                        "Tests YAML"
                        textarea { class: "min-h-36 rounded-lg border border-border bg-background px-3 py-2 font-mono text-xs text-foreground", name: "tests_yaml", "{trace_groups_yaml(&item.tests)}" }
                    }
                    label { class: "grid gap-1 text-xs text-foreground/60",
                        "Implementations YAML"
                        textarea { class: "min-h-36 rounded-lg border border-border bg-background px-3 py-2 font-mono text-xs text-foreground", name: "implementations_yaml", "{trace_groups_yaml(&item.implementations)}" }
                    }
                    p { class: "text-xs text-foreground/50", "A source-preserving diff is shown before files are changed. ID and document location stay fixed." }
                    button { class: "w-fit rounded-lg border border-border bg-foreground px-3 py-2 text-sm font-medium text-background", type: "submit", "Preview changes" }
                }
            }
        }
    }
}

fn trace_groups_yaml(groups: &[crate::model::SpecBrowserTraceGroup]) -> String {
    let mut languages = serde_yaml::Mapping::new();
    for group in groups {
        let references = group
            .references
            .iter()
            .map(|reference| {
                let mut mapping = serde_yaml::Mapping::new();
                mapping.insert(
                    serde_yaml::Value::String("file".to_string()),
                    serde_yaml::Value::String(reference.file.clone()),
                );
                if !reference.symbols.is_empty() {
                    mapping.insert(
                        serde_yaml::Value::String("symbols".to_string()),
                        serde_yaml::Value::Sequence(
                            reference
                                .symbols
                                .iter()
                                .cloned()
                                .map(serde_yaml::Value::String)
                                .collect(),
                        ),
                    );
                }
                if !reference.doc_contains.is_empty() {
                    mapping.insert(
                        serde_yaml::Value::String("doc_contains".to_string()),
                        serde_yaml::Value::Sequence(
                            reference
                                .doc_contains
                                .iter()
                                .cloned()
                                .map(serde_yaml::Value::String)
                                .collect(),
                        ),
                    );
                }
                if let Some(method) = &reference.method {
                    mapping.insert(
                        serde_yaml::Value::String("method".to_string()),
                        serde_yaml::Value::String(method.clone()),
                    );
                }
                if let Some(path) = &reference.path {
                    mapping.insert(
                        serde_yaml::Value::String("path".to_string()),
                        serde_yaml::Value::String(path.clone()),
                    );
                }
                serde_yaml::Value::Mapping(mapping)
            })
            .collect();
        languages.insert(
            serde_yaml::Value::String(group.language.clone()),
            serde_yaml::Value::Sequence(references),
        );
    }
    if languages.is_empty() {
        String::new()
    } else {
        serde_yaml::to_string(&serde_yaml::Value::Mapping(languages)).unwrap_or_default()
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
                    a {
                        class: "rounded-md border border-border bg-panel px-2 py-1 text-xs font-medium text-foreground/70 hover:bg-panel-muted",
                        href: format!("?pane=items&sidebar=1&cli=cli.show&spec_item={}", urlencoding::encode(&value)),
                        "{value}"
                    }
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

#[component]
fn DiagnosticsOverview(ui: WorkbenchUiState) -> Element {
    let tools = [
        (
            "Workspace validation",
            "Validate the layered graph, traces, and document consistency.",
            "cli.validate",
            "validate",
        ),
        (
            "Contributor doctor",
            "Check the local tools and contributor surfaces used by this workspace.",
            "cli.doctor",
            "doctor",
        ),
        (
            "Specification audit",
            "Review overlap, tension, and orphaned-policy candidates.",
            "cli.audit",
            "audit",
        ),
        (
            "Goal check",
            "Compare the active Goal Plan with the current branch range.",
            "cli.task.check",
            "goal",
        ),
    ];
    rsx! {
        section { class: "space-y-4", "data-diagnostics-overview": "true",
            div { class: "flex flex-wrap items-start justify-between gap-3 rounded-lg border border-border bg-background p-4",
                div {
                    p { class: "text-[10px] uppercase tracking-[0.24em] text-foreground/45", "All checks" }
                    h3 { class: "mt-1 text-base font-semibold", "Workspace diagnostics" }
                    p { class: "mt-1 text-sm text-foreground/65", "Refresh every available check, then open a finding to inspect its context." }
                }
                form { action: "/run", method: "post", "data-diagnostics-refresh-all": "true",
                    input { type: "hidden", name: "pane", value: "diagnostics" }
                    input { type: "hidden", name: "sidebar", value: "1" }
                    input { type: "hidden", name: "lang", value: "{ui.locale.slug()}" }
                    input { type: "hidden", name: "diagnostics_all", value: "1" }
                    button {
                        class: "rounded-lg border border-border bg-foreground px-3 py-2 text-sm font-medium text-background hover:bg-foreground/90",
                        type: "submit",
                        "Refresh all"
                    }
                }
            }
            div { class: "grid gap-3 md:grid-cols-2",
                for (title, description, command_id, tool_id) in tools {
                    a {
                        class: "rounded-lg border border-border bg-background p-4 hover:bg-panel-muted",
                        href: format!("?pane=diagnostics&sidebar=1&cli={command_id}"),
                        "data-diagnostic-tool": tool_id,
                        div { class: "flex items-start justify-between gap-3",
                            div {
                                h3 { class: "text-sm font-semibold", "{title}" }
                                p { class: "mt-2 text-sm text-foreground/65", "{description}" }
                            }
                            ScopeChip { label: if command_id == "cli.task.check" && ui.payload.state.goals.active.is_empty() { "skipped".to_string() } else { "ready".to_string() } }
                        }
                    }
                }
            }
        }
    }
}

fn selected_pane_detail(ui: WorkbenchUiState, active_pane: WorkbenchPane) -> Element {
    match active_pane {
        WorkbenchPane::Items => {
            if let Some(browser) = ui.spec_browser.clone() {
                rsx! {
                    SpecInfoBrowser {
                        browser,
                        command_query: ui.command_query.clone(),
                        spec_query: ui.spec_query.clone(),
                        spec_kind: ui.spec_kind.clone(),
                        locale: ui.locale,
                        command_id: "cli.show".to_string(),
                        category: Some(CommandCategory::Browse),
                    }
                }
            } else {
                rsx! { EmptyState { title: "No syu workspace".to_string(), body: "Initialize this workspace to create and browse its specification tree.".to_string() } }
            }
        }
        WorkbenchPane::Diagnostics => rsx! { DiagnosticsOverview { ui } },
        WorkbenchPane::Pulse => rsx! { RequestOverview { ui } },
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
