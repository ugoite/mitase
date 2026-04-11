use assert_cmd::cargo::CommandCargoExt;
use serde_json::Value;
use std::{path::PathBuf, process::Command};

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/workspaces")
        .join(name)
}

#[test]
// REQ-CORE-018
fn list_command_lists_philosophies_in_text_format() {
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("list")
        .arg("philosophy")
        .arg(fixture_path("passing"))
        .output()
        .expect("command should run");

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("PHIL-TRACE-001\tTrace everything"));
}

#[test]
// REQ-CORE-018
fn list_command_supports_json_output() {
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("list")
        .arg("feature")
        .arg(fixture_path("passing"))
        .arg("--format")
        .arg("json")
        .output()
        .expect("command should run");

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let json: Value = serde_json::from_slice(&output.stdout).expect("output should be valid JSON");
    assert_eq!(json["kind"], "feature");
    assert_eq!(
        json["items"]
            .as_array()
            .expect("items should be an array")
            .len(),
        3
    );
    assert_eq!(json["items"][0]["id"], "FEAT-TRACE-001");
}

#[test]
// REQ-CORE-018
fn list_command_accepts_plural_lookup_aliases() {
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("list")
        .arg("requirements")
        .arg(fixture_path("passing"))
        .output()
        .expect("command should run");

    assert!(output.status.success(), "plural alias should be accepted");
    assert!(String::from_utf8_lossy(&output.stdout).contains("REQ-TRACE-003"));
}

#[test]
// REQ-CORE-018
fn list_command_without_kind_lists_all_kinds() {
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("list")
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
    assert!(
        stdout.contains("=== philosophy ("),
        "should show philosophy section"
    );
    assert!(
        stdout.contains("=== policy ("),
        "should show policy section"
    );
    assert!(
        stdout.contains("=== requirement ("),
        "should show requirement section"
    );
    assert!(
        stdout.contains("=== feature ("),
        "should show feature section"
    );
    assert!(
        stdout.contains("PHIL-TRACE-001"),
        "should include philosophies"
    );
    assert!(stdout.contains("FEAT-TRACE-001"), "should include features");
}

#[test]
// REQ-CORE-018
fn list_command_dot_as_workspace_lists_all_kinds() {
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("list")
        .arg(fixture_path("passing"))
        .output()
        .expect("command should run");

    assert!(
        output.status.success(),
        "syu list <path> should list all kinds: stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("PHIL-TRACE-001"),
        "should list philosophies"
    );
    assert!(stdout.contains("FEAT-TRACE-001"), "should list features");
}

#[test]
// REQ-CORE-018
fn list_command_all_kinds_supports_json_output() {
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("list")
        .arg(fixture_path("passing"))
        .arg("--format")
        .arg("json")
        .output()
        .expect("command should run");

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let json: Value = serde_json::from_slice(&output.stdout).expect("output should be valid JSON");
    assert!(
        json["philosophy"].is_array(),
        "JSON should have philosophy array"
    );
    assert!(json["policy"].is_array(), "JSON should have policy array");
    assert!(
        json["requirement"].is_array(),
        "JSON should have requirement array"
    );
    assert!(json["feature"].is_array(), "JSON should have feature array");
    assert_eq!(json["philosophy"][0]["id"], "PHIL-TRACE-001");
}

#[test]
// REQ-CORE-018
fn list_command_rejects_kind_typos_before_treating_them_as_paths() {
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("list")
        .arg("philsophy")
        .output()
        .expect("command should run");

    assert!(!output.status.success(), "typoed kinds should fail");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("did you mean `philosophy`"),
        "stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("syu list --help"),
        "stderr should point users back to the list help:\n{stderr}",
    );
    assert!(
        !stderr.contains("failed to resolve workspace root"),
        "typoed kinds should not be reported as workspace path failures:\n{stderr}",
    );
}

#[test]
// REQ-CORE-018
fn list_command_keeps_kind_typos_helpful_when_workspace_is_explicit() {
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("list")
        .arg("philsophy")
        .arg(".")
        .output()
        .expect("command should run");

    assert!(!output.status.success(), "typoed kinds should fail");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("did you mean `philosophy`"),
        "stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("syu list ."),
        "stderr should keep the workspace-specific recovery hint:\n{stderr}",
    );
    assert!(
        !stderr.contains("failed to resolve workspace root"),
        "typoed kinds should not be reported as workspace path failures:\n{stderr}",
    );
}
