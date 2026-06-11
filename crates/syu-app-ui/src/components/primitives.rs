use crate::WorkbenchPane;
use crate::design::classes;
use crate::i18n::Locale;
use crate::model::CommandPaletteEntry;
use dioxus::prelude::*;
use syu_workbench::{
    EvidenceAttachment, EvidenceCommand, EvidenceRecord, EvidenceStatus, WorkbenchEvidenceKind,
};

#[component]
pub fn Panel(class: &'static str, children: Element) -> Element {
    rsx! {
        div { class: "{class}", {children} }
    }
}

#[component]
pub fn Button(label: String, active: bool, disabled: bool) -> Element {
    let class = if active {
        "inline-flex items-center gap-2 rounded-full border border-command-active bg-command-active px-3 py-1.5 text-xs font-medium uppercase tracking-[0.18em] text-background"
    } else {
        "inline-flex items-center gap-2 rounded-full border border-border bg-panel-muted px-3 py-1.5 text-xs font-medium uppercase tracking-[0.18em] text-foreground/80 hover:bg-panel"
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
            class: "inline-flex items-center gap-2 rounded-full border border-border bg-panel-muted px-3 py-1.5 text-xs font-medium uppercase tracking-[0.18em] text-foreground/80 hover:bg-panel",
            type: "button",
            title: "{label}",
            span { class: "grid h-5 w-5 place-items-center rounded-full border border-border/70 text-foreground/60", "{icon}" }
            span { class: "sr-only", "{label}" }
        }
    }
}

