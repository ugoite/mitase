use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::Command,
};

use tempfile::tempdir;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn write_mock_gh_script(dir: &Path) -> PathBuf {
    let gh_path = dir.join("gh");
    fs::write(
        &gh_path,
        r#"#!/usr/bin/env bash
set -euo pipefail
if [[ "$1" == "api" ]]; then
  cat "$MOCK_GH_QUEUE_JSON"
  exit 0
fi
if [[ "$1" == "pr" && "$2" == "merge" ]]; then
  printf '%s\n' "$*" >>"$MOCK_GH_MERGE_LOG"
  exit 0
fi
printf 'unexpected gh invocation: %s\n' "$*" >&2
exit 1
"#,
    )
    .expect("mock gh");
    let mut perms = fs::metadata(&gh_path).expect("metadata").permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&gh_path, perms).expect("chmod");
    gh_path
}

#[test]
fn merge_queue_reenroll_reports_when_no_clean_prs_are_dropped() {
    let tempdir = tempdir().expect("tempdir");
    let bin_dir = tempdir.path().join("bin");
    fs::create_dir_all(&bin_dir).expect("bin dir");
    write_mock_gh_script(&bin_dir);

    let queue_json = tempdir.path().join("queue.json");
    fs::write(
        &queue_json,
        r#"{"data":{"repository":{"pullRequests":{"nodes":[{"number":123,"title":"Already queued","state":"OPEN","baseRefName":"main","mergeStateStatus":"CLEAN","reviewDecision":"APPROVED","isInMergeQueue":true,"autoMergeRequest":{"enabledAt":"2026-01-01T00:00:00Z"},"mergeQueueEntry":{"state":"QUEUED"},"commits":{"nodes":[{"commit":{"statusCheckRollup":{"state":"SUCCESS"}}}]}}]}}}}"#,
    )
    .expect("queue json");
    let merge_log = tempdir.path().join("merge.log");
    let summary_path = tempdir.path().join("summary.md");

    let output = Command::new("bash")
        .arg(repo_root().join("scripts/ci/requeue-dropped-merge-queue-prs.sh"))
        .env(
            "PATH",
            format!("{}:{}", bin_dir.display(), std::env::var("PATH").unwrap()),
        )
        .env("MOCK_GH_QUEUE_JSON", &queue_json)
        .env("MOCK_GH_MERGE_LOG", &merge_log)
        .env("MERGE_QUEUE_REQUEUE_DRY_RUN", "true")
        .env("GITHUB_STEP_SUMMARY", &summary_path)
        .output()
        .expect("script should run");

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("no dropped clean PRs found"));
    assert!(
        !merge_log.exists(),
        "dry run without candidates should not invoke gh pr merge"
    );
    assert!(
        fs::read_to_string(&summary_path)
            .expect("summary")
            .contains("Merge queue re-enrollment")
    );
}

