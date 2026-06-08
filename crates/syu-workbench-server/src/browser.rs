use super::*;
use std::collections::BTreeSet;

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
    if let Some(kind) = view.spec_kind {
        ui.set_spec_kind(kind);
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
            .await
        {
            ui.cli_preview = Some(preview);
        }
    }
    if allow_run && view.diagnostics_all.as_deref() == Some("1") {
        ui.selected_cli_command_id = Some("diagnostics.all".to_string());
        ui.cli_preview = Some(
            run_all_diagnostics(
                server.inner.config.workspace_root.as_path(),
                !ui.payload.state.goals.active.is_empty(),
                server.inner.config.show_log || view.show_log.as_deref() == Some("1"),
            )
            .await,
        );
    }
    if allow_run && let Some(item_id) = view.item_edit.as_deref() {
        let values = if let Some(payload) = view.item_edit_payload.as_deref() {
            serde_json::from_str(payload).ok()
        } else {
            Some(ItemEditValues {
                title: view.title.unwrap_or_default(),
                summary: view.summary.unwrap_or_default(),
                description: view.description.unwrap_or_default(),
                product_design_principle: view.product_design_principle.unwrap_or_default(),
                coding_guideline: view.coding_guideline.unwrap_or_default(),
                priority: view.priority.unwrap_or_default(),
                status: view.status.unwrap_or_default(),
                linked_philosophies: split_item_links(view.linked_philosophies.as_deref()),
                linked_policies: split_item_links(view.linked_policies.as_deref()),
                linked_requirements: split_item_links(view.linked_requirements.as_deref()),
                linked_features: split_item_links(view.linked_features.as_deref()),
                tests_yaml: view.tests_yaml.unwrap_or_default(),
                implementations_yaml: view.implementations_yaml.unwrap_or_default(),
                source_hashes: BTreeMap::new(),
            })
        };
        ui.item_edit_preview = match values {
            Some(values) => Some(
                match preview_or_apply_item_edit(
                    &server,
                    item_id,
                    values,
                    view.item_edit_apply.as_deref() == Some("1"),
                )
                .await
                {
                    Ok(preview) => preview,
                    Err(error) => ItemEditPreview {
                        item_id: item_id.to_string(),
                        diff: error.to_string(),
                        apply_payload: String::new(),
                        applied: false,
                        message: "The item change could not be prepared or applied.".to_string(),
                    },
                },
            ),
            None => None,
        };
    }
    if let Some(goal_id) = view.goal {
        ui.payload.state.goals.selected_goal_id = Some(goal_id);
    }
    let requested_pane = view.pane.as_deref();
    let requested_is_palette = matches!(requested_pane, Some("commands" | "palette"));
    let active_pane = if requested_is_palette {
        ui.selected_cli_command_id
            .as_deref()
            .map(WorkbenchPane::for_cli)
            .or_else(|| ui.selected_action_id.map(WorkbenchPane::for_action))
            .unwrap_or(WorkbenchPane::Request)
    } else {
        requested_pane
            .and_then(WorkbenchPane::from_slug)
            .unwrap_or(WorkbenchPane::Request)
    };
    if !requested_is_palette {
        if ui
            .selected_cli_command_id
            .as_deref()
            .is_some_and(|id| WorkbenchPane::for_cli(id).role() != active_pane.role())
        {
            ui.selected_cli_command_id = None;
            ui.cli_preview = None;
        }
        if ui
            .selected_action_id
            .is_some_and(|id| WorkbenchPane::for_action(id).role() != active_pane.role())
        {
            ui.selected_action_id = None;
            ui.preview = None;
        }
    }
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct ItemEditValues {
    title: String,
    summary: String,
    description: String,
    product_design_principle: String,
    coding_guideline: String,
    priority: String,
    status: String,
    linked_philosophies: Vec<String>,
    linked_policies: Vec<String>,
    linked_requirements: Vec<String>,
    linked_features: Vec<String>,
    tests_yaml: String,
    implementations_yaml: String,
    source_hashes: BTreeMap<String, u64>,
}

