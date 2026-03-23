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

fn configured_workspace(output: &str) -> (tempfile::TempDir, PathBuf) {
    let tempdir = tempdir().expect("tempdir should be created");
    let workspace = tempdir.path().join("workspace");
    copy_dir_recursive(&fixture_path("passing"), &workspace);
    fs::write(
        workspace.join("syu.yaml"),
        format!(
            "version: {version}\nspec:\n  root: docs/syu\nvalidate:\n  default_fix: false\n  allow_planned: true\n  require_non_orphaned_items: true\n  require_reciprocal_links: true\n  require_symbol_trace_coverage: false\nreport:\n  output: {output}\nruntimes:\n  python:\n    command: auto\n  node:\n    command: auto\n",
            version = env!("CARGO_PKG_VERSION"),
        ),
    )
    .expect("config");
    (tempdir, workspace)
}

#[test]
// REQ-CORE-004
fn report_command_generates_markdown_output() {
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("report")
        .arg(fixture_path("passing"))
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
    assert!(stdout.contains("Requirement-to-test traceability: 3/3"));
}

#[test]
// REQ-CORE-004
fn report_command_writes_markdown_file() {
    let tempdir = tempdir().expect("tempdir should be created");
    let output_file = tempdir.path().join("report.md");

    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("report")
        .arg(fixture_path("failing"))
        .arg("--output")
        .arg(&output_file)
        .output()
        .expect("command should run");

    assert!(!output.status.success(), "failing fixture should fail");
    let report = fs::read_to_string(&output_file).expect("report file should exist");
    assert!(report.contains("REQ-FAIL-001"));
    assert!(report.contains("REQ-MISSING-999"));
}

#[test]
fn report_command_uses_configured_output_path_when_flag_is_absent() {
    let (_tempdir, workspace) = configured_workspace("docs/generated/syu-report.md");

    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("report")
        .arg(&workspace)
        .output()
        .expect("command should run");

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        workspace.join("docs/generated/syu-report.md").is_file(),
        "configured report path should be written"
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("wrote report to"));
}

#[test]
fn report_command_cli_output_overrides_configured_output_path() {
    let (_tempdir, workspace) = configured_workspace("docs/generated/syu-report.md");
    let cli_output = workspace.join("reports/custom.md");

    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("report")
        .arg(&workspace)
        .arg("--output")
        .arg(&cli_output)
        .output()
        .expect("command should run");

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(cli_output.is_file(), "cli output path should be written");
    assert!(
        !workspace.join("docs/generated/syu-report.md").exists(),
        "configured path should not be used when CLI output is present"
    );
}

#[test]
fn report_command_rejects_configured_paths_that_escape_workspace() {
    let (tempdir, workspace) = configured_workspace("../outside-report.md");

    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("report")
        .arg(&workspace)
        .output()
        .expect("command should run");

    assert!(
        !output.status.success(),
        "unsafe configured path should fail"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("must stay within workspace root"),
        "stderr:\n{}\nstdout:\n{}",
        String::from_utf8_lossy(&output.stderr),
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(
        !tempdir.path().join("outside-report.md").exists(),
        "report must not be written outside the workspace"
    );
}
