// FEAT-LOG-001
// REQ-CORE-636

use std::{
    collections::{BTreeMap, BTreeSet},
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use anyhow::{Context, Result, bail};
use syu_core::{HistoricalIdSnapshot, SectionKind};

use crate::{
    config::SyuConfig,
    config::resolve_spec_root,
    model::{FeatureDocument, PhilosophyDocument, PolicyDocument, RequirementDocument},
    workspace::resolve_workspace_root,
};

#[derive(Debug, Default)]
pub(crate) struct HistoricalIdIndex {
    enabled: bool,
    available: bool,
    start_ref: Option<String>,
    ids_by_section: BTreeMap<SectionKind, BTreeSet<String>>,
    ids_by_value: BTreeSet<String>,
    deleted_by_value: BTreeMap<String, HistoricalIdOccurrence>,
}

impl HistoricalIdIndex {
    pub(crate) fn enabled(&self) -> bool {
        self.enabled
    }

    pub(crate) fn available(&self) -> bool {
        self.available
    }

    pub(crate) fn contains(&self, id: &str) -> bool {
        self.ids_by_value.contains(id)
    }

    pub(crate) fn deleted_entry(&self, id: &str) -> Option<&HistoricalIdOccurrence> {
        self.deleted_by_value.get(id)
    }

    #[allow(dead_code)]
    pub(crate) fn snapshot(&self) -> HistoricalIdSnapshot {
        HistoricalIdSnapshot {
            enabled: self.enabled,
            available: self.available,
            start_ref: self.start_ref.clone(),
            ids_by_section: self
                .ids_by_section
                .iter()
                .map(|(kind, ids)| (*kind, ids.iter().cloned().collect()))
                .collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct HistoricalIdOccurrence {
    pub(crate) section: SectionKind,
    pub(crate) path: PathBuf,
    pub(crate) commit: String,
}

#[derive(Debug, Default)]
struct CommitSnapshot {
    ids_by_section: BTreeMap<SectionKind, BTreeSet<String>>,
    ids_by_value: BTreeSet<String>,
    occurrences_by_value: BTreeMap<String, HistoricalIdOccurrence>,
}

type GitObjectId = String;
type BlobCacheKey = (SectionKind, GitObjectId);

#[cfg(test)]
thread_local! {
    static PARSED_HISTORICAL_BLOB_COUNT: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct GitTreeBlobEntry {
    path: PathBuf,
    blob_id: GitObjectId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum GitPathChange {
    Upsert { path: PathBuf, blob_id: GitObjectId },
    Delete { path: PathBuf },
}

#[derive(Debug, Clone)]
struct HistoricalPathState {
    section: SectionKind,
    blob_id: GitObjectId,
}

#[derive(Debug, Default, Clone)]
struct ParsedHistoricalBlob {
    ids_by_value: BTreeSet<String>,
}

pub(crate) fn build_historical_id_index(
    workspace_root: &Path,
    config: &SyuConfig,
) -> Result<HistoricalIdIndex> {
    let mut index = HistoricalIdIndex {
        enabled: config.validate.historical_ids.enabled,
        available: false,
        start_ref: config.validate.historical_ids.start_ref.clone(),
        ids_by_section: BTreeMap::from([
            (SectionKind::Philosophy, BTreeSet::new()),
            (SectionKind::Policies, BTreeSet::new()),
            (SectionKind::Requirements, BTreeSet::new()),
            (SectionKind::Features, BTreeSet::new()),
        ]),
        ids_by_value: BTreeSet::new(),
        deleted_by_value: BTreeMap::new(),
    };

    if !index.enabled {
        return Ok(index);
    }

    let workspace_root = resolve_workspace_root(workspace_root)?;
    let repository_root = match git_repository_root(&workspace_root) {
        Ok(root) => root,
        Err(_) => return Ok(index),
    };
    let spec_root = resolve_spec_root(&workspace_root, config);
    let spec_root_relative = match spec_root.strip_prefix(&repository_root) {
        Ok(relative) => relative.to_path_buf(),
        Err(_) => return Ok(index),
    };

    let mut previous_commit_ids = BTreeSet::new();
    let mut latest_occurrences = BTreeMap::new();

    let (initial_commit, commits) = if let Some(start_ref) = index.start_ref.clone() {
        (start_ref, Vec::new())
    } else {
        let mut commits = git_rev_list(&repository_root, "HEAD")?;
        let Some(initial_commit) = commits.first().cloned() else {
            index.available = true;
            return Ok(index);
        };
        commits.remove(0);
        (initial_commit, commits)
    };

    let mut path_index =
        build_historical_path_index(&repository_root, &initial_commit, &spec_root_relative)?;
    let mut blob_cache = BTreeMap::new();
    ensure_blobs_parsed(&repository_root, &path_index, &mut blob_cache)?;
    let snapshot =
        snapshot_from_cached_paths(&initial_commit, &path_index, &blob_cache, &mut index);
    update_historical_reuse_index(
        &mut index,
        &mut previous_commit_ids,
        &mut latest_occurrences,
        snapshot,
    );

    let mut previous_processed_commit = initial_commit;
    let commits = if let Some(start_ref) = index.start_ref.clone() {
        let commit_range = format!("{start_ref}..HEAD");
        git_rev_list(&repository_root, &commit_range)?
    } else {
        commits
    };
    for commit in commits {
        match git_diff_tree_blob_changes(
            &repository_root,
            &previous_processed_commit,
            &commit,
            &spec_root_relative,
        ) {
            Ok(changes) => {
                apply_historical_path_changes(&mut path_index, changes, &spec_root_relative)
            }
            Err(_) => {
                path_index =
                    build_historical_path_index(&repository_root, &commit, &spec_root_relative)?;
            }
        }
        ensure_blobs_parsed(&repository_root, &path_index, &mut blob_cache)?;
        let snapshot = snapshot_from_cached_paths(&commit, &path_index, &blob_cache, &mut index);
        update_historical_reuse_index(
            &mut index,
            &mut previous_commit_ids,
            &mut latest_occurrences,
            snapshot,
        );
        previous_processed_commit = commit;
    }

    index.available = true;
    Ok(index)
}

fn update_historical_reuse_index(
    index: &mut HistoricalIdIndex,
    previous_commit_ids: &mut BTreeSet<String>,
    latest_occurrences: &mut BTreeMap<String, HistoricalIdOccurrence>,
    snapshot: CommitSnapshot,
) {
    for id in previous_commit_ids.difference(&snapshot.ids_by_value) {
        if index.deleted_by_value.contains_key(id) {
            continue;
        }

        if let Some(previous_occurrence) = latest_occurrences.get(id) {
            index
                .deleted_by_value
                .insert(id.clone(), previous_occurrence.clone());
        }
    }

    for (id, occurrence) in snapshot.occurrences_by_value {
        latest_occurrences.insert(id, occurrence);
    }

    *previous_commit_ids = snapshot.ids_by_value;
}

fn git_repository_root(workspace_root: &Path) -> Result<PathBuf> {
    let output = git_command(workspace_root)
        .arg("rev-parse")
        .arg("--show-toplevel")
        .output()
        .with_context(|| {
            format!(
                "failed to run `git rev-parse` in `{}`",
                workspace_root.display()
            )
        })?;
    if !output.status.success() {
        bail!(
            "workspace `{}` is not inside a Git repository, so historical IDs cannot be indexed.",
            workspace_root.display()
        );
    }

    parse_git_repository_root_stdout(output.stdout, workspace_root)
}

fn parse_git_repository_root_stdout(stdout: Vec<u8>, workspace_root: &Path) -> Result<PathBuf> {
    let root = String::from_utf8(stdout).context("git repository root should be valid UTF-8")?;
    let root = root.trim();
    if root.is_empty() {
        bail!(
            "git rev-parse returned an empty repository root for `{}`",
            workspace_root.display()
        );
    }

    Ok(PathBuf::from(root))
}

fn git_rev_list(repository_root: &Path, rev_range: &str) -> Result<Vec<String>> {
    let output = git_command(repository_root)
        .arg("rev-list")
        .arg("--reverse")
        .arg(rev_range)
        .output()
        .with_context(|| {
            format!(
                "failed to run `git rev-list {rev_range}` in `{}`",
                repository_root.display()
            )
        })?;
    if !output.status.success() {
        bail!(
            "failed to enumerate historical commits for `{}`: {}",
            rev_range,
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }

    let stdout =
        String::from_utf8(output.stdout).context("git rev-list output should be valid UTF-8")?;
    Ok(stdout
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect())
}

#[cfg(test)]
fn record_commit_snapshot(
    repository_root: &Path,
    spec_root_relative: &Path,
    commit: &str,
    index: &mut HistoricalIdIndex,
) -> Result<CommitSnapshot> {
    let path_index = build_historical_path_index(repository_root, commit, spec_root_relative)?;
    let mut blob_cache = BTreeMap::new();
    ensure_blobs_parsed(repository_root, &path_index, &mut blob_cache)?;
    Ok(snapshot_from_cached_paths(
        commit,
        &path_index,
        &blob_cache,
        index,
    ))
}

fn build_historical_path_index(
    repository_root: &Path,
    commit: &str,
    spec_root_relative: &Path,
) -> Result<BTreeMap<PathBuf, HistoricalPathState>> {
    let mut path_index = BTreeMap::new();
    for entry in git_tree_blob_entries(repository_root, commit, spec_root_relative)? {
        let Some(section) = historical_section_for_path(&entry.path, spec_root_relative) else {
            continue;
        };
        path_index.insert(
            entry.path,
            HistoricalPathState {
                section,
                blob_id: entry.blob_id,
            },
        );
    }
    Ok(path_index)
}

fn apply_historical_path_changes(
    path_index: &mut BTreeMap<PathBuf, HistoricalPathState>,
    changes: Vec<GitPathChange>,
    spec_root_relative: &Path,
) {
    for change in changes {
        match change {
            GitPathChange::Upsert { path, blob_id } => {
                let Some(section) = historical_section_for_path(&path, spec_root_relative) else {
                    path_index.remove(&path);
                    continue;
                };
                path_index.insert(path, HistoricalPathState { section, blob_id });
            }
            GitPathChange::Delete { path } => {
                path_index.remove(&path);
            }
        }
    }
}

fn ensure_blobs_parsed(
    repository_root: &Path,
    path_index: &BTreeMap<PathBuf, HistoricalPathState>,
    blob_cache: &mut BTreeMap<BlobCacheKey, ParsedHistoricalBlob>,
) -> Result<()> {
    let missing_keys = path_index
        .values()
        .map(|state| (state.section, state.blob_id.clone()))
        .filter(|key| !blob_cache.contains_key(key))
        .collect::<BTreeSet<_>>();
    let blobs = git_cat_file_batch(
        repository_root,
        missing_keys.iter().map(|(_, blob_id)| blob_id.clone()),
    )?;
    for (section, blob_id) in missing_keys {
        let Some(raw) = blobs.get(&blob_id) else {
            continue;
        };
        let parsed = parse_historical_blob_raw(section, raw, &blob_id)?;
        blob_cache.insert((section, blob_id), parsed);
    }
    Ok(())
}

fn snapshot_from_cached_paths(
    commit: &str,
    path_index: &BTreeMap<PathBuf, HistoricalPathState>,
    blob_cache: &BTreeMap<BlobCacheKey, ParsedHistoricalBlob>,
    index: &mut HistoricalIdIndex,
) -> CommitSnapshot {
    let mut snapshot = CommitSnapshot::default();
    for (path, state) in path_index {
        let Some(parsed) = blob_cache.get(&(state.section, state.blob_id.clone())) else {
            continue;
        };
        for id in &parsed.ids_by_value {
            record_id(
                index,
                &mut snapshot,
                state.section,
                path,
                commit,
                id.clone(),
            );
        }
    }
    snapshot
}

fn parse_historical_blob_raw(
    section: SectionKind,
    raw: &str,
    object_id: &str,
) -> Result<ParsedHistoricalBlob> {
    #[cfg(test)]
    PARSED_HISTORICAL_BLOB_COUNT.with(|count| count.set(count.get() + 1));

    let mut parsed = ParsedHistoricalBlob::default();
    match section {
        SectionKind::Philosophy => {
            let document: PhilosophyDocument = serde_yaml::from_str(raw)
                .with_context(|| format!("failed to parse historical blob object `{object_id}`"))?;
            parsed
                .ids_by_value
                .extend(document.philosophies.into_iter().map(|item| item.id));
        }
        SectionKind::Policies => {
            let document: PolicyDocument = serde_yaml::from_str(raw)
                .with_context(|| format!("failed to parse historical blob object `{object_id}`"))?;
            parsed
                .ids_by_value
                .extend(document.policies.into_iter().map(|item| item.id));
        }
        SectionKind::Requirements => {
            let document: RequirementDocument = serde_yaml::from_str(raw)
                .with_context(|| format!("failed to parse historical blob object `{object_id}`"))?;
            parsed
                .ids_by_value
                .extend(document.requirements.into_iter().map(|item| item.id));
        }
        SectionKind::Features => {
            let Some(document) = parse_feature_blob_raw(raw, object_id)? else {
                return Ok(parsed);
            };
            parsed
                .ids_by_value
                .extend(document.features.into_iter().map(|item| item.id));
        }
    }
    Ok(parsed)
}

#[cfg(test)]
fn parse_feature_blob(
    repository_root: &Path,
    commit: &str,
    path: &str,
) -> Result<Option<FeatureDocument>> {
    let Some(raw) = git_blob(repository_root, commit, path)? else {
        return Ok(None);
    };
    parse_feature_blob_raw(&raw, &format!("{commit}:{path}"))
}

fn parse_feature_blob_raw(raw: &str, source: &str) -> Result<Option<FeatureDocument>> {
    let value: serde_yaml::Value = match serde_yaml::from_str(raw) {
        Ok(value) => value,
        Err(_) => return Ok(None),
    };
    let Some(mapping) = value.as_mapping() else {
        return Ok(None);
    };
    if !mapping.contains_key(serde_yaml::Value::String("features".to_string())) {
        return Ok(None);
    }
    Ok(Some(serde_yaml::from_str(raw).with_context(|| {
        format!("failed to parse feature document `{source}`")
    })?))
}

#[cfg(test)]
fn parse_blob<T>(repository_root: &Path, commit: &str, path: &str) -> Result<Option<T>>
where
    T: for<'de> serde::Deserialize<'de>,
{
    let Some(raw) = git_blob(repository_root, commit, path)? else {
        return Ok(None);
    };
    Ok(Some(serde_yaml::from_str(&raw).with_context(|| {
        format!("failed to parse historical blob `{path}` at `{commit}`")
    })?))
}

#[cfg(test)]
fn git_blob(repository_root: &Path, commit: &str, path: &str) -> Result<Option<String>> {
    let spec = format!("{commit}:{path}");
    let output = git_command(repository_root)
        .arg("show")
        .arg(&spec)
        .output()
        .with_context(|| {
            format!(
                "failed to run `git show {spec}` in `{}`",
                repository_root.display()
            )
        })?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if git_blob_missing(&stderr, commit, path) {
            return Ok(None);
        }

        bail!(
            "failed to read historical blob `{path}` at `{commit}` in `{}`: {}",
            repository_root.display(),
            stderr.trim()
        );
    }
    Ok(Some(
        String::from_utf8(output.stdout).context("git blob output should be valid UTF-8")?,
    ))
}

#[cfg(test)]
fn git_blob_missing(stderr: &str, commit: &str, path: &str) -> bool {
    stderr.contains(&format!("path '{path}' does not exist in '{commit}'"))
}

#[cfg(test)]
fn git_tree_files(
    repository_root: &Path,
    commit: &str,
    spec_root_relative: &Path,
) -> Result<Vec<String>> {
    Ok(
        git_tree_blob_entries(repository_root, commit, spec_root_relative)?
            .into_iter()
            .map(|entry| path_to_git_string(&entry.path))
            .collect(),
    )
}

fn git_tree_blob_entries(
    repository_root: &Path,
    commit: &str,
    spec_root_relative: &Path,
) -> Result<Vec<GitTreeBlobEntry>> {
    let output = git_command(repository_root)
        .arg("ls-tree")
        .arg("-r")
        .arg("-z")
        .arg(commit)
        .arg("--")
        .args(spec_root_arg(spec_root_relative))
        .output()
        .with_context(|| {
            format!(
                "failed to run `git ls-tree {commit}` in `{}`",
                repository_root.display()
            )
        })?;
    if !output.status.success() {
        bail!(
            "failed to enumerate historical files for `{commit}` in `{}`: {}",
            repository_root.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }

    parse_git_tree_blob_entries(output.stdout)
}

fn parse_git_tree_blob_entries(stdout: Vec<u8>) -> Result<Vec<GitTreeBlobEntry>> {
    let mut entries = Vec::new();
    for record in stdout
        .split(|byte| *byte == 0)
        .filter(|record| !record.is_empty())
    {
        let record = std::str::from_utf8(record).context("git ls-tree output should be UTF-8")?;
        let Some((metadata, path)) = record.split_once('\t') else {
            bail!("failed to parse git ls-tree record `{record}`");
        };
        let mut fields = metadata.split_whitespace();
        let _mode = fields.next();
        let object_type = fields.next();
        let object_id = fields.next();
        if object_type != Some("blob") {
            continue;
        }
        let Some(object_id) = object_id else {
            bail!("failed to parse object id from git ls-tree record `{record}`");
        };
        entries.push(GitTreeBlobEntry {
            path: PathBuf::from(path),
            blob_id: object_id.to_string(),
        });
    }
    Ok(entries)
}

fn git_diff_tree_blob_changes(
    repository_root: &Path,
    previous_commit: &str,
    commit: &str,
    spec_root_relative: &Path,
) -> Result<Vec<GitPathChange>> {
    let output = git_command(repository_root)
        .arg("diff-tree")
        .arg("--raw")
        .arg("-z")
        .arg("--no-abbrev")
        .arg("-M")
        .arg("-r")
        .arg(previous_commit)
        .arg(commit)
        .arg("--")
        .args(spec_root_arg(spec_root_relative))
        .output()
        .with_context(|| {
            format!(
                "failed to run `git diff-tree {previous_commit} {commit}` in `{}`",
                repository_root.display()
            )
        })?;
    if !output.status.success() {
        bail!(
            "failed to enumerate historical changes for `{previous_commit}..{commit}` in `{}`: {}",
            repository_root.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }

    parse_git_diff_tree_blob_changes(output.stdout)
}

fn parse_git_diff_tree_blob_changes(stdout: Vec<u8>) -> Result<Vec<GitPathChange>> {
    let mut changes = Vec::new();
    let records = stdout
        .split(|byte| *byte == 0)
        .filter(|record| !record.is_empty())
        .collect::<Vec<_>>();
    let mut index = 0;
    while index < records.len() {
        let metadata =
            std::str::from_utf8(records[index]).context("git diff-tree output should be UTF-8")?;
        index += 1;
        let fields = metadata.split_whitespace().collect::<Vec<_>>();
        if fields.len() < 5 {
            bail!("failed to parse git diff-tree record `{metadata}`");
        }
        let new_oid = fields[3].to_string();
        let status = fields[4];
        let status_kind = status
            .chars()
            .next()
            .context("git diff-tree status should not be empty")?;
        let path_count = if matches!(status_kind, 'R' | 'C') {
            2
        } else {
            1
        };
        if index + path_count > records.len() {
            bail!("git diff-tree record `{metadata}` is missing path data");
        }
        let old_path = path_from_git_record(records[index])?;
        index += 1;
        let new_path = if path_count == 2 {
            let path = path_from_git_record(records[index])?;
            index += 1;
            Some(path)
        } else {
            None
        };

        match status_kind {
            'A' | 'M' | 'T' => changes.push(GitPathChange::Upsert {
                path: old_path,
                blob_id: new_oid,
            }),
            'D' => changes.push(GitPathChange::Delete { path: old_path }),
            'R' => {
                changes.push(GitPathChange::Delete { path: old_path });
                let Some(path) = new_path else {
                    bail!("rename record `{metadata}` is missing new path");
                };
                changes.push(GitPathChange::Upsert {
                    path,
                    blob_id: new_oid,
                });
            }
            'C' => {
                let Some(path) = new_path else {
                    bail!("copy record `{metadata}` is missing new path");
                };
                changes.push(GitPathChange::Upsert {
                    path,
                    blob_id: new_oid,
                });
            }
            _ => bail!("unsupported git diff-tree status `{status}`"),
        }
    }
    Ok(changes)
}

fn path_from_git_record(record: &[u8]) -> Result<PathBuf> {
    Ok(PathBuf::from(
        std::str::from_utf8(record).context("git path output should be UTF-8")?,
    ))
}

fn git_cat_file_batch(
    repository_root: &Path,
    object_ids: impl IntoIterator<Item = String>,
) -> Result<BTreeMap<String, String>> {
    let object_ids = object_ids.into_iter().collect::<BTreeSet<_>>();
    if object_ids.is_empty() {
        return Ok(BTreeMap::new());
    }

    let mut child = git_command(repository_root)
        .arg("cat-file")
        .arg("--batch")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| {
            format!(
                "failed to run `git cat-file --batch` in `{}`",
                repository_root.display()
            )
        })?;
    {
        let stdin = child
            .stdin
            .as_mut()
            .context("git cat-file stdin should be available")?;
        for object_id in &object_ids {
            writeln!(stdin, "{object_id}").context("failed to write object id to git cat-file")?;
        }
    }
    let output = child
        .wait_with_output()
        .context("failed to read git cat-file output")?;
    if !output.status.success() {
        bail!(
            "failed to read historical blobs in `{}`: {}",
            repository_root.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }

    parse_git_cat_file_batch(output.stdout)
}

fn parse_git_cat_file_batch(stdout: Vec<u8>) -> Result<BTreeMap<String, String>> {
    let mut blobs = BTreeMap::new();
    let mut offset = 0;
    while offset < stdout.len() {
        let Some(relative_header_end) = stdout[offset..].iter().position(|byte| *byte == b'\n')
        else {
            bail!("git cat-file output ended before a blob header");
        };
        let header_end = offset + relative_header_end;
        let header = std::str::from_utf8(&stdout[offset..header_end])
            .context("git cat-file header should be UTF-8")?;
        offset = header_end + 1;

        let fields = header.split_whitespace().collect::<Vec<_>>();
        if fields.len() != 3 {
            bail!("failed to parse git cat-file header `{header}`");
        }
        let object_id = fields[0].to_string();
        let object_type = fields[1];
        let size = fields[2]
            .parse::<usize>()
            .with_context(|| format!("failed to parse git cat-file size from `{header}`"))?;
        if object_type != "blob" {
            bail!("expected Git blob `{object_id}`, found `{object_type}`");
        }
        let content_end = offset + size;
        if content_end > stdout.len() {
            bail!("git cat-file blob `{object_id}` ended before {size} bytes");
        }
        let content = String::from_utf8(stdout[offset..content_end].to_vec())
            .with_context(|| format!("git blob `{object_id}` should be valid UTF-8"))?;
        blobs.insert(object_id, content);
        offset = content_end;
        if offset < stdout.len() && stdout[offset] == b'\n' {
            offset += 1;
        }
    }
    Ok(blobs)
}

fn spec_root_arg(spec_root_relative: &Path) -> Vec<&Path> {
    if spec_root_relative.as_os_str().is_empty() {
        Vec::new()
    } else {
        vec![spec_root_relative]
    }
}

fn is_yaml_file(path: &str) -> bool {
    path.ends_with(".yaml") || path.ends_with(".yml")
}

fn historical_section_for_path(path: &Path, spec_root_relative: &Path) -> Option<SectionKind> {
    let relative = if spec_root_relative.as_os_str().is_empty() {
        path
    } else {
        path.strip_prefix(spec_root_relative).ok()?
    };
    let relative = path_to_git_string(relative);
    if !is_yaml_file(&relative) {
        return None;
    }

    let (section, rest) = relative.split_once('/')?;
    match section {
        "philosophy" => Some(SectionKind::Philosophy),
        "policies" => Some(SectionKind::Policies),
        "requirements" => Some(SectionKind::Requirements),
        "features" if rest != "features.yaml" => Some(SectionKind::Features),
        _ => None,
    }
}

fn path_to_git_string(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn record_id(
    index: &mut HistoricalIdIndex,
    snapshot: &mut CommitSnapshot,
    kind: SectionKind,
    path: &Path,
    commit: &str,
    id: String,
) {
    index.ids_by_value.insert(id.clone());
    index
        .ids_by_section
        .entry(kind)
        .or_default()
        .insert(id.clone());
    snapshot.ids_by_value.insert(id.clone());
    snapshot
        .ids_by_section
        .entry(kind)
        .or_default()
        .insert(id.clone());
    snapshot
        .occurrences_by_value
        .entry(id)
        .or_insert_with(|| HistoricalIdOccurrence {
            section: kind,
            path: path.to_path_buf(),
            commit: commit.to_string(),
        });
}

fn git_command(workspace_root: &Path) -> Command {
    let mut command = Command::new("git");
    command.current_dir(workspace_root);
    command
}

#[cfg(test)]
mod tests {
    // REQ-CORE-024
    use super::*;

    use std::{
        fs,
        process::Command,
        sync::atomic::{AtomicU64, Ordering},
    };

    use tempfile::tempdir;

    static COMMIT_TIMESTAMP: AtomicU64 = AtomicU64::new(1_776_355_200);

    fn git(workspace: &Path, args: &[&str]) {
        let mut command = Command::new("git");
        command.arg("-C").arg(workspace).args(args);
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

    fn git_stdout(workspace: &Path, args: &[&str]) -> String {
        let mut command = Command::new("git");
        command.arg("-C").arg(workspace).args(args);
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
            .expect("git output should be utf8")
            .trim()
            .to_string()
    }

    fn git_commit(workspace: &Path, summary: &str) {
        let timestamp = COMMIT_TIMESTAMP.fetch_add(1, Ordering::Relaxed);
        let timestamp = format!("{timestamp} +0000");
        let mut command = Command::new("git");
        command
            .arg("-C")
            .arg(workspace)
            .args(["commit", "-m", summary])
            .env("GIT_AUTHOR_DATE", &timestamp)
            .env("GIT_COMMITTER_DATE", &timestamp);
        let output = command.output().expect("git commit should run");
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success(),
            "git commit failed\nstdout:\n{}\nstderr:\n{}",
            stdout,
            stderr
        );
    }

    fn init_git_repository(workspace: &Path) {
        git(workspace, &["init"]);
        git(workspace, &["config", "user.name", "Test User"]);
        git(workspace, &["config", "user.email", "test@example.com"]);
    }

    #[test]
    fn build_historical_id_index_returns_disabled_index_when_disabled() {
        let tempdir = tempdir().expect("tempdir should exist");
        let workspace = tempdir.path();
        let mut config = SyuConfig::default();
        config.validate.historical_ids.enabled = false;

        let index = build_historical_id_index(workspace, &config).expect("index should build");
        assert!(!index.enabled());
        assert!(!index.available());
        assert!(!index.contains("PHIL-HIST-000"));
    }

    #[test]
    fn update_historical_reuse_index_skips_already_deleted_ids() {
        let mut index = HistoricalIdIndex::default();
        let deleted_id = "REQ-HIST-DELETE-001".to_string();
        index.deleted_by_value.insert(
            deleted_id.clone(),
            HistoricalIdOccurrence {
                section: SectionKind::Requirements,
                path: PathBuf::from("requirements/core/req.yaml"),
                commit: "abc123".to_string(),
            },
        );
        let mut previous_commit_ids = BTreeSet::from([deleted_id.clone()]);
        let mut latest_occurrences = BTreeMap::from([(
            deleted_id.clone(),
            HistoricalIdOccurrence {
                section: SectionKind::Requirements,
                path: PathBuf::from("requirements/core/req.yaml"),
                commit: "abc123".to_string(),
            },
        )]);
        let snapshot = CommitSnapshot {
            ids_by_section: BTreeMap::new(),
            ids_by_value: BTreeSet::new(),
            occurrences_by_value: BTreeMap::new(),
        };

        update_historical_reuse_index(
            &mut index,
            &mut previous_commit_ids,
            &mut latest_occurrences,
            snapshot,
        );

        assert_eq!(index.deleted_by_value.len(), 1);
        assert!(index.deleted_by_value.contains_key(&deleted_id));
    }

    #[test]
    fn build_historical_id_index_supports_repository_root_spec_roots() {
        let tempdir = tempdir().expect("tempdir should exist");
        let workspace = tempdir.path();
        init_git_repository(workspace);

        fs::create_dir_all(workspace.join("philosophy")).expect("philosophy dir");
        fs::create_dir_all(workspace.join("policies")).expect("policies dir");
        fs::create_dir_all(workspace.join("requirements")).expect("requirements dir");
        fs::create_dir_all(workspace.join("features")).expect("features dir");

        fs::write(
            workspace.join("philosophy/foundation.yaml"),
            "category: Philosophy\nversion: 1\nlanguage: en\nphilosophies:\n  - id: PHIL-HIST-ROOT-001\n    title: Root history should be indexed.\n    product_design_principle: Keep the root spec root indexed.\n    coding_guideline: Support repository-root configurations.\n    linked_policies: []\n",
        )
        .expect("philosophy file");
        fs::write(
            workspace.join("features/features.yaml"),
            "version: \"1\"\nfiles: []\n",
        )
        .expect("feature registry");

        git(workspace, &["add", "."]);
        git_commit(workspace, "docs: add root historical ids");

        let config = SyuConfig {
            spec: crate::config::SpecConfig {
                root: PathBuf::from("."),
            },
            ..SyuConfig::default()
        };

        let index = build_historical_id_index(workspace, &config).expect("index should build");
        assert!(index.available());
        assert!(index.contains("PHIL-HIST-ROOT-001"));
    }

    #[test]
    fn record_commit_snapshot_reads_matching_files() {
        let tempdir = tempdir().expect("tempdir should exist");
        let workspace = tempdir.path();
        init_git_repository(workspace);

        fs::create_dir_all(workspace.join("philosophy")).expect("philosophy dir");
        fs::write(
            workspace.join("philosophy/foundation.yaml"),
            "category: Philosophy\nversion: 1\nlanguage: en\nphilosophies:\n  - id: PHIL-HIST-SNAPSHOT-001\n    title: Snapshot history should be indexed.\n    product_design_principle: Record files from the commit snapshot.\n    coding_guideline: Keep snapshot coverage explicit.\n    linked_policies: []\n",
        )
        .expect("philosophy file");

        git(workspace, &["add", "."]);
        git_commit(workspace, "docs: add snapshot historical ids");
        let commit = git_stdout(workspace, &["rev-parse", "HEAD"]);

        let mut index = HistoricalIdIndex::default();
        let snapshot = record_commit_snapshot(workspace, Path::new(""), &commit, &mut index)
            .expect("snapshot should build");

        assert!(snapshot.ids_by_value.contains("PHIL-HIST-SNAPSHOT-001"));
    }

    #[test]
    fn build_historical_id_index_skips_external_spec_roots() {
        let workspace_dir = tempdir().expect("tempdir should exist");
        let workspace = workspace_dir.path();
        init_git_repository(workspace);
        fs::write(workspace.join("placeholder.txt"), "placeholder\n").expect("placeholder file");
        git(workspace, &["add", "."]);
        git_commit(workspace, "chore: initialize repository");

        let external = tempdir().expect("external tempdir should exist");
        let mut config = SyuConfig::default();
        config.spec.root = external.path().join("external-spec");

        let index = build_historical_id_index(workspace, &config).expect("index should build");
        assert!(index.enabled());
        assert!(!index.available());
    }

    #[test]
    fn build_historical_id_index_returns_unavailable_when_workspace_is_not_a_git_repository() {
        let tempdir = tempdir().expect("tempdir should exist");
        let workspace = tempdir.path();
        let config = SyuConfig::default();

        let index = build_historical_id_index(workspace, &config).expect("index should build");
        assert!(index.enabled());
        assert!(!index.available());
    }

    #[test]
    fn build_historical_id_index_honors_start_refs_and_skips_feature_docs_without_features_lists() {
        let tempdir = tempdir().expect("tempdir should exist");
        let workspace = tempdir.path();
        init_git_repository(workspace);

        fs::create_dir_all(workspace.join("philosophy")).expect("philosophy dir");
        fs::create_dir_all(workspace.join("policies")).expect("policies dir");
        fs::create_dir_all(workspace.join("requirements/core")).expect("requirements dir");
        fs::create_dir_all(workspace.join("features")).expect("features dir");

        fs::write(
            workspace.join("philosophy/foundation.yaml"),
            "category: Philosophy\nversion: 1\nlanguage: en\nphilosophies:\n  - id: PHIL-HIST-START-001\n    title: Start refs should be indexed.\n    product_design_principle: History should include the start ref snapshot.\n    coding_guideline: Keep invalid feature documents out of the index.\n    linked_policies: []\n",
        )
        .expect("philosophy file");
        fs::write(
            workspace.join("policies/policies.yaml"),
            "category: Policy\nversion: 1\nlanguage: en\npolicies:\n  - id: POL-HIST-START-001\n    title: Policy history should be indexed.\n    summary: Start refs should include policy documents.\n    description: Cover the policy branch in the historical index.\n    linked_philosophies: []\n    linked_requirements: []\n",
        )
        .expect("policy file");
        fs::write(
            workspace.join("requirements/core/req.yaml"),
            "category: Core History\nprefix: REQ-HIST\nrequirements:\n  - id: REQ-HIST-START-001\n    title: Requirement history should be indexed.\n    description: Start refs should include requirement documents.\n    priority: medium\n    status: implemented\n    linked_policies: []\n    linked_features: []\n    tests: {}\n",
        )
        .expect("requirement file");
        fs::write(
            workspace.join("features/features.yaml"),
            "version: \"1\"\nfiles: []\n",
        )
        .expect("feature registry");
        fs::write(
            workspace.join("features/valid.yaml"),
            "category: History\nversion: 1\nfeatures:\n  - id: FEAT-HIST-START-001\n    title: Start refs should be indexed.\n    summary: Start refs should include valid feature documents.\n    status: implemented\n    linked_requirements: []\n    implementations: {}\n",
        )
        .expect("feature file");
        fs::write(workspace.join("notes.txt"), "ignore me\n").expect("non-yaml file");

        git(workspace, &["add", "."]);
        git_commit(workspace, "docs: add historical id start ref fixture");
        let start_ref = git_stdout(workspace, &["rev-parse", "HEAD"]);

        fs::write(
            workspace.join("features/broken.yaml"),
            "category: History\nversion: 1\nstories: []\n",
        )
        .expect("broken feature file");
        git(workspace, &["add", "."]);
        git_commit(workspace, "docs: add ignored historical feature fixture");

        let mut config = SyuConfig::default();
        config.spec.root = PathBuf::from(".");
        config.validate.historical_ids.start_ref = Some(start_ref);

        let index = build_historical_id_index(workspace, &config).expect("index should build");
        assert!(index.available());
        assert!(index.contains("PHIL-HIST-START-001"));
        assert!(index.contains("POL-HIST-START-001"));
        assert!(index.contains("REQ-HIST-START-001"));
        assert!(index.contains("FEAT-HIST-START-001"));
    }

    #[test]
    fn build_historical_id_index_handles_empty_historical_documents() {
        let tempdir = tempdir().expect("tempdir should exist");
        let workspace = tempdir.path();
        init_git_repository(workspace);

        fs::create_dir_all(workspace.join("philosophy")).expect("philosophy dir");
        fs::create_dir_all(workspace.join("policies")).expect("policies dir");
        fs::create_dir_all(workspace.join("requirements/core")).expect("requirements dir");
        fs::create_dir_all(workspace.join("features")).expect("features dir");
        fs::write(
            workspace.join("philosophy/empty.yaml"),
            "category: Philosophy\nversion: 1\nlanguage: en\nphilosophies: []\n",
        )
        .expect("philosophy file");
        fs::write(
            workspace.join("policies/empty.yaml"),
            "category: Policy\nversion: 1\nlanguage: en\npolicies: []\n",
        )
        .expect("policy file");
        fs::write(
            workspace.join("requirements/core/empty.yaml"),
            "category: Core History\nprefix: REQ-HIST\nrequirements: []\n",
        )
        .expect("requirement file");
        fs::write(
            workspace.join("features/empty.yaml"),
            "category: History\nversion: 1\nfeatures: []\n",
        )
        .expect("feature file");

        git(workspace, &["add", "."]);
        git_commit(workspace, "docs: add empty historical sections");

        let config = SyuConfig {
            spec: crate::config::SpecConfig {
                root: PathBuf::from("."),
            },
            ..SyuConfig::default()
        };

        let index = build_historical_id_index(workspace, &config).expect("index should build");

        assert!(index.available());
        assert!(!index.contains("REQ-HIST-MISSING-001"));
    }

    #[test]
    fn build_historical_id_index_surfaces_start_ref_snapshot_errors() {
        let tempdir = tempdir().expect("tempdir should exist");
        let workspace = tempdir.path();
        init_git_repository(workspace);
        fs::write(workspace.join("placeholder.txt"), "placeholder\n").expect("placeholder file");
        git(workspace, &["add", "."]);
        git_commit(workspace, "chore: initialize repository");

        let mut config = SyuConfig::default();
        config.spec.root = PathBuf::from(".");
        config.validate.historical_ids.start_ref = Some("definitely-not-a-ref".to_string());

        let error = build_historical_id_index(workspace, &config)
            .expect_err("invalid start ref should fail while recording the start snapshot");
        assert!(
            error
                .to_string()
                .contains("failed to enumerate historical files"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn build_historical_id_index_handles_empty_start_ref_ranges() {
        let tempdir = tempdir().expect("tempdir should exist");
        let workspace = tempdir.path();
        init_git_repository(workspace);
        fs::write(workspace.join("placeholder.txt"), "placeholder\n").expect("placeholder file");
        git(workspace, &["add", "."]);
        git_commit(workspace, "chore: initialize repository");
        let start_ref = git_stdout(workspace, &["rev-parse", "HEAD"]);

        let mut config = SyuConfig::default();
        config.spec.root = PathBuf::from(".");
        config.validate.historical_ids.start_ref = Some(start_ref);

        let index = build_historical_id_index(workspace, &config).expect("index should build");
        assert!(index.available());
    }

    #[test]
    fn historical_id_helpers_surface_git_and_parse_errors() {
        let tempdir = tempdir().expect("tempdir should exist");
        let workspace = tempdir.path();
        init_git_repository(workspace);

        fs::write(
            workspace.join("philosophy.yaml"),
            "category: Philosophy\nversion: 1\nlanguage: en\nphilosophies:\n  - id: PHIL-HIST-BROKEN-001\n    title: Broken documents should fail to parse.\n    product_design_principle: Historical parsing errors should remain visible.\n    coding_guideline: Keep YAML failures explicit.\n    linked_policies: []\n    - invalid\n",
        )
        .expect("malformed philosophy file");
        fs::create_dir_all(workspace.join("features")).expect("features dir");
        fs::write(
            workspace.join("features/broken.yaml"),
            "category: History\nversion: 1\nstories: []\n",
        )
        .expect("broken feature file");
        fs::write(
            workspace.join("features/invalid.yaml"),
            "category: History\nversion: 1\nfeatures: [\n",
        )
        .expect("invalid feature file");
        fs::write(workspace.join("features/not-a-mapping.yaml"), "- 1\n")
            .expect("non-mapping feature file");
        fs::write(
            workspace.join("features/structured.yaml"),
            "category: History\nversion: 1\nfeatures:\n  - id: FEAT-HIST-STRUCT-001\n    title: Structured feature docs should parse.\n    summary: Structured feature docs should hit the parse-error path.\n    status: implemented\n    linked_requirements: []\n    implementations: []\n",
        )
        .expect("structured feature file");

        git(workspace, &["add", "."]);
        git_commit(workspace, "docs: add malformed historical fixtures");
        let commit = git_stdout(workspace, &["rev-parse", "HEAD"]);
        let missing_repo = workspace.join("missing-repo");

        assert!(
            record_commit_snapshot(
                workspace,
                Path::new(""),
                &commit,
                &mut HistoricalIdIndex::default()
            )
            .is_err()
        );

        assert!(git_repository_root(&missing_repo).is_err());
        assert!(git_rev_list(workspace, "definitely-not-a-revision").is_err());
        assert!(git_rev_list(&missing_repo, "HEAD").is_err());
        assert!(
            git_tree_files(
                workspace,
                "definitely-not-a-commit",
                std::path::Path::new("")
            )
            .is_err()
        );
        assert!(git_tree_files(&missing_repo, "HEAD", std::path::Path::new("")).is_err());
        assert!(git_blob(&missing_repo, "HEAD", "philosophy.yaml").is_err());

        assert!(parse_blob::<PhilosophyDocument>(workspace, &commit, "philosophy.yaml").is_err());
        assert!(
            parse_blob::<PhilosophyDocument>(workspace, &commit, "missing.yaml")
                .expect("missing historical blob lookup should succeed")
                .is_none()
        );
        assert!(
            parse_feature_blob(workspace, &commit, "features/broken.yaml")
                .expect("feature blob lookup should succeed")
                .is_none()
        );
        assert!(
            parse_feature_blob(workspace, &commit, "features/missing.yaml")
                .expect("missing feature blob lookup should succeed")
                .is_none()
        );
        assert!(
            parse_feature_blob(workspace, &commit, "features/invalid.yaml")
                .expect("invalid feature blob lookup should succeed")
                .is_none()
        );
        assert!(
            parse_feature_blob(workspace, &commit, "features/not-a-mapping.yaml")
                .expect("non-mapping feature blob lookup should succeed")
                .is_none()
        );
        assert!(parse_feature_blob(workspace, &commit, "features/structured.yaml").is_err());
    }

    #[test]
    fn git_repository_root_rejects_empty_stdout() {
        let tempdir = tempdir().expect("tempdir should exist");
        let error = parse_git_repository_root_stdout(Vec::new(), tempdir.path())
            .expect_err("empty git root should fail");

        assert!(
            error
                .to_string()
                .contains("git rev-parse returned an empty repository root"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn build_historical_id_index_parses_unchanged_blobs_once() {
        let tempdir = tempdir().expect("tempdir should exist");
        let workspace = tempdir.path();
        init_git_repository(workspace);

        fs::create_dir_all(workspace.join("requirements/core")).expect("requirements dir");
        fs::write(
            workspace.join("requirements/core/unchanged.yaml"),
            "category: Core History\nprefix: REQ-HIST\nrequirements:\n  - id: REQ-HIST-CACHE-001\n    title: Unchanged blobs should be cached.\n    description: Historical indexing should not parse unchanged blobs repeatedly.\n    priority: medium\n    status: implemented\n    linked_policies: []\n    linked_features: []\n    tests: {}\n",
        )
        .expect("requirement file");
        git(workspace, &["add", "."]);
        git_commit(workspace, "docs: add cached historical requirement");

        for index in 0..20 {
            fs::write(workspace.join(format!("note-{index}.txt")), "unrelated\n")
                .expect("note file");
            git(workspace, &["add", "."]);
            git_commit(workspace, "chore: change unrelated file");
        }

        let config = SyuConfig {
            spec: crate::config::SpecConfig {
                root: PathBuf::from("."),
            },
            ..SyuConfig::default()
        };
        PARSED_HISTORICAL_BLOB_COUNT.with(|count| count.set(0));

        let index = build_historical_id_index(workspace, &config).expect("index should build");

        assert!(index.contains("REQ-HIST-CACHE-001"));
        PARSED_HISTORICAL_BLOB_COUNT.with(|count| assert_eq!(count.get(), 1));
    }

    #[test]
    fn build_historical_id_index_tracks_renamed_paths_without_reparsing_same_blob() {
        let tempdir = tempdir().expect("tempdir should exist");
        let workspace = tempdir.path();
        init_git_repository(workspace);

        fs::create_dir_all(workspace.join("requirements/core")).expect("requirements dir");
        fs::write(
            workspace.join("requirements/core/original name.yaml"),
            "category: Core History\nprefix: REQ-HIST\nrequirements:\n  - id: REQ-HIST-RENAME-001\n    title: Renamed blobs should keep history.\n    description: Historical indexing should follow raw rename records.\n    priority: medium\n    status: implemented\n    linked_policies: []\n    linked_features: []\n    tests: {}\n",
        )
        .expect("requirement file");
        git(workspace, &["add", "."]);
        git_commit(workspace, "docs: add renamed historical requirement");

        git(
            workspace,
            &[
                "mv",
                "requirements/core/original name.yaml",
                "requirements/core/café requirement.yaml",
            ],
        );
        git_commit(workspace, "docs: rename historical requirement");
        let rename_commit = git_stdout(workspace, &["rev-parse", "HEAD"]);

        fs::remove_file(workspace.join("requirements/core/café requirement.yaml"))
            .expect("remove renamed requirement");
        git(workspace, &["add", "-A"]);
        git_commit(workspace, "docs: delete renamed historical requirement");

        let config = SyuConfig {
            spec: crate::config::SpecConfig {
                root: PathBuf::from("."),
            },
            ..SyuConfig::default()
        };
        PARSED_HISTORICAL_BLOB_COUNT.with(|count| count.set(0));

        let index = build_historical_id_index(workspace, &config).expect("index should build");

        let deleted = index
            .deleted_entry("REQ-HIST-RENAME-001")
            .expect("renamed historical id should be recorded as deleted");
        assert_eq!(
            deleted.path,
            PathBuf::from("requirements/core/café requirement.yaml")
        );
        assert_eq!(deleted.commit, rename_commit);
        PARSED_HISTORICAL_BLOB_COUNT.with(|count| assert_eq!(count.get(), 1));
    }

    #[test]
    fn git_blob_distinguishes_missing_paths_from_unexpected_errors() {
        let tempdir = tempdir().expect("tempdir should exist");
        let workspace = tempdir.path();
        init_git_repository(workspace);

        fs::write(
            workspace.join("philosophy.yaml"),
            "category: Philosophy\nversion: 1\nlanguage: en\nphilosophies:\n  - id: PHIL-HIST-002\n    title: Blob lookup should work.\n    product_design_principle: Missing paths are normal; invalid commits are not.\n    coding_guideline: Keep Git failures visible.\n    linked_policies: []\n",
        )
        .expect("philosophy file");
        git(workspace, &["add", "."]);
        git_commit(workspace, "docs: add blob lookup fixture");

        let commit = {
            let output = Command::new("git")
                .arg("-C")
                .arg(workspace)
                .args(["rev-parse", "HEAD"])
                .output()
                .expect("git rev-parse should run");
            assert!(output.status.success(), "git rev-parse failed");
            String::from_utf8(output.stdout)
                .expect("commit should be utf8")
                .trim()
                .to_string()
        };

        assert_eq!(
            git_blob(workspace, &commit, "missing.yaml").expect("missing path lookup"),
            None
        );

        let error = git_blob(workspace, "definitely-not-a-commit", "philosophy.yaml")
            .expect_err("invalid commit should surface as an error");
        assert!(
            error
                .to_string()
                .contains("failed to read historical blob `philosophy.yaml`"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn git_blob_missing_matches_git_show_missing_path_errors() {
        let stderr = "fatal: path 'docs/syu/philosophy/foundation.yaml' does not exist in 'HEAD'";
        assert!(git_blob_missing(
            stderr,
            "HEAD",
            "docs/syu/philosophy/foundation.yaml"
        ));
        assert!(!git_blob_missing(
            "fatal: invalid object name 'HEAD'",
            "HEAD",
            "docs/syu/philosophy/foundation.yaml"
        ));
    }
}
