use crate::components::explorer::{EmptyDetail, PageHeader, page_href};
use crate::components::indicators::{EvidenceStamp, IndicatorStatus, StatusCircle};
use crate::i18n::Locale;
use crate::model::{PageSection, WorkbenchPage, WorkbenchUiState};
use dioxus::prelude::*;
use syu_task_model::{GoalPlanConfidence, GoalPlanPersistentItem};
use syu_workbench::{EvidenceStatus, JobStatus};

const TABS: [PageSection; 4] = [
    PageSection::Brief,
    PageSection::WorkScope,
    PageSection::Delivery,
    PageSection::Evidence,
];

#[component]
pub fn WorkPage(
    ui: WorkbenchUiState,
    section: Option<PageSection>,
    entity: Option<String>,
    focus_anchor: Option<String>,
) -> Element {
    let copy = ui.copy();
    let selected = section
        .filter(|section| TABS.contains(section))
        .unwrap_or(PageSection::Brief);
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
        if !creating && !goals.active.is_empty() {
            div { class: "mb-4 flex flex-wrap items-center gap-2",
                label { class: "sr-only", for: "work-selector", if ui.locale == Locale::Ja { "Work を選択" } else { "Select work" } }
                select { id: "work-selector", class: "min-w-72 rounded-lg border border-slate-300 bg-white px-3 py-2 text-sm", "data-work-selector": "true", "data-work-section": "{selected.slug()}", "data-work-lang": "{ui.locale.slug()}",
                    for goal in &goals.active { option { value: "{goal.goal_id}", selected: Some(goal.goal_id.as_str()) == goals.selected_goal_id.as_deref(), "{goal_title(goal)}" } }
                }
            }
        }
        nav { class: "mb-3 flex gap-1 border-b border-slate-200", "aria-label": "Work sections",
            for tab in TABS { a { class: tab_class(tab == selected), href: page_href(WorkbenchPage::Work, ui.locale, Some(tab), goals.selected_goal_id.as_deref(), None), "{copy.section_title(tab)}" } }
        }
        div { class: if goals.active.len() > 1 { "grid gap-3 lg:grid-cols-[18rem_minmax(0,1fr)]" } else { "grid gap-3" },
            if goals.active.len() > 1 {
                aside { class: "rounded-lg border border-slate-200 bg-slate-50 p-2", "aria-label": "Goals",
                    for goal in &goals.active {
                        a { class: if goals.active_goal().is_some_and(|active| active.goal_id == goal.goal_id) { "mb-1 block rounded-lg bg-slate-950 p-3 text-white" } else { "mb-1 block rounded-lg p-3 hover:bg-white" }, href: page_href(WorkbenchPage::Work, ui.locale, Some(selected), Some(&goal.goal_id), None),
                            div { class: "flex items-center gap-2", StatusCircle { status: goal_status(goal), label: goal_status_label(ui.locale, goal).to_string(), count: Some(goal_issue_count(goal)) } strong { class: "text-sm", "{goal_title(goal)}" } }
                        }
                    }
                }
            }
            section { class: focus_class(focused, anchor_for(selected)), "data-command-target": anchor_for(selected), tabindex: if focused == Some(anchor_for(selected)) { "-1" } else { "0" },
                if creating {
                    NewWorkStart { ui: ui.clone() }
                } else { match active_goal {
                    Some(goal) => match selected {
                        PageSection::Brief => rsx! { Brief { ui: ui.clone(), goal } },
                        PageSection::WorkScope => rsx! { WorkScope { ui: ui.clone(), goal } },
                        PageSection::Delivery => rsx! { Delivery { ui: ui.clone(), goal } },
                        PageSection::Evidence => rsx! { Evidence { ui: ui.clone(), goal_id: goal.goal_id } },
                        _ => rsx! {},
                    },
                    None => rsx! { EmptyDetail { title: if ui.locale == Locale::Ja { "Work がありません".to_string() } else { "No work yet".to_string() }, body: if ui.locale == Locale::Ja { "新しい Work を作成すると、目的・スコープ・完了条件がここに表示されます。".to_string() } else { "Create a Work to review its purpose, scope, and completion conditions here.".to_string() } } },
                } }
            }
        }
    }
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
                    span { class: "grid h-9 w-9 place-items-center rounded-full bg-slate-950 text-white", "▤" }
                    strong { class: "mt-4 block text-sm", if ui.locale == Locale::Ja { "Item から作成" } else { "Create from an Item" } }
                    span { class: "mt-1 block text-xs leading-5 text-slate-500", if ui.locale == Locale::Ja { "仕様 Item を選び、相互リンクと実装根拠を引き継ぎます。" } else { "Select a specification Item and carry its links and implementation evidence forward." } }
                }
                button { class: "group rounded-xl border border-slate-200 bg-white p-5 text-left transition hover:border-slate-400 hover:shadow-sm disabled:opacity-60", type: "button", "data-create-work-from-branch": "origin/main...HEAD", "data-work-lang": "{ui.locale.slug()}", "data-running-label": if ui.locale == Locale::Ja { "ブランチを分析中…" } else { "Analyzing branch…" },
                    span { class: "grid h-9 w-9 place-items-center rounded-full bg-blue-600 text-white", "↗" }
                    strong { class: "mt-4 block text-sm", if ui.locale == Locale::Ja { "現在のブランチから推論" } else { "Infer from the current branch" } }
                    span { class: "mt-1 block text-xs leading-5 text-slate-500", if ui.locale == Locale::Ja { "差分を Implementation Scope と完了条件へ変換します。" } else { "Turn the branch diff into implementation scope and completion conditions." } }
                }
            }
            div { class: "mt-5 flex justify-end", a { class: "rounded-lg px-3 py-2 text-sm font-semibold text-slate-500 hover:bg-slate-100", href: page_href(WorkbenchPage::Work, ui.locale, Some(PageSection::Brief), None, None), if ui.locale == Locale::Ja { "キャンセル" } else { "Cancel" } } }
        }
    }
}

