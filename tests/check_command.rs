use assert_cmd::cargo::CommandCargoExt;
use std::{fs, path::Path, path::PathBuf, process::Command};
use tempfile::tempdir;

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/workspaces")
        .join(name)
}

fn write_trace_path_workspace(root: &Path, trace_path: &str) {
    fs::create_dir_all(root.join("docs/syu/philosophy")).expect("philosophy dir");
    fs::create_dir_all(root.join("docs/syu/policies")).expect("policies dir");
    fs::create_dir_all(root.join("docs/syu/requirements")).expect("requirements dir");
    fs::create_dir_all(root.join("docs/syu/features")).expect("features dir");
    fs::create_dir_all(root.join("src")).expect("src dir");

    fs::write(
        root.join("syu.yaml"),
        format!(
            "version: {version}\nspec:\n  root: docs/syu\nvalidate:\n  default_fix: false\n  allow_planned: true\nruntimes:\n  python:\n    command: auto\n  node:\n    command: auto\n",
            version = env!("CARGO_PKG_VERSION"),
        ),
    )
    .expect("config");

    fs::write(
        root.join("docs/syu/philosophy/foundation.yaml"),
        "category: Philosophy\nversion: 1\nlanguage: en\n\nphilosophies:\n  - id: PHIL-001\n    title: Clear repository paths\n    product_design_principle: Keep trace paths portable.\n    coding_guideline: Prefer canonical repository-relative file paths.\n    linked_policies:\n      - POL-001\n",
    )
    .expect("philosophy");

    fs::write(
        root.join("docs/syu/policies/policies.yaml"),
        "category: Policies\nversion: 1\nlanguage: en\n\npolicies:\n  - id: POL-001\n    title: Keep traces portable\n    summary: Trace file paths should stay reviewable across environments.\n    description: Checked-in trace paths should use one portable repository-relative form.\n    linked_philosophies:\n      - PHIL-001\n    linked_requirements:\n      - REQ-001\n",
    )
    .expect("policy");

    fs::write(
        root.join("docs/syu/requirements/core.yaml"),
        format!(
            "category: Core Requirements\nprefix: REQ\n\nrequirements:\n  - id: REQ-001\n    title: Verify canonical trace paths\n    description: Requirement traces should stay portable across reviewers and CI.\n    priority: high\n    status: implemented\n    linked_policies:\n      - POL-001\n    linked_features:\n      - FEAT-001\n    tests:\n      rust:\n        - file: {trace_path}\n          symbols:\n            - trace_symbol\n",
        ),
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
        "category: Core Features\nversion: 1\n\nfeatures:\n  - id: FEAT-001\n    title: Verify canonical trace paths\n    summary: Feature implementations should keep portable trace paths.\n    status: implemented\n    linked_requirements:\n      - REQ-001\n    implementations:\n      rust:\n        - file: src/trace.rs\n          symbols:\n            - trace_symbol\n",
    )
    .expect("feature");

    fs::write(
        root.join("src/trace.rs"),
        "// REQ-001\n// FEAT-001\npub fn trace_symbol() {}\n",
    )
    .expect("trace source");
}

#[test]
// REQ-CORE-001
fn check_command_accepts_passing_workspace() {
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("validate")
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
    assert!(stdout.contains("syu validate passed"));
    assert!(stdout.contains("checks:"));
    assert!(stdout.contains("workspace items"));
    assert!(stdout.contains("(workspace, graph, delivery, trace)"));
    assert!(stdout.contains("validate.require_symbol_trace_coverage=false"));
    assert!(stdout.contains(
        "traceability: requirements=3/3 traces validated; features=3/3 traces validated"
    ));
    assert!(
        stdout.contains("What to do next:"),
        "success output should include next-step guidance: {stdout}"
    );
    assert!(
        stdout.contains("syu app ."),
        "next-step block should mention syu app: {stdout}"
    );
}

#[test]
// REQ-CORE-001
fn check_command_quiet_suppresses_next_step_guidance() {
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("validate")
        .arg(fixture_path("passing"))
        .arg("--quiet")
        .output()
        .expect("command should run");

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("syu validate passed"));
    assert!(stdout.contains("checks:"));
    assert!(
        !stdout.contains("What to do next:"),
        "quiet mode should suppress next-step guidance: {stdout}"
    );
}

