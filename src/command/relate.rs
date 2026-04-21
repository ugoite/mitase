// FEAT-RELATE-001
// REQ-CORE-023

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Write,
    path::{Component, Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use serde::Serialize;

use crate::{
    cli::{LookupKind, OutputFormat, RelateArgs},
    coverage::normalize_relative_path,
    model::{Feature, Requirement, TraceReference},
    workspace::{Workspace, load_workspace},
};

use super::{
    log::resolve_git_range_changed_files,
    lookup::{EntitySummary, WorkspaceEntity, WorkspaceLookup},
};

#[derive(Debug, Clone, Serialize)]
pub(crate) struct JsonRelateOutput {
    pub(crate) selection: SelectionSummary,
    pub(crate) direct_matches: DirectMatches,
    pub(crate) philosophies: Vec<RelatedNode>,
    pub(crate) policies: Vec<RelatedNode>,
    pub(crate) requirements: Vec<RelatedNode>,
    pub(crate) features: Vec<RelatedNode>,
    pub(crate) traces: Vec<RelatedTrace>,
    pub(crate) gaps: Vec<Gap>,
}

#[derive(Debug, Serialize)]
struct JsonRelateRangeOutput {
    range: String,
    philosophies: Vec<RelatedNode>,
    policies: Vec<RelatedNode>,
    requirements: Vec<RelatedNode>,
    features: Vec<RelatedNode>,
    traces: Vec<RelatedTrace>,
    gaps: Vec<Gap>,
    summary: RelateRangeSummary,
}

#[derive(Debug, Serialize)]
struct RelateRangeSummary {
    total_files: usize,
    total_philosophies: usize,
    total_policies: usize,
    total_requirements: usize,
    total_features: usize,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct SelectionSummary {
    pub(crate) kind: &'static str,
    pub(crate) query: String,
}

#[derive(Debug, Clone, Default, Serialize)]
pub(crate) struct DirectMatches {
    pub(crate) definitions: Vec<RelatedNode>,
    pub(crate) traces: Vec<RelatedTrace>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct RelatedNode {
    pub(crate) kind: &'static str,
    pub(crate) id: String,
    pub(crate) title: String,
    pub(crate) document_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct RelatedTrace {
    pub(crate) owner_kind: &'static str,
    pub(crate) owner_id: String,
    pub(crate) relation_kind: &'static str,
    pub(crate) language: String,
    pub(crate) file: String,
    pub(crate) symbols: Vec<String>,
    pub(crate) direct_match: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct Gap {
    pub(crate) kind: &'static str,
    pub(crate) id: String,
    pub(crate) message: String,
}

#[derive(Debug, Clone)]
struct RelationCatalog {
    philosophies: BTreeMap<String, RelatedNode>,
    policies: BTreeMap<String, RelatedNode>,
    requirements: BTreeMap<String, RelatedNode>,
    features: BTreeMap<String, RelatedNode>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SelectionKind {
    Definition,
    Path,
    Symbol,
}

#[derive(Debug, Clone)]
enum SelectionSource {
    Definition { kind: LookupKind, id: String },
    Path { path: String },
    RangePaths { paths: Vec<String> },
    Symbol { symbol: String },
}

#[derive(Debug, Clone)]
struct SelectionResolution {
    summary: SelectionSummary,
    source: SelectionSource,
    direct_matches: DirectMatches,
    related_ids: RelatedIds,
}

#[derive(Debug, Clone, Default)]
struct RelatedIds {
    philosophies: BTreeSet<String>,
    policies: BTreeSet<String>,
    requirements: BTreeSet<String>,
    features: BTreeSet<String>,
}

pub fn run_relate_command(args: &RelateArgs) -> Result<i32> {
    let workspace = load_workspace(&args.workspace)?;

    if let Some(range) = &args.range {
        return run_relate_range(&workspace, range, args.format);
    }

    let Some(selector) = &args.selector else {
        bail!("either SELECTOR or --range must be provided");
    };

    let report = build_relation_report(&workspace, selector)?;

    match args.format {
        OutputFormat::Text => print!("{}", render_relation_text(&report)),
        OutputFormat::Json => println!(
            "{}",
            serde_json::to_string_pretty(&report)
                .expect("serializing relate output to JSON should succeed")
        ),
    }

    Ok(0)
}

fn run_relate_range(workspace: &Workspace, range: &str, format: OutputFormat) -> Result<i32> {
    let changed_files = resolve_git_range_changed_files(&workspace.root, range)?;

    if changed_files.is_empty() {
        match format {
            OutputFormat::Text => {
                println!("Git range: {range}");
                println!("No files changed in range");
            }
            OutputFormat::Json => {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&JsonRelateRangeOutput {
                        range: range.to_string(),
                        philosophies: Vec::new(),
                        policies: Vec::new(),
                        requirements: Vec::new(),
                        features: Vec::new(),
                        traces: Vec::new(),
                        gaps: Vec::new(),
                        summary: RelateRangeSummary {
                            total_files: 0,
                            total_philosophies: 0,
                            total_policies: 0,
                            total_requirements: 0,
                            total_features: 0,
                        },
                    })
                    .expect("serializing empty relate range output to JSON should succeed")
                );
            }
        }
        return Ok(0);
    }

    let lookup = WorkspaceLookup::new(workspace);
    let catalog = RelationCatalog::load(lookup)?;

    let combined_ids = collect_related_ids_for_changed_files(workspace, &catalog, &changed_files);

    let expanded_ids = expand_related_ids(workspace, combined_ids);

    let report = JsonRelateRangeOutput {
        range: range.to_string(),
        philosophies: catalog.nodes_for(LookupKind::Philosophy, &expanded_ids.philosophies),
        policies: catalog.nodes_for(LookupKind::Policy, &expanded_ids.policies),
        requirements: catalog.nodes_for(LookupKind::Requirement, &expanded_ids.requirements),
        features: catalog.nodes_for(LookupKind::Feature, &expanded_ids.features),
        traces: collect_related_traces(
            workspace,
            &expanded_ids,
            &SelectionSource::RangePaths {
                paths: changed_files
                    .iter()
                    .map(|path| path.display().to_string())
                    .collect(),
            },
        ),
        gaps: collect_gaps(workspace, &expanded_ids),
        summary: RelateRangeSummary {
            total_files: changed_files.len(),
            total_philosophies: expanded_ids.philosophies.len(),
            total_policies: expanded_ids.policies.len(),
            total_requirements: expanded_ids.requirements.len(),
            total_features: expanded_ids.features.len(),
        },
    };

    match format {
        OutputFormat::Text => print!("{}", render_range_relation_text(&report)),
        OutputFormat::Json => println!(
            "{}",
            serde_json::to_string_pretty(&report)
                .expect("serializing relate range output to JSON should succeed")
        ),
    }

    Ok(0)
}

fn collect_related_ids_for_changed_files(
    workspace: &Workspace,
    catalog: &RelationCatalog,
    changed_files: &[PathBuf],
) -> RelatedIds {
    let mut combined_ids = RelatedIds::default();

    for file in changed_files {
        let file_str = file.display().to_string();

        if let Ok(normalized_path) = normalize_selector_path(&workspace.root, &file_str) {
            let definitions = catalog.nodes_matching_path(&normalized_path);
            for node in &definitions {
                combined_ids.add(node.lookup_kind(), &node.id);
            }

            let traces = collect_matching_traces_for_path(workspace, &normalized_path);
            for trace in &traces {
                combined_ids.add(trace.owner_lookup_kind(), &trace.owner_id);
            }
        }
    }

    combined_ids
}
pub(crate) fn build_relation_report(
    workspace: &Workspace,
    selector: &str,
) -> Result<JsonRelateOutput> {
    let lookup = WorkspaceLookup::new(workspace);
    let catalog = RelationCatalog::load(lookup)?;
    let selection = resolve_selection(workspace, lookup, &catalog, selector)?;
    let related_ids = expand_related_ids(workspace, selection.related_ids.clone());

    Ok(JsonRelateOutput {
        selection: selection.summary,
        direct_matches: selection.direct_matches,
        philosophies: catalog.nodes_for(LookupKind::Philosophy, &related_ids.philosophies),
        policies: catalog.nodes_for(LookupKind::Policy, &related_ids.policies),
        requirements: catalog.nodes_for(LookupKind::Requirement, &related_ids.requirements),
        features: catalog.nodes_for(LookupKind::Feature, &related_ids.features),
        traces: collect_related_traces(workspace, &related_ids, &selection.source),
        gaps: collect_gaps(workspace, &related_ids),
    })
}

fn resolve_selection(
    workspace: &Workspace,
    lookup: WorkspaceLookup<'_>,
    catalog: &RelationCatalog,
    selector: &str,
) -> Result<SelectionResolution> {
    if let Some(entity) = lookup.find(selector) {
        return Ok(resolve_definition_selection(catalog, entity));
    }

    if selector_matches_existing_workspace_path(workspace.root.as_path(), selector)? {
        return resolve_path_selection(workspace.root.as_path(), catalog, workspace, selector);
    }

    if is_path_like(selector) {
        return resolve_path_selection(workspace.root.as_path(), catalog, workspace, selector);
    }

    resolve_symbol_selection(workspace, selector)
}

fn resolve_definition_selection(
    catalog: &RelationCatalog,
    entity: WorkspaceEntity<'_>,
) -> SelectionResolution {
    let mut related_ids = RelatedIds::default();
    let node = match entity {
        WorkspaceEntity::Philosophy(item) => {
            related_ids.add(LookupKind::Philosophy, &item.id);
            catalog
                .node(LookupKind::Philosophy, &item.id)
                .expect("philosophy node should exist")
        }
        WorkspaceEntity::Policy(item) => {
            related_ids.add(LookupKind::Policy, &item.id);
            catalog
                .node(LookupKind::Policy, &item.id)
                .expect("policy node should exist")
        }
        WorkspaceEntity::Requirement(item) => {
            related_ids.add(LookupKind::Requirement, &item.id);
            catalog
                .node(LookupKind::Requirement, &item.id)
                .expect("requirement node should exist")
        }
        WorkspaceEntity::Feature(item) => {
            related_ids.add(LookupKind::Feature, &item.id);
            catalog
                .node(LookupKind::Feature, &item.id)
                .expect("feature node should exist")
        }
    };

    SelectionResolution {
        summary: SelectionSummary {
            kind: SelectionKind::Definition.label(),
            query: node.id.clone(),
        },
        source: SelectionSource::Definition {
            kind: node.lookup_kind(),
            id: node.id.clone(),
        },
        direct_matches: DirectMatches {
            definitions: vec![node],
            traces: Vec::new(),
        },
        related_ids,
    }
}

fn resolve_path_selection(
    workspace_root: &Path,
    catalog: &RelationCatalog,
    workspace: &Workspace,
    selector: &str,
) -> Result<SelectionResolution> {
    let normalized_path = normalize_selector_path(workspace_root, selector)?;
    let mut related_ids = RelatedIds::default();
    let definitions = catalog.nodes_matching_path(&normalized_path);
    for node in &definitions {
        related_ids.add(node.lookup_kind(), &node.id);
    }

    let traces = collect_matching_traces_for_path(workspace, &normalized_path);
    for trace in &traces {
        related_ids.add(trace.owner_lookup_kind(), &trace.owner_id);
    }

    if definitions.is_empty() && traces.is_empty() {
        bail!("path `{normalized_path}` is not referenced by the current specification graph");
    }

    Ok(SelectionResolution {
        summary: SelectionSummary {
            kind: SelectionKind::Path.label(),
            query: normalized_path.clone(),
        },
        source: SelectionSource::Path {
            path: normalized_path,
        },
        direct_matches: DirectMatches {
            definitions,
            traces,
        },
        related_ids,
    })
}

fn resolve_symbol_selection(workspace: &Workspace, selector: &str) -> Result<SelectionResolution> {
    let traces = collect_matching_traces_for_symbol(workspace, selector);
    if traces.is_empty() {
        bail!(
            "selector `{selector}` did not match any definition ID, traced path, or traced source symbol"
        );
    }

    let mut related_ids = RelatedIds::default();
    for trace in &traces {
        related_ids.add(trace.owner_lookup_kind(), &trace.owner_id);
    }

    Ok(SelectionResolution {
        summary: SelectionSummary {
            kind: SelectionKind::Symbol.label(),
            query: selector.to_string(),
        },
        source: SelectionSource::Symbol {
            symbol: selector.to_string(),
        },
        direct_matches: DirectMatches {
            definitions: Vec::new(),
            traces,
        },
        related_ids,
    })
}

fn is_path_like(selector: &str) -> bool {
    let path = Path::new(selector);
    selector.contains(std::path::MAIN_SEPARATOR)
        || selector.contains('/')
        || selector.contains('\\')
        || selector.starts_with("./")
        || selector.starts_with("../")
        || path.is_absolute()
        || path.extension().is_some()
}

fn selector_matches_existing_workspace_path(workspace_root: &Path, selector: &str) -> Result<bool> {
    let normalized = normalize_selector_path(workspace_root, selector)?;
    Ok(workspace_root.join(normalized).exists())
}

fn normalize_selector_path(workspace_root: &Path, selector: &str) -> Result<String> {
    let path = Path::new(selector);
    let normalized = if path.is_absolute() {
        let stripped = path.strip_prefix(workspace_root).with_context(|| {
            format!(
                "path selector `{}` must stay inside workspace `{}`",
                path.display(),
                workspace_root.display()
            )
        })?;
        normalize_relative_path(stripped)
    } else {
        normalize_relative_path(path)
    };

    if normalized.as_os_str().is_empty()
        || normalized.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        bail!(
            "path selector `{selector}` must stay inside workspace `{}`",
            workspace_root.display()
        );
    }

    Ok(normalized.display().to_string())
}

fn expand_related_ids(workspace: &Workspace, selected: RelatedIds) -> RelatedIds {
    let mut related = selected.clone();
    expand_upstream_ids(workspace, &selected, &mut related);
    expand_downstream_ids(workspace, &selected, &mut related);
    related
}

fn expand_upstream_ids(workspace: &Workspace, selected: &RelatedIds, related: &mut RelatedIds) {
    let mut feature_frontier = selected.features.clone();
    let mut requirement_frontier = selected.requirements.clone();
    let mut policy_frontier = selected.policies.clone();

    while !(feature_frontier.is_empty()
        && requirement_frontier.is_empty()
        && policy_frontier.is_empty())
    {
        let mut next_requirements = BTreeSet::new();
        for item in &workspace.features {
            if feature_frontier.contains(&item.id) {
                for id in &item.linked_requirements {
                    if related.requirements.insert(id.clone()) {
                        next_requirements.insert(id.clone());
                    }
                }
            }
        }

        let mut next_policies = BTreeSet::new();
        for item in &workspace.requirements {
            if requirement_frontier.contains(&item.id) {
                for id in &item.linked_policies {
                    if related.policies.insert(id.clone()) {
                        next_policies.insert(id.clone());
                    }
                }
            }
        }

        for item in &workspace.policies {
            if policy_frontier.contains(&item.id) {
                for id in &item.linked_philosophies {
                    related.philosophies.insert(id.clone());
                }
            }
        }

        feature_frontier.clear();
        requirement_frontier = next_requirements;
        policy_frontier = next_policies;
    }
}

fn expand_downstream_ids(workspace: &Workspace, selected: &RelatedIds, related: &mut RelatedIds) {
    let mut philosophy_frontier = selected.philosophies.clone();
    let mut policy_frontier = selected.policies.clone();
    let mut requirement_frontier = selected.requirements.clone();

    while !(philosophy_frontier.is_empty()
        && policy_frontier.is_empty()
        && requirement_frontier.is_empty())
    {
        let mut next_policies = BTreeSet::new();
        for item in &workspace.philosophies {
            if philosophy_frontier.contains(&item.id) {
                for id in &item.linked_policies {
                    if related.policies.insert(id.clone()) {
                        next_policies.insert(id.clone());
                    }
                }
            }
        }

        let mut next_requirements = BTreeSet::new();
        for item in &workspace.policies {
            if policy_frontier.contains(&item.id) {
                for id in &item.linked_requirements {
                    if related.requirements.insert(id.clone()) {
                        next_requirements.insert(id.clone());
                    }
                }
            }
        }

        for item in &workspace.requirements {
            if requirement_frontier.contains(&item.id) {
                for id in &item.linked_features {
                    related.features.insert(id.clone());
                }
            }
        }

        philosophy_frontier.clear();
        policy_frontier = next_policies;
        requirement_frontier = next_requirements;
    }
}

fn collect_matching_traces_for_path(workspace: &Workspace, path: &str) -> Vec<RelatedTrace> {
    let mut traces = Vec::new();
    for item in &workspace.requirements {
        traces.extend(collect_owner_traces(
            item,
            "requirement",
            "test",
            path,
            None,
        ));
    }
    for item in &workspace.features {
        traces.extend(collect_owner_traces(
            item,
            "feature",
            "implementation",
            path,
            None,
        ));
    }
    traces
}

fn collect_matching_traces_for_symbol(workspace: &Workspace, symbol: &str) -> Vec<RelatedTrace> {
    let mut traces = Vec::new();
    for item in &workspace.requirements {
        traces.extend(collect_owner_traces(
            item,
            "requirement",
            "test",
            "",
            Some(symbol),
        ));
    }
    for item in &workspace.features {
        traces.extend(collect_owner_traces(
            item,
            "feature",
            "implementation",
            "",
            Some(symbol),
        ));
    }
    traces
}

fn collect_related_traces(
    workspace: &Workspace,
    related_ids: &RelatedIds,
    source: &SelectionSource,
) -> Vec<RelatedTrace> {
    let mut traces = Vec::new();

    for item in &workspace.requirements {
        if related_ids.requirements.contains(&item.id) {
            traces.extend(collect_all_owner_traces(
                item,
                "requirement",
                "test",
                source,
            ));
        }
    }

    for item in &workspace.features {
        if related_ids.features.contains(&item.id) {
            traces.extend(collect_all_owner_traces(
                item,
                "feature",
                "implementation",
                source,
            ));
        }
    }

    traces
}

fn collect_gaps(workspace: &Workspace, related_ids: &RelatedIds) -> Vec<Gap> {
    let mut gaps = Vec::new();

    for item in &workspace.philosophies {
        if related_ids.philosophies.contains(&item.id) && item.linked_policies.is_empty() {
            gaps.push(Gap {
                kind: "philosophy",
                id: item.id.clone(),
                message: format!("philosophy `{}` does not link to any policies", item.id),
            });
        }
    }

    for item in &workspace.policies {
        if !related_ids.policies.contains(&item.id) {
            continue;
        }
        if item.linked_philosophies.is_empty() {
            gaps.push(Gap {
                kind: "policy",
                id: item.id.clone(),
                message: format!("policy `{}` does not link to any philosophies", item.id),
            });
        }
        if item.linked_requirements.is_empty() {
            gaps.push(Gap {
                kind: "policy",
                id: item.id.clone(),
                message: format!("policy `{}` does not link to any requirements", item.id),
            });
        }
    }

    for item in &workspace.requirements {
        if !related_ids.requirements.contains(&item.id) {
            continue;
        }
        if item.linked_policies.is_empty() {
            gaps.push(Gap {
                kind: "requirement",
                id: item.id.clone(),
                message: format!("requirement `{}` does not link to any policies", item.id),
            });
        }
        if item.linked_features.is_empty() {
            gaps.push(Gap {
                kind: "requirement",
                id: item.id.clone(),
                message: format!("requirement `{}` does not link to any features", item.id),
            });
        }
        if item.tests.is_empty() {
            gaps.push(Gap {
                kind: "requirement",
                id: item.id.clone(),
                message: format!("requirement `{}` does not declare any test traces", item.id),
            });
        }
    }

    for item in &workspace.features {
        if !related_ids.features.contains(&item.id) {
            continue;
        }
        if item.linked_requirements.is_empty() {
            gaps.push(Gap {
                kind: "feature",
                id: item.id.clone(),
                message: format!("feature `{}` does not link to any requirements", item.id),
            });
        }
        if item.implementations.is_empty() {
            gaps.push(Gap {
                kind: "feature",
                id: item.id.clone(),
                message: format!(
                    "feature `{}` does not declare any implementation traces",
                    item.id
                ),
            });
        }
    }

    gaps
}

fn render_relation_text(report: &JsonRelateOutput) -> String {
    let mut output = String::new();
    writeln!(
        output,
        "Selection: {} {}",
        report.selection.kind, report.selection.query
    )
    .expect("writing to String must succeed");
    write_node_section(
        &mut output,
        "Direct definition matches",
        &report.direct_matches.definitions,
    );
    write_trace_section(
        &mut output,
        "Direct trace matches",
        &report.direct_matches.traces,
    );
    write_node_section(&mut output, "Philosophies", &report.philosophies);
    write_node_section(&mut output, "Policies", &report.policies);
    write_node_section(&mut output, "Requirements", &report.requirements);
    write_node_section(&mut output, "Features", &report.features);
    write_trace_section(&mut output, "Traces", &report.traces);
    write_gap_section(&mut output, "Gaps", &report.gaps);
    output
}

fn render_range_relation_text(report: &JsonRelateRangeOutput) -> String {
    let mut output = String::new();
    writeln!(output, "Git range: {}", report.range).expect("writing to String must succeed");
    writeln!(output, "Changed files: {}", report.summary.total_files)
        .expect("writing to String must succeed");
    writeln!(output).expect("writing to String must succeed");

    writeln!(output, "Summary:").expect("writing to String must succeed");
    writeln!(
        output,
        "  Philosophies: {}",
        report.summary.total_philosophies
    )
    .expect("writing to String must succeed");
    writeln!(output, "  Policies: {}", report.summary.total_policies)
        .expect("writing to String must succeed");
    writeln!(
        output,
        "  Requirements: {}",
        report.summary.total_requirements
    )
    .expect("writing to String must succeed");
    writeln!(output, "  Features: {}", report.summary.total_features)
        .expect("writing to String must succeed");
    writeln!(output).expect("writing to String must succeed");

    write_node_section(&mut output, "Philosophies", &report.philosophies);
    write_node_section(&mut output, "Policies", &report.policies);
    write_node_section(&mut output, "Requirements", &report.requirements);
    write_node_section(&mut output, "Features", &report.features);
    write_trace_section(&mut output, "Traces", &report.traces);
    write_gap_section(&mut output, "Gaps", &report.gaps);
    output
}

fn write_node_section(output: &mut String, heading: &str, nodes: &[RelatedNode]) {
    writeln!(output, "{heading}:").expect("writing to String must succeed");
    if nodes.is_empty() {
        writeln!(output, "- none").expect("writing to String must succeed");
        return;
    }

    for node in nodes {
        writeln!(
            output,
            "- {} {}\t{}\t({})",
            node.kind, node.id, node.title, node.document_path
        )
        .expect("writing to String must succeed");
    }
}

fn write_trace_section(output: &mut String, heading: &str, traces: &[RelatedTrace]) {
    writeln!(output, "{heading}:").expect("writing to String must succeed");
    if traces.is_empty() {
        writeln!(output, "- none").expect("writing to String must succeed");
        return;
    }

    for trace in traces {
        writeln!(output, "- {}", render_trace_line(trace)).expect("writing to String must succeed");
    }
}

fn write_gap_section(output: &mut String, heading: &str, gaps: &[Gap]) {
    writeln!(output, "{heading}:").expect("writing to String must succeed");
    if gaps.is_empty() {
        writeln!(output, "- none").expect("writing to String must succeed");
        return;
    }

    for gap in gaps {
        writeln!(output, "- {}", gap.message).expect("writing to String must succeed");
    }
}

fn render_trace_line(trace: &RelatedTrace) -> String {
    let mut rendered = format!(
        "{} {} {} {}\t{}",
        trace.owner_kind, trace.owner_id, trace.relation_kind, trace.language, trace.file
    );
    if !trace.symbols.is_empty() {
        write!(
            rendered,
            "\t[{}]",
            trace
                .symbols
                .iter()
                .map(|symbol| format!("`{symbol}`"))
                .collect::<Vec<_>>()
                .join(", ")
        )
        .expect("writing to String must succeed");
    }
    if trace.direct_match {
        write!(rendered, " (direct match)").expect("writing to String must succeed");
    }
    rendered
}

impl RelationCatalog {
    fn load(lookup: WorkspaceLookup<'_>) -> Result<Self> {
        Ok(Self {
            philosophies: collect_node_map(lookup, LookupKind::Philosophy)?,
            policies: collect_node_map(lookup, LookupKind::Policy)?,
            requirements: collect_node_map(lookup, LookupKind::Requirement)?,
            features: collect_node_map(lookup, LookupKind::Feature)?,
        })
    }

    fn node(&self, kind: LookupKind, id: &str) -> Option<RelatedNode> {
        match kind {
            LookupKind::Philosophy => self.philosophies.get(id).cloned(),
            LookupKind::Policy => self.policies.get(id).cloned(),
            LookupKind::Requirement => self.requirements.get(id).cloned(),
            LookupKind::Feature => self.features.get(id).cloned(),
        }
    }

    fn nodes_for(&self, kind: LookupKind, ids: &BTreeSet<String>) -> Vec<RelatedNode> {
        ids.iter().filter_map(|id| self.node(kind, id)).collect()
    }

    fn nodes_matching_path(&self, path: &str) -> Vec<RelatedNode> {
        [
            &self.philosophies,
            &self.policies,
            &self.requirements,
            &self.features,
        ]
        .into_iter()
        .flat_map(|map| map.values())
        .filter(|node| node.document_path == path)
        .cloned()
        .collect()
    }
}

impl RelatedIds {
    fn add(&mut self, kind: LookupKind, id: &str) {
        match kind {
            LookupKind::Philosophy => {
                self.philosophies.insert(id.to_string());
            }
            LookupKind::Policy => {
                self.policies.insert(id.to_string());
            }
            LookupKind::Requirement => {
                self.requirements.insert(id.to_string());
            }
            LookupKind::Feature => {
                self.features.insert(id.to_string());
            }
        }
    }
}

impl SelectionKind {
    const fn label(self) -> &'static str {
        match self {
            Self::Definition => "definition",
            Self::Path => "path",
            Self::Symbol => "symbol",
        }
    }
}

impl RelatedNode {
    fn lookup_kind(&self) -> LookupKind {
        match self.kind {
            "philosophy" => LookupKind::Philosophy,
            "policy" => LookupKind::Policy,
            "requirement" => LookupKind::Requirement,
            "feature" => LookupKind::Feature,
            _ => unreachable!("related node kind should stay within the spec layers"),
        }
    }
}

impl RelatedTrace {
    fn owner_lookup_kind(&self) -> LookupKind {
        match self.owner_kind {
            "requirement" => LookupKind::Requirement,
            "feature" => LookupKind::Feature,
            _ => unreachable!("trace owners are requirements or features"),
        }
    }
}

trait TraceOwner {
    fn owner_id(&self) -> &str;
    fn trace_map(&self, relation_kind: &'static str) -> &BTreeMap<String, Vec<TraceReference>>;
}

impl TraceOwner for Requirement {
    fn owner_id(&self) -> &str {
        &self.id
    }

    fn trace_map(&self, relation_kind: &'static str) -> &BTreeMap<String, Vec<TraceReference>> {
        debug_assert_eq!(relation_kind, "test");
        &self.tests
    }
}

impl TraceOwner for Feature {
    fn owner_id(&self) -> &str {
        &self.id
    }

    fn trace_map(&self, relation_kind: &'static str) -> &BTreeMap<String, Vec<TraceReference>> {
        debug_assert_eq!(relation_kind, "implementation");
        &self.implementations
    }
}

fn collect_owner_traces<T: TraceOwner>(
    owner: &T,
    owner_kind: &'static str,
    relation_kind: &'static str,
    path_match: &str,
    symbol_match: Option<&str>,
) -> Vec<RelatedTrace> {
    let mut traces = Vec::new();
    for (language, references) in owner.trace_map(relation_kind) {
        for reference in references {
            let matches_path =
                !path_match.is_empty() && reference.file.display().to_string() == path_match;
            let matches_symbol = symbol_match
                .map(|symbol| {
                    reference
                        .symbols
                        .iter()
                        .any(|candidate| candidate == symbol)
                })
                .unwrap_or(false);
            if !matches_path && !matches_symbol {
                continue;
            }
            traces.push(RelatedTrace {
                owner_kind,
                owner_id: owner.owner_id().to_string(),
                relation_kind,
                language: language.clone(),
                file: reference.file.display().to_string(),
                symbols: reference.symbols.clone(),
                direct_match: true,
            });
        }
    }
    traces
}

fn collect_all_owner_traces<T: TraceOwner>(
    owner: &T,
    owner_kind: &'static str,
    relation_kind: &'static str,
    source: &SelectionSource,
) -> Vec<RelatedTrace> {
    let mut traces = Vec::new();
    for (language, references) in owner.trace_map(relation_kind) {
        for reference in references {
            traces.push(RelatedTrace {
                owner_kind,
                owner_id: owner.owner_id().to_string(),
                relation_kind,
                language: language.clone(),
                file: reference.file.display().to_string(),
                symbols: reference.symbols.clone(),
                direct_match: trace_is_direct_match(
                    source,
                    owner_kind,
                    owner.owner_id(),
                    reference,
                ),
            });
        }
    }
    traces
}

fn trace_is_direct_match(
    source: &SelectionSource,
    owner_kind: &str,
    owner_id: &str,
    reference: &TraceReference,
) -> bool {
    match source {
        SelectionSource::Definition { kind, id } => {
            *id == owner_id
                && matches!(
                    (*kind, owner_kind),
                    (LookupKind::Requirement, "requirement") | (LookupKind::Feature, "feature")
                )
        }
        SelectionSource::Path { path } => reference.file.display().to_string() == *path,
        SelectionSource::RangePaths { paths } => paths
            .iter()
            .any(|path| reference.file.display().to_string() == *path),
        SelectionSource::Symbol { symbol } => reference
            .symbols
            .iter()
            .any(|candidate| candidate == symbol),
    }
}

fn collect_node_map(
    lookup: WorkspaceLookup<'_>,
    kind: LookupKind,
) -> Result<BTreeMap<String, RelatedNode>> {
    Ok(lookup
        .entries_with_document_paths(kind)?
        .into_iter()
        .filter_map(|entry| {
            let id = entry.id.clone();
            let node = related_node_from_summary(kind, entry)?;
            Some((id, node))
        })
        .collect())
}

fn related_node_from_summary(kind: LookupKind, entry: EntitySummary) -> Option<RelatedNode> {
    Some(RelatedNode {
        kind: kind.label(),
        id: entry.id,
        title: entry.title,
        document_path: entry.document_path?,
    })
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};
    use std::path::{Path, PathBuf};

    use tempfile::tempdir;

    use crate::{
        cli::LookupKind,
        config::SyuConfig,
        coverage::normalize_relative_path,
        model::{Feature, Philosophy, Requirement},
        workspace::Workspace,
    };

    use super::{
        DirectMatches, JsonRelateOutput, RelatedIds, RelatedNode, RelatedTrace, RelationCatalog,
        SelectionKind, SelectionSource, SelectionSummary, collect_gaps, expand_related_ids,
        normalize_selector_path, render_relation_text, render_trace_line,
        resolve_definition_selection, resolve_path_selection, trace_is_direct_match,
    };

    #[test]
    fn normalize_relative_path_removes_current_directory_segments() {
        assert_eq!(
            normalize_relative_path(Path::new("./src/command/relate.rs")),
            Path::new("src/command/relate.rs")
        );
    }

    #[test]
    fn trace_is_direct_match_handles_each_selection_mode() {
        let reference = crate::model::TraceReference {
            file: PathBuf::from("src/command/relate.rs"),
            symbols: vec!["run_relate_command".to_string()],
            doc_contains: Vec::new(),
        };

        assert!(trace_is_direct_match(
            &SelectionSource::Definition {
                kind: LookupKind::Feature,
                id: "FEAT-RELATE-001".to_string(),
            },
            "feature",
            "FEAT-RELATE-001",
            &reference,
        ));
        assert!(trace_is_direct_match(
            &SelectionSource::Path {
                path: "src/command/relate.rs".to_string(),
            },
            "feature",
            "FEAT-RELATE-001",
            &reference,
        ));
        assert!(trace_is_direct_match(
            &SelectionSource::RangePaths {
                paths: vec![
                    "src/command/relate.rs".to_string(),
                    "src/other.rs".to_string()
                ],
            },
            "feature",
            "FEAT-RELATE-001",
            &reference,
        ));
        assert!(trace_is_direct_match(
            &SelectionSource::Symbol {
                symbol: "run_relate_command".to_string(),
            },
            "feature",
            "FEAT-RELATE-001",
            &reference,
        ));
    }

    #[test]
    fn collect_gaps_reports_sparse_requirements() {
        let workspace = Workspace {
            root: PathBuf::from("/tmp/workspace"),
            spec_root: PathBuf::from("/tmp/workspace/docs/syu"),
            config: SyuConfig::default(),
            philosophies: vec![Philosophy {
                id: "PHIL-001".to_string(),
                title: "Philosophy".to_string(),
                product_design_principle: "Principle".to_string(),
                coding_guideline: "Guideline".to_string(),
                linked_policies: Vec::new(),
            }],
            policies: Vec::new(),
            requirements: vec![Requirement {
                id: "REQ-001".to_string(),
                title: "Requirement".to_string(),
                description: "Description".to_string(),
                priority: "medium".to_string(),
                status: "planned".to_string(),
                linked_policies: Vec::new(),
                linked_features: Vec::new(),
                tests: BTreeMap::new(),
            }],
            features: vec![Feature {
                id: "FEAT-001".to_string(),
                title: "Feature".to_string(),
                summary: "Summary".to_string(),
                status: "planned".to_string(),
                linked_requirements: Vec::new(),
                implementations: BTreeMap::new(),
            }],
        };

        let gaps = collect_gaps(
            &workspace,
            &RelatedIds {
                philosophies: BTreeSet::from(["PHIL-001".to_string()]),
                policies: BTreeSet::new(),
                requirements: BTreeSet::from(["REQ-001".to_string()]),
                features: BTreeSet::new(),
            },
        );
        assert!(
            gaps.iter()
                .any(|gap| gap.message.contains("does not link to any policies"))
        );
        assert!(
            gaps.iter()
                .any(|gap| gap.message.contains("does not link to any features"))
        );
        assert!(
            gaps.iter()
                .any(|gap| gap.message.contains("does not declare any test traces"))
        );
    }

    #[test]
    fn render_relation_text_renders_empty_sections() {
        let rendered = render_relation_text(&JsonRelateOutput {
            selection: SelectionSummary {
                kind: SelectionKind::Symbol.label(),
                query: "run_relate_command".to_string(),
            },
            direct_matches: DirectMatches::default(),
            philosophies: Vec::new(),
            policies: Vec::new(),
            requirements: Vec::new(),
            features: Vec::new(),
            traces: Vec::new(),
            gaps: Vec::new(),
        });

        assert!(rendered.contains("Selection: symbol run_relate_command"));
        assert!(rendered.contains("Direct definition matches:\n- none"));
        assert!(rendered.contains("Gaps:\n- none"));
    }

    #[test]
    fn relation_catalog_nodes_matching_path_returns_all_items_from_one_document() {
        let tempdir = tempdir().expect("tempdir should exist");
        let workspace_root = tempdir.path().join("workspace");
        let spec_root = workspace_root.join("docs/syu");
        std::fs::create_dir_all(spec_root.join("philosophy")).expect("philosophy dir");
        std::fs::create_dir_all(spec_root.join("policies")).expect("policies dir");
        std::fs::create_dir_all(spec_root.join("requirements")).expect("requirements dir");
        std::fs::create_dir_all(spec_root.join("features")).expect("features dir");
        std::fs::write(
            spec_root.join("philosophy/foundation.yaml"),
            "category: Philosophy\nversion: 1\nlanguage: en\nphilosophies:\n  - id: PHIL-001\n    title: One\n    product_design_principle: One.\n    coding_guideline: One.\n    linked_policies: []\n",
        )
        .expect("philosophy file");
        std::fs::write(
            spec_root.join("policies/policies.yaml"),
            "category: Policies\nversion: 1\nlanguage: en\npolicies:\n  - id: POL-001\n    title: One\n    summary: One.\n    description: One.\n    linked_philosophies: []\n    linked_requirements: []\n",
        )
        .expect("policy file");
        std::fs::write(
            spec_root.join("requirements/core.yaml"),
            "category: Core\nprefix: REQ\n\nrequirements:\n  - id: REQ-001\n    title: One\n    description: One.\n    priority: medium\n    status: planned\n    linked_policies: []\n    linked_features: []\n    tests: {}\n  - id: REQ-002\n    title: Two\n    description: Two.\n    priority: medium\n    status: planned\n    linked_policies: []\n    linked_features: []\n    tests: {}\n",
        )
        .expect("requirement file");
        std::fs::write(
            spec_root.join("features/features.yaml"),
            "version: \"1\"\nfiles:\n  - kind: demo\n    file: demo.yaml\n",
        )
        .expect("feature registry");
        std::fs::write(
            spec_root.join("features/demo.yaml"),
            "category: Demo\nversion: 1\nfeatures:\n  - id: FEAT-001\n    title: Feature\n    summary: Feature.\n    status: planned\n    linked_requirements: []\n    implementations: {}\n",
        )
        .expect("feature file");

        let workspace = crate::workspace::load_workspace(&workspace_root).expect("workspace");
        let catalog =
            RelationCatalog::load(crate::command::lookup::WorkspaceLookup::new(&workspace))
                .expect("catalog");
        let matches = catalog.nodes_matching_path("docs/syu/requirements/core.yaml");
        assert_eq!(matches.len(), 2);
        assert!(matches.iter().any(|node| node.id == "REQ-001"));
        assert!(matches.iter().any(|node| node.id == "REQ-002"));
    }

    #[test]
    fn resolve_definition_selection_covers_each_layer_kind() {
        let workspace = demo_workspace();
        let catalog = demo_catalog();
        let lookup = crate::command::lookup::WorkspaceLookup::new(&workspace);

        let philosophy =
            resolve_definition_selection(&catalog, lookup.find("PHIL-001").expect("philosophy"));
        assert_eq!(philosophy.summary.kind, "definition");
        assert!(philosophy.related_ids.philosophies.contains("PHIL-001"));

        let policy =
            resolve_definition_selection(&catalog, lookup.find("POL-001").expect("policy"));
        assert!(policy.related_ids.policies.contains("POL-001"));

        let requirement =
            resolve_definition_selection(&catalog, lookup.find("REQ-001").expect("requirement"));
        assert!(requirement.related_ids.requirements.contains("REQ-001"));

        let feature =
            resolve_definition_selection(&catalog, lookup.find("FEAT-001").expect("feature"));
        assert!(feature.related_ids.features.contains("FEAT-001"));
    }

    #[test]
    fn resolve_path_selection_collects_definition_and_trace_matches() {
        let workspace = demo_workspace();
        let catalog = demo_catalog();

        let definition_selection = resolve_path_selection(
            workspace.root.as_path(),
            &catalog,
            &workspace,
            "docs/syu/requirements/core.yaml",
        )
        .expect("definition path should resolve");
        assert_eq!(definition_selection.direct_matches.definitions.len(), 1);
        assert!(
            definition_selection
                .related_ids
                .requirements
                .contains("REQ-001")
        );

        let trace_selection = resolve_path_selection(
            workspace.root.as_path(),
            &catalog,
            &workspace,
            "src/feature.rs",
        )
        .expect("trace path should resolve");
        assert_eq!(trace_selection.direct_matches.traces.len(), 1);
        assert!(trace_selection.related_ids.features.contains("FEAT-001"));
    }

    #[test]
    fn resolve_path_selection_rejects_unreferenced_paths() {
        let workspace = demo_workspace();
        let catalog = demo_catalog();

        let error = resolve_path_selection(
            workspace.root.as_path(),
            &catalog,
            &workspace,
            "src/missing.rs",
        )
        .expect_err("unreferenced paths should be rejected");
        assert!(
            error
                .to_string()
                .contains("is not referenced by the current specification graph")
        );
    }

    #[test]
    fn normalize_selector_path_handles_absolute_workspace_paths() {
        let normalized = normalize_selector_path(Path::new("/repo"), "/repo/src/command/relate.rs")
            .expect("absolute workspace path should normalize");
        assert_eq!(normalized, "src/command/relate.rs");

        let error = normalize_selector_path(Path::new("/repo"), "/outside/relate.rs")
            .expect_err("outside absolute path should fail");
        assert!(error.to_string().contains("must stay inside workspace"));
    }

    #[test]
    fn normalize_selector_path_rejects_parent_directory_segments() {
        let error = normalize_selector_path(Path::new("/repo"), "../src/command/relate.rs")
            .expect_err("parent directories should be rejected");
        assert!(error.to_string().contains("must stay inside workspace"));
    }

    #[test]
    fn expand_related_ids_walks_the_connected_component() {
        let workspace = demo_workspace();
        let related = expand_related_ids(
            &workspace,
            RelatedIds {
                philosophies: BTreeSet::from(["PHIL-001".to_string()]),
                policies: BTreeSet::new(),
                requirements: BTreeSet::new(),
                features: BTreeSet::new(),
            },
        );

        assert!(related.policies.contains("POL-001"));
        assert!(related.requirements.contains("REQ-001"));
        assert!(related.features.contains("FEAT-001"));
    }

    #[test]
    fn expand_related_ids_does_not_pull_in_sibling_branches() {
        let mut workspace = demo_workspace();
        workspace.policies[0]
            .linked_requirements
            .push("REQ-002".to_string());
        workspace.requirements.push(Requirement {
            id: "REQ-002".to_string(),
            title: "Sibling requirement".to_string(),
            description: "Description".to_string(),
            priority: "medium".to_string(),
            status: "implemented".to_string(),
            linked_policies: vec!["POL-001".to_string()],
            linked_features: vec!["FEAT-002".to_string()],
            tests: BTreeMap::new(),
        });
        workspace.features.push(Feature {
            id: "FEAT-002".to_string(),
            title: "Sibling feature".to_string(),
            summary: "Summary".to_string(),
            status: "implemented".to_string(),
            linked_requirements: vec!["REQ-002".to_string()],
            implementations: BTreeMap::new(),
        });

        let related = expand_related_ids(
            &workspace,
            RelatedIds {
                philosophies: BTreeSet::new(),
                policies: BTreeSet::new(),
                requirements: BTreeSet::from(["REQ-001".to_string()]),
                features: BTreeSet::new(),
            },
        );

        assert!(related.philosophies.contains("PHIL-001"));
        assert!(related.policies.contains("POL-001"));
        assert!(related.features.contains("FEAT-001"));
        assert!(!related.requirements.contains("REQ-002"));
        assert!(!related.features.contains("FEAT-002"));
    }

    #[test]
    fn collect_gaps_reports_sparse_policy_and_feature() {
        let workspace = demo_workspace();
        let gaps = collect_gaps(
            &workspace,
            &RelatedIds {
                philosophies: BTreeSet::new(),
                policies: BTreeSet::from(["POL-EMPTY".to_string()]),
                requirements: BTreeSet::new(),
                features: BTreeSet::from(["FEAT-EMPTY".to_string()]),
            },
        );

        assert!(
            gaps.iter()
                .any(|gap| gap.message.contains("does not link to any philosophies"))
        );
        assert!(
            gaps.iter()
                .any(|gap| gap.message.contains("does not link to any requirements"))
        );
        assert!(gaps.iter().any(|gap| {
            gap.message
                .contains("does not declare any implementation traces")
        }));
    }

    #[test]
    fn render_trace_line_marks_direct_matches() {
        let rendered = render_trace_line(&RelatedTrace {
            owner_kind: "feature",
            owner_id: "FEAT-001".to_string(),
            relation_kind: "implementation",
            language: "rust".to_string(),
            file: "src/feature.rs".to_string(),
            symbols: vec!["feature_symbol".to_string()],
            direct_match: true,
        });

        assert!(rendered.contains("feature FEAT-001 implementation rust\tsrc/feature.rs"));
        assert!(rendered.contains("[`feature_symbol`]"));
        assert!(rendered.contains("(direct match)"));
    }

    #[test]
    fn render_trace_line_handles_empty_symbols() {
        let rendered = render_trace_line(&RelatedTrace {
            owner_kind: "feature",
            owner_id: "FEAT-001".to_string(),
            relation_kind: "implementation",
            language: "rust".to_string(),
            file: "src/feature.rs".to_string(),
            symbols: Vec::new(),
            direct_match: false,
        });

        assert!(!rendered.contains('['));
        assert!(!rendered.contains("direct match"));
    }

    #[test]
    fn lookup_kind_helpers_cover_all_variants() {
        let node = RelatedNode {
            kind: "feature",
            id: "FEAT-001".to_string(),
            title: "Feature".to_string(),
            document_path: "docs/syu/features/demo.yaml".to_string(),
        };
        assert_eq!(node.lookup_kind(), LookupKind::Feature);

        let trace = RelatedTrace {
            owner_kind: "requirement",
            owner_id: "REQ-001".to_string(),
            relation_kind: "test",
            language: "rust".to_string(),
            file: "src/tests.rs".to_string(),
            symbols: vec!["req_test".to_string()],
            direct_match: false,
        };
        assert_eq!(trace.owner_lookup_kind(), LookupKind::Requirement);
    }

    #[test]
    fn lookup_kind_helpers_panic_for_invalid_kinds() {
        let invalid_node = RelatedNode {
            kind: "unknown",
            id: "X".to_string(),
            title: "Unknown".to_string(),
            document_path: "docs/unknown.yaml".to_string(),
        };
        assert!(std::panic::catch_unwind(|| invalid_node.lookup_kind()).is_err());

        let invalid_trace = RelatedTrace {
            owner_kind: "unknown",
            owner_id: "X".to_string(),
            relation_kind: "implementation",
            language: "rust".to_string(),
            file: "src/unknown.rs".to_string(),
            symbols: Vec::new(),
            direct_match: false,
        };
        assert!(std::panic::catch_unwind(|| invalid_trace.owner_lookup_kind()).is_err());
    }

    #[test]
    fn run_relate_command_requires_a_selector_or_range() {
        let error = super::run_relate_command(&crate::cli::RelateArgs {
            selector: None,
            workspace: PathBuf::from("."),
            range: None,
            format: crate::cli::OutputFormat::Text,
        })
        .expect_err("missing selector and range should fail");

        assert!(
            error
                .to_string()
                .contains("either SELECTOR or --range must be provided")
        );
    }

    #[test]
    fn run_relate_range_collects_definition_ids_from_changed_documents() {
        let workspace_root =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/workspaces/passing");
        let workspace = crate::workspace::load_workspace(&workspace_root).expect("workspace");
        let lookup = crate::command::lookup::WorkspaceLookup::new(&workspace);
        let document_path = lookup
            .document_path_for_id("FEAT-TRACE-001")
            .expect("feature path lookup")
            .expect("feature document path");
        let catalog = RelationCatalog::load(lookup).expect("catalog");
        let related_ids = super::collect_related_ids_for_changed_files(
            &workspace,
            &catalog,
            &[workspace_root.join(document_path)],
        );

        assert!(related_ids.features.contains("FEAT-TRACE-001"));
    }

    fn demo_workspace() -> Workspace {
        let mut requirement_tests = BTreeMap::new();
        requirement_tests.insert(
            "rust".to_string(),
            vec![crate::model::TraceReference {
                file: PathBuf::from("src/requirement.rs"),
                symbols: vec!["requirement_symbol".to_string()],
                doc_contains: Vec::new(),
            }],
        );

        let mut feature_impls = BTreeMap::new();
        feature_impls.insert(
            "rust".to_string(),
            vec![crate::model::TraceReference {
                file: PathBuf::from("src/feature.rs"),
                symbols: vec!["feature_symbol".to_string()],
                doc_contains: Vec::new(),
            }],
        );

        Workspace {
            root: PathBuf::from("/repo"),
            spec_root: PathBuf::from("/repo/docs/syu"),
            config: SyuConfig::default(),
            philosophies: vec![Philosophy {
                id: "PHIL-001".to_string(),
                title: "Philosophy".to_string(),
                product_design_principle: "Principle".to_string(),
                coding_guideline: "Guideline".to_string(),
                linked_policies: vec!["POL-001".to_string()],
            }],
            policies: vec![
                crate::model::Policy {
                    id: "POL-001".to_string(),
                    title: "Policy".to_string(),
                    summary: "Summary".to_string(),
                    description: "Description".to_string(),
                    linked_philosophies: vec!["PHIL-001".to_string()],
                    linked_requirements: vec!["REQ-001".to_string()],
                },
                crate::model::Policy {
                    id: "POL-EMPTY".to_string(),
                    title: "Empty policy".to_string(),
                    summary: "Summary".to_string(),
                    description: "Description".to_string(),
                    linked_philosophies: Vec::new(),
                    linked_requirements: Vec::new(),
                },
            ],
            requirements: vec![Requirement {
                id: "REQ-001".to_string(),
                title: "Requirement".to_string(),
                description: "Description".to_string(),
                priority: "medium".to_string(),
                status: "implemented".to_string(),
                linked_policies: vec!["POL-001".to_string()],
                linked_features: vec!["FEAT-001".to_string()],
                tests: requirement_tests,
            }],
            features: vec![
                Feature {
                    id: "FEAT-001".to_string(),
                    title: "Feature".to_string(),
                    summary: "Summary".to_string(),
                    status: "implemented".to_string(),
                    linked_requirements: vec!["REQ-001".to_string()],
                    implementations: feature_impls,
                },
                Feature {
                    id: "FEAT-EMPTY".to_string(),
                    title: "Empty feature".to_string(),
                    summary: "Summary".to_string(),
                    status: "planned".to_string(),
                    linked_requirements: Vec::new(),
                    implementations: BTreeMap::new(),
                },
            ],
        }
    }

    fn demo_catalog() -> RelationCatalog {
        RelationCatalog {
            philosophies: BTreeMap::from([(
                "PHIL-001".to_string(),
                RelatedNode {
                    kind: "philosophy",
                    id: "PHIL-001".to_string(),
                    title: "Philosophy".to_string(),
                    document_path: "docs/syu/philosophy/foundation.yaml".to_string(),
                },
            )]),
            policies: BTreeMap::from([(
                "POL-001".to_string(),
                RelatedNode {
                    kind: "policy",
                    id: "POL-001".to_string(),
                    title: "Policy".to_string(),
                    document_path: "docs/syu/policies/policies.yaml".to_string(),
                },
            )]),
            requirements: BTreeMap::from([(
                "REQ-001".to_string(),
                RelatedNode {
                    kind: "requirement",
                    id: "REQ-001".to_string(),
                    title: "Requirement".to_string(),
                    document_path: "docs/syu/requirements/core.yaml".to_string(),
                },
            )]),
            features: BTreeMap::from([(
                "FEAT-001".to_string(),
                RelatedNode {
                    kind: "feature",
                    id: "FEAT-001".to_string(),
                    title: "Feature".to_string(),
                    document_path: "docs/syu/features/demo.yaml".to_string(),
                },
            )]),
        }
    }
}
