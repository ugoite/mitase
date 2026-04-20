// FEAT-TRACE-001
// REQ-CORE-021

use std::{
    collections::BTreeMap,
    fmt::Write as _,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use serde::Serialize;

use crate::{
    cli::{LookupKind, OutputFormat, TraceArgs},
    coverage::normalize_relative_path,
    model::{Feature, Requirement, TraceReference},
    workspace::{Workspace, load_workspace},
};

use super::{
    log::resolve_git_range_changed_files,
    lookup::{EntitySummary, WorkspaceLookup},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
enum TraceLookupStatus {
    Owned,
    Partial,
    Unowned,
}

impl TraceLookupStatus {
    const fn label(self) -> &'static str {
        match self {
            Self::Owned => "owned",
            Self::Partial => "partial",
            Self::Unowned => "unowned",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MatchMode {
    File,
    Symbol,
    Wildcard,
}

impl MatchMode {
    const fn label(self) -> &'static str {
        match self {
            Self::File => "file",
            Self::Symbol => "symbol",
            Self::Wildcard => "wildcard",
        }
    }

    fn from_label(label: &str) -> Self {
        match label {
            "symbol" => Self::Symbol,
            "wildcard" => Self::Wildcard,
            _ => Self::File,
        }
    }

    fn matched_label(self, symbol: Option<&str>) -> String {
        match self {
            Self::File => "file".to_string(),
            Self::Symbol => format!("symbol `{}`", symbol.expect("symbol match should exist")),
            Self::Wildcard => "wildcard `*`".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq, PartialOrd, Ord)]
struct TraceOwnerMatch {
    kind: &'static str,
    id: String,
    title: String,
    trace_role: String,
    language: String,
    file: String,
    declared_symbols: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    matched_symbol: Option<String>,
    match_mode: &'static str,
}

#[derive(Debug, Clone, Serialize)]
struct TraceLookupOutput {
    file: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    symbol: Option<String>,
    status: TraceLookupStatus,
    matched_owners: Vec<TraceOwnerMatch>,
    file_only_owners: Vec<TraceOwnerMatch>,
    requirements: Vec<EntitySummary>,
    features: Vec<EntitySummary>,
    policies: Vec<EntitySummary>,
    philosophies: Vec<EntitySummary>,
}

#[derive(Debug, Clone, Serialize)]
struct TraceRangeOutput {
    range: String,
    files: Vec<TraceLookupOutput>,
    summary: TraceRangeSummary,
}

#[derive(Debug, Clone, Serialize)]
struct TraceRangeSummary {
    total_files: usize,
    owned_files: usize,
    partial_files: usize,
    unowned_files: usize,
    total_requirements: usize,
    total_features: usize,
    total_policies: usize,
    total_philosophies: usize,
}

#[derive(Debug, Clone, Copy)]
struct TraceOwnerMetadata<'a> {
    kind: LookupKind,
    id: &'a str,
    title: &'a str,
    trace_role: &'a str,
}

pub fn run_trace_command(args: &TraceArgs) -> Result<i32> {
    if let Some(range) = &args.range {
        let workspace = load_workspace(&args.workspace)?;
        if args.symbol.is_some() {
            bail!("--symbol cannot be used with --range");
        }
        return run_trace_range(&workspace, range, args.format);
    }

    let Some(file) = &args.file else {
        bail!("either FILE or --range must be provided");
    };

    let symbol = match args.symbol.as_deref() {
        Some(symbol) if symbol.trim().is_empty() => {
            bail!("trace symbol must not be empty or whitespace");
        }
        Some(symbol) => Some(symbol.trim()),
        None => None,
    };

    let workspace = load_workspace(&args.workspace)?;
    let normalized_file = normalize_lookup_file(&workspace, file)?;
    let output = lookup_trace(&workspace, &normalized_file, symbol);

    match args.format {
        OutputFormat::Text => print!("{}", render_text_output(&output)),
        OutputFormat::Json => println!(
            "{}",
            serde_json::to_string_pretty(&output)
                .expect("serializing trace lookup output to JSON should succeed")
        ),
    }

    Ok(0)
}

fn run_trace_range(workspace: &Workspace, range: &str, format: OutputFormat) -> Result<i32> {
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
                    serde_json::to_string_pretty(&TraceRangeOutput {
                        range: range.to_string(),
                        files: Vec::new(),
                        summary: TraceRangeSummary {
                            total_files: 0,
                            owned_files: 0,
                            partial_files: 0,
                            unowned_files: 0,
                            total_requirements: 0,
                            total_features: 0,
                            total_policies: 0,
                            total_philosophies: 0,
                        },
                    })
                    .expect("serializing empty trace range output to JSON should succeed")
                );
            }
        }
        return Ok(0);
    }

    let mut results = Vec::new();
    for file in &changed_files {
        match normalize_lookup_file(workspace, file) {
            Ok(normalized) => {
                let output = lookup_trace(workspace, &normalized, None);
                results.push(output);
            }
            Err(_) => {
                // Skip files outside workspace or with invalid paths
                continue;
            }
        }
    }

    let summary = compute_range_summary(&results);

    match format {
        OutputFormat::Text => print!("{}", render_range_text(range, &results, &summary)),
        OutputFormat::Json => println!(
            "{}",
            serde_json::to_string_pretty(&TraceRangeOutput {
                range: range.to_string(),
                files: results,
                summary,
            })
            .expect("serializing trace range output to JSON should succeed")
        ),
    }

    Ok(0)
}

