// FEAT-LOG-001
// REQ-CORE-024

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Write,
    path::{Component, Path, PathBuf},
    process::Command,
};

use anyhow::{Context, Result, bail};
use serde::Serialize;

use crate::{
    cli::{HistoryKind, LogArgs, LookupKind, OutputFormat},
    coverage::normalize_relative_path,
    model::{Feature, Requirement, TraceReference},
    workspace::{Workspace, load_workspace},
};

use super::{
    lookup::{WorkspaceEntity, WorkspaceLookup},
    relate::build_relation_report,
    shell_quote_path,
};

const GIT_RECORD_SEPARATOR: u8 = 0x1e;
const GIT_ENVIRONMENT_KEYS: [&str; 8] = [
    "GIT_ALTERNATE_OBJECT_DIRECTORIES",
    "GIT_CEILING_DIRECTORIES",
    "GIT_COMMON_DIR",
    "GIT_DIR",
    "GIT_INDEX_FILE",
    "GIT_OBJECT_DIRECTORY",
    "GIT_PREFIX",
    "GIT_WORK_TREE",
];

#[derive(Debug, Serialize)]
struct JsonLogOutput {
    id: String,
    entity_kind: &'static str,
    title: String,
    repository_root: String,
    kind: &'static str,
    include_related: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    scope: Option<JsonHistoryScope>,
    #[serde(skip_serializing_if = "Option::is_none")]
    path_filter: Option<String>,
    tracked_paths: Vec<TrackedPath>,
    commits: Vec<MatchedCommit>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct JsonHistoryScope {
    label: String,
    revision_range: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct TrackedPath {
    kind: &'static str,
    path: String,
    owner_kind: &'static str,
    owner_id: String,
    source: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    language: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    symbols: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct MatchedCommit {
    sha: String,
    short_sha: String,
    summary: String,
    author: String,
    authored_at: String,
    reasons: Vec<TrackedPath>,
}

#[derive(Debug, Clone)]
struct HistoryTarget {
    id: String,
    entity_kind: &'static str,
    title: String,
    tracked_paths: Vec<TrackedPath>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct HistoryScope {
    label: String,
    revision_range: String,
}

pub fn run_log_command(args: &LogArgs) -> Result<i32> {
    if args.limit == 0 {
        bail!("`--limit` must be greater than zero");
    }

    let workspace = load_workspace(&args.workspace)?;
    let lookup = WorkspaceLookup::new(&workspace);
    let path_filter = args
        .path
        .as_deref()
        .map(|path| normalize_path_filter(&workspace.root, path))
        .transpose()?
        .filter(|path| !path.as_os_str().is_empty());
    let mut target = build_history_target(
        lookup,
        &workspace.root,
        &args.id,
        args.kind,
        path_filter.as_deref(),
    )?;
    if args.include_related {
        let related =
            collect_related_tracked_paths(&workspace, &args.id, args.kind, path_filter.as_deref())?;
        target.tracked_paths.extend(related);
        dedupe_tracked_paths(&mut target.tracked_paths);
    }
    let repository_root = resolve_git_repository_root(&workspace.root)?;
    let merge_base_ref = args.merge_base_ref.as_deref();
    let scope = resolve_history_scope(&workspace.root, args.range.as_deref(), merge_base_ref)?;
    let commits = load_git_history(
        &workspace.root,
        args.limit,
        &target.tracked_paths,
        scope.as_ref(),
    )?;

    match args.format {
        OutputFormat::Text => print!(
            "{}",
            render_history_text(
                &target,
                &repository_root,
                args.kind,
                args.include_related,
                scope.as_ref(),
                path_filter.as_deref(),
                &commits,
            )
        ),
        OutputFormat::Json => println!(
            "{}",
            serde_json::to_string_pretty(&JsonLogOutput {
                id: target.id.clone(),
                entity_kind: target.entity_kind,
                title: target.title.clone(),
                repository_root: repository_root.display().to_string(),
                kind: args.kind.label(),
                include_related: args.include_related,
                scope: scope.as_ref().map(|scope| JsonHistoryScope {
                    label: scope.label.clone(),
                    revision_range: scope.revision_range.clone(),
                }),
                path_filter: path_filter.map(|path| path.display().to_string()),
                tracked_paths: target.tracked_paths.clone(),
                commits,
            })
            .expect("serializing log output to JSON should succeed")
        ),
    }

    Ok(0)
}

fn build_history_target(
    lookup: WorkspaceLookup<'_>,
    workspace_root: &Path,
    id: &str,
    kind: HistoryKind,
    path_filter: Option<&Path>,
) -> Result<HistoryTarget> {
    let (entity, definition_path) = resolve_history_entity(lookup, workspace_root, id)?;

    let (entity_kind, title, traces) = match entity {
        WorkspaceEntity::Requirement(item) => (
            "requirement",
            item.title.clone(),
            tracked_paths_for_requirement(item, kind, &definition_path)?,
        ),
        WorkspaceEntity::Feature(item) => (
            "feature",
            item.title.clone(),
            tracked_paths_for_feature(item, kind, &definition_path)?,
        ),
        WorkspaceEntity::Philosophy(item) => (
            "philosophy",
            item.title.clone(),
            tracked_paths_for_non_trace_layer(&item.id, "philosophy", kind, &definition_path)?,
        ),
        WorkspaceEntity::Policy(item) => (
            "policy",
            item.title.clone(),
            tracked_paths_for_non_trace_layer(&item.id, "policy", kind, &definition_path)?,
        ),
    };

    let mut tracked_paths = traces;
    if let Some(path_filter) = path_filter {
        tracked_paths
            .retain(|tracked| normalized_tracked_path(&tracked.path).starts_with(path_filter));
    }

    if tracked_paths.is_empty() {
        let mut filters = vec![format!("kind `{}`", kind.label())];
        if let Some(path_filter) = path_filter {
            filters.push(format!("path `{}`", path_filter.display()));
        }
        bail!(
            "no tracked history paths remain for `{id}` after applying {}",
            filters.join(" and ")
        );
    }

    Ok(HistoryTarget {
        id: id.to_string(),
        entity_kind,
        title,
        tracked_paths,
    })
}

fn resolve_history_entity<'a>(
    lookup: WorkspaceLookup<'a>,
    workspace_root: &Path,
    id: &str,
) -> Result<(WorkspaceEntity<'a>, String)> {
    let Some(kind) = lookup_kind_for_id(id) else {
        let workspace_arg = shell_quote_path(workspace_root);
        bail!(
            "definition `{id}` was not found in `{}`\n\nhint: Run `syu list {workspace_arg}` to see all available IDs, or `syu search {id} {workspace_arg}` to find related items.",
            workspace_root.display()
        );
    };

    let entries = lookup
        .entries_with_document_paths(kind)?
        .into_iter()
        .filter(|entry| entry.id == id)
        .collect::<Vec<_>>();

    if entries.is_empty() {
        let workspace_arg = shell_quote_path(workspace_root);
        bail!(
            "definition `{id}` was not found in `{}`\n\nhint: Run `syu list {workspace_arg}` to see all available IDs, or `syu search {id} {workspace_arg}` to find related items.",
            workspace_root.display()
        );
    }

    if entries.len() > 1 {
        let documents = entries
            .iter()
            .filter_map(|entry| entry.document_path.as_deref())
            .map(|path| format!("- {path}"))
            .collect::<Vec<_>>()
            .join("\n");
        bail!(
            "definition `{id}` is ambiguous because it appears in multiple documents:\n{documents}\n\nhint: fix the duplicate ID before using `syu log`."
        );
    }

    let entity = lookup
        .find(id)
        .expect("a document-path match must also exist in the workspace model");
    let definition_path = lookup
        .document_path_for_id(id)?
        .with_context(|| format!("checked-in definition path for `{id}` was not found"))?;

    Ok((entity, definition_path))
}

fn lookup_kind_for_id(id: &str) -> Option<LookupKind> {
    if id.starts_with("PHIL-") {
        return Some(LookupKind::Philosophy);
    }
    if id.starts_with("POL-") {
        return Some(LookupKind::Policy);
    }
    if id.starts_with("REQ-") {
        return Some(LookupKind::Requirement);
    }
    if id.starts_with("FEAT-") {
        return Some(LookupKind::Feature);
    }

    None
}

fn tracked_paths_for_requirement(
    item: &Requirement,
    kind: HistoryKind,
    definition_path: &str,
) -> Result<Vec<TrackedPath>> {
    match kind {
        HistoryKind::Implementation => bail!(
            "`{}` is a requirement, so `--kind implementation` is not available. Use `--kind test`, `--kind definition`, or omit the flag.",
            item.id
        ),
        HistoryKind::All => {
            let mut tracked = vec![TrackedPath::definition(
                "requirement",
                &item.id,
                "selected",
                definition_path,
            )];
            tracked.extend(collect_trace_paths(
                "requirement",
                &item.id,
                "selected",
                "test",
                &item.tests,
            ));
            Ok(tracked)
        }
        HistoryKind::Definition => Ok(vec![TrackedPath::definition(
            "requirement",
            &item.id,
            "selected",
            definition_path,
        )]),
        HistoryKind::Test => Ok(collect_trace_paths(
            "requirement",
            &item.id,
            "selected",
            "test",
            &item.tests,
        )),
    }
}

fn tracked_paths_for_feature(
    item: &Feature,
    kind: HistoryKind,
    definition_path: &str,
) -> Result<Vec<TrackedPath>> {
    match kind {
        HistoryKind::Test => bail!(
            "`{}` is a feature, so `--kind test` is not available. Use `--kind implementation`, `--kind definition`, or omit the flag.",
            item.id
        ),
        HistoryKind::All => {
            let mut tracked = vec![TrackedPath::definition(
                "feature",
                &item.id,
                "selected",
                definition_path,
            )];
            tracked.extend(collect_trace_paths(
                "feature",
                &item.id,
                "selected",
                "implementation",
                &item.implementations,
            ));
            Ok(tracked)
        }
        HistoryKind::Definition => Ok(vec![TrackedPath::definition(
            "feature",
            &item.id,
            "selected",
            definition_path,
        )]),
        HistoryKind::Implementation => Ok(collect_trace_paths(
            "feature",
            &item.id,
            "selected",
            "implementation",
            &item.implementations,
        )),
    }
}

fn tracked_paths_for_non_trace_layer(
    id: &str,
    layer_label: &'static str,
    kind: HistoryKind,
    definition_path: &str,
) -> Result<Vec<TrackedPath>> {
    match kind {
        HistoryKind::All | HistoryKind::Definition => Ok(vec![TrackedPath::definition(
            layer_label,
            id,
            "selected",
            definition_path,
        )]),
        HistoryKind::Test => bail!(
            "`{id}` is a {layer_label}, so `--kind test` is not available. Use `--kind definition` or omit the flag."
        ),
        HistoryKind::Implementation => bail!(
            "`{id}` is a {layer_label}, so `--kind implementation` is not available. Use `--kind definition` or omit the flag."
        ),
    }
}

fn collect_trace_paths(
    owner_kind: &'static str,
    owner_id: &str,
    source: &'static str,
    kind: &'static str,
    traces: &std::collections::BTreeMap<String, Vec<TraceReference>>,
) -> Vec<TrackedPath> {
    let mut tracked = Vec::new();
    for (language, references) in traces {
        for reference in references {
            tracked.push(TrackedPath {
                kind,
                path: reference.file.display().to_string(),
                owner_kind,
                owner_id: owner_id.to_string(),
                source,
                language: Some(language.clone()),
                symbols: reference.symbols.clone(),
            });
        }
    }
    tracked
}

fn collect_related_tracked_paths(
    workspace: &Workspace,
    selector: &str,
    kind: HistoryKind,
    path_filter: Option<&Path>,
) -> Result<Vec<TrackedPath>> {
    let report = build_relation_report(workspace, selector)?;
    let mut tracked = Vec::new();

    if matches!(kind, HistoryKind::All | HistoryKind::Definition) {
        for node in report
            .philosophies
            .iter()
            .chain(report.policies.iter())
            .chain(report.requirements.iter())
            .chain(report.features.iter())
        {
            tracked.push(TrackedPath::definition(
                node.kind,
                &node.id,
                "related",
                &node.document_path,
            ));
        }
        tracked.retain(|entry| !(entry.owner_id == selector && entry.source == "related"));
    }

    if !matches!(kind, HistoryKind::Definition) {
        for trace in &report.traces {
            if kind != HistoryKind::All && trace.relation_kind != kind.label() {
                continue;
            }
            tracked.push(TrackedPath {
                kind: trace.relation_kind,
                path: trace.file.clone(),
                owner_kind: trace.owner_kind,
                owner_id: trace.owner_id.clone(),
                source: "related",
                language: Some(trace.language.clone()),
                symbols: trace.symbols.clone(),
            });
        }
    }

    if let Some(path_filter) = path_filter {
        tracked.retain(|tracked| {
            tracked.kind == "definition"
                || normalized_tracked_path(&tracked.path).starts_with(path_filter)
        });
    }

    Ok(tracked)
}

fn dedupe_tracked_paths(tracked_paths: &mut Vec<TrackedPath>) {
    let mut deduped = Vec::with_capacity(tracked_paths.len());
    for tracked in tracked_paths.drain(..) {
        if !deduped.contains(&tracked) {
            deduped.push(tracked);
        }
    }
    *tracked_paths = deduped;
}

fn normalize_path_filter(workspace_root: &Path, path: &Path) -> Result<PathBuf> {
    if path.is_absolute() {
        let relative = path
            .strip_prefix(workspace_root)
            .map(Path::to_path_buf)
            .with_context(|| {
                format!(
                    "path filter `{}` must stay inside workspace `{}`",
                    path.display(),
                    workspace_root.display()
                )
            })?;
        return normalize_path_filter(workspace_root, &relative);
    }

    let normalized = normalize_relative_path(path);
    if normalized.is_absolute()
        || normalized
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        bail!(
            "path filter `{}` must stay inside workspace `{}`",
            path.display(),
            workspace_root.display()
        );
    }

    Ok(normalized)
}

fn resolve_git_repository_root(workspace_root: &Path) -> Result<PathBuf> {
    let output = git_command(workspace_root)
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .with_context(|| {
            format!(
                "failed to run `git rev-parse` in `{}`",
                workspace_root.display()
            )
        })?;

    if !output.status.success() {
        bail!(
            "workspace `{}` is not inside a Git repository, so `syu log` cannot inspect commit history.",
            workspace_root.display()
        );
    }

    let repository_root = String::from_utf8(output.stdout)
        .context("git repository root should be valid UTF-8")?
        .trim()
        .to_string();
    Ok(PathBuf::from(repository_root))
}

fn resolve_history_scope(
    workspace_root: &Path,
    range: Option<&str>,
    merge_base_ref: Option<&str>,
) -> Result<Option<HistoryScope>> {
    if let Some(range) = range {
        return Ok(Some(HistoryScope {
            label: format!("range `{range}`"),
            revision_range: range.to_string(),
        }));
    }

    let Some(reference) = merge_base_ref else {
        return Ok(None);
    };
    let output = git_command(workspace_root)
        .args(["merge-base", "HEAD", reference])
        .output()
        .with_context(|| {
            format!(
                "failed to run `git merge-base HEAD {reference}` in `{}`",
                workspace_root.display()
            )
        })?;
    if !output.status.success() {
        bail!(
            "failed to compute merge-base between `HEAD` and `{reference}` in `{}`: {}",
            workspace_root.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    let merge_base = String::from_utf8(output.stdout)
        .context("git merge-base output should be valid UTF-8")?
        .trim()
        .to_string();
    if merge_base.is_empty() {
        bail!(
            "failed to compute merge-base between `HEAD` and `{reference}` in `{}`: git returned an empty base SHA",
            workspace_root.display()
        );
    }

    Ok(Some(HistoryScope {
        label: format!("merge-base({reference})..HEAD"),
        revision_range: format!("{merge_base}..HEAD"),
    }))
}

fn load_git_history(
    workspace_root: &Path,
    limit: usize,
    tracked_paths: &[TrackedPath],
    scope: Option<&HistoryScope>,
) -> Result<Vec<MatchedCommit>> {
    if tracked_paths.is_empty() {
        return Ok(Vec::new());
    }

    let mut grouped_paths = BTreeMap::<PathBuf, Vec<TrackedPath>>::new();
    for tracked in tracked_paths {
        grouped_paths
            .entry(normalized_tracked_path(&tracked.path))
            .or_default()
            .push(tracked.clone());
    }

    let mut merged_commits = BTreeMap::<String, MatchedCommit>::new();
    for (path, reasons) in grouped_paths {
        for commit in load_git_history_for_path(workspace_root, limit, &path, scope)? {
            let entry = merged_commits
                .entry(commit.sha.clone())
                .or_insert_with(|| MatchedCommit {
                    reasons: Vec::new(),
                    ..commit
                });
            for reason in &reasons {
                if !entry.reasons.contains(reason) {
                    entry.reasons.push(reason.clone());
                }
            }
        }
    }

    order_commits_by_repository_history(workspace_root, merged_commits, limit)
}

fn load_git_history_for_path(
    workspace_root: &Path,
    limit: usize,
    path: &Path,
    scope: Option<&HistoryScope>,
) -> Result<Vec<MatchedCommit>> {
    let mut command = git_command(workspace_root);
    command.args([
        "log",
        "--follow",
        "--relative",
        "--max-count",
        &limit.to_string(),
        "--format=%x1E%H%x00%h%x00%an%x00%aI%x00%s",
        "--name-only",
        "-z",
    ]);
    if let Some(scope) = scope {
        command.arg(&scope.revision_range);
    }
    command.arg("--");
    command.arg(path);

    let output = command
        .output()
        .with_context(|| format!("failed to run `git log` in `{}`", workspace_root.display()))?;
    if !output.status.success() {
        bail!(
            "failed to read git history for `{}`: {}",
            workspace_root.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }

    parse_git_history(&output.stdout)
}

fn normalized_tracked_path(path: &str) -> PathBuf {
    normalize_relative_path(Path::new(path))
}

fn order_commits_by_repository_history(
    workspace_root: &Path,
    commits_by_sha: BTreeMap<String, MatchedCommit>,
    limit: usize,
) -> Result<Vec<MatchedCommit>> {
    let mut commits = commits_by_sha.into_values().collect::<Vec<_>>();
    if sort_commits_by_history_relationship(workspace_root, &mut commits).is_err() {
        commits.sort_by(commit_recency_cmp);
    }
    commits.truncate(limit);
    Ok(commits)
}

fn sort_commits_by_history_relationship(
    workspace_root: &Path,
    commits: &mut [MatchedCommit],
) -> Result<()> {
    let mut precedence = BTreeMap::<String, BTreeSet<String>>::new();
    let mut indegree = BTreeMap::<String, usize>::new();
    let mut cache = BTreeMap::<(String, String), bool>::new();

    for commit in commits.iter() {
        precedence.entry(commit.sha.clone()).or_default();
        indegree.entry(commit.sha.clone()).or_insert(0);
    }

    for left_index in 0..commits.len() {
        for right_index in (left_index + 1)..commits.len() {
            let left = &commits[left_index];
            let right = &commits[right_index];
            if commit_is_ancestor_of(workspace_root, &left.sha, &right.sha, &mut cache)? {
                if precedence
                    .entry(right.sha.clone())
                    .or_default()
                    .insert(left.sha.clone())
                {
                    *indegree.entry(left.sha.clone()).or_insert(0) += 1;
                }
            } else if commit_is_ancestor_of(workspace_root, &right.sha, &left.sha, &mut cache)?
                && precedence
                    .entry(left.sha.clone())
                    .or_default()
                    .insert(right.sha.clone())
            {
                *indegree.entry(right.sha.clone()).or_insert(0) += 1;
            }
        }
    }

    let commit_lookup = commits
        .iter()
        .cloned()
        .map(|commit| (commit.sha.clone(), commit))
        .collect::<BTreeMap<_, _>>();
    let mut ready = indegree
        .iter()
        .filter(|(_, degree)| **degree == 0)
        .map(|(sha, _)| sha.clone())
        .collect::<Vec<_>>();
    sort_ready_commits(&mut ready, &commit_lookup);
    let mut ordered = Vec::new();

    while let Some(sha) = ready.pop() {
        ordered.push(
            commit_lookup
                .get(&sha)
                .expect("ready commits should exist in the lookup")
                .clone(),
        );
        for dependent in precedence.get(&sha).into_iter().flatten() {
            let degree = indegree
                .get_mut(dependent)
                .expect("dependent commits should track indegree");
            *degree -= 1;
            if *degree == 0 {
                ready.push(dependent.clone());
            }
        }
        sort_ready_commits(&mut ready, &commit_lookup);
    }

    if ordered.len() != commits.len() {
        bail!("could not derive a stable candidate history order");
    }

    commits.clone_from_slice(&ordered);
    Ok(())
}

fn commit_is_ancestor_of(
    workspace_root: &Path,
    older_sha: &str,
    newer_sha: &str,
    cache: &mut BTreeMap<(String, String), bool>,
) -> Result<bool> {
    let cache_key = (older_sha.to_string(), newer_sha.to_string());
    if let Some(result) = cache.get(&cache_key) {
        return Ok(*result);
    }

    let output = Command::new("git")
        .arg("-C")
        .arg(workspace_root)
        .args(["merge-base", "--is-ancestor", older_sha, newer_sha])
        .output()
        .with_context(|| {
            format!(
                "failed to run `git merge-base --is-ancestor` in `{}`",
                workspace_root.display()
            )
        })?;

    let result = match output.status.code() {
        Some(0) => true,
        Some(1) => false,
        _ => {
            bail!(
                "failed to compare git history order for `{}`: {}",
                workspace_root.display(),
                String::from_utf8_lossy(&output.stderr).trim()
            )
        }
    };
    cache.insert(cache_key, result);
    Ok(result)
}

fn commit_recency_cmp(left: &MatchedCommit, right: &MatchedCommit) -> std::cmp::Ordering {
    right
        .authored_at
        .cmp(&left.authored_at)
        .then_with(|| right.sha.cmp(&left.sha))
}

fn sort_ready_commits(ready: &mut [String], commit_lookup: &BTreeMap<String, MatchedCommit>) {
    ready.sort_by(|left, right| commit_recency_cmp(&commit_lookup[right], &commit_lookup[left]));
}

pub(crate) fn git_command(workspace_root: &Path) -> Command {
    let mut command = Command::new("git");
    command.arg("-C").arg(workspace_root);
    for key in GIT_ENVIRONMENT_KEYS {
        command.env_remove(key);
    }
    command
}

pub(crate) fn resolve_git_range_changed_files(
    workspace_root: &Path,
    range: &str,
) -> Result<Vec<PathBuf>> {
    let output = git_command(workspace_root)
        .args(["diff", "--name-only", "--relative", range])
        .output()
        .with_context(|| {
            format!(
                "failed to run `git diff --name-only` for range `{}` in `{}`",
                range,
                workspace_root.display()
            )
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!(
            "git range `{}` is not valid in `{}`:\n{}",
            range,
            workspace_root.display(),
            stderr.trim()
        );
    }

    let files_str =
        String::from_utf8(output.stdout).context("git diff output should be valid UTF-8")?;

    let mut files = Vec::new();
    for line in files_str.lines() {
        let line = line.trim();
        if !line.is_empty() {
            files.push(PathBuf::from(line));
        }
    }

    Ok(files)
}

fn parse_git_history(raw: &[u8]) -> Result<Vec<MatchedCommit>> {
    let mut commits = Vec::new();

    for record in raw.split(|byte| *byte == GIT_RECORD_SEPARATOR) {
        if record.is_empty() {
            continue;
        }

        let fields = record.split(|byte| *byte == 0).collect::<Vec<_>>();
        if fields.len() < 5 {
            bail!("unexpected `git log` output while parsing commit history");
        }

        commits.push(MatchedCommit {
            sha: parse_git_field(fields[0])?,
            short_sha: parse_git_field(fields[1])?,
            author: parse_git_field(fields[2])?,
            authored_at: parse_git_field(fields[3])?,
            summary: parse_git_field(fields[4])?,
            reasons: Vec::new(),
        });
    }

    Ok(commits)
}

fn parse_git_field(field: &[u8]) -> Result<String> {
    Ok(std::str::from_utf8(field)
        .context("git output should be valid UTF-8")?
        .to_string())
}

fn render_history_text(
    target: &HistoryTarget,
    repository_root: &Path,
    kind: HistoryKind,
    include_related: bool,
    scope: Option<&HistoryScope>,
    path_filter: Option<&Path>,
    commits: &[MatchedCommit],
) -> String {
    let mut output = String::new();
    writeln!(
        output,
        "History: {} {} — {}",
        target.entity_kind, target.id, target.title
    )
    .expect("writing to String must succeed");
    writeln!(output, "Repository: {}", repository_root.display())
        .expect("writing to String must succeed");
    writeln!(output, "Selection: {}", kind.label()).expect("writing to String must succeed");
    writeln!(
        output,
        "Related surface: {}",
        if include_related {
            "included"
        } else {
            "selected only"
        }
    )
    .expect("writing to String must succeed");
    if let Some(scope) = scope {
        writeln!(output, "Scope: {}", scope.label).expect("writing to String must succeed");
    }
    if let Some(path_filter) = path_filter {
        writeln!(output, "Path filter: {}", path_filter.display())
            .expect("writing to String must succeed");
    }

    writeln!(output, "Tracked paths:").expect("writing to String must succeed");
    for tracked in &target.tracked_paths {
        writeln!(output, "- {}", render_tracked_path(tracked))
            .expect("writing to String must succeed");
    }

    if commits.is_empty() {
        writeln!(output, "Commits:\n- none").expect("writing to String must succeed");
        return output;
    }

    writeln!(output, "Commits:").expect("writing to String must succeed");
    for commit in commits {
        writeln!(
            output,
            "- {} {} {}",
            commit.short_sha, commit.authored_at, commit.summary
        )
        .expect("writing to String must succeed");
        for reason in &commit.reasons {
            writeln!(output, "  - {}", render_tracked_path(reason))
                .expect("writing to String must succeed");
        }
    }

    output
}

fn render_tracked_path(tracked: &TrackedPath) -> String {
    let mut output = format!("{}\t{}", tracked.kind, tracked.path);
    if let Some(language) = &tracked.language {
        write!(output, "\t{language}").expect("writing to String must succeed");
    }
    if !tracked.symbols.is_empty() {
        write!(
            output,
            "\t[{}]",
            tracked
                .symbols
                .iter()
                .map(|symbol| format!("`{symbol}`"))
                .collect::<Vec<_>>()
                .join(", ")
        )
        .expect("writing to String must succeed");
    }
    write!(
        output,
        "\t{} {} ({})",
        tracked.owner_kind, tracked.owner_id, tracked.source
    )
    .expect("writing to String must succeed");
    output
}

impl TrackedPath {
    fn definition(
        owner_kind: &'static str,
        owner_id: &str,
        source: &'static str,
        path: &str,
    ) -> Self {
        Self {
            kind: "definition",
            path: path.to_string(),
            owner_kind,
            owner_id: owner_id.to_string(),
            source,
            language: None,
            symbols: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::{
        fs,
        path::{Path, PathBuf},
        process::Command,
        sync::{LazyLock, Mutex, MutexGuard},
    };

    use tempfile::tempdir;

    use crate::{
        cli::LookupKind,
        model::{Feature, Requirement, TraceReference},
    };

    use super::{
        HistoryKind, HistoryScope, MatchedCommit, TrackedPath, collect_trace_paths,
        commit_is_ancestor_of, load_git_history, lookup_kind_for_id, normalize_path_filter,
        order_commits_by_repository_history, parse_git_history, render_history_text,
        resolve_history_scope, sort_commits_by_history_relationship, tracked_paths_for_feature,
        tracked_paths_for_requirement,
    };

    static PATH_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

    struct PathGuard {
        original: std::ffi::OsString,
        _lock: MutexGuard<'static, ()>,
    }

    impl PathGuard {
        fn set(paths: Vec<PathBuf>) -> Self {
            let lock = PATH_LOCK.lock().unwrap_or_else(|err| err.into_inner());
            let original = std::env::var_os("PATH").unwrap_or_default();
            unsafe {
                std::env::set_var(
                    "PATH",
                    std::env::join_paths(paths).expect("path should join"),
                );
            }
            Self {
                original,
                _lock: lock,
            }
        }
    }

    impl Drop for PathGuard {
        fn drop(&mut self) {
            unsafe {
                std::env::set_var("PATH", &self.original);
            }
        }
    }

    #[test]
    fn normalize_path_filter_accepts_relative_paths() {
        let normalized =
            normalize_path_filter(Path::new("/repo"), Path::new("./src/../src/command"))
                .expect("relative path should stay normalized");
        assert_eq!(normalized, Path::new("src/command"));
    }

    #[test]
    fn normalize_path_filter_strips_workspace_prefix_from_absolute_paths() {
        let normalized =
            normalize_path_filter(Path::new("/repo"), Path::new("/repo/src/command/log.rs"))
                .expect("absolute path inside workspace should be normalized");
        assert_eq!(normalized, Path::new("src/command/log.rs"));
    }

    #[test]
    fn normalize_path_filter_rejects_absolute_paths_outside_workspace() {
        let error = normalize_path_filter(Path::new("/repo"), Path::new("/outside/src/log.rs"))
            .expect_err("outside path should be rejected");
        assert!(error.to_string().contains("must stay inside workspace"));
    }

    #[test]
    fn normalize_path_filter_rejects_parent_segments() {
        let error = normalize_path_filter(Path::new("/repo"), Path::new("../src/log.rs"))
            .expect_err("parent paths should be rejected");
        assert!(error.to_string().contains("must stay inside workspace"));
    }

    #[test]
    fn lookup_kind_for_id_handles_supported_prefixes_and_unknowns() {
        assert!(matches!(
            lookup_kind_for_id("PHIL-001"),
            Some(LookupKind::Philosophy)
        ));
        assert!(matches!(
            lookup_kind_for_id("POL-001"),
            Some(LookupKind::Policy)
        ));
        assert!(matches!(
            lookup_kind_for_id("REQ-001"),
            Some(LookupKind::Requirement)
        ));
        assert!(matches!(
            lookup_kind_for_id("FEAT-001"),
            Some(LookupKind::Feature)
        ));
        assert!(lookup_kind_for_id("NOTE-001").is_none());
    }

    #[test]
    fn collect_trace_paths_preserves_language_and_symbols() {
        let mut traces = BTreeMap::new();
        traces.insert(
            "rust".to_string(),
            vec![TraceReference {
                file: PathBuf::from("src/log.rs"),
                symbols: vec!["run_log_command".to_string()],
                doc_contains: Vec::new(),
            }],
        );

        let tracked = collect_trace_paths(
            "feature",
            "FEAT-LOG-001",
            "selected",
            "implementation",
            &traces,
        );
        assert_eq!(tracked[0].kind, "implementation");
        assert_eq!(tracked[0].path, "src/log.rs");
        assert_eq!(tracked[0].owner_kind, "feature");
        assert_eq!(tracked[0].owner_id, "FEAT-LOG-001");
        assert_eq!(tracked[0].language.as_deref(), Some("rust"));
        assert_eq!(tracked[0].symbols, vec!["run_log_command"]);
    }

    #[test]
    fn requirement_kind_branches_return_expected_paths() {
        let requirement = Requirement {
            id: "REQ-LOG-001".to_string(),
            title: "Requirement history".to_string(),
            description: "Requirement history".to_string(),
            priority: "medium".to_string(),
            status: "implemented".to_string(),
            linked_policies: Vec::new(),
            linked_features: Vec::new(),
            tests: trace_map("src/history_tests.rs", "history_test"),
        };

        let all = tracked_paths_for_requirement(&requirement, HistoryKind::All, "docs/req.yaml")
            .expect("all history should work");
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].kind, "definition");
        assert_eq!(all[1].kind, "test");

        let definition =
            tracked_paths_for_requirement(&requirement, HistoryKind::Definition, "docs/req.yaml")
                .expect("definition history should work");
        assert_eq!(definition.len(), 1);
        assert_eq!(definition[0].kind, "definition");

        let tests = tracked_paths_for_requirement(&requirement, HistoryKind::Test, "docs/req.yaml")
            .expect("test history should work");
        assert_eq!(tests.len(), 1);
        assert_eq!(tests[0].kind, "test");

        let error = tracked_paths_for_requirement(
            &requirement,
            HistoryKind::Implementation,
            "docs/req.yaml",
        )
        .expect_err("requirements should reject implementation history");
        assert!(error.to_string().contains("--kind implementation"));
    }

    #[test]
    fn feature_kind_branches_return_expected_paths() {
        let feature = Feature {
            id: "FEAT-LOG-001".to_string(),
            title: "Feature history".to_string(),
            summary: "Feature history".to_string(),
            status: "implemented".to_string(),
            linked_requirements: Vec::new(),
            implementations: trace_map("src/history_feature.rs", "history_feature"),
        };

        let all = tracked_paths_for_feature(&feature, HistoryKind::All, "docs/feat.yaml")
            .expect("all history should work");
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].kind, "definition");
        assert_eq!(all[1].kind, "implementation");

        let definition =
            tracked_paths_for_feature(&feature, HistoryKind::Definition, "docs/feat.yaml")
                .expect("definition history should work");
        assert_eq!(definition.len(), 1);
        assert_eq!(definition[0].kind, "definition");

        let implementations =
            tracked_paths_for_feature(&feature, HistoryKind::Implementation, "docs/feat.yaml")
                .expect("implementation history should work");
        assert_eq!(implementations.len(), 1);
        assert_eq!(implementations[0].kind, "implementation");

        let error = tracked_paths_for_feature(&feature, HistoryKind::Test, "docs/feat.yaml")
            .expect_err("features should reject test history");
        assert!(error.to_string().contains("--kind test"));
    }

    #[test]
    fn load_git_history_returns_empty_without_tracked_paths() {
        let history =
            load_git_history(Path::new("."), 5, &[], None).expect("empty tracked paths are okay");
        assert!(history.is_empty());
    }

    #[test]
    fn resolve_history_scope_returns_none_without_filters() {
        assert_eq!(
            resolve_history_scope(Path::new("."), None, None).expect("scope should resolve"),
            None
        );
    }

    #[test]
    fn resolve_history_scope_keeps_explicit_ranges() {
        let scope = resolve_history_scope(Path::new("."), Some("origin/main..HEAD"), None)
            .expect("scope should resolve")
            .expect("scope should exist");
        assert_eq!(scope.label, "range `origin/main..HEAD`");
        assert_eq!(scope.revision_range, "origin/main..HEAD");
    }

    #[test]
    fn resolve_history_scope_reports_merge_base_failures() {
        let fake_bin = tempdir().expect("tempdir should exist");
        let _path_guard = PathGuard::set(vec![fake_bin.path().to_path_buf()]);
        write_fake_git_for_scope_merge_base_failure(fake_bin.path());

        let error = resolve_history_scope(Path::new("/repo"), None, Some("origin/main"))
            .expect_err("merge-base failures should be reported");
        assert!(error.to_string().contains("failed to compute merge-base"));
        assert!(error.to_string().contains("synthetic merge-base failure"));
    }

    #[test]
    fn resolve_history_scope_rejects_empty_merge_base_output() {
        let fake_bin = tempdir().expect("tempdir should exist");
        let _path_guard = PathGuard::set(vec![fake_bin.path().to_path_buf()]);
        write_fake_git_for_scope_merge_base_empty_output(fake_bin.path());

        let error = resolve_history_scope(Path::new("/repo"), None, Some("origin/main"))
            .expect_err("empty merge-base output should fail");
        assert!(error.to_string().contains("empty base SHA"));
    }

    #[test]
    fn resolve_history_scope_reports_spawn_failures() {
        let fake_bin = tempdir().expect("tempdir should exist");
        let _path_guard = PathGuard::set(vec![fake_bin.path().to_path_buf()]);

        let error = resolve_history_scope(Path::new("/repo"), None, Some("origin/main"))
            .expect_err("missing git binary should fail");
        assert!(
            error
                .to_string()
                .contains("failed to run `git merge-base HEAD origin/main`")
        );
    }

    #[test]
    fn collect_related_tracked_paths_excludes_selected_definition_from_related_set() {
        let workspace_root = tempdir().expect("tempdir should exist");
        write_related_workspace_fixture(workspace_root.path());
        let workspace =
            crate::workspace::load_workspace(workspace_root.path()).expect("workspace should load");

        let tracked = super::collect_related_tracked_paths(
            &workspace,
            "REQ-HIST-001",
            HistoryKind::Definition,
            None,
        )
        .expect("related tracked paths should resolve");
        assert!(tracked.iter().any(|entry| {
            entry.owner_id == "FEAT-HIST-001"
                && entry.owner_kind == "feature"
                && entry.kind == "definition"
                && entry.source == "related"
        }));
        assert!(
            !tracked
                .iter()
                .any(|entry| entry.owner_id == "REQ-HIST-001" && entry.source == "related")
        );
    }

    #[test]
    fn run_log_command_supports_related_surface_and_merge_base_scope() {
        let _lock = PATH_LOCK.lock().unwrap_or_else(|err| err.into_inner());
        let workspace_root = tempdir().expect("tempdir should exist");
        write_related_workspace_fixture(workspace_root.path());
        init_test_git_repository(workspace_root.path());
        git(workspace_root.path(), &["branch", "review-base", "HEAD"]);

        let exit = super::run_log_command(&crate::cli::LogArgs {
            id: "REQ-HIST-001".to_string(),
            workspace: workspace_root.path().to_path_buf(),
            kind: HistoryKind::Test,
            path: Some(PathBuf::from("src")),
            include_related: true,
            merge_base_ref: Some("review-base".to_string()),
            range: None,
            limit: 5,
            format: crate::cli::OutputFormat::Json,
        })
        .expect("log command should succeed");

        assert_eq!(exit, 0);
    }

    #[test]
    fn order_commits_by_repository_history_uses_candidate_ancestry() {
        let _lock = PATH_LOCK.lock().unwrap_or_else(|err| err.into_inner());
        let repo = tempdir().expect("tempdir should exist");
        init_test_git_repository(repo.path());

        fs::write(repo.path().join("history.txt"), "old\n").expect("history file should write");
        git(repo.path(), &["add", "history.txt"]);
        git(repo.path(), &["commit", "-m", "feat: old history"]);
        let old_sha = git_stdout(repo.path(), &["rev-parse", "HEAD"]);

        fs::write(repo.path().join("history.txt"), "new\n").expect("history file should write");
        git(repo.path(), &["add", "history.txt"]);
        git(repo.path(), &["commit", "-m", "feat: new history"]);
        let new_sha = git_stdout(repo.path(), &["rev-parse", "HEAD"]);

        let commits = BTreeMap::from([
            (
                old_sha.clone(),
                MatchedCommit {
                    sha: old_sha,
                    short_sha: "old".to_string(),
                    summary: "feat: old history".to_string(),
                    author: "Tester".to_string(),
                    authored_at: "2026-04-13T00:00:00+00:00".to_string(),
                    reasons: vec![TrackedPath::definition(
                        "feature",
                        "FEAT-LOG-001",
                        "selected",
                        "history.txt",
                    )],
                },
            ),
            (
                new_sha.clone(),
                MatchedCommit {
                    sha: new_sha,
                    short_sha: "new".to_string(),
                    summary: "feat: new history".to_string(),
                    author: "Tester".to_string(),
                    authored_at: "2026-04-13T01:00:00+00:00".to_string(),
                    reasons: vec![TrackedPath::definition(
                        "feature",
                        "FEAT-LOG-001",
                        "selected",
                        "history.txt",
                    )],
                },
            ),
        ]);

        let ordered = order_commits_by_repository_history(repo.path(), commits, 10)
            .expect("candidate ancestry ordering should succeed");
        assert_eq!(ordered.len(), 2);
        assert_eq!(ordered[0].summary, "feat: new history");
        assert_eq!(ordered[1].summary, "feat: old history");
    }

    #[test]
    fn order_commits_by_repository_history_falls_back_when_merge_base_fails() {
        let _lock = PATH_LOCK.lock().unwrap_or_else(|err| err.into_inner());
        let repo = tempdir().expect("tempdir should exist");
        init_test_git_repository(repo.path());

        let commits = BTreeMap::from([
            (
                "missing-old".to_string(),
                MatchedCommit {
                    sha: "missing-old".to_string(),
                    short_sha: "missing-old".to_string(),
                    summary: "old".to_string(),
                    author: "Tester".to_string(),
                    authored_at: "2026-04-13T00:00:00+00:00".to_string(),
                    reasons: vec![TrackedPath::definition(
                        "feature",
                        "FEAT-LOG-001",
                        "selected",
                        "docs/old.yaml",
                    )],
                },
            ),
            (
                "missing-new".to_string(),
                MatchedCommit {
                    sha: "missing-new".to_string(),
                    short_sha: "missing-new".to_string(),
                    summary: "new".to_string(),
                    author: "Tester".to_string(),
                    authored_at: "2026-04-13T01:00:00+00:00".to_string(),
                    reasons: vec![TrackedPath::definition(
                        "feature",
                        "FEAT-LOG-001",
                        "selected",
                        "docs/new.yaml",
                    )],
                },
            ),
        ]);

        write_fake_git_for_merge_base_failure(repo.path());
        let original_path = std::env::var_os("PATH").unwrap_or_default();
        unsafe {
            std::env::set_var(
                "PATH",
                std::env::join_paths(
                    std::iter::once(repo.path().to_path_buf())
                        .chain(std::env::split_paths(&original_path)),
                )
                .expect("path should join"),
            );
        }

        let ordered = order_commits_by_repository_history(repo.path(), commits, 10)
            .expect("fallback ordering should succeed");
        unsafe {
            std::env::set_var("PATH", original_path);
        }
        assert_eq!(ordered.len(), 2);
        assert_eq!(ordered[0].sha, "missing-new");
        assert_eq!(ordered[1].sha, "missing-old");
    }

    #[test]
    fn order_commits_by_repository_history_sorts_multiple_ready_dependents_by_recency() {
        let fake_bin = tempdir().expect("tempdir should exist");
        write_fake_git_for_merge_base_graph(
            fake_bin.path(),
            &[
                ("old-a", "new", true),
                ("old-b", "new", true),
                ("old-a", "old-b", false),
                ("old-b", "old-a", false),
            ],
        );
        let _path_guard = PathGuard::set(vec![fake_bin.path().to_path_buf()]);

        let commits = BTreeMap::from([
            (
                "old-a".to_string(),
                MatchedCommit {
                    sha: "old-a".to_string(),
                    short_sha: "old-a".to_string(),
                    summary: "old-a".to_string(),
                    author: "Tester".to_string(),
                    authored_at: "2026-04-13T00:00:00+00:00".to_string(),
                    reasons: vec![TrackedPath::definition(
                        "feature",
                        "FEAT-LOG-001",
                        "selected",
                        "docs/old-a.yaml",
                    )],
                },
            ),
            (
                "old-b".to_string(),
                MatchedCommit {
                    sha: "old-b".to_string(),
                    short_sha: "old-b".to_string(),
                    summary: "old-b".to_string(),
                    author: "Tester".to_string(),
                    authored_at: "2026-04-13T01:00:00+00:00".to_string(),
                    reasons: vec![TrackedPath::definition(
                        "feature",
                        "FEAT-LOG-001",
                        "selected",
                        "docs/old-b.yaml",
                    )],
                },
            ),
            (
                "new".to_string(),
                MatchedCommit {
                    sha: "new".to_string(),
                    short_sha: "new".to_string(),
                    summary: "new".to_string(),
                    author: "Tester".to_string(),
                    authored_at: "2026-04-13T02:00:00+00:00".to_string(),
                    reasons: vec![TrackedPath::definition(
                        "feature",
                        "FEAT-LOG-001",
                        "selected",
                        "docs/new.yaml",
                    )],
                },
            ),
        ]);

        let ordered = order_commits_by_repository_history(Path::new("/repo"), commits, 10)
            .expect("ordering should succeed");
        assert_eq!(
            ordered
                .iter()
                .map(|commit| commit.sha.as_str())
                .collect::<Vec<_>>(),
            vec!["new", "old-b", "old-a"]
        );
    }

    #[test]
    fn order_commits_by_repository_history_sorts_newly_ready_dependents_by_recency() {
        let fake_bin = tempdir().expect("tempdir should exist");
        write_fake_git_for_merge_base_graph(
            fake_bin.path(),
            &[
                ("root", "mid-a", true),
                ("root", "mid-b", true),
                ("mid-a", "mid-b", false),
                ("mid-b", "mid-a", false),
            ],
        );
        let _path_guard = PathGuard::set(vec![fake_bin.path().to_path_buf()]);

        let commits = BTreeMap::from([
            (
                "root".to_string(),
                MatchedCommit {
                    sha: "root".to_string(),
                    short_sha: "root".to_string(),
                    summary: "root".to_string(),
                    author: "Tester".to_string(),
                    authored_at: "2026-04-13T00:00:00+00:00".to_string(),
                    reasons: vec![TrackedPath::definition(
                        "feature",
                        "FEAT-LOG-001",
                        "selected",
                        "docs/root.yaml",
                    )],
                },
            ),
            (
                "mid-a".to_string(),
                MatchedCommit {
                    sha: "mid-a".to_string(),
                    short_sha: "mid-a".to_string(),
                    summary: "mid-a".to_string(),
                    author: "Tester".to_string(),
                    authored_at: "2026-04-13T01:00:00+00:00".to_string(),
                    reasons: vec![TrackedPath::definition(
                        "feature",
                        "FEAT-LOG-001",
                        "selected",
                        "docs/mid-a.yaml",
                    )],
                },
            ),
            (
                "mid-b".to_string(),
                MatchedCommit {
                    sha: "mid-b".to_string(),
                    short_sha: "mid-b".to_string(),
                    summary: "mid-b".to_string(),
                    author: "Tester".to_string(),
                    authored_at: "2026-04-13T02:00:00+00:00".to_string(),
                    reasons: vec![TrackedPath::definition(
                        "feature",
                        "FEAT-LOG-001",
                        "selected",
                        "docs/mid-b.yaml",
                    )],
                },
            ),
        ]);

        let ordered = order_commits_by_repository_history(Path::new("/repo"), commits, 10)
            .expect("ordering should succeed");
        assert_eq!(
            ordered
                .iter()
                .map(|commit| commit.sha.as_str())
                .collect::<Vec<_>>(),
            vec!["mid-b", "mid-a", "root"]
        );
    }

    #[test]
    fn order_commits_by_repository_history_reports_inconsistent_ancestry_cycles() {
        let fake_bin = tempdir().expect("tempdir should exist");
        write_fake_git_for_merge_base_graph(
            fake_bin.path(),
            &[
                ("old-a", "old-b", true),
                ("old-b", "old-c", true),
                ("old-c", "old-a", true),
            ],
        );
        let _path_guard = PathGuard::set(vec![fake_bin.path().to_path_buf()]);

        let mut commits = vec![
            MatchedCommit {
                sha: "old-a".to_string(),
                short_sha: "old-a".to_string(),
                summary: "old-a".to_string(),
                author: "Tester".to_string(),
                authored_at: "2026-04-13T00:00:00+00:00".to_string(),
                reasons: vec![TrackedPath::definition(
                    "feature",
                    "FEAT-LOG-001",
                    "selected",
                    "docs/old-a.yaml",
                )],
            },
            MatchedCommit {
                sha: "old-b".to_string(),
                short_sha: "old-b".to_string(),
                summary: "old-b".to_string(),
                author: "Tester".to_string(),
                authored_at: "2026-04-13T01:00:00+00:00".to_string(),
                reasons: vec![TrackedPath::definition(
                    "feature",
                    "FEAT-LOG-001",
                    "selected",
                    "docs/old-b.yaml",
                )],
            },
            MatchedCommit {
                sha: "old-c".to_string(),
                short_sha: "old-c".to_string(),
                summary: "old-c".to_string(),
                author: "Tester".to_string(),
                authored_at: "2026-04-13T02:00:00+00:00".to_string(),
                reasons: vec![TrackedPath::definition(
                    "feature",
                    "FEAT-LOG-001",
                    "selected",
                    "docs/old-c.yaml",
                )],
            },
        ];

        let error = sort_commits_by_history_relationship(Path::new("/repo"), &mut commits)
            .expect_err("inconsistent ancestry should fail");
        assert!(
            error
                .to_string()
                .contains("could not derive a stable candidate history order")
        );
    }

    #[test]
    fn commit_is_ancestor_of_returns_cached_results_without_spawning_git() {
        let mut cache = BTreeMap::from([(("old".to_string(), "new".to_string()), true)]);
        assert!(
            commit_is_ancestor_of(Path::new("/repo"), "old", "new", &mut cache)
                .expect("cached lookups should succeed")
        );
    }

    #[test]
    fn commit_is_ancestor_of_reports_spawn_failures() {
        let fake_bin = tempdir().expect("tempdir should exist");
        let _path_guard = PathGuard::set(vec![fake_bin.path().to_path_buf()]);

        let error = commit_is_ancestor_of(Path::new("/repo"), "old", "new", &mut BTreeMap::new())
            .expect_err("missing git binary should fail");
        assert!(
            error
                .to_string()
                .contains("failed to run `git merge-base --is-ancestor`")
        );
    }

    #[test]
    fn parse_git_history_rejects_malformed_records() {
        let error = parse_git_history(b"\x1ebad-record\x00")
            .expect_err("malformed git records should be rejected");
        assert!(error.to_string().contains("unexpected `git log` output"));
    }

    #[test]
    fn parse_git_history_parses_commit_records_without_reason_filtering() {
        let raw = b"\x1esha\x00short\x00author\x002026-04-13T00:00:00+00:00\x00subject\x00\nsrc/other.rs\x00";
        let commits = parse_git_history(raw).expect("parsing should succeed");
        assert_eq!(commits.len(), 1);
        assert_eq!(commits[0].summary, "subject");
        assert!(commits[0].reasons.is_empty());
    }

    #[test]
    fn parse_git_history_ignores_invalid_utf8_file_names_after_the_subject() {
        let raw =
            b"\x1esha\x00short\x00author\x002026-04-13T00:00:00+00:00\x00subject\x00\n\xff\x00";
        let commits = parse_git_history(raw).expect("parsing should succeed");
        assert_eq!(commits.len(), 1);
        assert_eq!(commits[0].sha, "sha");
        assert!(commits[0].reasons.is_empty());
    }

    #[test]
    fn render_history_text_handles_empty_commit_lists() {
        let rendered = render_history_text(
            &super::HistoryTarget {
                id: "REQ-LOG-001".to_string(),
                entity_kind: "requirement",
                title: "Requirement history".to_string(),
                tracked_paths: vec![TrackedPath::definition(
                    "requirement",
                    "REQ-LOG-001",
                    "selected",
                    "docs/req.yaml",
                )],
            },
            Path::new("/repo"),
            HistoryKind::Definition,
            false,
            None,
            Some(Path::new("docs")),
            &[],
        );
        assert!(rendered.contains("Related surface: selected only"));
        assert!(rendered.contains("Path filter: docs"));
        assert!(rendered.contains("Commits:\n- none"));
    }

    #[test]
    fn render_history_text_lists_commit_reasons() {
        let rendered = render_history_text(
            &super::HistoryTarget {
                id: "FEAT-LOG-001".to_string(),
                entity_kind: "feature",
                title: "Feature history".to_string(),
                tracked_paths: vec![TrackedPath {
                    kind: "implementation",
                    path: "src/history.rs".to_string(),
                    owner_kind: "feature",
                    owner_id: "FEAT-LOG-001".to_string(),
                    source: "selected",
                    language: Some("rust".to_string()),
                    symbols: vec!["history".to_string()],
                }],
            },
            Path::new("/repo"),
            HistoryKind::Implementation,
            true,
            Some(&HistoryScope {
                label: "merge-base(origin/main)..HEAD".to_string(),
                revision_range: "abc123..HEAD".to_string(),
            }),
            None,
            &[MatchedCommit {
                sha: "abc".to_string(),
                short_sha: "abc".to_string(),
                summary: "feat: update history".to_string(),
                author: "Tester".to_string(),
                authored_at: "2026-04-13T00:00:00+00:00".to_string(),
                reasons: vec![TrackedPath {
                    kind: "implementation",
                    path: "src/history.rs".to_string(),
                    owner_kind: "feature",
                    owner_id: "FEAT-LOG-001".to_string(),
                    source: "selected",
                    language: Some("rust".to_string()),
                    symbols: vec!["history".to_string()],
                }],
            }],
        );
        assert!(rendered.contains("Related surface: included"));
        assert!(rendered.contains("Scope: merge-base(origin/main)..HEAD"));
        assert!(rendered.contains("feat: update history"));
        assert!(rendered.contains(
            "implementation\tsrc/history.rs\trust\t[`history`]\tfeature FEAT-LOG-001 (selected)"
        ));
    }

    #[test]
    fn history_kind_labels_match_cli_values() {
        assert_eq!(HistoryKind::All.label(), "all");
        assert_eq!(HistoryKind::Definition.label(), "definition");
        assert_eq!(HistoryKind::Test.label(), "test");
        assert_eq!(HistoryKind::Implementation.label(), "implementation");
    }

    #[test]
    fn sort_ready_commits_prefers_newer_commits_first() {
        let mut ready = vec!["older".to_string(), "newer".to_string()];
        let commit_lookup = BTreeMap::from([
            (
                "older".to_string(),
                MatchedCommit {
                    sha: "older".to_string(),
                    short_sha: "older".to_string(),
                    summary: "older".to_string(),
                    author: "Tester".to_string(),
                    authored_at: "2026-04-13T00:00:00+00:00".to_string(),
                    reasons: vec![TrackedPath::definition(
                        "feature",
                        "FEAT-LOG-001",
                        "selected",
                        "docs/older.yaml",
                    )],
                },
            ),
            (
                "newer".to_string(),
                MatchedCommit {
                    sha: "newer".to_string(),
                    short_sha: "newer".to_string(),
                    summary: "newer".to_string(),
                    author: "Tester".to_string(),
                    authored_at: "2026-04-13T01:00:00+00:00".to_string(),
                    reasons: vec![TrackedPath::definition(
                        "feature",
                        "FEAT-LOG-001",
                        "selected",
                        "docs/newer.yaml",
                    )],
                },
            ),
        ]);

        super::sort_ready_commits(&mut ready, &commit_lookup);

        assert_eq!(ready, vec!["older".to_string(), "newer".to_string()]);
    }

    fn trace_map(path: &str, symbol: &str) -> BTreeMap<String, Vec<TraceReference>> {
        let mut traces = BTreeMap::new();
        traces.insert(
            "rust".to_string(),
            vec![TraceReference {
                file: PathBuf::from(path),
                symbols: vec![symbol.to_string()],
                doc_contains: Vec::new(),
            }],
        );
        traces
    }

    fn write_related_workspace_fixture(root: &Path) {
        fs::create_dir_all(root.join("docs/syu/philosophy")).expect("philosophy dir");
        fs::create_dir_all(root.join("docs/syu/policies")).expect("policies dir");
        fs::create_dir_all(root.join("docs/syu/requirements")).expect("requirements dir");
        fs::create_dir_all(root.join("docs/syu/features/cli")).expect("features dir");
        fs::create_dir_all(root.join("src")).expect("src dir");

        fs::write(
            root.join("syu.yaml"),
            format!(
                "version: {}\nspec:\n  root: docs/syu\nvalidate:\n  default_fix: false\n  allow_planned: true\n  require_non_orphaned_items: true\n  require_symbol_trace_coverage: false\n",
                env!("CARGO_PKG_VERSION")
            ),
        )
        .expect("config");
        fs::write(
            root.join("docs/syu/philosophy/foundation.yaml"),
            "category: Philosophy\nversion: 1\nlanguage: en\nphilosophies:\n  - id: PHIL-HIST-001\n    title: History should stay explorable.\n    product_design_principle: Keep commit history close to trace links.\n    coding_guideline: Prefer one-command repository history lookups.\n    linked_policies:\n      - POL-HIST-001\n",
        )
        .expect("philosophy file");
        fs::write(
            root.join("docs/syu/policies/policies.yaml"),
            "category: Policies\nversion: 1\nlanguage: en\npolicies:\n  - id: POL-HIST-001\n    title: History should be reachable from traces.\n    summary: Git history is useful when it is derived from checked-in trace metadata.\n    description: The repository history should be explorable from requirement and feature traces.\n    linked_philosophies:\n      - PHIL-HIST-001\n    linked_requirements:\n      - REQ-HIST-001\n",
        )
        .expect("policy file");
        fs::write(
            root.join("docs/syu/requirements/core.yaml"),
            "category: Core\nprefix: REQ-HIST\n\nrequirements:\n  - id: REQ-HIST-001\n    title: Requirement history lookup\n    description: Requirement history should show the traced test and checked-in definition.\n    priority: medium\n    status: implemented\n    linked_policies:\n      - POL-HIST-001\n    linked_features:\n      - FEAT-HIST-001\n    tests:\n      rust:\n        - file: src/history_tests.rs\n          symbols:\n            - requirement_history_test\n",
        )
        .expect("requirement file");
        fs::write(
            root.join("docs/syu/features/features.yaml"),
            "version: \"1\"\nfiles:\n  - kind: history\n    file: cli/history.yaml\n",
        )
        .expect("feature registry");
        fs::write(
            root.join("docs/syu/features/cli/history.yaml"),
            "category: History\nversion: 1\nfeatures:\n  - id: FEAT-HIST-001\n    title: Feature history lookup\n    summary: Feature history should show the traced implementation and checked-in definition.\n    status: implemented\n    linked_requirements:\n      - REQ-HIST-001\n    implementations:\n      rust:\n        - file: src/history_feature.rs\n          symbols:\n            - feature_history\n",
        )
        .expect("feature file");
        fs::write(
            root.join("src/history_tests.rs"),
            "// REQ-HIST-001\nfn requirement_history_test() {}\n",
        )
        .expect("history test file");
        fs::write(
            root.join("src/history_feature.rs"),
            "// FEAT-HIST-001\nfn feature_history() {}\n",
        )
        .expect("history feature file");
    }

    fn write_fake_git_for_scope_merge_base_failure(script_dir: &Path) {
        let script_path = script_dir.join("git");
        fs::write(
            &script_path,
            "#!/bin/sh\nset -eu\nif [ \"$3\" = \"merge-base\" ]; then\n  echo 'synthetic merge-base failure' >&2\n  exit 2\nfi\necho 'unexpected git invocation' >&2\nexit 1\n",
        )
        .expect("fake git script");
        set_executable(&script_path);
    }

    fn write_fake_git_for_scope_merge_base_empty_output(script_dir: &Path) {
        let script_path = script_dir.join("git");
        fs::write(
            &script_path,
            "#!/bin/sh\nset -eu\nif [ \"$3\" = \"merge-base\" ]; then\n  exit 0\nfi\necho 'unexpected git invocation' >&2\nexit 1\n",
        )
        .expect("fake git script");
        set_executable(&script_path);
    }

    fn set_executable(path: &Path) {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            let mut permissions = fs::metadata(path)
                .expect("metadata should exist")
                .permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(path, permissions).expect("permissions should update");
        }
    }

    fn init_test_git_repository(path: &Path) {
        fs::write(path.join("README.md"), "seed\n").expect("seed file should write");
        git(path, &["init"]);
        git(path, &["config", "user.name", "Test User"]);
        git(path, &["config", "user.email", "test@example.com"]);
        git(path, &["add", "."]);
        git(path, &["commit", "-m", "chore: seed"]);
    }

    fn git(path: &Path, args: &[&str]) {
        let mut command = Command::new("git");
        command.arg("-C").arg(path).args(args);
        for key in [
            "GIT_ALTERNATE_OBJECT_DIRECTORIES",
            "GIT_CEILING_DIRECTORIES",
            "GIT_COMMON_DIR",
            "GIT_DIR",
            "GIT_INDEX_FILE",
            "GIT_OBJECT_DIRECTORY",
            "GIT_PREFIX",
            "GIT_WORK_TREE",
        ] {
            command.env_remove(key);
        }
        let output = command.output().expect("git should run");
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success(),
            "git {:?} failed\nstdout:\n{}\nstderr:\n{}",
            args,
            stdout,
            stderr
        );
    }

    fn git_stdout(path: &Path, args: &[&str]) -> String {
        let mut command = Command::new("git");
        command.arg("-C").arg(path).args(args);
        for key in [
            "GIT_ALTERNATE_OBJECT_DIRECTORIES",
            "GIT_CEILING_DIRECTORIES",
            "GIT_COMMON_DIR",
            "GIT_DIR",
            "GIT_INDEX_FILE",
            "GIT_OBJECT_DIRECTORY",
            "GIT_PREFIX",
            "GIT_WORK_TREE",
        ] {
            command.env_remove(key);
        }
        let output = command.output().expect("git should run");
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success(),
            "git {:?} failed\nstdout:\n{}\nstderr:\n{}",
            args,
            stdout,
            stderr
        );
        String::from_utf8(output.stdout)
            .expect("git output should be valid utf-8")
            .trim()
            .to_string()
    }

    fn write_fake_git_for_merge_base_failure(script_dir: &Path) {
        let script_path = script_dir.join("git");
        fs::write(
            &script_path,
            "#!/bin/sh\nset -eu\nworkspace=\"$2\"\ncommand_name=\"$3\"\nif [ \"$command_name\" = \"merge-base\" ]; then\n  echo 'synthetic git merge-base failure' >&2\n  exit 2\nfi\nexec /usr/bin/git \"$@\"\n",
        )
        .expect("fake git script");

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            let mut permissions = fs::metadata(&script_path)
                .expect("fake git metadata")
                .permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(&script_path, permissions).expect("fake git permissions");
        }
    }

    fn write_fake_git_for_merge_base_graph(script_dir: &Path, edges: &[(&str, &str, bool)]) {
        let cases = edges
            .iter()
            .map(|(older, newer, result)| {
                let status = if *result { 0 } else { 1 };
                format!(
                    "if [ \"$5\" = \"{older}\" ] && [ \"$6\" = \"{newer}\" ]; then\n  exit {status}\nfi\n"
                )
            })
            .collect::<String>();
        let script_path = script_dir.join("git");
        fs::write(
            &script_path,
            format!(
                "#!/bin/sh\nset -eu\nif [ \"$3\" != \"merge-base\" ]; then\n  echo 'unexpected git invocation' >&2\n  exit 1\nfi\n{cases}exit 1\n"
            ),
        )
        .expect("fake git graph script");

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            let mut permissions = fs::metadata(&script_path)
                .expect("fake git metadata")
                .permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(&script_path, permissions).expect("fake git permissions");
        }
    }
}
