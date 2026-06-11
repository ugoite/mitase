use super::*;

#[component]
pub fn RequestIntakeCanvas(
    ui: WorkbenchUiState,
    on_run_action: Option<EventHandler<WorkbenchActionId>>,
) -> Element {
    let request = ui.payload.state.request.clone();
    let locale = ui.locale;
    rsx! {
        Panel { class: classes::PANEL_MUTED,
            div { class: "flex flex-col gap-4 p-4",
                div { class: classes::SECTION_HEADER,
                    h2 { class: classes::SECTION_TITLE, if locale == Locale::Ja { "依頼受付" } else { "Request Intake" } }
                    ScopeChip { label: temporary_artifact_label(&ui) }
                }
                RequestContextEditor { ui: ui.clone() }
                div { class: "grid gap-2 md:grid-cols-4",
                    FlowActionButton { label: if locale == Locale::Ja { "分類".to_string() } else { "Classify".to_string() }, action_id: WorkbenchActionId::RequestClassify, ui: ui.clone(), onclick: on_run_action }
                    FlowActionButton { label: if locale == Locale::Ja { "スコープ".to_string() } else { "Scope".to_string() }, action_id: WorkbenchActionId::RequestScope, ui: ui.clone(), onclick: on_run_action }
                    FlowActionButton { label: if locale == Locale::Ja { "雛形を確認".to_string() } else { "Preview scaffold".to_string() }, action_id: WorkbenchActionId::RequestScaffold, ui: ui.clone(), onclick: on_run_action }
                    FlowActionButton { label: if locale == Locale::Ja { "計画を生成".to_string() } else { "Generate plan".to_string() }, action_id: WorkbenchActionId::RequestPlan, ui: ui.clone(), onclick: on_run_action }
                }
                div { class: "grid gap-3 xl:grid-cols-3",
                    RequestClassificationPanel { request: request.clone(), locale }
                    RequestScopePanel { request: request.clone(), locale }
                    ScaffoldPreviewPanel { request, locale }
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
                    h3 { class: "text-sm font-semibold", if ui.locale == Locale::Ja { "変更依頼" } else { "Change request" } }
                    EvidenceBadge { kind: syu_workbench::WorkbenchEvidenceKind::RequestArtifact }
                }
                if let Some(artifact) = request {
                    p { class: "text-base leading-7 text-foreground", "{artifact.request}" }
                    div { class: "flex flex-wrap gap-2",
                        if let Some(area) = &artifact.context.affected_area {
                            ScopeChip { label: if ui.locale == Locale::Ja { format!("範囲: {area}") } else { format!("area: {area}") } }
                        }
                        for id in &artifact.context.linked_ids {
                            ScopeChip { label: id.clone() }
                        }
                    }
                    for constraint in &artifact.context.repository_constraints {
                    p { class: "text-sm text-foreground/70", { if ui.locale == Locale::Ja { format!("制約: {constraint}") } else { format!("constraint: {constraint}") } } }
                    }
                } else {
                    EmptyState {
                        title: if ui.locale == Locale::Ja { "依頼を貼り付け".to_string() } else { "Paste a request".to_string() },
                        body: (if ui.locale == Locale::Ja { "依頼テキストは、仕様を変更する前に一時的な Workbench アーティファクトになります。" } else { "Request text becomes a temporary Workbench artifact before any spec content changes." }).to_string()
                    }
                }
            }
        }
    }
}

#[component]
pub fn RequestClassificationPanel(
    request: Option<syu_workbench::ActiveRequestState>,
    locale: Locale,
) -> Element {
    rsx! {
        Panel { class: classes::PANEL_MUTED,
            div { class: "flex flex-col gap-2 p-3",
                div { class: classes::SECTION_HEADER,
                    h3 { class: "text-sm font-semibold", if locale == Locale::Ja { "分類" } else { "Classify" } }
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
                    EmptyState {
                        title: if locale == Locale::Ja { "未分類".to_string() } else { "Not classified".to_string() },
                        body: (if locale == Locale::Ja { "このキャンバスまたはコマンドパレットから request.classify を実行してください。" } else { "Run request.classify from this canvas or the command palette." }).to_string()
                    }
                }
            }
        }
    }
}

#[component]
pub fn RequestScopePanel(
    request: Option<syu_workbench::ActiveRequestState>,
    locale: Locale,
) -> Element {
    rsx! {
        Panel { class: classes::PANEL_MUTED,
            div { class: "flex flex-col gap-2 p-3",
                div { class: classes::SECTION_HEADER,
                    h3 { class: "text-sm font-semibold", if locale == Locale::Ja { "スコープ" } else { "Scope" } }
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
                    EmptyState {
                        title: if locale == Locale::Ja { "スコープ待ち".to_string() } else { "Scope pending".to_string() },
                        body: (if locale == Locale::Ja { "実装計画の前に、依頼を関連する仕様へ対応付けてください。" } else { "Map the request to relevant specs before planning implementation work." }).to_string()
                    }
                }
            }
        }
    }
}

#[component]
pub fn ScaffoldPreviewPanel(
    request: Option<syu_workbench::ActiveRequestState>,
    locale: Locale,
) -> Element {
    rsx! {
        Panel { class: classes::PANEL_MUTED,
            div { class: "flex flex-col gap-2 p-3",
                div { class: classes::SECTION_HEADER,
                    h3 { class: "text-sm font-semibold", if locale == Locale::Ja { "雛形プレビュー" } else { "Scaffold Preview" } }
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
                    EmptyState {
                        title: if locale == Locale::Ja { "雛形プレビューなし".to_string() } else { "No scaffold preview".to_string() },
                        body: (if locale == Locale::Ja { "確定した永続コンテンツとして扱わずに仕様更新をプレビューします。" } else { "Preview spec updates without treating them as committed persistent content." }).to_string()
                    }
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
    let locale = ui.locale;
    rsx! {
        section { class: "flex flex-col gap-3",
            div { class: classes::SECTION_HEADER,
                h2 { class: classes::SECTION_TITLE, if locale == Locale::Ja { "ゴール計画" } else { "Goal Plan" } }
                ScopeChip { label: format!("{} temporary cards", goals.len()) }
            }
            if goals.is_empty() {
                EmptyState {
                    title: if locale == Locale::Ja { "生成された Goal Plan なし".to_string() } else { "No generated Goal Plan".to_string() },
                    body: (if locale == Locale::Ja { "依頼の分類とスコープが終わったら request.plan を実行してください。" } else { "Run request.plan after the request is classified and scoped." }).to_string()
                }
            } else {
                for goal in goals {
                    if let Some(plan) = goal.goal_plan.clone() {
                        GoalPlanCard { plan: plan.clone() }
                    }
                }
                div { class: "grid gap-2 md:grid-cols-3",
                    FlowActionButton { label: if locale == Locale::Ja { "テスト選択".to_string() } else { "Select tests".to_string() }, action_id: WorkbenchActionId::GoalTestSelect, ui: ui.clone(), onclick: on_run_action }
                    FlowActionButton { label: if locale == Locale::Ja { "ゴール確認".to_string() } else { "Check goal".to_string() }, action_id: WorkbenchActionId::GoalCheck, ui: ui.clone(), onclick: on_run_action }
                    FlowActionButton { label: if locale == Locale::Ja { "次を割り当て".to_string() } else { "Assign next".to_string() }, action_id: WorkbenchActionId::AssignmentCreate, ui: ui.clone(), onclick: on_run_action }
                }
            }
        }
    }
}

#[component]
pub(super) fn GoalPlanCard(plan: GoalPlanArtifact) -> Element {
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
