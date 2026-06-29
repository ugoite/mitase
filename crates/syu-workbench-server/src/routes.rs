use super::*;

pub(super) async fn health(State(server): State<WorkbenchServer>) -> Json<WorkbenchHealth> {
    Json(WorkbenchHealth {
        ok: true,
        workspace_root: server.inner.config.workspace_root.display().to_string(),
        spec_root: server.inner.config.spec_root.display().to_string(),
        bind: server.inner.config.bind.clone(),
        port: server.inner.config.port,
    })
}

pub(super) async fn workspace_snapshot(
    State(server): State<WorkbenchServer>,
) -> Json<WorkbenchApiPayload> {
    Json(current_payload(&server).await)
}

pub(super) async fn list_actions(
    State(server): State<WorkbenchServer>,
) -> Json<WorkbenchActionCatalog> {
    let payload = current_payload(&server).await;
    Json(WorkbenchActionCatalog {
        actions: payload.actions,
        availability: payload.availability,
    })
}

pub(super) async fn run_action(
    State(server): State<WorkbenchServer>,
    Path(action_id): Path<String>,
    Json(body): Json<Value>,
) -> Result<Json<ActionRunResponse>, axum::http::StatusCode> {
    let response = execute_action(&server, &action_id, body)
        .await
        .map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;
    Ok(Json(response))
}

pub(super) async fn run_diagnostics(State(server): State<WorkbenchServer>) -> Json<JobRecord> {
    let job_id = format!("diagnostics-{}", evidence_timestamp());
    let record = JobRecord {
        id: job_id.clone(),
        action_id: Some("diagnostics.run".to_string()),
        status: "queued".to_string(),
        message: Some("all diagnostic checks queued".to_string()),
    };
    server
        .inner
        .jobs
        .write()
        .await
        .insert(job_id.clone(), record.clone());
    {
        let mut state = server.inner.state.write().await;
        state.job = JobState {
            status: JobStatus::Queued,
            action_id: Some("diagnostics.run".to_string()),
            message: record.message.clone(),
        };
    }
    server
        .inner
        .events
        .send(WorkbenchEvent::JobStarted {
            job_id: job_id.clone(),
        })
        .ok();
    let worker = server.clone();
    let worker_id = job_id.clone();
    task::spawn(async move {
        {
            if let Some(job) = worker.inner.jobs.write().await.get_mut(&worker_id) {
                job.status = "running".to_string();
                job.message = Some("workspace and Goal Plan checks are running".to_string());
            }
            worker.inner.state.write().await.job.status = JobStatus::Running;
        }
        worker
            .inner
            .events
            .send(WorkbenchEvent::JobOutput {
                job_id: worker_id.clone(),
                line: "workspace validation running".to_string(),
            })
            .ok();
        let validation = execute_action(&worker, "validation.run", serde_json::json!({})).await;
        let has_goal = worker.inner.state.read().await.goals.has_active_goal_plan();
        let goal = if has_goal {
            execute_action(&worker, "goal.check", serde_json::json!({}))
                .await
                .ok()
        } else {
            None
        };
        let failed = validation.is_err();
        {
            if let Some(job) = worker.inner.jobs.write().await.get_mut(&worker_id) {
                job.status = if failed { "failed" } else { "completed" }.to_string();
                job.message = Some(
                    if failed {
                        "diagnostics failed"
                    } else if goal.is_some() {
                        "workspace and Goal Plan checks completed"
                    } else {
                        "workspace checks completed; Goal Plan check disabled"
                    }
                    .to_string(),
                );
            }
            let mut state = worker.inner.state.write().await;
            state.job.status = if failed {
                JobStatus::Failed
            } else {
                JobStatus::Completed
            };
            state.job.message = Some(
                if failed {
                    "diagnostics failed"
                } else {
                    "diagnostics completed"
                }
                .to_string(),
            );
        }
        worker
            .inner
            .events
            .send(WorkbenchEvent::JobCompleted { job_id: worker_id })
            .ok();
    });
    Json(record)
}

pub(super) async fn settings_preview(
    State(server): State<WorkbenchServer>,
    Json(update): Json<SettingsUpdate>,
) -> Result<Json<SettingsPreview>, StatusCode> {
    preview_or_apply_settings(&server, update, false)
        .map(Json)
        .map_err(|_| StatusCode::BAD_REQUEST)
}

