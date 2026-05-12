// FEAT-LOG-001
// REQ-CORE-636

use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
    process::Command,
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

    let commits = if let Some(start_ref) = index.start_ref.clone() {
        let mut commits = Vec::new();
        record_commit_snapshot(
            &repository_root,
            &spec_root_relative,
            &start_ref,
            &mut index,
        )?;
        commits.extend(git_rev_list(
            &repository_root,
            &format!("{start_ref}..HEAD"),
        )?);
        commits
    } else {
        git_rev_list(&repository_root, "HEAD")?
    };

    for commit in commits {
        record_commit_snapshot(&repository_root, &spec_root_relative, &commit, &mut index)?;
    }

    index.available = true;
    Ok(index)
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

    let root =
        String::from_utf8(output.stdout).context("git repository root should be valid UTF-8")?;
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

fn record_commit_snapshot(
    repository_root: &Path,
    spec_root_relative: &Path,
    commit: &str,
    index: &mut HistoricalIdIndex,
) -> Result<()> {
    let files = git_tree_files(repository_root, commit, spec_root_relative)?;
    for file in files {
        if !is_yaml_file(&file) {
            continue;
        }

        if is_under_section(&file, spec_root_relative, "philosophy") {
            if let Some(document) =
                parse_blob::<PhilosophyDocument>(repository_root, commit, &file)?
            {
                for item in document.philosophies {
                    record_id(index, SectionKind::Philosophy, item.id);
                }
            }
        } else if is_under_section(&file, spec_root_relative, "policies") {
            if let Some(document) = parse_blob::<PolicyDocument>(repository_root, commit, &file)? {
                for item in document.policies {
                    record_id(index, SectionKind::Policies, item.id);
                }
            }
        } else if is_under_section(&file, spec_root_relative, "requirements") {
            if let Some(document) =
                parse_blob::<RequirementDocument>(repository_root, commit, &file)?
            {
                for item in document.requirements {
                    record_id(index, SectionKind::Requirements, item.id);
                }
            }
        } else if is_under_section(&file, spec_root_relative, "features") {
            if file.ends_with("features.yaml") {
                continue;
            }

            if let Some(document) = parse_feature_blob(repository_root, commit, &file)? {
                for item in document.features {
                    record_id(index, SectionKind::Features, item.id);
                }
            }
        }
    }

    Ok(())
}

fn parse_feature_blob(
    repository_root: &Path,
    commit: &str,
    path: &str,
) -> Result<Option<FeatureDocument>> {
    let Some(raw) = git_blob(repository_root, commit, path)? else {
        return Ok(None);
    };
    let value: serde_yaml::Value = match serde_yaml::from_str(&raw) {
        Ok(value) => value,
        Err(_) => return Ok(None),
    };
    let Some(mapping) = value.as_mapping() else {
        return Ok(None);
    };
    if !mapping.contains_key(serde_yaml::Value::String("features".to_string())) {
        return Ok(None);
    }
    Ok(Some(serde_yaml::from_str(&raw).with_context(|| {
        format!("failed to parse feature document `{path}` at `{commit}`")
    })?))
}

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

fn git_blob_missing(stderr: &str, commit: &str, path: &str) -> bool {
    stderr.contains(&format!("path '{path}' does not exist in '{commit}'"))
}

fn git_tree_files(
    repository_root: &Path,
    commit: &str,
    spec_root_relative: &Path,
) -> Result<Vec<String>> {
    let output = git_command(repository_root)
        .arg("ls-tree")
        .arg("-r")
        .arg("--name-only")
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

    let stdout =
        String::from_utf8(output.stdout).context("git ls-tree output should be valid UTF-8")?;
    Ok(stdout
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect())
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

fn is_under_section(path: &str, spec_root_relative: &Path, section: &str) -> bool {
    let prefix = if spec_root_relative.as_os_str().is_empty() {
        section.to_string()
    } else {
        format!(
            "{}/{}",
            spec_root_relative.to_string_lossy().replace('\\', "/"),
            section
        )
    };
    path == prefix || path.starts_with(&format!("{prefix}/"))
}

fn record_id(index: &mut HistoricalIdIndex, kind: SectionKind, id: String) {
    index.ids_by_value.insert(id.clone());
    index.ids_by_section.entry(kind).or_default().insert(id);
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
        assert!(
            output.status.success(),
            "git {:?} failed\nstdout:\n{}\nstderr:\n{}",
            args,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
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
        assert!(
            output.status.success(),
            "git commit failed\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn init_git_repository(workspace: &Path) {
        git(workspace, &["init"]);
        git(workspace, &["config", "user.name", "Test User"]);
        git(workspace, &["config", "user.email", "test@example.com"]);
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
