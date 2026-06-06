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
