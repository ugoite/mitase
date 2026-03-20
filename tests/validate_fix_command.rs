use assert_cmd::cargo::CommandCargoExt;
use std::{fs, path::Path, process::Command};
use tempfile::tempdir;

fn write_workspace(root: &Path, default_fix: bool) {
    fs::create_dir_all(root.join("docs/syu/philosophy")).expect("philosophy dir");
    fs::create_dir_all(root.join("docs/syu/policies")).expect("policies dir");
    fs::create_dir_all(root.join("docs/syu/requirements")).expect("requirements dir");
    fs::create_dir_all(root.join("docs/syu/features")).expect("features dir");
    fs::create_dir_all(root.join("src")).expect("src dir");

    fs::write(
        root.join("syu.yaml"),
        format!(
            "version: {version}\nspec:\n  root: docs/syu\nvalidate:\n  default_fix: {default_fix}\n  allow_planned: true\nruntimes:\n  python:\n    command: auto\n  node:\n    command: auto\n",
            version = env!("CARGO_PKG_VERSION"),
            default_fix = if default_fix { "true" } else { "false" }
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
    assert!(source.contains("/// REQ-001"));
    assert!(source.contains("/// requirement doc line"));
    assert!(source.contains("/// FEAT-001"));
    assert!(source.contains("/// feature doc line"));
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
