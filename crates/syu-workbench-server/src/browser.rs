use super::*;

pub(super) async fn workbench_index(
    State(server): State<WorkbenchServer>,
    Query(view): Query<WorkbenchViewQuery>,
) -> Html<String> {
    let state = server.inner.state.read().await.clone();
    let mut ui = WorkbenchUiState::from_state(shared_workbench_state(state));
    let browser_workspace = server.inner.browser_workspace.read().await;
    ui.spec_browser = Some(shared_spec_browser_model(
        &browser_workspace,
        view.spec_item.as_deref(),
    ));
    drop(browser_workspace);
    if let Some(query) = view.query {
        ui.set_query(query);
    }
    if let Some(locale) = view.lang.as_deref().and_then(Locale::from_slug) {
        ui.set_locale(locale);
    }
    if let Some(help_topic) = view.help.as_deref().and_then(HelpTopic::from_slug) {
        ui.set_help_topic(Some(help_topic));
    }
    if let Some(action) = view.action.and_then(shared_action_id) {
        let _ = ui.select_action(action);
        if view.run.as_deref() == Some("1")
            && let Some(preview) = run_workbench_action_preview(
                &server,
                action.label(),
                view.action_input.as_deref(),
                view.action_confirm.as_deref() == Some("1"),
            )
            .await
        {
            ui.preview = Some(preview);
        }
    }
    if let Some(command_id) = view.cli {
        let _ = ui.select_cli_command(command_id.clone());
        if view.run.as_deref() == Some("1")
            && let Some(preview) = run_cli_command_preview(
                &command_id,
                server.inner.config.workspace_root.as_path(),
                view.cli_arg.as_deref(),
                view.cli_confirm.as_deref() == Some("1"),
                server.inner.config.show_log || view.show_log.as_deref() == Some("1"),
            )
        {
            ui.cli_preview = Some(preview);
        }
    }
    if let Some(goal_id) = view.goal {
        ui.payload.state.goals.selected_goal_id = Some(goal_id);
    }
    let active_pane = view
        .pane
        .as_deref()
        .and_then(WorkbenchPane::from_slug)
        .unwrap_or(WorkbenchPane::Pulse);
    let sidebar_open = view
        .sidebar
        .as_deref()
        .map(|value| value != "0" && value != "false")
        .unwrap_or(true);
    let locale = ui.locale;
    let shell = render_element(rsx! {
        AppShell { ui, active_pane, sidebar_open }
    });
    Html(workbench_document(shell, locale))
}

pub(super) fn workbench_document(shell: String, locale: Locale) -> String {
    format!(
        r#"<!doctype html>
<html lang="{lang}">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <meta name="color-scheme" content="light">
  <meta name="referrer" content="same-origin">
  <title>Syu Workbench</title>
  <style>
    form:focus-within .command-palette-results {{ display: grid; }}
    form .command-palette-results:empty {{ display: none; }}
    summary::-webkit-details-marker {{ display: none; }}
  </style>
  <link rel="stylesheet" href="/assets/tailwind.css?v=workbench-palette-commands">
</head>
<body class="bg-background text-foreground antialiased">
  <div id="syu-workbench-root" data-ui="dioxus-ssr">{shell}</div>
  <script>
    (() => {{
      const palettes = document.querySelectorAll('[data-command-palette]');
      for (const palette of palettes) {{
        const input = palette.querySelector('[data-command-input]');
        const items = Array.from(palette.querySelectorAll('[data-command-item]'));
        if (!input || items.length === 0) continue;
        const applyFilter = () => {{
          const query = input.value.trim().toLowerCase();
          const scored = [];
          let visible = 0;
          for (const item of items) {{
            const text = (item.dataset.commandText || item.textContent || '').toLowerCase();
            const id = (item.dataset.commandId || '').toLowerCase();
            const title = (item.dataset.commandTitle || '').toLowerCase();
            const match = query === '' || text.includes(query);
            item.hidden = !match;
            if (match) {{
              const score = query === '' ? 3 : id.includes(query) ? 0 : title.includes(query) ? 1 : 2;
              scored.push([score, item]);
              visible += 1;
            }}
          }}
          scored.sort((left, right) => left[0] - right[0]);
          for (const [, item] of scored) item.parentElement.appendChild(item);
          palette.dataset.empty = visible === 0 ? 'true' : 'false';
        }};
        input.addEventListener('input', applyFilter);
        applyFilter();
      }}
    }})();
  </script>
</body>
</html>"#,
        shell = shell,
        lang = locale.slug()
    )
}