fn split_item_links(value: Option<&str>) -> Vec<String> {
    value
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .collect()
}

pub(super) async fn preview_or_apply_item_edit(
    server: &WorkbenchServer,
    item_id: &str,
    mut values: ItemEditValues,
    apply: bool,
) -> Result<ItemEditPreview> {
    let (document_path, kind, item_index) = {
        let workspace = server.inner.browser_workspace.read().await;
        let selected = workspace
            .sections
            .iter()
            .flat_map(|section| section.documents.iter())
            .find_map(|document| {
                document
                    .items
                    .iter()
                    .find(|item| item.id == item_id)
                    .map(|item| (document.path.clone(), item.kind))
            })
            .with_context(|| format!("unknown item `{item_id}`"))?;
        let item_index = workspace
            .item_index
            .iter()
            .map(|(id, entry)| (id.clone(), (entry.document_path.clone(), entry.kind)))
            .collect::<BTreeMap<_, _>>();
        (selected.0, selected.1, item_index)
    };
    let path = server.inner.config.spec_root.join(&document_path);
    let raw = fs::read_to_string(&path)
        .with_context(|| format!("failed to read `{}`", path.display()))?;
    let mut document: serde_yaml::Value = serde_yaml::from_str(&raw)
        .with_context(|| format!("failed to parse `{}`", path.display()))?;
    let item = yaml_item_mut(&mut document, kind, item_id)?;
    let mapping = item
        .as_mapping_mut()
        .context("spec item must be a mapping")?;
    let old_links = reciprocal_links(kind)
        .iter()
        .map(|relation| {
            (
                relation.source_field,
                yaml_string_list(mapping, relation.source_field),
            )
        })
        .collect::<BTreeMap<_, _>>();
    set_yaml_string(mapping, "title", values.title.clone());
    set_optional_yaml_string(mapping, "summary", values.summary.clone());
    set_optional_yaml_string(mapping, "description", values.description.clone());
    set_optional_yaml_string(
        mapping,
        "product_design_principle",
        values.product_design_principle.clone(),
    );
    set_optional_yaml_string(mapping, "coding_guideline", values.coding_guideline.clone());
    set_optional_yaml_string(mapping, "priority", values.priority.clone());
    set_optional_yaml_string(mapping, "status", values.status.clone());
    set_yaml_list(mapping, "linked_philosophies", &values.linked_philosophies);
    set_yaml_list(mapping, "linked_policies", &values.linked_policies);
    set_yaml_list(mapping, "linked_requirements", &values.linked_requirements);
    set_yaml_list(mapping, "linked_features", &values.linked_features);
    set_yaml_mapping_text(mapping, "tests", &values.tests_yaml)?;
    set_yaml_mapping_text(mapping, "implementations", &values.implementations_yaml)?;
    let updated_item = item.clone();
    let updated = replace_yaml_item_block(&raw, item_id, &updated_item)?;
    let mut changes = BTreeMap::from([(document_path.clone(), (raw, updated))]);
    for relation in reciprocal_links(kind) {
        let old = old_links
            .get(relation.source_field)
            .cloned()
            .unwrap_or_default();
        let new = values.link_values(relation.source_field);
        for target_id in old
            .iter()
            .chain(new.iter())
            .cloned()
            .collect::<BTreeSet<_>>()
        {
            let (target_path, target_kind) = item_index
                .get(&target_id)
                .with_context(|| format!("unknown linked item `{target_id}`"))?;
            if *target_kind != relation.target_kind {
                bail!(
                    "`{target_id}` is not a {} item",
                    relation.target_kind.label()
                );
            }
            let should_link = new.contains(&target_id);
            if !changes.contains_key(target_path) {
                let target = server.inner.config.spec_root.join(target_path);
                let target_raw = fs::read_to_string(&target)
                    .with_context(|| format!("failed to read `{}`", target.display()))?;
                changes.insert(target_path.clone(), (target_raw.clone(), target_raw));
            }
            let entry = changes
                .get_mut(target_path)
                .context("linked item source was not loaded")?;
            entry.1 = update_reciprocal_link(
                &entry.1,
                *target_kind,
                &target_id,
                relation.target_field,
                item_id,
                should_link,
            )?;
        }
    }
    changes.retain(|_, (before, after)| before != after);
    if apply {
        verify_source_hashes(
            &server.inner.config.spec_root,
            &changes,
            &values.source_hashes,
        )?;
        write_item_changes(&server.inner.config.spec_root, &changes)?;
    } else {
        values.source_hashes = changes
            .iter()
            .map(|(path, (before, _))| (path.clone(), text_hash(before)))
            .collect();
    }
    let diff = changes
        .iter()
        .map(|(relative, (before, after))| {
            let path = server.inner.config.spec_root.join(relative);
            format!(
                "--- {}\n+++ {}\n\n{}",
                path.display(),
                path.display(),
                line_diff(before, after)
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    Ok(ItemEditPreview {
        item_id: item_id.to_string(),
        diff,
        apply_payload: serde_json::to_string(&values)?,
        applied: apply,
        message: if apply {
            "The reviewed item change was applied.".to_string()
        } else {
            "Review this source-preserving item diff before applying it.".to_string()
        },
    })
}

#[derive(Clone, Copy)]
struct ReciprocalLink {
    source_field: &'static str,
    target_kind: SectionKind,
    target_field: &'static str,
}

fn reciprocal_links(kind: SectionKind) -> &'static [ReciprocalLink] {
    match kind {
        SectionKind::Philosophy => &[ReciprocalLink {
            source_field: "linked_policies",
            target_kind: SectionKind::Policies,
            target_field: "linked_philosophies",
        }],
        SectionKind::Policies => &[
            ReciprocalLink {
                source_field: "linked_philosophies",
                target_kind: SectionKind::Philosophy,
                target_field: "linked_policies",
            },
            ReciprocalLink {
                source_field: "linked_requirements",
                target_kind: SectionKind::Requirements,
                target_field: "linked_policies",
            },
        ],
        SectionKind::Requirements => &[
            ReciprocalLink {
                source_field: "linked_policies",
                target_kind: SectionKind::Policies,
                target_field: "linked_requirements",
            },
            ReciprocalLink {
                source_field: "linked_features",
                target_kind: SectionKind::Features,
                target_field: "linked_requirements",
            },
        ],
        SectionKind::Features => &[ReciprocalLink {
            source_field: "linked_requirements",
            target_kind: SectionKind::Requirements,
            target_field: "linked_features",
        }],
    }
}

impl ItemEditValues {
    fn link_values(&self, field: &str) -> Vec<String> {
        match field {
            "linked_philosophies" => self.linked_philosophies.clone(),
            "linked_policies" => self.linked_policies.clone(),
            "linked_requirements" => self.linked_requirements.clone(),
            "linked_features" => self.linked_features.clone(),
            _ => Vec::new(),
        }
    }
}

fn yaml_item_mut<'a>(
    document: &'a mut serde_yaml::Value,
    kind: SectionKind,
    item_id: &str,
) -> Result<&'a mut serde_yaml::Value> {
    let list_key = match kind {
        SectionKind::Philosophy => "philosophies",
        SectionKind::Policies => "policies",
        SectionKind::Requirements => "requirements",
        SectionKind::Features => "features",
    };
    document
        .as_mapping_mut()
        .and_then(|mapping| mapping.get_mut(serde_yaml::Value::String(list_key.to_string())))
        .and_then(serde_yaml::Value::as_sequence_mut)
        .context("spec document does not contain its item list")?
        .iter_mut()
        .find(|item| {
            item.as_mapping()
                .and_then(|mapping| mapping.get(serde_yaml::Value::String("id".to_string())))
                .and_then(serde_yaml::Value::as_str)
                == Some(item_id)
        })
        .with_context(|| format!("item `{item_id}` is missing from its document"))
}

