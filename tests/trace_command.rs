// FEAT-TRACE-001
// REQ-CORE-021

use assert_cmd::cargo::CommandCargoExt;
use serde_json::Value;
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
};
use tempfile::tempdir;

static COMMIT_TIMESTAMP: AtomicU64 = AtomicU64::new(1_776_355_200);

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/workspaces")
        .join(name)
}

fn copy_dir_recursive(source: &Path, destination: &Path) {
    fs::create_dir_all(destination).expect("destination dir");
    for entry in fs::read_dir(source).expect("source dir") {
        let entry = entry.expect("dir entry");
        let entry_type = entry.file_type().expect("entry type");
        let destination_path = destination.join(entry.file_name());
        if entry_type.is_dir() {
            copy_dir_recursive(&entry.path(), &destination_path);
        } else {
            fs::copy(entry.path(), destination_path).expect("file copy");
        }
    }
}

fn git(workspace: &Path, args: &[&str]) {
    let mut command = Command::new("git");
    command.arg("-C").arg(workspace).args(args);
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
    let output = command.output().expect("git commit should run");
    assert!(
        output.status.success(),
        "git commit failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn init_git_fixture_workspace() -> tempfile::TempDir {
    let tempdir = tempdir().expect("tempdir should exist");
    let workspace = tempdir.path().join("workspace");
    copy_dir_recursive(&fixture_path("passing"), &workspace);

    git(&workspace, &["init"]);
    git(&workspace, &["config", "user.name", "Test User"]);
    git(&workspace, &["config", "user.email", "test@example.com"]);
    git(&workspace, &["add", "."]);
    git_commit(&workspace, "chore: seed traced workspace");

    fs::write(
        workspace.join("src/rust_feature.rs"),
        "// FEAT-TRACE-001\npub fn feature_trace_rust() {\n    println!(\"changed\");\n}\n",
    )
    .expect("changed traced file");
    fs::write(workspace.join("src/unowned.rs"), "pub fn unowned() {}\n").expect("unowned source");
    git(&workspace, &["add", "."]);
    git_commit(&workspace, "feat: update traced and unowned files");

    tempdir
}

#[test]
fn trace_command_resolves_feature_owners_from_file_only_lookup() {
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .args([
            "trace",
            "src/rust_feature.rs",
            fixture_path("passing")
                .to_str()
                .expect("fixture path should be valid utf-8"),
        ])
        .output()
        .expect("trace command should run");

    assert!(output.status.success(), "trace lookup should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("File: src/rust_feature.rs"));
    assert!(stdout.contains("Status: owned"));
    assert!(stdout.contains("feature FEAT-TRACE-001"));
    assert!(stdout.contains("REQ-TRACE-001"));
    assert!(stdout.contains("POL-TRACE-001"));
    assert!(stdout.contains("PHIL-TRACE-001"));
}

#[test]
fn trace_command_supports_symbol_lookups_in_json() {
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .args([
            "trace",
            "src/rust_trace_tests.rs",
            fixture_path("passing")
                .to_str()
                .expect("fixture path should be valid utf-8"),
            "--symbol",
            "req_trace_rust_test",
            "--format",
            "json",
        ])
        .output()
        .expect("trace command should run");

    assert!(output.status.success(), "trace lookup should succeed");
    let json: Value =
        serde_json::from_slice(&output.stdout).expect("trace command should print valid JSON");
    assert_eq!(json["status"], "owned");
    assert_eq!(json["matched_owners"][0]["kind"], "requirement");
    assert_eq!(json["matched_owners"][0]["id"], "REQ-TRACE-001");
    assert_eq!(
        json["matched_owners"][0]["matched_symbol"],
        "req_trace_rust_test"
    );
    assert_eq!(json["requirements"][0]["id"], "REQ-TRACE-001");
    assert_eq!(json["features"][0]["id"], "FEAT-TRACE-001");
}

#[test]
fn trace_command_reports_partially_traced_symbols() {
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .args([
            "trace",
            "src/rust_feature.rs",
            fixture_path("passing")
                .to_str()
                .expect("fixture path should be valid utf-8"),
            "--symbol",
            "missing_symbol",
        ])
        .output()
        .expect("trace command should run");

    assert!(
        output.status.success(),
        "partial trace lookups should still succeed"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Status: partial"));
    assert!(stdout.contains("No trace owners matched symbol `missing_symbol`."));
    assert!(stdout.contains("File owners without a matching symbol:"));
    assert!(stdout.contains("feature FEAT-TRACE-001"));
}

#[test]
fn trace_command_reports_unowned_files_with_next_steps() {
    let tempdir = tempdir().expect("tempdir should exist");
    let workspace = tempdir.path().join("workspace");
    copy_dir_recursive(&fixture_path("passing"), &workspace);
    fs::write(workspace.join("src/unowned.rs"), "pub fn unowned() {}\n")
        .expect("unowned source should exist");

    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .args([
            "trace",
            "src/unowned.rs",
            workspace
                .to_str()
                .expect("workspace path should be valid utf-8"),
        ])
        .output()
        .expect("trace command should run");

    assert!(
        output.status.success(),
        "unowned trace lookups should still succeed"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Status: unowned"));
    assert!(stdout.contains("No requirement or feature traces reference `src/unowned.rs`."));
    assert!(stdout.contains("syu validate . --genre trace"));
}

#[test]
fn trace_command_reports_git_range_summary_for_changed_files() {
    let workspace = init_git_fixture_workspace();
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .current_dir(workspace.path().join("workspace"))
        .args(["trace", "--range", "HEAD~1..HEAD"])
        .output()
        .expect("trace range command should run");

    assert!(output.status.success(), "trace range should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Git range: HEAD~1..HEAD"));
    assert!(stdout.contains("Changed files: 2"));
    assert!(stdout.contains("Coverage: 1 owned, 0 partial, 1 unowned"));
    assert!(stdout.contains("feature FEAT-TRACE-001:"));
    assert!(stdout.contains("UNOWNED:"));
    assert!(stdout.contains("src/rust_feature.rs"));
    assert!(stdout.contains("src/unowned.rs"));
}

#[test]
fn trace_command_reports_empty_git_ranges_as_json() {
    let workspace = init_git_fixture_workspace();
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .current_dir(workspace.path().join("workspace"))
        .args(["trace", "--range", "HEAD..HEAD", "--format", "json"])
        .output()
        .expect("trace range command should run");

    assert!(output.status.success(), "empty trace range should succeed");
    let json: Value = serde_json::from_slice(&output.stdout).expect("json output");
    assert_eq!(json["range"], "HEAD..HEAD");
    assert_eq!(json["summary"]["total_files"], 0);
    assert!(json["files"].as_array().expect("files array").is_empty());
}

#[test]
fn trace_command_reports_empty_git_ranges_as_text() {
    let workspace = init_git_fixture_workspace();
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .current_dir(workspace.path().join("workspace"))
        .args(["trace", "--range", "HEAD..HEAD"])
        .output()
        .expect("trace range command should run");

    assert!(output.status.success(), "empty trace range should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Git range: HEAD..HEAD"));
    assert!(stdout.contains("No files changed in range"));
}

#[test]
fn trace_command_supports_git_range_json_output() {
    let workspace = init_git_fixture_workspace();
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .current_dir(workspace.path().join("workspace"))
        .args(["trace", "--range", "HEAD~1..HEAD", "--format", "json"])
        .output()
        .expect("trace range command should run");

    assert!(output.status.success(), "trace range should succeed");
    let json: Value = serde_json::from_slice(&output.stdout).expect("json output");
    assert_eq!(json["summary"]["total_files"], 2);
    assert_eq!(json["summary"]["owned_files"], 1);
    assert_eq!(json["summary"]["unowned_files"], 1);
}

#[test]
fn trace_command_rejects_invalid_git_ranges() {
    let workspace = init_git_fixture_workspace();
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .current_dir(workspace.path().join("workspace"))
        .args(["trace", "--range", "definitely-not-a-range"])
        .output()
        .expect("trace range command should run");

    assert_eq!(output.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("git range `definitely-not-a-range` is not valid")
    );
}

#[test]
fn trace_command_skips_invalid_git_diff_paths() {
    let tempdir = tempdir().expect("tempdir should exist");
    let script_dir = tempdir.path().join("bin");
    fs::create_dir_all(&script_dir).expect("script dir");
    let script_path = script_dir.join("git");
    fs::write(
        &script_path,
        "#!/bin/sh\nset -eu\nprintf '../outside.rs\\nsrc/rust_feature.rs\\n'\n",
    )
    .expect("fake git");

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let mut permissions = fs::metadata(&script_path).expect("metadata").permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&script_path, permissions).expect("permissions");
    }

    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .current_dir(fixture_path("passing"))
        .env(
            "PATH",
            format!(
                "{}:{}",
                script_dir.display(),
                std::env::var("PATH").expect("PATH env")
            ),
        )
        .args(["trace", "--range", "HEAD~1..HEAD", "--format", "json"])
        .output()
        .expect("trace range command should run");

    assert!(
        output.status.success(),
        "range should ignore invalid diff paths"
    );
    let json: Value = serde_json::from_slice(&output.stdout).expect("json output");
    assert_eq!(json["summary"]["total_files"], 1);
    assert_eq!(json["files"][0]["file"], "src/rust_feature.rs");
}

#[test]
fn trace_command_reports_missing_git_binary_for_range_resolution() {
    let tempdir = tempdir().expect("tempdir should exist");
    let empty_path = tempdir.path().join("empty-bin");
    fs::create_dir_all(&empty_path).expect("empty bin");

    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .current_dir(fixture_path("passing"))
        .env("PATH", empty_path)
        .args(["trace", "--range", "HEAD~1..HEAD"])
        .output()
        .expect("trace range command should run");

    assert_eq!(output.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("failed to run `git diff --name-only` for range `HEAD~1..HEAD`")
    );
}

#[test]
fn trace_command_rejects_symbol_with_git_range() {
    let workspace = init_git_fixture_workspace();
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .current_dir(workspace.path().join("workspace"))
        .args([
            "trace",
            "--range",
            "HEAD~1..HEAD",
            "--symbol",
            "feature_trace_rust",
        ])
        .output()
        .expect("trace range command should run");

    assert_eq!(output.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("--symbol cannot be used with --range")
    );
}
