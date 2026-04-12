use assert_cmd::cargo::CommandCargoExt;
use serde_json::Value;
use std::{path::PathBuf, process::Command};

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/workspaces")
        .join(name)
}

#[test]
// REQ-CORE-019
fn search_command_matches_ids_and_titles() {
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("search")
        .arg("FEAT-TRACE-002")
        .arg(fixture_path("passing"))
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("FEAT-TRACE-002\tfeature\tPython implementation trace"));
}

#[test]
// REQ-CORE-019
fn search_command_matches_philosophy_ids_and_titles() {
    let id_output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("search")
        .arg("PHIL-TRACE-001")
        .arg(fixture_path("passing"))
        .output()
        .expect("philosophy id search should run");

    assert!(id_output.status.success());
    assert!(
        String::from_utf8_lossy(&id_output.stdout)
            .contains("PHIL-TRACE-001\tphilosophy\tTrace everything")
    );

    let title_output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("search")
        .arg("everything")
        .arg(fixture_path("passing"))
        .output()
        .expect("philosophy title search should run");

    assert!(title_output.status.success());
    assert!(
        String::from_utf8_lossy(&title_output.stdout)
            .contains("PHIL-TRACE-001\tphilosophy\tTrace everything")
    );
}

#[test]
// REQ-CORE-019
fn search_command_matches_summaries_and_descriptions() {
    let summary_output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("search")
        .arg("source")
        .arg(fixture_path("passing"))
        .output()
        .expect("summary search should run");

    assert!(summary_output.status.success());
    let summary_stdout = String::from_utf8_lossy(&summary_output.stdout);
    assert!(summary_stdout.contains("FEAT-TRACE-001\tfeature\tRust implementation trace"));

    let description_output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("search")
        .arg("repository")
        .arg(fixture_path("passing"))
        .output()
        .expect("description search should run");

    assert!(description_output.status.success());
    let description_stdout = String::from_utf8_lossy(&description_output.stdout);
    assert!(
        description_stdout.contains("POL-TRACE-001\tpolicy\tRequirements need executable evidence")
    );
}

#[test]
// REQ-CORE-019
fn search_command_scopes_results_by_kind() {
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("search")
        .arg("python")
        .arg(fixture_path("passing"))
        .arg("--kind")
        .arg("requirement")
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("REQ-TRACE-002\trequirement\tPython requirement trace"));
    assert!(
        !stdout.contains("FEAT-TRACE-002"),
        "kind filter should exclude features"
    );
}

#[test]
// REQ-CORE-019
fn search_command_supports_json_output() {
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("search")
        .arg("trace")
        .arg(fixture_path("passing"))
        .arg("--kind")
        .arg("feature")
        .arg("--format")
        .arg("json")
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("output should be valid JSON");
    assert_eq!(json["query"], "trace");
    assert_eq!(json["kind"], "feature");
    assert_eq!(json["items"][0]["kind"], "feature");
    assert_eq!(json["items"][0]["id"], "FEAT-TRACE-001");
}

#[test]
// REQ-CORE-019
fn search_command_matches_browser_search_fields_only() {
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("search")
        .arg("machine-readable")
        .arg(fixture_path("passing"))
        .output()
        .expect("command should run");

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "no matches for `machine-readable`"
    );
}

#[test]
// REQ-CORE-019
fn search_command_reads_items_from_workspaces_with_validation_errors() {
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("search")
        .arg("REQ-FAIL")
        .arg(fixture_path("failing"))
        .output()
        .expect("command should run");

    assert!(
        output.status.success(),
        "validation issues should not block search"
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("REQ-FAIL-001"));
}

#[test]
// REQ-CORE-019
fn search_command_rejects_whitespace_only_queries() {
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("search")
        .arg("   ")
        .arg(fixture_path("passing"))
        .output()
        .expect("command should run");

    assert_eq!(output.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("search query must not be empty or whitespace")
    );
}
