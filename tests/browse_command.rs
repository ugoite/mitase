// REQ-CORE-015

use assert_cmd::{Command, cargo::CommandCargoExt};
use serde_json::Value;
use std::{fs, path::PathBuf, process::Command as StdCommand};
use tempfile::tempdir;

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/workspaces")
        .join(name)
}

#[test]
fn browse_command_shows_linked_details_without_failing_validation_errors() {
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("browse")
        .arg(fixture_path("failing"))
        .write_stdin("5\n1\n\n0\n0\n")
        .output()
        .expect("browse command should run");

    assert!(output.status.success(), "browse UI should not fail");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("errors ("));
    assert!(stdout.contains("SYU-graph-reference-001"));
    assert!(stdout.contains("Linked definitions must exist"));
}

#[test]
fn bare_syu_prints_help_when_not_attached_to_a_terminal() {
    let output = StdCommand::cargo_bin("syu")
        .expect("binary should build")
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Usage:"));
    assert!(stdout.contains("browse"));
    assert!(stdout.contains("list"));
    assert!(stdout.contains("show"));
    assert!(stdout.contains("validate"));
}

#[test]
fn browse_command_help_explains_when_to_use_non_interactive_mode() {
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("browse")
        .arg("--help")
        .output()
        .expect("command should run");

    assert!(output.status.success(), "help should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("syu browse . --non-interactive"),
        "help should show the non-interactive example:\n{stdout}",
    );
    assert!(
        stdout.contains(
            "workspace metadata, per-layer counts, grouped items, and the current validation errors"
        ),
        "help should explain the browse snapshot shape:\n{stdout}",
    );
    assert!(
        stdout.contains("syu browse . --non-interactive --format json"),
        "help should show the JSON browse example:\n{stdout}",
    );
}

#[test]
fn browse_command_can_follow_linked_entries() {
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("browse")
        .arg(fixture_path("passing"))
        .write_stdin("1\n1\n1\n0\n0\n0\n")
        .output()
        .expect("browse command should run");

    assert!(output.status.success(), "browse UI should not fail");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("=== philosophy detail ==="));
    assert!(stdout.contains("=== policy detail ==="));
}

#[test]
fn browse_command_walks_all_sections_in_a_passing_workspace() {
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("browse")
        .arg(fixture_path("passing"))
        .write_stdin("1\n1\n0\n0\n2\n1\n0\n0\n3\n1\n0\n0\n4\n1\n0\n0\n5\n\n0\n")
        .output()
        .expect("browse command should run");

    assert!(output.status.success(), "browse UI should not fail");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("=== philosophy detail ==="));
    assert!(stdout.contains("=== policy detail ==="));
    assert!(stdout.contains("=== feature detail ==="));
    assert!(stdout.contains("=== requirement detail ==="));
    assert!(stdout.contains("No validation issues are currently reported."));
}

#[test]
fn browse_command_menu_uses_documented_layer_order() {
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("browse")
        .arg(fixture_path("passing"))
        .write_stdin("0\n")
        .output()
        .expect("browse command should run");

    assert!(output.status.success(), "browse UI should not fail");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let philosophy_index = stdout.find("1. philosophy").expect("philosophy item");
    let policy_index = stdout.find("2. policy").expect("policy item");
    let requirement_index = stdout.find("3. requirement").expect("requirement item");
    let feature_index = stdout.find("4. feature").expect("feature item");
    let summary = stdout
        .lines()
        .find(|line| line.starts_with("philosophy="))
        .expect("summary line");

    assert!(
        philosophy_index < policy_index
            && policy_index < requirement_index
            && requirement_index < feature_index,
        "interactive menu should follow the documented layer order:\n{stdout}",
    );
    assert!(
        summary.find("requirement=").expect("requirement count")
            < summary.find("feature=").expect("feature count"),
        "summary counts should list requirement before feature:\n{summary}",
    );
}

#[test]
fn browse_command_menu_selection_three_opens_requirements() {
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("browse")
        .arg(fixture_path("passing"))
        .write_stdin("3\n1\n0\n0\n")
        .output()
        .expect("browse command should run");

    assert!(output.status.success(), "browse UI should not fail");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("=== requirement detail ==="),
        "selection 3 should open the requirement layer:\n{stdout}",
    );
    assert!(
        !stdout.contains("=== feature detail ==="),
        "selection 3 should not dispatch to the feature layer:\n{stdout}",
    );
}

#[test]
fn browse_command_menu_selection_four_opens_features() {
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("browse")
        .arg(fixture_path("passing"))
        .write_stdin("4\n1\n0\n0\n")
        .output()
        .expect("browse command should run");

    assert!(output.status.success(), "browse UI should not fail");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("=== feature detail ==="),
        "selection 4 should open the feature layer:\n{stdout}",
    );
    assert!(
        !stdout.contains("=== requirement detail ==="),
        "selection 4 should not dispatch to the requirement layer:\n{stdout}",
    );
}

