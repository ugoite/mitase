use assert_cmd::cargo::CommandCargoExt;
#[cfg(unix)]
use std::{
    env,
    io::Write,
    process::{Output, Stdio},
};
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

#[cfg(unix)]
fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

#[cfg(unix)]
fn run_interactive_add(current_dir: &Path, args: &[&str], input: &str) -> Output {
    let binary = env::var("CARGO_BIN_EXE_syu").expect("cargo should expose the syu binary path");
    let command = std::iter::once(binary.as_str())
        .chain(args.iter().copied())
        .map(shell_quote)
        .collect::<Vec<_>>()
        .join(" ");
    let mut child = Command::new("script")
        .current_dir(current_dir)
        .arg("-qec")
        .arg(command)
        .arg("/dev/null")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("interactive add should start");

    child
        .stdin
        .as_mut()
        .expect("stdin should be piped")
        .write_all(input.as_bytes())
        .expect("interactive input should be written");

    child
        .wait_with_output()
        .expect("interactive add should finish")
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

#[test]
// REQ-CORE-020
fn add_command_requires_an_explicit_id_when_not_attached_to_a_terminal() {
    let tempdir = tempdir().expect("tempdir should exist");
    let workspace = tempdir.path().join("workspace");
    init_workspace(&workspace, &[]);

    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .current_dir(&workspace)
        .args(["add", "requirement"])
        .output()
        .expect("command should run");

    assert_eq!(output.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("needs a definition ID when stdin/stdout are not terminals")
    );
}

#[cfg(unix)]
#[test]
// REQ-CORE-020
fn add_command_prompts_for_a_missing_id_in_the_current_workspace() {
    let tempdir = tempdir().expect("tempdir should exist");
    let workspace = tempdir.path().join("workspace");
    init_workspace(&workspace, &[]);

    let output = run_interactive_add(&workspace, &["add", "requirement"], "REQ-AUTH-LOGIN-001\n");

    assert!(
        output.status.success(),
        "stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("Definition ID:"));

    let file = workspace.join("docs/syu/requirements/auth/login.yaml");
    let contents = fs::read_to_string(&file).expect("interactive requirement file should exist");
    assert!(contents.contains("id: REQ-AUTH-LOGIN-001"));
}

#[cfg(unix)]
#[test]
// REQ-CORE-020
fn add_command_interactive_mode_retries_invalid_prompts_and_accepts_defaults() {
    let tempdir = tempdir().expect("tempdir should exist");
    let workspace = tempdir.path().join("workspace");
    init_workspace(&workspace, &[]);

    let output = run_interactive_add(
        &workspace,
        &["add", "feature", "--interactive"],
        "\nnot-an-id\nFEAT-AUTH-LOGIN-001\nAuth\n\n\n",
    );

    assert!(
        output.status.success(),
        "stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let transcript = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(stdout.contains("Definition ID:"));
    assert!(stdout.contains("Feature kind [auth]:"));
    assert!(stdout.contains("YAML file [features/auth/login.yaml]:"));
    assert!(transcript.contains("Definition ID is required."));
    assert!(transcript.contains("feature IDs must start with `FEAT-`"));
    assert!(transcript.contains("feature `--kind` must contain only lowercase ASCII letters"));

    let feature_file = workspace.join("docs/syu/features/auth/login.yaml");
    let registry = fs::read_to_string(workspace.join("docs/syu/features/features.yaml"))
        .expect("feature registry should exist");
    assert!(
        feature_file.exists(),
        "interactive feature file should use the default path"
    );
    assert!(registry.contains("file: auth/login.yaml"));
}

#[cfg(unix)]
#[test]
// REQ-CORE-020
fn add_command_interactive_mode_accepts_a_workspace_path_before_prompting() {
    let tempdir = tempdir().expect("tempdir should exist");
    let workspace = tempdir.path().join("workspace");
    init_workspace(&workspace, &[]);

    let output = run_interactive_add(
        tempdir.path(),
        &[
            "add",
            "feature",
            workspace.to_str().expect("workspace path should be utf-8"),
            "--interactive",
        ],
        "FEAT-AUTH-LOGIN-001\nauth\nfeatures/auth/flows.yaml\n",
    );

    assert!(
        output.status.success(),
        "stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Definition ID:"));
    assert!(stdout.contains("Feature kind [auth]:"));
    assert!(stdout.contains("YAML file [features/auth/login.yaml]:"));

    let feature_file = workspace.join("docs/syu/features/auth/flows.yaml");
    let registry = fs::read_to_string(workspace.join("docs/syu/features/features.yaml"))
        .expect("feature registry should exist");
    assert!(
        feature_file.exists(),
        "interactive feature file should be created"
    );
    assert!(registry.contains("file: auth/flows.yaml"));
}
