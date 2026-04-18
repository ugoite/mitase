use assert_cmd::cargo::CommandCargoExt;
use serde_json::Value;
use std::{fs, path::PathBuf, process::Command};
use tempfile::tempdir;

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/workspaces")
        .join(name)
}

#[test]
// REQ-CORE-018
fn list_command_lists_philosophies_in_text_format() {
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("list")
        .arg("philosophy")
        .arg(fixture_path("passing"))
        .output()
        .expect("command should run");

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("PHIL-TRACE-001\tTrace everything"));
}

#[test]
// REQ-CORE-018
fn list_command_supports_json_output() {
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("list")
        .arg("feature")
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

    let json: Value = serde_json::from_slice(&output.stdout).expect("output should be valid JSON");
    assert_eq!(json["kind"], "feature");
    assert_eq!(
        json["items"]
            .as_array()
            .expect("items should be an array")
            .len(),
        4
    );
    assert_eq!(json["items"][0]["id"], "FEAT-TRACE-001");
}

#[test]
// REQ-CORE-018
fn list_command_with_path_appends_document_paths_in_text_output() {
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("list")
        .arg("requirement")
        .arg(fixture_path("passing"))
        .arg("--with-path")
        .output()
        .expect("command should run");

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains(
        "REQ-TRACE-001\tRust requirement trace\tdocs/syu/requirements/traceability/core.yaml"
    ));
}

#[test]
// REQ-CORE-018
fn list_command_with_path_includes_document_paths_in_json_output() {
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("list")
        .arg("feature")
        .arg(fixture_path("passing"))
        .arg("--with-path")
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

    let json: Value = serde_json::from_slice(&output.stdout).expect("output should be valid JSON");
    assert_eq!(json["items"][0]["id"], "FEAT-TRACE-001");
    assert_eq!(
        json["items"][0]["document_path"],
        "docs/syu/features/traceability/core.yaml"
    );
}

#[test]
// REQ-CORE-018
fn list_command_all_kinds_with_path_supports_json_output() {
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("list")
        .arg(fixture_path("passing"))
        .arg("--with-path")
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

    let json: Value = serde_json::from_slice(&output.stdout).expect("output should be valid JSON");
    assert_eq!(
        json["philosophy"][0]["document_path"],
        "docs/syu/philosophy/foundation.yaml"
    );
    assert_eq!(
        json["policy"][0]["document_path"],
        "docs/syu/policies/policies.yaml"
    );
    assert_eq!(
        json["requirement"][0]["document_path"],
        "docs/syu/requirements/traceability/core.yaml"
    );
    assert_eq!(
        json["feature"][0]["document_path"],
        "docs/syu/features/traceability/core.yaml"
    );
}

#[test]
// REQ-CORE-018
fn list_command_accepts_plural_lookup_aliases() {
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("list")
        .arg("requirements")
        .arg(fixture_path("passing"))
        .output()
        .expect("command should run");

    assert!(output.status.success(), "plural alias should be accepted");
    assert!(String::from_utf8_lossy(&output.stdout).contains("REQ-TRACE-003"));
}

#[test]
// REQ-CORE-018
fn list_command_accepts_workspace_before_kind() {
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("list")
        .arg(fixture_path("passing"))
        .arg("requirements")
        .output()
        .expect("command should run");

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("REQ-TRACE-003"));
}

#[test]
// REQ-CORE-018
fn list_command_workspace_first_invalid_kind_preserves_workspace_hint() {
    let workspace = fixture_path("passing");
    let workspace_display = workspace.display().to_string();
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("list")
        .arg(&workspace)
        .arg("requirment")
        .output()
        .expect("command should run");

    assert!(!output.status.success(), "invalid kind should fail");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("invalid value 'requirment'"),
        "stderr:\n{stderr}"
    );
    assert!(
        stderr.contains(&format!("syu list {workspace_display}")),
        "stderr should preserve the workspace path in the recovery hint:\n{stderr}",
    );
}