pub(super) async fn run_workbench_action_preview(
    server: &WorkbenchServer,
    action_id: &str,
    action_input: Option<&str>,
    confirmed: bool,
) -> Option<WorkbenchActionRunPreview> {
    let action_input = action_input.unwrap_or("").trim();
    if workbench_action_needs_confirmation(action_id) && !confirmed {
        return shared_action_id(action_id.to_string()).map(|action| WorkbenchActionRunPreview {
            action_id: action,
            title: action_id.replace('.', " "),
            result_summary:
                "This command can change Workbench state or files. Confirm before running."
                    .to_string(),
            evidence_summary: "confirmation required".to_string(),
        });
    }
    let body = default_workbench_action_body(server, action_id, action_input).await;
    let missing_input = body.is_none();
    let body = body.unwrap_or_else(|| serde_json::json!({}));
    if missing_input {
        return shared_action_id(action_id.to_string()).map(|action| WorkbenchActionRunPreview {
            action_id: action,
            title: action_id.replace('.', " "),
            result_summary: "This command needs request, goal, assignment, or confirmation input before it can run.".to_string(),
            evidence_summary: "input required".to_string(),
        });
    }

    let action = shared_action_id(action_id.to_string())?;
    let response = execute_action(server, action_id, body).await;
    let (result_summary, evidence_summary) = match response {
        Ok(response) => (
            truncate_cli_output(
                &serde_json::to_string_pretty(&response.result).unwrap_or_default(),
            ),
            format!("{:?}", response.event),
        ),
        Err(error) => (
            format!("failed to run {action_id}: {error}"),
            "failed".to_string(),
        ),
    };
    Some(WorkbenchActionRunPreview {
        action_id: action,
        title: action_id.replace('.', " "),
        result_summary,
        evidence_summary,
    })
}

pub(super) async fn default_workbench_action_body(
    server: &WorkbenchServer,
    action_id: &str,
    action_input: &str,
) -> Option<Value> {
    let state = server.inner.state.read().await;
    match action_id {
        "request.new" => {
            if action_input.is_empty() {
                return None;
            }
            serde_json::to_value(RequestArtifact {
                version: 1,
                request: action_input.to_string(),
                context: Default::default(),
            })
            .ok()
        }
        "branch.scope" | "branch.infer_goal" | "trace.range" | "relate.range" | "spec.impact" => {
            Some(serde_json::json!({"range": "origin/main...HEAD"}))
        }
        "validation.run" | "history.show" => Some(serde_json::json!({})),
        "request.classify" | "request.scope" | "request.scaffold" | "request.plan" => state
            .request
            .as_ref()
            .and_then(|request| request.artifact.clone())
            .or_else(|| {
                (!action_input.is_empty()).then(|| RequestArtifact {
                    version: 1,
                    request: action_input.to_string(),
                    context: Default::default(),
                })
            })
            .and_then(|request| serde_json::to_value(request).ok()),
        "goal.check" | "goal.test_select" => state
            .goals
            .active
            .iter()
            .find_map(|goal| goal.goal_plan.clone())
            .and_then(|plan| serde_json::to_value(plan).ok()),
        "assignment.create" => {
            if !state.goals.has_active_goal_plan() {
                return None;
            }
            serde_json::to_value(AssignmentRequest {
                assignee: if action_input.trim().eq_ignore_ascii_case("ai") {
                    AssignmentAssignee::Ai {
                        model: "local".to_string(),
                    }
                } else {
                    AssignmentAssignee::Human {
                        name: if action_input.trim().is_empty() {
                            "Reviewer".to_string()
                        } else {
                            action_input.trim().to_string()
                        },
                    }
                },
                scope: BoundedScope {
                    range: Some("origin/main...HEAD".to_string()),
                    max_files: Some(12),
                    ..BoundedScope::default()
                },
                expected_evidence: vec![WorkbenchEvidenceKind::ValidationReport],
            })
            .ok()
        }
        "assignment.preview"
        | "assignment.run_dry"
        | "assignment.run"
        | "assignment.cancel"
        | "assignment.record_manual"
        | "assignment.collect_evidence" => state.assignment.as_ref().map(|_| serde_json::json!({})),
        "agent.run" => state.assignment.as_ref().map(|_| serde_json::json!({})),
        _ => None,
    }
}

pub(super) fn workbench_action_needs_confirmation(action_id: &str) -> bool {
    matches!(
        action_id,
        "request.new"
            | "request.scaffold"
            | "request.plan"
            | "branch.infer_goal"
            | "assignment.create"
            | "assignment.run_dry"
            | "assignment.run"
            | "assignment.cancel"
            | "assignment.record_manual"
            | "assignment.collect_evidence"
            | "agent.run"
    )
}

