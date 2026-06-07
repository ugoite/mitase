use super::*;

pub(super) async fn workbench_index(
    State(server): State<WorkbenchServer>,
    Query(view): Query<WorkbenchViewQuery>,
) -> Html<String> {
    render_workbench(server, view, false).await
}

pub(super) async fn workbench_run(
    State(server): State<WorkbenchServer>,
    headers: HeaderMap,
    Form(view): Form<WorkbenchViewQuery>,
) -> Result<Html<String>, StatusCode> {
    validate_same_origin(&headers)?;
    Ok(render_workbench(server, view, true).await)
}

fn validate_same_origin(headers: &HeaderMap) -> Result<(), StatusCode> {
    let host = headers
        .get(header::HOST)
        .and_then(|value| value.to_str().ok())
        .ok_or(StatusCode::FORBIDDEN)?;
    let origin = headers
        .get(header::ORIGIN)
        .and_then(|value| value.to_str().ok())
        .ok_or(StatusCode::FORBIDDEN)?;
    let origin_host = origin
        .strip_prefix("http://")
        .or_else(|| origin.strip_prefix("https://"))
        .ok_or(StatusCode::FORBIDDEN)?;
    if origin_host != host {
        return Err(StatusCode::FORBIDDEN);
    }
    Ok(())
}