fn yaml_string_list(mapping: &serde_yaml::Mapping, key: &str) -> Vec<String> {
    mapping
        .get(serde_yaml::Value::String(key.to_string()))
        .and_then(serde_yaml::Value::as_sequence)
        .into_iter()
        .flatten()
        .filter_map(serde_yaml::Value::as_str)
        .map(str::to_string)
        .collect()
}

fn update_reciprocal_link(
    raw: &str,
    kind: SectionKind,
    item_id: &str,
    field: &str,
    linked_id: &str,
    should_link: bool,
) -> Result<String> {
    let mut document: serde_yaml::Value = serde_yaml::from_str(raw)?;
    let item = yaml_item_mut(&mut document, kind, item_id)?;
    let mapping = item
        .as_mapping_mut()
        .context("spec item must be a mapping")?;
    let mut links = yaml_string_list(mapping, field);
    if should_link {
        if !links.iter().any(|value| value == linked_id) {
            links.push(linked_id.to_string());
        }
    } else {
        links.retain(|value| value != linked_id);
    }
    set_yaml_list(mapping, field, &links);
    replace_yaml_item_block(raw, item_id, &item.clone())
}

fn verify_source_hashes(
    spec_root: &FsPath,
    changes: &BTreeMap<String, (String, String)>,
    expected: &BTreeMap<String, u64>,
) -> Result<()> {
    for (relative, (before, _)) in changes {
        let current = fs::read_to_string(spec_root.join(relative))?;
        if expected.get(relative) != Some(&text_hash(&current)) || current != *before {
            bail!(
                "source document `{relative}` changed after preview; reload the item and review a new diff"
            );
        }
    }
    Ok(())
}