pub(super) fn run_cli_command_preview(
    command_id: &str,
    workspace_root: &FsPath,
    cli_arg: Option<&str>,
    confirmed: bool,
    show_log: bool,
) -> Option<CliCommandPreview> {
    let command = cli_command_catalog()
        .iter()
        .find(|command| command.id == command_id)?;
    let cli_arg = cli_arg.unwrap_or("").trim();
    if command.requires_input && cli_arg.is_empty() {
        return Some(CliCommandPreview {
            id: command.id.to_string(),
            title: command.title.to_string(),
            invocation: command.invocation.to_string(),
            result_summary: format!("{} needs input before it can run.", command.invocation),
            evidence_summary: "input required".to_string(),
            requires_input: command.requires_input,
            mutates_files: command.mutates_files,
        });
    }
    if command.mutates_files && !confirmed {
        return Some(CliCommandPreview {
            id: command.id.to_string(),
            title: command.title.to_string(),
            invocation: command.invocation.to_string(),
            result_summary: format!(
                "{} needs confirmation before writing files.",
                command.invocation
            ),
            evidence_summary: "confirmation required".to_string(),
            requires_input: command.requires_input,
            mutates_files: command.mutates_files,
        });
    }

    let cli_arg = cli_default_arg(command.id, cli_arg);
    if let Err(error) = ensure_cli_task_fixture(command.id, workspace_root, cli_arg) {
        return Some(CliCommandPreview {
            id: command.id.to_string(),
            title: command.title.to_string(),
            invocation: command.invocation.to_string(),
            result_summary: format!("failed to prepare command input: {error}"),
            evidence_summary: "failed".to_string(),
            requires_input: command.requires_input,
            mutates_files: command.mutates_files,
        });
    }
    let args = cli_command_args(command.id, cli_arg)?;
    if matches!(command.id, "cli.workbench" | "cli.lsp") {
        return Some(CliCommandPreview {
            id: command.id.to_string(),
            title: command.title.to_string(),
            invocation: command.invocation.to_string(),
            result_summary: "Already represented by this Workbench session.".to_string(),
            evidence_summary: "running".to_string(),
            requires_input: command.requires_input,
            mutates_files: command.mutates_files,
        });
    }

    let output = Command::new(std::env::current_exe().ok()?)
        .args(&args)
        .current_dir(workspace_root)
        .output();
    let (result_summary, evidence_summary) = match output {
        Ok(output) => {
            let status = output
                .status
                .code()
                .map_or_else(|| "signal".to_string(), |code| format!("exit {code}"));
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            let body = if stdout.trim().is_empty() {
                stderr.trim().to_string()
            } else {
                stdout.trim().to_string()
            };
            let result_summary = if show_log {
                format!(
                    "{}\n\nstdout:\n{}\n\nstderr:\n{}",
                    truncate_cli_output(&body),
                    stdout.trim(),
                    stderr.trim()
                )
            } else {
                truncate_cli_output(&body)
            };
            (result_summary, status)
        }
        Err(error) => (
            format!("failed to run {}: {error}", command.invocation),
            "failed".to_string(),
        ),
    };

    Some(CliCommandPreview {
        id: command.id.to_string(),
        title: command.title.to_string(),
        invocation: if cli_arg.is_empty() {
            command.invocation.to_string()
        } else {
            format!("{} · {}", command.invocation, cli_arg)
        },
        result_summary,
        evidence_summary,
        requires_input: command.requires_input,
        mutates_files: command.mutates_files,
    })
}

