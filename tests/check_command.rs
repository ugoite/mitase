use assert_cmd::cargo::CommandCargoExt;
use std::{path::PathBuf, process::Command};

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/workspaces")
        .join(name)
}

#[test]
// REQ-CORE-001
fn check_command_accepts_passing_workspace() {
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("validate")
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
    assert!(stdout.contains("syu validate passed"));
    assert!(stdout.contains("traceability: requirements=3/3 features=3/3"));
}

#[test]
// REQ-CORE-001
fn check_command_reports_missing_definition_links() {
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("validate")
        .arg(fixture_path("failing"))
        .output()
        .expect("command should run");

    assert!(!output.status.success(), "failing fixture should fail");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("SYU-graph-reference-001"));
    assert!(stdout.contains("referenced rules:"));
    assert!(stdout.contains("Linked definitions must exist"));
    assert!(stdout.contains("REQ-MISSING-999"));
}

#[test]
// REQ-CORE-002
fn check_command_verifies_requirement_test_traceability_in_all_supported_languages() {
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("validate")
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

    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("output should be valid JSON");
    assert_eq!(json["trace_summary"]["requirement_traces"]["declared"], 3);
    assert_eq!(json["trace_summary"]["requirement_traces"]["validated"], 3);
}

#[test]
// REQ-CORE-002
fn check_command_verifies_feature_implementation_traceability_in_all_supported_languages() {
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("validate")
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

    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("output should be valid JSON");
    assert_eq!(json["trace_summary"]["feature_traces"]["declared"], 3);
    assert_eq!(json["trace_summary"]["feature_traces"]["validated"], 3);
}

#[test]
fn check_alias_still_invokes_validate() {
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("check")
        .arg(fixture_path("passing"))
        .output()
        .expect("command should run");

    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("syu validate passed"));
}
