use crate::components::{
    AgentEvidenceView, CommandItem, CommandOutputView, DetailDrawer, EmptyState, EvidenceBadge,
    EvidenceDetailDrawer, EvidenceRecordCard, GoalCard, ManualDecisionEvidenceView, Panel,
    ScopeChip, ScopeEvidenceView, StatusDot, TestEvidenceView, ValidationEvidenceView,
};
use crate::design::classes;
use crate::i18n::{HelpTopic, Locale};
use crate::model::{CliCommandEntry, CliCommandPreview, WorkbenchUiState, WorkspacePulseSummary};
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
                                LabeledSelect { label: copy.language_label().to_string(), value: copy.language_name(ui.locale).to_string() }
                                LabeledSelect { label: copy.workspace_label().to_string(), value: summary.workspace.clone() }
                                LabeledSelect { label: copy.branch_label().to_string(), value: summary.branch.clone() }
                                LabeledSelect { label: copy.health_label().to_string(), value: summary.health.clone() }
                                div { class: "grid grid-cols-2 gap-2",
                                    a { class: "rounded-lg border border-border bg-background px-3 py-2 text-center text-xs font-medium text-foreground/75 hover:bg-panel-muted", href: view_href(&ui, active_pane, false, Locale::En, ui.help_topic), "EN" }
                                    a { class: "rounded-lg border border-border bg-background px-3 py-2 text-center text-xs font-medium text-foreground/75 hover:bg-panel-muted", href: view_href(&ui, active_pane, false, Locale::Ja, ui.help_topic), "日本語" }
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
fn LabeledSelect(label: String, value: String) -> Element {
    rsx! {
        label { class: "grid gap-1 text-xs text-foreground/55",
            span { class: "uppercase", "{label}" }
            select { class: "w-full truncate rounded-lg border border-border bg-background px-3 py-2 text-sm text-foreground outline-none", disabled: true,
                option { selected: true, "{value}" }
            }
        }
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
    let action_preview = ui
        .preview
        .clone()
        .or_else(|| ui.selected_action_id.and_then(|id| ui.action_preview(id)));
    let cli_preview = ui.cli_preview.clone().or_else(|| {
        ui.selected_cli_command_id
            .as_deref()
            .and_then(|id| ui.cli_command_preview(id))
    });
    let selected_action = ui.selected_action().cloned();
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
                {detail}
                if let Some(preview) = cli_preview {
                    div { class: "mt-4",
                        CliCommandResult { preview }
                    }
                } else if let Some(preview) = action_preview {
                    div { class: "mt-4",
                        DetailDrawer {
                            title: preview.title.clone(),
                            body: preview.result_summary.clone(),
                            evidence: preview.evidence_summary.clone(),
                        }
                    }
                } else if let Some(action) = selected_action {
                    div { class: "mt-4",
                        WorkbenchActionResult { action }
                    }
                }
            }
        }
    }
}

