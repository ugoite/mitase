use assert_cmd::cargo::CommandCargoExt;
use std::{fs, process::Command};
use tempfile::tempdir;

fn fixture_path(name: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/workspaces")
        .join(name)
}

// REQ-CORE-004
#[test]
fn binary_surfaces_report_io_errors() {
    let tempdir = tempdir().expect("tempdir should exist");
    let occupied = tempdir.path().join("occupied");
    fs::write(&occupied, "not a directory").expect("occupied file should exist");

    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("report")
        .arg(fixture_path("passing"))
        .arg("--output")
        .arg(occupied.join("report.md"))
        .output()
        .expect("command should run");

    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("error:"));
}
