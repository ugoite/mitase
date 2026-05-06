use assert_cmd::cargo::CommandCargoExt;
use serde_json::Value;
use std::{fs, path::Path, process::Command};
use tempfile::tempdir;

fn write_workspace(root: &Path, default_fix: bool) {
    write_workspace_with_ownership_mode(root, default_fix, "mapping");
}

fn write_workspace_with_ownership_mode(root: &Path, default_fix: bool, trace_ownership_mode: &str) {
    fs::create_dir_all(root.join("docs/syu/philosophy")).expect("philosophy dir");
    fs::create_dir_all(root.join("docs/syu/policies")).expect("policies dir");
    fs::create_dir_all(root.join("docs/syu/requirements")).expect("requirements dir");
    fs::create_dir_all(root.join("docs/syu/features")).expect("features dir");
    fs::create_dir_all(root.join("src")).expect("src dir");

    fs::write(
        root.join("syu.yaml"),
        format!(
            "version: {version}\nspec:\n  root: docs/syu\nvalidate:\n  default_fix: {default_fix}\n  allow_planned: true\n  trace_ownership_mode: {trace_ownership_mode}\nruntimes:\n  python:\n    command: auto\n  node:\n    command: auto\n",
            version = env!("CARGO_PKG_VERSION"),
            default_fix = if default_fix { "true" } else { "false" },
            trace_ownership_mode = trace_ownership_mode
        ),
    )
    .expect("config");

    fs::write(
        root.join("docs/syu/philosophy/foundation.yaml"),
        "category: Philosophy\nversion: 1\nlanguage: en\n\nphilosophies:\n  - id: PHIL-001\n    title: Executable agreement\n    product_design_principle: Keep change traceable.\n    coding_guideline: Prefer explicit links.\n    linked_policies:\n      - POL-001\n",
    )
    .expect("philosophy");

    fs::write(
        root.join("docs/syu/policies/policies.yaml"),
        "category: Policies\nversion: 1\nlanguage: en\n\npolicies:\n  - id: POL-001\n    title: Keep symbols documented\n    summary: Requirements and features should remain explainable.\n    description: Every trace should point to a documented symbol.\n    linked_philosophies:\n      - PHIL-001\n    linked_requirements:\n      - REQ-001\n",
    )
    .expect("policy");

    fs::write(
        root.join("docs/syu/requirements/core.yaml"),
        "category: Core Requirements\nprefix: REQ\n\nrequirements:\n  - id: REQ-001\n    title: Validate a documented Rust trace\n    description: A Rust trace should expose the requirement in documentation.\n    priority: high\n    status: implemented\n    linked_policies:\n      - POL-001\n    linked_features:\n      - FEAT-001\n    tests:\n      rust:\n        - file: src/trace.rs\n          symbols:\n            - req_trace\n          doc_contains:\n            - requirement doc line\n",
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
        "category: Core Features\nversion: 1\n\nfeatures:\n  - id: FEAT-001\n    title: Rust trace implementation\n    summary: Keep the implementation symbol documented.\n    status: implemented\n    linked_requirements:\n      - REQ-001\n    implementations:\n      rust:\n        - file: src/trace.rs\n          symbols:\n            - req_trace\n          doc_contains:\n            - feature doc line\n",
    )
    .expect("feature");

    fs::write(root.join("src/trace.rs"), "pub fn req_trace() {}\n").expect("rust trace");
}

#[test]
// REQ-CORE-003
fn validate_fix_repairs_missing_trace_docs_for_rust_sources() {
    let tempdir = tempdir().expect("tempdir should exist");
    write_workspace(tempdir.path(), false);

    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("validate")
        .arg(tempdir.path())
        .arg("--fix")
        .output()
        .expect("validate should run");

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("applied 2 autofix updates across 1 files"));
    assert!(stdout.contains("syu validate passed"));

    let source = fs::read_to_string(tempdir.path().join("src/trace.rs")).expect("source");
    assert!(source.contains("/// requirement doc line"));
    assert!(source.contains("/// feature doc line"));
    assert!(!source.contains("REQ-001"));
    assert!(!source.contains("FEAT-001"));
}

