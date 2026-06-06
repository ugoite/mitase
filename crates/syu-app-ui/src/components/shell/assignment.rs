use super::*;

#[component]
pub fn AssignGoalDialog(
    ui: WorkbenchUiState,
    on_run_action: Option<EventHandler<WorkbenchActionId>>,
) -> Element {
    let assignment = ui.payload.state.assignment.clone();
    let is_automated_assignee = assignment
        .as_ref()
        .is_some_and(assignment_has_automated_assignee);
    rsx! {
        Panel { class: classes::PANEL_MUTED,
            div { class: "space-y-4 p-4",
                div { class: classes::SECTION_HEADER,
                    h2 { class: classes::SECTION_TITLE, "Scoped Assignment" }
                    if let Some(assignment) = &assignment {
                        StatusDot {
                            tone_class: assignment_status_tone(assignment.status),
                            label: assignment.status.label().to_string(),
                        }
                    } else {
                        ScopeChip { label: "assignment-blocked".to_string() }
                    }
                }
                if let Some(assignment) = assignment {
                    AssigneeSelector { assignee: assignment.assignee.clone() }
                    ScopeGuardPreview { result: assignment.scope_guard.clone() }
                    AssignmentConstraintPanel { assignment: assignment.clone() }
                    AssignmentPromptPreview { assignment: assignment.clone() }
                    if let Some(run) = assignment.latest_run.clone() {
                        AgentRunPanel { run }
                    } else if matches!(assignment.assignee.as_ref().map(|assignee| assignee.kind), Some(AssigneeKind::Human)) {
                        HumanAssignmentPanel { assignment: assignment.clone() }
                    }
                    AssignmentEvidencePanel { assignment: assignment.clone() }
                    if let Some(on_run_action) = on_run_action {
                        div { class: "flex flex-wrap gap-2",
                            button {
                                class: "rounded-full border border-border bg-panel-muted px-3 py-1.5 text-xs uppercase tracking-[0.16em] text-foreground/70",
                                disabled: !assignment.is_runnable(),
                                onclick: move |_| on_run_action.call(WorkbenchActionId::AssignmentPreview),
                                "Preview"
                            }
                            if is_automated_assignee {
                                button {
                                    class: "rounded-full border border-command-active bg-command-active px-3 py-1.5 text-xs uppercase tracking-[0.16em] text-background",
                                    disabled: !assignment.is_runnable(),
                                    onclick: move |_| on_run_action.call(WorkbenchActionId::AssignmentRunDry),
                                    "Dry Run"
                                }
                            }
                        }
                    }
                } else {
                    EmptyState {
                        title: "No assignment loaded".to_string(),
                        body: "Create assignment keeps Goal scope, non-goals, tests, completion commands, and required evidence together.".to_string()
                    }
                }
            }
        }
    }
}

#[component]
pub fn AssigneeSelector(assignee: Option<Assignee>) -> Element {
    rsx! {
        section { class: "rounded-xl border border-border bg-background/30 p-3",
            p { class: "text-xs uppercase tracking-[0.18em] text-foreground/55", "Assignee Selector" }
            if let Some(assignee) = assignee {
                div { class: "mt-2 flex flex-wrap items-center gap-2",
                    ScopeChip { label: assignee.kind.label().to_string() }
                    ScopeChip { label: assignee.id.clone() }
                    p { class: "text-sm font-medium", "{assignee.display_name}" }
                }
            } else {
                p { class: "mt-2 text-sm text-evidence-warn", "assignment-blocked: assignee missing" }
            }
        }
    }
}

