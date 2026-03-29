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