#[test]
// REQ-CORE-003
fn validate_uses_config_default_fix_and_no_fix_disables_it() {
    let tempdir = tempdir().expect("tempdir should exist");
    write_workspace(tempdir.path(), true);

    let no_fix = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("validate")
        .arg(tempdir.path())
        .arg("--no-fix")
        .output()
        .expect("validate should run");

    assert!(
        !no_fix.status.success(),
        "validation should fail without fixes"
    );
    let source = fs::read_to_string(tempdir.path().join("src/trace.rs")).expect("source");
    assert_eq!(source, "pub fn req_trace() {}\n");

    let default_fix = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("validate")
        .arg(tempdir.path())
        .output()
        .expect("validate should run");

    assert!(
        default_fix.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&default_fix.stdout),
        String::from_utf8_lossy(&default_fix.stderr)
    );
    assert!(String::from_utf8_lossy(&default_fix.stdout).contains("applied 2 autofix updates"));
}

#[test]
// REQ-CORE-003
fn validate_fix_dry_run_reports_planned_changes_without_writing_files() {
    let tempdir = tempdir().expect("tempdir should exist");
    write_workspace_with_ownership_mode(tempdir.path(), false, "sidecar");

    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("validate")
        .arg(tempdir.path())
        .arg("--fix")
        .arg("--dry-run")
        .output()
        .expect("validate should run");

    assert!(
        !output.status.success(),
        "dry run should not mutate the workspace"
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("planned 4 autofix updates across 2 files"));
    assert!(stdout.contains("dry run; no files changed"));
    assert!(stdout.contains("planned fixes:"));
    assert!(stdout.contains("src/trace.rs.syu-ownership.yaml"));
    assert!(stdout.contains("SYU-trace-doc-001"));
    assert!(stdout.contains("SYU-trace-id-001"));

    let source = fs::read_to_string(tempdir.path().join("src/trace.rs")).expect("source");
    assert_eq!(source, "pub fn req_trace() {}\n");
    assert!(
        !tempdir
            .path()
            .join("src/trace.rs.syu-ownership.yaml")
            .exists()
    );
}

#[test]
// REQ-CORE-003
fn validate_fix_dry_run_exposes_machine_readable_plan() {
    let tempdir = tempdir().expect("tempdir should exist");
    write_workspace_with_ownership_mode(tempdir.path(), false, "sidecar");

    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("validate")
        .arg(tempdir.path())
        .arg("--fix")
        .arg("--dry-run")
        .arg("--format")
        .arg("json")
        .output()
        .expect("validate should run");

    assert!(
        !output.status.success(),
        "dry run should still preserve files"
    );

    let json: Value = serde_json::from_slice(&output.stdout).expect("output should be valid JSON");
    let plan = json["autofix_plan"].as_object().expect("plan should exist");
    assert_eq!(plan["planned_updates"].as_u64(), Some(4));
    assert_eq!(
        plan["updated_files"]
            .as_array()
            .expect("updated files should be an array")
            .len(),
        2
    );

    let changes = plan["changes"]
        .as_array()
        .expect("changes should be an array");
    assert_eq!(changes.len(), 4);
    assert!(changes.iter().any(|change| {
        change["path"]
            .as_str()
            .is_some_and(|path| path.ends_with("src/trace.rs"))
            && change["rules"]
                .as_array()
                .is_some_and(|rules| rules.iter().any(|rule| rule == "SYU-trace-doc-001"))
    }));
    assert!(changes.iter().any(|change| {
        change["path"]
            .as_str()
            .is_some_and(|path| path.ends_with("src/trace.rs.syu-ownership.yaml"))
            && change["rules"]
                .as_array()
                .is_some_and(|rules| rules.iter().any(|rule| rule == "SYU-trace-id-001"))
    }));
}

#[test]
// REQ-CORE-003
fn validate_fix_writes_sidecar_ownership_manifests_when_configured() {
    let tempdir = tempdir().expect("tempdir should exist");
    write_workspace_with_ownership_mode(tempdir.path(), false, "sidecar");

    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("validate")
        .arg(tempdir.path())
        .arg("--fix")
        .output()
        .expect("validate should run");

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("applied 4 autofix updates across 2 files"));
    assert!(stdout.contains("syu validate passed"));

    let source = fs::read_to_string(tempdir.path().join("src/trace.rs")).expect("source");
    assert!(!source.contains("REQ-001"));
    assert!(!source.contains("FEAT-001"));
    assert!(source.contains("requirement doc line"));
    assert!(source.contains("feature doc line"));

    let manifest = fs::read_to_string(tempdir.path().join("src/trace.rs.syu-ownership.yaml"))
        .expect("manifest");
    assert!(manifest.contains("id: FEAT-001"));
    assert!(manifest.contains("id: REQ-001"));
    assert!(manifest.contains("- req_trace"));
}
