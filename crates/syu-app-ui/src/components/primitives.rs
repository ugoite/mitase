use crate::design::classes;
use crate::model::CommandPaletteEntry;
use dioxus::prelude::*;
use syu_workbench::{
    EvidenceAttachment, EvidenceCommand, EvidenceRecord, EvidenceStatus, WorkbenchEvidenceKind,
};

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
pub fn CommandOutputView(
    title: String,
    summary: String,
    command: Option<EvidenceCommand>,
    attachment: Option<EvidenceAttachment>,
) -> Element {
    rsx! {
        section { class: "rounded-xl border border-border bg-background/40 p-3",
            div { class: "flex items-center justify-between gap-3",
                p { class: "text-xs uppercase tracking-[0.18em] text-foreground/55", "{title}" }
                if let Some(command) = &command {
                    ScopeChip { label: command.command.clone() }
                }
            }
            p { class: "mt-2 text-sm text-foreground/75", "{summary}" }
            if let Some(command) = &command {
                if !command.args.is_empty() {
                    p { class: "mt-2 text-xs text-foreground/55", "{command.args.join(\" \")}" }
                }
            }
            if let Some(attachment) = attachment {
                if let Some(summary) = attachment.summary {
                    p { class: "mt-2 text-xs uppercase tracking-[0.18em] text-foreground/55", "{summary}" }
                }
                if let Some(content) = attachment.content {
                    pre { class: "mt-2 max-h-40 overflow-auto rounded-lg border border-border bg-panel-muted p-2 text-xs text-foreground/70",
                        "{content}"
                    }
                }
                if attachment.truncated {
                    p { class: "mt-2 text-xs text-evidence-warn", "attachment truncated for the card view" }
                }
            }
        }
    }
}

#[component]
pub fn EvidenceDetailDrawer(record: EvidenceRecord) -> Element {
    rsx! {
        section { class: classes::DRAWER,
            div { class: "flex items-center justify-between gap-3",
                h3 { class: "text-sm font-semibold", "{record.summary}" }
                EvidenceBadge { kind: record.kind }
            }
            p { class: "mt-2 text-xs uppercase tracking-[0.18em] text-foreground/60", "{record.status.label()}" }
            if let Some(goal_id) = &record.goal_id {
                p { class: "mt-2 text-sm text-foreground/80", "Goal: {goal_id}" }
            }
            if let Some(command) = &record.command {
                p { class: "mt-2 text-sm text-foreground/75", "Command: {command.command}" }
            }
            if let Some(attachment) = record.attachments.first() {
                CommandOutputView {
                    title: "Evidence attachment".to_string(),
                    summary: record.summary.clone(),
                    command: record.command.clone(),
                    attachment: Some(attachment.clone()),
                }
            }
        }
    }
}

#[component]
pub fn EvidenceRecordCard(record: EvidenceRecord) -> Element {
    let tone_class = evidence_status_tone(record.status);
    let source_label = record_source_label(&record);
    let attachment = record.attachments.first().cloned();
    rsx! {
        article { class: classes::EVIDENCE_CARD,
            div { class: "flex items-start justify-between gap-3",
                div { class: "min-w-0 space-y-2",
                    div { class: "flex flex-wrap items-center gap-2",
                        StatusDot { tone_class: tone_class, label: record.status.label().to_string() }
                        EvidenceBadge { kind: record.kind }
                        if let Some(goal_id) = &record.goal_id {
                            ScopeChip { label: goal_id.clone() }
                        }
                    }
                    p { class: "text-sm font-medium text-foreground", "{record.summary}" }
                    if let Some(source_label) = source_label {
                        p { class: "text-xs uppercase tracking-[0.18em] text-foreground/55", "{source_label}" }
                    }
                }
                p { class: "shrink-0 text-xs text-foreground/45", "{record.timestamp}" }
            }
            if let Some(command) = &record.command {
                CommandOutputView {
                    title: "Command output".to_string(),
                    summary: record.summary.clone(),
                    command: Some(command.clone()),
                    attachment: attachment.clone(),
                }
            } else if let Some(attachment) = attachment {
                CommandOutputView {
                    title: "Evidence attachment".to_string(),
                    summary: record.summary.clone(),
                    command: None,
                    attachment: Some(attachment),
                }
            }
        }
    }
}

#[component]
pub fn ValidationEvidenceView(record: EvidenceRecord) -> Element {
    rsx! {
        EvidenceRecordCard { record }
    }
}

#[component]
pub fn TestEvidenceView(record: EvidenceRecord) -> Element {
    rsx! {
        EvidenceRecordCard { record }
    }
}

#[component]
pub fn ScopeEvidenceView(record: EvidenceRecord) -> Element {
    rsx! {
        EvidenceRecordCard { record }
    }
}

#[component]
pub fn AgentEvidenceView(record: EvidenceRecord) -> Element {
    rsx! {
        EvidenceRecordCard { record }
    }
}

#[component]
pub fn ManualDecisionEvidenceView(record: EvidenceRecord) -> Element {
    rsx! {
        EvidenceRecordCard { record }
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
pub fn EvidenceLogRow(entry: EvidenceRecord) -> Element {
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
        WorkbenchEvidenceKind::SpecImpactReport => "text-spec-linked",
        WorkbenchEvidenceKind::GoalPlanCheckReport => "text-evidence-warn",
        WorkbenchEvidenceKind::HistoryResponse => "text-evidence-pending",
        WorkbenchEvidenceKind::AssignmentState => "text-goal",
        WorkbenchEvidenceKind::JobState => "text-evidence-fail",
        _ => "text-foreground/70",
    }
}

fn evidence_status_tone(status: EvidenceStatus) -> &'static str {
    match status {
        EvidenceStatus::Pass => "bg-evidence-pass",
        EvidenceStatus::Warn => "bg-evidence-warn",
        EvidenceStatus::Fail => "bg-evidence-fail",
        EvidenceStatus::Pending => "bg-evidence-pending",
        EvidenceStatus::Skipped => "bg-foreground/30",
        EvidenceStatus::Unknown => "bg-foreground/30",
    }
}

fn record_source_label(record: &EvidenceRecord) -> Option<String> {
    match &record.source {
        Some(syu_workbench::EvidenceSource::Action { action_label, .. }) => action_label
            .as_ref()
            .map(|label| format!("source: {label}")),
        Some(syu_workbench::EvidenceSource::Command { command }) => {
            Some(format!("source: command {command}"))
        }
        Some(syu_workbench::EvidenceSource::Manual { actor }) => {
            Some(format!("source: manual {actor}"))
        }
        Some(syu_workbench::EvidenceSource::System { component }) => {
            Some(format!("source: {component}"))
        }
        None => None,
    }
}
