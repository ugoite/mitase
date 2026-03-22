// REQ-CORE-001
// REQ-CORE-002

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
    fs::create_dir_all(root.join("tests")).expect("tests dir");

    fs::write(
        root.join("syu.yaml"),
        format!(
            "version: {version}\nspec:\n  root: docs/syu\nvalidate:\n  default_fix: false\n  allow_planned: true\n  require_non_orphaned_items: true\n  require_symbol_trace_coverage: false\nruntimes:\n  python:\n    command: auto\n  node:\n    command: auto\n",
            version = env!("CARGO_PKG_VERSION"),
        ),
    )
    .expect("config");

    fs::write(
        root.join("docs/syu/philosophy/foundation.yaml"),
        "category: Philosophy\nversion: 1\nlanguage: en\n\nphilosophies:\n  - id: PHIL-001\n    title: Keep linked intent explicit\n    product_design_principle: Duplicate graph edges should stay visible.\n    coding_guideline: Keep adjacent links unique per list.\n    linked_policies:\n      - POL-001\n      - POL-001\n",
    )
    .expect("philosophy");

    fs::write(
        root.join("docs/syu/policies/policies.yaml"),
        "category: Policies\nversion: 1\nlanguage: en\n\npolicies:\n  - id: POL-001\n    title: Keep trace evidence distinct\n    summary: Repeated links or trace records should be rejected.\n    description: This fixture duplicates both graph links and trace mappings.\n    linked_philosophies:\n      - PHIL-001\n    linked_requirements:\n      - REQ-001\n      - REQ-001\n",
    )
    .expect("policy");

    fs::write(
        root.join("docs/syu/requirements/core.yaml"),
        "category: Core Requirements\nprefix: REQ\n\nrequirements:\n  - id: REQ-001\n    title: Requirement traces should stay unique\n    description: Duplicate requirement links and tests should fail validation.\n    priority: high\n    status: implemented\n    linked_policies:\n      - POL-001\n      - POL-001\n    linked_features:\n      - FEAT-001\n      - FEAT-001\n    tests:\n      rust:\n        - file: tests/duplicate_trace.rs\n          symbols:\n            - requirement_trace\n          doc_contains:\n            - duplicate requirement trace\n        - file: tests/duplicate_trace.rs\n          symbols:\n            - requirement_trace\n          doc_contains:\n            - duplicate requirement trace\n",
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
        "category: Core Features\nversion: 1\n\nfeatures:\n  - id: FEAT-001\n    title: Feature traces should stay unique\n    summary: Duplicate implementation records should fail validation.\n    status: implemented\n    linked_requirements:\n      - REQ-001\n      - REQ-001\n    implementations:\n      rust:\n        - file: src/duplicate_trace.rs\n          symbols:\n            - feature_impl\n          doc_contains:\n            - duplicate feature trace\n        - file: src/duplicate_trace.rs\n          symbols:\n            - feature_impl\n          doc_contains:\n            - duplicate feature trace\n",
    )
    .expect("feature");

    fs::write(
        root.join("tests/duplicate_trace.rs"),
        "/// REQ-001\n/// duplicate requirement trace\n#[test]\nfn requirement_trace() {}\n",
    )
    .expect("tests");

    fs::write(
        root.join("src/duplicate_trace.rs"),
        "/// FEAT-001\n/// duplicate feature trace\npub fn feature_impl() {}\n",
    )
    .expect("source");
}

#[test]
fn validate_reports_duplicate_relationship_entries() {
    let tempdir = tempdir().expect("tempdir should exist");
    write_workspace(tempdir.path());

    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("validate")
        .arg(tempdir.path())
        .output()
        .expect("validate should run");

    assert!(
        !output.status.success(),
        "duplicate relationships should fail"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("SYU-graph-duplicate-001"));
    assert!(stdout.contains("philosophy `PHIL-001` repeats linked policy `POL-001`"));
    assert!(stdout.contains("policy `POL-001` repeats linked requirement `REQ-001`"));
    assert!(stdout.contains("requirement `REQ-001` repeats linked feature `FEAT-001`"));
    assert!(stdout.contains("feature `FEAT-001` repeats linked requirement `REQ-001`"));
}

#[test]
fn validate_reports_duplicate_trace_entries() {
    let tempdir = tempdir().expect("tempdir should exist");
    write_workspace(tempdir.path());

    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("validate")
        .arg(tempdir.path())
        .arg("--format")
        .arg("json")
        .output()
        .expect("validate should run");

    assert!(!output.status.success(), "duplicate traces should fail");

    let json: Value = serde_json::from_slice(&output.stdout).expect("output should be valid JSON");
    let issues = json["issues"]
        .as_array()
        .expect("issues should be represented as an array");
    let trace_duplicates = issues
        .iter()
        .filter(|issue| issue["code"] == "SYU-trace-duplicate-001")
        .collect::<Vec<_>>();

    assert_eq!(trace_duplicates.len(), 2);
    assert!(trace_duplicates.iter().any(|issue| {
        issue["message"]
            .as_str()
            .is_some_and(|message| message.contains("file=`tests/duplicate_trace.rs`"))
    }));
    assert!(trace_duplicates.iter().any(|issue| {
        issue["message"]
            .as_str()
            .is_some_and(|message| message.contains("file=`src/duplicate_trace.rs`"))
    }));

    let rules = json["referenced_rules"]
        .as_array()
        .expect("referenced rules should be represented as an array");
    assert!(
        rules
            .iter()
            .any(|rule| rule["code"] == "SYU-trace-duplicate-001")
    );
}
