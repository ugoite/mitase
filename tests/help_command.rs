// REQ-CORE-019
// REQ-CORE-020

use assert_cmd::cargo::CommandCargoExt;
use std::process::Command;

#[test]
// REQ-CORE-010
fn root_help_includes_start_here_guidance() {
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("--help")
        .output()
        .expect("help should render");

    assert!(output.status.success(), "help should succeed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("New here?"));
    assert!(stdout.contains("syu init ."));
    assert!(stdout.contains("syu validate ."));
    assert!(stdout.contains("syu browse ."));
    assert!(stdout.contains("syu app ."));
    assert!(stdout.contains(
        "Browse the specification in your terminal (interactive prompts or text output)"
    ));
    assert!(stdout.contains("Start a local HTTP server and browser UI for workspace exploration"));
}

#[test]
fn workspace_help_uses_current_directory_default_consistently() {
    for command in [
        "browse", "show", "search", "app", "validate", "check", "report", "add",
    ] {
        let output = Command::cargo_bin("syu")
            .expect("binary should build")
            .args([command, "--help"])
            .output()
            .expect("help should render");

        assert!(output.status.success(), "{command} help should succeed");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("Workspace root containing syu.yaml and the configured spec tree"),
            "{command} help should describe the workspace root consistently",
        );
        assert!(
            stdout.contains("[default: .]"),
            "{command} help should keep the current-directory default",
        );
        assert!(
            !stdout.contains("default: docs/syu"),
            "{command} help should not claim docs/syu is the workspace default",
        );
    }
}

#[test]
fn search_help_mentions_kind_scoping_and_json_output() {
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .args(["search", "--help"])
        .output()
        .expect("help should render");

    assert!(output.status.success(), "search help should succeed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--kind"));
    assert!(stdout.contains("--format"));
    assert!(stdout.contains("syu search traceability --kind requirement"));
}

#[test]
fn add_help_mentions_explicit_file_and_feature_kind() {
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .args(["add", "--help"])
        .output()
        .expect("help should render");

    assert!(output.status.success(), "add help should succeed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--interactive"));
    assert!(stdout.contains("--file"));
    assert!(stdout.contains("--kind"));
    assert!(
        stdout.contains("prints the reciprocal-link follow-up and matching scaffold suggestions")
    );
    assert!(stdout.contains("syu add requirement --interactive"));
    assert!(stdout.contains("FEAT-AUTH-LOGIN-001 --kind auth"));
}

#[test]
fn init_help_lists_starter_templates() {
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .args(["init", "--help"])
        .output()
        .expect("help should render");

    assert!(output.status.success(), "init help should succeed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--template"));
    assert!(stdout.contains("rust-only"));
    assert!(stdout.contains("python-only"));
    assert!(stdout.contains("polyglot"));
}

#[test]
fn init_help_mentions_custom_spec_roots() {
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .args(["init", "--help"])
        .output()
        .expect("help should render");

    assert!(output.status.success(), "init help should succeed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--spec-root"));
    assert!(stdout.contains("docs/spec"));
    assert!(stdout.contains("spec/contracts"));
}

#[test]
fn init_help_mentions_id_prefix_options() {
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .args(["init", "--help"])
        .output()
        .expect("help should render");

    assert!(output.status.success(), "init help should succeed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--id-prefix"));
    assert!(stdout.contains("--requirement-prefix"));
    assert!(stdout.contains("PHIL-<stem>"));
    assert!(stdout.contains("REQ-<stem>"));
}

#[test]
fn validate_help_lists_temporary_config_overrides() {
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .args(["validate", "--help"])
        .output()
        .expect("help should render");

    assert!(output.status.success(), "validate help should succeed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--allow-planned"));
    assert!(stdout.contains("--require-non-orphaned-items"));
    assert!(stdout.contains("--require-reciprocal-links"));
    assert!(stdout.contains("--require-symbol-trace-coverage"));
}
