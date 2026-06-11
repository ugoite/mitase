use super::*;

#[component]
pub(super) fn CommandSurfaceOverview(ui: WorkbenchUiState) -> Element {
    let copy = ui.copy();
    rsx! {
        div { class: "space-y-3",
            div { class: "rounded-2xl border border-border bg-background p-4",
                p { class: "text-sm text-foreground/75", "{copy.command_surface_body()}" }
            }
        }
    }
}

#[component]
pub(super) fn GoalsOverview(ui: WorkbenchUiState) -> Element {
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
                            HelpLink { ui: ui.clone(), topic: HelpTopic::Goals }
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
                        HelpLink { ui: ui.clone(), topic: HelpTopic::Goals }
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
pub(super) fn RequestOverview(ui: WorkbenchUiState) -> Element {
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
                            HelpLink { ui: ui.clone(), topic: HelpTopic::Request }
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
                        HelpLink { ui: ui.clone(), topic: HelpTopic::Request }
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
pub(super) fn BranchOverview(ui: WorkbenchUiState) -> Element {
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
            div { class: "space-y-3", "data-scope-overview": "true",
                div { class: "rounded-2xl border border-border bg-panel p-4 shadow-sm",
                    div { class: "flex flex-wrap items-start justify-between gap-3",
                        div { class: "space-y-1",
                            p { class: "text-[10px] uppercase tracking-[0.24em] text-foreground/45", "branch" }
                            h3 { class: "text-base font-semibold text-foreground", "{report.range.clone()}" }
                        }
                        div { class: "flex items-center gap-2",
                            StatusDot { tone_class: "bg-evidence-pass", label: report.confidence.label().to_string() }
                            HelpLink { ui: ui.clone(), topic: HelpTopic::Branch }
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
            div { class: "space-y-3", "data-scope-overview": "true",
                EmptyState { title: "No branch scope".to_string(), body: "Load scope to see the diff and affected surface.".to_string() }
            }
        }
    }
}

#[component]
pub(super) fn AssignmentOverview(ui: WorkbenchUiState) -> Element {
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
                            HelpLink { ui: ui.clone(), topic: HelpTopic::Assignment }
                        }
                    }
                    div { class: "mt-4 grid gap-3 lg:grid-cols-[0.9fr_1.1fr]",
                        div { class: "space-y-3",
                            AssigneeSelector { assignee: assignment.assignee.clone(), locale: ui.locale }
                            div { class: "rounded-xl border border-border bg-background/30 p-3",
                                p { class: "text-xs uppercase tracking-[0.18em] text-foreground/55", "run mode" }
                                MiniSelect { label: "mode".to_string(), value: assignment.run_mode.label().to_string() }
                                MiniSelect { label: "evidence".to_string(), value: if assignment.evidence_requirements.is_empty() { "none".to_string() } else { format!("{} items", assignment.evidence_requirements.len()) } }
                            }
                        }
                        ScopeGuardPreview { result: assignment.scope_guard.clone(), locale: ui.locale }
                    }
                    div { class: "mt-4 grid gap-3 lg:grid-cols-2",
                        AssignmentPromptPreview { assignment: assignment.clone(), locale: ui.locale }
                        AssignmentEvidencePanel { assignment: assignment.clone(), locale: ui.locale }
                    }
                    div { class: "mt-4",
                        AssignmentConstraintPanel { assignment: assignment.clone(), locale: ui.locale }
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
pub(super) fn GraphOverview(ui: WorkbenchUiState) -> Element {
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
                            HelpLink { ui: ui.clone(), topic: HelpTopic::Graph }
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
                            HelpLink { ui: ui.clone(), topic: HelpTopic::Graph }
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
                                href: "?pane=commands&query=branch%20scope&action=branch.scope",
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
pub(super) fn EvidenceOverview(ui: WorkbenchUiState) -> Element {
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
                            HelpLink { ui: ui.clone(), topic: HelpTopic::Evidence }
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
                            HelpLink { ui: ui.clone(), topic: HelpTopic::Evidence }
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

pub(super) fn evidence_source_label(record: &EvidenceRecord) -> String {
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

pub(super) fn format_timestamp_ms(timestamp_ms: u64) -> String {
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

pub(super) fn civil_from_days(days: i64) -> (i64, i64, i64) {
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
pub(super) fn MetricTile(label: String, value: String) -> Element {
    rsx! {
        div { class: "rounded-xl border border-border bg-background p-3",
            p { class: "text-[10px] uppercase tracking-[0.24em] text-foreground/45", "{label}" }
            p { class: "mt-1 text-sm font-medium text-foreground", "{value}" }
        }
    }
}

#[component]
pub(super) fn MiniSelect(label: String, value: String) -> Element {
    rsx! {
        div { class: "space-y-1",
            p { class: "text-[10px] uppercase tracking-[0.24em] text-foreground/45", "{label}" }
            select { class: "w-full rounded-xl border border-border bg-background px-3 py-2 text-sm outline-none", disabled: true, option { selected: true, "{value}" } }
        }
    }
}