#[component]
fn WorkbenchActionResult(action: WorkbenchAction) -> Element {
    let needs_input = workbench_action_needs_text_input(action.id.label());
    let needs_confirmation = action.mutability.requires_confirmation();
    rsx! {
        section { class: classes::DRAWER,
            div { class: "flex items-center justify-between gap-3",
                h3 { class: "text-sm font-semibold", "{action.title}" }
                ScopeChip { label: if needs_confirmation { "confirm".to_string() } else { "ready".to_string() } }
            }
            p { class: "mt-2 text-sm text-foreground/75", "{action.description}" }
            form { class: "mt-3 grid gap-2 sm:grid-cols-[minmax(0,1fr)_auto]", action: "/", method: "get",
                input { type: "hidden", name: "pane", value: "commands" }
                input { type: "hidden", name: "sidebar", value: "0" }
                input { type: "hidden", name: "action", value: "{action.id.label()}" }
                input { type: "hidden", name: "run", value: "1" }
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
fn CliCommandResult(preview: CliCommandPreview) -> Element {
    let default_cli_arg = cli_input_placeholder(&preview.id);

    rsx! {
        section { class: classes::DRAWER,
            div { class: "flex items-center justify-between gap-3",
                h3 { class: "text-sm font-semibold", "{preview.title}" }
                ScopeChip { label: "result".to_string() }
            }
            p { class: "mt-2 whitespace-pre-wrap break-all text-sm text-foreground/75", "{preview.result_summary}" }
            p { class: "mt-2 break-all text-[10px] uppercase tracking-[0.24em] text-foreground/45", "{preview.invocation} · {preview.evidence_summary}" }
            if preview.requires_input || preview.mutates_files {
                form { class: "mt-3 grid gap-2 sm:grid-cols-[minmax(0,1fr)_auto]", action: "/", method: "get",
                    input { type: "hidden", name: "pane", value: "commands" }
                    input { type: "hidden", name: "sidebar", value: "0" }
                    input { type: "hidden", name: "cli", value: "{preview.id}" }
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
pub fn WorkbenchPulse(summary: WorkspacePulseSummary) -> Element {
    rsx! {
        div { class: "space-y-4",
            div { class: "grid gap-3 md:grid-cols-3",
                div { class: "space-y-1",
                    p { class: "text-[10px] uppercase tracking-[0.24em] text-foreground/45", "workspace" }
                    select {
                        class: "w-full rounded-xl border border-border bg-background px-3 py-2 text-sm outline-none",
                        disabled: true,
                        option { selected: true, "{summary.workspace}" }
                    }
                }
                div { class: "space-y-1",
                    p { class: "text-[10px] uppercase tracking-[0.24em] text-foreground/45", "branch" }
                    select {
                        class: "w-full rounded-xl border border-border bg-background px-3 py-2 text-sm outline-none",
                        disabled: true,
                        option { selected: true, "{summary.branch}" }
                    }
                }
                div { class: "space-y-1",
                    p { class: "text-[10px] uppercase tracking-[0.24em] text-foreground/45", "health" }
                    select {
                        class: "w-full rounded-xl border border-border bg-background px-3 py-2 text-sm outline-none",
                        disabled: true,
                        option { selected: true, "{summary.health}" }
                    }
                }
            }
            div { class: "grid gap-3 xl:grid-cols-3",
                a {
                    class: "rounded-2xl border border-border bg-background p-4 text-left hover:bg-panel",
                    href: "?pane=commands&sidebar=1&help=palette",
                    div { class: "flex items-center gap-3",
                        span { class: "grid h-10 w-10 place-items-center rounded-full border border-border bg-panel-muted text-xs", "⌘" }
                        div { class: "min-w-0",
                            p { class: "text-sm font-medium text-foreground", "Type a command" }
                            p { class: "text-xs uppercase tracking-[0.18em] text-foreground/45", "open palette" }
                        }
                    }
                }
                a {
                    class: "rounded-2xl border border-border bg-background p-4 text-left hover:bg-panel",
                    href: "?pane=pulse&sidebar=1&action=branch.scope",
                    div { class: "flex items-center gap-3",
                        span { class: "grid h-10 w-10 place-items-center rounded-full border border-border bg-panel-muted text-xs", "↻" }
                        div { class: "min-w-0",
                            p { class: "text-sm font-medium text-foreground", "Load branch scope" }
                            p { class: "text-xs uppercase tracking-[0.18em] text-foreground/45", "see changes" }
                        }
                    }
                }
                a {
                    class: "rounded-2xl border border-border bg-background p-4 text-left hover:bg-panel",
                    href: "?pane=pulse&sidebar=1&action=request.new",
                    div { class: "flex items-center gap-3",
                        span { class: "grid h-10 w-10 place-items-center rounded-full border border-border bg-panel-muted text-xs", "◌" }
                        div { class: "min-w-0",
                            p { class: "text-sm font-medium text-foreground", "Start a request" }
                            p { class: "text-xs uppercase tracking-[0.18em] text-foreground/45", "intake first" }
                        }
                    }
                }
            }
            div { class: "grid gap-3 md:grid-cols-3",
                div { class: "space-y-1",
                    PulseMetric { label: "ready".to_string(), value: summary.available_actions.to_string() }
                }
                PulseMetric { label: "latest".to_string(), value: summary.recent_evidence.clone() }
                PulseMetric { label: "open".to_string(), value: summary.next_action.clone() }
            }
        }
    }
}

#[component]
pub fn WorkspacePulse(summary: WorkspacePulseSummary) -> Element {
    rsx! { WorkbenchPulse { summary } }
}

#[component]
fn CommandSurfaceOverview(ui: WorkbenchUiState) -> Element {
    let copy = ui.copy();
    rsx! {
        div { class: "space-y-3",
            div { class: "rounded-2xl border border-border bg-background p-4",
                div { class: "flex flex-wrap items-center gap-2",
                    ScopeChip { label: copy.command_surface_title().to_string() }
                    ScopeChip { label: copy.command_surface_chip_one().to_string() }
                    ScopeChip { label: copy.command_surface_chip_two().to_string() }
                    ScopeChip { label: copy.command_surface_chip_three().to_string() }
                }
                p { class: "mt-3 text-sm text-foreground/75", "{copy.command_surface_body()}" }
            }
            div { class: "grid gap-3 md:grid-cols-3",
                MiniSelect { label: "focus".to_string(), value: copy.palette_hint().to_string() }
                MiniSelect { label: "open".to_string(), value: copy.palette_hint_active().to_string() }
                MiniSelect { label: "help".to_string(), value: copy.help_title(HelpTopic::Palette).to_string() }
            }
        }
    }
}

#[component]
fn GoalsOverview(ui: WorkbenchUiState) -> Element {
    let goals = &ui.payload.state.goals.active;
    let selected_goal_id = ui.payload.state.goals.selected_goal_id.as_ref();
    let selected_goal = selected_goal_id
        .and_then(|goal_id| goals.iter().find(|goal| &goal.goal_id == goal_id))
        .or_else(|| goals.first());
    let goal_plan = selected_goal.and_then(|goal| goal.goal_plan.as_ref());
    let goal_title = goal_plan
        .map(|plan| plan.goal.title.clone())
        .unwrap_or_else(|| "Untitled goal".to_string());
    let goal_statement = goal_plan
        .map(|plan| plan.goal.statement.clone())
        .unwrap_or_else(|| "pending".to_string());
    let goal_origin = goal_plan
        .map(|plan| {
            if plan.goal.inferred {
                "inferred"
            } else {
                "explicit"
            }
            .to_string()
        })
        .unwrap_or_else(|| "pending".to_string());
    let step_count = goal_plan
        .map(|plan| plan.implementation_plan.steps.len())
        .unwrap_or(0);
    let required_test_count = goal_plan
        .map(|plan| plan.test_plan.required_tests.len())
        .unwrap_or(0);
    let non_goal_count = goal_plan.map(|plan| plan.goal.non_goals.len()).unwrap_or(0);
    let goal_id = selected_goal
        .map(|goal| goal.goal_id.clone())
        .unwrap_or_default();
    let goal_plan_state = if goal_plan.is_some() {
        "plan ready"
    } else {
        "plan pending"
    };
    let goal_plan_tone = if goal_plan.is_some() {
        "bg-evidence-pass"
    } else {
        "bg-evidence-pending"
    };
    if selected_goal.is_some() {
        rsx! {
            div { class: "space-y-3",
                div { class: "rounded-2xl border border-border bg-panel p-4 shadow-sm",
                    div { class: "flex flex-wrap items-start justify-between gap-3",
                        div { class: "space-y-1",
                            p { class: "text-[10px] uppercase tracking-[0.24em] text-foreground/45", "goal" }
                            h3 { class: "text-base font-semibold text-foreground", "{goal_title}" }
                            p { class: "text-sm text-foreground/65", "{goal_id}" }
                        }
                        div { class: "flex items-center gap-2",
                            StatusDot { tone_class: goal_plan_tone, label: goal_plan_state.to_string() }
                            HelpLink { ui: ui.clone(), active_pane: WorkbenchPane::Goals, sidebar_open: true, topic: HelpTopic::Goals }
                        }
                    }
                    div { class: "mt-4 grid gap-3 lg:grid-cols-[1.2fr_0.8fr]",
                        div { class: "rounded-2xl border border-border bg-background p-4",
                            div { class: "flex items-center justify-between gap-3",
                                p { class: "text-[10px] uppercase tracking-[0.24em] text-foreground/45", "statement" }
                                ScopeChip { label: "goal path".to_string() }
                            }
                            p { class: "mt-3 text-base leading-7 text-foreground", "{goal_statement}" }
                            div { class: "mt-4 grid gap-3 sm:grid-cols-3",
                                div { class: "rounded-xl border border-border bg-panel-muted p-3",
                                    p { class: "text-[10px] uppercase tracking-[0.24em] text-foreground/45", "plan steps" }
                                    p { class: "mt-1 text-2xl font-semibold text-foreground", "{step_count}" }
                                }
                                div { class: "rounded-xl border border-border bg-panel-muted p-3",
                                    p { class: "text-[10px] uppercase tracking-[0.24em] text-foreground/45", "tests" }
                                    p { class: "mt-1 text-2xl font-semibold text-foreground", "{required_test_count}" }
                                }
                                div { class: "rounded-xl border border-border bg-panel-muted p-3",
                                    p { class: "text-[10px] uppercase tracking-[0.24em] text-foreground/45", "non-goals" }
                                    p { class: "mt-1 text-2xl font-semibold text-foreground", "{non_goal_count}" }
                                }
                            }
                        }
                        div { class: "space-y-3",
                            div { class: "rounded-2xl border border-border bg-background p-3",
                                p { class: "text-[10px] uppercase tracking-[0.24em] text-foreground/45", "origin" }
                                div { class: "mt-2 flex flex-wrap gap-2",
                                    ScopeChip { label: goal_origin }
                                    ScopeChip { label: if goal_plan.is_some() { "plan ready".to_string() } else { "plan pending".to_string() } }
                                }
                            }
                            div { class: "rounded-2xl border border-border bg-background p-3",
                                p { class: "text-[10px] uppercase tracking-[0.24em] text-foreground/45", "title" }
                                select {
                                    class: "mt-2 w-full rounded-xl border border-border bg-panel-muted px-3 py-2 text-sm outline-none",
                                    disabled: true,
                                    option { selected: true, "{goal_title}" }
                                }
                            }
                            div { class: "rounded-2xl border border-border bg-background p-3",
                                p { class: "text-[10px] uppercase tracking-[0.24em] text-foreground/45", "tests" }
                                select {
                                    class: "mt-2 w-full rounded-xl border border-border bg-panel-muted px-3 py-2 text-sm outline-none",
                                    disabled: true,
                                    option { selected: true, "{required_test_count} required" }
                                }
                            }
                        }
                    }
                    div { class: "mt-4 grid gap-3 lg:grid-cols-3",
                        div { class: "rounded-xl border border-border bg-background p-3",
                            p { class: "text-xs uppercase tracking-[0.18em] text-foreground/55", "plan" }
                            div { class: "mt-2 h-2 rounded-full bg-panel-muted" }
                            div { class: "mt-3 h-2 w-4/5 rounded-full bg-panel-muted" }
                            div { class: "mt-3 h-2 w-2/3 rounded-full bg-panel-muted" }
                        }
                        div { class: "rounded-xl border border-border bg-background p-3",
                            p { class: "text-xs uppercase tracking-[0.18em] text-foreground/55", "tests" }
                            div { class: "mt-2 h-2 rounded-full bg-panel-muted" }
                            div { class: "mt-3 h-2 w-3/4 rounded-full bg-panel-muted" }
                            div { class: "mt-3 h-2 w-1/2 rounded-full bg-panel-muted" }
                        }
                        div { class: "rounded-xl border border-border bg-background p-3",
                            p { class: "text-xs uppercase tracking-[0.18em] text-foreground/55", "limits" }
                            div { class: "mt-2 h-2 rounded-full bg-panel-muted" }
                            div { class: "mt-3 h-2 w-5/6 rounded-full bg-panel-muted" }
                            div { class: "mt-3 h-2 w-1/2 rounded-full bg-panel-muted" }
                        }
                    }
                }
            }
        }
    } else {
        rsx! {
            div { class: "space-y-3",
                div { class: "rounded-2xl border border-border bg-panel p-4 shadow-sm",
                    div { class: "flex items-start justify-between gap-3",
                        div { class: "space-y-2",
                            p { class: "text-[10px] uppercase tracking-[0.24em] text-foreground/45", "No goal yet" }
                            p { class: "text-sm text-foreground/65", "A goal card appears here once planning starts." }
                        }
                        HelpLink { ui: ui.clone(), active_pane: WorkbenchPane::Goals, sidebar_open: true, topic: HelpTopic::Goals }
                    }
                    div { class: "mt-4 grid gap-3 lg:grid-cols-[1.2fr_0.8fr]",
                        div { class: "rounded-2xl border border-dashed border-border bg-background p-4",
                            div { class: "flex items-center justify-between gap-3",
                                p { class: "text-xs uppercase tracking-[0.18em] text-foreground/55", "statement" }
                                div { class: "h-6 w-20 rounded-full bg-panel-muted" }
                            }
                            div { class: "mt-3 h-24 rounded-xl bg-panel-muted" }
                        }
                        div { class: "space-y-3",
                            div { class: "rounded-xl border border-border bg-background p-3",
                                p { class: "text-xs uppercase tracking-[0.18em] text-foreground/55", "origin" }
                                div { class: "mt-2 h-10 rounded-lg bg-panel-muted" }
                            }
                            div { class: "rounded-xl border border-border bg-background p-3",
                                p { class: "text-xs uppercase tracking-[0.18em] text-foreground/55", "title" }
                                div { class: "mt-2 h-10 rounded-lg bg-panel-muted" }
                            }
                            div { class: "rounded-xl border border-border bg-background p-3",
                                p { class: "text-xs uppercase tracking-[0.18em] text-foreground/55", "tests" }
                                div { class: "mt-2 h-10 rounded-lg bg-panel-muted" }
                            }
                        }
                    }
                    div { class: "mt-4 grid gap-3 lg:grid-cols-3",
                        div { class: "rounded-xl border border-dashed border-border bg-background p-3",
                            p { class: "text-xs uppercase tracking-[0.18em] text-foreground/55", "plan" }
                            div { class: "mt-2 h-2 rounded-full bg-panel-muted" }
                            div { class: "mt-3 h-2 w-4/5 rounded-full bg-panel-muted" }
                        }
                        div { class: "rounded-xl border border-dashed border-border bg-background p-3",
                            p { class: "text-xs uppercase tracking-[0.18em] text-foreground/55", "tests" }
                            div { class: "mt-2 h-2 rounded-full bg-panel-muted" }
                            div { class: "mt-3 h-2 w-3/4 rounded-full bg-panel-muted" }
                        }
                        div { class: "rounded-xl border border-dashed border-border bg-background p-3",
                            p { class: "text-xs uppercase tracking-[0.18em] text-foreground/55", "limits" }
                            div { class: "mt-2 h-2 rounded-full bg-panel-muted" }
                            div { class: "mt-3 h-2 w-2/3 rounded-full bg-panel-muted" }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn RequestOverview(ui: WorkbenchUiState) -> Element {
    let request = ui.payload.state.request.clone();
    if let Some(request) = request {
        let request_text = request
            .artifact
            .as_ref()
            .map(|artifact| artifact.request.clone())
            .unwrap_or_else(|| "request".to_string());
        let classification = request
            .classification
            .as_ref()
            .map(|classification| classification.classification.label().to_string())
            .unwrap_or_else(|| "pending".to_string());
        let scope_notes = request
            .scope
            .as_ref()
            .map(|scope| scope.notes.clone())
            .unwrap_or_default();
        let scope_requirements = request
            .scope
            .as_ref()
            .map(|scope| scope.requirements.len())
            .unwrap_or(0);
        let scope_features = request
            .scope
            .as_ref()
            .map(|scope| scope.features.len())
            .unwrap_or(0);
        let scope_policies = request
            .scope
            .as_ref()
            .map(|scope| scope.policies.len())
            .unwrap_or(0);
        let scope_philosophies = request
            .scope
            .as_ref()
            .map(|scope| scope.philosophies.len())
            .unwrap_or(0);
        let scope_ready = request.scope.is_some();
        let scope_note_text = scope_notes
            .first()
            .cloned()
            .unwrap_or_else(|| "Classify the request to open the scope view.".to_string());
        let scope_status_label = if scope_ready { "ready" } else { "no scope yet" };
        let request_tone = if request.classification.is_some() {
            "bg-evidence-pass"
        } else {
            "bg-evidence-pending"
        };
        let request_artifact_text = request
            .artifact
            .as_ref()
            .map(|artifact| artifact.request.clone())
            .unwrap_or_else(|| "none".to_string());
        rsx! {
            div { class: "space-y-3",
                div { class: "rounded-2xl border border-border bg-panel p-4 shadow-sm",
                    div { class: "flex flex-wrap items-start justify-between gap-3",
                        div { class: "space-y-1",
                            p { class: "text-[10px] uppercase tracking-[0.24em] text-foreground/45", "request" }
                            h3 { class: "text-base font-semibold text-foreground", "{request_text}" }
                        }
                        div { class: "flex items-center gap-2",
                            StatusDot { tone_class: request_tone, label: classification.clone() }
                            HelpLink { ui: ui.clone(), active_pane: WorkbenchPane::Request, sidebar_open: true, topic: HelpTopic::Request }
                        }
                    }
                    div { class: "mt-4 grid gap-3 lg:grid-cols-[0.9fr_1.1fr]",
                        div { class: "space-y-3",
                            div { class: "rounded-2xl border border-border bg-background p-4",
                                p { class: "text-[10px] uppercase tracking-[0.24em] text-foreground/45", "inbox" }
                                select {
                                    class: "mt-2 w-full rounded-xl border border-border bg-panel-muted px-3 py-2 text-sm outline-none",
                                    disabled: true,
                                    option { selected: true, "{request_artifact_text}" }
                                }
                            }
                            div { class: "rounded-2xl border border-border bg-background p-4",
                                p { class: "text-[10px] uppercase tracking-[0.24em] text-foreground/45", "classifier" }
                                select {
                                    class: "mt-2 w-full rounded-xl border border-border bg-panel-muted px-3 py-2 text-sm outline-none",
                                    disabled: true,
                                    option { selected: true, "{classification}" }
                                }
                            }
                        }
                        div { class: "rounded-2xl border border-border bg-background p-4",
                            div { class: "flex items-center justify-between gap-3",
                                p { class: "text-[10px] uppercase tracking-[0.24em] text-foreground/45", "scope" }
                                ScopeChip { label: scope_status_label.to_string() }
                            }
                            p { class: "mt-3 text-sm text-foreground/75", "{scope_note_text}" }
                            div { class: "mt-4 grid gap-3 md:grid-cols-2",
                                MiniSelect { label: "requirements".to_string(), value: scope_requirements.to_string() }
                                MiniSelect { label: "features".to_string(), value: scope_features.to_string() }
                                MiniSelect { label: "policies".to_string(), value: scope_policies.to_string() }
                                MiniSelect { label: "philosophies".to_string(), value: scope_philosophies.to_string() }
                            }
                        }
                    }
                }
            }
        }
    } else {
        rsx! {
            div { class: "space-y-3",
                div { class: "rounded-2xl border border-border bg-panel p-4 shadow-sm",
                    div { class: "flex items-start justify-between gap-3",
                        div { class: "space-y-2",
                            p { class: "text-[10px] uppercase tracking-[0.24em] text-foreground/45", "No request yet" }
                            p { class: "text-sm text-foreground/65", "A request card appears here after intake." }
                        }
                        HelpLink { ui: ui.clone(), active_pane: WorkbenchPane::Request, sidebar_open: true, topic: HelpTopic::Request }
                    }
                    div { class: "mt-4 grid gap-3 lg:grid-cols-[0.9fr_1.1fr]",
                        div { class: "space-y-3",
                            div { class: "rounded-2xl border border-dashed border-border bg-background p-4",
                                p { class: "text-xs uppercase tracking-[0.18em] text-foreground/55", "inbox" }
                                div { class: "mt-3 h-10 rounded-xl bg-panel-muted" }
                            }
                            div { class: "rounded-2xl border border-dashed border-border bg-background p-4",
                                p { class: "text-xs uppercase tracking-[0.18em] text-foreground/55", "classifier" }
                                div { class: "mt-3 h-10 rounded-xl bg-panel-muted" }
                            }
                        }
                        div { class: "rounded-2xl border border-dashed border-border bg-background p-4",
                            div { class: "flex items-center justify-between gap-3",
                                p { class: "text-xs uppercase tracking-[0.18em] text-foreground/55", "scope" }
                                div { class: "h-6 w-16 rounded-full bg-panel-muted" }
                            }
                            div { class: "mt-3 h-24 rounded-xl bg-panel-muted" }
                        }
                    }
                    div { class: "mt-4 grid gap-3 md:grid-cols-2 xl:grid-cols-4",
                        for label in ["requirements", "features", "policies", "philosophies"] {
                            div { class: "rounded-xl border border-dashed border-border bg-background p-3",
                                p { class: "text-xs uppercase tracking-[0.18em] text-foreground/55", "{label}" }
                                div { class: "mt-2 h-8 rounded-lg bg-panel-muted" }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn BranchOverview(ui: WorkbenchUiState) -> Element {
    let report = ui
        .payload
        .state
        .branch_scope
        .as_ref()
        .and_then(|state| state.report.as_ref());
    if let Some(report) = report {
        let ownership_status_label = |status: OwnershipStatus| match status {
            OwnershipStatus::Owned => "owned",
            OwnershipStatus::Partial => "partial",
            OwnershipStatus::Unowned => "unowned",
        };
        rsx! {
            div { class: "space-y-3",
                div { class: "rounded-2xl border border-border bg-panel p-4 shadow-sm",
                    div { class: "flex flex-wrap items-start justify-between gap-3",
                        div { class: "space-y-1",
                            p { class: "text-[10px] uppercase tracking-[0.24em] text-foreground/45", "branch" }
                            h3 { class: "text-base font-semibold text-foreground", "{report.range.clone()}" }
                        }
                        div { class: "flex items-center gap-2",
                            StatusDot { tone_class: "bg-evidence-pass", label: report.confidence.label().to_string() }
                            HelpLink { ui: ui.clone(), active_pane: WorkbenchPane::Branch, sidebar_open: true, topic: HelpTopic::Branch }
                        }
                    }
                    div { class: "mt-4 grid gap-3 lg:grid-cols-[1.1fr_0.9fr]",
                        div { class: "rounded-2xl border border-border bg-background p-3",
                            p { class: "text-xs uppercase tracking-[0.18em] text-foreground/55", "changed files" }
                            div { class: "mt-3 space-y-2",
                                for file in report.changed_files.iter().take(5) {
                                    button {
                                        class: "w-full rounded-xl border border-border bg-panel px-3 py-3 text-left hover:bg-background",
                                        type: "button",
                                        div { class: "flex items-center justify-between gap-3",
                                            p { class: "text-sm font-medium text-foreground", "{file.file}" }
                                            ScopeChip { label: ownership_status_label(file.status).to_string() }
                                        }
                                        div { class: "mt-2 flex flex-wrap gap-2",
                                            ScopeChip { label: if file.is_spec_file { "spec".to_string() } else { "code".to_string() } }
                                            ScopeChip { label: format!("{} symbols", file.symbols.len()) }
                                        }
                                    }
                                }
                            }
                        }
                        div { class: "grid gap-3 sm:grid-cols-3 lg:grid-cols-1",
                            MiniSelect { label: "files".to_string(), value: report.changed_files.len().to_string() }
                            MiniSelect { label: "specs".to_string(), value: report.spec_impact.affected_items.len().to_string() }
                            MiniSelect { label: "risk".to_string(), value: report.repo_risk.level.clone() }
                        }
                    }
                    div { class: "mt-4 rounded-2xl border border-border bg-background p-3",
                        SpecImpactGraph { ui: ui.clone() }
                    }
                }
            }
        }
    } else {
        rsx! {
            div { class: "space-y-3",
                EmptyState { title: "No branch scope".to_string(), body: "Load scope to see the diff and affected surface.".to_string() }
            }
        }
    }
}

#[component]
fn AssignmentOverview(ui: WorkbenchUiState) -> Element {
    let assignment = ui.payload.state.assignment.clone();
    if let Some(assignment) = assignment {
        rsx! {
            div { class: "space-y-3",
                div { class: "rounded-2xl border border-border bg-panel p-4 shadow-sm",
                    div { class: "flex flex-wrap items-start justify-between gap-3",
                        div { class: "space-y-1",
                            p { class: "text-[10px] uppercase tracking-[0.24em] text-foreground/45", "assignment" }
                            h3 { class: "text-base font-semibold text-foreground", "{assignment.assignee.as_ref().map(|assignee| assignee.display_name.clone()).unwrap_or_else(|| \"unassigned\".to_string())}" }
                        }
                        div { class: "flex items-center gap-2",
                            StatusDot {
                                tone_class: assignment_status_tone(assignment.status),
                                label: assignment.status.label().to_string(),
                            }
                            HelpLink { ui: ui.clone(), active_pane: WorkbenchPane::Assignment, sidebar_open: true, topic: HelpTopic::Assignment }
                        }
                    }
                    div { class: "mt-4 grid gap-3 lg:grid-cols-[0.9fr_1.1fr]",
                        div { class: "space-y-3",
                            AssigneeSelector { assignee: assignment.assignee.clone() }
                            div { class: "rounded-xl border border-border bg-background/30 p-3",
                                p { class: "text-xs uppercase tracking-[0.18em] text-foreground/55", "run mode" }
                                MiniSelect { label: "mode".to_string(), value: assignment.run_mode.label().to_string() }
                                MiniSelect { label: "evidence".to_string(), value: if assignment.evidence_requirements.is_empty() { "none".to_string() } else { format!("{} items", assignment.evidence_requirements.len()) } }
                            }
                        }
                        ScopeGuardPreview { result: assignment.scope_guard.clone() }
                    }
                    div { class: "mt-4 grid gap-3 lg:grid-cols-2",
                        AssignmentPromptPreview { assignment: assignment.clone() }
                        AssignmentEvidencePanel { assignment: assignment.clone() }
                    }
                    div { class: "mt-4",
                        AssignmentConstraintPanel { assignment: assignment.clone() }
                    }
                }
            }
        }
    } else {
        rsx! {
            div { class: "space-y-3",
                div { class: "rounded-2xl border border-border bg-panel p-4 shadow-sm",
                    div { class: "flex items-start justify-between gap-3",
                        div { class: "space-y-2",
                            p { class: "text-[10px] uppercase tracking-[0.24em] text-foreground/45", "No assignment yet" }
                            p { class: "text-sm text-foreground/65", "A handoff appears here once a goal is scoped." }
                        }
                        ScopeChip { label: "handoff".to_string() }
                    }
                    div { class: "mt-4 grid gap-3 lg:grid-cols-[0.9fr_1.1fr]",
                        div { class: "space-y-3",
                            div { class: "rounded-xl border border-dashed border-border bg-background p-3",
                                p { class: "text-xs uppercase tracking-[0.18em] text-foreground/55", "assignee" }
                                div { class: "mt-2 h-12 rounded-lg bg-panel-muted" }
                            }
                            div { class: "rounded-xl border border-dashed border-border bg-background p-3",
                                p { class: "text-xs uppercase tracking-[0.18em] text-foreground/55", "run mode" }
                                div { class: "mt-2 h-12 rounded-lg bg-panel-muted" }
                            }
                        }
                        div { class: "rounded-2xl border border-dashed border-border bg-background p-4",
                            p { class: "text-xs uppercase tracking-[0.18em] text-foreground/55", "scope guard" }
                            div { class: "mt-3 h-28 rounded-xl bg-panel-muted" }
                        }
                    }
                    div { class: "mt-4 grid gap-3 lg:grid-cols-2",
                        div { class: "rounded-2xl border border-dashed border-border bg-background p-4",
                            p { class: "text-xs uppercase tracking-[0.18em] text-foreground/55", "prompt" }
                            div { class: "mt-3 h-24 rounded-xl bg-panel-muted" }
                        }
                        div { class: "rounded-2xl border border-dashed border-border bg-background p-4",
                            p { class: "text-xs uppercase tracking-[0.18em] text-foreground/55", "evidence" }
                            div { class: "mt-3 h-24 rounded-xl bg-panel-muted" }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn GraphOverview(ui: WorkbenchUiState) -> Element {
    let report = ui
        .payload
        .state
        .branch_scope
        .as_ref()
        .and_then(|state| state.report.as_ref());
    let ready = report.is_some();
    let graph_status_label = if ready { "ready" } else { "waiting" };
    let graph_tone = if ready {
        "bg-evidence-pass"
    } else {
        "bg-evidence-pending"
    };
    let node_count = report
        .map(|report| report.spec_impact_graph.nodes.len())
        .unwrap_or(0);
    let edge_count = report
        .map(|report| report.spec_impact_graph.edges.len())
        .unwrap_or(0);
    if ready {
        rsx! {
            div { class: "space-y-3",
                div { class: "rounded-2xl border border-border bg-panel p-4 shadow-sm",
                    div { class: "flex flex-wrap items-start justify-between gap-3",
                        div { class: "space-y-1",
                            p { class: "text-[10px] uppercase tracking-[0.24em] text-foreground/45", "spec map" }
                            h3 { class: "text-base font-semibold text-foreground", "Spec graph" }
                        }
                        div { class: "flex items-center gap-2",
                            StatusDot { tone_class: graph_tone, label: graph_status_label.to_string() }
                            HelpLink { ui: ui.clone(), active_pane: WorkbenchPane::Graph, sidebar_open: true, topic: HelpTopic::Graph }
                        }
                    }
                    div { class: "mt-4 grid gap-3 md:grid-cols-3",
                        MiniSelect { label: "nodes".to_string(), value: node_count.to_string() }
                        MiniSelect { label: "edges".to_string(), value: edge_count.to_string() }
                        MiniSelect { label: "view".to_string(), value: "interactive map".to_string() }
                    }
                    div { class: "mt-4 rounded-2xl border border-border bg-background p-3",
                        SpecImpactGraph { ui: ui.clone() }
                    }
                }
            }
        }
    } else {
        rsx! {
            div { class: "space-y-3",
                div { class: "rounded-2xl border border-border bg-panel p-4 shadow-sm",
                    div { class: "flex flex-wrap items-start justify-between gap-3",
                        div { class: "space-y-1",
                            p { class: "text-[10px] uppercase tracking-[0.24em] text-foreground/45", "spec map" }
                            h3 { class: "text-base font-semibold text-foreground", "Spec graph" }
                        }
                        div { class: "flex items-center gap-2",
                            StatusDot { tone_class: graph_tone, label: graph_status_label.to_string() }
                            HelpLink { ui: ui.clone(), active_pane: WorkbenchPane::Graph, sidebar_open: true, topic: HelpTopic::Graph }
                        }
                    }
                    div { class: "mt-4 grid gap-3 md:grid-cols-3",
                        MiniSelect { label: "nodes".to_string(), value: "0".to_string() }
                        MiniSelect { label: "edges".to_string(), value: "0".to_string() }
                        MiniSelect { label: "view".to_string(), value: "interactive map".to_string() }
                    }
                    div { class: "mt-4 rounded-lg border border-dashed border-border bg-background p-4",
                        div { class: "grid gap-3 md:grid-cols-[minmax(0,1fr)_14rem]",
                            EmptyState {
                                title: "Branch scope not loaded".to_string(),
                                body: "Use the command palette to load the workspace graph.".to_string(),
                            }
                            a {
                                class: "flex items-center justify-center rounded-lg border border-border bg-panel px-3 py-2 text-sm font-medium text-foreground/80 hover:bg-panel-muted",
                                href: "?pane=commands&sidebar=0&query=branch%20scope&action=branch.scope",
                                "Load branch scope"
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn EvidenceOverview(ui: WorkbenchUiState) -> Element {
    let records = ui
        .payload
        .state
        .evidence_timeline
        .entries
        .iter()
        .rev()
        .take(3)
        .cloned()
        .collect::<Vec<_>>();
    let latest_record = records.first().cloned();
    if let Some(record) = latest_record {
        let record_status = record.status.label().to_string();
        let record_source = evidence_source_label(&record);
        let record_time = format_timestamp_ms(record.timestamp);
        let record_command = record.command.clone();
        let record_attachment = record.attachments.first().cloned();
        rsx! {
            div { class: "space-y-3",
                div { class: "rounded-2xl border border-border bg-panel p-4 shadow-sm",
                    div { class: "flex flex-wrap items-start justify-between gap-3",
                        div { class: "space-y-1",
                            p { class: "text-[10px] uppercase tracking-[0.24em] text-foreground/45", "activity" }
                            h3 { class: "text-base font-semibold text-foreground", "{record.summary}" }
                        }
                        div { class: "flex items-center gap-2",
                            EvidenceBadge { kind: record.kind }
                            HelpLink { ui: ui.clone(), active_pane: WorkbenchPane::Evidence, sidebar_open: true, topic: HelpTopic::Evidence }
                        }
                    }
                    div { class: "mt-4 grid gap-3 md:grid-cols-3",
                        MiniSelect { label: "status".to_string(), value: record_status }
                        MiniSelect { label: "source".to_string(), value: record_source }
                        MiniSelect { label: "time".to_string(), value: record_time }
                    }
                    if record_command.is_some() {
                        CommandOutputView {
                            title: "linked command".to_string(),
                            summary: record.summary.clone(),
                            command: record_command,
                            attachment: record_attachment,
                        }
                    }
                }
                if !records.is_empty() {
                    div { class: "space-y-2",
                        for record in records.into_iter().skip(1) {
                            EvidenceRecordCard { record }
                        }
                    }
                }
            }
        }
    } else {
        rsx! {
            div { class: "space-y-3",
                div { class: "rounded-2xl border border-border bg-panel p-4 shadow-sm",
                    div { class: "flex flex-wrap items-start justify-between gap-3",
                        div { class: "space-y-1",
                            p { class: "text-[10px] uppercase tracking-[0.24em] text-foreground/45", "activity" }
                            h3 { class: "text-base font-semibold text-foreground", "Evidence" }
                        }
                        div { class: "flex items-center gap-2",
                            EvidenceBadge { kind: syu_workbench::WorkbenchEvidenceKind::AssignmentState }
                            HelpLink { ui: ui.clone(), active_pane: WorkbenchPane::Evidence, sidebar_open: true, topic: HelpTopic::Evidence }
                        }
                    }
                    div { class: "mt-4 space-y-3",
                        div { class: "rounded-2xl border border-dashed border-border bg-background p-4",
                            div { class: "flex items-center justify-between gap-3",
                                div { class: "h-2 w-28 rounded-full bg-panel-muted" }
                                div { class: "h-2 w-16 rounded-full bg-panel-muted" }
                            }
                            div { class: "mt-4 space-y-3",
                                div { class: "h-11 rounded-xl bg-panel-muted" }
                                div { class: "h-11 rounded-xl bg-panel-muted" }
                            }
                        }
                        div { class: "grid gap-3 md:grid-cols-3",
                            div { class: "rounded-2xl border border-dashed border-border bg-background p-3",
                                p { class: "text-xs uppercase tracking-[0.18em] text-foreground/55", "status" }
                                div { class: "mt-2 h-10 rounded-lg bg-panel-muted" }
                            }
                            div { class: "rounded-2xl border border-dashed border-border bg-background p-3",
                                p { class: "text-xs uppercase tracking-[0.18em] text-foreground/55", "source" }
                                div { class: "mt-2 h-10 rounded-lg bg-panel-muted" }
                            }
                            div { class: "rounded-2xl border border-dashed border-border bg-background p-3",
                                p { class: "text-xs uppercase tracking-[0.18em] text-foreground/55", "time" }
                                div { class: "mt-2 h-10 rounded-lg bg-panel-muted" }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn evidence_source_label(record: &EvidenceRecord) -> String {
    match record.source.as_ref() {
        Some(EvidenceSource::Action {
            action_label,
            action_id,
        }) => action_label
            .clone()
            .or_else(|| action_id.as_ref().map(|id| id.label().to_string()))
            .unwrap_or_else(|| "action".to_string()),
        Some(EvidenceSource::Command { command }) => command.clone(),
        Some(EvidenceSource::Manual { actor }) => actor.clone(),
        Some(EvidenceSource::System { component }) => component.clone(),
        None => "system".to_string(),
    }
}

fn format_timestamp_ms(timestamp_ms: u64) -> String {
    const MILLIS_PER_SECOND: i64 = 1_000;
    const SECONDS_PER_MINUTE: i64 = 60;
    const MINUTES_PER_HOUR: i64 = 60;
    const HOURS_PER_DAY: i64 = 24;
    const SECONDS_PER_DAY: i64 = SECONDS_PER_MINUTE * MINUTES_PER_HOUR * HOURS_PER_DAY;

    let total_seconds = (timestamp_ms / MILLIS_PER_SECOND as u64) as i64;
    let seconds_of_day = total_seconds.rem_euclid(SECONDS_PER_DAY);
    let days = total_seconds.div_euclid(SECONDS_PER_DAY);

    let (year, month, day) = civil_from_days(days);
    let hour = seconds_of_day / (SECONDS_PER_MINUTE * MINUTES_PER_HOUR);
    let minute = (seconds_of_day % (SECONDS_PER_MINUTE * MINUTES_PER_HOUR)) / SECONDS_PER_MINUTE;
    let second = seconds_of_day % SECONDS_PER_MINUTE;

    format!("{year:04}-{month:02}-{day:02} {hour:02}:{minute:02}:{second:02} UTC")
}

fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let days = days + 719_468;
    let era = if days >= 0 { days } else { days - 146_096 } / 146_097;
    let doe = days - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let mut year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    year += if month <= 2 { 1 } else { 0 };
    (year, month, day)
}

#[component]
fn MetricTile(label: String, value: String) -> Element {
    rsx! {
        div { class: "rounded-xl border border-border bg-background p-3",
            p { class: "text-[10px] uppercase tracking-[0.24em] text-foreground/45", "{label}" }
            p { class: "mt-1 text-sm font-medium text-foreground", "{value}" }
        }
    }
}

#[component]
fn MiniSelect(label: String, value: String) -> Element {
    rsx! {
        div { class: "space-y-1",
            p { class: "text-[10px] uppercase tracking-[0.24em] text-foreground/45", "{label}" }
            select { class: "w-full rounded-xl border border-border bg-background px-3 py-2 text-sm outline-none", disabled: true, option { selected: true, "{value}" } }
        }
    }
}

#[component]
pub fn CommandPalette(ui: WorkbenchUiState, active_pane: WorkbenchPane) -> Element {
    let entries = ui.visible_actions();
    let cli_entries = ui.visible_cli_commands();
    let has_entries = !entries.is_empty() || !cli_entries.is_empty();
    let copy = ui.copy();
    rsx! {
        form { class: "group relative", action: "/", method: "get", "data-command-palette": "true",
            input { type: "hidden", name: "pane", value: active_pane.slug() }
            input { type: "hidden", name: "sidebar", value: "0" }
            div { class: "flex items-center gap-2",
                div { class: "relative min-w-0 flex-1",
                    span { class: "pointer-events-none absolute left-3 top-1/2 -translate-y-1/2 text-sm text-foreground/50", "⌘" }
                    input {
                        class: "w-full rounded-lg border border-border bg-background py-2.5 pl-10 pr-12 text-sm shadow-sm outline-none transition focus:border-foreground/20 focus:shadow-[0_0_0_4px_rgba(15,23,42,0.04)]",
                        value: "{ui.command_query}",
                        name: "query",
                        placeholder: copy.palette_placeholder(),
                        autocomplete: "off",
                        spellcheck: "false",
                        "data-command-input": "true",
                    }
                }
            }
            div { class: "command-palette-results absolute left-0 right-0 top-[calc(100%+0.5rem)] z-20 hidden max-h-[26rem] grid-cols-1 gap-1 overflow-auto rounded-lg border border-border bg-panel p-1.5 shadow-lg", "data-command-results": "true",
                for entry in entries {
                    CommandItem { entry: entry, selected: false }
                }
                for entry in cli_entries {
                    CliCommandItem { entry }
                }
                if !has_entries {
                    EmptyState { title: "No matches".to_string(), body: copy.palette_hint().to_string() }
                }
            }
        }
    }
}

#[component]
fn CliCommandItem(entry: CliCommandEntry) -> Element {
    let state = if entry.mutates_files {
        "confirm"
    } else if entry.requires_input {
        "input"
    } else {
        "ready"
    };
    rsx! {
        a {
            class: classes::COMMAND_ITEM,
            href: format!("?pane=commands&sidebar=0&cli={}", entry.id),
            title: "{entry.description}",
            "data-command-item": "true",
            "data-command-text": format!("{} {} {} {}", entry.id, entry.title, entry.description, entry.invocation),
            "data-command-id": entry.id,
            "data-command-title": entry.title,
            div { class: "flex items-start gap-3 text-left",
                span { class: "grid h-8 w-8 shrink-0 place-items-center rounded-full border border-border bg-panel-muted text-xs text-foreground/70", "›" }
                div { class: "flex min-w-0 flex-col gap-1",
                    span { class: "text-sm font-medium text-foreground", "{entry.title}" }
                    span { class: "text-[10px] uppercase tracking-[0.24em] text-foreground/45", "{entry.id}" }
                }
            }
            div { class: "flex flex-col items-end gap-1 text-xs uppercase tracking-[0.18em]",
                span { class: "normal-case tracking-normal text-foreground/50", "{state}" }
            }
        }
    }
}

#[component]
pub fn GoalRail(ui: WorkbenchUiState) -> Element {
    rsx! {
        Panel { class: classes::PANEL,
            div { class: classes::PANEL_INNER,
                div { class: classes::SECTION_HEADER,
                    h2 { class: classes::SECTION_TITLE, "Goals" }
                    ScopeChip { label: format!("{}", ui.payload.state.goals.active.len()) }
                }
                div { class: classes::SECTION_BODY,
                    if ui.payload.state.goals.active.is_empty() {
                        EmptyState { title: "None".to_string(), body: "The first goal appears here." }
                    } else {
                        for goal in &ui.payload.state.goals.active {
                            GoalCard {
                                goal_id: goal.goal_id.clone(),
                                title: goal_title(goal),
                                selected: ui.payload.state.goals.selected_goal_id.as_ref() == Some(&goal.goal_id),
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub fn GoalCanvas(
    ui: WorkbenchUiState,
    on_run_action: Option<EventHandler<WorkbenchActionId>>,
) -> Element {
    let summary = ui.pulse_summary();
    rsx! {
        Panel { class: classes::PANEL,
            div { class: classes::PANEL_INNER,
                WorkspacePulse { summary: summary.clone() }
                RequestIntakeCanvas {
                    ui: ui.clone(),
                    on_run_action: on_run_action,
                }
                GoalPlanCanvas {
                    ui: ui.clone(),
                    on_run_action: on_run_action,
                }
                BranchScopeLens { ui: ui.clone(), on_run_action: on_run_action }
                AssignGoalDialog { ui: ui.clone(), on_run_action: on_run_action }
                SpecImpactGraph { ui: ui.clone() }
                if let Some(preview) = ui.preview.clone().or_else(|| ui.selected_action_id.and_then(|id| ui.action_preview(id))) {
                    DetailDrawer {
                        title: preview.title.clone(),
                        body: preview.result_summary.clone(),
                        evidence: preview.evidence_summary.clone(),
                    }
                } else if let Some(action) = ui.selected_action() {
                    DetailDrawer {
                        title: action.title.clone(),
                        body: action.description.clone(),
                        evidence: format!("ready for {}", action.evidence_kind.label()),
                    }
                } else {
                    EmptyState { title: "No preview selected".to_string(), body: "Open the palette to inspect a command or preview the result.".to_string() }
                }
            }
        }
    }
}

#[component]
pub fn AssignGoalDialog(
    ui: WorkbenchUiState,
    on_run_action: Option<EventHandler<WorkbenchActionId>>,
) -> Element {
    let assignment = ui.payload.state.assignment.clone();
    let is_automated_assignee = assignment
        .as_ref()
        .is_some_and(assignment_has_automated_assignee);
    rsx! {
        Panel { class: classes::PANEL_MUTED,
            div { class: "space-y-4 p-4",
                div { class: classes::SECTION_HEADER,
                    h2 { class: classes::SECTION_TITLE, "Scoped Assignment" }
                    if let Some(assignment) = &assignment {
                        StatusDot {
                            tone_class: assignment_status_tone(assignment.status),
                            label: assignment.status.label().to_string(),
                        }
                    } else {
                        ScopeChip { label: "assignment-blocked".to_string() }
                    }
                }
                if let Some(assignment) = assignment {
                    AssigneeSelector { assignee: assignment.assignee.clone() }
                    ScopeGuardPreview { result: assignment.scope_guard.clone() }
                    AssignmentConstraintPanel { assignment: assignment.clone() }
                    AssignmentPromptPreview { assignment: assignment.clone() }
                    if let Some(run) = assignment.latest_run.clone() {
                        AgentRunPanel { run }
                    } else if matches!(assignment.assignee.as_ref().map(|assignee| assignee.kind), Some(AssigneeKind::Human)) {
                        HumanAssignmentPanel { assignment: assignment.clone() }
                    }
                    AssignmentEvidencePanel { assignment: assignment.clone() }
                    if let Some(on_run_action) = on_run_action {
                        div { class: "flex flex-wrap gap-2",
                            button {
                                class: "rounded-full border border-border bg-panel-muted px-3 py-1.5 text-xs uppercase tracking-[0.16em] text-foreground/70",
                                disabled: !assignment.is_runnable(),
                                onclick: move |_| on_run_action.call(WorkbenchActionId::AssignmentPreview),
                                "Preview"
                            }
                            if is_automated_assignee {
                                button {
                                    class: "rounded-full border border-command-active bg-command-active px-3 py-1.5 text-xs uppercase tracking-[0.16em] text-background",
                                    disabled: !assignment.is_runnable(),
                                    onclick: move |_| on_run_action.call(WorkbenchActionId::AssignmentRunDry),
                                    "Dry Run"
                                }
                            }
                        }
                    }
                } else {
                    EmptyState {
                        title: "No assignment loaded".to_string(),
                        body: "Create assignment keeps Goal scope, non-goals, tests, completion commands, and required evidence together.".to_string()
                    }
                }
            }
        }
    }
}

#[component]
pub fn AssigneeSelector(assignee: Option<Assignee>) -> Element {
    rsx! {
        section { class: "rounded-xl border border-border bg-background/30 p-3",
            p { class: "text-xs uppercase tracking-[0.18em] text-foreground/55", "Assignee Selector" }
            if let Some(assignee) = assignee {
                div { class: "mt-2 flex flex-wrap items-center gap-2",
                    ScopeChip { label: assignee.kind.label().to_string() }
                    ScopeChip { label: assignee.id.clone() }
                    p { class: "text-sm font-medium", "{assignee.display_name}" }
                }
            } else {
                p { class: "mt-2 text-sm text-evidence-warn", "assignment-blocked: assignee missing" }
            }
        }
    }
}

#[component]
pub fn ScopeGuardPreview(result: ScopeGuardResult) -> Element {
    rsx! {
        section { class: "rounded-xl border border-border bg-background/30 p-3",
            div { class: "flex flex-wrap items-center gap-2",
                p { class: "text-xs uppercase tracking-[0.18em] text-foreground/55", "Scope Guard Preview" }
                StatusDot { tone_class: scope_guard_tone(result.status), label: result.status.label().to_string() }
            }
            div { class: "mt-2 flex flex-wrap gap-2",
                ScopeChip { label: "scope-in".to_string() }
                ScopeChip { label: "scope-out".to_string() }
                ScopeChip { label: "out-of-scope changes".to_string() }
                ScopeChip { label: result.status.label().to_string() }
            }
            if !result.out_of_scope_files.is_empty() {
                div { class: "mt-3 space-y-2 rounded-lg border border-evidence-fail/40 bg-evidence-fail/10 p-3",
                    div { class: "flex items-center gap-2",
                        StatusDot { tone_class: "bg-evidence-fail", label: "scope-invalid".to_string() }
                        p { class: "text-sm font-medium text-foreground/80", "Out-of-scope changes" }
                    }
                    for file in result.out_of_scope_files {
                        p { class: "text-sm text-foreground/75", "{file}" }
                    }
                }
            }
            if !result.blockers.is_empty() {
                div { class: "mt-3 space-y-2 rounded-lg border border-evidence-fail/40 bg-evidence-fail/10 p-3",
                    for blocker in result.blockers {
                        div { class: "flex items-center gap-2",
                            StatusDot { tone_class: "bg-evidence-fail", label: "assignment-blocked".to_string() }
                            p { class: "text-sm text-foreground/80", "{blocker.code}: {blocker.message}" }
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub fn AssignmentConstraintPanel(assignment: Assignment) -> Element {
    rsx! {
        section { class: "grid gap-3 md:grid-cols-2",
            ConstraintList { title: "Allowed files".to_string(), token: "scope-in".to_string(), values: assignment.scope.include.clone() }
            ConstraintList { title: "Forbidden files".to_string(), token: "scope-out".to_string(), values: assignment.scope.exclude.clone() }
            ConstraintList { title: "Non-goals".to_string(), token: "assignment-ready".to_string(), values: assignment.scope.non_goals.clone() }
            ConstraintList { title: "Required tests".to_string(), token: "evidence-required".to_string(), values: assignment.scope.required_tests.clone() }
            ConstraintList { title: "Completion commands".to_string(), token: "run-dry".to_string(), values: assignment.scope.completion_commands.clone() }
            ConstraintList { title: "Linked spec context".to_string(), token: "spec-linked".to_string(), values: assignment.scope.linked_spec_context.clone() }
        }
    }
}

#[component]
fn ConstraintList(title: String, token: String, values: Vec<String>) -> Element {
    rsx! {
        div { class: "rounded-xl border border-border bg-background/30 p-3",
            div { class: "flex items-center justify-between gap-2",
                p { class: "text-xs uppercase tracking-[0.18em] text-foreground/55", "{title}" }
                ScopeChip { label: token }
            }
            if values.is_empty() {
                p { class: "mt-2 text-sm text-evidence-warn", "evidence-missing" }
            } else {
                ul { class: "mt-2 space-y-1",
                    for value in values {
                        li { class: "text-sm text-foreground/75", "{value}" }
                    }
                }
            }
        }
    }
}

#[component]
pub fn AssignmentPromptPreview(assignment: Assignment) -> Element {
    rsx! {
        section { class: "rounded-xl border border-border bg-background/30 p-3",
            div { class: "flex flex-wrap items-center gap-2",
                p { class: "text-xs uppercase tracking-[0.18em] text-foreground/55", "Assignment Prompt Preview" }
                ScopeChip { label: assignment.run_mode.label().to_string() }
            }
            pre { class: "mt-2 max-h-56 overflow-auto rounded-lg border border-border bg-panel-muted p-3 text-xs text-foreground/70",
                "{assignment.prompt_preview}"
            }
        }
    }
}

#[component]
pub fn AgentRunPanel(run: AgentRun) -> Element {
    rsx! {
        section { class: "rounded-xl border border-border bg-background/30 p-3",
            div { class: "flex flex-wrap items-center gap-2",
                p { class: "text-xs uppercase tracking-[0.18em] text-foreground/55", "Agent Run Panel" }
                StatusDot { tone_class: agent_run_tone(run.status), label: run.status.label().to_string() }
                ScopeChip { label: run.output.diff_summary.clone() }
            }
            CommandOutputView {
                title: "Runner output".to_string(),
                summary: run.status.label().to_string(),
                command: Some(syu_workbench::EvidenceCommand {
                    command: run.profile_id.clone(),
                    args: vec![run.mode.label().to_string()],
                }),
                attachment: Some(syu_workbench::EvidenceAttachment {
                    label: "stdout-stderr".to_string(),
                    mime_type: Some("text/plain".to_string()),
                    summary: Some("stdout/stderr".to_string()),
                    content: Some(format!("stdout:\n{}\nstderr:\n{}", run.output.stdout, run.output.stderr)),
                    truncated: false,
                }),
            }
        }
    }
}

#[component]
pub fn HumanAssignmentPanel(assignment: Assignment) -> Element {
    rsx! {
        section { class: "rounded-xl border border-border bg-background/30 p-3",
            div { class: "flex flex-wrap items-center gap-2",
                p { class: "text-xs uppercase tracking-[0.18em] text-foreground/55", "Human Assignment Panel" }
                ScopeChip { label: "manual".to_string() }
                ScopeChip { label: assignment.status.label().to_string() }
            }
            p { class: "mt-2 text-sm text-foreground/75", "Human assignment uses the same scoped handoff without command execution." }
        }
    }
}

#[component]
pub fn AssignmentEvidencePanel(assignment: Assignment) -> Element {
    rsx! {
        section { class: "rounded-xl border border-border bg-background/30 p-3",
            div { class: "flex flex-wrap items-center gap-2",
                p { class: "text-xs uppercase tracking-[0.18em] text-foreground/55", "Assignment Evidence Panel" }
                EvidenceBadge { kind: syu_workbench::WorkbenchEvidenceKind::AssignmentState }
            }
            if assignment.evidence_requirements.is_empty() {
                p { class: "mt-2 text-sm text-evidence-warn", "evidence-missing" }
            } else {
                div { class: "mt-2 flex flex-wrap gap-2",
                    for requirement in assignment.evidence_requirements {
                        ScopeChip { label: if requirement.required { "evidence-required".to_string() } else { "evidence-optional".to_string() } }
                        p { class: "text-sm text-foreground/75", "{requirement.description}" }
                    }
                }
            }
        }
    }
}

#[component]
pub fn BranchScopeLens(
    ui: WorkbenchUiState,
    on_run_action: Option<EventHandler<WorkbenchActionId>>,
) -> Element {
    let report = ui
        .payload
        .state
        .branch_scope
        .as_ref()
        .and_then(|state| state.report.clone());
    rsx! {
        Panel { class: classes::PANEL_MUTED,
            div { class: "flex flex-col gap-4 p-4",
                div { class: classes::SECTION_HEADER,
                    h2 { class: classes::SECTION_TITLE, "Branch Scope Lens" }
                    ScopeChip { label: report.as_ref().map(|report| report.range.clone()).unwrap_or_else(|| "range pending".to_string()) }
                }
                div { class: "grid gap-2 md:grid-cols-5",
                    FlowActionButton { label: "Load scope".to_string(), action_id: WorkbenchActionId::BranchScope, ui: ui.clone(), onclick: on_run_action }
                    FlowActionButton { label: "Infer goal".to_string(), action_id: WorkbenchActionId::BranchInferGoal, ui: ui.clone(), onclick: on_run_action }
                    FlowActionButton { label: "Spec impact".to_string(), action_id: WorkbenchActionId::SpecImpact, ui: ui.clone(), onclick: on_run_action }
                    FlowActionButton { label: "Trace range".to_string(), action_id: WorkbenchActionId::TraceRange, ui: ui.clone(), onclick: on_run_action }
                    FlowActionButton { label: "Relate range".to_string(), action_id: WorkbenchActionId::RelateRange, ui: ui.clone(), onclick: on_run_action }
                }
                if let Some(report) = report {
                    ImpactSummaryPanel { report: report.clone() }
                    GoalScopeComparisonPanel {
                        report: report.clone(),
                        plan: ui.payload.state.goals.active_goal().and_then(|goal| goal.goal_plan.clone()),
                    }
                    div { class: "grid gap-3 xl:grid-cols-2",
                        ChangedFilesPanel { report: report.clone() }
                        OwnershipPanel { report: report.clone() }
                        OutOfScopePanel { report: report.clone() }
                        AffectedSpecPanel { report: report.clone() }
                        SuggestedGoalSplitPanel { split: report.suggested_goal_split.clone() }
                        TestRecommendationPanel { report: report.clone() }
                    }
                } else {
                    EmptyState { title: "Branch scope pending".to_string(), body: "Load branch.scope to inspect changed files, owners, affected specs, test impact, and strict review status.".to_string() }
                }
            }
        }
    }
}

#[component]
pub fn SpecImpactGraph(ui: WorkbenchUiState) -> Element {
    let report = ui
        .payload
        .state
        .branch_scope
        .as_ref()
        .and_then(|state| state.report.clone());
    let initial_node = report
        .as_ref()
        .and_then(|report| report.spec_impact_graph.nodes.first())
        .map(|node| node.id.clone())
        .unwrap_or_default();
    let graph_layout = report.as_ref().map(|report| {
        let node_positions = report
            .spec_impact_graph
            .nodes
            .iter()
            .enumerate()
            .map(|(index, node)| (node.id.clone(), index))
            .collect::<HashMap<_, _>>();
        let svg_height = graph_view_height(report.spec_impact_graph.nodes.len());
        let view_box = format!("0 0 900 {svg_height}");

        (node_positions, svg_height, view_box)
    });
    let mut selected_node_id = use_signal(|| initial_node);
    rsx! {
        Panel { class: classes::PANEL_MUTED,
            div { class: "flex flex-col gap-4 p-4",
                div { class: classes::SECTION_HEADER,
                    h2 { class: classes::SECTION_TITLE, "Spec Impact Graph" }
                    ScopeLegend {}
                }
                if let (Some(report), Some((node_positions, svg_height, view_box))) = (report, graph_layout) {
                    div { class: "grid gap-3 xl:grid-cols-[minmax(0,1fr)_16rem]",
                        div { class: "min-h-72 rounded-xl border border-border bg-background p-3",
                            svg { class: "w-full", height: "{svg_height}", view_box, role: "img",
                                for edge in &report.spec_impact_graph.edges {
                                    if let (Some(from_index), Some(to_index)) = (node_positions.get(&edge.from), node_positions.get(&edge.to)) {
                                        GraphEdge {
                                            from_index: *from_index,
                                            to_index: *to_index,
                                            state: edge.state.clone(),
                                            label: format!("{} to {}", edge.from, edge.to),
                                        }
                                    }
                                }
                                for (index, node) in report.spec_impact_graph.nodes.iter().enumerate() {
                                    GraphNode {
                                        id: node.id.clone(),
                                        index,
                                        label: node.label.clone(),
                                        kind: node.kind.clone(),
                                        state: node.state.clone(),
                                        selected: selected_node_id.read().as_str() == node.id.as_str(),
                                        onclick: {
                                            let node_id = node.id.clone();
                                            move |_| selected_node_id.set(node_id.clone())
                                        },
                                    }
                                }
                            }
                        }
                        div { class: "space-y-2",
                            for node in &report.spec_impact_graph.nodes {
                                button {
                                    class: if selected_node_id.read().as_str() == node.id.as_str() { "w-full rounded-lg border border-command-active bg-panel-muted p-2 text-left" } else { "w-full rounded-lg border border-border bg-panel p-2 text-left" },
                                    type: "button",
                                    onclick: {
                                        let node_id = node.id.clone();
                                        move |_| selected_node_id.set(node_id.clone())
                                    },
                                    div { class: "flex flex-wrap items-center gap-2",
                                        ScopeChip { label: node.kind.clone() }
                                        ScopeChip { label: node.state.clone() }
                                    }
                                    p { class: "mt-2 text-sm font-medium", "{node.label}" }
                                    p { class: "mt-1 text-xs text-foreground/60", "{node.id}" }
                                }
                            }
                        }
                    }
                } else {
                    div { class: "grid gap-3 md:grid-cols-[minmax(0,1fr)_14rem]",
                        EmptyState {
                            title: "Branch scope not loaded".to_string(),
                            body: "Open Load branch scope from the command palette to build this graph.".to_string(),
                        }
                        a {
                            class: "flex items-center justify-center rounded-lg border border-border bg-background px-3 py-2 text-sm font-medium text-foreground/80 hover:bg-panel-muted",
                            href: "?pane=commands&sidebar=0&query=branch%20scope&action=branch.scope",
                            "Load branch scope"
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub fn GraphNode(
    id: String,
    index: usize,
    label: String,
    kind: String,
    state: String,
    selected: bool,
    onclick: EventHandler<MouseEvent>,
) -> Element {
    let (x, y) = graph_node_origin(index);
    let class = graph_state_class(&state);
    let selected_class = if selected {
        "stroke-[3px]"
    } else {
        "stroke-[1.5px]"
    };
    let short = truncate_label(&label, 26);
    rsx! {
        g { tabindex: "0", role: "button", onclick: move |event| onclick.call(event),
            title { "{kind}: {label}" }
            desc { "{id}" }
            rect { x: "{x}", y: "{y}", width: "172", height: "44", rx: "7", class: "fill-panel stroke-current {class} {selected_class}" }
            text { x: "{x + 12}", y: "{y + 19}", class: "fill-foreground text-[11px] font-semibold", "{short}" }
            text { x: "{x + 12}", y: "{y + 34}", class: "fill-foreground/60 text-[9px] uppercase", "{kind} / {state}" }
        }
    }
}

#[component]
pub fn GraphEdge(from_index: usize, to_index: usize, state: String, label: String) -> Element {
    let (from_x, from_y) = graph_edge_anchor(from_index, to_index);
    let (to_x, to_y) = graph_edge_anchor(to_index, from_index);
    let class = graph_state_class(&state);
    rsx! {
        g {
            title { "{label}" }
            line { x1: "{from_x}", y1: "{from_y}", x2: "{to_x}", y2: "{to_y}", class: "stroke-current {class}", stroke_width: "2" }
            circle { cx: "{to_x}", cy: "{to_y}", r: "3", class: "fill-current {class}" }
        }
    }
}

fn graph_node_origin(index: usize) -> (i32, i32) {
    (
        GRAPH_NODE_X + ((index % GRAPH_COLUMNS) as i32 * GRAPH_COLUMN_WIDTH),
        GRAPH_NODE_Y + ((index / GRAPH_COLUMNS) as i32 * GRAPH_ROW_HEIGHT),
    )
}

fn graph_edge_anchor(index: usize, target_index: usize) -> (i32, i32) {
    let (x, y) = graph_node_origin(index);
    let column = index % GRAPH_COLUMNS;
    let target_column = target_index % GRAPH_COLUMNS;
    let row = index / GRAPH_COLUMNS;
    let target_row = target_index / GRAPH_COLUMNS;

    if target_row > row {
        (x + (GRAPH_NODE_WIDTH / 2), y + GRAPH_NODE_HEIGHT)
    } else if target_row < row {
        (x + (GRAPH_NODE_WIDTH / 2), y)
    } else if target_column >= column {
        (x + GRAPH_NODE_WIDTH, y + (GRAPH_NODE_HEIGHT / 2))
    } else {
        (x, y + (GRAPH_NODE_HEIGHT / 2))
    }
}

fn graph_view_height(node_count: usize) -> i32 {
    let rows = node_count.max(1).div_ceil(GRAPH_COLUMNS);
    320.max(70 + (rows as i32 * GRAPH_ROW_HEIGHT))
}

#[component]
pub fn ScopeLegend() -> Element {
    rsx! {
        div { class: "flex flex-wrap justify-end gap-2",
            for label in ["spec-linked", "code-linked", "test-linked", "scope-in", "scope-out", "scope-ambiguous", "ownership-known", "ownership-missing", "ownership-ambiguous", "evidence-pass", "evidence-warn", "evidence-fail", "evidence-pending"] {
                span { class: "inline-flex items-center gap-1 text-[10px] uppercase text-foreground/70",
                    span { class: "h-2 w-2 rounded-full {graph_state_class(label)} bg-current" }
                    span { "{label}" }
                }
            }
        }
    }
}

#[component]
pub fn GoalScopeComparisonPanel(
    report: syu_workbench::BranchScopeReport,
    plan: Option<GoalPlanArtifact>,
) -> Element {
    let Some(plan) = plan else {
        return rsx! {
            EmptyState { title: "No Goal Plan comparison".to_string(), body: "Branch Scope Lens compares changed files against a selected Goal Plan when one is active.".to_string() }
        };
    };
    let include_patterns = plan
        .implementation_plan
        .scope
        .include
        .iter()
        .map(include_pattern)
        .collect::<Vec<_>>();
    let exclude_patterns = plan.implementation_plan.scope.exclude.clone();
    let mut included = Vec::new();
    let mut excluded = Vec::new();
    let mut uncovered = Vec::new();

    for file in &report.changed_files {
        if exclude_patterns
            .iter()
            .any(|pattern| path_matches_goal_pattern(&file.file, pattern))
        {
            excluded.push(file.file.clone());
        } else if include_patterns
            .iter()
            .any(|pattern| path_matches_goal_pattern(&file.file, pattern))
        {
            included.push(file.file.clone());
        } else {
            uncovered.push(file.file.clone());
        }
    }

    rsx! {
        Panel { class: classes::PANEL_MUTED,
            div { class: "flex flex-col gap-3 p-3",
                div { class: classes::SECTION_HEADER,
                    h3 { class: "text-sm font-semibold", "Goal Scope Comparison" }
                    ScopeChip { label: plan.goal.id.clone() }
                }
                div { class: "grid gap-3 md:grid-cols-3",
                    GoalComparisonColumn { title: "files included by Goal".to_string(), tone: "scope-in".to_string(), files: included }
                    GoalComparisonColumn { title: "files excluded by Goal".to_string(), tone: "scope-out".to_string(), files: excluded }
                    GoalComparisonColumn { title: "changed files not covered by Goal".to_string(), tone: "scope-ambiguous".to_string(), files: uncovered }
                }
                div { class: "grid gap-3 md:grid-cols-2",
                    div { class: classes::EVIDENCE_CARD,
                        p { class: "text-xs uppercase tracking-[0.18em] text-foreground/60", "tests required by Goal" }
                        for command in &plan.completion.must_pass {
                            p { class: "mt-1 text-sm text-test-linked", "{command}" }
                        }
                    }
                    div { class: classes::EVIDENCE_CARD,
                        p { class: "text-xs uppercase tracking-[0.18em] text-foreground/60", "tests detected from code ownership" }
                        for test in report.test_inventory.required_tests.iter().chain(report.test_inventory.linked_tests.iter()) {
                            p { class: "mt-1 text-sm text-test-linked", "{test}" }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn GoalComparisonColumn(title: String, tone: String, files: Vec<String>) -> Element {
    rsx! {
        div { class: classes::EVIDENCE_CARD,
            p { class: "text-xs uppercase tracking-[0.18em] text-foreground/60", "{title}" }
            if files.is_empty() {
                p { class: "mt-1 text-sm text-foreground/60", "none" }
            } else {
                for file in files {
                    p { class: "mt-1 text-sm {graph_state_class(&tone)}", "{file}" }
                }
            }
        }
    }
}

#[component]
pub fn ImpactSummaryPanel(report: syu_workbench::BranchScopeReport) -> Element {
    let strict_status = if report.spec_impact.out_of_scope_changes.is_empty()
        && report.trace_ownership.unowned_changes.is_empty()
    {
        "strict review: pass"
    } else {
        "strict review: warn"
    };
    rsx! {
        div { class: "grid gap-3 md:grid-cols-4",
            PulseMetric { label: "changed files".to_string(), value: report.changed_files.len().to_string() }
            PulseMetric { label: "affected specs".to_string(), value: report.spec_impact.affected_items.len().to_string() }
            PulseMetric { label: "tests".to_string(), value: report.test_inventory.total_tests.to_string() }
            PulseMetric { label: "strict status".to_string(), value: strict_status.to_string() }
        }
    }
}

#[component]
pub fn ChangedFilesPanel(report: syu_workbench::BranchScopeReport) -> Element {
    rsx! {
        Panel { class: classes::PANEL_MUTED,
            div { class: "flex flex-col gap-2 p-3",
                div { class: classes::SECTION_HEADER,
                    h3 { class: "text-sm font-semibold", "Changed Files" }
                    ScopeChip { label: format!("{} files", report.changed_files.len()) }
                }
                for file in &report.changed_files {
                    details { class: classes::EVIDENCE_CARD,
                        summary { class: "list-none cursor-pointer rounded-xl outline-none",
                            div { class: "flex flex-wrap items-center gap-2",
                                OwnershipBadge { status: format!("{:?}", file.status) }
                                ScopeChip { label: if file.is_spec_file { "spec-linked".to_string() } else { "code-linked".to_string() } }
                                ScopeChip { label: format!("{} symbols", file.symbols.len()) }
                            }
                            p { class: "mt-2 text-sm font-medium", "{file.file}" }
                        }
                        div { class: "mt-3 space-y-2",
                            for symbol in &file.symbols {
                                p { class: "text-xs text-foreground/65", "symbol: {symbol}" }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub fn OwnershipPanel(report: syu_workbench::BranchScopeReport) -> Element {
    rsx! {
        Panel { class: classes::PANEL_MUTED,
            div { class: "flex flex-col gap-2 p-3",
                div { class: classes::SECTION_HEADER,
                    h3 { class: "text-sm font-semibold", "Ownership" }
                    ScopeChip { label: format!("{} owned", report.trace_ownership.owned_files) }
                }
                if report.trace_ownership.unowned_changes.is_empty() && report.trace_ownership.ambiguous_ownership.is_empty() {
                    p { class: "text-sm text-ownership-known", "ownership-known" }
                } else {
                    for change in &report.trace_ownership.unowned_changes {
                        details { class: "rounded-xl border border-evidence-fail/40 bg-background/30 p-3",
                            summary { class: "list-none cursor-pointer rounded-lg outline-none",
                                p { class: "text-sm text-ownership-missing", "unowned: {change.file}" }
                            }
                            p { class: "mt-2 text-xs text-foreground/65", "{change.reason}" }
                        }
                    }
                    for change in &report.trace_ownership.ambiguous_ownership {
                        details { class: "rounded-xl border border-evidence-warn/40 bg-background/30 p-3",
                            summary { class: "list-none cursor-pointer rounded-lg outline-none",
                                p { class: "text-sm text-ownership-ambiguous", "ambiguous: {change.file}" }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub fn OwnershipBadge(status: String) -> Element {
    let state = match status.as_str() {
        "Owned" => "ownership-known",
        "Partial" => "ownership-ambiguous",
        _ => "ownership-missing",
    };
    rsx! {
        span { class: "{classes::CHIP} {graph_state_class(state)}", "{state}" }
    }
}

#[component]
pub fn OutOfScopePanel(report: syu_workbench::BranchScopeReport) -> Element {
    rsx! {
        Panel { class: classes::PANEL_MUTED,
            div { class: "flex flex-col gap-2 p-3",
                div { class: classes::SECTION_HEADER,
                    h3 { class: "text-sm font-semibold", "Out Of Scope" }
                    ScopeChip { label: format!("{} files", report.spec_impact.out_of_scope_changes.len()) }
                }
                if report.spec_impact.out_of_scope_changes.is_empty() {
                    p { class: "text-sm text-scope-in", "scope-in" }
                } else {
                    for change in &report.spec_impact.out_of_scope_changes {
                        details { class: "rounded-xl border border-evidence-fail/40 bg-background/30 p-3",
                            summary { class: "list-none cursor-pointer rounded-lg outline-none",
                                p { class: "text-sm text-scope-out", "{change.file}" }
                            }
                            p { class: "mt-2 text-xs text-foreground/65", "{change.reason}" }
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub fn AffectedSpecPanel(report: syu_workbench::BranchScopeReport) -> Element {
    let initial_spec = report
        .spec_impact
        .affected_items
        .first()
        .map(|item| item.id.clone())
        .unwrap_or_default();
    let mut selected_spec_id = use_signal(|| initial_spec);
    rsx! {
        Panel { class: classes::PANEL_MUTED,
            div { class: "flex flex-col gap-2 p-3",
                div { class: classes::SECTION_HEADER,
                    h3 { class: "text-sm font-semibold", "Affected Specs" }
                    ScopeChip { label: format!("{} linked", report.spec_impact.affected_items.len()) }
                }
                for item in &report.spec_impact.affected_items {
                    button {
                        class: if selected_spec_id.read().as_str() == item.id.as_str() { "w-full rounded-xl border border-command-active bg-panel p-3 text-left" } else { classes::EVIDENCE_CARD },
                        type: "button",
                        onclick: {
                            let item_id = item.id.clone();
                            move |_| selected_spec_id.set(item_id.clone())
                        },
                        div { class: "flex flex-wrap items-center gap-2",
                            ScopeChip { label: item.kind.clone() }
                            ScopeChip { label: if item.direct { "spec-linked".to_string() } else { "scope-ambiguous".to_string() } }
                        }
                        p { class: "mt-2 text-sm font-medium", "{item.id}" }
                        p { class: "text-xs text-foreground/65", "{item.title}" }
                    }
                }
            }
        }
    }
}

#[component]
pub fn SuggestedGoalSplitPanel(split: syu_workbench::SuggestedGoalSplit) -> Element {
    rsx! {
        Panel { class: classes::PANEL_MUTED,
            div { class: "flex flex-col gap-2 p-3",
                div { class: classes::SECTION_HEADER,
                    h3 { class: "text-sm font-semibold", "Suggested Goal Split" }
                    ScopeChip { label: format!("confidence: {}", split.confidence) }
                }
                details { class: "rounded-xl border border-border bg-background/30 p-3",
                    summary { class: "list-none cursor-pointer rounded-lg outline-none",
                        p { class: "text-sm font-medium text-foreground", "split preview" }
                    }
                    div { class: "mt-3 space-y-2",
                        for include in &split.include {
                            p { class: "text-sm text-scope-in", "include: {include}" }
                        }
                        for exclude in &split.exclude {
                            p { class: "text-sm text-scope-out", "exclude: {exclude}" }
                        }
                        for reason in &split.reasons {
                            p { class: "text-xs text-evidence-warn", "{reason}" }
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub fn TestRecommendationPanel(report: syu_workbench::BranchScopeReport) -> Element {
    rsx! {
        Panel { class: classes::PANEL_MUTED,
            div { class: "flex flex-col gap-2 p-3",
                div { class: classes::SECTION_HEADER,
                    h3 { class: "text-sm font-semibold", "Test Impact" }
                    ScopeChip { label: format!("{} tests", report.test_inventory.total_tests) }
                }
                details { class: "rounded-xl border border-border bg-background/30 p-3",
                    summary { class: "list-none cursor-pointer rounded-lg outline-none",
                        p { class: "text-sm font-medium text-foreground", "test list" }
                    }
                    div { class: "mt-3 space-y-1",
                        for test in report.test_inventory.required_tests.iter().chain(report.test_inventory.linked_tests.iter()) {
                            p { class: "text-sm text-test-linked", "{test}" }
                        }
                        if report.test_inventory.total_tests == 0 {
                            p { class: "text-sm text-evidence-warn", "evidence-pending" }
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub fn RequestIntakeCanvas(
    ui: WorkbenchUiState,
    on_run_action: Option<EventHandler<WorkbenchActionId>>,
) -> Element {
    let request = ui.payload.state.request.clone();
    rsx! {
        Panel { class: classes::PANEL_MUTED,
            div { class: "flex flex-col gap-4 p-4",
                div { class: classes::SECTION_HEADER,
                    h2 { class: classes::SECTION_TITLE, "Request Intake" }
                    ScopeChip { label: temporary_artifact_label(&ui) }
                }
                RequestContextEditor { ui: ui.clone() }
                div { class: "grid gap-2 md:grid-cols-4",
                    FlowActionButton { label: "Classify".to_string(), action_id: WorkbenchActionId::RequestClassify, ui: ui.clone(), onclick: on_run_action }
                    FlowActionButton { label: "Scope".to_string(), action_id: WorkbenchActionId::RequestScope, ui: ui.clone(), onclick: on_run_action }
                    FlowActionButton { label: "Preview scaffold".to_string(), action_id: WorkbenchActionId::RequestScaffold, ui: ui.clone(), onclick: on_run_action }
                    FlowActionButton { label: "Generate plan".to_string(), action_id: WorkbenchActionId::RequestPlan, ui: ui.clone(), onclick: on_run_action }
                }
                div { class: "grid gap-3 xl:grid-cols-3",
                    RequestClassificationPanel { request: request.clone() }
                    RequestScopePanel { request: request.clone() }
                    ScaffoldPreviewPanel { request }
                }
            }
        }
    }
}

#[component]
pub fn RequestContextEditor(ui: WorkbenchUiState) -> Element {
    let request = ui
        .payload
        .state
        .request
        .as_ref()
        .and_then(|state| state.artifact.as_ref());
    rsx! {
        Panel { class: classes::PANEL_MUTED,
            div { class: "flex flex-col gap-3 p-3",
                div { class: classes::SECTION_HEADER,
                    h3 { class: "text-sm font-semibold", "Change request" }
                    EvidenceBadge { kind: syu_workbench::WorkbenchEvidenceKind::RequestArtifact }
                }
                if let Some(artifact) = request {
                    p { class: "text-base leading-7 text-foreground", "{artifact.request}" }
                    div { class: "flex flex-wrap gap-2",
                        if let Some(area) = &artifact.context.affected_area {
                            ScopeChip { label: format!("area: {area}") }
                        }
                        for id in &artifact.context.linked_ids {
                            ScopeChip { label: id.clone() }
                        }
                    }
                    for constraint in &artifact.context.repository_constraints {
                        p { class: "text-sm text-foreground/70", "constraint: {constraint}" }
                    }
                } else {
                    EmptyState { title: "Paste a request".to_string(), body: "Request text becomes a temporary Workbench artifact before any spec content changes.".to_string() }
                }
            }
        }
    }
}

#[component]
pub fn RequestClassificationPanel(request: Option<syu_workbench::ActiveRequestState>) -> Element {
    rsx! {
        Panel { class: classes::PANEL_MUTED,
            div { class: "flex flex-col gap-2 p-3",
                div { class: classes::SECTION_HEADER,
                    h3 { class: "text-sm font-semibold", "Classify" }
                    EvidenceBadge { kind: syu_workbench::WorkbenchEvidenceKind::ClassificationOutcome }
                }
                if let Some(classification) = request.as_ref().and_then(|request| request.classification.as_ref()) {
                    ScopeChip { label: classification.classification.label().to_string() }
                    for reason in &classification.reasons {
                        p { class: "text-sm text-foreground/75", "{reason}" }
                    }
                    for item in classification.explicit_items.iter().chain(classification.related_items.iter()) {
                        p { class: "text-xs uppercase tracking-[0.16em] text-foreground/60", "{item.kind}: {item.id}" }
                    }
                } else {
                    EmptyState { title: "Not classified".to_string(), body: "Run request.classify from this canvas or the command palette.".to_string() }
                }
            }
        }
    }
}

#[component]
pub fn RequestScopePanel(request: Option<syu_workbench::ActiveRequestState>) -> Element {
    rsx! {
        Panel { class: classes::PANEL_MUTED,
            div { class: "flex flex-col gap-2 p-3",
                div { class: classes::SECTION_HEADER,
                    h3 { class: "text-sm font-semibold", "Scope" }
                    EvidenceBadge { kind: syu_workbench::WorkbenchEvidenceKind::ScopeOutcome }
                }
                if let Some(scope) = request.as_ref().and_then(|request| request.scope.as_ref()) {
                    div { class: "flex flex-wrap gap-2",
                        for requirement in &scope.requirements {
                            ScopeChip { label: requirement.id.clone() }
                        }
                        for feature in &scope.features {
                            ScopeChip { label: feature.id.clone() }
                        }
                    }
                    for note in &scope.notes {
                        p { class: "text-sm text-foreground/75", "{note}" }
                    }
                } else {
                    EmptyState { title: "Scope pending".to_string(), body: "Map the request to relevant specs before planning implementation work.".to_string() }
                }
            }
        }
    }
}

#[component]
pub fn ScaffoldPreviewPanel(request: Option<syu_workbench::ActiveRequestState>) -> Element {
    rsx! {
        Panel { class: classes::PANEL_MUTED,
            div { class: "flex flex-col gap-2 p-3",
                div { class: classes::SECTION_HEADER,
                    h3 { class: "text-sm font-semibold", "Scaffold Preview" }
                    EvidenceBadge { kind: syu_workbench::WorkbenchEvidenceKind::ScaffoldPlan }
                }
                if let Some(scaffold) = request.as_ref().and_then(|request| request.scaffold.as_ref()) {
                    for update in &scaffold.updates {
                        article { class: classes::EVIDENCE_CARD,
                            div { class: "flex flex-wrap items-center gap-2",
                                ScopeChip { label: scaffold_action_label(update.action).to_string() }
                                ScopeChip { label: scaffold_kind_label(update.kind).to_string() }
                                if let Some(id) = &update.id {
                                    ScopeChip { label: id.clone() }
                                }
                            }
                            p { class: "mt-2 text-sm font-medium", "{update.path}" }
                            p { class: "mt-1 text-xs text-foreground/65", "{update.contents}" }
                        }
                    }
                } else {
                    EmptyState { title: "No scaffold preview".to_string(), body: "Preview spec updates without treating them as committed persistent content.".to_string() }
                }
            }
        }
    }
}

#[component]
pub fn GoalPlanCanvas(
    ui: WorkbenchUiState,
    on_run_action: Option<EventHandler<WorkbenchActionId>>,
) -> Element {
    let goals = ui.payload.state.goals.active.clone();
    rsx! {
        section { class: "flex flex-col gap-3",
            div { class: classes::SECTION_HEADER,
                h2 { class: classes::SECTION_TITLE, "Goal Plan" }
                ScopeChip { label: format!("{} temporary cards", goals.len()) }
            }
            if goals.is_empty() {
                EmptyState { title: "No generated Goal Plan".to_string(), body: "Run request.plan after the request is classified and scoped.".to_string() }
            } else {
                for goal in goals {
                    if let Some(plan) = goal.goal_plan.clone() {
                        GoalPlanCard { plan: plan.clone() }
                    }
                }
                div { class: "grid gap-2 md:grid-cols-3",
                    FlowActionButton { label: "Select tests".to_string(), action_id: WorkbenchActionId::GoalTestSelect, ui: ui.clone(), onclick: on_run_action }
                    FlowActionButton { label: "Check goal".to_string(), action_id: WorkbenchActionId::GoalCheck, ui: ui.clone(), onclick: on_run_action }
                    FlowActionButton { label: "Assign next".to_string(), action_id: WorkbenchActionId::AssignmentCreate, ui: ui.clone(), onclick: on_run_action }
                }
            }
        }
    }
}

#[component]
fn GoalPlanCard(plan: GoalPlanArtifact) -> Element {
    rsx! {
        article { class: "rounded-2xl border border-goal-active bg-panel p-3",
            GoalCard { goal_id: plan.goal.id.clone(), title: plan.goal.title.clone(), selected: true }
            div { class: "mt-3 grid gap-3 xl:grid-cols-2",
                Panel { class: classes::PANEL_MUTED,
                    div { class: "flex flex-col gap-2 p-3",
                        div { class: classes::SECTION_HEADER,
                            h3 { class: "text-sm font-semibold", "Goal Statement" }
                            ScopeChip { label: confidence_label(plan.source.confidence.or(plan.implementation_plan.confidence)) }
                        }
                        p { class: "text-sm leading-6 text-foreground/80", "{plan.goal.statement}" }
                        for non_goal in &plan.goal.non_goals {
                            p { class: "text-sm text-scope-out", "non-goal: {non_goal}" }
                        }
                    }
                }
                GoalDependencyView { plan: plan.clone() }
                GoalScopePanel { plan: plan.clone() }
                GoalTestPlanPanel { plan: plan.clone() }
                GoalPlanExportPanel { plan: plan.clone() }
            }
        }
    }
}

#[component]
pub fn GoalDependencyView(plan: GoalPlanArtifact) -> Element {
    let items = persistent_item_labels(&plan);
    rsx! {
        Panel { class: classes::PANEL_MUTED,
            div { class: "flex flex-col gap-2 p-3",
                div { class: classes::SECTION_HEADER,
                    h3 { class: "text-sm font-semibold", "Persistent Spec Items" }
                    ScopeChip { label: format!("{} linked", items.len()) }
                }
                if items.is_empty() {
                    EmptyState { title: "No linked specs".to_string(), body: "Inferred suggestions must be reviewed before implementation.".to_string() }
                } else {
                    div { class: "flex flex-wrap gap-2",
                        for item in items {
                            ScopeChip { label: item }
                        }
                    }
                }
                if plan.spec_mapping.spec_updates_required {
                    p { class: "text-sm text-evidence-warn", "spec updates required" }
                    for update in &plan.spec_mapping.spec_updates.expected_updates {
                        p { class: "text-sm text-foreground/75", "{update}" }
                    }
                }
            }
        }
    }
}

#[component]
pub fn GoalScopePanel(plan: GoalPlanArtifact) -> Element {
    rsx! {
        Panel { class: classes::PANEL_MUTED,
            div { class: "flex flex-col gap-2 p-3",
                div { class: classes::SECTION_HEADER,
                    h3 { class: "text-sm font-semibold", "Scope" }
                    ScopeChip { label: "include / exclude".to_string() }
                }
                div { class: "flex flex-wrap gap-2",
                    for include in &plan.implementation_plan.scope.include {
                        ScopeChip { label: format!("include: {}", include_pattern(include)) }
                    }
                    for exclude in &plan.implementation_plan.scope.exclude {
                        ScopeChip { label: format!("exclude: {exclude}") }
                    }
                }
                for step in &plan.implementation_plan.steps {
                    p { class: "text-sm text-foreground/75", "step: {step}" }
                }
            }
        }
    }
}

#[component]
pub fn GoalTestPlanPanel(plan: GoalPlanArtifact) -> Element {
    rsx! {
        Panel { class: classes::PANEL_MUTED,
            div { class: "flex flex-col gap-2 p-3",
                div { class: classes::SECTION_HEADER,
                    h3 { class: "text-sm font-semibold", "Tests and Evidence" }
                    EvidenceBadge { kind: syu_workbench::WorkbenchEvidenceKind::GoalPlanCheckReport }
                }
                ScopeChip { label: format!("selection: {:?}", plan.test_plan.selection_mode) }
                if plan.test_plan.required_tests.is_empty() {
                    p { class: "text-sm text-foreground/70", "required tests: covered by completion commands" }
                } else {
                    for language in plan.test_plan.required_tests.keys() {
                        p { class: "text-sm text-foreground/75", "required tests: {language}" }
                    }
                }
                for command in &plan.completion.must_pass {
                    p { class: "text-sm text-evidence-pass", "must pass: {command}" }
                }
                for warning in &plan.warnings {
                    p { class: "text-sm text-evidence-warn", "evidence note: {warning}" }
                }
            }
        }
    }
}

#[component]
pub fn GoalPlanExportPanel(plan: GoalPlanArtifact) -> Element {
    rsx! {
        Panel { class: classes::PANEL_MUTED,
            div { class: "flex flex-col gap-2 p-3",
                div { class: classes::SECTION_HEADER,
                    h3 { class: "text-sm font-semibold", "Export YAML" }
                    EvidenceBadge { kind: syu_workbench::WorkbenchEvidenceKind::GoalPlanArtifact }
                }
                p { class: "text-sm text-foreground/75", "Export target: .syu/workbench/goals/{plan.goal.id}.yaml" }
                pre { class: "max-h-56 overflow-auto rounded-xl border border-border bg-background p-3 text-xs text-foreground/80",
                    "{goal_plan_yaml_preview(&plan)}"
                }
            }
        }
    }
}

#[component]
pub fn EvidenceTimeline(entries: Vec<EvidenceRecord>, goal_id: Option<String>) -> Element {
    let filtered_entries = scoped_evidence_entries(entries, goal_id.as_deref());

    rsx! {
        div { class: "space-y-3",
            if filtered_entries.is_empty() {
                EmptyState {
                    title: "Evidence timeline".to_string(),
                    body: "Append evidence by running goal checks, test selection, or validation.".to_string()
                }
            } else {
                for record in filtered_entries {
                    { render_evidence_timeline_record(record) }
                }
            }
        }
    }
}

fn render_evidence_timeline_record(record: EvidenceRecord) -> Element {
    match record.kind {
        syu_workbench::WorkbenchEvidenceKind::ValidationReport => {
            rsx! { ValidationEvidenceView { record } }
        }
        syu_workbench::WorkbenchEvidenceKind::TaskTestSelectionPlan => {
            rsx! { TestEvidenceView { record } }
        }
        syu_workbench::WorkbenchEvidenceKind::BranchScopeReport
        | syu_workbench::WorkbenchEvidenceKind::SpecImpactReport => {
            rsx! { ScopeEvidenceView { record } }
        }
        syu_workbench::WorkbenchEvidenceKind::AgentRun
        | syu_workbench::WorkbenchEvidenceKind::JobState => {
            rsx! { AgentEvidenceView { record } }
        }
        syu_workbench::WorkbenchEvidenceKind::AssignmentState => {
            rsx! { ManualDecisionEvidenceView { record } }
        }
        _ => rsx! { EvidenceRecordCard { record } },
    }
}

#[component]
pub fn EvidencePanel(ui: WorkbenchUiState) -> Element {
    let active_goal = ui.payload.state.goals.active_goal().cloned();
    let goal_id = active_goal.as_ref().map(|goal| goal.goal_id.clone());
    let latest = latest_scoped_evidence(
        &ui.payload.state.evidence_timeline.entries,
        goal_id.as_deref(),
    );
    rsx! {
        Panel { class: classes::PANEL,
            div { class: classes::PANEL_INNER,
                div { class: classes::SECTION_HEADER,
                    h2 { class: classes::SECTION_TITLE, "Evidence Timeline" }
                    if let Some(goal) = &active_goal {
                        ScopeChip { label: format!("goal {}", goal.goal_id) }
                    } else {
                        ScopeChip { label: "workspace".to_string() }
                    }
                }
                if let Some(record) = latest {
                    EvidenceDetailDrawer { record }
                }
                div { class: classes::SECTION_BODY,
                    EvidenceTimeline {
                        entries: ui.payload.state.evidence_timeline.entries.clone(),
                        goal_id: goal_id.clone(),
                    }
                }
            }
        }
    }
}

fn scoped_evidence_entries(
    entries: Vec<EvidenceRecord>,
    goal_id: Option<&str>,
) -> Vec<EvidenceRecord> {
    match goal_id {
        Some(goal_id) => entries
            .into_iter()
            .filter(|entry| entry.goal_id.as_deref() == Some(goal_id))
            .collect(),
        None => entries,
    }
}

fn latest_scoped_evidence(
    entries: &[EvidenceRecord],
    goal_id: Option<&str>,
) -> Option<EvidenceRecord> {
    match goal_id {
        Some(goal_id) => entries
            .iter()
            .rev()
            .find(|entry| entry.goal_id.as_deref() == Some(goal_id))
            .cloned(),
        None => entries.last().cloned(),
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
mod tests {
    use super::*;
    use crate::model::build_demo_state;
    use dioxus_ssr::render_element;
    use syu_workbench::{
        AgentRunMode, Assignee, Assignment, AssignmentScope, AssignmentStatus, ScopeGuardResult,
        ScopeGuardStatus, WorkbenchActionId, WorkbenchState,
    };

    #[test]
    fn app_shell_renders_command_palette_first_shell() {
        let html = render_element(rsx! {
            AppShell { ui: build_demo_state(), active_pane: WorkbenchPane::Commands, sidebar_open: true }
        });

        assert!(html.contains("Syu"));
        assert!(html.contains("Type a command"));
        assert!(html.contains("Command palette"));
        assert!(html.contains("data-command-palette"));
        assert!(!html.contains("navigation"));
    }

    #[test]
    fn command_palette_renders_disabled_reason_for_unavailable_actions() {
        let mut ui = WorkbenchUiState::from_state(WorkbenchState::default());
        ui.set_query("goal");

        let html = render_element(rsx! {
            AppShell { ui, active_pane: WorkbenchPane::Commands, sidebar_open: true }
        });

        assert!(html.contains("goal.check"));
    }

    #[test]
    fn human_assignments_hide_the_dry_run_action() {
        let human_assignment = Assignment {
            assignee: Some(Assignee::human("Manual Reviewer")),
            run_mode: AgentRunMode::Manual,
            status: AssignmentStatus::AssignmentReady,
            scope_guard: ScopeGuardResult {
                status: ScopeGuardStatus::ScopeValid,
                blockers: Vec::new(),
                out_of_scope_files: Vec::new(),
            },
            scope: AssignmentScope::default(),
            ..Assignment::default()
        };
        let automated_assignment = Assignment {
            assignee: Some(Assignee::local_command("local-coder", "Local coder")),
            ..human_assignment.clone()
        };

        assert!(!assignment_has_automated_assignee(&human_assignment));
        assert!(assignment_has_automated_assignee(&automated_assignment));
    }

    #[test]
    fn goal_canvas_renders_a_read_only_action_preview_placeholder() {
        let mut ui = build_demo_state();
        ui.run_read_only_action(WorkbenchActionId::HistoryShow);

        let html = render_element(rsx! {
            GoalCanvas {
                ui,
                on_run_action: None
            }
        });

        let pulse = html.find("workspace").expect("workspace should render");
        let preview = html
            .find("Preview opened for")
            .expect("preview should render");

        assert!(pulse < preview);
        assert!(html.contains("Preview opened for"));
        assert!(html.contains("Ready to review"));
    }

    #[test]
    fn goal_plan_yaml_preview_roundtrips_through_the_task_model_schema() {
        let ui = build_demo_state();
        let plan = ui
            .payload
            .state
            .goals
            .active_goal()
            .and_then(|goal| goal.goal_plan.as_ref())
            .expect("demo state should include an active goal plan");

        let yaml = goal_plan_yaml_preview(plan);
        let parsed: GoalPlanArtifact =
            serde_yaml::from_str(&yaml).expect("preview should deserialize as a GoalPlanArtifact");

        assert_eq!(&parsed, plan);
        assert!(yaml.contains("test_plan:"));
        assert!(yaml.contains("coverage:"));
    }

    #[test]
    fn evidence_panel_renders_placeholder_when_empty() {
        let ui = WorkbenchUiState::from_state(WorkbenchState::default());

        let html = render_element(rsx! {
            EvidencePanel { ui }
        });

        assert!(html.contains("Append evidence by running goal checks"));
    }
}