async fn render_workbench(
    server: WorkbenchServer,
    view: WorkbenchViewQuery,
    allow_run: bool,
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
    if let Some(query) = view.spec_query {
        ui.set_spec_query(query);
    }
    ui.set_command_category(
        view.category
            .as_deref()
            .and_then(CommandCategory::from_slug),
    );
    if let Some(locale) = view.lang.as_deref().and_then(Locale::from_slug) {
        ui.set_locale(locale);
    }
    if let Some(help_topic) = view.help.as_deref().and_then(HelpTopic::from_slug) {
        ui.set_help_topic(Some(help_topic));
    }
    if let Some(action) = view.action.and_then(shared_action_id) {
        let _ = ui.select_action(action);
        if allow_run
            && view.run.as_deref() == Some("1")
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
        if allow_run
            && view.run.as_deref() == Some("1")
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
    [data-result-item][aria-current="page"] {{
      border-color: var(--color-foreground);
      background: var(--color-foreground);
      color: var(--color-background);
    }}
    @media (min-width: 64rem) {{
      [data-result-grid] [data-result-detail-panel],
      [data-spec-browser-grid] [data-spec-detail] {{
        grid-column: span 2 / span 2;
      }}
      [data-spec-browser-grid] [data-spec-search] {{
        grid-column: 1 / -1;
      }}
    }}
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
      for (const item of document.querySelectorAll('[data-result-item]')) {{
        item.addEventListener('click', (event) => {{
          event.preventDefault();
          const id = item.dataset.resultItem;
          const surface = item.closest('[data-result-kind]');
          if (!surface || !id) return;
          for (const detail of surface.querySelectorAll('[data-result-detail]')) {{
            detail.hidden = detail.dataset.resultDetail !== id;
          }}
          for (const candidate of surface.querySelectorAll('[data-result-item]')) {{
            candidate.setAttribute('aria-current', candidate === item ? 'page' : 'false');
          }}
        }});
      }}
      for (const form of document.querySelectorAll('[data-command-run-form]')) {{
        form.addEventListener('submit', (event) => {{
          if (!form.checkValidity()) return;
          if (form.dataset.running === 'true') {{
            event.preventDefault();
            return;
          }}
          form.dataset.running = 'true';
          const button = form.querySelector('[data-command-run-button]');
          const status = form.querySelector('[data-command-run-status]');
          const runningLabel = button?.dataset.runningLabel || 'Running...';
          if (button) {{
            button.disabled = true;
            button.setAttribute('aria-disabled', 'true');
            button.innerHTML = `<span class="inline-block h-3.5 w-3.5 animate-spin rounded-full border-2 border-current border-r-transparent" aria-hidden="true"></span><span>${{runningLabel}}</span>`;
            button.classList.add('inline-flex', 'items-center', 'justify-center', 'gap-2', 'opacity-80');
          }}
          if (status) {{
            status.textContent = runningLabel;
            status.classList.remove('hidden');
          }}
        }});
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
        return shared_action_id(action_id.to_string()).map(|action| {
            typed_action_preview(
                action,
                action_id,
                "This command can change Workbench state or files. Confirm before running.",
                "confirmation required",
                CommandResultStatus::Pending,
                None,
            )
        });
    }
    let body = default_workbench_action_body(server, action_id, action_input).await;
    let missing_input = body.is_none();
    let body = body.unwrap_or_else(|| serde_json::json!({}));
    if missing_input {
        return shared_action_id(action_id.to_string()).map(|action| {
            typed_action_preview(
                action,
                action_id,
                "This command needs request, goal, assignment, or confirmation input before it can run.",
                "input required",
                CommandResultStatus::Pending,
                None,
            )
        });
    }

    let action = shared_action_id(action_id.to_string())?;
    let response = execute_action(server, action_id, body).await;
    let (result_summary, evidence_summary, status, structured) = match response {
        Ok(response) => {
            let result_summary = truncate_cli_output(
                &serde_json::to_string_pretty(&response.result).unwrap_or_default(),
            );
            (
                result_summary,
                format!("{:?}", response.event),
                CommandResultStatus::Pass,
                Some(response.result),
            )
        }
        Err(error) => (
            format!("failed to run {action_id}: {error}"),
            "failed".to_string(),
            CommandResultStatus::Fail,
            None,
        ),
    };
    Some(typed_action_preview(
        action,
        action_id,
        &result_summary,
        &evidence_summary,
        status,
        structured,
    ))
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
        return Some(typed_cli_preview(
            *command,
            command.invocation.to_string(),
            format!("{} needs input before it can run.", command.invocation),
            "input required".to_string(),
            CommandResultStatus::Pending,
            None,
            None,
        ));
    }
    if command.mutates_files && !confirmed {
        return Some(typed_cli_preview(
            *command,
            command.invocation.to_string(),
            format!(
                "{} needs confirmation before writing files.",
                command.invocation
            ),
            "confirmation required".to_string(),
            CommandResultStatus::Pending,
            None,
            None,
        ));
    }

    let cli_arg = cli_default_arg(command.id, cli_arg);
    if let Err(error) = ensure_cli_task_fixture(command.id, workspace_root, cli_arg) {
        return Some(typed_cli_preview(
            *command,
            command.invocation.to_string(),
            format!("failed to prepare command input: {error}"),
            "failed".to_string(),
            CommandResultStatus::Fail,
            None,
            None,
        ));
    }
    let args = cli_command_args(command.id, cli_arg)?;
    if matches!(command.id, "cli.workbench" | "cli.lsp") {
        return Some(typed_cli_preview(
            *command,
            command.invocation.to_string(),
            "Already represented by this Workbench session.".to_string(),
            "running".to_string(),
            CommandResultStatus::Ready,
            None,
            None,
        ));
    }

    let output = Command::new(std::env::current_exe().ok()?)
        .args(&args)
        .current_dir(workspace_root)
        .output();
    let (result_summary, evidence_summary, status, diagnostics, structured) = match output {
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
            let typed_status = if output.status.success() {
                CommandResultStatus::Pass
            } else {
                CommandResultStatus::Fail
            };
            (
                result_summary,
                status,
                typed_status,
                Some(format!(
                    "stdout:\n{}\n\nstderr:\n{}",
                    stdout.trim(),
                    stderr.trim()
                )),
                serde_json::from_str::<Value>(&body).ok(),
            )
        }
        Err(error) => (
            format!("failed to run {}: {error}", command.invocation),
            "failed".to_string(),
            CommandResultStatus::Fail,
            None,
            None,
        ),
    };

    Some(typed_cli_preview(
        *command,
        if cli_arg.is_empty() {
            command.invocation.to_string()
        } else {
            format!("{} · {}", command.invocation, cli_arg)
        },
        result_summary,
        evidence_summary,
        status,
        diagnostics,
        structured,
    ))
}

