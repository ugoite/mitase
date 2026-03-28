use assert_cmd::cargo::CommandCargoExt;
use serde_json::Value;
use std::{fs, path::PathBuf, process::Command};
use tempfile::tempdir;

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/workspaces")
        .join(name)
}

fn write_show_fixture_workspace() -> tempfile::TempDir {
    let tempdir = tempdir().expect("tempdir should exist");
    let docs_root = tempdir.path().join("docs/syu");
    fs::create_dir_all(docs_root.join("philosophy")).expect("philosophy dir");
    fs::create_dir_all(docs_root.join("policies")).expect("policies dir");
    fs::create_dir_all(docs_root.join("requirements")).expect("requirements dir");
    fs::create_dir_all(docs_root.join("features")).expect("features dir");

    fs::write(
        docs_root.join("philosophy/foundation.yaml"),
        "category: Philosophy\nversion: 1\nlanguage: en\nphilosophies:\n  - id: PHIL-001\n    title: Solo philosophy\n    product_design_principle: Keep lookups explicit.\n    coding_guideline: Prefer one-command inspection.\n    linked_policies: []\n",
    )
    .expect("philosophy file");
    fs::write(
        docs_root.join("policies/policies.yaml"),
        "category: Policies\nversion: 1\nlanguage: en\npolicies:\n  - id: POL-001\n    title: Solo policy\n    summary: Empty links are okay.\n    description: Show should still render details.\n    linked_philosophies: []\n    linked_requirements: []\n",
    )
    .expect("policy file");
    fs::write(
        docs_root.join("requirements/core.yaml"),
        "category: Core\nprefix: REQ\nrequirements:\n  - id: REQ-001\n    title: Doc-only requirement\n    description: Uses doc_contains without symbols.\n    priority: medium\n    status: implemented\n    linked_policies: []\n    linked_features: []\n    tests:\n      rust:\n        - file: src/doc_only.rs\n          doc_contains:\n            - REQ-001\n",
    )
    .expect("requirement file");
    fs::write(
        docs_root.join("features/features.yaml"),
        "version: \"1\"\nfiles:\n  - kind: solo\n    file: solo.yaml\n",
    )
    .expect("feature registry");
    fs::write(
        docs_root.join("features/solo.yaml"),
        "category: Features\nversion: 1\nfeatures:\n  - id: FEAT-001\n    title: Empty feature\n    summary: Leaves implementations empty.\n    status: planned\n    linked_requirements: []\n    implementations: {}\n",
    )
    .expect("feature file");

    tempdir
}

#[test]
// REQ-CORE-018
fn show_command_renders_philosophy_details_in_text_format() {
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("show")
        .arg("PHIL-TRACE-001")
        .arg(fixture_path("passing"))
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Kind: philosophy"));
    assert!(stdout.contains("Linked policies:"));
    assert!(stdout.contains("POL-TRACE-001\tRequirements need executable evidence"));
}

#[test]
// REQ-CORE-018
fn show_command_renders_policy_details_in_text_format() {
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("show")
        .arg("POL-TRACE-001")
        .arg(fixture_path("passing"))
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Kind: policy"));
    assert!(stdout.contains("Linked philosophies:"));
    assert!(stdout.contains("PHIL-TRACE-001\tTrace everything"));
}

#[test]
// REQ-CORE-018
fn show_command_renders_requirement_details_in_text_format() {
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("show")
        .arg("REQ-TRACE-001")
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
    assert!(stdout.contains("Kind: requirement"));
    assert!(stdout.contains("ID: REQ-TRACE-001"));
    assert!(stdout.contains("Linked features:"));
    assert!(stdout.contains("FEAT-TRACE-001\tRust implementation trace"));
    assert!(stdout.contains("Declared tests:"));
    assert!(stdout.contains("src/rust_trace_tests.rs"));
}

