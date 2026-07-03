use assert_cmd::Command;
use std::fs;
use std::process::Command as ProcessCommand;
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
    assert!(text.contains("kind: command"));
    assert!(text.contains("program: cargo"));
    assert!(text.contains("- invalid_credentials"));
    Command::cargo_bin("syu")
        .unwrap()
        .args(["validate", "fixtures/v1/valid-web-app", "--plan"])
        .arg(plan)
        .args(["--range", "HEAD..HEAD"])
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
fn export_context_rejects_tampered_and_blocked_plans() {
    let temp = tempdir().unwrap();
    let plan = temp.path().join("plan.yaml");
    let pack = temp.path().join("context.yaml");
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
    let slice = text
        .lines()
        .find_map(|line| line.strip_prefix("- id: "))
        .unwrap()
        .to_string();

    fs::write(&plan, text.replacen("status: ready", "status: blocked", 1)).unwrap();
    Command::cargo_bin("syu")
        .unwrap()
        .args([
            "work",
            "export-context",
            "--plan",
            plan.to_str().unwrap(),
            "--slice",
            &slice,
            "--workspace",
            "fixtures/v1/valid-web-app",
            "--out",
            pack.to_str().unwrap(),
        ])
        .assert()
        .failure();
}

#[test]
fn export_context_uses_canonical_slice_and_rich_artifact_metadata() {
    let temp = tempdir().unwrap();
    let plan = temp.path().join("plan.yaml");
    let pack = temp.path().join("context.yaml");
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
    let slice = text
        .lines()
        .find_map(|line| line.strip_prefix("- id: "))
        .unwrap()
        .to_string();

    Command::cargo_bin("syu")
        .unwrap()
        .args([
            "work",
            "export-context",
            "--plan",
            plan.to_str().unwrap(),
            "--slice",
            &slice,
            "--workspace",
            "fixtures/v1/valid-web-app",
            "--out",
            pack.to_str().unwrap(),
        ])
        .assert()
        .success();
    let context = fs::read_to_string(pack).unwrap();
    assert!(context.contains("access: editable"));
    assert!(context.contains("selector:"));
    assert!(context.contains("content_hash: sha256:"));
    assert!(context.contains("excerpt_hash: sha256:"));
    assert!(context.contains("kind: http"));
}

#[test]
fn exact_requested_targets_are_exact_and_verification_can_be_editable() {
    let temp = tempdir().unwrap();
    let request = temp.path().join("request.yaml");
    let plan = temp.path().join("plan.yaml");
    fs::write(
        &request,
        concat!(
            "schema: syu/work-request/v1\n",
            "id: WORK-EXACT-VERIFY\n",
            "summary: Update the regression test.\n",
            "operation: modify\n",
            "seeds: []\n",
            "requested_targets:\n",
            "  - REQ-AUTH-001#binding.login-test/target.case\n",
            "constraints:\n",
            "  include_facets: [verification]\n",
            "  exclude_paths: []\n",
            "  max_slices: 3\n",
        ),
    )
    .unwrap();
    Command::cargo_bin("syu")
        .unwrap()
        .args(["work", "plan", "--request"])
        .arg(&request)
        .args(["--out"])
        .arg(&plan)
        .args(["--workspace", "fixtures/v1/valid-web-app"])
        .assert()
        .success();
    let text = fs::read_to_string(plan).unwrap();
    assert!(text.contains("id: invalid-credentials-verify-case"));
    assert!(text.contains("resolved_path: tests/login.rs"));
    assert!(text.contains("access: editable"));
    assert!(!text.contains("id: invalid-credentials-ui"));
    assert!(!text.contains("id: invalid-credentials-backend\n  goal"));
}

#[test]
fn validate_uses_configured_baseline_without_explicit_range() {
    let status = ProcessCommand::new("git")
        .args(["update-ref", "refs/remotes/origin/main", "HEAD"])
        .status()
        .unwrap();
    assert!(status.success(), "git update-ref failed");
    Command::cargo_bin("syu")
        .unwrap()
        .args(["validate", "fixtures/v1/valid-web-app"])
        .assert()
        .success();
}

#[test]
fn export_context_rejects_stale_revision() {
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
    let slice = text
        .lines()
        .find_map(|line| line.strip_prefix("- id: "))
        .unwrap()
        .to_string();
    fs::write(&plan, text.replacen("revision: ", "revision: stale-", 1)).unwrap();
    Command::cargo_bin("syu")
        .unwrap()
        .args([
            "work",
            "export-context",
            "--plan",
            plan.to_str().unwrap(),
            "--slice",
            &slice,
            "--workspace",
            "fixtures/v1/valid-web-app",
        ])
        .assert()
        .failure();
}

#[test]
fn normal_validate_does_not_require_git_repository() {
    let temp = tempdir().unwrap();
    fs::create_dir_all(temp.path().join("spec")).unwrap();
    fs::create_dir_all(temp.path().join("src")).unwrap();
    fs::write(
        temp.path().join("syu.yaml"),
        concat!(
            "schema: syu/config/v1\n",
            "workspace:\n",
            "  spec_roots: [spec]\n",
            "  artifact_roots: [src]\n",
            "  excludes: []\n",
            "profiles: { active: [], custom: {} }\n",
            "validation:\n",
            "  preset: standard\n",
            "  deny_warnings: false\n",
            "  rules: {}\n",
            "  changed:\n",
            "    require_owned_changes: false\n",
            "work:\n",
            "  slicing:\n",
            "    max_editable_files: 1\n",
            "    max_editable_symbols: 1\n",
            "    max_verification_targets: 1\n",
            "    max_readonly_targets: 1\n",
            "    max_total_bytes: 1024\n",
            "  context:\n",
            "    include_parent_principles: false\n",
            "    include_parent_rules: false\n",
            "adapters: { enabled: [rust] }\n",
        ),
    )
    .unwrap();
    fs::write(
        temp.path().join("spec/requirement.yaml"),
        concat!(
            "schema: syu/spec/v1\n",
            "kind: requirements\n",
            "namespace: sample\n",
            "category: Sample\n",
            "requirements:\n",
            "  - id: REQ-SAMPLE-001\n",
            "    title: Sample\n",
            "    description: Sample requirement.\n",
            "    priority: medium\n",
            "    status: planned\n",
            "    criteria:\n",
            "      - id: check\n",
            "        kind: behavior\n",
            "        statement: Sample criterion.\n",
            "        governed_by: []\n",
        ),
    )
    .unwrap();
    Command::cargo_bin("syu")
        .unwrap()
        .args(["validate"])
        .arg(temp.path())
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
