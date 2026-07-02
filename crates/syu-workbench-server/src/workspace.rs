use super::*;

pub(super) fn validate_bind(bind: &str, allow_remote_bind: bool) -> Result<()> {
    let parsed = parse_bind_address(bind)?;
    if !allow_remote_bind && !parsed.is_loopback() {
        bail!("remote bind `{bind}` is disabled unless `--allow-remote-bind` is set");
    }
    Ok(())
}

pub(super) fn parse_bind_address(bind: &str) -> Result<IpAddr> {
    if bind.eq_ignore_ascii_case("localhost") {
        return Ok(IpAddr::from([127, 0, 0, 1]));
    }
    bind.parse::<IpAddr>()
        .with_context(|| format!("`{bind}` is not a valid IP address"))
}

pub(super) fn parse_socket_addr(bind: &str, port: u16) -> Result<SocketAddr> {
    let ip = parse_bind_address(bind)?;
    Ok(SocketAddr::new(ip, port))
}

pub(super) fn initial_state(
    workspace: &BrowserWorkspace,
    config: &WorkbenchLaunchConfig,
) -> WorkbenchState {
    WorkbenchState {
        workspace: Some(WorkspaceSnapshot {
            workspace_root: config.workspace_root.clone(),
            spec_root: config.spec_root.clone(),
            branch: current_git_branch(&config.workspace_root),
            validation_summary: Some(format!("{} items", workspace.item_index.len())),
        }),
        request: None,
        goals: Default::default(),
        branch_scope: None,
        evidence_timeline: EvidenceTimelineState::default(),
        assignment: None,
        job: JobState::default(),
        command_palette: CommandPaletteState::default(),
        confirmation: Some(WorkbenchConfirmationMetadata {
            confirmed_by: "local".to_string(),
            rationale: Some("starting a local Workbench server".to_string()),
            scope_token: None,
        }),
    }
}

pub(super) fn current_git_branch(workspace_root: &FsPath) -> Option<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(workspace_root)
        .arg("branch")
        .arg("--show-current")
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
    (!branch.is_empty()).then_some(branch)
}

pub(super) fn collect_source_documents(spec_root: &FsPath) -> Result<Vec<SourceDocument>> {
    if !spec_root.exists() {
        return Ok(Vec::new());
    }
    let mut documents = Vec::new();
    collect_yaml_documents(spec_root, spec_root, &mut documents)?;
    Ok(documents)
}

pub(super) fn collect_yaml_documents(
    spec_root: &FsPath,
    directory: &FsPath,
    documents: &mut Vec<SourceDocument>,
) -> Result<()> {
    for entry in fs::read_dir(directory)
        .with_context(|| format!("failed to read directory `{}`", directory.display()))?
    {
        let path = entry?.path();
        if path.is_dir() {
            collect_yaml_documents(spec_root, &path, documents)?;
            continue;
        }
        if !is_yaml_path(&path) {
            continue;
        }
        if path.file_name().and_then(|name| name.to_str()) == Some("features.yaml") {
            continue;
        }
        if let Some(section) = section_for_path(spec_root, &path)? {
            let content = fs::read_to_string(&path)
                .with_context(|| format!("failed to read `{}`", path.display()))?;
            let rel = path
                .strip_prefix(spec_root)
                .with_context(|| format!("failed to make `{}` relative", path.display()))?;
            documents.push(SourceDocument {
                section,
                path: rel.to_string_lossy().replace('\\', "/"),
                content,
            });
        }
    }
    Ok(())
}

pub(super) fn section_for_path(spec_root: &FsPath, path: &FsPath) -> Result<Option<SectionKind>> {
    let rel = path
        .strip_prefix(spec_root)
        .with_context(|| format!("failed to make `{}` relative", path.display()))?;
    let mut components = rel.components();
    let first = match components.next() {
        Some(Component::Normal(value)) => value.to_string_lossy().to_string(),
        _ => return Ok(None),
    };
    Ok(match first.as_str() {
        "philosophy" => Some(SectionKind::Philosophy),
        "policies" => Some(SectionKind::Policies),
        "requirements" => Some(SectionKind::Requirements),
        "features" => Some(SectionKind::Features),
        _ => None,
    })
}