fn typed_cli_preview(
    command: syu_app_ui::model::CliCommandEntry,
    invocation: String,
    summary: String,
    detail: String,
    status: CommandResultStatus,
    diagnostics: Option<String>,
    structured: Option<Value>,
) -> CliCommandPreview {
    let result = structured.map_or_else(
        || {
            typed_result(
                command.category(),
                command.id,
                command.title,
                summary.clone(),
                detail.clone(),
                status,
                diagnostics,
            )
        },
        |value| typed_result_from_json(command.category(), summary.clone(), status, value),
    );
    CliCommandPreview {
        id: command.id.to_string(),
        title: command.title.to_string(),
        invocation,
        result_summary: summary,
        evidence_summary: detail,
        requires_input: command.requires_input,
        mutates_files: command.mutates_files,
        category: command.category(),
        effect: command.effect(),
        result,
    }
}

fn typed_action_preview(
    action_id: shared_workbench::WorkbenchActionId,
    action_label: &str,
    summary: &str,
    detail: &str,
    status: CommandResultStatus,
    structured: Option<Value>,
) -> WorkbenchActionRunPreview {
    let category = workbench_action_category(action_id);
    let effect = shared_workbench::WorkbenchActionRegistry::standard()
        .action(action_id)
        .map(syu_app_ui::model::workbench_action_effect)
        .unwrap_or(CommandEffect::ReadOnly);
    WorkbenchActionRunPreview {
        action_id,
        title: action_label.replace('.', " "),
        result_summary: summary.to_string(),
        evidence_summary: detail.to_string(),
        category,
        effect,
        result: structured.map_or_else(
            || {
                typed_result(
                    category,
                    action_label,
                    &action_label.replace('.', " "),
                    summary.to_string(),
                    detail.to_string(),
                    status,
                    None,
                )
            },
            |value| typed_result_from_json(category, summary.to_string(), status, value),
        ),
    }
}

fn typed_result(
    category: CommandCategory,
    id: &str,
    title: &str,
    summary: String,
    detail: String,
    status: CommandResultStatus,
    diagnostics: Option<String>,
) -> TypedCommandResult {
    TypedCommandResult {
        kind: category_result_kind(category),
        status,
        summary: summary.clone(),
        items: vec![CommandResultItem {
            id: id.to_string(),
            title: title.to_string(),
            summary,
            detail,
            status,
        }],
        diagnostics,
    }
}

pub(super) fn typed_result_from_json(
    category: CommandCategory,
    _summary: String,
    status: CommandResultStatus,
    value: Value,
) -> TypedCommandResult {
    let values = structured_result_values(value);
    let items = values
        .into_iter()
        .enumerate()
        .map(|(index, (field_name, value))| {
            let id = json_string(&value, &["id", "rule", "kind", "name"])
                .or(field_name.clone())
                .unwrap_or_else(|| format!("result-{}", index + 1));
            let title = json_string(&value, &["title", "summary", "message", "kind"])
                .or_else(|| field_name.as_deref().map(humanize_json_key))
                .unwrap_or_else(|| default_result_item_title(category, index));
            let item_status = json_status(&value).unwrap_or(status);
            CommandResultItem {
                id,
                title,
                summary: json_string(&value, &["summary", "message", "description"])
                    .unwrap_or_else(|| summarize_json_value(&value)),
                detail: serde_json::to_string_pretty(&value).unwrap_or_default(),
                status: item_status,
            }
        })
        .collect::<Vec<_>>();
    let aggregate_status = if items
        .iter()
        .any(|item| item.status == CommandResultStatus::Fail)
    {
        CommandResultStatus::Fail
    } else if items
        .iter()
        .any(|item| item.status == CommandResultStatus::Warn)
    {
        CommandResultStatus::Warn
    } else {
        status
    };
    TypedCommandResult {
        kind: category_result_kind(category),
        status: aggregate_status,
        summary: typed_result_summary(category, items.len(), aggregate_status),
        items,
        diagnostics: None,
    }
}

fn structured_result_values(value: Value) -> Vec<(Option<String>, Value)> {
    if let Some(items) = preferred_result_array(&value).filter(|items| !items.is_empty()) {
        return items.iter().cloned().map(|item| (None, item)).collect();
    }
    if let Value::Object(object) = value {
        return object
            .into_iter()
            .filter(|(_, value)| !matches!(value, Value::Array(items) if items.is_empty()))
            .map(|(key, value)| (Some(key), value))
            .collect();
    }
    vec![(None, value)]
}