fn compute_range_summary(results: &[TraceLookupOutput]) -> TraceRangeSummary {
    let mut requirements = std::collections::BTreeSet::new();
    let mut features = std::collections::BTreeSet::new();
    let mut policies = std::collections::BTreeSet::new();
    let mut philosophies = std::collections::BTreeSet::new();

    let mut owned = 0;
    let mut partial = 0;
    let mut unowned = 0;

    for result in results {
        match result.status {
            TraceLookupStatus::Owned => owned += 1,
            TraceLookupStatus::Partial => partial += 1,
            TraceLookupStatus::Unowned => unowned += 1,
        }

        for req in &result.requirements {
            requirements.insert(&req.id);
        }
        for feat in &result.features {
            features.insert(&feat.id);
        }
        for pol in &result.policies {
            policies.insert(&pol.id);
        }
        for phil in &result.philosophies {
            philosophies.insert(&phil.id);
        }
    }

    TraceRangeSummary {
        total_files: results.len(),
        owned_files: owned,
        partial_files: partial,
        unowned_files: unowned,
        total_requirements: requirements.len(),
        total_features: features.len(),
        total_policies: policies.len(),
        total_philosophies: philosophies.len(),
    }
}

fn render_range_text(
    range: &str,
    results: &[TraceLookupOutput],
    summary: &TraceRangeSummary,
) -> String {
    let mut output = String::new();
    writeln!(output, "Git range: {range}").unwrap();
    writeln!(output, "Changed files: {}", summary.total_files).unwrap();
    writeln!(
        output,
        "Coverage: {} owned, {} partial, {} unowned\n",
        summary.owned_files, summary.partial_files, summary.unowned_files
    )
    .unwrap();

    let mut by_owner = BTreeMap::<String, Vec<&TraceLookupOutput>>::new();
    for result in results {
        if result.matched_owners.is_empty() && result.file_only_owners.is_empty() {
            by_owner
                .entry("UNOWNED".to_string())
                .or_default()
                .push(result);
        } else {
            let owners = if !result.matched_owners.is_empty() {
                &result.matched_owners
            } else {
                &result.file_only_owners
            };
            for owner in owners {
                by_owner
                    .entry(format!("{} {}", owner.kind, owner.id))
                    .or_default()
                    .push(result);
            }
        }
    }

    for (owner, files) in &by_owner {
        writeln!(output, "{owner}:").unwrap();
        for file in files {
            writeln!(output, "  - {}", file.file).unwrap();
        }
        writeln!(output).unwrap();
    }

    writeln!(output, "Summary:").unwrap();
    if summary.total_requirements > 0 {
        writeln!(output, "  Requirements: {}", summary.total_requirements).unwrap();
    }
    if summary.total_features > 0 {
        writeln!(output, "  Features: {}", summary.total_features).unwrap();
    }
    if summary.total_policies > 0 {
        writeln!(output, "  Policies: {}", summary.total_policies).unwrap();
    }
    if summary.total_philosophies > 0 {
        writeln!(output, "  Philosophies: {}", summary.total_philosophies).unwrap();
    }

    output
}

fn normalize_lookup_file(workspace: &Workspace, file: &Path) -> Result<PathBuf> {
    let relative = if file.is_absolute() {
        file.strip_prefix(&workspace.root)
            .with_context(|| {
                format!(
                    "trace file `{}` must stay under workspace `{}`",
                    file.display(),
                    workspace.root.display()
                )
            })?
            .to_path_buf()
    } else {
        file.to_path_buf()
    };
    let normalized = normalize_relative_path(&relative);
    if normalized.as_os_str().is_empty() {
        bail!("trace file must not be empty");
    }
    if normalized
        .components()
        .any(|component| matches!(component, std::path::Component::ParentDir))
    {
        bail!(
            "trace file `{}` must stay under workspace `{}`",
            file.display(),
            workspace.root.display()
        );
    }
    Ok(normalized)
}

