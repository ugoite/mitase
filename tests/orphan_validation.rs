// REQ-CORE-001

use assert_cmd::cargo::CommandCargoExt;
use std::{fs, path::Path, process::Command};
use tempfile::tempdir;

fn write_workspace(root: &Path, require_non_orphaned_items: bool) {
    fs::create_dir_all(root.join("docs/spec/philosophy")).expect("philosophy dir");
    fs::create_dir_all(root.join("docs/spec/policies")).expect("policies dir");
    fs::create_dir_all(root.join("docs/spec/requirements")).expect("requirements dir");
    fs::create_dir_all(root.join("docs/spec/features")).expect("features dir");

    fs::write(
        root.join("syu.yaml"),
        format!(
            "version: {version}\nspec:\n  root: docs/spec\nvalidate:\n  default_fix: false\n  allow_planned: true\n  require_non_orphaned_items: {require_non_orphaned_items}\n  require_symbol_trace_coverage: false\nruntimes:\n  python:\n    command: auto\n  node:\n    command: auto\n",
            version = env!("CARGO_PKG_VERSION"),
            require_non_orphaned_items = if require_non_orphaned_items { "true" } else { "false" },
        ),
    )
    .expect("config");

    fs::write(
        root.join("docs/spec/philosophy/foundation.yaml"),
        "category: Philosophy\nversion: 1\nlanguage: en\n\nphilosophies:\n  - id: PHIL-001\n    title: Keep the graph deliberate\n    product_design_principle: Every layer should stay explicit.\n    coding_guideline: Avoid silent drift.\n    linked_policies: []\n",
    )
    .expect("philosophy");

    fs::write(
        root.join("docs/spec/policies/policies.yaml"),
        "category: Policies\nversion: 1\nlanguage: en\n\npolicies:\n  - id: POL-001\n    title: Keep isolated entries visible\n    summary: Orphans should be configurable.\n    description: This fixture isolates a policy on purpose.\n    linked_philosophies: []\n    linked_requirements: []\n",
    )
    .expect("policy");

    fs::write(
        root.join("docs/spec/requirements/core.yaml"),
        "category: Core Requirements\nprefix: REQ\n\nrequirements:\n  - id: REQ-001\n    title: Keep orphan handling configurable\n    description: This fixture isolates a requirement on purpose.\n    priority: high\n    status: planned\n    linked_policies: []\n    linked_features: []\n    tests: {}\n",
    )
    .expect("requirement");

    fs::write(
        root.join("docs/spec/features/features.yaml"),
        format!(
            "version: \"{}\"\nfiles:\n  - kind: core\n    file: core.yaml\n",
            env!("CARGO_PKG_VERSION")
        ),
    )
    .expect("feature registry");

    fs::write(
        root.join("docs/spec/features/core.yaml"),
        "category: Core Features\nversion: 1\n\nfeatures:\n  - id: FEAT-001\n    title: Toggle orphan enforcement\n    summary: This fixture isolates a feature on purpose.\n    status: planned\n    linked_requirements: []\n    implementations: {}\n",
    )
    .expect("feature");
}

#[test]
fn validate_rejects_orphaned_definitions_by_default() {
    let tempdir = tempdir().expect("tempdir should exist");
    write_workspace(tempdir.path(), true);

    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("validate")
        .arg(tempdir.path())
        .output()
        .expect("validate should run");

    assert!(!output.status.success(), "orphaned definitions should fail");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("orphaned-definition"));
    assert!(stdout.contains("Definitions must not be isolated from the layered graph"));
}

#[test]
fn validate_allows_orphaned_definitions_when_disabled_in_config() {
    let tempdir = tempdir().expect("tempdir should exist");
    write_workspace(tempdir.path(), false);

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
}