#[component]
pub fn ScopeGuardPreview(result: ScopeGuardResult) -> Element {
    rsx! {
        section { class: "rounded-xl border border-border bg-background/30 p-3",
            div { class: "flex flex-wrap items-center gap-2",
                p { class: "text-xs uppercase tracking-[0.18em] text-foreground/55", "Scope Guard Preview" }
                StatusDot { tone_class: scope_guard_tone(result.status), label: result.status.label().to_string() }
            }
            div { class: "mt-2 flex flex-wrap gap-2",
                ScopeChip { label: "scope-in".to_string() }
                ScopeChip { label: "scope-out".to_string() }
                ScopeChip { label: "out-of-scope changes".to_string() }
                ScopeChip { label: result.status.label().to_string() }
            }
            if !result.out_of_scope_files.is_empty() {
                div { class: "mt-3 space-y-2 rounded-lg border border-evidence-fail/40 bg-evidence-fail/10 p-3",
                    div { class: "flex items-center gap-2",
                        StatusDot { tone_class: "bg-evidence-fail", label: "scope-invalid".to_string() }
                        p { class: "text-sm font-medium text-foreground/80", "Out-of-scope changes" }
                    }
                    for file in result.out_of_scope_files {
                        p { class: "text-sm text-foreground/75", "{file}" }
                    }
                }
            }
            if !result.blockers.is_empty() {
                div { class: "mt-3 space-y-2 rounded-lg border border-evidence-fail/40 bg-evidence-fail/10 p-3",
                    for blocker in result.blockers {
                        div { class: "flex items-center gap-2",
                            StatusDot { tone_class: "bg-evidence-fail", label: "assignment-blocked".to_string() }
                            p { class: "text-sm text-foreground/80", "{blocker.code}: {blocker.message}" }
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub fn AssignmentConstraintPanel(assignment: Assignment) -> Element {
    rsx! {
        section { class: "grid gap-3 md:grid-cols-2",
            ConstraintList { title: "Allowed files".to_string(), token: "scope-in".to_string(), values: assignment.scope.include.clone() }
            ConstraintList { title: "Forbidden files".to_string(), token: "scope-out".to_string(), values: assignment.scope.exclude.clone() }
            ConstraintList { title: "Non-goals".to_string(), token: "assignment-ready".to_string(), values: assignment.scope.non_goals.clone() }
            ConstraintList { title: "Required tests".to_string(), token: "evidence-required".to_string(), values: assignment.scope.required_tests.clone() }
            ConstraintList { title: "Completion commands".to_string(), token: "run-dry".to_string(), values: assignment.scope.completion_commands.clone() }
            ConstraintList { title: "Linked spec context".to_string(), token: "spec-linked".to_string(), values: assignment.scope.linked_spec_context.clone() }
        }
    }
}

#[component]
pub(super) fn ConstraintList(title: String, token: String, values: Vec<String>) -> Element {
    rsx! {
        div { class: "rounded-xl border border-border bg-background/30 p-3",
            div { class: "flex items-center justify-between gap-2",
                p { class: "text-xs uppercase tracking-[0.18em] text-foreground/55", "{title}" }
                ScopeChip { label: token }
            }
            if values.is_empty() {
                p { class: "mt-2 text-sm text-evidence-warn", "evidence-missing" }
            } else {
                ul { class: "mt-2 space-y-1",
                    for value in values {
                        li { class: "text-sm text-foreground/75", "{value}" }
                    }
                }
            }
        }
    }
}

#[component]
pub fn AssignmentPromptPreview(assignment: Assignment) -> Element {
    rsx! {
        section { class: "rounded-xl border border-border bg-background/30 p-3",
            div { class: "flex flex-wrap items-center gap-2",
                p { class: "text-xs uppercase tracking-[0.18em] text-foreground/55", "Assignment Prompt Preview" }
                ScopeChip { label: assignment.run_mode.label().to_string() }
            }
            pre { class: "mt-2 max-h-56 overflow-auto rounded-lg border border-border bg-panel-muted p-3 text-xs text-foreground/70",
                "{assignment.prompt_preview}"
            }
        }
    }
}

#[component]
pub fn AgentRunPanel(run: AgentRun) -> Element {
    rsx! {
        section { class: "rounded-xl border border-border bg-background/30 p-3",
            div { class: "flex flex-wrap items-center gap-2",
                p { class: "text-xs uppercase tracking-[0.18em] text-foreground/55", "Agent Run Panel" }
                StatusDot { tone_class: agent_run_tone(run.status), label: run.status.label().to_string() }
                ScopeChip { label: run.output.diff_summary.clone() }
            }
            CommandOutputView {
                title: "Runner output".to_string(),
                summary: run.status.label().to_string(),
                command: Some(syu_workbench::EvidenceCommand {
                    command: run.profile_id.clone(),
                    args: vec![run.mode.label().to_string()],
                }),
                attachment: Some(syu_workbench::EvidenceAttachment {
                    label: "stdout-stderr".to_string(),
                    mime_type: Some("text/plain".to_string()),
                    summary: Some("stdout/stderr".to_string()),
                    content: Some(format!("stdout:\n{}\nstderr:\n{}", run.output.stdout, run.output.stderr)),
                    truncated: false,
                }),
            }
        }
    }
}

#[component]
pub fn HumanAssignmentPanel(assignment: Assignment) -> Element {
    rsx! {
        section { class: "rounded-xl border border-border bg-background/30 p-3",
            div { class: "flex flex-wrap items-center gap-2",
                p { class: "text-xs uppercase tracking-[0.18em] text-foreground/55", "Human Assignment Panel" }
                ScopeChip { label: "manual".to_string() }
                ScopeChip { label: assignment.status.label().to_string() }
            }
            p { class: "mt-2 text-sm text-foreground/75", "Human assignment uses the same scoped handoff without command execution." }
        }
    }
}

#[component]
pub fn AssignmentEvidencePanel(assignment: Assignment) -> Element {
    rsx! {
        section { class: "rounded-xl border border-border bg-background/30 p-3",
            div { class: "flex flex-wrap items-center gap-2",
                p { class: "text-xs uppercase tracking-[0.18em] text-foreground/55", "Assignment Evidence Panel" }
                EvidenceBadge { kind: syu_workbench::WorkbenchEvidenceKind::AssignmentState }
            }
            if assignment.evidence_requirements.is_empty() {
                p { class: "mt-2 text-sm text-evidence-warn", "evidence-missing" }
            } else {
                div { class: "mt-2 flex flex-wrap gap-2",
                    for requirement in assignment.evidence_requirements {
                        ScopeChip { label: if requirement.required { "evidence-required".to_string() } else { "evidence-optional".to_string() } }
                        p { class: "text-sm text-foreground/75", "{requirement.description}" }
                    }
                }
            }
        }
    }
}