fn write_item_changes(
    spec_root: &FsPath,
    changes: &BTreeMap<String, (String, String)>,
) -> Result<()> {
    let mut staged = Vec::new();
    for (relative, (_, after)) in changes {
        let path = spec_root.join(relative);
        let file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .context("item source path must have a UTF-8 file name")?;
        let temp = path.with_file_name(format!(".{file_name}.syu-workbench-{}", text_hash(after)));
        if let Err(error) = fs::write(&temp, after) {
            for (_, staged_temp) in &staged {
                let _ = fs::remove_file(staged_temp);
            }
            return Err(error).with_context(|| format!("failed to stage `{}`", path.display()));
        }
        staged.push((path, temp));
    }
    let mut applied = Vec::new();
    for (path, temp) in &staged {
        if let Err(error) = fs::rename(temp, path) {
            for (applied_path, before) in applied.iter().rev() {
                let _ = fs::write(applied_path, before);
            }
            for (_, remaining) in &staged {
                let _ = fs::remove_file(remaining);
            }
            return Err(error).with_context(|| format!("failed to apply `{}`", path.display()));
        }
        let before = changes
            .iter()
            .find_map(|(relative, (before, _))| {
                (spec_root.join(relative) == *path).then_some(before.clone())
            })
            .context("applied item edit is missing its rollback source")?;
        applied.push((path.clone(), before));
    }
    Ok(())
}

