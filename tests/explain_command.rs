use assert_cmd::cargo::CommandCargoExt;
use serde_json::Value;
use std::{path::PathBuf, process::Command};

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/workspaces")
        .join(name)
}

#[test]
fn explain_command_summarizes_a_requirement_chain_in_text() {
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .args([
            "explain",
            "REQ-TRACE-001",
            fixture_path("passing").to_str().expect("utf8 path"),
        ])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Selection: definition REQ-TRACE-001"));
    assert!(stdout.contains("Assessment: aligned"));
    assert!(stdout.contains("Connected chain:"));
    assert!(stdout.contains("requirement REQ-TRACE-001"));
    assert!(stdout.contains("feature FEAT-TRACE-001"));
    assert!(stdout.contains("Traces in scope:"));
    assert!(stdout.contains("src/rust_trace_tests.rs"));
    assert!(stdout.contains("src/rust_feature.rs"));
    assert!(stdout.contains("Obvious gaps:\n- none"));
}

#[test]
fn explain_command_supports_symbol_queries_in_json() {
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .args([
            "explain",
            "feature_trace_rust",
            fixture_path("passing").to_str().expect("utf8 path"),
            "--format",
            "json",
        ])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("json output");
    assert_eq!(json["selection"]["kind"], "symbol");
    assert_eq!(json["selection"]["query"], "feature_trace_rust");
    assert_eq!(json["assessment"], "aligned");
    assert_eq!(
        json["direct_matches"]["traces"][0]["owner_id"],
        "FEAT-TRACE-001"
    );
    assert_eq!(json["chain"]["requirements"][0]["id"], "REQ-TRACE-001");
    assert_eq!(json["chain"]["features"][0]["id"], "FEAT-TRACE-001");
}
