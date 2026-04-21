use assert_cmd::cargo::CommandCargoExt;
use serde_json::Value;
use std::{fs, path::Path};
use tempfile::tempdir;

fn write_file(path: &Path, contents: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("parent directories should exist");
    }
    fs::write(path, contents).expect("file should be written");
}

fn write_workspace(root: &Path) {
    write_file(
        &root.join("syu.yaml"),
        &format!(
            "version: {version}\nspec:\n  root: docs/syu\nvalidate:\n  default_fix: false\n  allow_planned: true\n  require_non_orphaned_items: true\n  require_reciprocal_links: true\n  require_symbol_trace_coverage: false\n",
            version = env!("CARGO_PKG_VERSION"),
        ),
    );
    write_file(
        &root.join("docs/syu/philosophy/foundation.yaml"),
        r#"category: Philosophy
version: 1
language: en

philosophies:
  - id: PHIL-001
    title: Keep the workflow explainable
    product_design_principle: The tool should stay explainable and low ceremony.
    coding_guideline: Prefer explicit code over magic.
    linked_policies:
      - POL-001
      - POL-002
      - POL-003
"#,
    );
    write_file(
        &root.join("docs/syu/policies/policies.yaml"),
        r#"category: Policies
version: 1
language: en

policies:
  - id: POL-001
    title: Prefer automatic checks
    summary: Automatic checks keep reviews explainable.
    description: Automatic enforcement should stay explainable.
    linked_philosophies:
      - PHIL-001
    linked_requirements:
      - REQ-001
      - REQ-002
  - id: POL-002
    title: Keep requirements concrete
    summary: Requirements should become features quickly.
    description: Downstream obligations should stay concrete.
    linked_philosophies:
      - PHIL-001
    linked_requirements:
      - REQ-003
  - id: POL-003
    title: Remove drift
    summary: Policy language should not drift.
    description: Keep the policy fresh.
    linked_philosophies:
      - PHIL-001
    linked_requirements: []
"#,
    );
    write_file(
        &root.join("docs/syu/requirements/core.yaml"),
        r#"category: Core Requirements
prefix: REQ

requirements:
  - id: REQ-001
    title: Automatic review summary output
    description: Automatic review summary output should stay concise and explainable for pull request review.
    priority: medium
    status: implemented
    linked_policies:
      - POL-001
    linked_features:
      - FEAT-001
    tests: {}
  - id: REQ-002
    title: Automatic review summary reporting
    description: Automatic review summary reporting should stay concise and explainable for pull request review.
    priority: medium
    status: implemented
    linked_policies:
      - POL-001
    linked_features:
      - FEAT-001
    tests: {}
  - id: REQ-003
    title: Support manual review routing
    description: Manual review routing should let maintainers override automated decisions.
    priority: medium
    status: implemented
    linked_policies:
      - POL-002
    linked_features:
      - FEAT-001
    tests: {}
"#,
    );
    write_file(
        &root.join("docs/syu/features/features.yaml"),
        r#"version: "1"
updated: "2026-04"

files:
  - kind: audit
    file: cli/audit.yaml
"#,
    );
    write_file(
        &root.join("docs/syu/features/cli/audit.yaml"),
        r#"category: Audit CLI
version: 1

features:
  - id: FEAT-001
    title: Manual review override
    summary: Manual review override lets maintainers bypass automatic routing when needed.
    status: implemented
    linked_requirements:
      - REQ-001
      - REQ-002
      - REQ-003
    implementations: {}
"#,
    );
}

fn write_browser_terminal_workspace(root: &Path) {
    write_file(
        &root.join("syu.yaml"),
        &format!(
            "version: {version}\nspec:\n  root: docs/syu\nvalidate:\n  default_fix: false\n  allow_planned: true\n  require_non_orphaned_items: true\n  require_reciprocal_links: true\n  require_symbol_trace_coverage: false\n",
            version = env!("CARGO_PKG_VERSION"),
        ),
    );
    write_file(
        &root.join("docs/syu/philosophy/foundation.yaml"),
        r#"category: Philosophy
version: 1
language: en

philosophies:
  - id: PHIL-001
    title: Keep navigation focused
    product_design_principle: Browser and terminal flows should both stay focused on the spec.
    coding_guideline: Keep navigation paths explicit.
    linked_policies:
      - POL-001
"#,
    );
    write_file(
        &root.join("docs/syu/policies/policies.yaml"),
        r#"category: Policies
version: 1
language: en

policies:
  - id: POL-001
    title: Keep terminal review paths concise
    summary: Terminal review commands should stay concise.
    description: Reviewers should be able to inspect traceability from the terminal without ceremony.
    linked_philosophies:
      - PHIL-001
    linked_requirements:
      - REQ-001
"#,
    );
    write_file(
        &root.join("docs/syu/requirements/core.yaml"),
        r#"category: Core Requirements
prefix: REQ

requirements:
  - id: REQ-001
    title: Support focused reviewer navigation
    description: Reviewer navigation should support both browser and terminal workflows without losing trace context.
    priority: medium
    status: implemented
    linked_policies:
      - POL-001
    linked_features:
      - FEAT-001
    tests: {}
"#,
    );
    write_file(
        &root.join("docs/syu/features/features.yaml"),
        r#"version: "1"
updated: "2026-04"

files:
  - kind: audit
    file: cli/audit.yaml
"#,
    );
    write_file(
        &root.join("docs/syu/features/cli/audit.yaml"),
        r#"category: Audit CLI
version: 1

features:
  - id: FEAT-001
    title: Browser reviewer explorer
    summary: Browser reviewer explorer helps maintainers follow traceability visually when needed.
    status: implemented
    linked_requirements:
      - REQ-001
    implementations: {}
"#,
    );
}

#[test]
// REQ-CORE-025
fn audit_command_reports_overlap_tension_and_orphaned_policies() {
    let tempdir = tempdir().expect("tempdir should exist");
    write_workspace(tempdir.path());

    let output = std::process::Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("audit")
        .arg(tempdir.path())
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("[overlap]"));
    assert!(stdout.contains("REQ-001 and REQ-002"));
    assert!(stdout.contains("[tension]"));
    assert!(stdout.contains("FEAT-001"));
    assert!(stdout.contains("[orphaned-policy]"));
    assert!(stdout.contains("POL-003"));
}

#[test]
// REQ-CORE-025
fn audit_command_supports_json_output() {
    let tempdir = tempdir().expect("tempdir should exist");
    write_workspace(tempdir.path());

    let output = std::process::Command::cargo_bin("syu")
        .expect("binary should build")
        .args(["audit", "--format", "json"])
        .arg(tempdir.path())
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("output should be valid json");
    assert!(
        json["summary"]["overlap_candidates"]
            .as_u64()
            .expect("overlap count should be numeric")
            >= 1
    );
    assert!(
        json["summary"]["tension_candidates"]
            .as_u64()
            .expect("tension count should be numeric")
            >= 1
    );
    assert_eq!(json["summary"]["orphaned_policies"], 1);
    assert!(
        json["findings"]
            .as_array()
            .expect("findings should be an array")
            .iter()
            .any(|finding| finding["kind"] == "overlap")
    );
}

#[test]
// REQ-CORE-025
fn audit_command_does_not_flag_browser_terminal_pairs_as_tension() {
    let tempdir = tempdir().expect("tempdir should exist");
    write_browser_terminal_workspace(tempdir.path());

    let output = std::process::Command::cargo_bin("syu")
        .expect("binary should build")
        .args(["audit", "--format", "json"])
        .arg(tempdir.path())
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("output should be valid json");
    assert_eq!(json["summary"]["tension_candidates"], 0);
}