#[test]
fn merge_queue_reenroll_dry_run_reports_candidates_without_merging() {
    let tempdir = tempdir().expect("tempdir");
    let bin_dir = tempdir.path().join("bin");
    fs::create_dir_all(&bin_dir).expect("bin dir");
    write_mock_gh_script(&bin_dir);

    let queue_json = tempdir.path().join("queue.json");
    fs::write(
        &queue_json,
        r#"{"data":{"repository":{"pullRequests":{"nodes":[{"number":369,"title":"Dropped queue entry","state":"OPEN","baseRefName":"main","mergeStateStatus":"CLEAN","reviewDecision":"APPROVED","isInMergeQueue":false,"autoMergeRequest":null,"mergeQueueEntry":null,"commits":{"nodes":[{"commit":{"statusCheckRollup":{"state":"SUCCESS"}}}]}}]}}}}"#,
    )
    .expect("queue json");
    let merge_log = tempdir.path().join("merge.log");
    let summary_path = tempdir.path().join("summary.md");

    let output = Command::new("bash")
        .arg(repo_root().join("scripts/ci/requeue-dropped-merge-queue-prs.sh"))
        .env(
            "PATH",
            format!("{}:{}", bin_dir.display(), std::env::var("PATH").unwrap()),
        )
        .env("MOCK_GH_QUEUE_JSON", &queue_json)
        .env("MOCK_GH_MERGE_LOG", &merge_log)
        .env("MERGE_QUEUE_REQUEUE_DRY_RUN", "true")
        .env("GITHUB_STEP_SUMMARY", &summary_path)
        .output()
        .expect("script should run");

    assert!(output.status.success(), "dry run should still succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("would re-enable auto-merge"));
    assert!(stdout.contains("PR #369"));
    assert!(
        !merge_log.exists(),
        "dry run should not call gh pr merge for candidates"
    );
    let summary = fs::read_to_string(&summary_path).expect("summary");
    assert!(summary.contains("| #369 | Dropped queue entry | CLEAN | APPROVED | SUCCESS |"));
}

#[test]
fn merge_queue_reenroll_skips_prs_without_successful_status_rollup() {
    let tempdir = tempdir().expect("tempdir");
    let bin_dir = tempdir.path().join("bin");
    fs::create_dir_all(&bin_dir).expect("bin dir");
    write_mock_gh_script(&bin_dir);

    let queue_json = tempdir.path().join("queue.json");
    fs::write(
        &queue_json,
        r#"{"data":{"repository":{"pullRequests":{"nodes":[{"number":369,"title":"Missing rollup","state":"OPEN","baseRefName":"main","mergeStateStatus":"CLEAN","reviewDecision":"APPROVED","isInMergeQueue":false,"autoMergeRequest":null,"mergeQueueEntry":null,"commits":{"nodes":[{"commit":{"statusCheckRollup":null}}]}}]}}}}"#,
    )
    .expect("queue json");
    let merge_log = tempdir.path().join("merge.log");

    let output = Command::new("bash")
        .arg(repo_root().join("scripts/ci/requeue-dropped-merge-queue-prs.sh"))
        .env(
            "PATH",
            format!("{}:{}", bin_dir.display(), std::env::var("PATH").unwrap()),
        )
        .env("MOCK_GH_QUEUE_JSON", &queue_json)
        .env("MOCK_GH_MERGE_LOG", &merge_log)
        .env("MERGE_QUEUE_REQUEUE_DRY_RUN", "true")
        .output()
        .expect("script should run");

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("no dropped clean PRs found"),
        "missing rollup should not count as a requeue candidate"
    );
    assert!(
        !merge_log.exists(),
        "PR without a successful status rollup should not be merged"
    );
}

#[test]
fn merge_queue_reenroll_requeues_dropped_clean_prs() {
    let tempdir = tempdir().expect("tempdir");
    let bin_dir = tempdir.path().join("bin");
    fs::create_dir_all(&bin_dir).expect("bin dir");
    write_mock_gh_script(&bin_dir);

    let queue_json = tempdir.path().join("queue.json");
    fs::write(
        &queue_json,
        r#"{"data":{"repository":{"pullRequests":{"nodes":[{"number":369,"title":"Dropped queue entry","state":"OPEN","baseRefName":"main","mergeStateStatus":"CLEAN","reviewDecision":"APPROVED","isInMergeQueue":false,"autoMergeRequest":null,"mergeQueueEntry":null,"commits":{"nodes":[{"commit":{"statusCheckRollup":{"state":"SUCCESS"}}}]}}]}}}}"#,
    )
    .expect("queue json");
    let merge_log = tempdir.path().join("merge.log");

    let output = Command::new("bash")
        .arg(repo_root().join("scripts/ci/requeue-dropped-merge-queue-prs.sh"))
        .env(
            "PATH",
            format!("{}:{}", bin_dir.display(), std::env::var("PATH").unwrap()),
        )
        .env("MOCK_GH_QUEUE_JSON", &queue_json)
        .env("MOCK_GH_MERGE_LOG", &merge_log)
        .output()
        .expect("script should run");

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let merge_invocations = fs::read_to_string(&merge_log).expect("merge log");
    assert!(merge_invocations.contains("pr merge 369 --repo ugoite/syu --auto --squash"));
    assert!(String::from_utf8_lossy(&output.stdout).contains("re-enabled auto-merge"));
}
