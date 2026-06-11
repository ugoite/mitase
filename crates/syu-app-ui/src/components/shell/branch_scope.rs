use super::*;

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
                    h2 { class: classes::SECTION_TITLE, if ui.locale == Locale::Ja { "ブランチ範囲" } else { "Branch Scope Lens" } }
                    ScopeChip { label: report.as_ref().map(|report| report.range.clone()).unwrap_or_else(|| if ui.locale == Locale::Ja { "範囲待ち".to_string() } else { "range pending".to_string() }) }
                }
                div { class: "grid gap-2 md:grid-cols-5",
                    FlowActionButton { label: if ui.locale == Locale::Ja { "範囲を読む".to_string() } else { "Load scope".to_string() }, action_id: WorkbenchActionId::BranchScope, ui: ui.clone(), onclick: on_run_action }
                    FlowActionButton { label: if ui.locale == Locale::Ja { "ゴール推定".to_string() } else { "Infer goal".to_string() }, action_id: WorkbenchActionId::BranchInferGoal, ui: ui.clone(), onclick: on_run_action }
                    FlowActionButton { label: if ui.locale == Locale::Ja { "影響を見る".to_string() } else { "Spec impact".to_string() }, action_id: WorkbenchActionId::SpecImpact, ui: ui.clone(), onclick: on_run_action }
                    FlowActionButton { label: if ui.locale == Locale::Ja { "範囲を追跡".to_string() } else { "Trace range".to_string() }, action_id: WorkbenchActionId::TraceRange, ui: ui.clone(), onclick: on_run_action }
                    FlowActionButton { label: if ui.locale == Locale::Ja { "関連を見る".to_string() } else { "Relate range".to_string() }, action_id: WorkbenchActionId::RelateRange, ui: ui.clone(), onclick: on_run_action }
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
                    EmptyState {
                        title: if ui.locale == Locale::Ja { "ブランチ範囲待ち".to_string() } else { "Branch scope pending".to_string() },
                        body: (if ui.locale == Locale::Ja { "branch.scope を読み込むと、変更ファイル、所有者、影響を受ける仕様、テストへの影響、厳格レビュー状態を確認できます。" } else { "Load branch.scope to inspect changed files, owners, affected specs, test impact, and strict review status." }).to_string()
                    }
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
                    h2 { class: classes::SECTION_TITLE, if ui.locale == Locale::Ja { "仕様影響グラフ" } else { "Spec Impact Graph" } }
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
                            title: if ui.locale == Locale::Ja { "ブランチ範囲未読込".to_string() } else { "Branch scope not loaded".to_string() },
                            body: (if ui.locale == Locale::Ja { "コマンドパレットから「範囲を読む」を開いてこのグラフを作成します。" } else { "Open Load branch scope from the command palette to build this graph." }).to_string(),
                        }
                        a {
                            class: "flex items-center justify-center rounded-lg border border-border bg-background px-3 py-2 text-sm font-medium text-foreground/80 hover:bg-panel-muted",
                            href: "?pane=commands&query=branch%20scope&action=branch.scope",
                            { if ui.locale == Locale::Ja { "範囲を読む" } else { "Load branch scope" } }
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

pub(super) fn graph_node_origin(index: usize) -> (i32, i32) {
    (
        GRAPH_NODE_X + ((index % GRAPH_COLUMNS) as i32 * GRAPH_COLUMN_WIDTH),
        GRAPH_NODE_Y + ((index / GRAPH_COLUMNS) as i32 * GRAPH_ROW_HEIGHT),
    )
}

pub(super) fn graph_edge_anchor(index: usize, target_index: usize) -> (i32, i32) {
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

pub(super) fn graph_view_height(node_count: usize) -> i32 {
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
                    span { if label == "spec-linked" { "仕様連携" } else if label == "code-linked" { "コード連携" } else if label == "test-linked" { "テスト連携" } else if label == "scope-in" { "範囲内" } else if label == "scope-out" { "範囲外" } else if label == "scope-ambiguous" { "範囲あいまい" } else if label == "ownership-known" { "所有者あり" } else if label == "ownership-missing" { "所有者なし" } else if label == "ownership-ambiguous" { "所有者あいまい" } else if label == "evidence-pass" { "合格" } else if label == "evidence-warn" { "警告" } else if label == "evidence-fail" { "失敗" } else { "保留" } }
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
pub(super) fn GoalComparisonColumn(title: String, tone: String, files: Vec<String>) -> Element {
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