pub(super) async fn settings_apply(
    State(server): State<WorkbenchServer>,
    headers: HeaderMap,
    Json(update): Json<SettingsUpdate>,
) -> Result<Json<SettingsPreview>, StatusCode> {
    validate_same_origin(&headers)?;
    let preview =
        preview_or_apply_settings(&server, update, true).map_err(|_| StatusCode::BAD_REQUEST)?;
    server
        .inner
        .events
        .send(WorkbenchEvent::WorkspaceReloaded {
            workspace_root: server.inner.config.workspace_root.display().to_string(),
            spec_root: preview.update.spec_root.clone(),
            item_count: server.inner.browser_workspace.read().await.item_index.len(),
        })
        .ok();
    Ok(Json(preview))
}

pub(super) async fn spec_graph(State(server): State<WorkbenchServer>) -> Json<BrowserWorkspace> {
    Json(server.inner.browser_workspace.read().await.clone())
}

pub(super) async fn spec_item(
    State(server): State<WorkbenchServer>,
    Path(id): Path<String>,
) -> Result<Json<SpecItemResponse>, axum::http::StatusCode> {
    let workspace = server.inner.browser_workspace.read().await;
    let item = workspace
        .sections
        .iter()
        .flat_map(|section| section.documents.iter())
        .flat_map(|document| {
            document
                .items
                .iter()
                .cloned()
                .map(move |item| (document.path.clone(), item))
        })
        .find(|(_, item)| item.id == id)
        .ok_or(axum::http::StatusCode::NOT_FOUND)?;
    let (document_path, item) = item;
    let section = workspace
        .item_index
        .get(&id)
        .map(|entry| entry.kind)
        .ok_or(axum::http::StatusCode::NOT_FOUND)?;
    Ok(Json(SpecItemResponse {
        section,
        document_path,
        item,
    }))
}

