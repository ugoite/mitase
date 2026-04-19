// REQ-CORE-018
// REQ-CORE-019
// REQ-CORE-020
// REQ-CORE-021
// REQ-CORE-023
// REQ-CORE-024

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
    assert!(stdout.contains("syu templates"));
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
        "browse", "show", "search", "trace", "app", "validate", "check", "report", "add", "relate",
        "log",
    ] {
        let output = Command::cargo_bin("syu")
            .expect("binary should build")
            .args([command, "--help"])
            .output()
            .expect("help should render");

        assert!(output.status.success(), "{command} help should succeed");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains(
                "Workspace root or any child directory; syu walks upward to find syu.yaml and the configured spec tree"
            ),
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
fn log_help_mentions_kind_path_and_json_output() {
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .args(["log", "--help"])
        .output()
        .expect("help should render");

    assert!(output.status.success(), "log help should succeed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--kind"));
    assert!(stdout.contains("--path"));
    assert!(stdout.contains("--format"));
    assert!(stdout.contains("Philosophy, policy, requirement, or feature ID"));
    assert!(stdout.contains("syu log FEAT-CHECK-001 --kind implementation --path src/command"));
}

#[test]
fn relate_help_mentions_ids_paths_symbols_and_json_output() {
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .args(["relate", "--help"])
        .output()
        .expect("help should render");

    assert!(output.status.success(), "relate help should succeed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--format"));
    assert!(stdout.contains("syu relate REQ-CORE-018"));
    assert!(stdout.contains("syu relate src/command/search.rs"));
    assert!(stdout.contains("syu relate run_search_command"));
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
fn trace_help_mentions_symbol_lookup_and_json_output() {
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .args(["trace", "--help"])
        .output()
        .expect("help should render");

    assert!(output.status.success(), "trace help should succeed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--symbol"));
    assert!(stdout.contains("--format"));
    assert!(stdout.contains("syu trace src/rust_feature.rs --symbol feature_trace_rust"));
}

#[test]
fn list_help_mentions_spec_root_and_child_directory_examples() {
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .args(["list", "--help"])
        .output()
        .expect("help should render");

    assert!(output.status.success(), "list help should succeed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("syu list requirement docs/syu"));
    assert!(stdout.contains("syu list requirement docs/syu/features"));
    assert!(stdout.contains(
        "Pass the workspace root, the configured spec.root directory, or any child directory."
    ));
    assert!(stdout.contains(
        "syu walks upward until it finds syu.yaml, then resolves the configured spec.root from that workspace."
    ));
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
// REQ-CORE-024
fn validate_help_mentions_warning_exit_code_for_automation() {
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .args(["validate", "--help"])
        .output()
        .expect("help should render");

    assert!(output.status.success(), "validate help should succeed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--warning-exit-code"));
    assert!(stdout.contains("warnings but no errors"));
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
    assert!(stdout.contains("syu templates"));
    assert!(stdout.contains("--template"));
    assert!(stdout.contains("docs-first"));
    assert!(stdout.contains("rust-only"));
    assert!(stdout.contains("python-only"));
    assert!(stdout.contains("go-only"));
    assert!(stdout.contains("java-only"));
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
// REQ-CORE-009
fn templates_help_mentions_json_and_init_follow_up() {
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .args(["templates", "--help"])
        .output()
        .expect("help should render");

    assert!(output.status.success(), "templates help should succeed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--format"));
    assert!(stdout.contains("syu templates --format json"));
    assert!(stdout.contains("syu init --template"));
    assert!(stdout.contains("related checked-in examples"));
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
