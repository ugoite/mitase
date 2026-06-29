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

pub(super) fn validate_same_origin(headers: &HeaderMap) -> Result<(), StatusCode> {
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
    mut view: WorkbenchViewQuery,
    allow_mutation: bool,
) -> Html<String> {
    let active_page = view
        .page
        .as_deref()
        .and_then(WorkbenchPage::from_slug)
        .unwrap_or_default();
    if active_page == WorkbenchPage::Scope
        && server.inner.state.read().await.branch_scope.is_none()
        && let Ok(report) =
            build_branch_scope(&server.inner.config.workspace_root, "origin/main...HEAD").await
    {
        server.inner.state.write().await.branch_scope = Some(report);
    }
    let state = server.inner.state.read().await.clone();
    let mut ui = WorkbenchUiState::from_state(shared_workbench_state(state));
    let browser_workspace = server.inner.browser_workspace.read().await;
    ui.spec_browser = Some(shared_spec_browser_model(
        &browser_workspace,
        view.entity.as_deref(),
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
    if let Some(locale) = view.lang.as_deref().and_then(Locale::from_slug) {
        ui.set_locale(locale);
    }
    ui.settings = Some(load_workspace_settings(&server));
    if allow_mutation && let Some(item_id) = view.item_edit.clone() {
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
                    &item_id,
                    values,
                    view.item_edit_apply.as_deref() == Some("1"),
                )
                .await
                {
                    Ok(preview) => preview,
                    Err(error) => ItemEditPreview {
                        item_id: item_id.clone(),
                        diff: error.to_string(),
                        apply_payload: String::new(),
                        applied: false,
                        message: "The item change could not be prepared or applied.".to_string(),
                    },
                },
            ),
            None => None,
        };
        view.entity = Some(item_id);
    }
    if active_page == WorkbenchPage::Work
        && let Some(goal_id) = view.entity.as_ref()
        && ui
            .payload
            .state
            .goals
            .active
            .iter()
            .any(|goal| &goal.goal_id == goal_id)
    {
        ui.payload.state.goals.selected_goal_id = Some(goal_id.clone());
    }
    let section = view.section.as_deref().and_then(PageSection::from_slug);
    let focus = view.focus.as_deref().and_then(FocusIntent::from_slug);
    let locale = ui.locale;
    let shell = render_element(rsx! {
        AppShell { ui, active_page, section, entity: view.entity, focus, sidebar_open: true }
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
    let item_exists = server
        .inner
        .browser_workspace
        .read()
        .await
        .item_index
        .contains_key(item_id);
    if !item_exists {
        return preview_or_apply_item_create(server, item_id, values, apply).await;
    }
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

async fn preview_or_apply_item_create(
    server: &WorkbenchServer,
    item_id: &str,
    mut values: ItemEditValues,
    apply: bool,
) -> Result<ItemEditPreview> {
    let kind = section_kind_for_item_id(item_id)
        .with_context(|| format!("cannot infer Item kind from `{item_id}`"))?;
    let (document_path, item_index) = {
        let workspace = server.inner.browser_workspace.read().await;
        let document_path = workspace
            .sections
            .iter()
            .find(|section| section.kind == kind)
            .and_then(|section| section.documents.first())
            .map(|document| document.path.clone())
            .with_context(|| format!("no {} source document is available", kind.label()))?;
        let item_index = workspace
            .item_index
            .iter()
            .map(|(id, entry)| (id.clone(), (entry.document_path.clone(), entry.kind)))
            .collect::<BTreeMap<_, _>>();
        (document_path, item_index)
    };
    let path = server.inner.config.spec_root.join(&document_path);
    let raw = fs::read_to_string(&path)
        .with_context(|| format!("failed to read `{}`", path.display()))?;
    let mut mapping = serde_yaml::Mapping::new();
    set_yaml_string(&mut mapping, "id", item_id.to_string());
    set_yaml_string(&mut mapping, "title", values.title.clone());
    set_optional_yaml_string(&mut mapping, "summary", values.summary.clone());
    set_optional_yaml_string(&mut mapping, "description", values.description.clone());
    set_optional_yaml_string(
        &mut mapping,
        "product_design_principle",
        values.product_design_principle.clone(),
    );
    set_optional_yaml_string(
        &mut mapping,
        "coding_guideline",
        values.coding_guideline.clone(),
    );
    set_optional_yaml_string(&mut mapping, "priority", values.priority.clone());
    set_optional_yaml_string(&mut mapping, "status", values.status.clone());
    set_yaml_list(
        &mut mapping,
        "linked_philosophies",
        &values.linked_philosophies,
    );
    set_yaml_list(&mut mapping, "linked_policies", &values.linked_policies);
    set_yaml_list(
        &mut mapping,
        "linked_requirements",
        &values.linked_requirements,
    );
    set_yaml_list(&mut mapping, "linked_features", &values.linked_features);
    set_yaml_mapping_text(&mut mapping, "tests", &values.tests_yaml)?;
    set_yaml_mapping_text(
        &mut mapping,
        "implementations",
        &values.implementations_yaml,
    )?;
    let updated = append_yaml_item_block(&raw, kind, &serde_yaml::Value::Mapping(mapping))?;
    let mut changes = BTreeMap::from([(document_path.clone(), (raw, updated))]);
    for relation in reciprocal_links(kind) {
        for target_id in values.link_values(relation.source_field) {
            let (target_path, target_kind) = item_index
                .get(&target_id)
                .with_context(|| format!("unknown linked item `{target_id}`"))?;
            if *target_kind != relation.target_kind {
                bail!(
                    "`{target_id}` is not a {} item",
                    relation.target_kind.label()
                );
            }
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
                true,
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
            "The reviewed Item draft was created.".to_string()
        } else {
            "Review this source-preserving Item draft diff before applying it.".to_string()
        },
    })
}

fn section_kind_for_item_id(item_id: &str) -> Option<SectionKind> {
    if item_id.starts_with("PHIL-") {
        Some(SectionKind::Philosophy)
    } else if item_id.starts_with("POL-") {
        Some(SectionKind::Policies)
    } else if item_id.starts_with("REQ-") {
        Some(SectionKind::Requirements)
    } else if item_id.starts_with("FEAT-") {
        Some(SectionKind::Features)
    } else {
        None
    }
}

pub(super) fn append_yaml_item_block(
    raw: &str,
    kind: SectionKind,
    item: &serde_yaml::Value,
) -> Result<String> {
    let collection = match kind {
        SectionKind::Philosophy => "philosophies",
        SectionKind::Policies => "policies",
        SectionKind::Requirements => "requirements",
        SectionKind::Features => "features",
    };
    let mut rendered = serde_yaml::to_string(item)?
        .trim()
        .lines()
        .map(str::to_string)
        .collect::<Vec<_>>();
    let first = rendered
        .first_mut()
        .context("created Item must not be empty")?;
    *first = format!("  - {first}");
    for line in rendered.iter_mut().skip(1) {
        *line = format!("    {line}");
    }
    let mut lines = raw.lines().map(str::to_string).collect::<Vec<_>>();
    let start = lines
        .iter()
        .position(|line| line.trim_end() == format!("{collection}:"))
        .with_context(|| format!("missing `{collection}` collection"))?;
    let end = lines
        .iter()
        .enumerate()
        .skip(start + 1)
        .find(|(_, line)| !line.is_empty() && !line.starts_with(' ') && !line.starts_with('#'))
        .map(|(index, _)| index)
        .unwrap_or(lines.len());
    lines.splice(end..end, rendered);
    let mut result = lines.join("\n");
    result.push('\n');
    Ok(result)
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

pub(super) fn load_workspace_settings(server: &WorkbenchServer) -> WorkspaceSettingsState {
    let path = server.inner.config.workspace_root.join("syu.yaml");
    let raw_yaml = fs::read_to_string(&path).unwrap_or_default();
    let yaml = serde_yaml::from_str::<serde_yaml::Value>(&raw_yaml).unwrap_or_default();
    let strict_review = yaml
        .get("workbench")
        .and_then(|value| value.get("strict_review"))
        .and_then(serde_yaml::Value::as_bool)
        .unwrap_or(false);
    WorkspaceSettingsState {
        workspace_root: server.inner.config.workspace_root.display().to_string(),
        spec_root: server.inner.config.spec_root.display().to_string(),
        bind: server.inner.config.bind.clone(),
        port: server.inner.config.port,
        strict_review,
        raw_yaml,
    }
}

pub(super) fn preview_or_apply_settings(
    server: &WorkbenchServer,
    mut update: SettingsUpdate,
    apply: bool,
) -> Result<SettingsPreview> {
    let path = server.inner.config.workspace_root.join("syu.yaml");
    let before = fs::read_to_string(&path).unwrap_or_default();
    let source_hash = text_hash(&before);
    let source_hash_text = source_hash.to_string();
    if apply && update.source_hash.as_deref() != Some(source_hash_text.as_str()) {
        bail!("syu.yaml changed after preview; refresh and review the new diff");
    }
    if update.bind.parse::<IpAddr>().is_err() {
        bail!("workbench bind must be an IP address");
    }
    if !matches!(update.bind.as_str(), "127.0.0.1" | "::1")
        && !server.inner.config.allow_remote_bind
    {
        bail!("remote bind requires explicit allow_remote_bind authorization");
    }
    let spec = PathBuf::from(update.spec_root.trim());
    let resolved_spec = if spec.is_absolute() {
        spec
    } else {
        server.inner.config.workspace_root.join(spec)
    };
    if !resolved_spec.starts_with(&server.inner.config.workspace_root) {
        bail!("Workbench Settings can only select a spec root inside this workspace");
    }
    let relative_spec = resolved_spec
        .strip_prefix(&server.inner.config.workspace_root)
        .unwrap_or(&resolved_spec)
        .display()
        .to_string();
    update.spec_root = relative_spec;
    let mut after = replace_yaml_scalar(&before, "spec", "root", &update.spec_root);
    after = replace_yaml_scalar(&after, "workbench", "bind", &update.bind);
    after = replace_yaml_scalar(&after, "workbench", "port", &update.port.to_string());
    after = replace_yaml_scalar(
        &after,
        "workbench",
        "strict_review",
        if update.strict_review {
            "true"
        } else {
            "false"
        },
    );
    serde_yaml::from_str::<serde_yaml::Value>(&after)
        .context("updated syu.yaml is not valid YAML")?;
    if apply {
        let temp = path.with_extension("yaml.syu-tmp");
        fs::write(&temp, &after)
            .with_context(|| format!("failed to write `{}`", temp.display()))?;
        fs::rename(&temp, &path)
            .with_context(|| format!("failed to replace `{}`", path.display()))?;
    }
    Ok(SettingsPreview {
        valid: true,
        applied: apply,
        diff: line_diff(&before, &after),
        message: if apply {
            "Validated and applied source-preserving syu.yaml changes.".to_string()
        } else {
            "Schema and semantic validation passed; review the source-preserving diff.".to_string()
        },
        source_hash: source_hash_text,
        update,
    })
}

fn replace_yaml_scalar(raw: &str, section: &str, key: &str, value: &str) -> String {
    let mut lines = raw.lines().map(str::to_string).collect::<Vec<_>>();
    let section_header = format!("{section}:");
    if let Some(section_index) = lines
        .iter()
        .position(|line| line.trim_end() == section_header)
    {
        let end = lines
            .iter()
            .enumerate()
            .skip(section_index + 1)
            .find(|(_, line)| {
                !line.is_empty() && !line.starts_with([' ', '\t']) && !line.starts_with('#')
            })
            .map(|(index, _)| index)
            .unwrap_or(lines.len());
        let prefix = format!("  {key}:");
        if let Some(index) = (section_index + 1..end)
            .find(|index| lines[*index].trim_start().starts_with(&format!("{key}:")))
        {
            let comment = lines[index]
                .split_once(" #")
                .map(|(_, comment)| format!(" #{}", comment))
                .unwrap_or_default();
            lines[index] = format!("{prefix} {value}{comment}");
        } else {
            lines.insert(end, format!("{prefix} {value}"));
        }
    } else {
        if lines.last().is_some_and(|line| !line.is_empty()) {
            lines.push(String::new());
        }
        lines.push(section_header);
        lines.push(format!("  {key}: {value}"));
    }
    let mut result = lines.join("\n");
    result.push('\n');
    result
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
    [data-command-palette]:focus-within .command-palette-results {{ display: block; }}
    [data-command-target] {{ scroll-margin-top: 7rem; }}
    summary::-webkit-details-marker {{ display: none; }}
    @media (prefers-reduced-motion: reduce) {{ * {{ scroll-behavior: auto !important; animation: none !important; transition: none !important; }} }}
  </style>
  <link rel="stylesheet" href="/assets/tailwind.css?v=workbench-pages">
</head>
<body class="bg-background text-foreground antialiased">
  <div id="syu-workbench-root" data-ui="dioxus-ssr">{shell}</div>
  <script>
    (() => {{
      const rootSelector = '#syu-workbench-root';
      let eventReloadTimer;
      if ('scrollRestoration' in history) history.scrollRestoration = 'manual';
      const samePage = (url) => {{ const next = new URL(url, location.href); return next.origin === location.origin && next.pathname === '/'; }};
      const replaceWorkbench = async (url, options = {{}}, push = true) => {{
        const response = await fetch(url, {{ ...options, headers: {{ Accept: 'text/html', ...(options.headers || {{}}) }} }});
        if (!response.ok) throw new Error('Workbench request failed');
        const parsed = new DOMParser().parseFromString(await response.text(), 'text/html');
        const next = parsed.querySelector(rootSelector);
        const current = document.querySelector(rootSelector);
        if (!next || !current) return location.assign(url);
        current.replaceWith(next);
        document.documentElement.lang = parsed.documentElement.lang || document.documentElement.lang;
        if (push) history.pushState({{ workbench: true }}, '', url);
        init(next);
      }};
      const init = (root = document) => {{
        for (const palette of root.querySelectorAll('[data-command-palette]')) {{
          const input = palette.querySelector('[data-command-input]');
          const items = [...palette.querySelectorAll('[data-command-item]')];
          if (!input) continue;
          input.addEventListener('input', () => {{
            const query = input.value.trim().toLowerCase();
            for (const item of items) item.hidden = query !== '' && !(item.dataset.commandText || '').toLowerCase().includes(query);
          }});
        }}
        const workSelector = root.querySelector('[data-work-selector]');
        workSelector?.addEventListener('change', async () => {{
          const params = new URLSearchParams({{
            page: 'work',
            section: workSelector.dataset.workSection || 'brief',
            entity: workSelector.value,
            lang: workSelector.dataset.workLang || document.documentElement.lang || 'en',
          }});
          await replaceWorkbench('/?' + params.toString());
        }});
        const diagnosticFilter = root.querySelector('[data-diagnostics-filter]');
        diagnosticFilter?.addEventListener('input', () => {{
          const query = diagnosticFilter.value.trim().toLowerCase();
          for (const check of root.querySelectorAll('[data-diagnostic-check]')) {{
            check.hidden = query !== '' && !check.textContent.toLowerCase().includes(query);
          }}
        }});
        const scopeMode = root.querySelector('[data-scope-mode]');
        const scopeTarget = root.querySelector('[data-scope-target]');
        const navigateScope = async (mode) => {{
          if (!scopeTarget) return;
          const options = [...scopeTarget.options];
          for (const option of options) option.hidden = option.dataset.scopeSource !== mode;
          const selected = options.find((option) => option.dataset.scopeSource === mode);
          if (!selected) return;
          if (selected.hidden || scopeTarget.selectedOptions[0]?.dataset.scopeSource !== mode) scopeTarget.value = selected.value;
          const params = new URLSearchParams({{
            page: 'scope',
            section: scopeTarget.dataset.scopeSection || 'code-tests',
            lang: scopeTarget.dataset.scopeLang || document.documentElement.lang || 'en',
          }});
          if (mode === 'goal' && scopeTarget.value) params.set('entity', scopeTarget.value);
          await replaceWorkbench('/?' + params.toString());
        }};
        scopeMode?.addEventListener('change', () => navigateScope(scopeMode.value));
        scopeTarget?.addEventListener('change', () => navigateScope(scopeMode?.value || 'branch'));
        const target = root.querySelector('[data-command-target].border-red-500');
        if (target) {{
          const largeTarget = target.getBoundingClientRect().height > innerHeight * 0.5;
          target.scrollIntoView({{ block: largeTarget ? 'start' : 'center', behavior: largeTarget || matchMedia('(prefers-reduced-motion: reduce)').matches ? 'auto' : 'smooth' }});
          target.focus?.({{ preventScroll: true }});
          setTimeout(() => {{ target.classList.remove('border-red-500', 'border-2'); target.classList.add('border-slate-200', 'border'); }}, 3000);
        }}
        const diagnostics = root.querySelector('[data-run-diagnostics]');
        diagnostics?.addEventListener('click', async () => {{
          diagnostics.disabled = true;
          diagnostics.textContent = diagnostics.dataset.runningLabel || 'Running…';
          await fetch('/api/diagnostics/run', {{ method: 'POST', headers: {{ 'Content-Type': 'application/json' }}, body: '{{}}' }});
          await replaceWorkbench(location.href, {{}}, false);
        }});
        for (const button of root.querySelectorAll('[data-create-work-from-item]')) {{
          button.addEventListener('click', async () => {{
            button.disabled = true;
            const id = button.dataset.createWorkFromItem;
            const response = await fetch('/api/items/' + encodeURIComponent(id) + '/work', {{ method: 'POST' }});
            if (!response.ok) throw new Error('Item-driven Work creation failed');
            const plan = await response.json();
            const params = new URLSearchParams({{ page: 'work', section: 'brief', entity: plan.goal.id, lang: button.dataset.workLang || document.documentElement.lang || 'en' }});
            await replaceWorkbench('/?' + params.toString());
          }});
        }}
        for (const button of root.querySelectorAll('[data-create-work-from-branch]')) {{
          button.addEventListener('click', async () => {{
            const originalLabel = button.textContent;
            button.disabled = true;
            button.textContent = button.dataset.runningLabel || 'Creating Work…';
            try {{
              const response = await fetch('/api/actions/branch.infer_goal/run', {{
                method: 'POST',
                headers: {{ 'Content-Type': 'application/json' }},
                body: JSON.stringify({{ range: button.dataset.createWorkFromBranch || 'origin/main...HEAD' }}),
              }});
              if (!response.ok) throw new Error('Branch-driven Work creation failed');
              const action = await response.json();
              const goalId = action.result?.goal?.id;
              if (!goalId) throw new Error('Created Work did not return a Goal id');
              const params = new URLSearchParams({{ page: 'work', section: 'brief', entity: goalId, lang: button.dataset.workLang || document.documentElement.lang || 'en' }});
              await replaceWorkbench('/?' + params.toString());
            }} catch (error) {{
              button.disabled = false;
              button.textContent = originalLabel;
              throw error;
            }}
          }});
        }}
        const settingsForm = root.querySelector('[data-settings-form]');
        const settingsPayload = () => {{
          const data = new FormData(settingsForm);
          return {{
            spec_root: String(data.get('spec_root') || ''),
            bind: String(data.get('bind') || ''),
            port: Number(data.get('port') || 0),
            strict_review: String(data.get('strict_review')) === 'true',
            source_hash: root.querySelector('[data-settings-source-hash]')?.value || null,
          }};
        }};
        const renderSettingsPreview = (preview) => {{
          const diff = root.querySelector('[data-settings-diff]');
          const message = root.querySelector('[data-settings-message]');
          const hash = root.querySelector('[data-settings-source-hash]');
          if (diff) diff.textContent = preview.diff;
          if (message) message.textContent = preview.message;
          if (hash) hash.value = String(preview.source_hash);
        }};
        root.querySelector('[data-settings-validate]')?.addEventListener('click', async () => {{
          const response = await fetch('/api/settings/preview', {{ method: 'POST', headers: {{ 'Content-Type': 'application/json' }}, body: JSON.stringify(settingsPayload()) }});
          renderSettingsPreview(await response.json());
        }});
        root.querySelector('[data-settings-apply]')?.addEventListener('click', async () => {{
          const response = await fetch('/api/settings/apply', {{ method: 'POST', headers: {{ 'Content-Type': 'application/json' }}, body: JSON.stringify(settingsPayload()) }});
          renderSettingsPreview(await response.json());
        }});
        for (const form of root.querySelectorAll('form')) form.addEventListener('submit', async (event) => {{
          if (!form.checkValidity()) return;
          const action = new URL(form.action || location.href, location.href);
          if (action.pathname !== '/' && action.pathname !== '/run') return;
          event.preventDefault();
          const data = new URLSearchParams(new FormData(form));
          if ((form.method || 'get').toLowerCase() === 'post') {{
            await replaceWorkbench(action.href, {{ method: 'POST', body: data }}, false);
            const destination = new URL('/', location.origin);
            for (const key of ['page', 'section', 'entity', 'focus', 'lang']) {{
              const value = data.get(key);
              if (value) destination.searchParams.set(key, value);
            }}
            const editedItem = data.get('item_edit');
            if (editedItem) destination.searchParams.set('entity', editedItem);
            history.pushState({{ workbench: true }}, '', destination);
          }} else {{ action.search = data.toString(); await replaceWorkbench(action.href); }}
        }});
      }};
      document.addEventListener('click', async (event) => {{
        const link = event.target instanceof Element ? event.target.closest('a[href]') : null;
        if (!link || event.defaultPrevented || event.metaKey || event.ctrlKey || event.shiftKey || event.altKey) return;
        if (!samePage(link.href)) return;
        event.preventDefault();
        await replaceWorkbench(link.href);
      }});
      window.addEventListener('popstate', () => replaceWorkbench(location.href, {{}}, false));
      const events = new EventSource('/api/events');
      events.onmessage = () => {{ clearTimeout(eventReloadTimer); eventReloadTimer = setTimeout(() => replaceWorkbench(location.href, {{}}, false), 150); }};
      history.replaceState({{ workbench: true }}, '', location.href);
      init();
    }})();
  </script>
</body>
</html>"#,
        shell = shell,
        lang = locale.slug(),
    )
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