pub(super) fn is_yaml_path(path: &FsPath) -> bool {
    matches!(
        path.extension().and_then(|ext| ext.to_str()),
        Some("yaml" | "yml")
    )
}

pub(super) fn load_browser_workspace(
    workspace_root: &FsPath,
    spec_root: &FsPath,
) -> Result<BrowserWorkspace> {
    let source_documents = collect_source_documents(spec_root)?;
    let payload = AppPayload {
        workspace_root: workspace_root.display().to_string(),
        spec_root: spec_root.display().to_string(),
        app_server: AppServer {
            bind: "127.0.0.1".to_string(),
            port: 3000,
            remotely_reachable: false,
        },
        source_documents,
        validation: ValidationSnapshot::default(),
        historical_ids: HistoricalIdSnapshot::default(),
    };
    Ok(build_browser_workspace(payload))
}

pub(super) async fn classify_request(
    server: &WorkbenchServer,
    request: &RequestArtifact,
) -> ClassificationOutcome {
    let plan = shared_work_plan_input(
        server,
        request,
        None,
        None,
        None,
        WorkConstraints::default(),
    )
    .await;
    let explicit_items = explicit_request_items(server, request).await;
    let related_items = search_request_items(server, request).await;
    ClassificationOutcome {
        classification: request_classification_from_work_plan(&plan),
        reasons: vec![format!(
            "shared planner resolved {:?} + {:?}",
            plan.intent.kind, plan.intent.operation
        )],
        explicit_items,
        related_items,
        request: request.request.clone(),
        context: request.context.clone(),
    }
}

pub(super) async fn scope_request(
    server: &WorkbenchServer,
    request: &RequestArtifact,
) -> ScopeOutcome {
    let plan = shared_work_plan_input(
        server,
        request,
        None,
        None,
        None,
        WorkConstraints::default(),
    )
    .await;
    let classification = classify_request(server, request).await;
    let workspace = server.inner.browser_workspace.read().await;
    let requirements = collect_scope_search_results(&workspace, &plan, WorkSurface::Requirement);
    let policies = collect_scope_search_results(&workspace, &plan, WorkSurface::Policy);
    let philosophies = collect_scope_search_results(&workspace, &plan, WorkSurface::Philosophy);
    let features = collect_scope_feature_candidates(&workspace, &plan);
    ScopeOutcome {
        classification,
        signals: ScopeSignals {
            policy_discussion: !policies.is_empty()
                || plan
                    .intent
                    .requested_surfaces
                    .contains(&WorkSurface::Policy),
            philosophy_discussion: !philosophies.is_empty()
                || plan
                    .intent
                    .requested_surfaces
                    .contains(&WorkSurface::Philosophy),
            planned_feature_updates: features.iter().any(|feature| feature.planned_state_update),
        },
        requirements,
        features,
        policies,
        philosophies,
        notes: vec![format!(
            "shared planner resolved {:?} + {:?}",
            plan.intent.kind, plan.intent.operation
        )],
    }
}

pub(super) async fn scaffold_request(
    server: &WorkbenchServer,
    request: &RequestArtifact,
) -> ScaffoldPlan {
    let classification = classify_request(server, request).await;
    let kind = if classification.classification == RequestClassification::Delete {
        ScaffoldUpdateKind::Feature
    } else {
        ScaffoldUpdateKind::Requirement
    };
    ScaffoldPlan {
        updates: vec![ScaffoldUpdate {
            kind,
            action: ScaffoldAction::Create,
            path: "docs/syu/requests/generated.yaml".to_string(),
            id: None,
            contents: request.request.clone(),
        }],
    }
}

