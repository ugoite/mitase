use assert_cmd::Command;
use std::fs;
use tempfile::tempdir;

#[test]
fn validates_repository_and_plans_fixture() {
    Command::cargo_bin("syu")
        .unwrap()
        .args(["validate", "."])
        .assert()
        .success();
    let temp = tempdir().unwrap();
    let plan = temp.path().join("plan.yaml");
    Command::cargo_bin("syu")
        .unwrap()
        .args([
            "work",
            "plan",
            "--request",
            "fixtures/v1/valid-web-app/work.yaml",
            "--out",
        ])
        .arg(&plan)
        .args(["--workspace", "fixtures/v1/valid-web-app"])
        .assert()
        .success();
    let text = fs::read_to_string(&plan).unwrap();
    assert!(text.contains("schema: syu/work-plan/v1"));
    assert!(text.contains("Contract counterpart; readonly"));
    assert!(text.contains("PHIL-AUTH-001#principle.generic-failure"));
    assert!(text.contains("POL-AUTH-001#rule.generic-failure"));
    assert!(text.contains("command: cargo test invalid_credentials"));
    Command::cargo_bin("syu")
        .unwrap()
        .args(["validate", "fixtures/v1/valid-web-app", "--plan"])
        .arg(plan)
        .assert()
        .success();
}

#[test]
fn rejects_stale_target_snapshots() {
    let temp = tempdir().unwrap();
    let plan = temp.path().join("plan.yaml");
    Command::cargo_bin("syu")
        .unwrap()
        .args([
            "work",
            "plan",
            "--request",
            "fixtures/v1/valid-web-app/work.yaml",
            "--out",
        ])
        .arg(&plan)
        .args(["--workspace", "fixtures/v1/valid-web-app"])
        .assert()
        .success();
    let text = fs::read_to_string(&plan).unwrap();
    fs::write(
        &plan,
        text.replacen("content_hash: sha256:", "content_hash: sha256:stale", 1),
    )
    .unwrap();
    Command::cargo_bin("syu")
        .unwrap()
        .args(["validate", "fixtures/v1/valid-web-app", "--plan"])
        .arg(plan)
        .assert()
        .failure();
}

#[test]
fn rejects_tampered_path_selector_and_budget_snapshots() {
    let temp = tempdir().unwrap();
    let plan = temp.path().join("plan.yaml");
    Command::cargo_bin("syu")
        .unwrap()
        .args([
            "work",
            "plan",
            "--request",
            "fixtures/v1/valid-web-app/work.yaml",
            "--out",
        ])
        .arg(&plan)
        .args(["--workspace", "fixtures/v1/valid-web-app"])
        .assert()
        .success();
    let text = fs::read_to_string(&plan).unwrap();
    let tampered = text
        .replacen(
            "resolved_path: api/login.rs",
            "resolved_path: web/login.ts",
            1,
        )
        .replacen(
            "description: symbols login",
            "description: symbols forged",
            1,
        )
        .replacen("editable_files: 1", "editable_files: 99", 1);
    fs::write(&plan, tampered).unwrap();
    Command::cargo_bin("syu")
        .unwrap()
        .args(["validate", "fixtures/v1/valid-web-app", "--plan"])
        .arg(plan)
        .assert()
        .failure();
}

#[test]
fn deleted_command_is_not_an_alias() {
    Command::cargo_bin("syu")
        .unwrap()
        .args(["task", "check"])
        .assert()
        .failure();
}

#[test]
fn rejects_unknown_configuration_fields() {
    Command::cargo_bin("syu")
        .unwrap()
        .args(["validate", "fixtures/v1/invalid-unknown-field"])
        .assert()
        .failure();
}