pub(super) fn cli_command_args(command_id: &str, cli_arg: &str) -> Option<Vec<String>> {
    let args = match command_id {
        "cli.browse" => vec!["browse", ".", "--non-interactive"],
        "cli.list" => vec!["list"],
        "cli.audit" => vec!["audit", "."],
        "cli.doctor" => vec!["doctor", "."],
        "cli.validate" => vec!["validate", "."],
        "cli.report" => vec!["report", "."],
        "cli.templates" => vec!["templates"],
        "cli.task.infer" => vec!["task", "infer", "--range", "origin/main...HEAD"],
        "cli.workbench" | "cli.lsp" => {
            return Some(Vec::new());
        }
        "cli.show" => vec!["show", cli_arg],
        "cli.search" => vec!["search", cli_arg],
        "cli.log" => vec!["log", cli_arg],
        "cli.explain" => vec!["explain", cli_arg],
        "cli.relate" => vec!["relate", cli_arg],
        "cli.trace" => vec!["trace", cli_arg],
        "cli.completion" => vec!["completion", cli_arg],
        "cli.task.classify" => vec!["task", "classify", cli_arg],
        "cli.task.scope" => vec!["task", "scope", cli_arg],
        "cli.task.scaffold" => vec!["task", "scaffold", cli_arg],
        "cli.task.plan" => vec!["task", "plan", cli_arg],
        "cli.task.test_select" => vec!["task", "test-select", cli_arg],
        "cli.task.check" => vec!["task", "check", cli_arg, "--range", "origin/main...HEAD"],
        "cli.add" => {
            let parts = cli_arg.split_whitespace().collect::<Vec<_>>();
            if parts.len() < 2 {
                return Some(
                    vec!["add", "--help"]
                        .into_iter()
                        .map(String::from)
                        .collect(),
                );
            }
            let mut args = vec!["add".to_string()];
            args.extend(parts.into_iter().map(String::from));
            return Some(args);
        }
        "cli.init" => vec!["init", "."],
        _ => return None,
    };
    Some(args.into_iter().map(String::from).collect())
}

pub(super) fn cli_default_arg<'a>(command_id: &str, cli_arg: &'a str) -> &'a str {
    if !cli_arg.is_empty() {
        return cli_arg;
    }
    match command_id {
        "cli.task.classify" | "cli.task.scope" | "cli.task.scaffold" | "cli.task.plan" => {
            "target/syu/workbench/request.yaml"
        }
        "cli.task.test_select" | "cli.task.check" => "target/syu/workbench/goal.yaml",
        _ => cli_arg,
    }
}

pub(super) fn ensure_cli_task_fixture(
    command_id: &str,
    workspace_root: &FsPath,
    cli_arg: &str,
) -> Result<()> {
    match command_id {
        "cli.task.classify" | "cli.task.scope" | "cli.task.scaffold" | "cli.task.plan" => {
            ensure_request_fixture(workspace_root, cli_arg)
        }
        "cli.task.test_select" | "cli.task.check" => ensure_goal_fixture(workspace_root, cli_arg),
        _ => Ok(()),
    }
}

pub(super) fn ensure_request_fixture(workspace_root: &FsPath, relative_path: &str) -> Result<()> {
    let path = workspace_root.join(relative_path);
    if path.exists() {
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create `{}`", parent.display()))?;
    }
    let artifact = RequestArtifact {
        version: 1,
        request: "Make Syu Workbench command palette commands usable for beginners.".to_string(),
        context: RequestArtifactContext {
            affected_area: Some("Workbench browser UI and command palette".to_string()),
            repository_constraints: vec![
                "Keep behavior Rust/Dioxus-native.".to_string(),
                "Use current Syu repository specs as the source of truth.".to_string(),
            ],
            linked_ids: vec!["REQ-WORKBENCH-001".to_string()],
        },
    };
    let yaml = serde_yaml::to_string(&artifact).context("failed to encode request fixture")?;
    fs::write(&path, yaml).with_context(|| format!("failed to write `{}`", path.display()))
}