pub(super) async fn build_shared_goal_plan(
    server: &WorkbenchServer,
    request: &RequestPlanRequest,
) -> GoalPlanArtifact {
    let request_path = request
        .request_path
        .clone()
        .unwrap_or_else(|| "request.yaml".to_string());
    let plan = shared_work_plan_with_options(server, request).await;
    let context = GoalPlanConversionContext {
        goal_id: request
            .goal_id
            .clone()
            .unwrap_or_else(|| default_goal_id_for_request(&request.request, &request_path)),
        source_mode: GoalPlanSourceMode::RequestDriven,
        source_path: Some(request_path),
        plan_output_path: request
            .plan_output_path
            .clone()
            .unwrap_or_else(|| ".syu/tasks/current.yaml".to_string()),
        range: None,
        confidence: if plan.executable {
            GoalPlanConfidence::High
        } else {
            GoalPlanConfidence::Low
        },
    };
    goal_plan_from_work_plan(&request.request.request, &plan, &context)
}

fn default_goal_id_for_request(request: &RequestArtifact, request_path: &str) -> String {
    if let Some(id) = request.explicit_ids().first() {
        return format!("GOAL-{id}");
    }
    let stem = FsPath::new(request_path)
        .file_stem()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .unwrap_or("request");
    let compact = stem
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_uppercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    format!(
        "GOAL-{}",
        if compact.is_empty() { "WORK" } else { &compact }
    )
}

async fn shared_work_plan_with_options(
    server: &WorkbenchServer,
    request: &RequestPlanRequest,
) -> WorkPlan {
    shared_work_plan_input(
        server,
        &request.request,
        request.kind,
        request.operation,
        request.mode,
        request.constraints.clone(),
    )
    .await
}

async fn shared_work_plan_input(
    server: &WorkbenchServer,
    request: &RequestArtifact,
    explicit_kind: Option<WorkKind>,
    explicit_operation: Option<WorkOperation>,
    explicit_mode: Option<WorkMode>,
    constraints: WorkConstraints,
) -> WorkPlan {
    let search_candidates = search_request_items(server, request)
        .await
        .into_iter()
        .enumerate()
        .filter_map(|(index, item)| {
            work_surface_from_label(&item.kind).map(|surface| syu_task_model::WorkCandidate {
                id: item.id,
                surface,
                score: 50u32.saturating_sub((index as u32) * 10),
                match_reasons: vec!["workbench request search candidate".to_string()],
            })
        })
        .collect::<Vec<_>>();
    let workspace = server.inner.browser_workspace.read().await;
    let mut seeds = request
        .explicit_ids()
        .into_iter()
        .map(|id| WorkSeed {
            surface: work_surface_from_id(&id),
            id,
            source_role: SourceRole::Seed,
        })
        .collect::<Vec<_>>();
    let mut diagnostics = Vec::new();
    if seeds.is_empty() && search_candidates.len() == 1 {
        let candidate = &search_candidates[0];
        seeds.push(WorkSeed {
            id: candidate.id.clone(),
            surface: candidate.surface,
            source_role: SourceRole::Inferred,
        });
    } else if seeds.is_empty() && !search_candidates.is_empty() {
        diagnostics.push(WorkDiagnostic {
            rule: "WORK_AMBIGUOUS_SEED".to_string(),
            subject: syu_task_model::work_request_text(request),
            message: "request matched multiple candidate Items without a confident unique winner"
                .to_string(),
        });
    }
    let nodes = workspace
        .sections
        .iter()
        .flat_map(|section| section.documents.iter())
        .flat_map(|document| {
            document.items.iter().map(|item| WorkGraphNode {
                id: item.id.clone(),
                surface: work_surface_from_section(item.kind),
                document_path: Some(document.path.clone()),
                status: item.status.clone(),
                linked_ids: item
                    .linked_philosophies
                    .iter()
                    .chain(&item.linked_policies)
                    .chain(&item.linked_requirements)
                    .chain(&item.linked_features)
                    .cloned()
                    .collect(),
                implementations: browser_trace_targets(&item.implementations),
                tests: browser_trace_targets(&item.tests),
            })
        })
        .collect();
    plan_work(WorkPlanningInput {
        request: syu_task_model::work_request_text(request),
        explicit_kind,
        explicit_operation,
        explicit_mode,
        seeds,
        search_candidates,
        nodes,
        constraints,
        diagnostics,
        ..Default::default()
    })
}