#[test]
// REQ-CORE-018
fn list_command_help_documents_both_argument_orders() {
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("list")
        .arg("--help")
        .output()
        .expect("command should run");

    assert!(output.status.success(), "help should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("syu list requirement path/to/workspace"),
        "help should show the kind-first example:\n{stdout}",
    );
    assert!(
        stdout.contains("syu list path/to/workspace requirement"),
        "help should show the workspace-first example:\n{stdout}",
    );
    assert!(
        stdout.contains(
            "Pass the workspace root, the configured spec.root directory, or any child directory."
        ),
        "help should explain the workspace discovery behavior:\n{stdout}",
    );
    assert!(
        stdout.contains(
            "syu walks upward until it finds syu.yaml, then resolves the configured spec.root from that workspace."
        ),
        "help should explain how spec.root is resolved from the discovered workspace:\n{stdout}",
    );
    assert!(
        stdout.contains("syu list requirement docs/syu"),
        "help should include a spec.root example:\n{stdout}",
    );
    assert!(
        stdout.contains("syu list requirement docs/syu/features"),
        "help should include a child-directory-under-spec-root example:\n{stdout}",
    );
    assert!(
        stdout.contains("emitted as JSON for automation"),
        "help should explain the automation-oriented list output:\n{stdout}",
    );
    assert!(
        stdout.contains("workspace metadata, per-layer counts, and the current validation errors"),
        "help should distinguish list output from the browse snapshot:\n{stdout}",
    );
}

#[test]
// REQ-CORE-018
fn list_command_rejects_two_kind_positionals() {
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("list")
        .arg("requirement")
        .arg("feature")
        .output()
        .expect("command should run");

    assert!(!output.status.success(), "two kinds should fail");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("received two layer kinds"),
        "stderr should explain the positional ambiguity:\n{stderr}",
    );
    assert!(
        stderr.contains("syu list requirement ."),
        "stderr should show a direct usage example:\n{stderr}",
    );
}

#[test]
// REQ-CORE-018
fn list_command_without_kind_lists_all_kinds() {
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("list")
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
    assert!(
        stdout.contains("=== philosophy ("),
        "should show philosophy section"
    );
    assert!(
        stdout.contains("=== policy ("),
        "should show policy section"
    );
    assert!(
        stdout.contains("=== requirement ("),
        "should show requirement section"
    );
    assert!(
        stdout.contains("=== feature ("),
        "should show feature section"
    );
    assert!(
        stdout.contains("PHIL-TRACE-001"),
        "should include philosophies"
    );
    assert!(stdout.contains("FEAT-TRACE-001"), "should include features");
}

#[test]
// REQ-CORE-018
fn list_command_dot_as_workspace_lists_all_kinds() {
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("list")
        .arg(fixture_path("passing"))
        .output()
        .expect("command should run");

    assert!(
        output.status.success(),
        "syu list <path> should list all kinds: stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("PHIL-TRACE-001"),
        "should list philosophies"
    );
    assert!(stdout.contains("FEAT-TRACE-001"), "should list features");
}

#[test]
// REQ-CORE-018
fn list_command_all_kinds_supports_json_output() {
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("list")
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

    let json: Value = serde_json::from_slice(&output.stdout).expect("output should be valid JSON");
    assert!(
        json["philosophy"].is_array(),
        "JSON should have philosophy array"
    );
    assert!(json["policy"].is_array(), "JSON should have policy array");
    assert!(
        json["requirement"].is_array(),
        "JSON should have requirement array"
    );
    assert!(json["feature"].is_array(), "JSON should have feature array");
    assert_eq!(json["philosophy"][0]["id"], "PHIL-TRACE-001");
}

#[test]
// REQ-CORE-018
fn list_command_rejects_kind_typos_before_treating_them_as_paths() {
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("list")
        .arg("philsophy")
        .output()
        .expect("command should run");

    assert!(!output.status.success(), "typoed kinds should fail");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("did you mean `philosophy`"),
        "stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("syu list --help"),
        "stderr should point users back to the list help:\n{stderr}",
    );
    assert!(
        !stderr.contains("failed to resolve workspace root"),
        "typoed kinds should not be reported as workspace path failures:\n{stderr}",
    );
}

