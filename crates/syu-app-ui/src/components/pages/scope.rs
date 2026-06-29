#![allow(unused_braces)]

use crate::components::explorer::{EmptyDetail, PageHeader, page_href};
use crate::components::indicators::{EvidenceStamp, IndicatorStatus, StatusCircle};
use crate::i18n::Locale;
use crate::model::{
    ImplementationSlice, PageSection, SliceSource, WorkbenchPage, WorkbenchUiState,
    implementation_slices,
};
use dioxus::prelude::*;
use syu_workbench::{BranchScopeConfidence, OwnershipStatus};

const TABS: [PageSection; 5] = [
    PageSection::CodeTests,
    PageSection::Feature,
    PageSection::Requirement,
    PageSection::Policy,
    PageSection::Philosophy,
];

#[component]
pub fn ScopePage(
    ui: WorkbenchUiState,
    section: Option<PageSection>,
    entity: Option<String>,
    focus_anchor: Option<String>,
) -> Element {
    let copy = ui.copy();
    let selected_tab = section
        .filter(|item| TABS.contains(item))
        .unwrap_or(PageSection::CodeTests);
    let report = ui
        .payload
        .state
        .branch_scope
        .as_ref()
        .and_then(|state| state.report.as_ref());
    let mut slices = report.map(implementation_slices).unwrap_or_default();
    if let Some(goal_id) = entity.as_deref()
        && let Some(goal) = ui
            .payload
            .state
            .goals
            .active
            .iter()
            .find(|goal| goal.goal_id == goal_id)
        && let Some(plan) = goal.goal_plan.as_ref()
    {
        let files = plan
            .implementation_plan
            .scope
            .include
            .iter()
            .map(|entry| entry.pattern().to_string())
            .collect::<Vec<_>>();
        slices.insert(
            0,
            ImplementationSlice {
                id: goal.goal_id.clone(),
                title: plan.goal.title.clone(),
                summary: plan.goal.statement.clone(),
                rationale: format!("This slice is bounded by active Goal {}.", goal.goal_id),
                source: SliceSource::ActiveGoal,
                confidence: match plan.implementation_plan.confidence {
                    Some(syu_task_model::GoalPlanConfidence::High) => BranchScopeConfidence::High,
                    Some(syu_task_model::GoalPlanConfidence::Low) => BranchScopeConfidence::Low,
                    _ => BranchScopeConfidence::Medium,
                },
                include: files.clone(),
                exclude: plan.implementation_plan.scope.exclude.clone(),
                files,
                symbols: Vec::new(),
                tests: plan.completion.must_pass.clone(),
                spec_ids: plan
                    .spec_mapping
                    .persistent_items
                    .philosophies
                    .iter()
                    .chain(plan.spec_mapping.persistent_items.policies.iter())
                    .chain(plan.spec_mapping.persistent_items.requirements.iter())
                    .chain(plan.spec_mapping.persistent_items.features.iter())
                    .map(|item| item.id().to_string())
                    .collect(),
                ownership: OwnershipStatus::Owned,
                evidence: vec![format!("active Goal {}", goal.goal_id)],
                warnings: plan.warnings.clone(),
            },
        );
    }
    if let Some(item_id) = entity.as_deref()
        && let Some(item) = ui.spec_browser.as_ref().and_then(|browser| {
            browser
                .sections
                .iter()
                .flat_map(|section| section.documents.iter())
                .flat_map(|document| document.items.iter())
                .find(|item| item.id == item_id)
        })
    {
        let files = item
            .implementations
            .iter()
            .flat_map(|group| group.references.iter())
            .map(|reference| reference.file.clone())
            .collect::<Vec<_>>();
        let symbols = item
            .implementations
            .iter()
            .flat_map(|group| group.references.iter())
            .flat_map(|reference| reference.symbols.iter())
            .cloned()
            .collect::<Vec<_>>();
        let tests = item
            .tests
            .iter()
            .flat_map(|group| group.references.iter())
            .map(|reference| reference.file.clone())
            .collect::<Vec<_>>();
        slices.insert(
            0,
            ImplementationSlice {
                id: item.id.clone(),
                title: item.title.clone(),
                summary: item.summary.clone().unwrap_or_else(|| item.title.clone()),
                rationale: format!("This slice starts from specification Item {}.", item.id),
                source: SliceSource::ItemDriven,
                confidence: BranchScopeConfidence::High,
                include: files.clone(),
                exclude: vec!["unrelated specification Items".to_string()],
                files,
                symbols,
                tests,
                spec_ids: vec![item.id.clone()],
                ownership: if item.implementations.is_empty() {
                    OwnershipStatus::Unowned
                } else {
                    OwnershipStatus::Owned
                },
                evidence: vec![format!("Item-driven source {}", item.id)],
                warnings: if item.implementations.is_empty() {
                    vec![
                        "No implementation trace is linked; confirm scope before assignment."
                            .to_string(),
                    ]
                } else {
                    Vec::new()
                },
            },
        );
    }
    let selected_id = entity
        .clone()
        .filter(|id| slices.iter().any(|slice| slice.id == *id))
        .or_else(|| slices.first().map(|slice| slice.id.clone()));
    let selected_slice = selected_id
        .as_ref()
        .and_then(|id| slices.iter().find(|slice| slice.id == *id))
        .cloned();
    let focused = focus_anchor.as_deref() == Some("scope-selector");
    let goal_selected = entity.as_deref().is_some_and(|id| {
        ui.payload
            .state
            .goals
            .active
            .iter()
            .any(|goal| goal.goal_id == id)
    });
    rsx! {
        PageHeader { kicker: "Workbench".to_string(), title: copy.page_title(WorkbenchPage::Scope).to_string(), description: copy.page_summary(WorkbenchPage::Scope).to_string(), actions: rsx! {
            if let Some(slice) = selected_slice.as_ref() {
                match slice.source {
                    SliceSource::ActiveGoal => rsx! { a { class: "rounded-lg border border-slate-300 bg-white px-3 py-2 text-sm font-semibold", href: page_href(WorkbenchPage::Work, ui.locale, Some(PageSection::WorkScope), Some(&slice.id), None), if ui.locale == Locale::Ja { "Goal を開く" } else { "Open Goal" } } },
                    SliceSource::ItemDriven => rsx! { button { class: "rounded-lg border border-slate-300 bg-white px-3 py-2 text-sm font-semibold", type: "button", "data-create-work-from-item": "{slice.id}", "data-work-lang": "{ui.locale.slug()}", if ui.locale == Locale::Ja { "Goal に昇格" } else { "Promote to Goal" } } },
                    SliceSource::BranchDiff => rsx! { button { class: "rounded-lg border border-slate-300 bg-white px-3 py-2 text-sm font-semibold", type: "button", "data-create-work-from-branch": "origin/main...HEAD", "data-work-lang": "{ui.locale.slug()}", "data-running-label": if ui.locale == Locale::Ja { "Goal を生成中…" } else { "Creating Goal…" }, if ui.locale == Locale::Ja { "Goal に昇格" } else { "Promote to Goal" } } },
                    SliceSource::Manual => rsx! { a { class: "rounded-lg border border-slate-300 bg-white px-3 py-2 text-sm font-semibold", href: page_href(WorkbenchPage::Work, ui.locale, Some(PageSection::Brief), Some("new"), None), if ui.locale == Locale::Ja { "新しい Work を作成" } else { "Create new Work" } } },
                }
            }
        } }
        div { id: "scope-selector", "data-command-target": "scope-selector", tabindex: "-1", class: if focused { "mb-4 grid gap-2 rounded-lg border-2 border-red-500 p-2 md:grid-cols-[8rem_minmax(0,1fr)]" } else { "mb-4 grid gap-2 md:grid-cols-[8rem_minmax(0,1fr)]" },
            select { class: "rounded-lg border border-slate-300 bg-white px-3 py-2 text-sm", "data-scope-mode": "true", "data-scope-lang": "{ui.locale.slug()}", "data-scope-section": "{selected_tab.slug()}", option { value: "branch", selected: !goal_selected, if ui.locale == Locale::Ja { "ブランチ" } else { "Branch" } } option { value: "goal", selected: goal_selected, disabled: ui.payload.state.goals.active.is_empty(), "Goal" } }
            select { class: "rounded-lg border border-slate-300 bg-white px-3 py-2 text-sm", "data-scope-target": "true", "data-scope-lang": "{ui.locale.slug()}", "data-scope-section": "{selected_tab.slug()}",
                option { value: "", "data-scope-source": "branch", hidden: goal_selected, selected: !goal_selected, if let Some(report) = report { "{report.range}" } else if ui.locale == Locale::Ja { "現在のブランチ" } else { "Current branch" } }
                for goal in &ui.payload.state.goals.active { option { value: "{goal.goal_id}", "data-scope-source": "goal", hidden: !goal_selected, selected: entity.as_deref() == Some(goal.goal_id.as_str()), "{goal.goal_id}" } }
            }
        }
        nav { class: "mb-3 flex flex-wrap gap-1 border-b border-slate-200", "aria-label": "Scope sections", for tab in TABS { a { class: tab_class(tab == selected_tab), href: page_href(WorkbenchPage::Scope, ui.locale, Some(tab), selected_id.as_deref(), None), "{tab_icon(tab)} {copy.section_title(tab)}" } } }
        div { class: "grid items-start gap-3 lg:grid-cols-[18rem_minmax(0,1fr)]",
            aside { class: "rounded-lg border border-slate-200 bg-slate-50 p-2", "aria-label": "Implementation slices",
                div { class: "flex items-center justify-between px-2 py-2", span { class: "text-xs font-medium uppercase text-slate-500", "Implementation slices" } span { class: "rounded-full border bg-white px-2 py-0.5 text-xs", "{slices.len()}" } }
                for slice in &slices { a { class: if selected_id.as_deref() == Some(slice.id.as_str()) { "mb-1 block rounded-lg bg-slate-950 p-3 text-white" } else { "mb-1 block rounded-lg p-3 hover:bg-white" }, href: page_href(WorkbenchPage::Scope, ui.locale, Some(selected_tab), Some(&slice.id), None),
                    div { class: "flex items-start gap-2", StatusCircle { status: ownership_indicator(slice.ownership), label: ownership_label(ui.locale, slice.ownership).to_string(), count: Some(usize::from(slice.ownership != OwnershipStatus::Owned)) } div { strong { class: "block text-sm", "{slice.title}" } span { class: "mt-1 block text-xs opacity-70", "{confidence_label(ui.locale, slice.confidence)} · " if ui.locale == Locale::Ja { "推論" } else { "inferred" } } } }
                } }
                if slices.is_empty() { p { class: "px-2 py-6 text-center text-sm text-slate-500", if ui.locale == Locale::Ja { "Implementation Slice はまだありません。" } else { "No implementation slices yet." } } }
            }
            section { class: "rounded-lg border border-slate-200 bg-slate-50 p-4",
                if let Some(slice) = selected_slice { SliceDetail { ui: ui.clone(), slice, tab: selected_tab } } else { EmptyDetail { title: if ui.locale == Locale::Ja { "スコープを分析してください".to_string() } else { "Analyze the current scope".to_string() }, body: if ui.locale == Locale::Ja { "ブランチまたは Goal の差分から Implementation Slice と根拠を表示します。".to_string() } else { "Implementation slices and their evidence will be derived from the branch or Goal.".to_string() } } }
            }
        }
    }
}

