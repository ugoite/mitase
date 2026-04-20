use assert_cmd::cargo::CommandCargoExt;
use std::{path::PathBuf, process::Command};

// REQ-CORE-002
// FEAT-CONTRIB-001
fn example_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples")
        .join(name)
}

#[test]
// REQ-CORE-012
fn docs_first_example_validates() {
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("validate")
        .arg(example_path("docs-first"))
        .output()
        .expect("validate should run");

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("definitions: philosophies=1 policies=1 requirements=2 features=2"));
    assert!(stdout.contains(
        "traceability: requirements=2/2 traces validated; features=2/2 traces validated"
    ));
}

#[test]
// REQ-CORE-012
fn csharp_fallback_example_validates() {
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("validate")
        .arg(example_path("csharp-fallback"))
        .output()
        .expect("validate should run");

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("definitions: philosophies=1 policies=1 requirements=1 features=1"));
    assert!(stdout.contains(
        "traceability: requirements=1/1 traces validated; features=1/1 traces validated"
    ));
}

#[test]
// REQ-CORE-012
fn rust_only_example_validates() {
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("validate")
        .arg(example_path("rust-only"))
        .output()
        .expect("validate should run");

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
// REQ-CORE-012
fn python_only_example_validates() {
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("validate")
        .arg(example_path("python-only"))
        .output()
        .expect("validate should run");

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
// REQ-CORE-012
fn ruby_only_example_validates() {
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("validate")
        .arg(example_path("ruby-only"))
        .output()
        .expect("validate should run");

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains(
        "traceability: requirements=1/1 traces validated; features=1/1 traces validated"
    ));
}

#[test]
// REQ-CORE-012
fn go_only_example_validates() {
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("validate")
        .arg(example_path("go-only"))
        .output()
        .expect("validate should run");

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains(
        "traceability: requirements=1/1 traces validated; features=1/1 traces validated"
    ));
}

#[test]
// REQ-CORE-012
fn java_only_example_validates() {
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("validate")
        .arg(example_path("java-only"))
        .output()
        .expect("validate should run");

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains(
        "traceability: requirements=1/1 traces validated; features=1/1 traces validated"
    ));
}

#[test]
// REQ-CORE-012
fn typescript_only_example_validates() {
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("validate")
        .arg(example_path("typescript-only"))
        .output()
        .expect("validate should run");

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains(
        "traceability: requirements=1/1 traces validated; features=1/1 traces validated"
    ));
}

#[test]
// REQ-CORE-012
fn polyglot_example_validates() {
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("validate")
        .arg(example_path("polyglot"))
        .output()
        .expect("validate should run");

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains(
        "traceability: requirements=3/3 traces validated; features=3/3 traces validated"
    ));
}

#[test]
// REQ-CORE-012
fn team_scale_example_validates() {
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("validate")
        .arg(example_path("team-scale"))
        .output()
        .expect("validate should run");

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("definitions: philosophies=1 policies=2 requirements=3 features=4"));
    assert!(stdout.contains(
        "traceability: requirements=3/3 traces validated; features=4/4 traces validated"
    ));
}

#[test]
// REQ-CORE-012
fn browser_ui_example_validates() {
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("validate")
        .arg(example_path("browser-ui"))
        .output()
        .expect("validate should run");

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("definitions: philosophies=1 policies=1 requirements=1 features=1"));
    assert!(stdout.contains(
        "traceability: requirements=1/1 traces validated; features=1/1 traces validated"
    ));
}
