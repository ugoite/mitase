use assert_cmd::cargo::CommandCargoExt;
use std::{fs, path::Path, process::Command};
use tempfile::tempdir;

fn init_workspace(workspace: &Path, extra_args: &[&str]) {
    let mut command = Command::cargo_bin("syu").expect("binary should build");
    command.arg("init").arg(workspace);
    command.args(extra_args);
    let output = command.output().expect("init should run");
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
// REQ-CORE-020
fn add_command_appends_philosophy_to_the_default_file() {
    let tempdir = tempdir().expect("tempdir should exist");
    let workspace = tempdir.path().join("workspace");
    init_workspace(&workspace, &[]);

    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .args(["add", "philosophy", "PHIL-002"])
        .arg(&workspace)
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let file = fs::read_to_string(workspace.join("docs/syu/philosophy/foundation.yaml"))
        .expect("philosophy file should exist");
    assert!(file.contains("- id: PHIL-002"));
    assert!(file.contains("title: New philosophy"));
    assert!(file.contains("linked_policies: []"));
}

#[test]
// REQ-CORE-020
fn add_command_creates_requirement_files_from_the_id_prefix() {
    let tempdir = tempdir().expect("tempdir should exist");
    let workspace = tempdir.path().join("workspace");
    init_workspace(&workspace, &[]);

    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .args(["add", "requirement", "REQ-AUTH-001"])
        .arg(&workspace)
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let file = workspace.join("docs/syu/requirements/auth/auth.yaml");
    let contents = fs::read_to_string(&file).expect("requirement file should exist");
    assert!(contents.contains("prefix: REQ-AUTH"));
    assert!(contents.contains("id: REQ-AUTH-001"));
    assert!(contents.contains("status: planned"));
}

#[test]
// REQ-CORE-020
fn add_command_updates_the_feature_registry_for_new_feature_files() {
    let tempdir = tempdir().expect("tempdir should exist");
    let workspace = tempdir.path().join("workspace");
    init_workspace(&workspace, &[]);

    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .args(["add", "feature", "FEAT-AUTH-LOGIN-001", "--kind", "auth"])
        .arg(&workspace)
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let feature_file = workspace.join("docs/syu/features/auth/login.yaml");
    let registry = fs::read_to_string(workspace.join("docs/syu/features/features.yaml"))
        .expect("feature registry should exist");
    assert!(feature_file.exists(), "feature file should be created");
    assert!(registry.contains("kind: auth"));
    assert!(registry.contains("file: auth/login.yaml"));
}

#[test]
// REQ-CORE-020
fn add_command_defaults_feature_kind_from_the_feature_id_prefix() {
    let tempdir = tempdir().expect("tempdir should exist");
    let workspace = tempdir.path().join("workspace");
    init_workspace(&workspace, &[]);

    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .args(["add", "feature", "FEAT-AUTH-LOGIN-001"])
        .arg(&workspace)
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let feature_file = workspace.join("docs/syu/features/auth/login.yaml");
    let registry = fs::read_to_string(workspace.join("docs/syu/features/features.yaml"))
        .expect("feature registry should exist");
    assert!(feature_file.exists(), "feature file should be created");
    assert!(registry.contains("kind: auth"));
    assert!(registry.contains("file: auth/login.yaml"));
}

#[test]
// REQ-CORE-020
fn add_command_reuses_existing_feature_registry_entries_for_shared_files() {
    let tempdir = tempdir().expect("tempdir should exist");
    let workspace = tempdir.path().join("workspace");
    init_workspace(&workspace, &[]);

    let first = Command::cargo_bin("syu")
        .expect("binary should build")
        .args([
            "add",
            "feature",
            "FEAT-AUTH-LOGIN-001",
            "--kind",
            "auth",
            "--file",
            "features/auth/shared.yaml",
        ])
        .arg(&workspace)
        .output()
        .expect("first command should run");
    assert!(first.status.success());

    let second = Command::cargo_bin("syu")
        .expect("binary should build")
        .args([
            "add",
            "feature",
            "FEAT-AUTH-RESET-001",
            "--kind",
            "auth",
            "--file",
            "features/auth/shared.yaml",
        ])
        .arg(&workspace)
        .output()
        .expect("second command should run");
    assert!(second.status.success());

    let feature_file = fs::read_to_string(workspace.join("docs/syu/features/auth/shared.yaml"))
        .expect("shared feature file should exist");
    let registry = fs::read_to_string(workspace.join("docs/syu/features/features.yaml"))
        .expect("feature registry should exist");
    assert!(feature_file.contains("id: FEAT-AUTH-LOGIN-001"));
    assert!(feature_file.contains("id: FEAT-AUTH-RESET-001"));
    assert_eq!(registry.matches("file: auth/shared.yaml").count(), 1);
}

#[test]
// REQ-CORE-020
fn add_command_does_not_mutate_feature_files_when_registry_kind_conflicts() {
    let tempdir = tempdir().expect("tempdir should exist");
    let workspace = tempdir.path().join("workspace");
    init_workspace(&workspace, &[]);

    let feature_path = workspace.join("docs/syu/features/core/core.yaml");
    let before = fs::read_to_string(&feature_path).expect("core feature file should exist");

    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .args([
            "add",
            "feature",
            "FEAT-AUTH-NEW-001",
            "--kind",
            "auth",
            "--file",
            "features/core/core.yaml",
        ])
        .arg(&workspace)
        .output()
        .expect("command should run");

    assert_eq!(output.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("feature registry already tracks `core/core.yaml` under kind `core`")
    );
    assert_eq!(
        fs::read_to_string(&feature_path).expect("core feature file should remain readable"),
        before
    );
}

#[test]
// REQ-CORE-020
fn add_command_rejects_feature_registry_paths_as_targets() {
    let tempdir = tempdir().expect("tempdir should exist");
    let workspace = tempdir.path().join("workspace");
    init_workspace(&workspace, &[]);

    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .args([
            "add",
            "feature",
            "FEAT-AUTH-001",
            "--kind",
            "auth",
            "--file",
            "features/features.yaml",
        ])
        .arg(&workspace)
        .output()
        .expect("command should run");

    assert_eq!(output.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("feature stubs must use a feature document path")
    );
}

#[test]
// REQ-CORE-020
fn add_command_honors_explicit_files_in_custom_spec_roots() {
    let tempdir = tempdir().expect("tempdir should exist");
    let workspace = tempdir.path().join("workspace");
    init_workspace(&workspace, &["--spec-root", "docs/spec"]);

    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .args([
            "add",
            "feature",
            "FEAT-AUTH-001",
            "--kind",
            "auth",
            "--file",
            "docs/spec/features/auth/flows.yaml",
        ])
        .arg(&workspace)
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let feature_file = workspace.join("docs/spec/features/auth/flows.yaml");
    let registry = fs::read_to_string(workspace.join("docs/spec/features/features.yaml"))
        .expect("feature registry should exist");
    assert!(
        feature_file.exists(),
        "explicit feature file should be created"
    );
    assert!(registry.contains("file: auth/flows.yaml"));
}

#[test]
// REQ-CORE-020
fn add_command_rejects_duplicate_ids() {
    let tempdir = tempdir().expect("tempdir should exist");
    let workspace = tempdir.path().join("workspace");
    init_workspace(&workspace, &[]);

    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .args(["add", "requirement", "REQ-001"])
        .arg(&workspace)
        .output()
        .expect("command should run");

    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("already exists"));
}

#[test]
// REQ-CORE-020
fn add_command_rejects_feature_kind_for_non_feature_layers() {
    let tempdir = tempdir().expect("tempdir should exist");
    let workspace = tempdir.path().join("workspace");
    init_workspace(&workspace, &[]);

    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .args(["add", "requirement", "REQ-AUTH-001", "--kind", "auth"])
        .arg(&workspace)
        .output()
        .expect("command should run");

    assert_eq!(output.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("only supported when scaffolding features")
    );
}
