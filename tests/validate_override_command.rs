// REQ-CORE-001
// REQ-CORE-002

use assert_cmd::cargo::CommandCargoExt;
use std::{fs, path::Path, process::Command};
use tempfile::tempdir;

fn write_config(
    root: &Path,
    allow_planned: bool,
    require_non_orphaned_items: bool,
    require_reciprocal_links: bool,
    require_symbol_trace_coverage: bool,
) {
    fs::write(
        root.join("syu.yaml"),
        format!(
            "version: {version}\nspec:\n  root: docs/syu\nvalidate:\n  default_fix: false\n  allow_planned: {allow_planned}\n  require_non_orphaned_items: {require_non_orphaned_items}\n  require_reciprocal_links: {require_reciprocal_links}\n  require_symbol_trace_coverage: {require_symbol_trace_coverage}\nruntimes:\n  python:\n    command: auto\n  node:\n    command: auto\n",
            version = env!("CARGO_PKG_VERSION"),
            allow_planned = if allow_planned { "true" } else { "false" },
            require_non_orphaned_items = if require_non_orphaned_items { "true" } else { "false" },
            require_reciprocal_links = if require_reciprocal_links { "true" } else { "false" },
            require_symbol_trace_coverage = if require_symbol_trace_coverage {
                "true"
            } else {
                "false"
            },
        ),
    )
    .expect("config");
}

fn write_planned_workspace(root: &Path, allow_planned: bool) {
    fs::create_dir_all(root.join("docs/syu/philosophy")).expect("philosophy dir");
    fs::create_dir_all(root.join("docs/syu/policies")).expect("policies dir");
    fs::create_dir_all(root.join("docs/syu/requirements")).expect("requirements dir");
    fs::create_dir_all(root.join("docs/syu/features")).expect("features dir");

    write_config(root, allow_planned, true, true, false);

    fs::write(
        root.join("docs/syu/philosophy/foundation.yaml"),
        "category: Philosophy\nversion: 1\nlanguage: en\n\nphilosophies:\n  - id: PHIL-001\n    title: Keep planning explicit\n    product_design_principle: Planned work should stay visible until delivery starts.\n    coding_guideline: Prefer explicit status values over implied intent.\n    linked_policies:\n      - POL-001\n",
    )
    .expect("philosophy");
    fs::write(
        root.join("docs/syu/policies/policies.yaml"),
        "category: Policies\nversion: 1\nlanguage: en\n\npolicies:\n  - id: POL-001\n    title: Planned work should remain reviewable\n    summary: Delivery states should support gradual adoption.\n    description: This fixture exercises temporary validate overrides for planned work.\n    linked_philosophies:\n      - PHIL-001\n    linked_requirements:\n      - REQ-001\n",
    )
    .expect("policy");
    fs::write(
        root.join("docs/syu/requirements/core.yaml"),
        "category: Core Requirements\nprefix: REQ\n\nrequirements:\n  - id: REQ-001\n    title: Planned requirements can exist before delivery starts\n    description: Planned items should stay trace-free until implementation begins.\n    priority: high\n    status: planned\n    linked_policies:\n      - POL-001\n    linked_features:\n      - FEAT-001\n    tests: {}\n",
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
        "category: Core Features\nversion: 1\n\nfeatures:\n  - id: FEAT-001\n    title: Planned features can stay undocumented by traces\n    summary: Delivery claims should not appear before the work ships.\n    status: planned\n    linked_requirements:\n      - REQ-001\n    implementations: {}\n",
    )
    .expect("feature");
}

fn write_orphan_workspace(root: &Path) {
    fs::create_dir_all(root.join("docs/syu/philosophy")).expect("philosophy dir");
    fs::create_dir_all(root.join("docs/syu/policies")).expect("policies dir");
    fs::create_dir_all(root.join("docs/syu/requirements")).expect("requirements dir");
    fs::create_dir_all(root.join("docs/syu/features")).expect("features dir");

    write_config(root, true, true, true, false);

    fs::write(
        root.join("docs/syu/philosophy/foundation.yaml"),
        "category: Philosophy\nversion: 1\nlanguage: en\n\nphilosophies:\n  - id: PHIL-001\n    title: Keep the main graph connected\n    product_design_principle: Real work should stay linked across the stack.\n    coding_guideline: Prefer deliberate adjacent-layer links.\n    linked_policies:\n      - POL-001\n  - id: PHIL-ORPHAN-001\n    title: Temporary brainstorm\n    product_design_principle: Early ideas can start disconnected.\n    coding_guideline: This item intentionally has no links yet.\n    linked_policies: []\n",
    )
    .expect("philosophy");
    fs::write(
        root.join("docs/syu/policies/policies.yaml"),
        "category: Policies\nversion: 1\nlanguage: en\n\npolicies:\n  - id: POL-001\n    title: Keep the main graph connected\n    summary: One connected chain keeps the fixture otherwise valid.\n    description: The orphan philosophy should be the only failure without overrides.\n    linked_philosophies:\n      - PHIL-001\n    linked_requirements:\n      - REQ-001\n",
    )
    .expect("policy");
    fs::write(
        root.join("docs/syu/requirements/core.yaml"),
        "category: Core Requirements\nprefix: REQ\n\nrequirements:\n  - id: REQ-001\n    title: Connected requirements stay discoverable\n    description: The main path remains valid so the orphan stands out.\n    priority: high\n    status: planned\n    linked_policies:\n      - POL-001\n    linked_features:\n      - FEAT-001\n    tests: {}\n",
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
        "category: Core Features\nversion: 1\n\nfeatures:\n  - id: FEAT-001\n    title: Connected feature stays discoverable\n    summary: This keeps the base fixture valid.\n    status: planned\n    linked_requirements:\n      - REQ-001\n    implementations: {}\n",
    )
    .expect("feature");
}