#[component]
fn SliceDetail(ui: WorkbenchUiState, slice: ImplementationSlice, tab: PageSection) -> Element {
    let is_spec_tab = tab != PageSection::CodeTests;
    rsx! {
        div { class: "flex flex-wrap items-start justify-between gap-3",
            div { p { class: "text-[10px] uppercase tracking-[0.2em] text-slate-400", "Implementation slice · {slice.id}" } h2 { class: "mt-1 text-lg font-semibold", "{slice.title}" } }
            div { class: "flex gap-2", EvidenceStamp { label: if ui.locale == Locale::Ja { "推論".to_string() } else { "inferred".to_string() }, inferred: true } span { class: "rounded-full border border-slate-200 bg-white px-2.5 py-1 text-xs", "{confidence_label(ui.locale, slice.confidence)}" } }
        }
        section { class: "mt-4 rounded-lg border border-slate-200 bg-white p-4", h3 { class: "text-[10px] uppercase tracking-[0.18em] text-slate-500", if ui.locale == Locale::Ja { "なぜこのまとまりか" } else { "Why this slice" } } p { class: "mt-2 text-sm leading-6 text-slate-700", "{rationale(ui.locale, &slice)}" } }
        if is_spec_tab {
            section { class: "mt-3 rounded-lg border border-slate-200 bg-white p-4", h3 { class: "text-[10px] uppercase tracking-[0.18em] text-slate-500", "{ui.copy().section_title(tab)}" } if slice.spec_ids.is_empty() { p { class: "mt-2 text-sm text-slate-500", if ui.locale == Locale::Ja { "この階層との関連はまだ確認されていません。" } else { "No relationship to this layer has been verified." } } } else { div { class: "mt-2 flex flex-wrap gap-2", for id in &slice.spec_ids { span { class: "rounded-full border border-slate-200 px-2.5 py-1 text-xs", "{id}" } } } } }
        } else {
            div { class: "mt-3 grid gap-3 md:grid-cols-2", SliceList { title: if ui.locale == Locale::Ja { "対象コード".to_string() } else { "Code in scope".to_string() }, values: slice.files.iter().chain(slice.symbols.iter()).cloned().collect() } SliceList { title: if ui.locale == Locale::Ja { "関連する仕様とテスト".to_string() } else { "Related specs and tests".to_string() }, values: slice.spec_ids.iter().chain(slice.tests.iter()).cloned().collect() } }
            div { class: "mt-3 grid gap-3 md:grid-cols-2", SliceList { title: if ui.locale == Locale::Ja { "含める".to_string() } else { "Include".to_string() }, values: slice.include.clone() } SliceList { title: if ui.locale == Locale::Ja { "含めない".to_string() } else { "Exclude".to_string() }, values: slice.exclude.clone() } }
        }
        section { class: "mt-3 rounded-lg border border-slate-200 bg-white p-4", h3 { class: "text-[10px] uppercase tracking-[0.18em] text-slate-500", if ui.locale == Locale::Ja { "根拠と注意" } else { "Evidence and warnings" } } div { class: "mt-2 flex flex-wrap gap-2", for item in slice.evidence.iter().chain(slice.warnings.iter()) { span { class: "rounded-full border border-slate-200 px-2.5 py-1 text-xs", "{item}" } } } }
    }
}

