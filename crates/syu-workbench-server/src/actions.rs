use super::*;

pub(super) async fn execute_action(
    server: &WorkbenchServer,
    action_id: &str,
    body: Value,
) -> Result<ActionRunResponse> {
    let request = serde_json::from_value::<RequestArtifact>(body.clone()).ok();
    let event = match action_id {
        "request.new" => {
            let request = request.context("request artifact required")?;
            let state = ActiveRequestState {
                request_path: None,
                artifact: Some(request.clone()),
                classification: None,
                scope: None,
                scaffold: None,
            };
            {
                let mut workbench_state = server.inner.state.write().await;
                workbench_state.request = Some(state.clone());
            }
            ActionRunResponse {
                action_id: action_id.to_string(),
                event: WorkbenchEvent::RequestCreated {
                    request: request.request.clone(),
                },
                result: serde_json::to_value(state)?,
            }
        }
        "request.classify" => {
            let request = request.context("request artifact required")?;
            let outcome = classify_request(server, &request).await;
            let classification = outcome.classification;
            let request_text = outcome.request.clone();
            {
                let mut state = server.inner.state.write().await;
                let request_state = state
                    .request
                    .get_or_insert_with(ActiveRequestState::default);
                request_state.artifact = Some(request.clone());
                request_state.classification = Some(outcome.clone());
            }
            ActionRunResponse {
                action_id: action_id.to_string(),
                event: WorkbenchEvent::RequestClassified {
                    classification,
                    request: request_text,
                },
                result: serde_json::to_value(outcome)?,
            }
        }
        "request.scope" => {
            let request = request.context("request artifact required")?;
            let outcome = scope_request(server, &request).await;
            {
                let mut state = server.inner.state.write().await;
                let request_state = state
                    .request
                    .get_or_insert_with(ActiveRequestState::default);
                request_state.artifact = Some(request.clone());
                request_state.scope = Some(outcome.clone());
            }
            ActionRunResponse {
                action_id: action_id.to_string(),
                event: WorkbenchEvent::RequestScoped {
                    request: request.request,
                    requirement_count: outcome.requirements.len(),
                },
                result: serde_json::to_value(outcome)?,
            }
        }
        "request.scaffold" => {
            let request = request.context("request artifact required")?;
            let plan = scaffold_request(server, &request).await;
            {
                let mut state = server.inner.state.write().await;
                let request_state = state
                    .request
                    .get_or_insert_with(ActiveRequestState::default);
                request_state.artifact = Some(request.clone());
                request_state.scaffold = Some(plan.clone());
            }
            ActionRunResponse {
                action_id: action_id.to_string(),
                event: WorkbenchEvent::RequestScaffolded {
                    request: request.request,
                },
                result: serde_json::to_value(plan)?,
            }
        }
        "request.plan" => {
            let request = request.context("request artifact required")?;
            let plan = goal_plan_from_request(
                server,
                &RequestPlanRequest {
                    request: request.clone(),
                    request_path: None,
                },
            )
            .await;
            {
                let mut state = server.inner.state.write().await;
                let request_state = state
                    .request
                    .get_or_insert_with(ActiveRequestState::default);
                request_state.artifact = Some(request.clone());
                state.goals.selected_goal_id = Some(plan.goal.id.clone());
                state.goals.active.push(ActiveGoalState {
                    goal_id: plan.goal.id.clone(),
                    goal_plan: Some(plan.clone()),
                    ..ActiveGoalState::default()
                });
            }
            ActionRunResponse {
                action_id: action_id.to_string(),
                event: WorkbenchEvent::GoalPlanGenerated {
                    goal_id: plan.goal.id.clone(),
                },
                result: serde_json::to_value(plan)?,
            }
        }
        "branch.scope" => {
            let range = body
                .get("range")
                .and_then(Value::as_str)
                .unwrap_or("origin/main...HEAD");
            let report = build_branch_scope(&server.inner.config.workspace_root, range).await?;
            {
                let mut state = server.inner.state.write().await;
                state.branch_scope = Some(report.clone());
            }
            ActionRunResponse {
                action_id: action_id.to_string(),
                event: WorkbenchEvent::BranchScopeUpdated {
                    range: range.to_string(),
                    changed_files: report.changed_files.len(),
                },
                result: serde_json::to_value(report)?,
            }
        }
        "trace.range" | "relate.range" | "spec.impact" => {
            let range = body
                .get("range")
                .and_then(Value::as_str)
                .unwrap_or("origin/main...HEAD");
            let report = build_branch_scope(&server.inner.config.workspace_root, range).await?;
            {
                let mut state = server.inner.state.write().await;
                state.branch_scope = Some(report.clone());
                state.evidence_timeline.append(evidence_entry(
                    WorkbenchEvidenceKind::BranchScopeReport,
                    EvidenceStatus::Pass,
                    format!("{action_id} refreshed branch impact"),
                    None,
                    Some(action_id.to_string()),
                    Some(EvidenceSource::Action {
                        action_id: Some(action_id.to_string()),
                        action_label: Some(action_id.to_string()),
                    }),
                    vec![json_attachment(&report)],
                ));
            }
            ActionRunResponse {
                action_id: action_id.to_string(),
                event: WorkbenchEvent::BranchScopeUpdated {
                    range: range.to_string(),
                    changed_files: report.changed_files.len(),
                },
                result: serde_json::to_value(report)?,
            }
        }
        "branch.infer_goal" => {
            let range = body
                .get("range")
                .and_then(Value::as_str)
                .unwrap_or("origin/main...HEAD");
            let report = if let Some(report) = {
                let state = server.inner.state.read().await;
                state.branch_scope.clone()
            } {
                report
            } else {
                build_branch_scope(&server.inner.config.workspace_root, range).await?
            };
            let plan = GoalPlanArtifact {
                version: 1,
                kind: "syu.goal_plan".to_string(),
                request_path: Some(format!("branch:{range}")),
                request: Some(format!("Infer goal from {range}")),
                classification: Some(RequestClassification::Change.label().to_string()),
                goal: GoalPlanGoal {
                    id: "GOAL-BRANCH-001".to_string(),
                    title: "Infer goal from branch".to_string(),
                    statement: format!(
                        "Review {} changed files from {range}",
                        report.changed_files.len()
                    ),
                    non_goals: vec!["Do not widen scope without confirmation".to_string()],
                    inferred: true,
                },
                source: GoalPlanSource {
                    mode: GoalPlanSourceMode::DiffInferred,
                    range: Some(range.to_string()),
                    confidence: Some(GoalPlanConfidence::Medium),
                    evidence: Some(GoalPlanSourceEvidence {
                        changed_files: report
                            .changed_files
                            .iter()
                            .map(|file| file.file.clone())
                            .collect(),
                        ..GoalPlanSourceEvidence::default()
                    }),
                    ..GoalPlanSource::default()
                },
                spec_mapping: GoalPlanSpecMapping {
                    persistent_items: GoalPlanPersistentItems::default(),
                    spec_updates: Default::default(),
                    spec_updates_required: false,
                    spec_update_reasons: Vec::new(),
                },
                implementation_plan: GoalPlanImplementationPlan {
                    confidence: Some(GoalPlanConfidence::Medium),
                    scope: GoalPlanScope {
                        include: report
                            .changed_files
                            .iter()
                            .map(|file| GoalPlanScopeInclude::Pattern(file.file.clone()))
                            .collect(),
                        exclude: vec!["target/**".to_string(), "docs/generated/**".to_string()],
                    },
                    steps: vec![
                        "Inspect branch scope".to_string(),
                        "Confirm linked requirements".to_string(),
                    ],
                },
                test_plan: GoalPlanTestPlan {
                    selection_mode: GoalPlanSelectionMode::Minimal,
                    confidence: Some(GoalPlanConfidence::Medium),
                    required_tests: BTreeMap::new(),
                    suggested_tests: BTreeMap::new(),
                },
                coverage: GoalPlanCoverage {
                    mode: GoalPlanCoverageMode::ChangedLines,
                    threshold: 100,
                    include: Vec::new(),
                    exclude: Vec::new(),
                },
                completion: GoalPlanCompletion {
                    must_pass: vec!["syu validate .".to_string()],
                },
                warnings: Vec::new(),
            };
            {
                let mut state = server.inner.state.write().await;
                state.goals.selected_goal_id = Some(plan.goal.id.clone());
                state.goals.active.push(ActiveGoalState {
                    goal_id: plan.goal.id.clone(),
                    goal_plan: Some(plan.clone()),
                    ..ActiveGoalState::default()
                });
                state.evidence_timeline.append(evidence_entry(
                    WorkbenchEvidenceKind::GoalPlanArtifact,
                    EvidenceStatus::Pass,
                    format!("goal inferred from {range}"),
                    Some(plan.goal.id.clone()),
                    Some(action_id.to_string()),
                    Some(EvidenceSource::Action {
                        action_id: Some(action_id.to_string()),
                        action_label: Some(action_id.to_string()),
                    }),
                    vec![json_attachment(&plan)],
                ));
            }
            ActionRunResponse {
                action_id: action_id.to_string(),
                event: WorkbenchEvent::GoalPlanGenerated {
                    goal_id: plan.goal.id.clone(),
                },
                result: serde_json::to_value(plan)?,
            }
        }
        "goal.check" => {
            let plan = serde_json::from_value::<GoalPlanArtifact>(body.clone())
                .context("goal plan artifact required")?;
            let report = build_goal_check(server, &plan, "origin/main...HEAD").await;
            {
                let mut state = server.inner.state.write().await;
                state.goals.active_goal_mut().goal_id = plan.goal.id.clone();
                state.goals.active_goal_mut().check_report = Some(report.clone());
                let status = if report
                    .issues
                    .iter()
                    .any(|issue| issue.severity == syu_domain::Severity::Error)
                {
                    EvidenceStatus::Fail
                } else if report
                    .issues
                    .iter()
                    .any(|issue| issue.severity == syu_domain::Severity::Warning)
                {
                    EvidenceStatus::Warn
                } else {
                    EvidenceStatus::Pass
                };
                state.evidence_timeline.append(evidence_entry(
                    WorkbenchEvidenceKind::GoalPlanCheckReport,
                    status,
                    if matches!(status, EvidenceStatus::Pass) {
                        format!("goal check passed for {}", report.plan_path)
                    } else {
                        format!("goal check found {} issues", report.issues.len())
                    },
                    Some(plan.goal.id.clone()),
                    Some(action_id.to_string()),
                    Some(EvidenceSource::Action {
                        action_id: Some(action_id.to_string()),
                        action_label: Some(action_id.to_string()),
                    }),
                    vec![json_attachment(&report)],
                ));
            }
            ActionRunResponse {
                action_id: action_id.to_string(),
                event: WorkbenchEvent::GoalChecked {
                    goal_id: plan.goal.id,
                },
                result: serde_json::to_value(report)?,
            }
        }
        "goal.test_select" => {
            let plan = serde_json::from_value::<GoalPlanArtifact>(body.clone())
                .context("goal plan artifact required")?;
            let selection = TaskTestSelectionPlan {
                goal_id: plan.goal.id.clone(),
                goal_title: plan.goal.title.clone(),
                selection_mode: "minimal".to_string(),
                commands: vec![TaskTestSelectionCommand {
                    language: "rust".to_string(),
                    command: "cargo test".to_string(),
                    reason: "baseline repository validation".to_string(),
                }],
                escalation: TaskTestSelectionEscalation {
                    level: "none".to_string(),
                    reason: "request-scoped test set is sufficient".to_string(),
                },
                warnings: Vec::new(),
            };
            {
                let mut state = server.inner.state.write().await;
                state.goals.active_goal_mut().goal_id = plan.goal.id.clone();
                state.goals.active_goal_mut().test_selection = Some(selection.clone());
                state.evidence_timeline.append(evidence_entry(
                    WorkbenchEvidenceKind::TaskTestSelectionPlan,
                    EvidenceStatus::Pass,
                    format!(
                        "selected {} tests for {}",
                        selection.commands.len(),
                        selection.goal_id
                    ),
                    Some(plan.goal.id.clone()),
                    Some(action_id.to_string()),
                    Some(EvidenceSource::Action {
                        action_id: Some(action_id.to_string()),
                        action_label: Some(action_id.to_string()),
                    }),
                    vec![json_attachment(&selection)],
                ));
            }
            ActionRunResponse {
                action_id: action_id.to_string(),
                event: WorkbenchEvent::GoalTestsSelected {
                    goal_id: plan.goal.id,
                },
                result: serde_json::to_value(selection)?,
            }
        }
        "assignment.create" => {
            let assignment = serde_json::from_value::<AssignmentRequest>(body.clone())
                .context("assignment request required")?;
            let goal_id = {
                let state = server.inner.state.read().await;
                state
                    .goals
                    .selected_goal_id
                    .clone()
                    .or_else(|| state.goals.active.first().map(|goal| goal.goal_id.clone()))
                    .unwrap_or_else(|| "goal-1".to_string())
            };
            let state = AssignmentState {
                goal_id: Some(goal_id.clone()),
                assignee: Some(assignment.assignee),
                scope: Some(assignment.scope),
                expected_evidence: assignment.expected_evidence,
            };
            {
                let mut workbench_state = server.inner.state.write().await;
                workbench_state.assignment = Some(state.clone());
                workbench_state.evidence_timeline.append(evidence_entry(
                    WorkbenchEvidenceKind::AssignmentState,
                    EvidenceStatus::Pass,
                    format!("assignment created for {goal_id}"),
                    Some(goal_id.clone()),
                    Some(action_id.to_string()),
                    Some(EvidenceSource::Action {
                        action_id: Some(action_id.to_string()),
                        action_label: Some(action_id.to_string()),
                    }),
                    vec![json_attachment(&state)],
                ));
            }
            ActionRunResponse {
                action_id: action_id.to_string(),
                event: WorkbenchEvent::AssignmentCreated { goal_id },
                result: serde_json::to_value(state)?,
            }
        }
        "assignment.preview" => {
            let assignment = {
                let state = server.inner.state.read().await;
                state.assignment.clone().context("assignment required")?
            };
            ActionRunResponse {
                action_id: action_id.to_string(),
                event: WorkbenchEvent::EvidenceAdded {
                    kind: "assignment".to_string(),
                    summary: "assignment previewed".to_string(),
                },
                result: serde_json::to_value(assignment)?,
            }
        }
        "assignment.run_dry"
        | "assignment.run"
        | "assignment.cancel"
        | "assignment.record_manual"
        | "assignment.collect_evidence" => {
            let assignment = {
                let state = server.inner.state.read().await;
                state.assignment.clone().context("assignment required")?
            };
            let status = if action_id == "assignment.cancel" {
                "cancelled"
            } else if action_id == "assignment.run" || action_id == "assignment.run_dry" {
                "completed"
            } else {
                "recorded"
            };
            {
                let mut state = server.inner.state.write().await;
                state.evidence_timeline.append(evidence_entry(
                    WorkbenchEvidenceKind::AssignmentState,
                    EvidenceStatus::Pass,
                    format!("{action_id} {status}"),
                    assignment.goal_id.clone(),
                    Some(action_id.to_string()),
                    Some(EvidenceSource::Action {
                        action_id: Some(action_id.to_string()),
                        action_label: Some(action_id.to_string()),
                    }),
                    vec![json_attachment(&assignment)],
                ));
                if action_id == "assignment.cancel" {
                    state.assignment = None;
                }
            }
            ActionRunResponse {
                action_id: action_id.to_string(),
                event: WorkbenchEvent::EvidenceAdded {
                    kind: "assignment".to_string(),
                    summary: format!("{action_id} {status}"),
                },
                result: serde_json::json!({
                    "status": status,
                    "assignment": assignment,
                }),
            }
        }
        "agent.run" => {
            let job_id = "job-1".to_string();
            {
                let mut jobs = server.inner.jobs.write().await;
                jobs.insert(
                    job_id.clone(),
                    JobRecord {
                        id: job_id.clone(),
                        action_id: Some(action_id.to_string()),
                        status: "running".to_string(),
                        message: Some("queued".to_string()),
                    },
                );
            }
            server
                .inner
                .events
                .send(WorkbenchEvent::JobStarted {
                    job_id: job_id.clone(),
                })
                .ok();
            server
                .inner
                .events
                .send(WorkbenchEvent::JobOutput {
                    job_id: job_id.clone(),
                    line: "executing bounded goal scope".to_string(),
                })
                .ok();
            {
                let mut jobs = server.inner.jobs.write().await;
                if let Some(job) = jobs.get_mut(&job_id) {
                    job.status = "completed".to_string();
                    job.message = Some("completed".to_string());
                }
            }
            server
                .inner
                .events
                .send(WorkbenchEvent::JobCompleted {
                    job_id: job_id.clone(),
                })
                .ok();
            ActionRunResponse {
                action_id: action_id.to_string(),
                event: WorkbenchEvent::JobCompleted { job_id },
                result: serde_json::json!({"status": "completed"}),
            }
        }
        "history.show" => {
            let timeline = {
                let state = server.inner.state.read().await;
                state.evidence_timeline.clone()
            };
            ActionRunResponse {
                action_id: action_id.to_string(),
                event: WorkbenchEvent::EvidenceAdded {
                    kind: "history".to_string(),
                    summary: format!("{} evidence entries", timeline.entries.len()),
                },
                result: serde_json::to_value(timeline)?,
            }
        }
        "validation.run" => ActionRunResponse {
            action_id: action_id.to_string(),
            event: {
                {
                    let mut state = server.inner.state.write().await;
                    state
                        .workspace
                        .get_or_insert_with(WorkspaceSnapshot::default)
                        .validation_summary = Some("validation snapshot refreshed".to_string());
                    state.evidence_timeline.append(evidence_entry(
                        WorkbenchEvidenceKind::ValidationReport,
                        EvidenceStatus::Pass,
                        "validation snapshot refreshed",
                        None,
                        Some(action_id.to_string()),
                        Some(EvidenceSource::Command {
                            command: action_id.to_string(),
                        }),
                        vec![json_attachment(&serde_json::json!({"status": "ok"}))],
                    ));
                }
                WorkbenchEvent::ValidationUpdated {
                    summary: "validation snapshot refreshed".to_string(),
                }
            },
            result: serde_json::json!({"status": "ok"}),
        },
        other => bail!("unsupported action id `{other}`"),
    };

    server.inner.events.send(event.event.clone()).ok();
    Ok(event)
}