#[test]
// REQ-CORE-018
fn list_command_with_path_uses_absolute_document_paths_for_external_spec_roots() {
    let tempdir = tempdir().expect("tempdir should exist");
    let workspace_root = tempdir.path().join("workspace");
    let spec_root = tempdir.path().join("external-spec");
    fs::create_dir_all(&workspace_root).expect("workspace dir");
    fs::create_dir_all(spec_root.join("philosophy")).expect("philosophy dir");
    fs::create_dir_all(spec_root.join("policies")).expect("policies dir");
    fs::create_dir_all(spec_root.join("requirements/core")).expect("requirements dir");
    fs::create_dir_all(spec_root.join("features/core")).expect("features dir");

    fs::write(
        workspace_root.join("syu.yaml"),
        format!(
            "version: {version}\nspec:\n  root: {spec_root}\nvalidate:\n  default_fix: false\n  allow_planned: true\n  require_non_orphaned_items: true\n  require_reciprocal_links: true\n  require_symbol_trace_coverage: false\napp:\n  bind: 127.0.0.1\n  port: 3000\nruntimes:\n  python:\n    command: auto\n  node:\n    command: auto\n",
            version = env!("CARGO_PKG_VERSION"),
            spec_root = spec_root.display(),
        ),
    )
    .expect("config should write");
    fs::write(
        spec_root.join("philosophy/foundation.yaml"),
        "category: Philosophy\nversion: 1\nlanguage: en\n\nphilosophies:\n  - id: PHIL-EXT-001\n    title: External spec root\n    product_design_principle: Keep specs outside the workspace root when needed.\n    coding_guideline: Resolve the configured path explicitly.\n    linked_policies:\n      - POL-EXT-001\n",
    )
    .expect("philosophy should write");
    fs::write(
        spec_root.join("policies/policies.yaml"),
        "category: Policies\nversion: 1\nlanguage: en\n\npolicies:\n  - id: POL-EXT-001\n    title: Keep links reciprocal\n    summary: Policies still point back to philosophy.\n    description: External roots should behave like in-repo roots.\n    linked_philosophies:\n      - PHIL-EXT-001\n    linked_requirements:\n      - REQ-EXT-001\n",
    )
    .expect("policy should write");
    fs::write(
        spec_root.join("requirements/core/core.yaml"),
        "category: Core Requirements\nprefix: REQ-EXT\n\nrequirements:\n  - id: REQ-EXT-001\n    title: External requirement\n    description: Requirement lives outside the workspace root.\n    priority: high\n    status: planned\n    linked_policies:\n      - POL-EXT-001\n    linked_features:\n      - FEAT-EXT-001\n    tests: {}\n",
    )
    .expect("requirement should write");
    fs::write(
        spec_root.join("features/features.yaml"),
        format!(
            "version: \"{}\"\nupdated: \"generated by test\"\n\nfiles:\n  - kind: core\n    file: core/core.yaml\n",
            env!("CARGO_PKG_VERSION")
        ),
    )
    .expect("registry should write");
    fs::write(
        spec_root.join("features/core/core.yaml"),
        "category: Core Features\nversion: 1\n\nfeatures:\n  - id: FEAT-EXT-001\n    title: External feature\n    summary: Feature lives outside the workspace root.\n    status: planned\n    linked_requirements:\n      - REQ-EXT-001\n    implementations: {}\n",
    )
    .expect("feature should write");

    let expected_path = spec_root.join("philosophy/foundation.yaml");
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("list")
        .arg("philosophy")
        .arg(&workspace_root)
        .arg("--with-path")
        .output()
        .expect("command should run");

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains(&expected_path.display().to_string()));
}

#[test]
// REQ-CORE-018
fn list_command_keeps_kind_typos_helpful_when_workspace_is_explicit() {
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("list")
        .arg("philsophy")
        .arg(".")
        .output()
        .expect("command should run");

    assert!(!output.status.success(), "typoed kinds should fail");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("did you mean `philosophy`"),
        "stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("syu list ."),
        "stderr should keep the workspace-specific recovery hint:\n{stderr}",
    );
    assert!(
        !stderr.contains("failed to resolve workspace root"),
        "typoed kinds should not be reported as workspace path failures:\n{stderr}",
    );
}
