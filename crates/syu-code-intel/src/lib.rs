use anyhow::{Context, Result, bail};
use serde::Serialize;
use std::{
    path::{Path, PathBuf},
    process::Command,
};

pub mod branch_scope;

pub use branch_scope::{
    AffectedSpecItem, AmbiguousOwnership, BranchScopeConfidence, BranchScopeEvidence,
    BranchScopeReport, ChangedFileReport, ChangedSymbolReport, OutOfScopeChange, RepoRiskSummary,
    SuggestedGoalSplit, TestInventoryReport, TraceOwnershipReport, UnownedChange,
};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OwnershipStatus {
    Owned,
    Partial,
    Unowned,
}

pub fn resolve_git_range_changed_files(workspace_root: &Path, range: &str) -> Result<Vec<PathBuf>> {
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

pub(crate) fn git_command(workspace_root: &Path) -> Command {
    let mut command = Command::new("git");
    command.arg("-C").arg(workspace_root);
    for key in GIT_ENVIRONMENT_KEYS {
        command.env_remove(key);
    }
    command
}

pub fn is_shared_utility_path(path: &Path) -> bool {
    let rendered = path.display().to_string().to_lowercase();
    rendered.contains("shared")
        || rendered.contains("common")
        || rendered.contains("util")
        || rendered.contains("helper")
        || rendered.contains("generated")
}

pub fn confidence_for_branch_scope(
    changed_files: &[String],
    unowned_files: &[String],
    ambiguous_files: &[String],
    spec_files: &[String],
    _has_planned_features: bool,
) -> BranchScopeConfidence {
    if !unowned_files.is_empty() {
        return BranchScopeConfidence::Low;
    }
    if !spec_files.is_empty()
        || changed_files.len() > 1
        || changed_files
            .iter()
            .any(|file| is_shared_utility_path(Path::new(file)))
    {
        return BranchScopeConfidence::Medium;
    }
    if !ambiguous_files.is_empty() {
        return BranchScopeConfidence::Medium;
    }
    BranchScopeConfidence::High
}

pub fn dedupe_strings(values: &mut Vec<String>) {
    values.sort();
    values.dedup();
}

pub fn flatten_symbols(file: &str, symbols: &[String]) -> Vec<ChangedSymbolReport> {
    symbols
        .iter()
        .map(|symbol| ChangedSymbolReport {
            file: file.to_string(),
            symbol: symbol.clone(),
            owners: Vec::new(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{BranchScopeConfidence, confidence_for_branch_scope, resolve_git_range_changed_files};
    use std::{fs, path::Path};
    use tempfile::tempdir;

    #[test]
    fn git_range_changed_files_collects_relative_paths() {
        let dir = tempdir().expect("tempdir");
        let root = dir.path();

        std::process::Command::new("git")
            .arg("init")
            .current_dir(root)
            .output()
            .expect("git init");
        std::process::Command::new("git")
            .args(["config", "user.email", "codex@example.com"])
            .current_dir(root)
            .output()
            .expect("git config email");
        std::process::Command::new("git")
            .args(["config", "user.name", "Codex"])
            .current_dir(root)
            .output()
            .expect("git config name");

        fs::write(root.join("a.txt"), "one").expect("write a");
        std::process::Command::new("git")
            .args(["add", "."])
            .current_dir(root)
            .output()
            .expect("git add");
        std::process::Command::new("git")
            .args(["commit", "-m", "base"])
            .current_dir(root)
            .output()
            .expect("git commit base");

        fs::write(root.join("a.txt"), "two").expect("write a2");
        fs::write(root.join("b.txt"), "new").expect("write b");
        std::process::Command::new("git")
            .args(["add", "."])
            .current_dir(root)
            .output()
            .expect("git add second");
        std::process::Command::new("git")
            .args(["commit", "-m", "update"])
            .current_dir(root)
            .output()
            .expect("git commit second");

        let files = resolve_git_range_changed_files(root, "HEAD~1..HEAD").expect("diff files");
        assert_eq!(
            files,
            vec![
                Path::new("a.txt").to_path_buf(),
                Path::new("b.txt").to_path_buf()
            ]
        );
    }

    #[test]
    fn ambiguous_ownership_caps_confidence_at_medium() {
        let confidence = confidence_for_branch_scope(
            &["src/lib.rs".to_string()],
            &[],
            &["src/lib.rs".to_string()],
            &[],
            true,
        );

        assert_eq!(confidence, BranchScopeConfidence::Medium);
    }
}