#[component]
fn SliceList(title: String, values: Vec<String>) -> Element {
    rsx! { section { class: "rounded-lg border border-slate-200 bg-white p-4", h3 { class: "text-[10px] uppercase tracking-[0.18em] text-slate-500", "{title}" } if values.is_empty() { p { class: "mt-2 text-sm text-slate-400", "—" } } else { ul { class: "mt-2 space-y-1 text-sm text-slate-700", for value in values { li { "{value}" } } } } } }
}
fn tab_class(active: bool) -> &'static str {
    if active {
        "border-b-2 border-slate-950 px-3 py-2 text-sm font-semibold"
    } else {
        "px-3 py-2 text-sm font-semibold text-slate-500"
    }
}
fn tab_icon(tab: PageSection) -> &'static str {
    match tab {
        PageSection::CodeTests => "⌘",
        PageSection::Feature => "◇",
        PageSection::Requirement => "□",
        PageSection::Policy => "◌",
        PageSection::Philosophy => "○",
        _ => "",
    }
}
fn confidence_label(locale: Locale, confidence: BranchScopeConfidence) -> &'static str {
    match (locale, confidence) {
        (Locale::Ja, BranchScopeConfidence::High) => "信頼度 高",
        (Locale::Ja, BranchScopeConfidence::Medium) => "信頼度 中",
        (Locale::Ja, BranchScopeConfidence::Low) => "信頼度 低",
        (_, BranchScopeConfidence::High) => "high confidence",
        (_, BranchScopeConfidence::Medium) => "medium confidence",
        (_, BranchScopeConfidence::Low) => "low confidence",
    }
}
fn ownership_indicator(status: OwnershipStatus) -> IndicatorStatus {
    match status {
        OwnershipStatus::Owned => IndicatorStatus::Success,
        OwnershipStatus::Partial => IndicatorStatus::Warning,
        OwnershipStatus::Unowned => IndicatorStatus::Error,
    }
}
fn ownership_label(locale: Locale, status: OwnershipStatus) -> &'static str {
    match (locale, status) {
        (Locale::Ja, OwnershipStatus::Owned) => "所有確認済み",
        (Locale::Ja, OwnershipStatus::Partial) => "所有が曖昧",
        (Locale::Ja, OwnershipStatus::Unowned) => "所有未確認",
        (_, OwnershipStatus::Owned) => "owned",
        (_, OwnershipStatus::Partial) => "ambiguous ownership",
        (_, OwnershipStatus::Unowned) => "unowned",
    }
}
fn rationale(locale: Locale, slice: &ImplementationSlice) -> String {
    if locale != Locale::Ja {
        return slice.rationale.clone();
    }
    if slice.source == SliceSource::ItemDriven {
        return format!(
            "仕様 Item {} を起点として実装範囲を抽出しています。",
            slice.spec_ids.join("、")
        );
    }
    if slice.source == SliceSource::ActiveGoal {
        return "アクティブな Goal Plan の境界から実装範囲を抽出しています。".to_string();
    }
    if slice.spec_ids.is_empty() {
        format!(
            "ブランチ差分に {} が含まれますが、仕様上の所有はまだ確認されていません。",
            slice.files.join("、")
        )
    } else {
        format!(
            "ブランチ差分が {} に紐づくコードを変更しています。",
            slice.spec_ids.join("、")
        )
    }
}