pub(super) async fn item_work(
    State(server): State<WorkbenchServer>,
    Path(id): Path<String>,
) -> Result<Json<GoalPlanArtifact>, StatusCode> {
    if let Some(existing) = {
        let state = server.inner.state.read().await;
        state.goals.active.iter().find_map(|goal| {
            goal.goal_plan.as_ref().and_then(|plan| {
                (plan
                    .source
                    .evidence
                    .as_ref()
                    .and_then(|evidence| evidence.item_id.as_deref())
                    == Some(id.as_str()))
                .then(|| plan.clone())
            })
        })
    } {
        server.inner.state.write().await.goals.selected_goal_id = Some(existing.goal.id.clone());
        return Ok(Json(existing));
    }
    let item = {
        let workspace = server.inner.browser_workspace.read().await;
        workspace
            .sections
            .iter()
            .flat_map(|section| section.documents.iter())
            .flat_map(|document| document.items.iter())
            .find(|item| item.id == id)
            .cloned()
            .ok_or(StatusCode::NOT_FOUND)?
    };
    let mut persistent_items = GoalPlanPersistentItems::default();
    let persistent = GoalPlanPersistentItem::Id(item.id.clone());
    match item.kind {
        SectionKind::Philosophy => persistent_items.philosophies.push(persistent),
        SectionKind::Policies => persistent_items.policies.push(persistent),
        SectionKind::Requirements => persistent_items.requirements.push(persistent),
        SectionKind::Features => persistent_items.features.push(persistent),
    }
    let include = item
        .implementations
        .iter()
        .flat_map(|group| group.references.iter())
        .map(|reference| {
            GoalPlanScopeInclude::Entry(syu_task_model::GoalPlanScopeIncludeDetails {
                file: reference.file.clone(),
                symbols: reference.symbols.clone(),
            })
        })
        .collect::<Vec<_>>();
    let goal_id = format!("GOAL-{}", item.id);
    let plan = GoalPlanArtifact {
        version: 1,
        kind: "syu.goal_plan".to_string(),
        request_path: None,
        request: Some(format!("Create Work from {}", item.id)),
        classification: Some(RequestClassification::Change.label().to_string()),
        source: GoalPlanSource {
            mode: GoalPlanSourceMode::ItemDriven,
            confidence: Some(GoalPlanConfidence::High),
            evidence: Some(GoalPlanSourceEvidence {
                item_id: Some(item.id.clone()),
                ..GoalPlanSourceEvidence::default()
            }),
            ..GoalPlanSource::default()
        },
        goal: GoalPlanGoal {
            id: goal_id.clone(),
            title: item.title.clone(),
            statement: item
                .summary
                .clone()
                .or(item.description.clone())
                .unwrap_or_else(|| format!("Implement {}", item.title)),
            non_goals: vec!["Do not change unrelated specification Items.".to_string()],
            inferred: false,
        },
        spec_mapping: GoalPlanSpecMapping {
            persistent_items,
            ..GoalPlanSpecMapping::default()
        },
        implementation_plan: GoalPlanImplementationPlan {
            confidence: Some(GoalPlanConfidence::High),
            scope: GoalPlanScope {
                include,
                exclude: vec!["target/**".to_string(), "docs/generated/**".to_string()],
            },
            steps: vec![
                "Confirm the Item acceptance criteria and linked code.".to_string(),
                "Implement the bounded change and collect evidence.".to_string(),
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
            must_pass: vec!["cargo test".to_string(), "syu validate .".to_string()],
        },
        warnings: if item.implementations.is_empty() {
            vec!["No implementation trace is linked; confirm scope before assignment.".to_string()]
        } else {
            Vec::new()
        },
    };
    {
        let mut state = server.inner.state.write().await;
        state.goals.selected_goal_id = Some(goal_id.clone());
        state.goals.active.push(ActiveGoalState {
            goal_id: goal_id.clone(),
            goal_plan: Some(plan.clone()),
            ..ActiveGoalState::default()
        });
        state.evidence_timeline.append(evidence_entry(
            WorkbenchEvidenceKind::GoalPlanArtifact,
            EvidenceStatus::Pass,
            format!("Item-driven Work created from {}", item.id),
            Some(goal_id),
            Some("item.work".to_string()),
            Some(EvidenceSource::System {
                component: "items".to_string(),
            }),
            vec![json_attachment(&plan)],
        ));
    }
    server
        .inner
        .events
        .send(WorkbenchEvent::GoalPlanGenerated {
            goal_id: plan.goal.id.clone(),
        })
        .ok();
    Ok(Json(plan))
}

#[derive(Debug, Deserialize)]
pub(super) struct BranchScopeQuery {
    range: String,
}

pub(super) async fn branch_scope(
    State(server): State<WorkbenchServer>,
    Query(query): Query<BranchScopeQuery>,
) -> Result<Json<BranchScopeReport>, axum::http::StatusCode> {
    let report = build_branch_scope(&server.inner.config.workspace_root, &query.range)
        .await
        .map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;
    {
        let mut state = server.inner.state.write().await;
        state.branch_scope = Some(report.clone());
    }
    server
        .inner
        .events
        .send(WorkbenchEvent::BranchScopeUpdated {
            range: query.range,
            changed_files: report.changed_files.len(),
        })
        .ok();
    Ok(Json(report))
}

pub(super) async fn request_classify(
    State(server): State<WorkbenchServer>,
    Json(request): Json<RequestArtifact>,
) -> Json<ClassificationOutcome> {
    let outcome = classify_request(&server, &request).await;
    server
        .inner
        .events
        .send(WorkbenchEvent::RequestClassified {
            classification: outcome.classification,
            request: outcome.request.clone(),
        })
        .ok();
    Json(outcome)
}

pub(super) async fn request_scope(
    State(server): State<WorkbenchServer>,
    Json(request): Json<RequestArtifact>,
) -> Json<ScopeOutcome> {
    let outcome = scope_request(&server, &request).await;
    server
        .inner
        .events
        .send(WorkbenchEvent::RequestScoped {
            request: request.request,
            requirement_count: outcome.requirements.len(),
        })
        .ok();
    Json(outcome)
}

pub(super) async fn request_scaffold(
    State(server): State<WorkbenchServer>,
    Json(request): Json<RequestArtifact>,
) -> Json<ScaffoldPlan> {
    let plan = scaffold_request(&server, &request).await;
    server
        .inner
        .events
        .send(WorkbenchEvent::RequestScaffolded {
            request: request.request,
        })
        .ok();
    Json(plan)
}

pub(super) async fn request_plan(
    State(server): State<WorkbenchServer>,
    Json(request): Json<RequestPlanRequest>,
) -> Json<GoalPlanArtifact> {
    let plan = goal_plan_from_request(&server, &request).await;
    server
        .inner
        .events
        .send(WorkbenchEvent::GoalPlanGenerated {
            goal_id: plan.goal.id.clone(),
        })
        .ok();
    Json(plan)
}

pub(super) async fn list_goals(
    State(server): State<WorkbenchServer>,
) -> Json<Vec<ActiveGoalState>> {
    Json(server.inner.state.read().await.goals.active.clone())
}

pub(super) async fn goal_by_id(
    State(server): State<WorkbenchServer>,
    Path(id): Path<String>,
) -> Result<Json<ActiveGoalState>, axum::http::StatusCode> {
    server
        .inner
        .state
        .read()
        .await
        .goals
        .active
        .iter()
        .find(|goal| goal.goal_id == id)
        .cloned()
        .map(Json)
        .ok_or(axum::http::StatusCode::NOT_FOUND)
}

pub(super) async fn goal_test_select(
    State(server): State<WorkbenchServer>,
    Path(id): Path<String>,
    Json(plan): Json<GoalPlanArtifact>,
) -> Json<TaskTestSelectionPlan> {
    let selection = TaskTestSelectionPlan {
        goal_id: id.clone(),
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
        state.goals.active_goal_mut().goal_id = id.clone();
        state.goals.active_goal_mut().test_selection = Some(selection.clone());
        state.evidence_timeline.append(evidence_entry(
            WorkbenchEvidenceKind::TaskTestSelectionPlan,
            EvidenceStatus::Pass,
            format!(
                "selected {} tests for {}",
                selection.commands.len(),
                selection.goal_id
            ),
            Some(id.clone()),
            Some("goal.test_select".to_string()),
            Some(EvidenceSource::Action {
                action_id: Some("goal.test_select".to_string()),
                action_label: Some("goal.test_select".to_string()),
            }),
            vec![json_attachment(&selection)],
        ));
    }
    server
        .inner
        .events
        .send(WorkbenchEvent::GoalTestsSelected { goal_id: id })
        .ok();
    Json(selection)
}

pub(super) async fn goal_check(
    State(server): State<WorkbenchServer>,
    Path(id): Path<String>,
    Json(request): Json<GoalCheckRequest>,
) -> Json<GoalPlanCheckReport> {
    let range = request
        .range
        .unwrap_or_else(|| "origin/main...HEAD".to_string());
    let report = build_goal_check(&server, &request.plan, &range).await;
    {
        let mut state = server.inner.state.write().await;
        state.goals.active_goal_mut().goal_id = id.clone();
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
            Some(id.clone()),
            Some("goal.check".to_string()),
            Some(EvidenceSource::Action {
                action_id: Some("goal.check".to_string()),
                action_label: Some("goal.check".to_string()),
            }),
            vec![json_attachment(&report)],
        ));
    }
    server
        .inner
        .events
        .send(WorkbenchEvent::GoalChecked { goal_id: id })
        .ok();
    Json(report)
}

