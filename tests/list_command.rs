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
