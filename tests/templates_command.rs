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
        "docs-first\ttemplate-and-example\texamples/docs-first\tStarter for documentation-heavy repos with markdown acceptance anchors, a shell trace, and a wildcard-owned YAML file."
    ));
    assert!(stdout.contains(
        "rust-only\ttemplate-and-example\texamples/rust-only\tStarter for Rust-first repos"
    ));
    assert!(stdout.contains(
        "python-only\ttemplate-and-example\texamples/python-only\tStarter for Python-first repos"
    ));
    assert!(
        stdout.contains(
            "go-only\ttemplate-and-example\texamples/go-only\tStarter for Go-first repos with Go-oriented IDs plus a minimal go.mod, source, and test files."
        )
    );
    assert!(
        stdout.contains(
            "java-only\ttemplate-and-example\texamples/java-only\tStarter for Java-first repos with Java-oriented IDs plus a minimal pom.xml, source, and test files."
        )
    );
    assert!(
        stdout.contains(
            "typescript-only\ttemplate-and-example\texamples/typescript-only\tStarter for TypeScript-first repos with TypeScript-oriented IDs plus a minimal package.json, source, and test files."
        )
    );
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
    assert_eq!(templates.len(), 8);
    assert_eq!(templates[0]["name"], "generic");
    assert_eq!(templates[0]["relationship"], "starter-only");
    assert_eq!(templates[1]["name"], "docs-first");
    assert_eq!(templates[1]["related_example"], "examples/docs-first");
    assert_eq!(templates[2]["name"], "rust-only");
    assert_eq!(templates[2]["related_example"], "examples/rust-only");
    assert_eq!(templates[4]["name"], "go-only");
    assert_eq!(templates[4]["related_example"], "examples/go-only");
    assert_eq!(templates[5]["name"], "java-only");
    assert_eq!(templates[5]["related_example"], "examples/java-only");
    assert_eq!(templates[6]["name"], "typescript-only");
    assert_eq!(templates[6]["related_example"], "examples/typescript-only");
    assert_eq!(templates[7]["name"], "polyglot");
}
