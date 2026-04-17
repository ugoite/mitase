use assert_cmd::cargo::CommandCargoExt;
use serde_json::Value;
use std::process::Command;

#[test]
// REQ-CORE-009
fn templates_command_lists_all_supported_templates_in_text_output() {
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("templates")
        .output()
        .expect("command should run");

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.starts_with("name\trelationship\trelated_example\tdescription\n"));
    assert!(stdout.contains(
        "generic\tstarter-only\t-\tStarter with minimal four-layer files, neutral IDs, and core file names."
    ));
    assert!(stdout.contains(
        "rust-only\ttemplate-and-example\texamples/rust-only\tStarter for Rust-first repos"
    ));
    assert!(stdout.contains(
        "python-only\ttemplate-and-example\texamples/python-only\tStarter for Python-first repos"
    ));
    assert!(stdout.contains(
        "polyglot\ttemplate-and-example\texamples/polyglot\tStarter for mixed-language repos"
    ));
}

#[test]
// REQ-CORE-009
fn templates_command_supports_json_output() {
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .args(["templates", "--format", "json"])
        .output()
        .expect("command should run");

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let json: Value = serde_json::from_slice(&output.stdout).expect("output should be valid JSON");
    let templates = json["templates"]
        .as_array()
        .expect("templates should be an array");
    assert_eq!(templates.len(), 4);
    assert_eq!(templates[0]["name"], "generic");
    assert_eq!(templates[0]["relationship"], "starter-only");
    assert_eq!(templates[1]["name"], "rust-only");
    assert_eq!(templates[1]["related_example"], "examples/rust-only");
    assert_eq!(templates[3]["name"], "polyglot");
}