fn browser_trace_targets(groups: &[syu_core::BrowserTraceGroup]) -> Vec<TraceTarget> {
    groups
        .iter()
        .flat_map(|group| {
            group.references.iter().map(|reference| TraceTarget {
                language: group.language.clone(),
                file: reference.file.clone(),
                symbols: reference.symbols.clone(),
            })
        })
        .collect()
}

fn work_surface_from_section(section: SectionKind) -> WorkSurface {
    match section {
        SectionKind::Philosophy => WorkSurface::Philosophy,
        SectionKind::Policies => WorkSurface::Policy,
        SectionKind::Requirements => WorkSurface::Requirement,
        SectionKind::Features => WorkSurface::Feature,
    }
}

fn work_surface_from_label(label: &str) -> Option<WorkSurface> {
    match label {
        "philosophy" => Some(WorkSurface::Philosophy),
        "policy" => Some(WorkSurface::Policy),
        "requirement" => Some(WorkSurface::Requirement),
        "feature" => Some(WorkSurface::Feature),
        _ => None,
    }
}

fn work_surface_from_id(id: &str) -> WorkSurface {
    if id.starts_with("PHIL-") {
        WorkSurface::Philosophy
    } else if id.starts_with("POL-") {
        WorkSurface::Policy
    } else if id.starts_with("FEAT-") {
        WorkSurface::Feature
    } else {
        WorkSurface::Requirement
    }
}

fn request_classification_from_work_plan(plan: &WorkPlan) -> RequestClassification {
    match plan.intent.operation {
        WorkOperation::Create => RequestClassification::Create,
        WorkOperation::Delete | WorkOperation::Supersede => RequestClassification::Delete,
        _ => RequestClassification::Change,
    }
}

async fn explicit_request_items(
    server: &WorkbenchServer,
    request: &RequestArtifact,
) -> Vec<SearchResult> {
    let ids = request.explicit_ids();
    if ids.is_empty() {
        return Vec::new();
    }
    let workspace = server.inner.browser_workspace.read().await;
    let mut items = workspace
        .sections
        .iter()
        .flat_map(|section| {
            section.documents.iter().flat_map(|document| {
                document.items.iter().filter_map(|item| {
                    ids.iter().any(|id| id == &item.id).then(|| SearchResult {
                        id: item.id.clone(),
                        kind: section.kind.label().to_string(),
                        title: item.title.clone(),
                    })
                })
            })
        })
        .collect::<Vec<_>>();
    items.sort_by(|a, b| a.id.cmp(&b.id));
    items.dedup_by(|a, b| a.id == b.id);
    items
}

fn collect_scope_search_results(
    workspace: &BrowserWorkspace,
    plan: &WorkPlan,
    surface: WorkSurface,
) -> Vec<SearchResult> {
    let mut results = plan
        .impact
        .items
        .iter()
        .filter(|item| item.surface == surface)
        .filter_map(|item| browser_search_result(workspace, &item.id))
        .collect::<Vec<_>>();
    results.sort_by(|a, b| a.id.cmp(&b.id));
    results.dedup_by(|a, b| a.id == b.id);
    results
}

fn collect_scope_feature_candidates(
    workspace: &BrowserWorkspace,
    plan: &WorkPlan,
) -> Vec<ScopeFeatureCandidate> {
    let mut features = plan
        .impact
        .items
        .iter()
        .filter(|item| item.surface == WorkSurface::Feature)
        .filter_map(|item| {
            let (status, linked_requirements, title) =
                browser_feature_details(workspace, &item.id)?;
            Some(ScopeFeatureCandidate {
                id: item.id.clone(),
                title,
                planned_state_update: item.impact_role == syu_task_model::ImpactRole::FollowUp
                    || status.eq_ignore_ascii_case("planned"),
                status,
                linked_requirements,
            })
        })
        .collect::<Vec<_>>();
    features.sort_by(|a, b| a.id.cmp(&b.id));
    features.dedup_by(|a, b| a.id == b.id);
    features
}

