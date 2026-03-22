// REQ-CORE-001

use assert_cmd::cargo::CommandCargoExt;
use std::{fs, path::Path, process::Command};
use tempfile::tempdir;

fn write_workspace(root: &Path, require_reciprocal_links: bool, broken_reference: bool) {
    fs::create_dir_all(root.join("docs/syu/philosophy")).expect("philosophy dir");
    fs::create_dir_all(root.join("docs/syu/policies")).expect("policies dir");
    fs::create_dir_all(root.join("docs/syu/requirements")).expect("requirements dir");
    fs::create_dir_all(root.join("docs/syu/features")).expect("features dir");

    fs::write(
        root.join("syu.yaml"),
        format!(
            "version: {version}\nspec:\n  root: docs/syu\nvalidate:\n  default_fix: false\n  allow_planned: true\n  require_non_orphaned_items: true\n  require_reciprocal_links: {require_reciprocal_links}\n  require_symbol_trace_coverage: false\nruntimes:\n  python:\n    command: auto\n  node:\n    command: auto\n",
            version = env!("CARGO_PKG_VERSION"),
            require_reciprocal_links = if require_reciprocal_links {
                "true"
            } else {
                "false"
            },
        ),
    )
    .expect("config");

    fs::write(
        root.join("docs/syu/philosophy/foundation.yaml"),
        "category: Philosophy\nversion: 1\nlanguage: en\n\nphilosophies:\n  - id: PHIL-001\n    title: Keep graph intent deliberate\n    product_design_principle: Forward links should stay reviewable.\n    coding_guideline: Tighten reciprocity after migration when needed.\n    linked_policies:\n      - POL-001\n",
    )
    .expect("philosophy");

    let linked_requirement = if broken_reference {
        "REQ-MISSING"
    } else {
        "REQ-001"
    };
    fs::write(
        root.join("docs/syu/policies/policies.yaml"),
        format!(
            "category: Policies\nversion: 1\nlanguage: en\n\npolicies:\n  - id: POL-001\n    title: Let repositories phase in reciprocity\n    summary: Missing backlinks may be temporary during migration.\n    description: This fixture keeps reference validation active while reciprocal enforcement is configurable.\n    linked_philosophies:\n      - PHIL-001\n    linked_requirements:\n      - {linked_requirement}\n",
        ),
    )
    .expect("policy");

    fs::write(
        root.join("docs/syu/requirements/core.yaml"),
        "category: Core Requirements\nprefix: REQ\n\nrequirements:\n  - id: REQ-001\n    title: Keep requirement links explicit\n    description: This fixture intentionally omits a backlink to the policy.\n    priority: high\n    status: planned\n    linked_policies: []\n    linked_features:\n      - FEAT-001\n    tests: {}\n",
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
        "category: Core Features\nversion: 1\n\nfeatures:\n  - id: FEAT-001\n    title: Keep feature links explicit\n    summary: This fixture keeps feature reciprocity intact.\n    status: planned\n    linked_requirements:\n      - REQ-001\n    implementations: {}\n",
    )
    .expect("feature");
}

#[test]
fn validate_rejects_missing_reciprocal_links_by_default() {
    let tempdir = tempdir().expect("tempdir should exist");
    write_workspace(tempdir.path(), true, false);

    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("validate")
        .arg(tempdir.path())
        .output()
        .expect("validate should run");

    assert!(
        !output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("SYU-graph-reciprocal-001"));
}

#[test]
fn validate_allows_missing_reciprocal_links_when_disabled_in_config() {
    let tempdir = tempdir().expect("tempdir should exist");
    write_workspace(tempdir.path(), false, false);

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
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("syu validate passed"));
    assert!(!stdout.contains("SYU-graph-reciprocal-001"));
}

#[test]
fn validate_still_reports_missing_references_when_reciprocal_links_disabled() {
    let tempdir = tempdir().expect("tempdir should exist");
    write_workspace(tempdir.path(), false, true);

    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("validate")
        .arg(tempdir.path())
        .output()
        .expect("validate should run");

    assert!(
        !output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("SYU-graph-reference-001"));
    assert!(!stdout.contains("SYU-graph-reciprocal-001"));
}
