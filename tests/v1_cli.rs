use assert_cmd::Command;
use std::fs;
use tempfile::tempdir;

#[test]
fn current_workspace_validates_and_reports_closed_loop() {
    Command::cargo_bin("syu")
        .unwrap()
        .args(["validate", "workspace", "."])
        .assert()
        .success();
    let output = Command::cargo_bin("syu")
        .unwrap()
        .args(["readiness", "report", ".", "--format", "json"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["target"], "closed-loop");
}

#[test]
fn obsolete_config_shape_is_rejected() {
    let temp = tempdir().unwrap();
    fs::create_dir_all(temp.path().join("docs/syu")).unwrap();
    fs::write(
        temp.path().join("syu.yaml"),
        "schema: syu/config/v1\nversion: 1\nspec: { root: docs/syu }\n",
    )
    .unwrap();
    let output = Command::cargo_bin("syu")
        .unwrap()
        .args(["validate", "workspace"])
        .arg(temp.path())
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("obsolete pre-release"));
}

#[test]
fn workbench_projection_does_not_implicitly_plan() {
    let output = Command::cargo_bin("syu")
        .unwrap()
        .args([
            "workbench",
            "project",
            "--workspace",
            ".",
            "--format",
            "json",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let projection: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(projection["work"]["request"].is_null());
    assert!(projection["work"]["plan"].is_null());
    assert_eq!(projection["diagnostics"]["validation"]["state"], "not_run");
}
