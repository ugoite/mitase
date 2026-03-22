// REQ-CORE-001

use assert_cmd::cargo::CommandCargoExt;
use serde_json::Value;
use std::{fs, path::Path, process::Command};
use tempfile::tempdir;

fn write_workspace(root: &Path) {
    fs::create_dir_all(root.join("docs/syu/philosophy")).expect("philosophy dir");
    fs::create_dir_all(root.join("docs/syu/policies")).expect("policies dir");
    fs::create_dir_all(root.join("docs/syu/requirements")).expect("requirements dir");
    fs::create_dir_all(root.join("docs/syu/features")).expect("features dir");
    fs::create_dir_all(root.join("src")).expect("src dir");

    fs::write(
        root.join("syu.yaml"),
        format!(
            "version: {version}\nspec:\n  root: docs/syu\nvalidate:\n  default_fix: false\n  allow_planned: true\n  require_non_orphaned_items: false\n  require_symbol_trace_coverage: false\nruntimes:\n  python:\n    command: auto\n  node:\n    command: auto\n",
            version = env!("CARGO_PKG_VERSION"),
        ),
    )
    .expect("config");

    fs::write(
        root.join("docs/syu/philosophy/foundation.yaml"),
        "category: Philosophy\nversion: 1\nlanguage: en\n\nphilosophies:\n  - id: PHIL-WARN-001\n    title: Keep weakly linked ideas visible\n    product_design_principle: Warnings should stay reviewable.\n    coding_guideline: Keep optional gaps explicit.\n    linked_policies: []\n  - id: PHIL-OK-001\n    title: Keep traceability connected\n    product_design_principle: Connected items should stay explicit.\n    coding_guideline: Prefer reciprocal links.\n    linked_policies:\n      - POL-001\n",
    )
    .expect("philosophy");

    fs::write(
        root.join("docs/syu/policies/policies.yaml"),
        "category: Policies\nversion: 1\nlanguage: en\n\npolicies:\n  - id: POL-001\n    title: Connected work should point to real evidence\n    summary: Keep validation output filterable without hiding repository drift.\n    description: This fixture keeps one valid chain plus one warning-only philosophy.\n    linked_philosophies:\n      - PHIL-OK-001\n    linked_requirements:\n      - REQ-001\n",
    )
    .expect("policy");

    fs::write(
        root.join("docs/syu/requirements/core.yaml"),
        "category: Core Requirements\nprefix: REQ\n\nrequirements:\n  - id: REQ-001\n    title: Requirement traces should point to real files\n    description: This fixture intentionally points to a missing test file.\n    priority: high\n    status: implemented\n    linked_policies:\n      - POL-001\n    linked_features:\n      - FEAT-001\n    tests:\n      rust:\n        - file: tests/missing_trace.rs\n          symbols:\n            - missing_trace\n",
    )
    .expect("requirement");

    fs::write(
        root.join("docs/syu/features/features.yaml"),
        format!(
            "version: \"{}\"\nfiles:\n  - kind: core\n    file: core.yaml\n",
            env!("CARGO_PKG_VERSION")
        ),
    )
    .expect("feature registry");

    fs::write(
        root.join("docs/syu/features/core.yaml"),
        "category: Core Features\nversion: 1\n\nfeatures:\n  - id: FEAT-001\n    title: Feature traces should stay valid\n    summary: The feature trace passes so the requirement trace failure stands out.\n    status: implemented\n    linked_requirements:\n      - REQ-001\n    implementations:\n      rust:\n        - file: src/feature.rs\n          symbols:\n            - feature_impl\n",
    )
    .expect("feature");

    fs::write(
        root.join("src/feature.rs"),
        "/// FEAT-001\npub fn feature_impl() {}\n",
    )
    .expect("source");
}

#[test]
fn validate_filters_text_output_by_severity() {
    let tempdir = tempdir().expect("tempdir should exist");
    write_workspace(tempdir.path());

    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("validate")
        .arg(tempdir.path())
        .arg("--severity")
        .arg("warning")
        .output()
        .expect("validate should run");

    assert!(
        !output.status.success(),
        "filtered warnings should still reflect hidden errors"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("syu validate failed (filtered view)"));
    assert!(stdout.contains("filters: severity=warning"));
    assert!(stdout.contains("showing 1 of 2 issues after filtering"));
    assert!(stdout.contains("SYU-graph-links-001"));
    assert!(!stdout.contains("SYU-trace-file-002"));
}

#[test]
fn validate_filters_json_output_by_genre() {
    let tempdir = tempdir().expect("tempdir should exist");
    write_workspace(tempdir.path());

    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("validate")
        .arg(tempdir.path())
        .arg("--format")
        .arg("json")
        .arg("--genre")
        .arg("trace")
        .output()
        .expect("validate should run");

    assert!(
        !output.status.success(),
        "trace-only filtered output should still reflect hidden warnings or errors"
    );

    let json: Value = serde_json::from_slice(&output.stdout).expect("output should be valid JSON");
    assert_eq!(json["filtered_view"]["genres"][0], "trace");
    assert_eq!(json["filtered_view"]["displayed_issue_count"], 1);
    assert_eq!(json["filtered_view"]["total_issue_count"], 2);
    assert_eq!(json["filtered_view"]["hidden_issue_count"], 1);

    let issues = json["issues"]
        .as_array()
        .expect("issues should be represented as an array");
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0]["code"], "SYU-trace-file-002");

    let rules = json["referenced_rules"]
        .as_array()
        .expect("referenced rules should be represented as an array");
    assert_eq!(rules.len(), 1);
    assert_eq!(rules[0]["genre"], "trace");
}

#[test]
fn validate_reports_when_rule_filters_hide_every_issue() {
    let tempdir = tempdir().expect("tempdir should exist");
    write_workspace(tempdir.path());

    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("validate")
        .arg(tempdir.path())
        .arg("--rule")
        .arg("SYU-coverage-public-001")
        .output()
        .expect("validate should run");

    assert!(
        !output.status.success(),
        "hidden errors should still produce a failing exit code"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("filters: rule=SYU-coverage-public-001"));
    assert!(stdout.contains("showing 0 of 2 issues after filtering"));
    assert!(stdout.contains("no issues matched the active filters."));
}