fn write_missing_backlink_workspace(root: &Path) {
    fs::create_dir_all(root.join("docs/syu/philosophy")).expect("philosophy dir");
    fs::create_dir_all(root.join("docs/syu/policies")).expect("policies dir");
    fs::create_dir_all(root.join("docs/syu/requirements")).expect("requirements dir");
    fs::create_dir_all(root.join("docs/syu/features")).expect("features dir");

    write_config(root, true, false, true, false);

    fs::write(
        root.join("docs/syu/philosophy/foundation.yaml"),
        "category: Philosophy\nversion: 1\nlanguage: en\n\nphilosophies:\n  - id: PHIL-001\n    title: Forward links should stay explicit\n    product_design_principle: One-way references should still be explainable during migration.\n    coding_guideline: Back-links can be phased in later.\n    linked_policies:\n      - POL-001\n",
    )
    .expect("philosophy");
    fs::write(
        root.join("docs/syu/policies/policies.yaml"),
        "category: Policies\nversion: 1\nlanguage: en\n\npolicies:\n  - id: POL-001\n    title: One-way links can appear during migration\n    summary: This fixture isolates the reciprocal-link rule.\n    description: The requirement links to the feature but the feature omits the backlink.\n    linked_philosophies:\n      - PHIL-001\n    linked_requirements:\n      - REQ-001\n",
    )
    .expect("policy");
    fs::write(
        root.join("docs/syu/requirements/core.yaml"),
        "category: Core Requirements\nprefix: REQ\n\nrequirements:\n  - id: REQ-001\n    title: Migrating graphs may start one-way\n    description: The requirement points to a real feature during migration.\n    priority: high\n    status: planned\n    linked_policies:\n      - POL-001\n    linked_features:\n      - FEAT-001\n    tests: {}\n",
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
        "category: Core Features\nversion: 1\n\nfeatures:\n  - id: FEAT-001\n    title: Migrating feature backlink\n    summary: This feature intentionally omits the reciprocal requirement link.\n    status: planned\n    linked_requirements: []\n    implementations: {}\n",
    )
    .expect("feature");
}

fn write_coverage_workspace(root: &Path) {
    fs::create_dir_all(root.join("docs/syu/philosophy")).expect("philosophy dir");
    fs::create_dir_all(root.join("docs/syu/policies")).expect("policies dir");
    fs::create_dir_all(root.join("docs/syu/requirements")).expect("requirements dir");
    fs::create_dir_all(root.join("docs/syu/features")).expect("features dir");
    fs::create_dir_all(root.join("src")).expect("src dir");
    fs::create_dir_all(root.join("tests")).expect("tests dir");

    write_config(root, true, true, true, false);

    fs::write(
        root.join("docs/syu/philosophy/foundation.yaml"),
        "category: Philosophy\nversion: 1\nlanguage: en\n\nphilosophies:\n  - id: PHIL-001\n    title: Public APIs should stay owned\n    product_design_principle: Shipped symbols should remain attributable to real features.\n    coding_guideline: Keep requirement tests and feature implementations explicit.\n    linked_policies:\n      - POL-001\n",
    )
    .expect("philosophy");
    fs::write(
        root.join("docs/syu/policies/policies.yaml"),
        "category: Policies\nversion: 1\nlanguage: en\n\npolicies:\n  - id: POL-001\n    title: Coverage can be enabled gradually\n    summary: This fixture proves CLI overrides can tighten coverage on demand.\n    description: The repository declares one owned public symbol and one stray public symbol.\n    linked_philosophies:\n      - PHIL-001\n    linked_requirements:\n      - REQ-001\n",
    )
    .expect("policy");
    fs::write(
        root.join("docs/syu/requirements/core.yaml"),
        "category: Core Requirements\nprefix: REQ\n\nrequirements:\n  - id: REQ-001\n    title: Tests should stay owned by requirements\n    description: The requirement trace is valid so coverage is the only tightening.\n    priority: high\n    status: implemented\n    linked_policies:\n      - POL-001\n    linked_features:\n      - FEAT-001\n    tests:\n      rust:\n        - file: tests/coverage.rs\n          symbols:\n            - requirement_test\n",
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
        "category: Core Features\nversion: 1\n\nfeatures:\n  - id: FEAT-001\n    title: Public APIs should stay feature-owned\n    summary: One public symbol is intentionally left unclaimed.\n    status: implemented\n    linked_requirements:\n      - REQ-001\n    implementations:\n      rust:\n        - file: src/coverage.rs\n          symbols:\n            - owned_symbol\n",
    )
    .expect("feature");
    fs::write(
        root.join("src/coverage.rs"),
        "/// FEAT-001\npub fn owned_symbol() {}\n\npub fn stray_symbol() {}\n",
    )
    .expect("source");
    fs::write(
        root.join("tests/coverage.rs"),
        "// REQ-001\n#[test]\nfn requirement_test() {\n    assert!(true);\n}\n",
    )
    .expect("test");
}

