use assert_cmd::cargo::CommandCargoExt;
use std::{fs, process::Command};
use tempfile::tempdir;

#[test]
// REQ-CORE-009
fn init_command_bootstraps_a_workspace_that_validate_accepts() {
    let tempdir = tempdir().expect("tempdir should exist");
    let workspace = tempdir.path().join("demo");

    let init = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("init")
        .arg(&workspace)
        .arg("--name")
        .arg("demo")
        .output()
        .expect("init should run");

    assert!(
        init.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&init.stdout),
        String::from_utf8_lossy(&init.stderr)
    );
    assert!(workspace.join("syu.yaml").exists());
    assert!(workspace.join("docs/syu/features/core/core.yaml").exists());

    let config = fs::read_to_string(workspace.join("syu.yaml")).expect("config should exist");
    let requirement = fs::read_to_string(workspace.join("docs/syu/requirements/core/core.yaml"))
        .expect("requirement should exist");
    let feature = fs::read_to_string(workspace.join("docs/syu/features/core/core.yaml"))
        .expect("feature should exist");

    assert!(config.contains(env!("CARGO_PKG_VERSION")));
    assert!(config.contains("require_reciprocal_links: true"));
    assert!(requirement.contains("status: planned"));
    assert!(feature.contains("status: planned"));

    let validate = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("validate")
        .arg(&workspace)
        .output()
        .expect("validate should run");

    assert!(
        validate.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&validate.stdout),
        String::from_utf8_lossy(&validate.stderr)
    );
    assert!(String::from_utf8_lossy(&validate.stdout).contains("syu validate passed"));
}

#[test]
// REQ-CORE-009
fn init_command_requires_force_when_generated_files_exist() {
    let tempdir = tempdir().expect("tempdir should exist");
    fs::write(
        tempdir.path().join("syu.yaml"),
        format!("version: {}\n", env!("CARGO_PKG_VERSION")),
    )
    .expect("config should exist");

    let init = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("init")
        .arg(tempdir.path())
        .output()
        .expect("init should run");

    assert!(!init.status.success(), "init should refuse overwrite");
    assert!(String::from_utf8_lossy(&init.stderr).contains("refusing to overwrite"));
}
