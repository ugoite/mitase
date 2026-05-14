// REQ-CORE-001

use assert_cmd::cargo::CommandCargoExt;
use serde_json::Value;
use std::{fs, path::Path, process::Command};
use tempfile::tempdir;

fn write_workspace(root: &Path, requirement_id: &str) {
    fs::create_dir_all(root.join("docs/syu/philosophy")).expect("philosophy dir");
    fs::create_dir_all(root.join("docs/syu/policies")).expect("policies dir");
    fs::create_dir_all(root.join("docs/syu/requirements/core")).expect("requirements dir");
    fs::create_dir_all(root.join("docs/syu/features/core")).expect("features dir");

    fs::write(
        root.join("syu.yaml"),
        format!(
            "version: {version}\nspec:\n  root: docs/syu\nvalidate:\n  default_fix: false\n  allow_planned: true\n  require_non_orphaned_items: true\n  require_reciprocal_links: true\n  require_symbol_trace_coverage: false\n  historical_ids:\n    enabled: true\nruntimes:\n  python:\n    command: auto\n  node:\n    command: auto\n",
            version = env!("CARGO_PKG_VERSION"),
        ),
    )
    .expect("config");

    fs::write(
        root.join("docs/syu/philosophy/foundation.yaml"),
        "category: Philosophy\nversion: 1\nlanguage: en\n\nphilosophies:\n  - id: PHIL-001\n    title: Keep IDs stable\n    product_design_principle: Stable IDs should keep their meaning across revisions.\n    coding_guideline: Prefer fresh IDs over recycling old ones.\n    linked_policies:\n      - POL-001\n",
    )
    .expect("philosophy");
    fs::write(
        root.join("docs/syu/policies/policies.yaml"),
        format!(
            "category: Policies\nversion: 1\nlanguage: en\n\npolicies:\n  - id: POL-001\n    title: Keep identifiers stable\n    summary: IDs should stay readable and unambiguous.\n    description: Reusing a deleted ID makes historical intent harder to audit.\n    linked_philosophies:\n      - PHIL-001\n    linked_requirements:\n      - {requirement_id}\n",
        ),
    )
    .expect("policy");
    fs::write(
        root.join("docs/syu/requirements/core/req.yaml"),
        format!(
            "category: Core Requirements\nprefix: REQ\n\nrequirements:\n  - id: {requirement_id}\n    title: Keep the requirement ID stable\n    description: The current workspace should stay valid apart from historical ID reuse.\n    priority: high\n    status: planned\n    linked_policies:\n      - POL-001\n    linked_features:\n      - FEAT-001\n    tests: {{}}\n",
        ),
    )
    .expect("requirement");
    fs::write(
        root.join("docs/syu/features/features.yaml"),
        format!(
            "version: \"{}\"\nfiles:\n  - kind: core\n    file: core/feature.yaml\n",
            env!("CARGO_PKG_VERSION"),
        ),
    )
    .expect("feature registry");
    fs::write(
        root.join("docs/syu/features/core/feature.yaml"),
        format!(
            "category: Core Features\nversion: 1\n\nfeatures:\n  - id: FEAT-001\n    title: Keep the feature link stable\n    summary: The feature points back to the current requirement ID.\n    status: planned\n    linked_requirements:\n      - {requirement_id}\n    implementations: {{}}\n",
        ),
    )
    .expect("feature");
}

fn git(workspace: &Path, args: &[&str]) {
    let mut command = Command::new("git");
    command.arg("-C").arg(workspace).args(args);
    let output = command.output().expect("git should run");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "git {:?} failed\nstdout:\n{}\nstderr:\n{}",
        args,
        stdout,
        stderr
    );
}

fn git_stdout(workspace: &Path, args: &[&str]) -> String {
    let mut command = Command::new("git");
    command.arg("-C").arg(workspace).args(args);
    let output = command.output().expect("git should run");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "git {:?} failed\nstdout:\n{}\nstderr:\n{}",
        args,
        stdout,
        stderr
    );
    String::from_utf8(output.stdout)
        .expect("git output should be utf8")
        .trim()
        .to_string()
}

#[test]
fn validate_rejects_reintroduced_deleted_requirement_ids() {
    let tempdir = tempdir().expect("tempdir should exist");
    let workspace = tempdir.path();

    git(workspace, &["init"]);
    git(workspace, &["config", "user.name", "Test User"]);
    git(workspace, &["config", "user.email", "test@example.com"]);

    write_workspace(workspace, "REQ-001");
    git(workspace, &["add", "."]);
    git(workspace, &["commit", "-m", "initial requirement id"]);

    write_workspace(workspace, "REQ-002");
    git(workspace, &["add", "."]);
    git(workspace, &["commit", "-m", "replace requirement id"]);

    let historical_commit = git_stdout(workspace, &["rev-parse", "HEAD~1"]);

    write_workspace(workspace, "REQ-001");

    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("validate")
        .arg(workspace)
        .arg("--format")
        .arg("json")
        .output()
        .expect("validate should run");

    assert!(
        !output.status.success(),
        "historical ID reuse should fail validation"
    );

    let json: Value = serde_json::from_slice(&output.stdout).expect("output should be JSON");
    let issues = json["issues"]
        .as_array()
        .expect("issues should be represented as an array");
    let historical_issue = issues
        .iter()
        .find(|issue| issue["code"] == "SYU-workspace-historical-001")
        .expect("historical ID reuse should be reported");

    let message = historical_issue["message"]
        .as_str()
        .expect("message should be a string");
    assert!(message.contains("docs/syu/requirements/core/req.yaml"));
    assert!(message.contains(&historical_commit));
    assert!(
        historical_issue["suggestion"]
            .as_str()
            .is_some_and(|suggestion| suggestion.contains("validate.historical_ids.enabled"))
    );
}