pub(super) fn ensure_goal_fixture(workspace_root: &FsPath, relative_path: &str) -> Result<()> {
    let path = workspace_root.join(relative_path);
    if path.exists() {
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create `{}`", parent.display()))?;
    }
    let plan = GoalPlanArtifact {
        version: 1,
        kind: "syu.goal_plan".to_string(),
        request_path: Some("target/syu/workbench/request.yaml".to_string()),
        request: Some("Make Syu Workbench command palette commands usable for beginners.".to_string()),
        classification: Some(RequestClassification::Change.label().to_string()),
        source: GoalPlanSource {
            mode: GoalPlanSourceMode::RequestDriven,
            request_artifact: Some("target/syu/workbench/request.yaml".to_string()),
            classification: Some(RequestClassification::Change.label().to_string()),
            confidence: Some(GoalPlanConfidence::Medium),
            evidence: Some(GoalPlanSourceEvidence {
                traced_requirements: vec!["REQ-WORKBENCH-001".to_string()],
                traced_features: vec!["FEAT-WORKBENCH-SHELL-001".to_string()],
                ..GoalPlanSourceEvidence::default()
            }),
            ..GoalPlanSource::default()
        },
        goal: GoalPlanGoal {
            id: "GOAL-WORKBENCH-PALETTE-001".to_string(),
            title: "Make command palette commands usable".to_string(),
            statement: "Every Workbench palette command can be opened, understood, and executed with safe defaults.".to_string(),
            non_goals: vec!["Do not reintroduce a separate browser frontend.".to_string()],
            inferred: false,
        },
        spec_mapping: GoalPlanSpecMapping {
            persistent_items: GoalPlanPersistentItems {
                requirements: vec![GoalPlanPersistentItem::Id(
                    "REQ-WORKBENCH-001".to_string(),
                )],
                features: vec![GoalPlanPersistentItem::Id(
                    "FEAT-WORKBENCH-SHELL-001".to_string(),
                )],
                ..GoalPlanPersistentItems::default()
            },
            spec_updates: Default::default(),
            spec_updates_required: false,
            spec_update_reasons: Vec::new(),
        },
        implementation_plan: GoalPlanImplementationPlan {
            confidence: Some(GoalPlanConfidence::Medium),
            scope: GoalPlanScope {
                include: vec![GoalPlanScopeInclude::Pattern("**".to_string())],
                exclude: vec!["target/**".to_string()],
            },
            steps: vec![
                "Audit command palette commands".to_string(),
                "Fix commands that cannot run from defaults".to_string(),
                "Verify desktop and mobile rendering".to_string(),
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
            exclude: vec!["target/**".to_string()],
        },
        completion: GoalPlanCompletion {
            must_pass: vec!["target/debug/syu validate .".to_string()],
        },
        warnings: Vec::new(),
    };
    let yaml = serde_yaml::to_string(&plan).context("failed to encode goal fixture")?;
    fs::write(&path, yaml).with_context(|| format!("failed to write `{}`", path.display()))
}

pub(super) fn truncate_cli_output(output: &str) -> String {
    const LIMIT: usize = 1200;
    if output.chars().count() <= LIMIT {
        return output.to_string();
    }
    let mut truncated = output.chars().take(LIMIT).collect::<String>();
    truncated.push_str("\n...");
    truncated
}

pub(super) fn shared_spec_browser_model(
    workspace: &BrowserWorkspace,
    selected_item_id: Option<&str>,
) -> SpecBrowserModel {
    let selected_item_id = selected_item_id
        .map(str::to_string)
        .or_else(|| workspace.item_index.keys().next().cloned());
    SpecBrowserModel {
        sections: workspace
            .sections
            .iter()
            .map(|section| SpecBrowserSection {
                label: section.label.clone(),
                documents: section
                    .documents
                    .iter()
                    .map(|document| SpecBrowserDocument {
                        path: document.path.clone(),
                        title: document.title.clone(),
                        folder_segments: document.folder_segments.clone(),
                        items: document
                            .items
                            .iter()
                            .map(shared_spec_browser_item)
                            .collect(),
                    })
                    .collect(),
            })
            .collect(),
        selected_item_id,
    }
}

pub(super) fn shared_spec_browser_item(item: &BrowserItem) -> SpecBrowserItem {
    SpecBrowserItem {
        kind: item.kind.label().to_string(),
        id: item.id.clone(),
        title: item.title.clone(),
        summary: item.summary.clone(),
        description: item.description.clone(),
        product_design_principle: item.product_design_principle.clone(),
        coding_guideline: item.coding_guideline.clone(),
        priority: item.priority.clone(),
        status: item.status.clone(),
        linked_philosophies: item.linked_philosophies.clone(),
        linked_policies: item.linked_policies.clone(),
        linked_requirements: item.linked_requirements.clone(),
        linked_features: item.linked_features.clone(),
        tests: item
            .tests
            .iter()
            .map(|group| SpecBrowserTraceGroup {
                language: group.language.clone(),
                references: group
                    .references
                    .iter()
                    .map(|reference| SpecBrowserTraceReference {
                        file: reference.file.clone(),
                        symbols: reference.symbols.clone(),
                        doc_contains: reference.doc_contains.clone(),
                        method: reference.method.clone(),
                        path: reference.path.clone(),
                    })
                    .collect(),
            })
            .collect(),
        implementations: item
            .implementations
            .iter()
            .map(|group| SpecBrowserTraceGroup {
                language: group.language.clone(),
                references: group
                    .references
                    .iter()
                    .map(|reference| SpecBrowserTraceReference {
                        file: reference.file.clone(),
                        symbols: reference.symbols.clone(),
                        doc_contains: reference.doc_contains.clone(),
                        method: reference.method.clone(),
                        path: reference.path.clone(),
                    })
                    .collect(),
            })
            .collect(),
    }
}

pub(super) async fn workbench_css() -> Response {
    (
        [
            (header::CONTENT_TYPE, "text/css; charset=utf-8"),
            (header::CACHE_CONTROL, "no-cache"),
        ],
        include_str!("../../syu-app-ui/assets/tailwind.css"),
    )
        .into_response()
}
