use crate::components::explorer::{EmptyDetail, PageHeader, page_href};
use crate::components::icons::{IconName, SyuIcon};
use crate::components::indicators::{
    ImpactRoleBadge, IndicatorStatus, StatusCircle, WorkKindBadge,
};
use crate::i18n::Locale;
use crate::model::{PageSection, WorkbenchPage, WorkbenchUiState};
use dioxus::prelude::*;
use syu_task_model::{WorkKind, WorkPlan};
use syu_workbench::{EvidenceStatus, JobStatus};

#[component]
pub fn WorkPage(
    ui: WorkbenchUiState,
    section: Option<PageSection>,
    entity: Option<String>,
    focus_anchor: Option<String>,
) -> Element {
    let copy = ui.copy();
    let _ = section;
    let goals = &ui.payload.state.goals;
    let creating = entity.as_deref() == Some("new");
    let active_goal = (!creating).then(|| goals.active_goal().cloned()).flatten();
    let focused = focus_anchor.as_deref();
    rsx! {
        PageHeader {
            kicker: "Workbench".to_string(),
            title: copy.page_title(WorkbenchPage::Work).to_string(),
            description: copy.page_summary(WorkbenchPage::Work).to_string(),
            actions: rsx! {
                a { class: "rounded-lg border border-slate-300 bg-white px-3 py-2 text-sm font-semibold", href: page_href(WorkbenchPage::Work, ui.locale, Some(PageSection::Brief), Some("new"), None), "{copy.new_work()}" }
            }
        }
        div { class: if !goals.active.is_empty() { "grid gap-3 lg:grid-cols-[18rem_minmax(0,1fr)]" } else { "grid gap-3" },
            if !goals.active.is_empty() && !creating {
                aside { class: "rounded-lg border border-slate-200 bg-slate-50 p-2", "aria-label": "Goals", "data-goal-rail": "true",
                    for goal in &goals.active {
                        a { class: if goals.active_goal().is_some_and(|active| active.goal_id == goal.goal_id) { "mb-1 block rounded-lg bg-slate-950 p-3 text-white" } else { "mb-1 block rounded-lg p-3 hover:bg-white" }, href: page_href(WorkbenchPage::Work, ui.locale, None, Some(&goal.goal_id), None),
                            div { class: "flex items-center gap-2", StatusCircle { status: goal_status(goal), label: goal_status_label(ui.locale, goal).to_string(), count: Some(goal_issue_count(goal)) } strong { class: "text-sm", "{goal_title(goal)}" } }
                        }
                    }
                }
            }
            section { class: focus_class(focused, "work-detail"), "data-command-target": "work-detail", tabindex: if focused == Some("work-detail") { "-1" } else { "0" },
                if creating {
                    NewWorkStart { ui: ui.clone() }
                } else { match active_goal {
                    Some(goal) => rsx! { WorkDetails { ui: ui.clone(), goal } },
                    None => rsx! { EmptyDetail { title: if ui.locale == Locale::Ja { "Work がありません".to_string() } else { "No work yet".to_string() }, body: if ui.locale == Locale::Ja { "新しい Work を作成すると、目的・スコープ・完了条件がここに表示されます。".to_string() } else { "Create a Work to review its purpose, scope, and completion conditions here.".to_string() } } },
                } }
            }
        }
    }
}

#[component]
fn WorkDetails(ui: WorkbenchUiState, goal: syu_workbench::ActiveGoalState) -> Element {
    let Some(plan) = goal.goal_plan else {
        return rsx! { EmptyDetail { title: "Plan pending".to_string(), body: "The planner is preparing this Work.".to_string() } };
    };
    let work = plan.work.clone();
    let kind = work
        .as_ref()
        .map(|work| work.intent.kind)
        .unwrap_or(WorkKind::Deliver);
    let status = if work.as_ref().is_none_or(|work| work.executable) {
        IndicatorStatus::Success
    } else {
        IndicatorStatus::Warning
    };
    rsx! {
        header { class: "flex flex-wrap items-start justify-between gap-3 border-b border-slate-200 pb-4",
            div { h2 { class: "text-xl font-semibold", "{plan.goal.title}" } p { class: "mt-1 text-sm text-slate-600", "{plan.goal.statement}" } }
            div { class: "flex items-center gap-2", WorkKindBadge { kind } StatusCircle { status, label: if status == IndicatorStatus::Success { "ready".to_string() } else { "attention required".to_string() }, count: None } }
        }
        div { class: "mt-4 grid gap-4",
            OutcomeSection { locale: ui.locale, kind, plan: plan.clone() }
            if let Some(work) = work.clone() { ImpactSection { work: work.clone() } }
            if let Some(work) = work.clone() { VerificationSection { work } }
            Activity { ui: ui.clone(), goal_id: goal.goal_id }
        }
    }
}