#[test]
fn browse_command_handles_detail_views_without_links() {
    let tempdir = tempdir().expect("tempdir should exist");
    let docs_root = tempdir.path().join("docs/syu");
    fs::create_dir_all(docs_root.join("philosophy")).expect("philosophy dir");
    fs::create_dir_all(docs_root.join("policies")).expect("policies dir");
    fs::create_dir_all(docs_root.join("requirements")).expect("requirements dir");
    fs::create_dir_all(docs_root.join("features")).expect("features dir");

    fs::write(
        docs_root.join("philosophy/foundation.yaml"),
        "category: Philosophy\nversion: 1\nlanguage: en\nphilosophies:\n  - id: PHIL-001\n    title: Solo philosophy\n    product_design_principle: Keep browsing resilient.\n    coding_guideline: Prefer explicit navigation.\n    linked_policies: []\n",
    )
    .expect("philosophy file");
    fs::write(
        docs_root.join("policies/policies.yaml"),
        "category: Policies\nversion: 1\nlanguage: en\npolicies:\n  - id: POL-001\n    title: Solo policy\n    summary: No outward links.\n    description: Browse should still render details.\n    linked_philosophies: []\n    linked_requirements: []\n",
    )
    .expect("policy file");
    fs::write(
        docs_root.join("requirements/core.yaml"),
        "category: Core\nprefix: REQ\nrequirements:\n  - id: REQ-001\n    title: Solo requirement\n    description: No outward links.\n    priority: high\n    status: planned\n    linked_policies: []\n    linked_features: []\n    tests: {}\n",
    )
    .expect("requirement file");
    fs::write(
        docs_root.join("features/features.yaml"),
        "version: \"1\"\nfiles:\n  - kind: solo\n    file: solo.yaml\n",
    )
    .expect("feature registry");
    fs::write(
        docs_root.join("features/solo.yaml"),
        "category: Features\nversion: 1\nlanguage: en\nfeatures:\n  - id: FEAT-001\n    title: Solo feature\n    summary: No outward links.\n    status: planned\n    linked_requirements: []\n    implementations: {}\n",
    )
    .expect("feature file");

    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("browse")
        .arg(tempdir.path())
        .write_stdin("1\n1\n")
        .output()
        .expect("browse command should run");

    assert!(output.status.success(), "browse UI should not fail");
}

#[test]
fn browse_command_handles_empty_workspaces_and_prompt_retries() {
    let tempdir = tempdir().expect("tempdir should exist");
    fs::create_dir_all(tempdir.path()).expect("workspace dir");

    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("browse")
        .arg(tempdir.path())
        .write_stdin("\n9\n1\n\nq\n")
        .output()
        .expect("browse command should run");

    assert!(output.status.success(), "browse UI should not fail");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Please enter a number between 0 and 5."));
    assert!(stdout.contains("No philosophy entries are currently available."));
}

#[test]
// REQ-CORE-015
fn browse_command_non_interactive_prints_spec_tree_and_exits() {
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("browse")
        .arg(fixture_path("passing"))
        .arg("--non-interactive")
        .output()
        .expect("browse command should run");

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("=== syu spec tree ==="),
        "should show spec tree header"
    );
    assert!(
        stdout.contains("=== philosophy"),
        "should show philosophy section"
    );
    assert!(
        stdout.contains("=== requirement"),
        "should show requirement section"
    );
    assert!(
        stdout.contains("PHIL-TRACE-001"),
        "should list philosophy IDs"
    );
}

#[test]
// REQ-CORE-015
fn browse_command_non_interactive_shows_errors_when_workspace_is_failing() {
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("browse")
        .arg(fixture_path("failing"))
        .arg("--non-interactive")
        .output()
        .expect("browse command should run");

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("=== errors ("),
        "should show errors section: {stdout}"
    );
    assert!(
        stdout.contains("SYU-graph-reference-001"),
        "should list error codes: {stdout}"
    );
    assert!(
        stdout.contains("Linked definitions must exist"),
        "should include rule titles: {stdout}"
    );
    assert!(
        stdout.contains("Linked requirement `REQ-MISSING-999` does not exist."),
        "should include issue messages: {stdout}"
    );
    assert!(
        stdout.contains("suggestion: Declare requirement `REQ-MISSING-999`"),
        "should include actionable suggestions: {stdout}"
    );
}

#[test]
// REQ-CORE-015
fn browse_command_non_interactive_supports_json_output() {
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .args([
            "browse",
            fixture_path("passing").to_str().expect("utf8 path"),
            "--non-interactive",
            "--format",
            "json",
        ])
        .output()
        .expect("browse command should run");

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let json: Value = serde_json::from_slice(&output.stdout).expect("output should be valid json");
    assert_eq!(json["definition_counts"]["philosophies"], 1);
    assert_eq!(json["groups"]["philosophies"][0]["id"], "PHIL-TRACE-001");
    assert_eq!(json["groups"]["requirements"][0]["id"], "REQ-TRACE-001");
    assert_eq!(
        json["issues"]
            .as_array()
            .expect("issues should be an array")
            .len(),
        0
    );
    assert_eq!(
        json["spec_root"],
        fixture_path("passing")
            .join("docs/syu")
            .display()
            .to_string()
    );
}
