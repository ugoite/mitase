use assert_cmd::cargo::CommandCargoExt;
use serde_yaml::Value;
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
    let parsed_config: Value = serde_yaml::from_str(&config).expect("config should be valid yaml");
    let requirement = fs::read_to_string(workspace.join("docs/syu/requirements/core/core.yaml"))
        .expect("requirement should exist");
    let feature = fs::read_to_string(workspace.join("docs/syu/features/core/core.yaml"))
        .expect("feature should exist");

    assert!(config.contains(env!("CARGO_PKG_VERSION")));
    assert_eq!(parsed_config["app"]["bind"].as_str(), Some("127.0.0.1"));
    assert_eq!(parsed_config["app"]["port"].as_u64(), Some(3000));
    assert_eq!(
        parsed_config["validate"]["require_reciprocal_links"].as_bool(),
        Some(true)
    );
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
fn init_command_bootstraps_a_custom_spec_root_that_validate_accepts() {
    let tempdir = tempdir().expect("tempdir should exist");
    let workspace = tempdir.path().join("demo");

    let init = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("init")
        .arg(&workspace)
        .arg("--name")
        .arg("demo")
        .arg("--spec-root")
        .arg("spec/contracts")
        .output()
        .expect("init should run");

    assert!(
        init.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&init.stdout),
        String::from_utf8_lossy(&init.stderr)
    );
    assert!(workspace.join("syu.yaml").exists());
    assert!(
        workspace
            .join("spec/contracts/features/core/core.yaml")
            .exists()
    );

    let config = fs::read_to_string(workspace.join("syu.yaml")).expect("config should exist");
    let parsed_config: Value = serde_yaml::from_str(&config).expect("config should be valid yaml");
    assert_eq!(
        parsed_config["spec"]["root"].as_str(),
        Some("spec/contracts")
    );

    let stdout = String::from_utf8_lossy(&init.stdout);
    assert!(stdout.contains(&format!("{}/", workspace.join("spec/contracts").display())));
    assert!(stdout.contains("spec/contracts/philosophy/foundation.yaml"));

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

#[test]
// REQ-CORE-009
fn init_command_prints_workspace_aware_next_steps_for_explicit_paths() {
    let tempdir = tempdir().expect("tempdir should exist");
    let workspace = tempdir.path().join("demo workspace");

    let init = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("init")
        .arg(&workspace)
        .output()
        .expect("init should run");

    assert!(
        init.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&init.stdout),
        String::from_utf8_lossy(&init.stderr)
    );

    let stdout = String::from_utf8_lossy(&init.stdout);
    let workspace_arg = format!("'{}'", workspace.display());
    assert!(stdout.contains(&format!("Run `syu validate {workspace_arg}`")));
    assert!(stdout.contains(&format!("Run `syu browse {workspace_arg}`")));
    assert!(stdout.contains(&format!("`syu app {workspace_arg}`")));
    assert!(stdout.contains(&format!("{}/", workspace.join("docs/syu").display())));
}

#[test]
// REQ-CORE-009
fn init_command_rejects_spec_roots_outside_the_workspace() {
    let tempdir = tempdir().expect("tempdir should exist");
    let workspace = tempdir.path().join("demo");

    let init = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("init")
        .arg(&workspace)
        .arg("--spec-root")
        .arg("../spec")
        .output()
        .expect("init should run");

    assert!(
        !init.status.success(),
        "init should reject invalid spec roots"
    );
    assert!(String::from_utf8_lossy(&init.stderr).contains("--spec-root"));
}