#[component]
fn OutcomeSection(
    locale: Locale,
    kind: WorkKind,
    plan: syu_task_model::GoalPlanArtifact,
) -> Element {
    let non_goals = plan.goal.non_goals.join(", ");
    let title = match kind {
        WorkKind::Govern => "Principle or policy change",
        WorkKind::Restructure => "Topology operation",
        WorkKind::Verify => "Quality target",
        WorkKind::Repair => "Proposed minimal fix",
        WorkKind::Retire => "Retired subject",
        WorkKind::Review => "Question and recommendation",
        WorkKind::Adopt => "Workspace target",
        _ => "Outcome",
    };
    rsx! { section { "data-work-section": "outcome", h3 { class: "text-xs font-semibold uppercase tracking-wide text-slate-500", "{title}" } p { class: "mt-2 text-sm leading-6", "{plan.goal.statement}" } if !non_goals.is_empty() { p { class: "mt-2 text-xs text-slate-500", if locale == Locale::Ja { "対象外: " } else { "Excluded: " } "{non_goals}" } } } }
}

#[component]
fn ImpactSection(work: WorkPlan) -> Element {
    rsx! { section { class: "rounded-lg border border-slate-200 bg-white", "data-work-section": "impact",
        h3 { class: "border-b border-slate-200 px-4 py-3 text-xs font-semibold uppercase tracking-wide text-slate-500", "Impact" }
        div { class: "divide-y divide-slate-100",
            for item in &work.impact.items { div { class: "grid gap-2 px-4 py-3 sm:grid-cols-[auto_9rem_minmax(0,1fr)] sm:items-center", ImpactRoleBadge { role: item.impact_role } strong { class: "text-sm", "{item.id}" } p { class: "text-xs text-slate-600", "{item.reason}" } } }
            for item in &work.impact.repository { div { class: "grid gap-2 px-4 py-3 sm:grid-cols-[auto_9rem_minmax(0,1fr)] sm:items-center", ImpactRoleBadge { role: item.impact_role } strong { class: "text-sm", "{item.path}" } p { class: "text-xs text-slate-600", "{item.reason}" } } }
        }
        for suggestion in &work.impact.split_suggestions { div { class: "border-t border-amber-200 bg-amber-50 px-4 py-3 text-xs text-amber-900", "data-split-suggestion": "true", "{suggestion.reason}" } }
    } }
}

#[component]
fn VerificationSection(work: WorkPlan) -> Element {
    rsx! { section { "data-work-section": "verification", h3 { class: "text-xs font-semibold uppercase tracking-wide text-slate-500", "Verification" } ul { class: "mt-2 space-y-1 text-sm", for check in &work.verification.completion { li { class: "rounded bg-slate-100 px-3 py-2 font-mono text-xs", "{check.render()}" } } } } }
}

#[component]
fn Activity(ui: WorkbenchUiState, goal_id: String) -> Element {
    rsx! { details { class: "border-t border-slate-200 pt-3", "data-work-section": "activity", summary { class: "cursor-pointer text-sm font-semibold", "Activity" } div { class: "mt-3", Evidence { ui, goal_id } } } }
}

#[component]
fn NewWorkStart(ui: WorkbenchUiState) -> Element {
    rsx! {
        div { class: "mx-auto max-w-3xl py-4",
            p { class: "text-[10px] uppercase tracking-[0.2em] text-slate-400", if ui.locale == Locale::Ja { "New Work · source first" } else { "New Work · source first" } }
            h2 { class: "mt-1 text-xl font-semibold", if ui.locale == Locale::Ja { "新しい Work の作成方法" } else { "Choose how to create the new Work" } }
            p { class: "mt-2 text-sm leading-6 text-slate-600", if ui.locale == Locale::Ja { "根拠のない空の Work は作らず、Item または現在のブランチを起点に Goal Plan を生成します。" } else { "Start from an Item or the current branch so every Work begins with traceable evidence." } }
            div { class: "mt-5 grid gap-3 md:grid-cols-2",
                a { class: "group rounded-xl border border-slate-200 bg-white p-5 transition hover:border-slate-400 hover:shadow-sm", href: page_href(WorkbenchPage::Items, ui.locale, Some(PageSection::Requirement), None, None),
                    span { class: "grid h-9 w-9 place-items-center rounded-full bg-slate-950 text-white", SyuIcon { name: IconName::Items, size: 19 } }
                    strong { class: "mt-4 block text-sm", if ui.locale == Locale::Ja { "Item から作成" } else { "Create from an Item" } }
                    span { class: "mt-1 block text-xs leading-5 text-slate-500", if ui.locale == Locale::Ja { "仕様 Item を選び、相互リンクと実装根拠を引き継ぎます。" } else { "Select a specification Item and carry its links and implementation evidence forward." } }
                }
                button { class: "group rounded-xl border border-slate-200 bg-white p-5 text-left transition hover:border-slate-400 hover:shadow-sm disabled:opacity-60", type: "button", "data-create-work-from-branch": "origin/main...HEAD", "data-work-lang": "{ui.locale.slug()}", "data-running-label": if ui.locale == Locale::Ja { "ブランチを分析中…" } else { "Analyzing branch…" },
                    span { class: "grid h-9 w-9 place-items-center rounded-full bg-blue-600 text-white", SyuIcon { name: IconName::Trace, size: 19 } }
                    strong { class: "mt-4 block text-sm", if ui.locale == Locale::Ja { "現在のブランチから推論" } else { "Infer from the current branch" } }
                    span { class: "mt-1 block text-xs leading-5 text-slate-500", if ui.locale == Locale::Ja { "差分を Implementation Scope と完了条件へ変換します。" } else { "Turn the branch diff into implementation scope and completion conditions." } }
                }
            }
            div { class: "mt-5 flex justify-end", a { class: "rounded-lg px-3 py-2 text-sm font-semibold text-slate-500 hover:bg-slate-100", href: page_href(WorkbenchPage::Work, ui.locale, Some(PageSection::Brief), None, None), if ui.locale == Locale::Ja { "キャンセル" } else { "Cancel" } } }
        }
    }
}

