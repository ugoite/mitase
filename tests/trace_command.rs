// FEAT-TRACE-001
// REQ-CORE-021

use assert_cmd::cargo::CommandCargoExt;
use serde_json::Value;
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};
use tempfile::tempdir;

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/workspaces")
        .join(name)
}

fn copy_dir_recursive(source: &Path, destination: &Path) {
    fs::create_dir_all(destination).expect("destination dir");
    for entry in fs::read_dir(source).expect("source dir") {
        let entry = entry.expect("dir entry");
        let entry_type = entry.file_type().expect("entry type");
        let destination_path = destination.join(entry.file_name());
        if entry_type.is_dir() {
            copy_dir_recursive(&entry.path(), &destination_path);
        } else {
            fs::copy(entry.path(), destination_path).expect("file copy");
        }
    }
}

#[test]
fn trace_command_resolves_feature_owners_from_file_only_lookup() {
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .args([
            "trace",
            "src/rust_feature.rs",
            fixture_path("passing")
                .to_str()
                .expect("fixture path should be valid utf-8"),
        ])
        .output()
        .expect("trace command should run");

    assert!(output.status.success(), "trace lookup should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("File: src/rust_feature.rs"));
    assert!(stdout.contains("Status: owned"));
    assert!(stdout.contains("feature FEAT-TRACE-001"));
    assert!(stdout.contains("REQ-TRACE-001"));
    assert!(stdout.contains("POL-TRACE-001"));
    assert!(stdout.contains("PHIL-TRACE-001"));
}

#[test]
fn trace_command_supports_symbol_lookups_in_json() {
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .args([
            "trace",
            "src/rust_trace_tests.rs",
            fixture_path("passing")
                .to_str()
                .expect("fixture path should be valid utf-8"),
            "--symbol",
            "req_trace_rust_test",
            "--format",
            "json",
        ])
        .output()
        .expect("trace command should run");

    assert!(output.status.success(), "trace lookup should succeed");
    let json: Value =
        serde_json::from_slice(&output.stdout).expect("trace command should print valid JSON");
    assert_eq!(json["status"], "owned");
    assert_eq!(json["matched_owners"][0]["kind"], "requirement");
    assert_eq!(json["matched_owners"][0]["id"], "REQ-TRACE-001");
    assert_eq!(
        json["matched_owners"][0]["matched_symbol"],
        "req_trace_rust_test"
    );
    assert_eq!(json["requirements"][0]["id"], "REQ-TRACE-001");
    assert_eq!(json["features"][0]["id"], "FEAT-TRACE-001");
}

#[test]
fn trace_command_reports_partially_traced_symbols() {
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .args([
            "trace",
            "src/rust_feature.rs",
            fixture_path("passing")
                .to_str()
                .expect("fixture path should be valid utf-8"),
            "--symbol",
            "missing_symbol",
        ])
        .output()
        .expect("trace command should run");

    assert!(
        output.status.success(),
        "partial trace lookups should still succeed"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Status: partial"));
    assert!(stdout.contains("No trace owners matched symbol `missing_symbol`."));
    assert!(stdout.contains("File owners without a matching symbol:"));
    assert!(stdout.contains("feature FEAT-TRACE-001"));
}

#[test]
fn trace_command_reports_unowned_files_with_next_steps() {
    let tempdir = tempdir().expect("tempdir should exist");
    let workspace = tempdir.path().join("workspace");
    copy_dir_recursive(&fixture_path("passing"), &workspace);
    fs::write(workspace.join("src/unowned.rs"), "pub fn unowned() {}\n")
        .expect("unowned source should exist");

    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .args([
            "trace",
            "src/unowned.rs",
            workspace
                .to_str()
                .expect("workspace path should be valid utf-8"),
        ])
        .output()
        .expect("trace command should run");

    assert!(
        output.status.success(),
        "unowned trace lookups should still succeed"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Status: unowned"));
    assert!(stdout.contains("No requirement or feature traces reference `src/unowned.rs`."));
    assert!(stdout.contains("syu validate . --genre trace"));
}
