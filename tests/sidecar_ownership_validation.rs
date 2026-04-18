use assert_cmd::cargo::CommandCargoExt;
use std::{fs, path::Path, process::Command};
use tempfile::tempdir;

fn write_workspace(root: &Path, with_manifest: bool) {
    fs::create_dir_all(root.join("docs/syu/philosophy")).expect("philosophy dir");
    fs::create_dir_all(root.join("docs/syu/policies")).expect("policies dir");
    fs::create_dir_all(root.join("docs/syu/requirements")).expect("requirements dir");
    fs::create_dir_all(root.join("docs/syu/features")).expect("features dir");
    fs::create_dir_all(root.join("src")).expect("src dir");

    fs::write(
        root.join("syu.yaml"),
        format!(
            "version: {version}\nspec:\n  root: docs/syu\nvalidate:\n  allow_planned: true\n  trace_ownership_mode: sidecar\nruntimes:\n  python:\n    command: auto\n  node:\n    command: auto\n",
            version = env!("CARGO_PKG_VERSION")
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
        "category: Core Requirements\nprefix: REQ\n\nrequirements:\n  - id: REQ-001\n    title: Validate a sidecar-owned Rust trace\n    description: A Rust trace can declare ownership in a sidecar manifest.\n    priority: high\n    status: implemented\n    linked_policies:\n      - POL-001\n    linked_features:\n      - FEAT-001\n    tests:\n      rust:\n        - file: src/trace.rs\n          symbols:\n            - req_trace\n",
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
        "category: Core Features\nversion: 1\n\nfeatures:\n  - id: FEAT-001\n    title: Rust trace implementation\n    summary: Keep the implementation symbol explicit.\n    status: implemented\n    linked_requirements:\n      - REQ-001\n    implementations:\n      rust:\n        - file: src/trace.rs\n          symbols:\n            - req_trace\n",
    )
    .expect("feature");

    fs::write(root.join("src/trace.rs"), "pub fn req_trace() {}\n").expect("rust trace");

    if with_manifest {
        fs::write(
            root.join("src/trace.rs.syu-ownership.yaml"),
            "version: 1\nowners:\n  - id: FEAT-001\n    symbols:\n      - req_trace\n  - id: REQ-001\n    symbols:\n      - req_trace\n",
        )
        .expect("ownership manifest");
    }
}

#[test]
// REQ-CORE-002
fn validate_accepts_sidecar_ownership_manifests() {
    let tempdir = tempdir().expect("tempdir should exist");
    write_workspace(tempdir.path(), true);

    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("validate")
        .arg(tempdir.path())
        .output()
        .expect("validate should run");

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("syu validate passed"));
}

#[test]
// REQ-CORE-002
fn validate_reports_missing_sidecar_ownership_manifests() {
    let tempdir = tempdir().expect("tempdir should exist");
    write_workspace(tempdir.path(), false);

    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("validate")
        .arg(tempdir.path())
        .output()
        .expect("validate should run");

    assert!(
        !output.status.success(),
        "validation should fail without a sidecar ownership manifest"
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("SYU-trace-id-001"));
    assert!(stdout.contains("src/trace.rs.syu-ownership.yaml"));
}

#[test]
// REQ-CORE-002
fn validate_reports_duplicate_sidecar_owner_entries() {
    let tempdir = tempdir().expect("tempdir should exist");
    write_workspace(tempdir.path(), false);
    fs::write(
        tempdir.path().join("src/trace.rs.syu-ownership.yaml"),
        "version: 1\nowners:\n  - id: REQ-001\n    symbols:\n      - req_trace\n  - id: REQ-001\n    symbols:\n      - other_symbol\n",
    )
    .expect("ownership manifest");

    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("validate")
        .arg(tempdir.path())
        .output()
        .expect("validate should run");

    assert!(
        !output.status.success(),
        "validation should fail with duplicate sidecar owner entries"
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("SYU-trace-id-001"));
    assert!(stdout.contains("duplicate ownership entry `REQ-001`"));
}