fn tab_class(active: bool) -> &'static str {
    if active {
        "border-b-2 border-slate-950 px-3 py-2 text-sm font-semibold text-slate-950"
    } else {
        "px-3 py-2 text-sm font-semibold text-slate-500 hover:text-slate-950"
    }
}
fn anchor_for(section: PageSection) -> &'static str {
    match section {
        PageSection::Brief => "work-brief",
        PageSection::WorkScope => "work-scope",
        PageSection::Delivery => "assignment",
        PageSection::Evidence => "evidence-timeline",
        _ => "work-brief",
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
fn Brief(ui: WorkbenchUiState, goal: syu_workbench::ActiveGoalState) -> Element {
    let Some(plan) = goal.goal_plan else {
        return rsx! { EmptyDetail { title: "Goal Plan pending".to_string(), body: "The Goal exists, but its human-readable plan has not been generated yet.".to_string() } };
    };
    let confidence = confidence_label(
        ui.locale,
        plan.source
            .confidence
            .or(plan.implementation_plan.confidence),
    );
    let linked = plan
        .spec_mapping
        .persistent_items
        .philosophies
        .iter()
        .chain(plan.spec_mapping.persistent_items.policies.iter())
        .chain(plan.spec_mapping.persistent_items.requirements.iter())
        .chain(plan.spec_mapping.persistent_items.features.iter())
        .map(item_id)
        .collect::<Vec<_>>();
    rsx! {
        div { class: "flex flex-wrap items-start justify-between gap-3",
            div { p { class: "text-[10px] uppercase tracking-[0.2em] text-slate-400", "Goal · human-readable brief" } h2 { class: "mt-1 text-lg font-semibold", "{plan.goal.statement}" } }
            div { class: "flex gap-2", EvidenceStamp { label: if plan.goal.inferred { if ui.locale == Locale::Ja { "推論".to_string() } else { "inferred".to_string() } } else { if ui.locale == Locale::Ja { "確認済み".to_string() } else { "verified".to_string() } }, inferred: plan.goal.inferred } span { class: "rounded-full border border-slate-200 bg-white px-2.5 py-1 text-xs", "{confidence}" } }
        }
        div { class: "mt-4 border-l-2 border-blue-500 bg-blue-50 p-3 text-sm text-blue-900", if ui.locale == Locale::Ja { "最初に目的と境界を確認し、技術的な根拠は後段で確認します。" } else { "Review the purpose and boundaries first; technical evidence follows." } }
        div { class: "mt-4 grid gap-3 md:grid-cols-2",
            InfoBlock { title: if ui.locale == Locale::Ja { "目的".to_string() } else { "Purpose".to_string() }, values: vec![plan.goal.statement.clone()] }
            InfoBlock { title: if ui.locale == Locale::Ja { "完了後の利用体験".to_string() } else { "Expected outcome".to_string() }, values: plan.implementation_plan.steps.first().cloned().into_iter().collect() }
            InfoBlock { title: if ui.locale == Locale::Ja { "対象外".to_string() } else { "Non-goals".to_string() }, values: plan.goal.non_goals.clone() }
            InfoBlock { title: if ui.locale == Locale::Ja { "未解決事項・警告".to_string() } else { "Open questions and warnings".to_string() }, values: plan.warnings.clone() }
        }
        if !linked.is_empty() { div { class: "mt-4 flex flex-wrap gap-2 border-t border-slate-200 pt-3", for id in linked { span { class: "rounded-full border border-slate-200 bg-white px-2.5 py-1 text-xs", "{id}" } } } }
    }
}

#[component]
fn WorkScope(ui: WorkbenchUiState, goal: syu_workbench::ActiveGoalState) -> Element {
    let Some(plan) = goal.goal_plan else {
        return rsx! { EmptyDetail { title: "Scope pending".to_string(), body: "Generate a Goal Plan to establish an implementation boundary.".to_string() } };
    };
    let include = plan
        .implementation_plan
        .scope
        .include
        .iter()
        .map(|item| item.pattern().to_string())
        .collect();
    rsx! { h2 { class: "text-lg font-semibold", if ui.locale == Locale::Ja { "実装範囲と境界" } else { "Implementation scope and boundaries" } } div { class: "mt-4 grid gap-3 md:grid-cols-2", InfoBlock { title: if ui.locale == Locale::Ja { "含める".to_string() } else { "Include".to_string() }, values: include } InfoBlock { title: if ui.locale == Locale::Ja { "含めない".to_string() } else { "Exclude".to_string() }, values: plan.implementation_plan.scope.exclude.clone() } InfoBlock { title: if ui.locale == Locale::Ja { "実装手順".to_string() } else { "Implementation steps".to_string() }, values: plan.implementation_plan.steps.clone() } InfoBlock { title: if ui.locale == Locale::Ja { "推論上の注意".to_string() } else { "Inference warnings".to_string() }, values: plan.warnings.clone() } } }
}

#[component]
fn Delivery(ui: WorkbenchUiState, goal: syu_workbench::ActiveGoalState) -> Element {
    let plan = goal.goal_plan;
    let assignment = ui.payload.state.assignment.clone();
    rsx! {
        div { class: "flex flex-wrap items-center justify-between gap-3", h2 { class: "text-lg font-semibold", if ui.locale == Locale::Ja { "実装の引き渡しと完了条件" } else { "Implementation handoff and completion" } } if let Some(assignment) = &assignment { StatusCircle { status: if assignment.blockers.is_empty() { IndicatorStatus::Success } else { IndicatorStatus::Warning }, label: assignment.scope_guard.status.label().to_string(), count: Some(assignment.blockers.len()) } } }
        div { class: "mt-4 grid gap-3 md:grid-cols-2",
            InfoBlock { title: if ui.locale == Locale::Ja { "担当".to_string() } else { "Assignee".to_string() }, values: assignment.as_ref().and_then(|item| item.assignee.as_ref()).map(|item| format!("{} · {}", item.display_name, item.kind.label())).into_iter().collect() }
            InfoBlock { title: if ui.locale == Locale::Ja { "安全な実行".to_string() } else { "Safety".to_string() }, values: assignment.as_ref().map(|item| vec![format!("{} · isolated worktree: {}", item.run_mode.label(), item.permissions.require_isolated_worktree)]).unwrap_or_default() }
            InfoBlock { title: if ui.locale == Locale::Ja { "実装ステップ".to_string() } else { "Implementation steps".to_string() }, values: plan.as_ref().map(|item| item.implementation_plan.steps.clone()).unwrap_or_default() }
            InfoBlock { title: if ui.locale == Locale::Ja { "完了条件".to_string() } else { "Completion conditions".to_string() }, values: plan.as_ref().map(|item| item.completion.must_pass.clone()).unwrap_or_default() }
            InfoBlock { title: if ui.locale == Locale::Ja { "必須テスト".to_string() } else { "Required tests".to_string() }, values: assignment.as_ref().map(|item| item.scope.required_tests.clone()).unwrap_or_default() }
            InfoBlock { title: if ui.locale == Locale::Ja { "ブロッカー".to_string() } else { "Blockers".to_string() }, values: assignment.as_ref().map(|item| item.blockers.iter().map(|blocker| blocker.message.clone()).collect()).unwrap_or_default() }
        }
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

#[component]
fn InfoBlock(title: String, values: Vec<String>) -> Element {
    rsx! { section { class: "rounded-lg border border-slate-200 bg-white p-4", h3 { class: "text-[10px] uppercase tracking-[0.18em] text-slate-500", "{title}" } if values.is_empty() { p { class: "mt-2 text-sm text-slate-400", "—" } } else if values.len() == 1 { p { class: "mt-2 text-sm leading-6 text-slate-700", "{values[0]}" } } else { ul { class: "mt-2 list-disc space-y-1 pl-5 text-sm text-slate-700", for value in values { li { "{value}" } } } } } }
}

fn item_id(item: &GoalPlanPersistentItem) -> String {
    item.id().to_string()
}
fn confidence_label(locale: Locale, confidence: Option<GoalPlanConfidence>) -> &'static str {
    match (locale, confidence) {
        (Locale::Ja, Some(GoalPlanConfidence::High)) => "信頼度 高",
        (Locale::Ja, Some(GoalPlanConfidence::Medium)) => "信頼度 中",
        (Locale::Ja, Some(GoalPlanConfidence::Low)) => "信頼度 低",
        (_, Some(GoalPlanConfidence::High)) => "high confidence",
        (_, Some(GoalPlanConfidence::Medium)) => "medium confidence",
        (_, Some(GoalPlanConfidence::Low)) => "low confidence",
        (Locale::Ja, None) => "信頼度 未評価",
        (_, None) => "confidence pending",
    }
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
