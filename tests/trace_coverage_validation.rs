// REQ-CORE-002

use assert_cmd::cargo::CommandCargoExt;
use std::{fs, path::Path, process::Command};
use tempfile::tempdir;

fn write_workspace(root: &Path, cover_everything: bool) {
    fs::create_dir_all(root.join("docs/syu/philosophy")).expect("philosophy dir");
    fs::create_dir_all(root.join("docs/syu/policies")).expect("policies dir");
    fs::create_dir_all(root.join("docs/syu/requirements")).expect("requirements dir");
    fs::create_dir_all(root.join("docs/syu/features")).expect("features dir");
    fs::create_dir_all(root.join("src")).expect("src dir");
    fs::create_dir_all(root.join("tests")).expect("tests dir");

    fs::write(
        root.join("syu.yaml"),
        format!(
            "version: {version}\nspec:\n  root: docs/syu\nvalidate:\n  default_fix: false\n  allow_planned: true\n  require_non_orphaned_items: true\n  require_symbol_trace_coverage: true\nruntimes:\n  python:\n    command: auto\n  node:\n    command: auto\n",
            version = env!("CARGO_PKG_VERSION"),
        ),
    )
    .expect("config");

    fs::write(
        root.join("docs/syu/philosophy/foundation.yaml"),
        "category: Philosophy\nversion: 1\nlanguage: en\n\nphilosophies:\n  - id: PHIL-001\n    title: Keep the graph explicit\n    product_design_principle: Every layer should be connected.\n    coding_guideline: Prefer explicit ownership.\n    linked_policies:\n      - POL-001\n",
    )
    .expect("philosophy");

    fs::write(
        root.join("docs/syu/policies/policies.yaml"),
        "category: Policies\nversion: 1\nlanguage: en\n\npolicies:\n  - id: POL-001\n    title: Coverage can be enforced when needed\n    summary: Public symbols and tests may require ownership.\n    description: This fixture turns the strict coverage rule on.\n    linked_philosophies:\n      - PHIL-001\n    linked_requirements:\n      - REQ-001\n",
    )
    .expect("policy");

    let requirement_symbols = if cover_everything {
        "          symbols:\n            - '*'\n"
    } else {
        "          symbols:\n            - covered_case\n"
    };
    fs::write(
        root.join("docs/syu/requirements/core.yaml"),
        format!(
            "category: Core Requirements\nprefix: REQ\n\nrequirements:\n  - id: REQ-001\n    title: Tests must stay justified\n    description: Each test should link to a requirement.\n    priority: high\n    status: implemented\n    linked_policies:\n      - POL-001\n    linked_features:\n      - FEAT-001\n    tests:\n      rust:\n        - file: tests/coverage.rs\n{requirement_symbols}",
        ),
    )
    .expect("requirement");

    let feature_symbols = if cover_everything {
        "          symbols:\n            - '*'\n"
    } else {
        "          symbols:\n            - covered_api\n"
    };
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
        format!(
            "category: Core Features\nversion: 1\n\nfeatures:\n  - id: FEAT-001\n    title: Public APIs must stay owned\n    summary: Each public API should link to a feature.\n    status: implemented\n    linked_requirements:\n      - REQ-001\n    implementations:\n      rust:\n        - file: src/lib.rs\n{feature_symbols}",
        ),
    )
    .expect("feature");

    fs::write(
        root.join("src/lib.rs"),
        "/// FEAT-001\npub fn covered_api() {}\n\npub fn uncovered_api() {}\n",
    )
    .expect("source");
    fs::write(
        root.join("tests/coverage.rs"),
        "/// REQ-001\n#[test]\nfn covered_case() {}\n\n#[test]\nfn uncovered_case() {}\n",
    )
    .expect("tests");
}

#[test]
fn validate_reports_untracked_public_symbols_and_tests() {
    let tempdir = tempdir().expect("tempdir should exist");
    write_workspace(tempdir.path(), false);

    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("validate")
        .arg(tempdir.path())
        .output()
        .expect("validate should run");

    assert!(!output.status.success(), "coverage gaps should fail");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("public-symbol-untracked"));
    assert!(stdout.contains("test-symbol-untracked"));
}

#[test]
fn validate_accepts_wildcard_file_coverage() {
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
}