#[test]
// REQ-CORE-001
fn check_command_reports_missing_definition_links() {
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("validate")
        .arg(fixture_path("failing"))
        .output()
        .expect("command should run");

    assert!(!output.status.success(), "failing fixture should fail");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("SYU-graph-reference-001"));
    assert!(stdout.contains("referenced rules:"));
    assert!(stdout.contains("Linked definitions must exist"));
    assert!(stdout.contains("REQ-MISSING-999"));
}

#[test]
// REQ-CORE-001
fn check_command_filters_visible_issues_by_spec_id() {
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("validate")
        .arg(fixture_path("failing"))
        .arg("--id")
        .arg("REQ-FAIL-001")
        .output()
        .expect("command should run");

    assert!(
        !output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("syu validate failed (filtered view)"));
    assert!(stdout.contains("filters: id=REQ-FAIL-001"));
    assert!(stdout.contains("showing 2 of 5 issues after filtering"));
    assert!(stdout.contains("requirement REQ-FAIL-001"));
    assert!(!stdout.contains("feature FEAT-FAIL-001"));
    assert!(!stdout.contains("policy POL-FAIL-001"));
}

#[test]
fn check_command_suggests_init_for_uninitialized_workspaces() {
    let tempdir = tempdir().expect("tempdir should exist");
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("validate")
        .arg(tempdir.path())
        .output()
        .expect("command should run");

    assert!(!output.status.success(), "empty workspace should fail");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("SYU-workspace-load-001"));
    assert!(stdout.contains("suggestion:"));
    assert!(stdout.contains("syu init ."));
}

#[test]
// REQ-CORE-002
fn check_command_verifies_requirement_test_traceability_in_all_supported_languages() {
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("validate")
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

    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("output should be valid JSON");
    assert_eq!(json["trace_summary"]["requirement_traces"]["declared"], 3);
    assert_eq!(json["trace_summary"]["requirement_traces"]["validated"], 3);
}

#[test]
// REQ-CORE-002
fn check_command_verifies_feature_implementation_traceability_in_all_supported_languages() {
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("validate")
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

    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("output should be valid JSON");
    assert_eq!(json["trace_summary"]["feature_traces"]["declared"], 3);
    assert_eq!(json["trace_summary"]["feature_traces"]["validated"], 3);
}

#[test]
fn check_alias_still_invokes_validate() {
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("check")
        .arg(fixture_path("passing"))
        .output()
        .expect("command should run");

    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("syu validate passed"));
}

#[test]
// REQ-CORE-001
fn check_command_reports_disabled_validation_toggles_from_config() {
    let tempdir = tempdir().expect("tempdir should exist");
    write_trace_path_workspace(tempdir.path(), "src/trace.rs");
    fs::write(
        tempdir.path().join("syu.yaml"),
        format!(
            "version: {version}\nspec:\n  root: docs/syu\nvalidate:\n  default_fix: false\n  allow_planned: true\n  require_non_orphaned_items: false\n  require_reciprocal_links: false\n  require_symbol_trace_coverage: false\nruntimes:\n  python:\n    command: auto\n  node:\n    command: auto\n",
            version = env!("CARGO_PKG_VERSION"),
        ),
    )
    .expect("config");

    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("validate")
        .arg(tempdir.path())
        .output()
        .expect("command should run");

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("validate.require_non_orphaned_items=false"));
    assert!(stdout.contains("validate.require_reciprocal_links=false"));
    assert!(stdout.contains("validate.require_symbol_trace_coverage=false"));
}

#[test]
// REQ-CORE-002
fn check_command_warns_for_non_canonical_relative_trace_paths() {
    let tempdir = tempdir().expect("tempdir should exist");
    write_trace_path_workspace(tempdir.path(), "./src/../src/trace.rs");

    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("validate")
        .arg(tempdir.path())
        .output()
        .expect("command should run");

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("SYU-trace-file-003"));
    assert!(stdout.contains("src/trace.rs"));
}

#[test]
// REQ-CORE-002
fn check_command_warns_for_backslash_trace_paths() {
    let tempdir = tempdir().expect("tempdir should exist");
    write_trace_path_workspace(tempdir.path(), "src\\\\trace.rs");

    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("validate")
        .arg(tempdir.path())
        .output()
        .expect("command should run");

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("SYU-trace-file-003"));
    assert!(stdout.contains("src/trace.rs"));
}
