use crate::components::{
    Button, CommandItem, DetailDrawer, EmptyState, EvidenceBadge, EvidenceLogRow, GoalCard,
    IconButton, Panel, ScopeChip, StatusDot,
};
use crate::design::classes;
use crate::model::{WorkbenchUiState, WorkspacePulseSummary};
use dioxus::prelude::*;
use syu_task_model::{
    GoalPlanArtifact, GoalPlanConfidence, GoalPlanPersistentItem, GoalPlanScopeInclude,
    ScaffoldAction, ScaffoldUpdateKind,
};
use syu_workbench::WorkbenchActionId;

#[component]
pub fn AppShell(ui: WorkbenchUiState) -> Element {
    let mut ui_state = use_signal(|| ui);
    let ui = ui_state.read().clone();
    rsx! {
        div { class: classes::APP_SHELL,
            div { class: classes::PAGE_FRAME,
                StatusBar { ui: ui.clone() }
                if ui.command_palette_open {
                    CommandPalette {
                        ui: ui.clone(),
                        on_query_change: move |query: String| ui_state.write().set_query(query),
                        on_select_action: move |action_id: WorkbenchActionId| {
                            ui_state.write().select_action(action_id);
                        },
                    }
                }
                div { class: classes::MAIN_GRID,
                    GoalRail { ui: ui.clone() }
                    GoalCanvas {
                        ui: ui.clone(),
                        on_run_action: move |action_id: WorkbenchActionId| {
                            ui_state.write().select_action(action_id);
                        },
                    }
                    EvidencePanel { ui: ui.clone() }
                }
            }
        }
    }
}

#[component]
pub fn StatusBar(ui: WorkbenchUiState) -> Element {
    let summary = ui.pulse_summary();
    rsx! {
        header { class: classes::CHROME_BAR,
            div { class: "flex items-center gap-3",
                Button { label: "Cmd+K".to_string(), active: ui.command_palette_open, disabled: false }
                div { class: classes::CHROME_META,
                    ScopeChip { label: summary.workspace.clone() }
                    ScopeChip { label: summary.branch.clone() }
                    ScopeChip { label: summary.health.clone() }
                }
            }
            div { class: "flex items-center gap-2",
                IconButton { label: "Workspace".to_string(), icon: "⌁".to_string() }
                StatusDot { tone_class: "bg-evidence-pass", label: format!("{} actions", summary.available_actions) }
            }
        }
    }
}

#[component]
pub fn WorkspacePulse(summary: WorkspacePulseSummary) -> Element {
    rsx! {
        Panel { class: classes::PANEL_MUTED,
            div { class: "flex flex-col gap-3 p-4",
                div { class: classes::SECTION_HEADER,
                    h2 { class: classes::SECTION_TITLE, "Workbench Pulse" }
                    ScopeChip { label: summary.health.clone() }
                }
                div { class: "grid gap-3 md:grid-cols-2",
                    div { class: "space-y-1",
                        p { class: "text-xs uppercase tracking-[0.18em] text-foreground/60", "workspace" }
                        p { class: "text-sm", "{summary.workspace}" }
                    }
                    div { class: "space-y-1",
                        p { class: "text-xs uppercase tracking-[0.18em] text-foreground/60", "branch" }
                        p { class: "text-sm", "{summary.branch}" }
                    }
                }
                div { class: "grid gap-3 md:grid-cols-3",
                    PulseMetric { label: "available actions".to_string(), value: summary.available_actions.to_string() }
                    PulseMetric { label: "recent evidence".to_string(), value: summary.recent_evidence.clone() }
                    PulseMetric { label: "next suggested".to_string(), value: summary.next_action.clone() }
                }
            }
        }
    }
}

