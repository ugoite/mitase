// REQ-CORE-001
// REQ-CORE-004
// REQ-CORE-015
// REQ-CORE-018
// REQ-CORE-019
// REQ-CORE-025

use assert_cmd::cargo::CommandCargoExt;
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};
use tempfile::tempdir;

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

fn configured_workspace() -> (tempfile::TempDir, PathBuf) {
    let tempdir = tempdir().expect("tempdir should exist");
    let workspace = tempdir.path().join("workspace");
    copy_dir_recursive(&fixture_path("passing"), &workspace);
    fs::write(
        workspace.join("syu.yaml"),
        format!(
            "version: {version}\nspec:\n  root: docs/syu\nvalidate:\n  default_fix: false\n  allow_planned: true\n  require_non_orphaned_items: true\n  require_reciprocal_links: true\n  require_symbol_trace_coverage: false\napp:\n  bind: 127.0.0.1\n  port: 3000\nruntimes:\n  python:\n    command: auto\n  node:\n    command: auto\n",
            version = env!("CARGO_PKG_VERSION"),
        ),
    )
    .expect("config");
    (tempdir, workspace)
}

#[test]
fn validate_command_discovers_workspace_from_nested_current_directory() {
    let (_tempdir, workspace) = configured_workspace();
    let nested = workspace.join("frontend");
    let workspace_root = workspace
        .canonicalize()
        .expect("workspace should canonicalize");
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .current_dir(&nested)
        .arg("validate")
        .output()
        .expect("command should run");

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout)
            .contains(&format!("workspace: {}", workspace_root.display()))
    );
}

#[test]
fn browse_command_discovers_workspace_from_child_directory() {
    let (_tempdir, workspace) = configured_workspace();
    let nested = workspace.join("frontend");
    let workspace_root = workspace
        .canonicalize()
        .expect("workspace should canonicalize");
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("browse")
        .arg(&nested)
        .arg("--non-interactive")
        .output()
        .expect("command should run");

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("=== syu spec tree ==="));
    assert!(stdout.contains(&format!("workspace: {}", workspace_root.display())));
}

#[test]
fn list_command_discovers_workspace_from_child_directory() {
    let (_tempdir, workspace) = configured_workspace();
    let nested = workspace.join("frontend");
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("list")
        .arg("requirement")
        .arg(&nested)
        .output()
        .expect("command should run");

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("REQ-TRACE-001\tRust requirement trace")
    );
}

#[test]
fn show_command_discovers_workspace_from_child_directory() {
    let (_tempdir, workspace) = configured_workspace();
    let nested = workspace.join("frontend");
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("show")
        .arg("REQ-TRACE-001")
        .arg(&nested)
        .output()
        .expect("command should run");

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("ID: REQ-TRACE-001"));
}

#[test]
fn search_command_discovers_workspace_from_child_directory() {
    let (_tempdir, workspace) = configured_workspace();
    let nested = workspace.join("frontend");
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("search")
        .arg("FEAT-TRACE-002")
        .arg(&nested)
        .output()
        .expect("command should run");

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout)
            .contains("FEAT-TRACE-002\tfeature\tPython implementation trace")
    );
}

#[test]
fn audit_command_discovers_workspace_from_child_directory() {
    let (_tempdir, workspace) = configured_workspace();
    let nested = workspace.join("frontend");
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("audit")
        .arg(&nested)
        .output()
        .expect("command should run");

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("audit workspace:"));
}

#[test]
fn report_command_discovers_workspace_from_child_directory() {
    let (_tempdir, workspace) = configured_workspace();
    let nested = workspace.join("frontend");
    let workspace_root = workspace
        .canonicalize()
        .expect("workspace should canonicalize");
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("report")
        .arg(&nested)
        .output()
        .expect("command should run");

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("# syu validation report"));
    assert!(stdout.contains(&format!("- Workspace: `{}`", workspace_root.display())));
}