#[component]
pub fn StatusDot(tone_class: &'static str, label: String) -> Element {
    rsx! {
        span { class: "inline-flex items-center gap-2 text-[10px] uppercase tracking-[0.22em] text-foreground/65",
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
        section { class: "mt-3 rounded-xl border border-border/70 bg-background/30 p-3",
            div { class: "flex items-center justify-between gap-3",
                p { class: "text-[10px] uppercase tracking-[0.24em] text-foreground/45", "{title}" }
                if let Some(command) = &command {
                    ScopeChip { label: command.command.clone() }
                }
            }
            p { class: "mt-2 break-all text-sm text-foreground/75", "{summary}" }
            if let Some(command) = &command {
                if !command.args.is_empty() {
                    p { class: "mt-2 break-all text-xs text-foreground/50", "{command.args.join(\" \")}" }
                }
            }
            if let Some(attachment) = attachment {
                if let Some(summary) = attachment.summary {
                    p { class: "mt-2 text-[10px] uppercase tracking-[0.24em] text-foreground/45", "{summary}" }
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
            p { class: "mt-2 text-[10px] uppercase tracking-[0.24em] text-foreground/55", "{record.status.label()}" }
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
    let timestamp_label = format_timestamp_ms(record.timestamp);
    rsx! {
        details { class: classes::EVIDENCE_CARD,
            summary { class: "list-none cursor-pointer rounded-xl outline-none",
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
                            p { class: "text-[10px] uppercase tracking-[0.24em] text-foreground/45", "{source_label}" }
                        }
                    }
                    p { class: "shrink-0 text-xs text-foreground/45", "{timestamp_label}" }
                }
            }
            div { class: "mt-3 space-y-3",
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
        "rounded-xl border border-goal-active/80 bg-panel/80 p-3 shadow-[0_0_0_1px_rgba(255,255,255,0.02)]"
    } else {
        "rounded-xl border border-border bg-panel-muted p-3"
    };
    rsx! {
        a {
            class: class,
            href: format!("?pane=goals&goal={goal_id}"),
            aria_current: if selected { "page" } else { "false" },
            p { class: "text-[10px] uppercase tracking-[0.24em] text-foreground/55", "{goal_id}" }
            p { class: "mt-1 text-sm font-medium text-foreground/90", "{title}" }
        }
    }
}

#[component]
pub fn CommandItem(
    entry: CommandPaletteEntry,
    selected: bool,
    locale: Locale,
    category: Option<crate::model::CommandCategory>,
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
    let category_param =
        category.map_or_else(String::new, |value| format!("&category={}", value.slug()));
    let pane_slug = command_item_pane_slug(WorkbenchPane::for_action(entry.action.id));
    let href = if entry.availability.available && !entry.action.mutability.requires_confirmation() {
        format!(
            "?pane={}&lang={}&action={}&run=1{}",
            pane_slug,
            locale.slug(),
            entry.action.id.label(),
            category_param,
        )
    } else {
        format!(
            "?pane={}&lang={}&action={}{}",
            pane_slug,
            locale.slug(),
            entry.action.id.label(),
            category_param,
        )
    };
    let action_category = crate::model::workbench_action_category(entry.action.id);
    rsx! {
        a {
            class: "{class} {disabled_class}",
            href,
            title: "{entry.action.description}",
            "data-command-item": "true",
            "data-command-text": format!("{} {} {}", entry.action.id.label(), entry.action.title, entry.action.description),
            "data-command-id": entry.action.id.label(),
            "data-command-title": entry.action.title.clone(),
            "data-command-category": action_category.slug(),
            div { class: "flex items-start gap-3 text-left",
                span { class: "grid h-8 w-8 shrink-0 place-items-center rounded-full border border-border bg-panel-muted text-xs text-foreground/70", "{action_icon(entry.action.id)}" }
                div { class: "flex min-w-0 flex-col gap-1",
                    span { class: "text-sm font-medium text-foreground", "{entry.action.title}" }
                    span { class: "text-[10px] uppercase tracking-[0.24em] text-foreground/45", "{entry.action.id.label()}" }
                }
            }
            div { class: "flex flex-col items-end gap-1 text-xs uppercase tracking-[0.18em]",
                span { class: "rounded-full border border-border bg-background px-2 py-0.5 text-[9px] tracking-[0.16em] text-foreground/60", "{action_category.label()}" }
                if let Some(reason) = &entry.disabled_reason {
                    span { class: "max-w-[11rem] text-right normal-case tracking-normal text-foreground/50", "{reason}" }
                } else {
                    span { class: "normal-case tracking-normal text-foreground/50", "ready" }
                }
            }
        }
    }
}

fn command_item_pane_slug(pane: WorkbenchPane) -> &'static str {
    if pane == WorkbenchPane::Pulse {
        WorkbenchPane::Request.slug()
    } else {
        pane.slug()
    }
}

#[component]
pub fn EmptyState(title: String, body: String) -> Element {
    rsx! {
        div { class: classes::EMPTY_STATE,
            p { class: "text-sm font-semibold text-foreground/90", "{title}" }
            p { class: "mt-1 text-sm text-foreground/65", "{body}" }
        }
    }
}

#[component]
pub fn DetailDrawer(title: String, body: String, evidence: String) -> Element {
    rsx! {
        section { class: classes::DRAWER,
            div { class: "flex items-center justify-between gap-3",
                h3 { class: "text-sm font-semibold", "{title}" }
                ScopeChip { label: "result".to_string() }
            }
            p { class: "mt-2 break-all text-sm text-foreground/75", "{body}" }
            p { class: "mt-2 break-all text-[10px] uppercase tracking-[0.24em] text-foreground/45", "{evidence}" }
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
        WorkbenchEvidenceKind::AgentRun => "text-evidence-pending",
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

pub(crate) fn action_icon(action_id: syu_workbench::WorkbenchActionId) -> &'static str {
    match action_id {
        syu_workbench::WorkbenchActionId::RequestNew => "＋",
        syu_workbench::WorkbenchActionId::RequestClassify => "◌",
        syu_workbench::WorkbenchActionId::RequestScope => "⌁",
        syu_workbench::WorkbenchActionId::RequestScaffold => "▣",
        syu_workbench::WorkbenchActionId::RequestPlan => "▤",
        syu_workbench::WorkbenchActionId::GoalTestSelect => "✓",
        syu_workbench::WorkbenchActionId::GoalCheck => "◎",
        syu_workbench::WorkbenchActionId::BranchScope => "↻",
        syu_workbench::WorkbenchActionId::BranchInferGoal => "↗",
        syu_workbench::WorkbenchActionId::SpecImpact => "◈",
        syu_workbench::WorkbenchActionId::TraceRange => "⋯",
        syu_workbench::WorkbenchActionId::RelateRange => "⊕",
        syu_workbench::WorkbenchActionId::ValidationRun => "⟐",
        syu_workbench::WorkbenchActionId::HistoryShow => "⌂",
        syu_workbench::WorkbenchActionId::AssignmentCreate => "✦",
        syu_workbench::WorkbenchActionId::AssignmentPreview => "◫",
        syu_workbench::WorkbenchActionId::AssignmentRunDry => "↯",
        syu_workbench::WorkbenchActionId::AssignmentRun => "▶",
        syu_workbench::WorkbenchActionId::AssignmentCancel => "✕",
        syu_workbench::WorkbenchActionId::AssignmentRecordManual => "✎",
        syu_workbench::WorkbenchActionId::AssignmentCollectEvidence => "⟡",
        syu_workbench::WorkbenchActionId::AgentRun => "⌘",
    }
}