#[test]
// REQ-CORE-018
fn show_command_renders_feature_details_in_text_format() {
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("show")
        .arg("FEAT-TRACE-001")
        .arg(fixture_path("passing"))
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Kind: feature"));
    assert!(stdout.contains("Linked requirements:"));
    assert!(stdout.contains("REQ-TRACE-001\tRust requirement trace"));
    assert!(stdout.contains("Declared implementations:"));
}

#[test]
// REQ-CORE-018
fn show_command_supports_json_output_for_each_kind() {
    for (id, kind) in [
        ("PHIL-TRACE-001", "philosophy"),
        ("POL-TRACE-001", "policy"),
        ("REQ-TRACE-001", "requirement"),
        ("FEAT-TRACE-002", "feature"),
    ] {
        let output = Command::cargo_bin("syu")
            .expect("binary should build")
            .arg("show")
            .arg(id)
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

        let json: Value =
            serde_json::from_slice(&output.stdout).expect("output should be valid JSON");
        assert_eq!(json["kind"], kind);
        assert_eq!(json["item"]["id"], id);
    }
}

#[test]
// REQ-CORE-018
fn show_command_supports_json_output() {
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("show")
        .arg("FEAT-TRACE-002")
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
    assert_eq!(json["item"]["id"], "FEAT-TRACE-002");
    assert_eq!(json["item"]["linked_requirements"][0], "REQ-TRACE-002");
}

#[test]
// REQ-CORE-018
fn show_command_reads_items_from_workspaces_with_validation_errors() {
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("show")
        .arg("REQ-FAIL-001")
        .arg(fixture_path("failing"))
        .output()
        .expect("command should run");

    assert!(
        output.status.success(),
        "validation issues should not block lookups"
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("ID: REQ-FAIL-001"));
}

#[test]
// REQ-CORE-018
fn show_command_handles_empty_links_and_doc_only_traces() {
    let tempdir = write_show_fixture_workspace();

    let philosophy = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("show")
        .arg("PHIL-001")
        .arg(tempdir.path())
        .output()
        .expect("command should run");
    assert!(philosophy.status.success());
    let philosophy_stdout = String::from_utf8_lossy(&philosophy.stdout);
    assert!(philosophy_stdout.contains("Linked policies:"));
    assert!(philosophy_stdout.contains("- none"));

    let feature = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("show")
        .arg("FEAT-001")
        .arg(tempdir.path())
        .output()
        .expect("command should run");
    assert!(feature.status.success());
    let feature_stdout = String::from_utf8_lossy(&feature.stdout);
    assert!(feature_stdout.contains("Declared implementations:"));
    assert!(feature_stdout.contains("- none"));

    let requirement = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("show")
        .arg("REQ-001")
        .arg(tempdir.path())
        .output()
        .expect("command should run");
    assert!(requirement.status.success());
    let requirement_stdout = String::from_utf8_lossy(&requirement.stdout);
    assert!(requirement_stdout.contains("symbols: -"));
    assert!(requirement_stdout.contains("doc_contains: REQ-001"));
}

#[test]
// REQ-CORE-018
fn show_command_errors_when_the_id_is_missing() {
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("show")
        .arg("REQ-MISSING-999")
        .arg(fixture_path("passing"))
        .output()
        .expect("command should run");

    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("was not found"));
    assert!(
        stderr.contains("syu list"),
        "hint should suggest syu list: {stderr}"
    );
}

#[test]
// REQ-CORE-018
fn show_command_errors_for_ids_without_supported_prefixes() {
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("show")
        .arg("MISSING")
        .arg(fixture_path("passing"))
        .output()
        .expect("command should run");

    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("was not found"));
}

#[test]
// REQ-CORE-018
fn show_command_missing_id_in_json_mode_omits_hint() {
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("show")
        .arg("REQ-MISSING-999")
        .arg(fixture_path("passing"))
        .arg("--format")
        .arg("json")
        .output()
        .expect("command should run");

    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("was not found"));
    assert!(
        !stderr.contains("syu list"),
        "json mode should not show hint: {stderr}"
    );
}