pub(super) async fn goal_assign(
    State(server): State<WorkbenchServer>,
    Path(id): Path<String>,
    Json(request): Json<AssignmentRequest>,
) -> Json<AssignmentState> {
    let assignment = AssignmentState {
        goal_id: Some(id.clone()),
        assignee: Some(request.assignee.clone()),
        scope: Some(request.scope.clone()),
        expected_evidence: request.expected_evidence.clone(),
    };
    {
        let mut state = server.inner.state.write().await;
        state.assignment = Some(assignment.clone());
    }
    server
        .inner
        .events
        .send(WorkbenchEvent::AssignmentCreated { goal_id: id })
        .ok();
    Json(assignment)
}

pub(super) async fn list_evidence(
    State(server): State<WorkbenchServer>,
) -> Json<EvidenceTimelineState> {
    Json(server.inner.state.read().await.evidence_timeline.clone())
}

pub(super) async fn list_jobs(State(server): State<WorkbenchServer>) -> Json<Vec<JobRecord>> {
    Json(server.inner.jobs.read().await.values().cloned().collect())
}

pub(super) async fn job_by_id(
    State(server): State<WorkbenchServer>,
    Path(id): Path<String>,
) -> Result<Json<JobRecord>, axum::http::StatusCode> {
    server
        .inner
        .jobs
        .read()
        .await
        .get(&id)
        .cloned()
        .map(Json)
        .ok_or(axum::http::StatusCode::NOT_FOUND)
}