fn browser_search_result(workspace: &BrowserWorkspace, id: &str) -> Option<SearchResult> {
    workspace.sections.iter().find_map(|section| {
        section.documents.iter().find_map(|document| {
            document.items.iter().find_map(|item| {
                (item.id == id).then(|| SearchResult {
                    id: item.id.clone(),
                    kind: section.kind.label().to_string(),
                    title: item.title.clone(),
                })
            })
        })
    })
}

fn browser_feature_details(
    workspace: &BrowserWorkspace,
    id: &str,
) -> Option<(String, Vec<String>, String)> {
    workspace.sections.iter().find_map(|section| {
        section.documents.iter().find_map(|document| {
            document.items.iter().find_map(|item| {
                (item.id == id).then(|| {
                    (
                        item.status
                            .clone()
                            .unwrap_or_else(|| "implemented".to_string()),
                        item.linked_requirements.clone(),
                        item.title.clone(),
                    )
                })
            })
        })
    })
}

pub(super) async fn build_branch_scope(
    workspace_root: &FsPath,
    range: &str,
) -> Result<BranchScopeReport> {
    let changed_files = resolve_git_range_changed_files(workspace_root, range)?;
    let changed_file_reports = changed_files
        .iter()
        .map(|file| ChangedFileReport {
            file: file.display().to_string(),
            symbols: Vec::new(),
            owners: Vec::new(),
            status: OwnershipStatus::Unowned,
            is_spec_file: false,
        })
        .collect::<Vec<_>>();
    Ok(BranchScopeReport::from_evidence(BranchScopeEvidence {
        range: range.to_string(),
        changed_files: changed_file_reports.clone(),
        trace_ownership: changed_file_reports,
        spec_items: Vec::new(),
        required_tests: Vec::new(),
        linked_tests: Vec::new(),
        include_patterns: Vec::new(),
        exclude_patterns: vec!["docs/generated/**".to_string(), "target/**".to_string()],
        allowed_ids: Vec::new(),
        unowned_files: Vec::new(),
        ambiguous_files: Vec::new(),
        spec_files: Vec::new(),
        direct_items: Vec::new(),
        related_items: Vec::new(),
        has_planned_features: false,
        out_of_scope_changes: Vec::new(),
    }))
}

pub(super) async fn build_goal_check(
    server: &WorkbenchServer,
    plan: &GoalPlanArtifact,
    range: &str,
) -> GoalPlanCheckReport {
    let changed_files = resolve_git_range_changed_files(&server.inner.config.workspace_root, range)
        .unwrap_or_default()
        .into_iter()
        .map(|file| file.display().to_string())
        .collect::<Vec<_>>();
    let mut issues = Vec::new();
    if plan.goal.statement.trim().is_empty() {
        issues.push(Issue::warning(
            "SYU-goal-check-001",
            "goal.statement",
            None,
            "goal statement is blank",
            Some("add a concrete statement".to_string()),
        ));
    }
    GoalPlanCheckReport {
        plan_path: plan
            .request_path
            .clone()
            .unwrap_or_else(|| "in-memory".to_string()),
        range: range.to_string(),
        changed_files,
        issues,
    }
}

pub(super) async fn search_request_items(
    server: &WorkbenchServer,
    request: &RequestArtifact,
) -> Vec<SearchResult> {
    let query = request.request.to_ascii_lowercase();
    let workspace = server.inner.browser_workspace.read().await;
    let mut results = Vec::new();
    for section in &workspace.sections {
        for document in &section.documents {
            for item in &document.items {
                if item.id.to_ascii_lowercase().contains(&query)
                    || item.title.to_ascii_lowercase().contains(&query)
                    || item
                        .summary
                        .as_ref()
                        .is_some_and(|value| value.to_ascii_lowercase().contains(&query))
                    || item
                        .description
                        .as_ref()
                        .is_some_and(|value| value.to_ascii_lowercase().contains(&query))
                {
                    results.push(SearchResult {
                        id: item.id.clone(),
                        kind: section.kind.label().to_string(),
                        title: item.title.clone(),
                    });
                }
            }
        }
    }
    results
}