fn lookup_trace(workspace: &Workspace, file: &Path, symbol: Option<&str>) -> TraceLookupOutput {
    let lookup = WorkspaceLookup::new(workspace);
    let file_label = file.display().to_string();
    let mut matched_owners = Vec::new();
    let mut file_only_owners = Vec::new();

    collect_requirement_matches(
        &workspace.requirements,
        &file_label,
        file,
        symbol,
        &mut matched_owners,
        &mut file_only_owners,
    );
    collect_feature_matches(
        &workspace.features,
        &file_label,
        file,
        symbol,
        &mut matched_owners,
        &mut file_only_owners,
    );

    matched_owners.sort();
    matched_owners.dedup();
    file_only_owners.sort();
    file_only_owners.dedup();

    let context_matches = if matched_owners.is_empty() {
        &file_only_owners
    } else {
        &matched_owners
    };
    let related = collect_related_entities(lookup, context_matches);

    TraceLookupOutput {
        file: file_label,
        symbol: symbol.map(ToString::to_string),
        status: lookup_status(&matched_owners, &file_only_owners),
        matched_owners,
        file_only_owners,
        requirements: related.requirements,
        features: related.features,
        policies: related.policies,
        philosophies: related.philosophies,
    }
}

fn collect_requirement_matches(
    requirements: &[Requirement],
    file_label: &str,
    file: &Path,
    symbol: Option<&str>,
    matched_owners: &mut Vec<TraceOwnerMatch>,
    file_only_owners: &mut Vec<TraceOwnerMatch>,
) {
    for requirement in requirements {
        collect_trace_matches(
            TraceOwnerMetadata {
                kind: LookupKind::Requirement,
                id: &requirement.id,
                title: &requirement.title,
                trace_role: "test",
            },
            &requirement.tests,
            file_label,
            file,
            symbol,
            matched_owners,
            file_only_owners,
        );
    }
}