pub(super) async fn cancel_job(
    State(server): State<WorkbenchServer>,
    Path(id): Path<String>,
) -> Result<Json<JobRecord>, axum::http::StatusCode> {
    let mut jobs = server.inner.jobs.write().await;
    let job = jobs.get_mut(&id).ok_or(axum::http::StatusCode::NOT_FOUND)?;
    job.status = "cancelled".to_string();
    job.message = Some("cancelled by user".to_string());
    server
        .inner
        .events
        .send(WorkbenchEvent::JobCancelled { job_id: id.clone() })
        .ok();
    Ok(Json(job.clone()))
}

pub(super) async fn events(
    State(server): State<WorkbenchServer>,
) -> Sse<impl futures_util::Stream<Item = std::result::Result<Event, Infallible>> + Send + 'static>
{
    let current = current_event_snapshot(&server).await;
    let initial =
        futures_util::stream::once(async move { Ok(event_to_sse("workspace_reloaded", &current)) });
    let stream =
        BroadcastStream::new(server.inner.events.subscribe()).filter_map(|message| async move {
            match message {
                Ok(event) => Some(Ok(event_to_sse(event_name(&event), &event))),
                Err(_) => None,
            }
        });
    let combined = initial.chain(stream);
    Sse::new(combined).keep_alive(KeepAlive::new().interval(Duration::from_secs(15)))
}

pub(super) async fn current_payload(server: &WorkbenchServer) -> WorkbenchApiPayload {
    let state = server.inner.state.read().await.clone();
    WorkbenchApiPayload::new(state)
}

pub(super) async fn current_event_snapshot(server: &WorkbenchServer) -> WorkbenchEvent {
    let workspace = server.inner.browser_workspace.read().await;
    WorkbenchEvent::WorkspaceReloaded {
        workspace_root: workspace.workspace_root.clone(),
        spec_root: workspace.spec_root.clone(),
        item_count: workspace.item_index.len(),
    }
}

pub(super) fn event_name(event: &WorkbenchEvent) -> &'static str {
    match event {
        WorkbenchEvent::WorkspaceReloaded { .. } => "workspace_reloaded",
        WorkbenchEvent::ValidationUpdated { .. } => "validation_updated",
        WorkbenchEvent::RequestCreated { .. } => "request_created",
        WorkbenchEvent::RequestClassified { .. } => "request_classified",
        WorkbenchEvent::RequestScoped { .. } => "request_scoped",
        WorkbenchEvent::RequestScaffolded { .. } => "request_scaffolded",
        WorkbenchEvent::GoalPlanGenerated { .. } => "goal_plan_generated",
        WorkbenchEvent::GoalTestsSelected { .. } => "goal_tests_selected",
        WorkbenchEvent::GoalChecked { .. } => "goal_checked",
        WorkbenchEvent::BranchScopeUpdated { .. } => "branch_scope_updated",
        WorkbenchEvent::EvidenceAdded { .. } => "evidence_added",
        WorkbenchEvent::AssignmentCreated { .. } => "assignment_created",
        WorkbenchEvent::JobStarted { .. } => "job_started",
        WorkbenchEvent::JobOutput { .. } => "job_output",
        WorkbenchEvent::JobCompleted { .. } => "job_completed",
        WorkbenchEvent::JobCancelled { .. } => "job_cancelled",
    }
}

pub(super) fn event_to_sse(name: &str, value: &impl Serialize) -> Event {
    let payload = serde_json::to_string(value).expect("event should serialize");
    Event::default().event(name).data(payload)
}
