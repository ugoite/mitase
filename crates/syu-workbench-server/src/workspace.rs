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
    let explicit_items = search_request_items(server, request).await;
    let classification = if request.request.to_ascii_lowercase().contains("delete")
        || request.request.to_ascii_lowercase().contains("remove")
    {
        RequestClassification::Delete
    } else if request.request.to_ascii_lowercase().contains("update")
        || request.request.to_ascii_lowercase().contains("change")
        || request.request.to_ascii_lowercase().contains("replace")
    {
        RequestClassification::Change
    } else {
        RequestClassification::Create
    };
    ClassificationOutcome {
        classification,
        reasons: vec![format!(
            "classified from request text as {classification:?}"
        )],
        explicit_items: explicit_items.clone(),
        related_items: explicit_items,
        request: request.request.clone(),
        context: request.context.clone(),
    }
}

pub(super) async fn scope_request(
    server: &WorkbenchServer,
    request: &RequestArtifact,
) -> ScopeOutcome {
    let classification = classify_request(server, request).await;
    let items = search_request_items(server, request).await;
    let requirements = items
        .iter()
        .filter(|item| item.kind == "requirement")
        .cloned()
        .collect::<Vec<_>>();
    let features = items
        .iter()
        .filter(|item| item.kind == "feature")
        .map(|item| ScopeFeatureCandidate {
            id: item.id.clone(),
            title: item.title.clone(),
            status: "implemented".to_string(),
            linked_requirements: request.context.linked_ids.clone(),
            planned_state_update: false,
        })
        .collect::<Vec<_>>();
    let policies = items
        .iter()
        .filter(|item| item.kind == "policy")
        .cloned()
        .collect::<Vec<_>>();
    let philosophies = items
        .iter()
        .filter(|item| item.kind == "philosophy")
        .cloned()
        .collect::<Vec<_>>();
    ScopeOutcome {
        classification,
        signals: ScopeSignals {
            policy_discussion: !policies.is_empty(),
            philosophy_discussion: !philosophies.is_empty(),
            planned_feature_updates: !features.is_empty(),
        },
        requirements,
        features,
        policies,
        philosophies,
        notes: vec!["derived from request text".to_string()],
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

pub(super) async fn goal_plan_from_request(
    server: &WorkbenchServer,
    request: &RequestPlanRequest,
) -> GoalPlanArtifact {
    let classification = classify_request(server, &request.request).await;
    let scope = scope_request(server, &request.request).await;
    let explicit_ids = request.request.explicit_ids();
    let request_path = request
        .request_path
        .clone()
        .unwrap_or_else(|| "request.yaml".to_string());
    GoalPlanArtifact {
        version: 1,
        kind: "syu.goal_plan".to_string(),
        request_path: Some(request_path.clone()),
        request: Some(request.request.request.clone()),
        classification: Some(classification.classification.label().to_string()),
        source: GoalPlanSource {
            mode: GoalPlanSourceMode::RequestDriven,
            request_artifact: Some(request_path),
            classification: Some(classification.classification.label().to_string()),
            range: None,
            confidence: Some(GoalPlanConfidence::Medium),
            evidence: Some(GoalPlanSourceEvidence {
                item_id: None,
                changed_files: Vec::new(),
                traced_requirements: explicit_ids.clone(),
                traced_features: Vec::new(),
                traced_policies: Vec::new(),
                traced_philosophies: Vec::new(),
            }),
        },
        goal: GoalPlanGoal {
            id: "GOAL-001".to_string(),
            title: "Plan the requested Workbench change".to_string(),
            statement: request.request.request.clone(),
            non_goals: vec!["Do not create a persistent spec layer".to_string()],
            inferred: false,
        },
        spec_mapping: GoalPlanSpecMapping {
            persistent_items: GoalPlanPersistentItems {
                requirements: scope
                    .requirements
                    .iter()
                    .map(|item| {
                        GoalPlanPersistentItem::Item(GoalPlanPersistentItemDetails {
                            id: item.id.clone(),
                            title: Some(item.title.clone()),
                            document_path: None,
                        })
                    })
                    .collect(),
                features: scope
                    .features
                    .iter()
                    .map(|item| {
                        GoalPlanPersistentItem::Item(GoalPlanPersistentItemDetails {
                            id: item.id.clone(),
                            title: Some(item.title.clone()),
                            document_path: None,
                        })
                    })
                    .collect(),
                policies: scope
                    .policies
                    .iter()
                    .map(|item| {
                        GoalPlanPersistentItem::Item(GoalPlanPersistentItemDetails {
                            id: item.id.clone(),
                            title: Some(item.title.clone()),
                            document_path: None,
                        })
                    })
                    .collect(),
                philosophies: scope
                    .philosophies
                    .iter()
                    .map(|item| {
                        GoalPlanPersistentItem::Item(GoalPlanPersistentItemDetails {
                            id: item.id.clone(),
                            title: Some(item.title.clone()),
                            document_path: None,
                        })
                    })
                    .collect(),
            },
            spec_updates: Default::default(),
            spec_updates_required: false,
            spec_update_reasons: Vec::new(),
        },
        implementation_plan: GoalPlanImplementationPlan {
            confidence: Some(GoalPlanConfidence::Medium),
            scope: GoalPlanScope {
                include: vec![GoalPlanScopeInclude::Pattern("src/**".to_string())],
                exclude: vec!["docs/generated/**".to_string()],
            },
            steps: vec![
                "Review the request".to_string(),
                "Update the smallest typed surface".to_string(),
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
    }
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
