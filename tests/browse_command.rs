// REQ-CORE-015

use assert_cmd::{Command, cargo::CommandCargoExt};
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
