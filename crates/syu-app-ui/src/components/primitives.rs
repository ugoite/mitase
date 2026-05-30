use crate::design::classes;
use crate::model::CommandPaletteEntry;
use dioxus::prelude::*;
use syu_workbench::{EvidenceEntry, WorkbenchEvidenceKind};

#[component]
pub fn Panel(class: &'static str, children: Element) -> Element {
    rsx! {
        div { class: "{classes::PANEL} {class}", {children} }
    }
}

#[component]
pub fn Button(label: String, active: bool, disabled: bool) -> Element {
    let class = if active {
        "rounded-full border border-command-active bg-command-active px-3 py-1.5 text-sm font-medium text-background"
    } else {
        "rounded-full border border-border bg-panel-muted px-3 py-1.5 text-sm font-medium text-foreground hover:bg-panel"
    };
    rsx! {
        button {
            class: "{class}",
            disabled: disabled,
            type: "button",
            {label}
        }
    }
}

#[component]
pub fn IconButton(label: String, icon: String) -> Element {
    rsx! {
        button {
            class: "inline-flex items-center gap-2 rounded-full border border-border bg-panel-muted px-3 py-1.5 text-sm hover:bg-panel",
            type: "button",
            span { class: "text-foreground/60", "{icon}" }
            span { "{label}" }
        }
    }
}

#[component]
pub fn StatusDot(tone_class: &'static str, label: String) -> Element {
    rsx! {
        span { class: "inline-flex items-center gap-2 text-xs uppercase tracking-[0.18em] text-foreground/70",
            span { class: "{classes::STATUS_DOT} {tone_class}" }
            span { "{label}" }
        }
    }
}

#[component]
pub fn ScopeChip(label: String) -> Element {
    rsx! { span { class: classes::CHIP, "{label}" } }
}

#[component]
pub fn EvidenceBadge(kind: WorkbenchEvidenceKind) -> Element {
    let tone = evidence_tone(kind);
    rsx! {
        span { class: "{classes::CHIP} {tone}",
            "{kind.label()}"
        }
    }
}

#[component]
pub fn GoalCard(goal_id: String, title: String, selected: bool) -> Element {
    let class = if selected {
        "rounded-xl border border-goal-active bg-panel-muted p-3"
    } else {
        "rounded-xl border border-border bg-panel-muted p-3"
    };
    rsx! {
        article { class: class,
            p { class: "text-xs uppercase tracking-[0.18em] text-foreground/60", "{goal_id}" }
            p { class: "mt-1 text-sm font-medium", "{title}" }
        }
    }
}

#[component]
pub fn CommandItem(
    entry: CommandPaletteEntry,
    selected: bool,
    onclick: EventHandler<MouseEvent>,
) -> Element {
    let class = if selected {
        format!("{} {}", classes::COMMAND_ITEM, classes::COMMAND_ITEM_ACTIVE)
    } else {
        classes::COMMAND_ITEM.to_string()
    };
    let disabled_class = if entry.availability.available {
        ""
    } else {
        classes::COMMAND_ITEM_DISABLED
    };
    rsx! {
        button {
            class: "{class} {disabled_class}",
            onclick: onclick,
            type: "button",
            disabled: !entry.availability.available,
            div { class: "flex flex-col gap-1 text-left",
                span { class: "text-sm font-medium", "{entry.action.title}" }
                span { class: "text-xs text-foreground/65", "{entry.action.description}" }
            }
            div { class: "flex flex-col items-end gap-1 text-xs uppercase tracking-[0.18em]",
                span { "{entry.action.id.label()}" }
                if let Some(reason) = &entry.disabled_reason {
                    span { class: "normal-case tracking-normal text-evidence-warn", "{reason}" }
                } else {
                    span { class: "normal-case tracking-normal text-evidence-pass", "ready" }
                }
            }
        }
    }
}

#[component]
pub fn EmptyState(title: String, body: String) -> Element {
    rsx! {
        div { class: classes::EMPTY_STATE,
            p { class: "text-sm font-semibold", "{title}" }
            p { class: "mt-1 text-sm", "{body}" }
        }
    }
}

#[component]
pub fn DetailDrawer(title: String, body: String, evidence: String) -> Element {
    rsx! {
        section { class: classes::DRAWER,
            div { class: "flex items-center justify-between gap-3",
                h3 { class: "text-sm font-semibold", "{title}" }
                EvidenceBadge { kind: WorkbenchEvidenceKind::HistoryResponse }
            }
            p { class: "mt-2 text-sm text-foreground/80", "{body}" }
            p { class: "mt-2 text-xs uppercase tracking-[0.18em] text-evidence-pending", "{evidence}" }
        }
    }
}

#[component]
pub fn EvidenceLogRow(entry: EvidenceEntry) -> Element {
    rsx! {
        article { class: classes::EVIDENCE_CARD,
            div { class: "flex items-center justify-between gap-3",
                p { class: "text-sm font-medium", "{entry.summary}" }
                EvidenceBadge { kind: entry.kind }
            }
            if let Some(action_id) = entry.action_id {
                p { class: "mt-2 text-xs text-foreground/60", "{action_id.label()}" }
            }
        }
    }
}

fn evidence_tone(kind: WorkbenchEvidenceKind) -> &'static str {
    match kind {
        WorkbenchEvidenceKind::ValidationReport => "text-evidence-pass",
        WorkbenchEvidenceKind::BranchScopeReport => "text-scope-in",
        WorkbenchEvidenceKind::GoalPlanCheckReport => "text-evidence-warn",
        WorkbenchEvidenceKind::HistoryResponse => "text-evidence-pending",
        WorkbenchEvidenceKind::AssignmentState => "text-goal",
        WorkbenchEvidenceKind::JobState => "text-evidence-fail",
        _ => "text-foreground/70",
    }
}