fn text_hash(value: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

fn set_yaml_string(mapping: &mut serde_yaml::Mapping, key: &str, value: String) {
    mapping.insert(
        serde_yaml::Value::String(key.to_string()),
        serde_yaml::Value::String(value),
    );
}

fn set_optional_yaml_string(mapping: &mut serde_yaml::Mapping, key: &str, value: String) {
    let yaml_key = serde_yaml::Value::String(key.to_string());
    if value.is_empty() && !mapping.contains_key(&yaml_key) {
        return;
    }
    mapping.insert(yaml_key, serde_yaml::Value::String(value));
}

fn set_yaml_list(mapping: &mut serde_yaml::Mapping, key: &str, values: &[String]) {
    let yaml_key = serde_yaml::Value::String(key.to_string());
    if values.is_empty() && !mapping.contains_key(&yaml_key) {
        return;
    }
    mapping.insert(
        yaml_key,
        serde_yaml::Value::Sequence(
            values
                .iter()
                .cloned()
                .map(serde_yaml::Value::String)
                .collect(),
        ),
    );
}

fn set_yaml_mapping_text(mapping: &mut serde_yaml::Mapping, key: &str, value: &str) -> Result<()> {
    let yaml_key = serde_yaml::Value::String(key.to_string());
    if value.trim().is_empty() && !mapping.contains_key(&yaml_key) {
        return Ok(());
    }
    let parsed = if value.trim().is_empty() {
        serde_yaml::Value::Mapping(serde_yaml::Mapping::new())
    } else {
        serde_yaml::from_str(value)
            .with_context(|| format!("`{key}` must be valid YAML mapping content"))?
    };
    if !parsed.is_mapping() {
        bail!("`{key}` must be a YAML mapping grouped by language");
    }
    mapping.insert(yaml_key, parsed);
    Ok(())
}

pub(super) fn replace_yaml_item_block(
    raw: &str,
    item_id: &str,
    item: &serde_yaml::Value,
) -> Result<String> {
    let lines = raw.split_inclusive('\n').collect::<Vec<_>>();
    let marker = format!("- id: {item_id}");
    let start = lines
        .iter()
        .position(|line| line.trim_end().trim_start() == marker)
        .with_context(|| format!("could not locate source block for `{item_id}`"))?;
    let indent = lines[start].len() - lines[start].trim_start().len();
    let end = lines
        .iter()
        .enumerate()
        .skip(start + 1)
        .find(|(_, line)| {
            line.len() - line.trim_start().len() == indent && line.trim_start().starts_with("- id:")
        })
        .map_or(lines.len(), |(index, _)| index);
    let yaml = serde_yaml::to_string(item)?;
    let mut rendered = String::new();
    for (index, line) in yaml.lines().enumerate() {
        if index == 0 {
            rendered.push_str(&" ".repeat(indent));
            rendered.push_str("- ");
            rendered.push_str(line);
        } else {
            rendered.push_str(&" ".repeat(indent + 2));
            rendered.push_str(line);
        }
        rendered.push('\n');
    }
    let mut updated = String::new();
    updated.push_str(&lines[..start].concat());
    updated.push_str(&rendered);
    updated.push_str(&lines[end..].concat());
    Ok(updated)
}

fn line_diff(before: &str, after: &str) -> String {
    let before_lines = before.lines().collect::<Vec<_>>();
    let after_lines = after.lines().collect::<Vec<_>>();
    let mut result = String::new();
    for line in before_lines
        .iter()
        .filter(|line| !after_lines.contains(line))
    {
        result.push_str("- ");
        result.push_str(line);
        result.push('\n');
    }
    for line in after_lines
        .iter()
        .filter(|line| !before_lines.contains(line))
    {
        result.push_str("+ ");
        result.push_str(line);
        result.push('\n');
    }
    if result.is_empty() {
        result.push_str("No changes.");
    }
    result
}

async fn run_all_diagnostics(
    workspace_root: &FsPath,
    goal_available: bool,
    show_log: bool,
) -> CliCommandPreview {
    let mut items = Vec::new();
    let mut overall = CommandResultStatus::Pass;
    for (id, title) in [
        ("cli.validate", "Workspace validation"),
        ("cli.doctor", "Contributor doctor"),
        ("cli.audit", "Specification audit"),
    ] {
        if let Some(preview) =
            run_cli_command_preview(id, workspace_root, None, false, show_log).await
        {
            if preview.result.status == CommandResultStatus::Fail {
                overall = CommandResultStatus::Fail;
            } else if preview.result.status == CommandResultStatus::Warn
                && overall != CommandResultStatus::Fail
            {
                overall = CommandResultStatus::Warn;
            }
            items.push(CommandResultItem {
                id: id.to_string(),
                title: title.to_string(),
                summary: preview.result.summary.clone(),
                detail: preview.result.summary,
                status: preview.result.status,
            });
        }
    }
    if goal_available {
        if let Some(preview) =
            run_cli_command_preview("cli.task.check", workspace_root, None, false, show_log).await
        {
            items.push(CommandResultItem {
                id: "cli.task.check".to_string(),
                title: "Goal check".to_string(),
                summary: preview.result.summary.clone(),
                detail: preview.result.summary,
                status: preview.result.status,
            });
        }
    } else {
        items.push(CommandResultItem {
            id: "cli.task.check".to_string(),
            title: "Goal check".to_string(),
            summary: "Skipped because no active Goal Plan is available.".to_string(),
            detail: "Create or select a Goal Plan, then refresh diagnostics again.".to_string(),
            status: CommandResultStatus::Pending,
        });
    }
    CliCommandPreview {
        id: "diagnostics.all".to_string(),
        title: "Refresh all diagnostics".to_string(),
        invocation: "Workbench diagnostics refresh".to_string(),
        result_summary: "Refreshed all available diagnostics.".to_string(),
        evidence_summary: format!("{} tools", items.len()),
        requires_input: false,
        mutates_files: false,
        category: CommandCategory::Check,
        effect: syu_app_ui::model::CommandEffect::ReadOnly,
        result: TypedCommandResult {
            kind: syu_app_ui::model::CommandResultKind::CheckDetail,
            status: overall,
            summary: "All diagnostics refreshed".to_string(),
            items,
            diagnostics: None,
        },
    }
}

pub(super) fn workbench_document(shell: String, locale: Locale) -> String {
    format!(
        r#"<!doctype html>
<html lang="{lang}">
<head>
  <meta charset="utf-8">
  <base href="/">
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
      if ('scrollRestoration' in history) history.scrollRestoration = 'manual';
      const rootSelector = '#syu-workbench-root';
      const sameWorkbenchUrl = (url) => {{
        try {{
          const next = new URL(url, window.location.href);
          return next.origin === window.location.origin && next.pathname === '/';
        }} catch (_) {{
          return false;
        }}
      }};
      const replaceWorkbench = async (url, options = {{}}, push = true, historyUrl = url) => {{
        const response = await fetch(url, {{
          ...options,
          headers: {{
            Accept: 'text/html',
            ...(options.headers || {{}}),
          }},
        }});
        if (!response.ok) throw new Error(`Workbench request failed: ${{response.status}}`);
        const html = await response.text();
        const documentNext = new DOMParser().parseFromString(html, 'text/html');
        const nextRoot = documentNext.querySelector(rootSelector);
        const currentRoot = document.querySelector(rootSelector);
        if (!nextRoot || !currentRoot) {{
          window.location.assign(url);
          return;
        }}
        const currentScrollY = window.scrollY;
        nextRoot.style.minHeight = `${{Math.max(currentRoot.offsetHeight, window.scrollY + window.innerHeight)}}px`;
        currentRoot.replaceWith(nextRoot);
        document.documentElement.lang = documentNext.documentElement.lang || document.documentElement.lang;
        if (push) history.pushState({{ syuWorkbench: true }}, '', historyUrl);
        initWorkbench(nextRoot);
        window.scrollTo(0, currentScrollY);
        requestAnimationFrame(() => window.scrollTo(0, currentScrollY));
        setTimeout(() => window.scrollTo(0, currentScrollY), 0);
      }};
      const formUrl = (form) => {{
        const action = form.getAttribute('action') || window.location.href;
        return new URL(action, window.location.href);
      }};
      const submitGetForm = (form) => {{
        const url = formUrl(form);
        const params = new URLSearchParams(new FormData(form));
        url.search = params.toString();
        return replaceWorkbench(url.href);
      }};
      const submitPostForm = (form) => {{
        const url = formUrl(form);
        const displayUrl = new URL('/', window.location.origin);
        displayUrl.search = new URLSearchParams(new FormData(form)).toString();
        return replaceWorkbench(url.href, {{
          method: 'POST',
          body: new URLSearchParams(new FormData(form)),
        }}, true, displayUrl.href);
      }};
      const markRunning = (form) => {{
        if (!form.matches('[data-command-run-form]')) return;
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
      }};
      const initWorkbench = (root = document) => {{
        const palettes = root.querySelectorAll('[data-command-palette]:not([data-enhanced])');
        for (const palette of palettes) {{
          palette.dataset.enhanced = 'true';
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
        for (const item of root.querySelectorAll('[data-result-item]:not([data-enhanced])')) {{
          item.dataset.enhanced = 'true';
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
        for (const item of root.querySelectorAll('[data-spec-tree-item]:not([data-enhanced])')) {{
          item.dataset.enhanced = 'true';
          item.addEventListener('click', (event) => {{
            const id = item.dataset.specItemTarget;
            const surface = item.closest('[data-spec-kind-panel]');
            if (!id || !surface) return;
            const detail = surface.querySelector(`[data-spec-detail-card="${{CSS.escape(id)}}"]`);
            if (!detail) return;
            event.preventDefault();
            for (const card of surface.querySelectorAll('[data-spec-detail-card]')) {{
              card.hidden = card !== detail;
            }}
            for (const candidate of surface.querySelectorAll('[data-spec-tree-item]')) {{
              candidate.setAttribute('aria-current', candidate === item ? 'page' : 'false');
            }}
            const url = new URL(item.href, window.location.href);
            history.pushState({{ syuWorkbench: true }}, '', url.href);
          }});
        }}
        for (const form of root.querySelectorAll('form:not([data-enhanced])')) {{
          form.dataset.enhanced = 'true';
          form.addEventListener('submit', async (event) => {{
            if (!form.checkValidity()) return;
            if (form.dataset.running === 'true') {{
              event.preventDefault();
              return;
            }}
            const method = (form.getAttribute('method') || 'get').toLowerCase();
            const url = formUrl(form);
            if (!sameWorkbenchUrl(url.href) && url.pathname !== '/run') return;
            event.preventDefault();
            markRunning(form);
            try {{
              if (method === 'post') {{
                await submitPostForm(form);
              }} else {{
                await submitGetForm(form);
              }}
            }} catch (_) {{
              form.submit();
            }}
          }});
        }}
      }};
      document.addEventListener('click', async (event) => {{
        if (!(event.target instanceof Element)) return;
        const link = event.target.closest('a[href]');
        if (!link || event.defaultPrevented) return;
        if (event.metaKey || event.ctrlKey || event.shiftKey || event.altKey || event.button !== 0) return;
        if (link.target && link.target !== '_self') return;
        if (link.hasAttribute('download')) return;
        const href = link.getAttribute('href') || '';
        if (href.startsWith('#')) return;
        const url = new URL(href, window.location.href);
        if (!sameWorkbenchUrl(url.href)) return;
        event.preventDefault();
        try {{
          await replaceWorkbench(url.href);
        }} catch (_) {{
          window.location.assign(url.href);
        }}
      }});
      window.addEventListener('popstate', async () => {{
        try {{
          await replaceWorkbench(window.location.href, {{}}, false);
        }} catch (_) {{
          window.location.reload();
        }}
      }});
      history.replaceState({{ syuWorkbench: true }}, '', window.location.href);
      initWorkbench(document);
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

pub(super) async fn run_cli_command_preview(
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

    let executable = std::env::current_exe().ok()?;
    let workspace_root = workspace_root.to_path_buf();
    let output = task::spawn_blocking(move || {
        Command::new(executable)
            .args(&args)
            .current_dir(workspace_root)
            .output()
    })
    .await
    .ok()?;
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
                show_log
                    .then(|| format!("stdout:\n{}\n\nstderr:\n{}", stdout.trim(), stderr.trim())),
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