fn collect_feature_matches(
    features: &[Feature],
    file_label: &str,
    file: &Path,
    symbol: Option<&str>,
    matched_owners: &mut Vec<TraceOwnerMatch>,
    file_only_owners: &mut Vec<TraceOwnerMatch>,
) {
    for feature in features {
        collect_trace_matches(
            TraceOwnerMetadata {
                kind: LookupKind::Feature,
                id: &feature.id,
                title: &feature.title,
                trace_role: "implementation",
            },
            &feature.implementations,
            file_label,
            file,
            symbol,
            matched_owners,
            file_only_owners,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn collect_trace_matches(
    owner: TraceOwnerMetadata<'_>,
    references: &BTreeMap<String, Vec<TraceReference>>,
    file_label: &str,
    file: &Path,
    symbol: Option<&str>,
    matched_owners: &mut Vec<TraceOwnerMatch>,
    file_only_owners: &mut Vec<TraceOwnerMatch>,
) {
    for (language, items) in references {
        for reference in items {
            if normalize_relative_path(&reference.file) != file {
                continue;
            }

            if let Some(mode) = match_mode(reference, symbol) {
                matched_owners.push(trace_owner_match(
                    owner, language, file_label, reference, mode, symbol,
                ));
            } else if symbol.is_some() {
                file_only_owners.push(trace_owner_match(
                    owner,
                    language,
                    file_label,
                    reference,
                    MatchMode::File,
                    None,
                ));
            }
        }
    }
}

fn trace_owner_match(
    owner: TraceOwnerMetadata<'_>,
    language: &str,
    file_label: &str,
    reference: &TraceReference,
    match_mode: MatchMode,
    symbol: Option<&str>,
) -> TraceOwnerMatch {
    let matched_symbol = match match_mode {
        MatchMode::Symbol => symbol.map(ToString::to_string),
        MatchMode::Wildcard => Some("*".to_string()),
        MatchMode::File => None,
    };

    TraceOwnerMatch {
        kind: owner.kind.label(),
        id: owner.id.to_string(),
        title: owner.title.to_string(),
        trace_role: owner.trace_role.to_string(),
        language: language.to_string(),
        file: file_label.to_string(),
        declared_symbols: reference.symbols.clone(),
        matched_symbol,
        match_mode: match_mode.label(),
    }
}

fn match_mode(reference: &TraceReference, symbol: Option<&str>) -> Option<MatchMode> {
    let Some(symbol) = symbol else {
        return Some(MatchMode::File);
    };

    if reference.symbols.iter().any(|candidate| candidate == "*") {
        return Some(MatchMode::Wildcard);
    }

    if reference
        .symbols
        .iter()
        .any(|candidate| candidate == symbol)
    {
        return Some(MatchMode::Symbol);
    }

    None
}

fn lookup_status(
    matched_owners: &[TraceOwnerMatch],
    file_only_owners: &[TraceOwnerMatch],
) -> TraceLookupStatus {
    if !matched_owners.is_empty() {
        TraceLookupStatus::Owned
    } else if !file_only_owners.is_empty() {
        TraceLookupStatus::Partial
    } else {
        TraceLookupStatus::Unowned
    }
}

struct RelatedEntities {
    requirements: Vec<EntitySummary>,
    features: Vec<EntitySummary>,
    policies: Vec<EntitySummary>,
    philosophies: Vec<EntitySummary>,
}

fn collect_related_entities(
    lookup: WorkspaceLookup<'_>,
    matches: &[TraceOwnerMatch],
) -> RelatedEntities {
    let mut requirements = BTreeMap::new();
    let mut features = BTreeMap::new();
    let mut policies = BTreeMap::new();
    let mut philosophies = BTreeMap::new();

    for owner in matches {
        match owner.kind {
            "requirement" => {
                insert_summary(
                    &mut requirements,
                    lookup,
                    LookupKind::Requirement,
                    &owner.id,
                );
                if let Some(requirement) = lookup.requirement(&owner.id) {
                    for feature_id in &requirement.linked_features {
                        insert_summary(&mut features, lookup, LookupKind::Feature, feature_id);
                    }
                    collect_requirement_context(
                        lookup,
                        requirement,
                        &mut policies,
                        &mut philosophies,
                    );
                }
            }
            "feature" => {
                insert_summary(&mut features, lookup, LookupKind::Feature, &owner.id);
                if let Some(feature) = lookup.feature(&owner.id) {
                    for requirement_id in &feature.linked_requirements {
                        insert_summary(
                            &mut requirements,
                            lookup,
                            LookupKind::Requirement,
                            requirement_id,
                        );
                        if let Some(requirement) = lookup.requirement(requirement_id) {
                            collect_requirement_context(
                                lookup,
                                requirement,
                                &mut policies,
                                &mut philosophies,
                            );
                        }
                    }
                }
            }
            _ => {}
        }
    }

    RelatedEntities {
        requirements: requirements.into_values().collect(),
        features: features.into_values().collect(),
        policies: policies.into_values().collect(),
        philosophies: philosophies.into_values().collect(),
    }
}

fn collect_requirement_context(
    lookup: WorkspaceLookup<'_>,
    requirement: &Requirement,
    policies: &mut BTreeMap<String, EntitySummary>,
    philosophies: &mut BTreeMap<String, EntitySummary>,
) {
    for policy_id in &requirement.linked_policies {
        insert_summary(policies, lookup, LookupKind::Policy, policy_id);
        if let Some(policy) = lookup.policy(policy_id) {
            for philosophy_id in &policy.linked_philosophies {
                insert_summary(philosophies, lookup, LookupKind::Philosophy, philosophy_id);
            }
        }
    }
}

fn insert_summary(
    map: &mut BTreeMap<String, EntitySummary>,
    lookup: WorkspaceLookup<'_>,
    kind: LookupKind,
    id: &str,
) {
    let Some(title) = lookup.title_for(kind, id) else {
        return;
    };
    map.entry(id.to_string()).or_insert_with(|| EntitySummary {
        id: id.to_string(),
        title: title.to_string(),
        document_path: None,
    });
}

fn render_text_output(output: &TraceLookupOutput) -> String {
    let mut rendered = String::new();

    writeln!(&mut rendered, "File: {}", output.file).expect("writing to a string should succeed");
    if let Some(symbol) = &output.symbol {
        writeln!(&mut rendered, "Symbol: {symbol}").expect("writing to a string should succeed");
    }
    writeln!(&mut rendered, "Status: {}", output.status.label())
        .expect("writing to a string should succeed");

    match output.status {
        TraceLookupStatus::Owned => {
            writeln!(&mut rendered, "Matched trace owners:")
                .expect("writing to a string should succeed");
            for owner in &output.matched_owners {
                push_owner_match(&mut rendered, owner);
            }
        }
        TraceLookupStatus::Partial => {
            let symbol = output
                .symbol
                .as_deref()
                .expect("partial trace lookups should include a symbol");
            writeln!(&mut rendered, "No trace owners matched symbol `{symbol}`.")
                .expect("writing to a string should succeed");
            writeln!(&mut rendered, "File owners without a matching symbol:")
                .expect("writing to a string should succeed");
            for owner in &output.file_only_owners {
                push_owner_match(&mut rendered, owner);
            }
            writeln!(
                &mut rendered,
                "Hint: Trace the symbol explicitly in the matching requirement or feature, or use `*` when the whole file belongs to one owner."
            )
            .expect("writing to a string should succeed");
        }
        TraceLookupStatus::Unowned => {
            writeln!(
                &mut rendered,
                "No requirement or feature traces reference `{}`.",
                query_label(output)
            )
            .expect("writing to a string should succeed");
            writeln!(
                &mut rendered,
                "Hint: Add the file to a requirement test trace or feature implementation trace, then rerun `syu validate . --genre trace`."
            )
            .expect("writing to a string should succeed");
            return rendered;
        }
    }

    push_entity_section(&mut rendered, "Requirements", &output.requirements);
    push_entity_section(&mut rendered, "Features", &output.features);
    push_entity_section(&mut rendered, "Policies", &output.policies);
    push_entity_section(&mut rendered, "Philosophies", &output.philosophies);
    rendered
}

fn push_owner_match(rendered: &mut String, owner: &TraceOwnerMatch) {
    let matched_by =
        MatchMode::from_label(owner.match_mode).matched_label(owner.matched_symbol.as_deref());
    writeln!(
        rendered,
        "- {} {}\t{} ({}, {}, matched by {})",
        owner.kind, owner.id, owner.title, owner.language, owner.trace_role, matched_by
    )
    .expect("writing to a string should succeed");
    if !owner.declared_symbols.is_empty() {
        writeln!(
            rendered,
            "  declared symbols: {}",
            owner.declared_symbols.join(", ")
        )
        .expect("writing to a string should succeed");
    }
}

fn push_entity_section(rendered: &mut String, heading: &str, items: &[EntitySummary]) {
    writeln!(rendered, "{heading}:").expect("writing to a string should succeed");
    if items.is_empty() {
        writeln!(rendered, "- none").expect("writing to a string should succeed");
        return;
    }

    for item in items {
        writeln!(rendered, "- {}\t{}", item.id, item.title)
            .expect("writing to a string should succeed");
    }
}

fn query_label(output: &TraceLookupOutput) -> String {
    match &output.symbol {
        Some(symbol) => format!("{}::{symbol}", output.file),
        None => output.file.clone(),
    }
}

#[cfg(test)]
mod tests {
    use std::{
        collections::BTreeMap,
        path::{Path, PathBuf},
    };

    use tempfile::tempdir;

    use crate::{
        cli::{OutputFormat, TraceArgs},
        command::lookup::EntitySummary,
        config::SyuConfig,
        model::TraceReference,
        workspace::Workspace,
    };

    use super::{
        MatchMode, TraceLookupOutput, TraceLookupStatus, TraceOwnerMatch, TraceOwnerMetadata,
        collect_related_entities, collect_requirement_context, insert_summary, match_mode,
        normalize_lookup_file, query_label, render_text_output, trace_owner_match,
    };

    #[test]
    fn normalize_lookup_file_accepts_workspace_relative_paths() {
        let tempdir = tempdir().expect("tempdir should exist");
        let workspace = Workspace {
            root: tempdir.path().to_path_buf(),
            spec_root: tempdir.path().join("docs/syu"),
            config: SyuConfig::default(),
            philosophies: Vec::new(),
            policies: Vec::new(),
            requirements: Vec::new(),
            features: Vec::new(),
        };

        let normalized = normalize_lookup_file(&workspace, Path::new("./src/../src/lib.rs"))
            .expect("relative file should normalize");
        assert_eq!(normalized, PathBuf::from("src/lib.rs"));
    }

    #[test]
    fn normalize_lookup_file_rejects_absolute_paths_outside_the_workspace() {
        let tempdir = tempdir().expect("tempdir should exist");
        let workspace = Workspace {
            root: tempdir.path().to_path_buf(),
            spec_root: tempdir.path().join("docs/syu"),
            config: SyuConfig::default(),
            philosophies: Vec::new(),
            policies: Vec::new(),
            requirements: Vec::new(),
            features: Vec::new(),
        };

        let error = normalize_lookup_file(&workspace, Path::new("/tmp/outside.rs"))
            .expect_err("outside paths should fail");
        assert!(error.to_string().contains("must stay under workspace"));
    }

    #[test]
    fn normalize_lookup_file_accepts_absolute_paths_within_the_workspace() {
        let tempdir = tempdir().expect("tempdir should exist");
        let workspace = Workspace {
            root: tempdir.path().to_path_buf(),
            spec_root: tempdir.path().join("docs/syu"),
            config: SyuConfig::default(),
            philosophies: Vec::new(),
            policies: Vec::new(),
            requirements: Vec::new(),
            features: Vec::new(),
        };

        let absolute = tempdir.path().join("src/lib.rs");
        let normalized =
            normalize_lookup_file(&workspace, &absolute).expect("workspace-relative path");
        assert_eq!(normalized, PathBuf::from("src/lib.rs"));
    }

    #[test]
    fn normalize_lookup_file_rejects_empty_paths() {
        let tempdir = tempdir().expect("tempdir should exist");
        let workspace = Workspace {
            root: tempdir.path().to_path_buf(),
            spec_root: tempdir.path().join("docs/syu"),
            config: SyuConfig::default(),
            philosophies: Vec::new(),
            policies: Vec::new(),
            requirements: Vec::new(),
            features: Vec::new(),
        };

        let error =
            normalize_lookup_file(&workspace, Path::new("")).expect_err("empty file should fail");
        assert!(error.to_string().contains("must not be empty"));
    }

    #[test]
    fn normalize_lookup_file_rejects_relative_paths_that_escape_the_workspace() {
        let tempdir = tempdir().expect("tempdir should exist");
        let workspace = Workspace {
            root: tempdir.path().to_path_buf(),
            spec_root: tempdir.path().join("docs/syu"),
            config: SyuConfig::default(),
            philosophies: Vec::new(),
            policies: Vec::new(),
            requirements: Vec::new(),
            features: Vec::new(),
        };

        let error = normalize_lookup_file(&workspace, Path::new("../outside.rs"))
            .expect_err("escaping relative paths should fail");
        assert!(error.to_string().contains("must stay under workspace"));
    }

    #[test]
    fn query_label_includes_symbols_when_present() {
        let label = query_label(&TraceLookupOutput {
            file: "src/lib.rs".to_string(),
            symbol: Some("run".to_string()),
            status: TraceLookupStatus::Unowned,
            matched_owners: Vec::new(),
            file_only_owners: Vec::new(),
            requirements: Vec::new(),
            features: Vec::new(),
            policies: Vec::new(),
            philosophies: Vec::new(),
        });
        assert_eq!(label, "src/lib.rs::run");
    }

    #[test]
    fn match_mode_helpers_cover_all_labels() {
        assert_eq!(MatchMode::from_label("file"), MatchMode::File);
        assert_eq!(MatchMode::from_label("symbol"), MatchMode::Symbol);
        assert_eq!(MatchMode::from_label("wildcard"), MatchMode::Wildcard);
        assert_eq!(MatchMode::Wildcard.label(), "wildcard");
        assert_eq!(
            MatchMode::Symbol.matched_label(Some("run_trace_command")),
            "symbol `run_trace_command`"
        );
        assert_eq!(MatchMode::Wildcard.matched_label(None), "wildcard `*`");
    }

    #[test]
    fn run_trace_command_rejects_blank_symbols_before_loading_the_workspace() {
        let error = super::run_trace_command(&TraceArgs {
            file: Some(PathBuf::from("src/lib.rs")),
            workspace: PathBuf::from("."),
            symbol: Some("   ".to_string()),
            range: None,
            format: OutputFormat::Text,
        })
        .expect_err("blank symbols should fail");

        assert!(
            error
                .to_string()
                .contains("must not be empty or whitespace")
        );
    }

    #[test]
    fn wildcard_trace_owner_matches_record_the_wildcard_symbol() {
        let owner = trace_owner_match(
            TraceOwnerMetadata {
                kind: crate::cli::LookupKind::Feature,
                id: "FEAT-TRACE-001",
                title: "Trace",
                trace_role: "implementation",
            },
            "rust",
            "src/lib.rs",
            &TraceReference {
                file: PathBuf::from("src/lib.rs"),
                symbols: vec!["*".to_string()],
                doc_contains: Vec::new(),
            },
            MatchMode::Wildcard,
            None,
        );

        assert_eq!(owner.match_mode, "wildcard");
        assert_eq!(owner.matched_symbol.as_deref(), Some("*"));
    }

    #[test]
    fn match_mode_detects_wildcard_ownership() {
        let mode = match_mode(
            &TraceReference {
                file: PathBuf::from("src/lib.rs"),
                symbols: vec!["*".to_string()],
                doc_contains: Vec::new(),
            },
            Some("run"),
        );
        assert_eq!(mode, Some(MatchMode::Wildcard));
    }

    #[test]
    fn collect_related_entities_skips_unknown_owners_without_panicking() {
        let tempdir = tempdir().expect("tempdir should exist");
        let workspace = Workspace {
            root: tempdir.path().to_path_buf(),
            spec_root: tempdir.path().join("docs/syu"),
            config: SyuConfig::default(),
            philosophies: Vec::new(),
            policies: Vec::new(),
            requirements: Vec::new(),
            features: Vec::new(),
        };

        let related = collect_related_entities(
            crate::command::lookup::WorkspaceLookup::new(&workspace),
            &[TraceOwnerMatch {
                kind: "unknown",
                id: "MISSING".to_string(),
                title: "Missing".to_string(),
                trace_role: "implementation".to_string(),
                language: "rust".to_string(),
                file: "src/lib.rs".to_string(),
                declared_symbols: Vec::new(),
                matched_symbol: None,
                match_mode: "file",
            }],
        );

        assert!(related.requirements.is_empty());
        assert!(related.features.is_empty());
        assert!(related.policies.is_empty());
        assert!(related.philosophies.is_empty());
    }

    #[test]
    fn related_entity_collection_handles_missing_links_gracefully() {
        let tempdir = tempdir().expect("tempdir should exist");
        let workspace = Workspace {
            root: tempdir.path().to_path_buf(),
            spec_root: tempdir.path().join("docs/syu"),
            config: SyuConfig::default(),
            philosophies: Vec::new(),
            policies: Vec::new(),
            requirements: Vec::new(),
            features: Vec::new(),
        };

        let related = collect_related_entities(
            crate::command::lookup::WorkspaceLookup::new(&workspace),
            &[
                TraceOwnerMatch {
                    kind: "requirement",
                    id: "REQ-MISSING-001".to_string(),
                    title: "Missing requirement".to_string(),
                    trace_role: "test".to_string(),
                    language: "rust".to_string(),
                    file: "src/lib.rs".to_string(),
                    declared_symbols: Vec::new(),
                    matched_symbol: None,
                    match_mode: "file",
                },
                TraceOwnerMatch {
                    kind: "feature",
                    id: "FEAT-MISSING-001".to_string(),
                    title: "Missing feature".to_string(),
                    trace_role: "implementation".to_string(),
                    language: "rust".to_string(),
                    file: "src/lib.rs".to_string(),
                    declared_symbols: Vec::new(),
                    matched_symbol: None,
                    match_mode: "file",
                },
            ],
        );

        assert!(related.requirements.is_empty());
        assert!(related.features.is_empty());
    }

    #[test]
    fn missing_policy_links_are_ignored_when_collecting_context() {
        let tempdir = tempdir().expect("tempdir should exist");
        let workspace = Workspace {
            root: tempdir.path().to_path_buf(),
            spec_root: tempdir.path().join("docs/syu"),
            config: SyuConfig::default(),
            philosophies: Vec::new(),
            policies: Vec::new(),
            requirements: vec![crate::model::Requirement {
                id: "REQ-TRACE-001".to_string(),
                title: "Trace".to_string(),
                description: "desc".to_string(),
                priority: "medium".to_string(),
                status: "implemented".to_string(),
                linked_policies: vec!["POL-MISSING-001".to_string()],
                linked_features: Vec::new(),
                tests: BTreeMap::new(),
            }],
            features: Vec::new(),
        };
        let requirement = &workspace.requirements[0];
        let mut policies = BTreeMap::new();
        let mut philosophies = BTreeMap::new();

        collect_requirement_context(
            crate::command::lookup::WorkspaceLookup::new(&workspace),
            requirement,
            &mut policies,
            &mut philosophies,
        );

        assert!(policies.is_empty());
        assert!(philosophies.is_empty());
    }

    #[test]
    fn insert_summary_ignores_unknown_ids() {
        let tempdir = tempdir().expect("tempdir should exist");
        let workspace = Workspace {
            root: tempdir.path().to_path_buf(),
            spec_root: tempdir.path().join("docs/syu"),
            config: SyuConfig::default(),
            philosophies: Vec::new(),
            policies: Vec::new(),
            requirements: Vec::new(),
            features: Vec::new(),
        };
        let mut summaries = BTreeMap::new();

        insert_summary(
            &mut summaries,
            crate::command::lookup::WorkspaceLookup::new(&workspace),
            crate::cli::LookupKind::Requirement,
            "REQ-MISSING-001",
        );

        assert!(summaries.is_empty());
    }

    #[test]
    fn render_text_output_reports_empty_related_sections() {
        let rendered = render_text_output(&TraceLookupOutput {
            file: "src/lib.rs".to_string(),
            symbol: Some("run_trace_command".to_string()),
            status: TraceLookupStatus::Owned,
            matched_owners: vec![
                TraceOwnerMatch {
                    kind: "feature",
                    id: "FEAT-TRACE-001".to_string(),
                    title: "Trace".to_string(),
                    trace_role: "implementation".to_string(),
                    language: "rust".to_string(),
                    file: "src/lib.rs".to_string(),
                    declared_symbols: vec!["run_trace_command".to_string()],
                    matched_symbol: Some("run_trace_command".to_string()),
                    match_mode: "symbol",
                },
                TraceOwnerMatch {
                    kind: "feature",
                    id: "FEAT-TRACE-002".to_string(),
                    title: "Wildcard".to_string(),
                    trace_role: "implementation".to_string(),
                    language: "rust".to_string(),
                    file: "src/lib.rs".to_string(),
                    declared_symbols: vec!["*".to_string()],
                    matched_symbol: Some("*".to_string()),
                    match_mode: "wildcard",
                },
            ],
            file_only_owners: Vec::new(),
            requirements: Vec::new(),
            features: Vec::new(),
            policies: Vec::new(),
            philosophies: Vec::new(),
        });

        assert!(rendered.contains("matched by symbol `run_trace_command`"));
        assert!(rendered.contains("matched by wildcard `*`"));
        assert!(rendered.contains("Requirements:\n- none"));
    }

    #[test]
    fn run_trace_command_requires_a_file_or_range() {
        let error = super::run_trace_command(&TraceArgs {
            file: None,
            workspace: PathBuf::from("."),
            symbol: None,
            range: None,
            format: OutputFormat::Text,
        })
        .expect_err("missing file and range should fail");

        assert!(
            error
                .to_string()
                .contains("either FILE or --range must be provided")
        );
    }

    #[test]
    fn compute_range_summary_counts_partial_results() {
        let summary = super::compute_range_summary(&[
            TraceLookupOutput {
                file: "owned.rs".to_string(),
                symbol: None,
                status: TraceLookupStatus::Owned,
                matched_owners: Vec::new(),
                file_only_owners: Vec::new(),
                requirements: vec![EntitySummary {
                    id: "REQ-1".to_string(),
                    title: "Req".to_string(),
                    document_path: None,
                }],
                features: Vec::new(),
                policies: Vec::new(),
                philosophies: Vec::new(),
            },
            TraceLookupOutput {
                file: "partial.rs".to_string(),
                symbol: None,
                status: TraceLookupStatus::Partial,
                matched_owners: Vec::new(),
                file_only_owners: Vec::new(),
                requirements: Vec::new(),
                features: vec![EntitySummary {
                    id: "FEAT-1".to_string(),
                    title: "Feat".to_string(),
                    document_path: None,
                }],
                policies: Vec::new(),
                philosophies: Vec::new(),
            },
            TraceLookupOutput {
                file: "unowned.rs".to_string(),
                symbol: None,
                status: TraceLookupStatus::Unowned,
                matched_owners: Vec::new(),
                file_only_owners: Vec::new(),
                requirements: Vec::new(),
                features: Vec::new(),
                policies: vec![EntitySummary {
                    id: "POL-1".to_string(),
                    title: "Pol".to_string(),
                    document_path: None,
                }],
                philosophies: vec![EntitySummary {
                    id: "PHIL-1".to_string(),
                    title: "Phil".to_string(),
                    document_path: None,
                }],
            },
        ]);

        assert_eq!(summary.total_files, 3);
        assert_eq!(summary.owned_files, 1);
        assert_eq!(summary.partial_files, 1);
        assert_eq!(summary.unowned_files, 1);
        assert_eq!(summary.total_requirements, 1);
        assert_eq!(summary.total_features, 1);
        assert_eq!(summary.total_policies, 1);
        assert_eq!(summary.total_philosophies, 1);
    }

    #[test]
    fn render_range_text_groups_file_only_and_unowned_results() {
        let rendered = super::render_range_text(
            "HEAD~1..HEAD",
            &[
                TraceLookupOutput {
                    file: "file-only.rs".to_string(),
                    symbol: None,
                    status: TraceLookupStatus::Owned,
                    matched_owners: Vec::new(),
                    file_only_owners: vec![TraceOwnerMatch {
                        kind: "feature",
                        id: "FEAT-1".to_string(),
                        title: "Feature".to_string(),
                        trace_role: "implementation".to_string(),
                        language: "rust".to_string(),
                        file: "file-only.rs".to_string(),
                        declared_symbols: Vec::new(),
                        matched_symbol: None,
                        match_mode: "file",
                    }],
                    requirements: Vec::new(),
                    features: Vec::new(),
                    policies: Vec::new(),
                    philosophies: Vec::new(),
                },
                TraceLookupOutput {
                    file: "unowned.rs".to_string(),
                    symbol: None,
                    status: TraceLookupStatus::Unowned,
                    matched_owners: Vec::new(),
                    file_only_owners: Vec::new(),
                    requirements: Vec::new(),
                    features: Vec::new(),
                    policies: Vec::new(),
                    philosophies: Vec::new(),
                },
            ],
            &super::TraceRangeSummary {
                total_files: 2,
                owned_files: 1,
                partial_files: 0,
                unowned_files: 1,
                total_requirements: 0,
                total_features: 1,
                total_policies: 0,
                total_philosophies: 0,
            },
        );

        assert!(rendered.contains("feature FEAT-1:"));
        assert!(rendered.contains("UNOWNED:"));
        assert!(rendered.contains("Features: 1"));
    }
}