#[component]
pub fn CommandPalette(
    ui: WorkbenchUiState,
    on_query_change: EventHandler<String>,
    on_select_action: EventHandler<WorkbenchActionId>,
) -> Element {
    let entries = ui.visible_actions();
    rsx! {
        Panel { class: classes::PANEL_MUTED,
            div { class: "flex flex-col gap-3 p-4",
                div { class: classes::SECTION_HEADER,
                    h2 { class: classes::SECTION_TITLE, "Command Palette" }
                    ScopeChip { label: if ui.command_palette_open { "open".to_string() } else { "closed".to_string() } }
                }
                input {
                    class: "w-full rounded-xl border border-border bg-background px-3 py-2 text-sm outline-none",
                    value: "{ui.command_query}",
                    placeholder: "Filter actions",
                    oninput: move |event| on_query_change.call(event.value())
                }
                div { class: "space-y-2",
                    for entry in entries.iter().cloned() {
                        CommandItem {
                            entry: entry.clone(),
                            selected: {
                                let action_id = entry.action.id;
                                ui.selected_action_id == Some(action_id)
                            },
                            onclick: move |_| on_select_action.call(entry.action.id),
                        }
                    }
                }
                if entries.is_empty() {
                    EmptyState { title: "No matching actions".to_string(), body: "Try a different command palette filter.".to_string() }
                }
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
                    h2 { class: classes::SECTION_TITLE, "Goal Rail" }
                    ScopeChip { label: format!("{} goals", ui.payload.state.goals.active.len()) }
                }
                div { class: classes::SECTION_BODY,
                    if ui.payload.state.goals.active.is_empty() {
                        EmptyState { title: "No active goals".to_string(), body: "The Workbench will anchor the first goal here.".to_string() }
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
                    EmptyState { title: "No action selected".to_string(), body: "Open the palette to run a read-only action or inspect the next suggested step.".to_string() }
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
    rsx! {
        Panel { class: classes::PANEL_MUTED,
            div { class: "flex flex-col gap-4 p-4",
                div { class: classes::SECTION_HEADER,
                    h2 { class: classes::SECTION_TITLE, "Spec Impact Graph" }
                    ScopeLegend {}
                }
                if let Some(report) = report {
                    div { class: "grid gap-3 xl:grid-cols-[minmax(0,1fr)_16rem]",
                        div { class: "min-h-72 rounded-xl border border-border bg-background p-3",
                            svg { class: "h-72 w-full", view_box: "0 0 900 320", role: "img",
                                for (index, edge) in report.spec_impact_graph.edges.iter().enumerate() {
                                    GraphEdge { index, state: edge.state.clone(), label: format!("{} to {}", edge.from, edge.to) }
                                }
                                for (index, node) in report.spec_impact_graph.nodes.iter().enumerate() {
                                    GraphNode {
                                        index,
                                        label: node.label.clone(),
                                        kind: node.kind.clone(),
                                        state: node.state.clone(),
                                    }
                                }
                            }
                        }
                        div { class: "space-y-2",
                            for node in &report.spec_impact_graph.nodes {
                                article { class: "rounded-lg border border-border bg-panel p-2",
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
                    EmptyState { title: "No impact graph".to_string(), body: "A Branch Scope report supplies typed nodes and edges for specs, code, and tests.".to_string() }
                }
            }
        }
    }
}

#[component]
pub fn GraphNode(index: usize, label: String, kind: String, state: String) -> Element {
    let x = 95 + ((index % 4) as i32 * 210);
    let y = 56 + ((index / 4) as i32 * 86);
    let class = graph_state_class(&state);
    let short = truncate_label(&label, 26);
    rsx! {
        g { tabindex: "0",
            title { "{kind}: {label}" }
            rect { x: "{x}", y: "{y}", width: "172", height: "44", rx: "7", class: "fill-panel stroke-current {class}" }
            text { x: "{x + 12}", y: "{y + 19}", class: "fill-foreground text-[11px] font-semibold", "{short}" }
            text { x: "{x + 12}", y: "{y + 34}", class: "fill-foreground/60 text-[9px] uppercase", "{kind} / {state}" }
        }
    }
}

#[component]
pub fn GraphEdge(index: usize, state: String, label: String) -> Element {
    let from_x = 266 + ((index % 4) as i32 * 210);
    let from_y = 78 + ((index / 4) as i32 * 86);
    let to_x = 305 + ((index % 4) as i32 * 210);
    let to_y = if index % 3 == 2 { from_y + 86 } else { from_y };
    let class = graph_state_class(&state);
    rsx! {
        g {
            title { "{label}" }
            line { x1: "{from_x}", y1: "{from_y}", x2: "{to_x}", y2: "{to_y}", class: "stroke-current {class}", stroke_width: "2" }
            circle { cx: "{to_x}", cy: "{to_y}", r: "3", class: "fill-current {class}" }
        }
    }
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
                    article { class: classes::EVIDENCE_CARD,
                        div { class: "flex flex-wrap items-center gap-2",
                            OwnershipBadge { status: format!("{:?}", file.status) }
                            ScopeChip { label: if file.is_spec_file { "spec-linked".to_string() } else { "code-linked".to_string() } }
                        }
                        p { class: "mt-2 text-sm font-medium", "{file.file}" }
                        for symbol in &file.symbols {
                            p { class: "text-xs text-foreground/65", "symbol: {symbol}" }
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
                for change in &report.trace_ownership.unowned_changes {
                    p { class: "text-sm text-ownership-missing", "unowned: {change.file}" }
                    p { class: "text-xs text-foreground/65", "{change.reason}" }
                }
                for change in &report.trace_ownership.ambiguous_ownership {
                    p { class: "text-sm text-ownership-ambiguous", "ambiguous: {change.file}" }
                }
                if report.trace_ownership.unowned_changes.is_empty() && report.trace_ownership.ambiguous_ownership.is_empty() {
                    p { class: "text-sm text-ownership-known", "ownership-known" }
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
                        p { class: "text-sm text-scope-out", "{change.file}" }
                        p { class: "text-xs text-foreground/65", "{change.reason}" }
                    }
                }
            }
        }
    }
}

#[component]
pub fn AffectedSpecPanel(report: syu_workbench::BranchScopeReport) -> Element {
    rsx! {
        Panel { class: classes::PANEL_MUTED,
            div { class: "flex flex-col gap-2 p-3",
                div { class: classes::SECTION_HEADER,
                    h3 { class: "text-sm font-semibold", "Affected Specs" }
                    ScopeChip { label: format!("{} linked", report.spec_impact.affected_items.len()) }
                }
                for item in &report.spec_impact.affected_items {
                    article { class: classes::EVIDENCE_CARD,
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

#[component]
pub fn TestRecommendationPanel(report: syu_workbench::BranchScopeReport) -> Element {
    rsx! {
        Panel { class: classes::PANEL_MUTED,
            div { class: "flex flex-col gap-2 p-3",
                div { class: classes::SECTION_HEADER,
                    h3 { class: "text-sm font-semibold", "Test Impact" }
                    ScopeChip { label: format!("{} tests", report.test_inventory.total_tests) }
                }
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
pub fn EvidencePanel(ui: WorkbenchUiState) -> Element {
    rsx! {
        Panel { class: classes::PANEL,
            div { class: classes::PANEL_INNER,
                div { class: classes::SECTION_HEADER,
                    h2 { class: classes::SECTION_TITLE, "Evidence" }
                    ScopeChip { label: format!("{} entries", ui.payload.state.evidence_timeline.entries.len()) }
                }
                div { class: classes::SECTION_BODY,
                    if ui.payload.state.evidence_timeline.entries.is_empty() {
                        EmptyState { title: "Evidence placeholder".to_string(), body: "The first implementation keeps proof visible here.".to_string() }
                    } else {
                        for entry in &ui.payload.state.evidence_timeline.entries {
                            EvidenceLogRow { entry: entry.clone() }
                        }
                    }
                }
            }
        }
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
    rsx! {
        button {
            class: class,
            disabled: !available,
            onclick: move |_| {
                if let Some(handler) = onclick {
                    handler.call(action_id);
                }
            },
            type: "button",
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::build_demo_state;
    use dioxus_ssr::render_element;
    use syu_workbench::{WorkbenchActionId, WorkbenchState};

    #[test]
    fn app_shell_renders_workbench_pulse_before_the_side_panels() {
        let html = render_element(rsx! {
            AppShell { ui: build_demo_state() }
        });

        assert!(html.contains("Workbench Pulse"));
        assert!(html.contains("Goal Rail"));
        assert!(html.contains("Evidence"));
        assert!(html.contains("Command Palette"));
    }

    #[test]
    fn command_palette_renders_disabled_reason_for_unavailable_actions() {
        let mut ui = WorkbenchUiState::from_state(WorkbenchState::default());
        ui.set_query("goal");

        let html = render_element(rsx! {
            AppShell { ui }
        });

        assert!(html.contains("disabled: missing active_goal_plan"));
        assert!(html.contains("goal.check"));
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

        let pulse = html.find("Workbench Pulse").expect("pulse should render");
        let preview = html
            .find("Read-only action placeholder")
            .expect("preview should render");

        assert!(pulse < preview);
        assert!(html.contains("Read-only action placeholder"));
        assert!(html.contains("Evidence placeholder"));
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

        assert!(html.contains("Evidence placeholder"));
    }
}