#[test]
fn validate_cli_override_can_allow_planned_items() {
    let tempdir = tempdir().expect("tempdir should exist");
    write_planned_workspace(tempdir.path(), false);

    let baseline = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("validate")
        .arg(tempdir.path())
        .output()
        .expect("validate should run");
    assert!(
        !baseline.status.success(),
        "planned items should fail without override"
    );

    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("validate")
        .arg(tempdir.path())
        .arg("--allow-planned")
        .output()
        .expect("validate should run");

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn validate_cli_override_can_forbid_planned_items() {
    let tempdir = tempdir().expect("tempdir should exist");
    write_planned_workspace(tempdir.path(), true);

    let baseline = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("validate")
        .arg(tempdir.path())
        .output()
        .expect("validate should run");
    assert!(
        baseline.status.success(),
        "planned items should pass with config"
    );

    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("validate")
        .arg(tempdir.path())
        .arg("--allow-planned=false")
        .output()
        .expect("validate should run");

    assert!(
        !output.status.success(),
        "CLI override should tighten planned items"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("SYU-delivery-planned-001"));
}

#[test]
fn validate_cli_override_for_planned_items_mentions_cli_source() {
    let tempdir = tempdir().expect("tempdir should exist");
    write_planned_workspace(tempdir.path(), true);

    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("validate")
        .arg(tempdir.path())
        .arg("--allow-planned=false")
        .output()
        .expect("validate should run");

    assert!(
        !output.status.success(),
        "CLI override should tighten planned items"
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--allow-planned=false"));
    assert!(!stdout.contains("syu.yaml forbids planned items"));
    assert!(stdout.contains("rerun without `--allow-planned=false`"));
}

#[test]
fn validate_cli_override_can_disable_orphan_checks() {
    let tempdir = tempdir().expect("tempdir should exist");
    write_orphan_workspace(tempdir.path());

    let baseline = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("validate")
        .arg(tempdir.path())
        .output()
        .expect("validate should run");
    assert!(
        !baseline.status.success(),
        "orphaned items should fail without override"
    );

    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("validate")
        .arg(tempdir.path())
        .arg("--require-non-orphaned-items=false")
        .output()
        .expect("validate should run");

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn validate_cli_override_can_disable_reciprocal_checks() {
    let tempdir = tempdir().expect("tempdir should exist");
    write_missing_backlink_workspace(tempdir.path());

    let baseline = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("validate")
        .arg(tempdir.path())
        .output()
        .expect("validate should run");
    assert!(
        !baseline.status.success(),
        "missing backlinks should fail without override"
    );

    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("validate")
        .arg(tempdir.path())
        .arg("--require-reciprocal-links=false")
        .output()
        .expect("validate should run");

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn validate_cli_override_can_enable_symbol_trace_coverage() {
    let tempdir = tempdir().expect("tempdir should exist");
    write_coverage_workspace(tempdir.path());

    let baseline = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("validate")
        .arg(tempdir.path())
        .output()
        .expect("validate should run");
    assert!(
        baseline.status.success(),
        "coverage should stay off without override\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&baseline.stdout),
        String::from_utf8_lossy(&baseline.stderr)
    );

    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("validate")
        .arg(tempdir.path())
        .arg("--require-symbol-trace-coverage")
        .output()
        .expect("validate should run");

    assert!(
        !output.status.success(),
        "override should enable strict coverage"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("SYU-coverage-public-001"));
}
