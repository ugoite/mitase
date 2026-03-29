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