fn summarize_json_value(value: &Value) -> String {
    match value {
        Value::Null => "No value".to_string(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        Value::String(value) => value.clone(),
        Value::Array(items) => format!("{} items", items.len()),
        Value::Object(fields) => format!("{} fields", fields.len()),
    }
}

fn humanize_json_key(key: &str) -> String {
    let mut words = key.split('_').filter(|word| !word.is_empty());
    let first = words.next().unwrap_or(key);
    let mut title = first.to_string();
    if let Some(initial) = title.get_mut(0..1) {
        initial.make_ascii_uppercase();
    }
    for word in words {
        title.push(' ');
        title.push_str(word);
    }
    title
}

fn typed_result_summary(
    category: CommandCategory,
    item_count: usize,
    status: CommandResultStatus,
) -> String {
    let noun = match category {
        CommandCategory::Browse => "items",
        CommandCategory::Check => "checks",
        CommandCategory::Plan => "proposals",
        CommandCategory::Change => "changes",
        CommandCategory::Operate => "events",
        CommandCategory::Generate => "artifacts",
    };
    format!("{item_count} {noun} · {}", status.label())
}

fn default_result_item_title(category: CommandCategory, index: usize) -> String {
    let noun = match category {
        CommandCategory::Browse => "Item",
        CommandCategory::Check => "Check",
        CommandCategory::Plan => "Proposal",
        CommandCategory::Change => "Change",
        CommandCategory::Operate => "Event",
        CommandCategory::Generate => "Artifact",
    };
    format!("{noun} {}", index + 1)
}

fn preferred_result_array(value: &Value) -> Option<&[Value]> {
    if let Some(items) = value.as_array() {
        return Some(items);
    }
    let object = value.as_object()?;
    [
        "issues",
        "findings",
        "checks",
        "items",
        "results",
        "matches",
        "templates",
        "updates",
        "commands",
    ]
    .into_iter()
    .find_map(|key| object.get(key).and_then(Value::as_array))
    .map(Vec::as_slice)
}

fn json_string(value: &Value, keys: &[&str]) -> Option<String> {
    let object = value.as_object()?;
    keys.iter()
        .find_map(|key| object.get(*key).and_then(Value::as_str).map(str::to_string))
}

fn json_status(value: &Value) -> Option<CommandResultStatus> {
    let status = json_string(value, &["status", "severity", "level"])?;
    match status.to_lowercase().as_str() {
        "pass" | "passed" | "success" | "ok" | "info" => Some(CommandResultStatus::Pass),
        "warn" | "warning" => Some(CommandResultStatus::Warn),
        "fail" | "failed" | "error" => Some(CommandResultStatus::Fail),
        "pending" | "unknown" => Some(CommandResultStatus::Pending),
        _ => None,
    }
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
    let mut args = args.into_iter().map(String::from).collect::<Vec<_>>();
    if cli_command_supports_json(command_id) {
        args.extend(["--format".to_string(), "json".to_string()]);
    }
    Some(args)
}

fn cli_command_supports_json(command_id: &str) -> bool {
    matches!(
        command_id,
        "cli.browse"
            | "cli.list"
            | "cli.show"
            | "cli.search"
            | "cli.audit"
            | "cli.log"
            | "cli.explain"
            | "cli.relate"
            | "cli.trace"
            | "cli.doctor"
            | "cli.validate"
            | "cli.init"
            | "cli.templates"
            | "cli.task.classify"
            | "cli.task.scope"
            | "cli.task.scaffold"
            | "cli.task.plan"
            | "cli.task.test_select"
            | "cli.task.infer"
            | "cli.task.check"
    )
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
    let path = safe_fixture_path(workspace_root, relative_path)?;
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
    let path = safe_fixture_path(workspace_root, relative_path)?;
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

fn safe_fixture_path(workspace_root: &FsPath, relative_path: &str) -> Result<PathBuf> {
    let relative = FsPath::new(relative_path);
    if relative.is_absolute()
        || relative
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        bail!("fixture path must be a normalized relative path");
    }
    let fixture_root = FsPath::new("target/syu/workbench");
    if !relative.starts_with(fixture_root) {
        bail!("fixture path must stay under `{}`", fixture_root.display());
    }
    Ok(workspace_root.join(relative))
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