fn focus_class(focus: Option<&str>, anchor: &str) -> &'static str {
    if focus == Some(anchor) {
        "rounded-lg border-2 border-red-500 bg-slate-50 p-4 outline-none"
    } else {
        "rounded-lg border border-slate-200 bg-slate-50 p-4 outline-none"
    }
}

fn goal_title(goal: &syu_workbench::ActiveGoalState) -> String {
    goal.goal_plan
        .as_ref()
        .map(|plan| format!("{} {}", plan.goal.id, plan.goal.title))
        .unwrap_or_else(|| goal.goal_id.clone())
}
fn goal_status(goal: &syu_workbench::ActiveGoalState) -> IndicatorStatus {
    if goal
        .check_report
        .as_ref()
        .is_some_and(|report| !report.passed())
    {
        IndicatorStatus::Warning
    } else if goal.goal_plan.is_some() {
        IndicatorStatus::Success
    } else {
        IndicatorStatus::Disabled
    }
}
fn goal_issue_count(goal: &syu_workbench::ActiveGoalState) -> usize {
    goal.check_report
        .as_ref()
        .map(|report| report.error_count() + report.warning_count())
        .unwrap_or(0)
}
fn goal_status_label(locale: Locale, goal: &syu_workbench::ActiveGoalState) -> &'static str {
    if goal.goal_plan.is_some() {
        if locale == Locale::Ja {
            "計画あり"
        } else {
            "plan ready"
        }
    } else if locale == Locale::Ja {
        "計画待ち"
    } else {
        "plan pending"
    }
}

#[component]
fn Evidence(ui: WorkbenchUiState, goal_id: String) -> Element {
    let records = ui
        .payload
        .state
        .evidence_timeline
        .entries
        .iter()
        .filter(|record| record.goal_id.as_deref().is_none_or(|id| id == goal_id))
        .cloned()
        .collect::<Vec<_>>();
    rsx! { h2 { class: "text-lg font-semibold", if ui.locale == Locale::Ja { "実行・確認の履歴" } else { "Execution and review history" } } div { class: "mt-3 divide-y divide-slate-200 border-y border-slate-200",
        for record in records.iter().rev() { EvidenceRecordRow { locale: ui.locale, record: record.clone() } }
        if records.is_empty() { p { class: "py-8 text-center text-sm text-slate-500", if ui.locale == Locale::Ja { "この Goal の証拠はまだありません。" } else { "No evidence has been recorded for this Goal." } } }
    } }
}

#[component]
fn EvidenceRecordRow(locale: Locale, record: syu_workbench::EvidenceRecord) -> Element {
    let raw = record
        .attachments
        .iter()
        .filter_map(|item| item.content.clone())
        .collect::<Vec<_>>()
        .join("\n");
    rsx! { article { class: "grid gap-2 py-3 sm:grid-cols-[8rem_minmax(0,1fr)_auto]", StatusCircle { status: evidence_indicator(record.status), label: format!("{:?}", record.status), count: None } div { strong { class: "text-sm", "{record.summary}" } p { class: "mt-1 text-xs text-slate-500", "{record.kind.label()} · {record.timestamp}" } } if !record.attachments.is_empty() { details { class: "text-xs", summary { class: "cursor-pointer", if locale == Locale::Ja { "根拠を表示" } else { "Show evidence" } } pre { class: "mt-2 max-w-xl overflow-auto rounded bg-slate-950 p-3 text-slate-100", "{raw}" } } } } }
}

fn evidence_indicator(status: EvidenceStatus) -> IndicatorStatus {
    match status {
        EvidenceStatus::Pass => IndicatorStatus::Success,
        EvidenceStatus::Warn => IndicatorStatus::Warning,
        EvidenceStatus::Fail => IndicatorStatus::Error,
        EvidenceStatus::Pending => IndicatorStatus::Running,
        EvidenceStatus::Skipped | EvidenceStatus::Unknown => IndicatorStatus::Disabled,
    }
}
#[allow(dead_code)]
fn job_indicator(status: JobStatus) -> IndicatorStatus {
    match status {
        JobStatus::Completed => IndicatorStatus::Success,
        JobStatus::Failed => IndicatorStatus::Error,
        JobStatus::Queued | JobStatus::Running => IndicatorStatus::Running,
        JobStatus::Idle => IndicatorStatus::Disabled,
    }
}
