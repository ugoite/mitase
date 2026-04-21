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
if [[ "$1" == "run" && "$2" == "list" ]]; then
  if [[ -n "${EXPECT_GH_RUN_REPO:-}" ]]; then
    shift 2
    if [[ "$1" != "--repo" || "$2" != "$EXPECT_GH_RUN_REPO" ]]; then
      printf 'expected gh run list --repo %s, got: %s\n' "$EXPECT_GH_RUN_REPO" "$*" >&2
      exit 1
    fi
  fi
  cat "$MOCK_GH_RUNS_JSON"
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
fn merge_queue_watchdog_reports_healthy_queue() {
    let tempdir = tempdir().expect("tempdir");
    let bin_dir = tempdir.path().join("bin");
    fs::create_dir_all(&bin_dir).expect("bin dir");
    write_mock_gh_script(&bin_dir);

    let queue_json = tempdir.path().join("queue.json");
    fs::write(
        &queue_json,
        r#"{"data":{"repository":{"pullRequests":{"nodes":[{"number":123,"title":"Healthy queue entry","mergeStateStatus":"CLEAN","isInMergeQueue":true,"autoMergeRequest":{"enabledAt":"2026-01-01T00:00:00Z"},"mergeQueueEntry":{"state":"QUEUED"}}]}}}}"#,
    )
    .expect("queue json");
    let runs_json = tempdir.path().join("runs.json");
    fs::write(
        &runs_json,
        r#"[
  {"databaseId": 1, "workflowName": "ci", "headBranch": "gh-readonly-queue/main/pr-123-abc", "status": "completed", "conclusion": "success"},
  {"databaseId": 2, "workflowName": "codeql", "headBranch": "gh-readonly-queue/main/pr-123-abc", "status": "completed", "conclusion": "success"}
]"#,
    )
    .expect("runs json");
    let summary_path = tempdir.path().join("summary.md");

    let output = Command::new("bash")
        .arg(repo_root().join("scripts/ci/check-merge-queue-health.sh"))
        .env(
            "PATH",
            format!("{}:{}", bin_dir.display(), std::env::var("PATH").unwrap()),
        )
        .env("MOCK_GH_QUEUE_JSON", &queue_json)
        .env("MOCK_GH_RUNS_JSON", &runs_json)
        .env("GITHUB_STEP_SUMMARY", &summary_path)
        .output()
        .expect("script should run");

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("no stuck AWAITING_CHECKS entries"));
    assert!(
        fs::read_to_string(&summary_path)
            .expect("summary")
            .contains("Merge queue watchdog")
    );
}

#[test]
// REQ-CORE-014
fn merge_queue_watchdog_scopes_merge_group_runs_to_requested_repo() {
    let tempdir = tempdir().expect("tempdir");
    let bin_dir = tempdir.path().join("bin");
    fs::create_dir_all(&bin_dir).expect("bin dir");
    write_mock_gh_script(&bin_dir);

    let queue_json = tempdir.path().join("queue.json");
    fs::write(
        &queue_json,
        r#"{"data":{"repository":{"pullRequests":{"nodes":[{"number":123,"title":"Healthy queue entry","mergeStateStatus":"CLEAN","isInMergeQueue":true,"autoMergeRequest":{"enabledAt":"2026-01-01T00:00:00Z"},"mergeQueueEntry":{"state":"QUEUED"}}]}}}}"#,
    )
    .expect("queue json");
    let runs_json = tempdir.path().join("runs.json");
    fs::write(
        &runs_json,
        r#"[
  {"databaseId": 1, "workflowName": "ci", "headBranch": "gh-readonly-queue/main/pr-123-abc", "status": "completed", "conclusion": "success"},
  {"databaseId": 2, "workflowName": "codeql", "headBranch": "gh-readonly-queue/main/pr-123-abc", "status": "completed", "conclusion": "success"}
]"#,
    )
    .expect("runs json");

    let output = Command::new("bash")
        .arg(repo_root().join("scripts/ci/check-merge-queue-health.sh"))
        .arg("octo/example")
        .env(
            "PATH",
            format!("{}:{}", bin_dir.display(), std::env::var("PATH").unwrap()),
        )
        .env("MOCK_GH_QUEUE_JSON", &queue_json)
        .env("MOCK_GH_RUNS_JSON", &runs_json)
        .env("EXPECT_GH_RUN_REPO", "octo/example")
        .output()
        .expect("script should run");

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn merge_queue_watchdog_fails_for_stuck_awaiting_checks_entries() {
    let tempdir = tempdir().expect("tempdir");
    let bin_dir = tempdir.path().join("bin");
    fs::create_dir_all(&bin_dir).expect("bin dir");
    write_mock_gh_script(&bin_dir);

    let queue_json = tempdir.path().join("queue.json");
    fs::write(
        &queue_json,
        r#"{"data":{"repository":{"pullRequests":{"nodes":[{"number":328,"title":"Queue stuck","mergeStateStatus":"CLEAN","isInMergeQueue":true,"autoMergeRequest":{"enabledAt":"2026-01-01T00:00:00Z"},"mergeQueueEntry":{"state":"AWAITING_CHECKS"}}]}}}}"#,
    )
    .expect("queue json");
    let runs_json = tempdir.path().join("runs.json");
    fs::write(
        &runs_json,
        r#"[
  {"databaseId": 24468330653, "workflowName": "ci", "headBranch": "gh-readonly-queue/main/pr-328-abc", "status": "completed", "conclusion": "success"},
  {"databaseId": 24468330633, "workflowName": "codeql", "headBranch": "gh-readonly-queue/main/pr-328-abc", "status": "completed", "conclusion": "success"}
]"#,
    )
    .expect("runs json");
    let summary_path = tempdir.path().join("summary.md");

    let output = Command::new("bash")
        .arg(repo_root().join("scripts/ci/check-merge-queue-health.sh"))
        .env(
            "PATH",
            format!("{}:{}", bin_dir.display(), std::env::var("PATH").unwrap()),
        )
        .env("MOCK_GH_QUEUE_JSON", &queue_json)
        .env("MOCK_GH_RUNS_JSON", &runs_json)
        .env("GITHUB_STEP_SUMMARY", &summary_path)
        .output()
        .expect("script should run");

    assert!(
        !output.status.success(),
        "watchdog should fail for stuck queue entries"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("PR #328"));
    assert!(stdout.contains("ci#24468330653"));
    assert!(stdout.contains("codeql#24468330633"));
    assert!(stdout.contains("gh pr merge <number> --auto --squash"));
    let summary = fs::read_to_string(&summary_path).expect("summary");
    assert!(summary.contains("| #328 | Queue stuck | CLEAN | enabled |"));
}
