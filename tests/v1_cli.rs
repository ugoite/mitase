use assert_cmd::Command;
use std::fs;
use std::path::Path;
use std::process::Command as ProcessCommand;
use syu_work_model::{WorkPlan, work_plan_digest};
use tempfile::tempdir;

fn git(dir: &Path, args: &[&str]) {
    let status = ProcessCommand::new("git")
        .args(args)
        .current_dir(dir)
        .status()
        .unwrap();
    assert!(status.success(), "git {:?} failed", args);
}

fn init_workspace_repo(root: &Path) {
    git(root, &["init"]);
    git(root, &["config", "user.name", "Codex"]);
    git(root, &["config", "user.email", "codex@example.com"]);
    git(root, &["add", "."]);
    git(root, &["commit", "-m", "init"]);
}

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
fn rejects_rehashed_editable_target_tampering_against_basis_revision() {
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
    let mut work_plan: WorkPlan =
        serde_yaml::from_str(&fs::read_to_string(&plan).unwrap()).unwrap();
    let target = &mut work_plan.slices[0].editable_targets[0];
    target.content_hash = "sha256:forged".into();
    target.excerpt_hash = "sha256:forged".into();
    work_plan.canonical_digest = work_plan_digest(&work_plan);
    fs::write(&plan, serde_yaml::to_string(&work_plan).unwrap()).unwrap();
    let output = Command::cargo_bin("syu")
        .unwrap()
        .args(["validate", "fixtures/v1/valid-web-app", "--plan"])
        .arg(&plan)
        .args(["--range", "HEAD..HEAD"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("plan structure does not match the canonical planner output"));
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
            "  - ref: REQ-AUTH-001#binding.login-test/target.case\n",
            "    transition: modify\n",
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
    let text = fs::read_to_string(&plan).unwrap();
    assert!(text.contains("criterion-REQ-AUTH-001#binding.login-test/target.case"));
    assert!(text.contains("resolved_path: tests/login.rs"));
    assert!(text.contains("access: editable"));
    assert!(!text.contains("id: invalid-credentials-ui"));
    assert!(!text.contains("id: invalid-credentials-backend\n  goal"));
}

#[test]
fn work_plan_rejects_mixed_seeds_and_requested_targets() {
    let temp = tempdir().unwrap();
    let request = temp.path().join("request.yaml");
    let plan = temp.path().join("plan.yaml");
    fs::write(
        &request,
        concat!(
            "schema: syu/work-request/v1\n",
            "id: WORK-MIXED-SELECTION\n",
            "summary: Mixed work selection.\n",
            "operation: modify\n",
            "seeds: [REQ-AUTH-001#criterion.invalid-credentials]\n",
            "requested_targets:\n",
            "  - ref: REQ-AUTH-001#binding.login-test/target.case\n",
            "    transition: modify\n",
            "constraints:\n",
            "  include_facets: []\n",
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
        .failure();
    let text = fs::read_to_string(&plan).unwrap();
    assert!(text.contains("status: blocked"));
    assert!(text.contains("request cannot combine seeds and requested targets"));
}

#[test]
fn exclude_paths_skip_unresolvable_targets_before_resolution() {
    let temp = tempdir().unwrap();
    fs::create_dir_all(temp.path().join("spec")).unwrap();
    fs::create_dir_all(temp.path().join("src")).unwrap();
    fs::create_dir_all(temp.path().join("tests")).unwrap();
    fs::write(
        temp.path().join("syu.yaml"),
        concat!(
            "schema: syu/config/v1\n",
            "workspace:\n",
            "  spec_roots: [spec]\n",
            "  artifact_roots: [src, tests]\n",
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
            "    max_editable_files: 2\n",
            "    max_editable_symbols: 2\n",
            "    max_verification_targets: 1\n",
            "    max_readonly_targets: 1\n",
            "    max_total_bytes: 8192\n",
            "  context:\n",
            "    include_parent_principles: false\n",
            "    include_parent_rules: false\n",
            "adapters: { enabled: [rust] }\n",
        ),
    )
    .unwrap();
    fs::write(
        temp.path().join("spec/foundation.yaml"),
        concat!(
            "schema: syu/spec/v1\n",
            "kind: philosophies\n",
            "namespace: sample\n",
            "category: Sample\n",
            "philosophies:\n",
            "  - id: PHIL-SAMPLE-EXCLUDE\n",
            "    title: Sample\n",
            "    summary: Sample philosophy.\n",
            "    principles:\n",
            "      - { id: governed, statement: Keep modifications governed., applies_to: [product] }\n",
            "    bindings: []\n",
        ),
    )
    .unwrap();
    fs::write(
        temp.path().join("spec/policy.yaml"),
        concat!(
            "schema: syu/spec/v1\n",
            "kind: policies\n",
            "namespace: sample\n",
            "category: Sample\n",
            "policies:\n",
            "  - id: POL-SAMPLE-EXCLUDE\n",
            "    title: Sample\n",
            "    summary: Sample policy.\n",
            "    description: Sample policy.\n",
            "    rules:\n",
            "      - id: governed\n",
            "        level: should\n",
            "        statement: Keep modifications governed.\n",
            "        governed_by: [PHIL-SAMPLE-EXCLUDE#principle.governed]\n",
            "    bindings: []\n",
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
            "  - id: REQ-SAMPLE-EXCLUDE\n",
            "    title: Sample\n",
            "    description: Sample requirement.\n",
            "    priority: medium\n",
            "    status: implemented\n",
            "    criteria:\n",
            "      - id: change\n",
            "        kind: behavior\n",
            "        statement: Modify the governed behavior.\n",
            "        governed_by: [POL-SAMPLE-EXCLUDE#rule.governed]\n",
            "    bindings:\n",
            "      - id: verify\n",
            "        role: verification\n",
            "        facet: verification\n",
            "        responsibility: Verify the modified behavior.\n",
            "        targets:\n",
            "          - { id: case, adapter: rust, path: tests/check.rs, selector: { kind: symbol, names: [check_behavior] } }\n",
            "        verifies: [REQ-SAMPLE-EXCLUDE#criterion.change]\n",
        ),
    )
    .unwrap();
    fs::write(
        temp.path().join("spec/feature.yaml"),
        concat!(
            "schema: syu/spec/v1\n",
            "kind: features\n",
            "namespace: sample\n",
            "category: Sample\n",
            "features:\n",
            "  - id: FEAT-SAMPLE-EXCLUDE\n",
            "    title: Sample\n",
            "    summary: Exclude a missing target.\n",
            "    status: implemented\n",
            "    bindings:\n",
            "      - id: app\n",
            "        role: implementation\n",
            "        facet: backend\n",
            "        responsibility: Modify the function body.\n",
            "        targets:\n",
            "          - { id: live, adapter: rust, path: src/app.rs, selector: { kind: symbol, names: [governed_behavior] } }\n",
            "          - { id: app, adapter: rust, path: src/missing.rs, selector: { kind: symbol, names: [governed_behavior] } }\n",
            "        satisfies: [REQ-SAMPLE-EXCLUDE#criterion.change]\n",
        ),
    )
    .unwrap();
    fs::write(
        temp.path().join("src/app.rs"),
        "fn governed_behavior() {}\n",
    )
    .unwrap();
    fs::write(
        temp.path().join("tests/check.rs"),
        "fn check_behavior() {}\n",
    )
    .unwrap();
    init_workspace_repo(temp.path());
    let request = temp.path().join("work.yaml");
    let plan = temp.path().join("plan.yaml");
    fs::write(
        &request,
        concat!(
            "schema: syu/work-request/v1\n",
            "id: WORK-EXCLUDE-001\n",
            "summary: Exclude missing implementation target.\n",
            "operation: modify\n",
            "seeds: [REQ-SAMPLE-EXCLUDE#criterion.change]\n",
            "constraints:\n",
            "  include_facets: []\n",
            "  exclude_paths: [src/missing.rs]\n",
            "  max_slices: 2\n",
        ),
    )
    .unwrap();
    Command::cargo_bin("syu")
        .unwrap()
        .args(["work", "plan", "--request"])
        .arg(&request)
        .args(["--out"])
        .arg(&plan)
        .args(["--workspace"])
        .arg(temp.path())
        .assert()
        .success();
    let text = fs::read_to_string(&plan).unwrap();
    assert!(text.contains("status: ready"));
    assert!(!text.contains("SYU-TARGET-002"));
    assert!(!text.contains("resolved_path: src/missing.rs"));
}

#[test]
fn modify_plan_validates_after_editable_body_change() {
    let temp = tempdir().unwrap();
    fs::create_dir_all(temp.path().join("spec")).unwrap();
    fs::create_dir_all(temp.path().join("src")).unwrap();
    fs::create_dir_all(temp.path().join("tests")).unwrap();
    fs::write(
        temp.path().join("syu.yaml"),
        concat!(
            "schema: syu/config/v1\n",
            "workspace:\n",
            "  spec_roots: [spec]\n",
            "  artifact_roots: [src, tests]\n",
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
            "    max_editable_files: 2\n",
            "    max_editable_symbols: 2\n",
            "    max_verification_targets: 1\n",
            "    max_readonly_targets: 1\n",
            "    max_total_bytes: 8192\n",
            "  context:\n",
            "    include_parent_principles: false\n",
            "    include_parent_rules: false\n",
            "adapters: { enabled: [rust] }\n",
        ),
    )
    .unwrap();
    fs::write(
        temp.path().join("spec/foundation.yaml"),
        concat!(
            "schema: syu/spec/v1\n",
            "kind: philosophies\n",
            "namespace: sample\n",
            "category: Sample\n",
            "philosophies:\n",
            "  - id: PHIL-SAMPLE-MODIFY\n",
            "    title: Sample\n",
            "    summary: Sample philosophy.\n",
            "    principles:\n",
            "      - { id: governed, statement: Keep modifications governed., applies_to: [product] }\n",
            "    bindings: []\n",
        ),
    )
    .unwrap();
    fs::write(
        temp.path().join("spec/policy.yaml"),
        concat!(
            "schema: syu/spec/v1\n",
            "kind: policies\n",
            "namespace: sample\n",
            "category: Sample\n",
            "policies:\n",
            "  - id: POL-SAMPLE-MODIFY\n",
            "    title: Sample\n",
            "    summary: Sample policy.\n",
            "    description: Sample policy.\n",
            "    rules:\n",
            "      - id: governed\n",
            "        level: should\n",
            "        statement: Keep modifications governed.\n",
            "        governed_by: [PHIL-SAMPLE-MODIFY#principle.governed]\n",
            "    bindings: []\n",
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
            "  - id: REQ-SAMPLE-MODIFY\n",
            "    title: Sample\n",
            "    description: Sample requirement.\n",
            "    priority: medium\n",
            "    status: implemented\n",
            "    criteria:\n",
            "      - id: change\n",
            "        kind: behavior\n",
            "        statement: Modify the governed behavior.\n",
            "        governed_by: [POL-SAMPLE-MODIFY#rule.governed]\n",
            "    bindings:\n",
            "      - id: verify\n",
            "        role: verification\n",
            "        facet: verification\n",
            "        responsibility: Verify the modified behavior.\n",
            "        targets:\n",
            "          - { id: case, adapter: rust, path: tests/check.rs, selector: { kind: symbol, names: [check_behavior] } }\n",
            "        verifies: [REQ-SAMPLE-MODIFY#criterion.change]\n",
        ),
    )
    .unwrap();
    fs::write(
        temp.path().join("spec/feature.yaml"),
        concat!(
            "schema: syu/spec/v1\n",
            "kind: features\n",
            "namespace: sample\n",
            "category: Sample\n",
            "features:\n",
            "  - id: FEAT-SAMPLE-MODIFY\n",
            "    title: Sample\n",
            "    summary: Modify a function body.\n",
            "    status: implemented\n",
            "    bindings:\n",
            "      - id: app\n",
            "        role: implementation\n",
            "        facet: backend\n",
            "        responsibility: Modify the function body.\n",
            "        targets:\n",
            "          - { id: app, adapter: rust, path: src/app.rs, selector: { kind: symbol, names: [governed_behavior] } }\n",
            "        satisfies: [REQ-SAMPLE-MODIFY#criterion.change]\n",
        ),
    )
    .unwrap();
    fs::write(
        temp.path().join("src/app.rs"),
        "fn governed_behavior() {\n    let value = 1;\n    assert_eq!(value, 1);\n}\n\nfn sibling_behavior() {\n    assert_eq!(2, 2);\n}\n",
    )
    .unwrap();
    fs::write(
        temp.path().join("tests/check.rs"),
        "fn check_behavior() {}\n",
    )
    .unwrap();
    init_workspace_repo(temp.path());
    let request = temp.path().join("work.yaml");
    let plan = temp.path().join("plan.yaml");
    fs::write(
        &request,
        concat!(
            "schema: syu/work-request/v1\n",
            "id: WORK-MODIFY-001\n",
            "summary: Modify the governed target.\n",
            "operation: modify\n",
            "seeds: [REQ-SAMPLE-MODIFY#criterion.change]\n",
            "constraints: { include_facets: [], exclude_paths: [], max_slices: 2, max_added_bytes_per_target: 256, max_added_lines_per_target: 32 }\n",
        ),
    )
    .unwrap();
    Command::cargo_bin("syu")
        .unwrap()
        .args(["work", "plan", "--request"])
        .arg(&request)
        .args(["--out"])
        .arg(&plan)
        .args(["--workspace"])
        .arg(temp.path())
        .assert()
        .success();
    fs::write(
        temp.path().join("src/app.rs"),
        "fn governed_behavior() {\n    let value = 2;\n    assert_eq!(value, 2);\n}\n\nfn sibling_behavior() {\n    assert_eq!(2, 2);\n}\n",
    )
    .unwrap();
    Command::cargo_bin("syu")
        .unwrap()
        .args(["validate"])
        .arg(temp.path())
        .args(["--plan"])
        .arg(&plan)
        .args(["--range", "HEAD"])
        .assert()
        .success();
}

#[test]
fn modify_plan_rejects_same_file_sibling_change_outside_scope() {
    let temp = tempdir().unwrap();
    fs::create_dir_all(temp.path().join("spec")).unwrap();
    fs::create_dir_all(temp.path().join("src")).unwrap();
    fs::create_dir_all(temp.path().join("tests")).unwrap();
    fs::write(
        temp.path().join("syu.yaml"),
        concat!(
            "schema: syu/config/v1\n",
            "workspace:\n",
            "  spec_roots: [spec]\n",
            "  artifact_roots: [src, tests]\n",
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
            "    max_editable_files: 2\n",
            "    max_editable_symbols: 2\n",
            "    max_verification_targets: 1\n",
            "    max_readonly_targets: 1\n",
            "    max_total_bytes: 8192\n",
            "  context:\n",
            "    include_parent_principles: false\n",
            "    include_parent_rules: false\n",
            "adapters: { enabled: [rust] }\n",
        ),
    )
    .unwrap();
    fs::write(
        temp.path().join("spec/foundation.yaml"),
        concat!(
            "schema: syu/spec/v1\n",
            "kind: philosophies\n",
            "namespace: sample\n",
            "category: Sample\n",
            "philosophies:\n",
            "  - id: PHIL-SAMPLE-SIBLING\n",
            "    title: Sample\n",
            "    summary: Sample philosophy.\n",
            "    principles:\n",
            "      - { id: governed, statement: Keep modifications governed., applies_to: [product] }\n",
            "    bindings: []\n",
        ),
    )
    .unwrap();
    fs::write(
        temp.path().join("spec/policy.yaml"),
        concat!(
            "schema: syu/spec/v1\n",
            "kind: policies\n",
            "namespace: sample\n",
            "category: Sample\n",
            "policies:\n",
            "  - id: POL-SAMPLE-SIBLING\n",
            "    title: Sample\n",
            "    summary: Sample policy.\n",
            "    description: Sample policy.\n",
            "    rules:\n",
            "      - id: governed\n",
            "        level: should\n",
            "        statement: Keep modifications governed.\n",
            "        governed_by: [PHIL-SAMPLE-SIBLING#principle.governed]\n",
            "    bindings: []\n",
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
            "  - id: REQ-SAMPLE-SIBLING\n",
            "    title: Sample\n",
            "    description: Sample requirement.\n",
            "    priority: medium\n",
            "    status: implemented\n",
            "    criteria:\n",
            "      - id: change\n",
            "        kind: behavior\n",
            "        statement: Modify the governed behavior.\n",
            "        governed_by: [POL-SAMPLE-SIBLING#rule.governed]\n",
            "    bindings:\n",
            "      - id: verify\n",
            "        role: verification\n",
            "        facet: verification\n",
            "        responsibility: Verify the modified behavior.\n",
            "        targets:\n",
            "          - { id: case, adapter: rust, path: tests/check.rs, selector: { kind: symbol, names: [check_behavior] } }\n",
            "        verifies: [REQ-SAMPLE-SIBLING#criterion.change]\n",
        ),
    )
    .unwrap();
    fs::write(
        temp.path().join("spec/feature.yaml"),
        concat!(
            "schema: syu/spec/v1\n",
            "kind: features\n",
            "namespace: sample\n",
            "category: Sample\n",
            "features:\n",
            "  - id: FEAT-SAMPLE-SIBLING\n",
            "    title: Sample\n",
            "    summary: Modify a function body.\n",
            "    status: implemented\n",
            "    bindings:\n",
            "      - id: app\n",
            "        role: implementation\n",
            "        facet: backend\n",
            "        responsibility: Modify the function body.\n",
            "        targets:\n",
            "          - { id: app, adapter: rust, path: src/app.rs, selector: { kind: symbol, names: [governed_behavior] } }\n",
            "        satisfies: [REQ-SAMPLE-SIBLING#criterion.change]\n",
        ),
    )
    .unwrap();
    fs::write(
        temp.path().join("src/app.rs"),
        "fn governed_behavior() {\n    let value = 1;\n    assert_eq!(value, 1);\n}\n\nfn sibling_behavior() {\n    assert_eq!(2, 2);\n}\n",
    )
    .unwrap();
    fs::write(
        temp.path().join("tests/check.rs"),
        "fn check_behavior() {}\n",
    )
    .unwrap();
    init_workspace_repo(temp.path());
    let request = temp.path().join("work.yaml");
    let plan = temp.path().join("plan.yaml");
    fs::write(
        &request,
        concat!(
            "schema: syu/work-request/v1\n",
            "id: WORK-SIBLING-001\n",
            "summary: Modify the governed target.\n",
            "operation: modify\n",
            "seeds: [REQ-SAMPLE-SIBLING#criterion.change]\n",
            "constraints: { include_facets: [], exclude_paths: [], max_slices: 2, max_added_bytes_per_target: 256, max_added_lines_per_target: 32 }\n",
        ),
    )
    .unwrap();
    Command::cargo_bin("syu")
        .unwrap()
        .args(["work", "plan", "--request"])
        .arg(&request)
        .args(["--out"])
        .arg(&plan)
        .args(["--workspace"])
        .arg(temp.path())
        .assert()
        .success();
    fs::write(
        temp.path().join("src/app.rs"),
        "fn governed_behavior() {\n    let value = 1;\n    assert_eq!(value, 1);\n}\n\nfn sibling_behavior() {\n    let sibling = 3;\n    assert_eq!(sibling, 3);\n}\n",
    )
    .unwrap();
    Command::cargo_bin("syu")
        .unwrap()
        .args(["validate"])
        .arg(temp.path())
        .args(["--plan"])
        .arg(&plan)
        .args(["--range", "HEAD"])
        .assert()
        .failure();
}

#[test]
fn modify_plan_does_not_allow_head_head_to_hide_scope_violations() {
    let temp = tempdir().unwrap();
    fs::create_dir_all(temp.path().join("spec")).unwrap();
    fs::create_dir_all(temp.path().join("src")).unwrap();
    fs::create_dir_all(temp.path().join("tests")).unwrap();
    fs::write(
        temp.path().join("syu.yaml"),
        concat!(
            "schema: syu/config/v1\n",
            "workspace:\n",
            "  spec_roots: [spec]\n",
            "  artifact_roots: [src, tests]\n",
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
            "    max_editable_files: 2\n",
            "    max_editable_symbols: 2\n",
            "    max_verification_targets: 1\n",
            "    max_readonly_targets: 1\n",
            "    max_total_bytes: 8192\n",
            "  context:\n",
            "    include_parent_principles: false\n",
            "    include_parent_rules: false\n",
            "adapters: { enabled: [rust] }\n",
        ),
    )
    .unwrap();
    fs::write(
        temp.path().join("spec/foundation.yaml"),
        concat!(
            "schema: syu/spec/v1\n",
            "kind: philosophies\n",
            "namespace: sample\n",
            "category: Sample\n",
            "philosophies:\n",
            "  - id: PHIL-SAMPLE-BYPASS\n",
            "    title: Sample\n",
            "    summary: Sample philosophy.\n",
            "    principles:\n",
            "      - { id: governed, statement: Keep modifications governed., applies_to: [product] }\n",
            "    bindings: []\n",
        ),
    )
    .unwrap();
    fs::write(
        temp.path().join("spec/policy.yaml"),
        concat!(
            "schema: syu/spec/v1\n",
            "kind: policies\n",
            "namespace: sample\n",
            "category: Sample\n",
            "policies:\n",
            "  - id: POL-SAMPLE-BYPASS\n",
            "    title: Sample\n",
            "    summary: Sample policy.\n",
            "    description: Sample policy.\n",
            "    rules:\n",
            "      - id: governed\n",
            "        level: should\n",
            "        statement: Keep modifications governed.\n",
            "        governed_by: [PHIL-SAMPLE-BYPASS#principle.governed]\n",
            "    bindings: []\n",
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
            "  - id: REQ-SAMPLE-BYPASS\n",
            "    title: Sample\n",
            "    description: Sample requirement.\n",
            "    priority: medium\n",
            "    status: implemented\n",
            "    criteria:\n",
            "      - id: change\n",
            "        kind: behavior\n",
            "        statement: Modify the governed behavior.\n",
            "        governed_by: [POL-SAMPLE-BYPASS#rule.governed]\n",
            "    bindings:\n",
            "      - id: verify\n",
            "        role: verification\n",
            "        facet: verification\n",
            "        responsibility: Verify the modified behavior.\n",
            "        targets:\n",
            "          - { id: case, adapter: rust, path: tests/check.rs, selector: { kind: symbol, names: [check_behavior] } }\n",
            "        verifies: [REQ-SAMPLE-BYPASS#criterion.change]\n",
        ),
    )
    .unwrap();
    fs::write(
        temp.path().join("spec/feature.yaml"),
        concat!(
            "schema: syu/spec/v1\n",
            "kind: features\n",
            "namespace: sample\n",
            "category: Sample\n",
            "features:\n",
            "  - id: FEAT-SAMPLE-BYPASS\n",
            "    title: Sample\n",
            "    summary: Modify a function body.\n",
            "    status: implemented\n",
            "    bindings:\n",
            "      - id: app\n",
            "        role: implementation\n",
            "        facet: backend\n",
            "        responsibility: Modify the function body.\n",
            "        targets:\n",
            "          - { id: app, adapter: rust, path: src/app.rs, selector: { kind: symbol, names: [governed_behavior] } }\n",
            "        satisfies: [REQ-SAMPLE-BYPASS#criterion.change]\n",
        ),
    )
    .unwrap();
    fs::write(
        temp.path().join("src/app.rs"),
        "fn governed_behavior() {\n    let value = 1;\n    assert_eq!(value, 1);\n}\n\nfn sibling_behavior() {\n    assert_eq!(2, 2);\n}\n",
    )
    .unwrap();
    fs::write(
        temp.path().join("tests/check.rs"),
        "fn check_behavior() {}\n",
    )
    .unwrap();
    init_workspace_repo(temp.path());
    let request = temp.path().join("work.yaml");
    let plan = temp.path().join("plan.yaml");
    fs::write(
        &request,
        concat!(
            "schema: syu/work-request/v1\n",
            "id: WORK-BYPASS-001\n",
            "summary: Modify the governed target.\n",
            "operation: modify\n",
            "seeds: [REQ-SAMPLE-BYPASS#criterion.change]\n",
            "constraints: { include_facets: [], exclude_paths: [], max_slices: 2, max_added_bytes_per_target: 256, max_added_lines_per_target: 32 }\n",
        ),
    )
    .unwrap();
    Command::cargo_bin("syu")
        .unwrap()
        .args(["work", "plan", "--request"])
        .arg(&request)
        .args(["--out"])
        .arg(&plan)
        .args(["--workspace"])
        .arg(temp.path())
        .assert()
        .success();
    fs::write(
        temp.path().join("src/app.rs"),
        "fn governed_behavior() {\n    let value = 1;\n    assert_eq!(value, 1);\n}\n\nfn sibling_behavior() {\n    let sibling = 3;\n    assert_eq!(sibling, 3);\n}\n",
    )
    .unwrap();
    let output = Command::cargo_bin("syu")
        .unwrap()
        .args(["validate"])
        .arg(temp.path())
        .args(["--plan"])
        .arg(&plan)
        .args(["--range", "HEAD..HEAD"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("reported range does not cover all actual post-state changes"));
}

#[test]
fn add_plan_supports_missing_declared_target_and_validates_after_creation() {
    let temp = tempdir().unwrap();
    fs::create_dir_all(temp.path().join("spec")).unwrap();
    fs::create_dir_all(temp.path().join("src")).unwrap();
    fs::create_dir_all(temp.path().join("tests")).unwrap();
    fs::write(
        temp.path().join("syu.yaml"),
        concat!(
            "schema: syu/config/v1\n",
            "workspace:\n",
            "  spec_roots: [spec]\n",
            "  artifact_roots: [src, tests]\n",
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
            "    max_editable_files: 2\n",
            "    max_editable_symbols: 2\n",
            "    max_verification_targets: 1\n",
            "    max_readonly_targets: 1\n",
            "    max_total_bytes: 8192\n",
            "  context:\n",
            "    include_parent_principles: false\n",
            "    include_parent_rules: false\n",
            "adapters: { enabled: [rust] }\n",
        ),
    )
    .unwrap();
    fs::write(
        temp.path().join("spec/foundation.yaml"),
        concat!(
            "schema: syu/spec/v1\n",
            "kind: philosophies\n",
            "namespace: sample\n",
            "category: Sample\n",
            "philosophies:\n",
            "  - id: PHIL-SAMPLE-ADD\n",
            "    title: Sample\n",
            "    summary: Sample philosophy.\n",
            "    principles:\n",
            "      - { id: governed, statement: Keep additions governed., applies_to: [product] }\n",
            "    bindings: []\n",
        ),
    )
    .unwrap();
    fs::write(
        temp.path().join("spec/policy.yaml"),
        concat!(
            "schema: syu/spec/v1\n",
            "kind: policies\n",
            "namespace: sample\n",
            "category: Sample\n",
            "policies:\n",
            "  - id: POL-SAMPLE-ADD\n",
            "    title: Sample\n",
            "    summary: Sample policy.\n",
            "    description: Sample policy.\n",
            "    rules:\n",
            "      - id: governed\n",
            "        level: should\n",
            "        statement: Keep additions governed.\n",
            "        governed_by: [PHIL-SAMPLE-ADD#principle.governed]\n",
            "    bindings: []\n",
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
            "  - id: REQ-SAMPLE-ADD\n",
            "    title: Sample\n",
            "    description: Sample requirement.\n",
            "    priority: medium\n",
            "    status: implemented\n",
            "    criteria:\n",
            "      - id: create\n",
            "        kind: behavior\n",
            "        statement: Create the new behavior.\n",
            "        governed_by: [POL-SAMPLE-ADD#rule.governed]\n",
            "    bindings:\n",
            "      - id: verify\n",
            "        role: verification\n",
            "        facet: verification\n",
            "        responsibility: Verify the new behavior.\n",
            "        targets:\n",
            "          - { id: case, adapter: rust, path: tests/check.rs, selector: { kind: symbol, names: [check_new_behavior] } }\n",
            "        verifies: [REQ-SAMPLE-ADD#criterion.create]\n",
        ),
    )
    .unwrap();
    fs::write(
        temp.path().join("spec/feature.yaml"),
        concat!(
            "schema: syu/spec/v1\n",
            "kind: features\n",
            "namespace: sample\n",
            "category: Sample\n",
            "features:\n",
            "  - id: FEAT-SAMPLE-ADD\n",
            "    title: Sample\n",
            "    summary: Add a new function.\n",
            "    status: implemented\n",
            "    bindings:\n",
            "      - id: app\n",
            "        role: implementation\n",
            "        facet: backend\n",
            "        responsibility: Add the new function.\n",
            "        targets:\n",
            "          - { id: new, adapter: rust, path: src/new.rs, selector: { kind: symbol, names: [new_behavior] } }\n",
            "        satisfies: [REQ-SAMPLE-ADD#criterion.create]\n",
        ),
    )
    .unwrap();
    fs::write(
        temp.path().join("tests/check.rs"),
        "fn check_new_behavior() {}\n",
    )
    .unwrap();
    init_workspace_repo(temp.path());
    let request = temp.path().join("work.yaml");
    let plan = temp.path().join("plan.yaml");
    fs::write(
        &request,
        concat!(
            "schema: syu/work-request/v1\n",
            "id: WORK-ADD-001\n",
            "summary: Add the new target.\n",
            "operation: add\n",
            "seeds: [REQ-SAMPLE-ADD#criterion.create]\n",
            "constraints: { include_facets: [], exclude_paths: [], max_slices: 2, max_added_bytes_per_target: 256, max_added_lines_per_target: 32 }\n",
        ),
    )
    .unwrap();
    Command::cargo_bin("syu")
        .unwrap()
        .args(["work", "plan", "--request"])
        .arg(&request)
        .args(["--out"])
        .arg(&plan)
        .args(["--workspace"])
        .arg(temp.path())
        .assert()
        .success();
    let text = fs::read_to_string(&plan).unwrap();
    assert!(text.contains("lifecycle: ensure-present"));
    let slice = text
        .lines()
        .find_map(|line| line.strip_prefix("- id: "))
        .unwrap()
        .to_string();
    Command::cargo_bin("syu")
        .unwrap()
        .args(["work", "export-context", "--plan"])
        .arg(&plan)
        .args(["--slice", &slice, "--workspace"])
        .arg(temp.path())
        .assert()
        .success();
    fs::write(temp.path().join("src/new.rs"), "fn new_behavior() {}\n").unwrap();
    fs::write(
        temp.path().join("tests/check.rs"),
        "fn check_new_behavior() {}\n",
    )
    .unwrap();
    Command::cargo_bin("syu")
        .unwrap()
        .args(["validate"])
        .arg(temp.path())
        .args(["--plan"])
        .arg(&plan)
        .args(["--range", "HEAD"])
        .assert()
        .success();
}

#[test]
fn add_plan_existing_file_uses_real_container_snapshot_and_nonzero_budget() {
    let temp = tempdir().unwrap();
    fs::create_dir_all(temp.path().join("spec")).unwrap();
    fs::create_dir_all(temp.path().join("src")).unwrap();
    fs::create_dir_all(temp.path().join("tests")).unwrap();
    fs::write(
        temp.path().join("syu.yaml"),
        concat!(
            "schema: syu/config/v1\n",
            "workspace:\n",
            "  spec_roots: [spec]\n",
            "  artifact_roots: [src, tests]\n",
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
            "    max_editable_files: 2\n",
            "    max_editable_symbols: 2\n",
            "    max_verification_targets: 1\n",
            "    max_readonly_targets: 1\n",
            "    max_total_bytes: 4096\n",
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
            "  - id: REQ-SAMPLE-ADD-EXISTING\n",
            "    title: Sample\n",
            "    description: Sample requirement.\n",
            "    priority: medium\n",
            "    status: implemented\n",
            "    criteria:\n",
            "      - id: create\n",
            "        kind: behavior\n",
            "        statement: Add a new sibling function.\n",
            "        governed_by: []\n",
            "    bindings:\n",
            "      - id: verify\n",
            "        role: verification\n",
            "        facet: verification\n",
            "        responsibility: Verify new behavior.\n",
            "        targets:\n",
            "          - { id: case, adapter: rust, path: tests/check.rs, selector: { kind: symbol, names: [check_new_behavior] } }\n",
            "        verifies: [REQ-SAMPLE-ADD-EXISTING#criterion.create]\n",
        ),
    )
    .unwrap();
    fs::write(
        temp.path().join("spec/feature.yaml"),
        concat!(
            "schema: syu/spec/v1\n",
            "kind: features\n",
            "namespace: sample\n",
            "category: Sample\n",
            "features:\n",
            "  - id: FEAT-SAMPLE-ADD-EXISTING\n",
            "    title: Sample\n",
            "    summary: Add a new function to an existing file.\n",
            "    status: implemented\n",
            "    bindings:\n",
            "      - id: app\n",
            "        role: implementation\n",
            "        facet: backend\n",
            "        responsibility: Add the new function.\n",
            "        targets:\n",
            "          - { id: new, adapter: rust, path: src/lib.rs, selector: { kind: symbol, names: [new_behavior] } }\n",
            "        satisfies: [REQ-SAMPLE-ADD-EXISTING#criterion.create]\n",
        ),
    )
    .unwrap();
    fs::write(
        temp.path().join("src/lib.rs"),
        "fn existing_behavior() {}\n",
    )
    .unwrap();
    fs::write(
        temp.path().join("tests/check.rs"),
        "fn check_new_behavior() {}\n",
    )
    .unwrap();
    init_workspace_repo(temp.path());
    let request = temp.path().join("work.yaml");
    let plan = temp.path().join("plan.yaml");
    fs::write(
        &request,
        concat!(
            "schema: syu/work-request/v1\n",
            "id: WORK-ADD-EXISTING-001\n",
            "summary: Add the new target.\n",
            "operation: add\n",
            "seeds: [REQ-SAMPLE-ADD-EXISTING#criterion.create]\n",
            "constraints: { include_facets: [], exclude_paths: [], max_slices: 2, max_added_bytes_per_target: 256, max_added_lines_per_target: 32 }\n",
        ),
    )
    .unwrap();
    Command::cargo_bin("syu")
        .unwrap()
        .args(["work", "plan", "--request"])
        .arg(&request)
        .args(["--out"])
        .arg(&plan)
        .args(["--workspace"])
        .arg(temp.path())
        .assert()
        .success();
    let text = fs::read_to_string(&plan).unwrap();
    assert!(text.contains("lifecycle: ensure-present"));
    assert!(!text.contains("content_hash: declared"));
    assert!(!text.contains("excerpt_hash: declared"));
    assert!(!text.contains("line_end: 18446744073709551615"));
    assert!(!text.contains("budget_bytes: 0"));
    let slice = text
        .lines()
        .find_map(|line| line.strip_prefix("- id: "))
        .unwrap()
        .to_string();
    let pack = temp.path().join("context.yaml");
    Command::cargo_bin("syu")
        .unwrap()
        .args(["work", "export-context", "--plan"])
        .arg(&plan)
        .args(["--slice", &slice, "--workspace"])
        .arg(temp.path())
        .args(["--out"])
        .arg(&pack)
        .assert()
        .success();
    let context = fs::read_to_string(&pack).unwrap();
    assert!(context.contains("Container context for new target."));
    assert!(context.contains("mode: readonly"));
}

#[test]
fn add_plan_enforces_line_budget_for_file_targets() {
    let temp = tempdir().unwrap();
    fs::create_dir_all(temp.path().join("spec")).unwrap();
    fs::create_dir_all(temp.path().join("src")).unwrap();
    fs::create_dir_all(temp.path().join("tests")).unwrap();
    fs::write(
        temp.path().join("syu.yaml"),
        concat!(
            "schema: syu/config/v1\n",
            "workspace:\n",
            "  spec_roots: [spec]\n",
            "  artifact_roots: [src, tests]\n",
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
            "    max_editable_files: 2\n",
            "    max_editable_symbols: 2\n",
            "    max_verification_targets: 1\n",
            "    max_readonly_targets: 1\n",
            "    max_total_bytes: 8192\n",
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
            "  - id: REQ-SAMPLE-FILE-ADD\n",
            "    title: Sample\n",
            "    description: Sample requirement.\n",
            "    priority: medium\n",
            "    status: implemented\n",
            "    criteria:\n",
            "      - id: create\n",
            "        kind: behavior\n",
            "        statement: Create the new file.\n",
            "        governed_by: []\n",
            "    bindings:\n",
            "      - id: verify\n",
            "        role: verification\n",
            "        facet: verification\n",
            "        responsibility: Verify the new file.\n",
            "        targets:\n",
            "          - { id: case, adapter: rust, path: tests/check.rs, selector: { kind: symbol, names: [check_new_file] } }\n",
            "        verifies: [REQ-SAMPLE-FILE-ADD#criterion.create]\n",
        ),
    )
    .unwrap();
    fs::write(
        temp.path().join("spec/feature.yaml"),
        concat!(
            "schema: syu/spec/v1\n",
            "kind: features\n",
            "namespace: sample\n",
            "category: Sample\n",
            "features:\n",
            "  - id: FEAT-SAMPLE-FILE-ADD\n",
            "    title: Sample\n",
            "    summary: Add a new file.\n",
            "    status: implemented\n",
            "    bindings:\n",
            "      - id: app\n",
            "        role: implementation\n",
            "        facet: backend\n",
            "        responsibility: Add the new file.\n",
            "        targets:\n",
            "          - { id: new, adapter: rust, path: src/new.rs, selector: { kind: file } }\n",
            "        satisfies: [REQ-SAMPLE-FILE-ADD#criterion.create]\n",
        ),
    )
    .unwrap();
    fs::write(
        temp.path().join("tests/check.rs"),
        "fn check_new_file() {}\n",
    )
    .unwrap();
    init_workspace_repo(temp.path());
    let request = temp.path().join("work.yaml");
    let plan = temp.path().join("plan.yaml");
    fs::write(
        &request,
        concat!(
            "schema: syu/work-request/v1\n",
            "id: WORK-ADD-FILE-001\n",
            "summary: Add the new file.\n",
            "operation: add\n",
            "seeds: [REQ-SAMPLE-FILE-ADD#criterion.create]\n",
            "constraints: { include_facets: [], exclude_paths: [], max_slices: 2, max_added_bytes_per_target: 4096, max_added_lines_per_target: 1 }\n",
        ),
    )
    .unwrap();
    Command::cargo_bin("syu")
        .unwrap()
        .args(["work", "plan", "--request"])
        .arg(&request)
        .args(["--out"])
        .arg(&plan)
        .args(["--workspace"])
        .arg(temp.path())
        .assert()
        .success();
    fs::write(
        temp.path().join("src/new.rs"),
        "fn a() {}\nfn b() {}\nfn c() {}\n",
    )
    .unwrap();
    Command::cargo_bin("syu")
        .unwrap()
        .args(["validate"])
        .arg(temp.path())
        .args(["--plan"])
        .arg(&plan)
        .args(["--range", "HEAD"])
        .assert()
        .failure();
}

#[test]
fn add_plan_rejects_noop_existing_target() {
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
    let request = temp.path().join("request.yaml");
    fs::write(
        &request,
        concat!(
            "schema: syu/work-request/v1\n",
            "id: WORK-ADD-NOOP-001\n",
            "summary: Attempt to add an existing target.\n",
            "operation: add\n",
            "seeds: [REQ-AUTH-001#criterion.invalid-credentials]\n",
            "constraints: { include_facets: [], exclude_paths: [], max_slices: 2, max_added_bytes_per_target: 256, max_added_lines_per_target: 32 }\n",
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
        .failure();
    let text = fs::read_to_string(&plan).unwrap();
    assert!(text.contains("add request does not introduce any new target"));
}

#[test]
fn remove_plan_rejects_missing_target() {
    let temp = tempdir().unwrap();
    fs::create_dir_all(temp.path().join("spec")).unwrap();
    fs::create_dir_all(temp.path().join("src")).unwrap();
    fs::create_dir_all(temp.path().join("tests")).unwrap();
    fs::write(
        temp.path().join("syu.yaml"),
        concat!(
            "schema: syu/config/v1\n",
            "workspace:\n",
            "  spec_roots: [spec]\n",
            "  artifact_roots: [src, tests]\n",
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
            "    max_editable_files: 2\n",
            "    max_editable_symbols: 2\n",
            "    max_verification_targets: 1\n",
            "    max_readonly_targets: 1\n",
            "    max_total_bytes: 4096\n",
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
            "  - id: REQ-SAMPLE-REMOVE-MISSING\n",
            "    title: Sample\n",
            "    description: Sample requirement.\n",
            "    priority: medium\n",
            "    status: implemented\n",
            "    criteria:\n",
            "      - id: drop\n",
            "        kind: behavior\n",
            "        statement: Remove the obsolete behavior.\n",
            "        governed_by: []\n",
            "    bindings:\n",
            "      - id: verify\n",
            "        role: verification\n",
            "        facet: verification\n",
            "        responsibility: Verify removal behavior.\n",
            "        targets:\n",
            "          - { id: case, adapter: rust, path: tests/check.rs, selector: { kind: symbol, names: [check_removed_behavior] } }\n",
            "        verifies: [REQ-SAMPLE-REMOVE-MISSING#criterion.drop]\n",
        ),
    )
    .unwrap();
    fs::write(
        temp.path().join("spec/feature.yaml"),
        concat!(
            "schema: syu/spec/v1\n",
            "kind: features\n",
            "namespace: sample\n",
            "category: Sample\n",
            "features:\n",
            "  - id: FEAT-SAMPLE-REMOVE-MISSING\n",
            "    title: Sample\n",
            "    summary: Remove a function.\n",
            "    status: implemented\n",
            "    bindings:\n",
            "      - id: app\n",
            "        role: implementation\n",
            "        facet: backend\n",
            "        responsibility: Remove the function.\n",
            "        targets:\n",
            "          - { id: old, adapter: rust, path: src/old.rs, selector: { kind: symbol, names: [old_behavior] } }\n",
            "        satisfies: [REQ-SAMPLE-REMOVE-MISSING#criterion.drop]\n",
        ),
    )
    .unwrap();
    fs::write(
        temp.path().join("tests/check.rs"),
        "fn check_removed_behavior() {}\n",
    )
    .unwrap();
    init_workspace_repo(temp.path());
    let request = temp.path().join("request.yaml");
    let plan = temp.path().join("plan.yaml");
    fs::write(
        &request,
        concat!(
            "schema: syu/work-request/v1\n",
            "id: WORK-REMOVE-NOOP-001\n",
            "summary: Attempt to remove a missing target.\n",
            "operation: remove\n",
            "seeds: [REQ-SAMPLE-REMOVE-MISSING#criterion.drop]\n",
            "constraints: { include_facets: [], exclude_paths: [], max_slices: 2 }\n",
        ),
    )
    .unwrap();
    Command::cargo_bin("syu")
        .unwrap()
        .args(["work", "plan", "--request"])
        .arg(&request)
        .args(["--out"])
        .arg(&plan)
        .args(["--workspace"])
        .arg(temp.path())
        .assert()
        .failure();
    let text = fs::read_to_string(&plan).unwrap();
    assert!(text.contains("remove target does not exist"));
}

#[test]
fn remove_plan_validates_after_target_is_deleted() {
    let temp = tempdir().unwrap();
    fs::create_dir_all(temp.path().join("spec")).unwrap();
    fs::create_dir_all(temp.path().join("src")).unwrap();
    fs::create_dir_all(temp.path().join("tests")).unwrap();
    fs::write(
        temp.path().join("syu.yaml"),
        concat!(
            "schema: syu/config/v1\n",
            "workspace:\n",
            "  spec_roots: [spec]\n",
            "  artifact_roots: [src, tests]\n",
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
            "    max_editable_files: 2\n",
            "    max_editable_symbols: 2\n",
            "    max_verification_targets: 1\n",
            "    max_readonly_targets: 1\n",
            "    max_total_bytes: 4096\n",
            "  context:\n",
            "    include_parent_principles: false\n",
            "    include_parent_rules: false\n",
            "adapters: { enabled: [rust] }\n",
        ),
    )
    .unwrap();
    fs::write(
        temp.path().join("spec/foundation.yaml"),
        concat!(
            "schema: syu/spec/v1\n",
            "kind: philosophies\n",
            "namespace: sample\n",
            "category: Sample\n",
            "philosophies:\n",
            "  - id: PHIL-SAMPLE-REMOVE\n",
            "    title: Sample\n",
            "    summary: Sample philosophy.\n",
            "    principles:\n",
            "      - { id: governed, statement: Keep removals governed., applies_to: [product] }\n",
            "    bindings: []\n",
        ),
    )
    .unwrap();
    fs::write(
        temp.path().join("spec/policy.yaml"),
        concat!(
            "schema: syu/spec/v1\n",
            "kind: policies\n",
            "namespace: sample\n",
            "category: Sample\n",
            "policies:\n",
            "  - id: POL-SAMPLE-REMOVE\n",
            "    title: Sample\n",
            "    summary: Sample policy.\n",
            "    description: Sample policy.\n",
            "    rules:\n",
            "      - id: governed\n",
            "        level: should\n",
            "        statement: Keep removals governed.\n",
            "        governed_by: [PHIL-SAMPLE-REMOVE#principle.governed]\n",
            "    bindings: []\n",
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
            "  - id: REQ-SAMPLE-REMOVE\n",
            "    title: Sample\n",
            "    description: Sample requirement.\n",
            "    priority: medium\n",
            "    status: implemented\n",
            "    criteria:\n",
            "      - id: drop\n",
            "        kind: behavior\n",
            "        statement: Remove the obsolete behavior.\n",
            "        governed_by: [POL-SAMPLE-REMOVE#rule.governed]\n",
            "    bindings:\n",
            "      - id: verify\n",
            "        role: verification\n",
            "        facet: verification\n",
            "        responsibility: Verify removal behavior.\n",
            "        targets:\n",
            "          - { id: case, adapter: rust, path: tests/check.rs, selector: { kind: symbol, names: [check_removed_behavior] } }\n",
            "        verifies: [REQ-SAMPLE-REMOVE#criterion.drop]\n",
        ),
    )
    .unwrap();
    fs::write(
        temp.path().join("spec/feature.yaml"),
        concat!(
            "schema: syu/spec/v1\n",
            "kind: features\n",
            "namespace: sample\n",
            "category: Sample\n",
            "features:\n",
            "  - id: FEAT-SAMPLE-REMOVE\n",
            "    title: Sample\n",
            "    summary: Remove a function.\n",
            "    status: implemented\n",
            "    bindings:\n",
            "      - id: app\n",
            "        role: implementation\n",
            "        facet: backend\n",
            "        responsibility: Remove the function.\n",
            "        targets:\n",
            "          - { id: old, adapter: rust, path: src/old.rs, selector: { kind: symbol, names: [old_behavior] } }\n",
            "          - { id: keep, adapter: rust, path: src/keep.rs, selector: { kind: symbol, names: [keep_behavior] } }\n",
            "        satisfies: [REQ-SAMPLE-REMOVE#criterion.drop]\n",
        ),
    )
    .unwrap();
    fs::write(temp.path().join("src/old.rs"), "fn old_behavior() {}\n").unwrap();
    fs::write(temp.path().join("src/keep.rs"), "fn keep_behavior() {}\n").unwrap();
    fs::write(
        temp.path().join("tests/check.rs"),
        "fn check_removed_behavior() {}\n",
    )
    .unwrap();
    init_workspace_repo(temp.path());
    let request = temp.path().join("work.yaml");
    let plan = temp.path().join("plan.yaml");
    fs::write(
        &request,
        concat!(
            "schema: syu/work-request/v1\n",
            "id: WORK-REMOVE-001\n",
            "summary: Remove the old target.\n",
            "operation: remove\n",
            "seeds: []\n",
            "requested_targets:\n",
            "  - { ref: FEAT-SAMPLE-REMOVE#binding.app/target.old, transition: remove }\n",
            "constraints: { include_facets: [], exclude_paths: [], max_slices: 2 }\n",
        ),
    )
    .unwrap();
    Command::cargo_bin("syu")
        .unwrap()
        .args(["work", "plan", "--request"])
        .arg(&request)
        .args(["--out"])
        .arg(&plan)
        .args(["--workspace"])
        .arg(temp.path())
        .assert()
        .success();
    let text = fs::read_to_string(&plan).unwrap();
    assert!(text.contains("lifecycle: ensure-absent"));
    fs::remove_file(temp.path().join("src/old.rs")).unwrap();
    let output = Command::cargo_bin("syu")
        .unwrap()
        .args(["validate"])
        .arg(temp.path())
        .args(["--plan"])
        .arg(&plan)
        .args(["--range", "HEAD"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(
        String::from_utf8(output.stdout)
            .unwrap()
            .contains("SYU-WORK-011")
    );
    fs::write(
        temp.path().join("spec/feature.yaml"),
        fs::read_to_string(temp.path().join("spec/feature.yaml"))
            .unwrap()
            .replace(
                "      - { id: old, adapter: rust, path: src/old.rs, selector: { kind: symbol, names: [old_behavior] } }\n",
                "",
            ),
    )
    .unwrap();
    Command::cargo_bin("syu")
        .unwrap()
        .args(["validate"])
        .arg(temp.path())
        .args(["--plan"])
        .arg(&plan)
        .args(["--range", "HEAD"])
        .assert()
        .success();
}

#[test]
fn oversized_slice_is_split_deterministically() {
    let temp = tempdir().unwrap();
    fs::create_dir_all(temp.path().join("spec")).unwrap();
    fs::create_dir_all(temp.path().join("src")).unwrap();
    fs::create_dir_all(temp.path().join("tests")).unwrap();
    fs::write(
        temp.path().join("spec/foundation.yaml"),
        concat!(
            "schema: syu/spec/v1\n",
            "kind: philosophies\n",
            "namespace: sample\n",
            "category: Sample\n",
            "philosophies:\n",
            "  - id: PHIL-SAMPLE-001\n",
            "    title: Sample\n",
            "    summary: Sample philosophy.\n",
            "    principles:\n",
            "      - { id: governed, statement: Keep behavior aligned., applies_to: [product] }\n",
            "    bindings: []\n",
        ),
    )
    .unwrap();
    fs::write(
        temp.path().join("spec/policy.yaml"),
        concat!(
            "schema: syu/spec/v1\n",
            "kind: policies\n",
            "namespace: sample\n",
            "category: Sample\n",
            "policies:\n",
            "  - id: POL-SAMPLE-001\n",
            "    title: Sample\n",
            "    summary: Sample policy.\n",
            "    description: Sample policy.\n",
            "    rules:\n",
            "      - id: governed\n",
            "        level: should\n",
            "        statement: Keep behavior aligned.\n",
            "        governed_by: [PHIL-SAMPLE-001#principle.governed]\n",
            "    bindings: []\n",
        ),
    )
    .unwrap();
    fs::write(
        temp.path().join("syu.yaml"),
        concat!(
            "schema: syu/config/v1\n",
            "workspace:\n",
            "  spec_roots: [spec]\n",
            "  artifact_roots: [src, tests]\n",
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
            "    max_editable_symbols: 4\n",
            "    max_verification_targets: 2\n",
            "    max_readonly_targets: 4\n",
            "    max_total_bytes: 4096\n",
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
            "      - id: multi\n",
            "        kind: behavior\n",
            "        statement: Keep behavior aligned across files.\n",
            "        governed_by: [POL-SAMPLE-001#rule.governed]\n",
            "    bindings:\n",
            "      - id: tests\n",
            "        role: verification\n",
            "        facet: verification\n",
            "        responsibility: Verify the split behavior.\n",
            "        targets:\n",
            "          - { id: case, adapter: rust, path: tests/check.rs, selector: { kind: symbol, names: [check_behavior] } }\n",
            "        verifies: [REQ-SAMPLE-001#criterion.multi]\n",
        ),
    )
    .unwrap();
    fs::write(
        temp.path().join("spec/feature.yaml"),
        concat!(
            "schema: syu/spec/v1\n",
            "kind: features\n",
            "namespace: sample\n",
            "category: Sample\n",
            "features:\n",
            "  - id: FEAT-SAMPLE-001\n",
            "    title: Sample\n",
            "    summary: Multi-file implementation.\n",
            "    status: implemented\n",
            "    bindings:\n",
            "      - id: app\n",
            "        role: implementation\n",
            "        facet: backend\n",
            "        responsibility: Update both files.\n",
            "        targets:\n",
            "          - { id: alpha, adapter: rust, path: src/a.rs, selector: { kind: symbol, names: [alpha] } }\n",
            "          - { id: beta, adapter: rust, path: src/b.rs, selector: { kind: symbol, names: [beta] } }\n",
            "        satisfies: [REQ-SAMPLE-001#criterion.multi]\n",
        ),
    )
    .unwrap();
    fs::write(temp.path().join("src/a.rs"), "fn alpha() {}\n").unwrap();
    fs::write(temp.path().join("src/b.rs"), "fn beta() {}\n").unwrap();
    fs::write(
        temp.path().join("tests/check.rs"),
        "fn check_behavior() {}\n",
    )
    .unwrap();
    let request = temp.path().join("work.yaml");
    fs::write(
        &request,
        concat!(
            "schema: syu/work-request/v1\n",
            "id: WORK-SPLIT-001\n",
            "summary: Split a multi-file change.\n",
            "operation: modify\n",
            "seeds: [REQ-SAMPLE-001#criterion.multi]\n",
            "constraints: { include_facets: [], exclude_paths: [], max_slices: 4 }\n",
        ),
    )
    .unwrap();
    init_workspace_repo(temp.path());
    let plan = temp.path().join("plan.yaml");
    Command::cargo_bin("syu")
        .unwrap()
        .args(["work", "plan", "--request"])
        .arg(&request)
        .args(["--out"])
        .arg(&plan)
        .args(["--workspace"])
        .arg(temp.path())
        .assert()
        .success();
    let text = fs::read_to_string(&plan).unwrap();
    assert!(text.contains("status: ready"));
    assert!(text.contains("part01"));
    assert!(text.contains("part02"));
    assert!(text.contains("resolved_path: src/a.rs"));
    assert!(text.contains("resolved_path: src/b.rs"));
    Command::cargo_bin("syu")
        .unwrap()
        .args(["validate"])
        .arg(temp.path())
        .args(["--plan"])
        .arg(&plan)
        .assert()
        .success();
    let slice_ids = text
        .lines()
        .filter_map(|line| line.strip_prefix("- id: "))
        .map(str::to_string)
        .collect::<Vec<_>>();
    for slice_id in slice_ids {
        Command::cargo_bin("syu")
            .unwrap()
            .args(["work", "export-context", "--plan"])
            .arg(&plan)
            .args(["--slice", &slice_id, "--workspace"])
            .arg(temp.path())
            .assert()
            .success();
    }
}

#[test]
fn post_state_multi_slice_validation_requires_selected_slice() {
    let temp = tempdir().unwrap();
    fs::create_dir_all(temp.path().join("spec")).unwrap();
    fs::create_dir_all(temp.path().join("src")).unwrap();
    fs::create_dir_all(temp.path().join("tests")).unwrap();
    fs::write(
        temp.path().join("syu.yaml"),
        concat!(
            "schema: syu/config/v1\n",
            "workspace:\n",
            "  spec_roots: [spec]\n",
            "  artifact_roots: [src, tests]\n",
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
            "    max_total_bytes: 4096\n",
            "  context:\n",
            "    include_parent_principles: false\n",
            "    include_parent_rules: false\n",
            "adapters: { enabled: [rust] }\n",
        ),
    )
    .unwrap();
    fs::write(
        temp.path().join("spec/foundation.yaml"),
        concat!(
            "schema: syu/spec/v1\n",
            "kind: philosophies\n",
            "namespace: sample\n",
            "category: Sample\n",
            "philosophies:\n",
            "  - id: PHIL-SAMPLE-MULTI\n",
            "    title: Sample\n",
            "    summary: Sample philosophy.\n",
            "    principles:\n",
            "      - { id: governed, statement: Keep changes governed., applies_to: [product] }\n",
            "    bindings: []\n",
        ),
    )
    .unwrap();
    fs::write(
        temp.path().join("spec/policy.yaml"),
        concat!(
            "schema: syu/spec/v1\n",
            "kind: policies\n",
            "namespace: sample\n",
            "category: Sample\n",
            "policies:\n",
            "  - id: POL-SAMPLE-MULTI\n",
            "    title: Sample\n",
            "    summary: Sample policy.\n",
            "    description: Sample policy.\n",
            "    rules:\n",
            "      - id: governed\n",
            "        level: should\n",
            "        statement: Keep changes governed.\n",
            "        governed_by: [PHIL-SAMPLE-MULTI#principle.governed]\n",
            "    bindings: []\n",
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
            "  - id: REQ-SAMPLE-MULTI\n",
            "    title: Sample\n",
            "    description: Sample requirement.\n",
            "    priority: medium\n",
            "    status: planned\n",
            "    criteria:\n",
            "      - id: multi\n",
            "        kind: behavior\n",
            "        statement: Keep behavior aligned across files.\n",
            "        governed_by: [POL-SAMPLE-MULTI#rule.governed]\n",
            "    bindings:\n",
            "      - id: tests\n",
            "        role: verification\n",
            "        facet: verification\n",
            "        responsibility: Verify the split behavior.\n",
            "        targets:\n",
            "          - { id: case, adapter: rust, path: tests/check.rs, selector: { kind: symbol, names: [check_behavior] } }\n",
            "        verifies: [REQ-SAMPLE-MULTI#criterion.multi]\n",
        ),
    )
    .unwrap();
    fs::write(
        temp.path().join("spec/feature.yaml"),
        concat!(
            "schema: syu/spec/v1\n",
            "kind: features\n",
            "namespace: sample\n",
            "category: Sample\n",
            "features:\n",
            "  - id: FEAT-SAMPLE-MULTI\n",
            "    title: Sample\n",
            "    summary: Multi-file implementation.\n",
            "    status: implemented\n",
            "    bindings:\n",
            "      - id: app\n",
            "        role: implementation\n",
            "        facet: backend\n",
            "        responsibility: Update both files.\n",
            "        targets:\n",
            "          - { id: alpha, adapter: rust, path: src/a.rs, selector: { kind: symbol, names: [alpha] } }\n",
            "          - { id: beta, adapter: rust, path: src/b.rs, selector: { kind: symbol, names: [beta] } }\n",
            "        satisfies: [REQ-SAMPLE-MULTI#criterion.multi]\n",
        ),
    )
    .unwrap();
    fs::write(temp.path().join("src/a.rs"), "fn alpha() {}\n").unwrap();
    fs::write(temp.path().join("src/b.rs"), "fn beta() {}\n").unwrap();
    fs::write(
        temp.path().join("tests/check.rs"),
        "fn check_behavior() {}\n",
    )
    .unwrap();
    let request = temp.path().join("work.yaml");
    fs::write(
        &request,
        concat!(
            "schema: syu/work-request/v1\n",
            "id: WORK-MULTI-001\n",
            "summary: Split a multi-file change.\n",
            "operation: modify\n",
            "seeds: [REQ-SAMPLE-MULTI#criterion.multi]\n",
            "constraints: { include_facets: [], exclude_paths: [], max_slices: 4 }\n",
        ),
    )
    .unwrap();
    init_workspace_repo(temp.path());
    let plan = temp.path().join("plan.yaml");
    Command::cargo_bin("syu")
        .unwrap()
        .args(["work", "plan", "--request"])
        .arg(&request)
        .args(["--out"])
        .arg(&plan)
        .args(["--workspace"])
        .arg(temp.path())
        .assert()
        .success();
    fs::write(
        temp.path().join("src/a.rs"),
        "fn alpha() { let value = 1; }\n",
    )
    .unwrap();
    let output = Command::cargo_bin("syu")
        .unwrap()
        .args(["validate"])
        .arg(temp.path())
        .args(["--plan"])
        .arg(&plan)
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(
        String::from_utf8(output.stdout)
            .unwrap()
            .contains("post-state validation requires --slice")
    );
}

#[test]
fn work_plan_requires_clean_governed_workspace() {
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
            "    max_total_bytes: 4096\n",
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
            "  - id: REQ-SAMPLE-CLEAN\n",
            "    title: Sample\n",
            "    description: Sample requirement.\n",
            "    priority: medium\n",
            "    status: implemented\n",
            "    criteria:\n",
            "      - id: change\n",
            "        kind: behavior\n",
            "        statement: Keep behavior aligned.\n",
            "        governed_by: []\n",
        ),
    )
    .unwrap();
    fs::write(
        temp.path().join("spec/feature.yaml"),
        concat!(
            "schema: syu/spec/v1\n",
            "kind: features\n",
            "namespace: sample\n",
            "category: Sample\n",
            "features:\n",
            "  - id: FEAT-SAMPLE-CLEAN\n",
            "    title: Sample\n",
            "    summary: Sample feature.\n",
            "    status: implemented\n",
            "    bindings:\n",
            "      - id: app\n",
            "        role: implementation\n",
            "        facet: backend\n",
            "        responsibility: Update the function body.\n",
            "        targets:\n",
            "          - { id: app, adapter: rust, path: src/app.rs, selector: { kind: symbol, names: [app] } }\n",
            "        satisfies: [REQ-SAMPLE-CLEAN#criterion.change]\n",
        ),
    )
    .unwrap();
    fs::write(temp.path().join("src/app.rs"), "fn app() {}\n").unwrap();
    let request = temp.path().join("work.yaml");
    fs::write(
        &request,
        concat!(
            "schema: syu/work-request/v1\n",
            "id: WORK-CLEAN-001\n",
            "summary: Plan from a clean workspace.\n",
            "operation: modify\n",
            "seeds: [REQ-SAMPLE-CLEAN#criterion.change]\n",
            "constraints: { include_facets: [], exclude_paths: [], max_slices: 1 }\n",
        ),
    )
    .unwrap();
    init_workspace_repo(temp.path());
    fs::write(
        temp.path().join("spec/requirement.yaml"),
        fs::read_to_string(temp.path().join("spec/requirement.yaml"))
            .unwrap()
            .replace("Keep behavior aligned.", "Changed before planning."),
    )
    .unwrap();
    let output = Command::cargo_bin("syu")
        .unwrap()
        .args(["work", "plan", "--request"])
        .arg(&request)
        .args(["--out"])
        .arg(temp.path().join("plan.yaml"))
        .args(["--workspace"])
        .arg(temp.path())
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stderr.contains("clean governed workspace") || stdout.contains("clean governed workspace")
    );
}

#[test]
fn verification_budget_overflow_blocks_instead_of_splitting_closure() {
    let temp = tempdir().unwrap();
    fs::create_dir_all(temp.path().join("spec")).unwrap();
    fs::create_dir_all(temp.path().join("src")).unwrap();
    fs::create_dir_all(temp.path().join("tests")).unwrap();
    fs::write(
        temp.path().join("syu.yaml"),
        concat!(
            "schema: syu/config/v1\n",
            "workspace:\n",
            "  spec_roots: [spec]\n",
            "  artifact_roots: [src, tests]\n",
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
            "    max_editable_symbols: 4\n",
            "    max_verification_targets: 1\n",
            "    max_readonly_targets: 4\n",
            "    max_total_bytes: 4096\n",
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
            "  - id: REQ-SAMPLE-020\n",
            "    title: Sample\n",
            "    description: Sample requirement.\n",
            "    priority: medium\n",
            "    status: planned\n",
            "    criteria:\n",
            "      - id: dual\n",
            "        kind: behavior\n",
            "        statement: Keep both checks green.\n",
            "        governed_by: []\n",
            "    bindings:\n",
            "      - id: check-one\n",
            "        role: verification\n",
            "        facet: verification\n",
            "        responsibility: First verification.\n",
            "        targets:\n",
            "          - { id: one, adapter: rust, path: tests/check_one.rs, selector: { kind: symbol, names: [check_one] } }\n",
            "        verifies: [REQ-SAMPLE-020#criterion.dual]\n",
            "      - id: check-two\n",
            "        role: verification\n",
            "        facet: verification\n",
            "        responsibility: Second verification.\n",
            "        targets:\n",
            "          - { id: two, adapter: rust, path: tests/check_two.rs, selector: { kind: symbol, names: [check_two] } }\n",
            "        verifies: [REQ-SAMPLE-020#criterion.dual]\n",
        ),
    )
    .unwrap();
    fs::write(
        temp.path().join("spec/feature.yaml"),
        concat!(
            "schema: syu/spec/v1\n",
            "kind: features\n",
            "namespace: sample\n",
            "category: Sample\n",
            "features:\n",
            "  - id: FEAT-SAMPLE-020\n",
            "    title: Sample\n",
            "    summary: One implementation.\n",
            "    status: implemented\n",
            "    bindings:\n",
            "      - id: app\n",
            "        role: implementation\n",
            "        facet: backend\n",
            "        responsibility: Update one handler.\n",
            "        targets:\n",
            "          - { id: handler, adapter: rust, path: src/lib.rs, selector: { kind: symbol, names: [handler] } }\n",
            "        satisfies: [REQ-SAMPLE-020#criterion.dual]\n",
        ),
    )
    .unwrap();
    fs::write(temp.path().join("src/lib.rs"), "fn handler() {}\n").unwrap();
    fs::write(
        temp.path().join("tests/check_one.rs"),
        "fn check_one() {}\n",
    )
    .unwrap();
    fs::write(
        temp.path().join("tests/check_two.rs"),
        "fn check_two() {}\n",
    )
    .unwrap();
    let request = temp.path().join("work.yaml");
    fs::write(
        &request,
        concat!(
            "schema: syu/work-request/v1\n",
            "id: WORK-SPLIT-VERIFY\n",
            "summary: Keep verification closure intact.\n",
            "operation: modify\n",
            "seeds: [REQ-SAMPLE-020#criterion.dual]\n",
            "constraints: { include_facets: [], exclude_paths: [], max_slices: 4 }\n",
        ),
    )
    .unwrap();
    init_workspace_repo(temp.path());
    let plan = temp.path().join("plan.yaml");
    Command::cargo_bin("syu")
        .unwrap()
        .args(["work", "plan", "--request"])
        .arg(&request)
        .args(["--out"])
        .arg(&plan)
        .args(["--workspace"])
        .arg(temp.path())
        .assert()
        .failure();
    let text = fs::read_to_string(plan).unwrap();
    assert!(text.contains("status: blocked"));
    assert!(!text.contains("part01"));
}

#[test]
fn split_respects_max_slices_constraint() {
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
            "    max_editable_symbols: 4\n",
            "    max_verification_targets: 1\n",
            "    max_readonly_targets: 1\n",
            "    max_total_bytes: 4096\n",
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
            "  - id: REQ-SAMPLE-002\n",
            "    title: Sample\n",
            "    description: Multi-file implementation.\n",
            "    priority: medium\n",
            "    status: planned\n",
            "    criteria:\n",
            "      - id: multi\n",
            "        kind: behavior\n",
            "        statement: Keep behavior aligned across files.\n",
            "        governed_by: []\n",
        ),
    )
    .unwrap();
    fs::write(
        temp.path().join("spec/feature.yaml"),
        concat!(
            "schema: syu/spec/v1\n",
            "kind: features\n",
            "namespace: sample\n",
            "category: Sample\n",
            "features:\n",
            "  - id: FEAT-SAMPLE-002\n",
            "    title: Sample\n",
            "    summary: Multi-file implementation.\n",
            "    status: implemented\n",
            "    bindings:\n",
            "      - id: app\n",
            "        role: implementation\n",
            "        facet: backend\n",
            "        responsibility: Update both files.\n",
            "        targets:\n",
            "          - { id: alpha, adapter: rust, path: src/a.rs, selector: { kind: symbol, names: [alpha] } }\n",
            "          - { id: beta, adapter: rust, path: src/b.rs, selector: { kind: symbol, names: [beta] } }\n",
            "        satisfies: [REQ-SAMPLE-002#criterion.multi]\n",
        ),
    )
    .unwrap();
    fs::write(temp.path().join("src/a.rs"), "fn alpha() {}\n").unwrap();
    fs::write(temp.path().join("src/b.rs"), "fn beta() {}\n").unwrap();
    let request = temp.path().join("work.yaml");
    fs::write(
        &request,
        concat!(
            "schema: syu/work-request/v1\n",
            "id: WORK-SPLIT-002\n",
            "summary: Split but cap slice count.\n",
            "operation: modify\n",
            "seeds: [REQ-SAMPLE-002#criterion.multi]\n",
            "constraints: { include_facets: [], exclude_paths: [], max_slices: 1 }\n",
        ),
    )
    .unwrap();
    init_workspace_repo(temp.path());
    let plan = temp.path().join("plan.yaml");
    Command::cargo_bin("syu")
        .unwrap()
        .args(["work", "plan", "--request"])
        .arg(&request)
        .args(["--out"])
        .arg(&plan)
        .args(["--workspace"])
        .arg(temp.path())
        .assert()
        .failure();
    let text = fs::read_to_string(plan).unwrap();
    assert!(text.contains("status: blocked"));
    assert!(text.contains("exceed requested maximum 1"));
}

#[test]
fn exact_documentation_target_builds_document_slice() {
    let temp = tempdir().unwrap();
    fs::create_dir_all(temp.path().join("spec")).unwrap();
    fs::create_dir_all(temp.path().join("src")).unwrap();
    fs::create_dir_all(temp.path().join("docs")).unwrap();
    fs::write(
        temp.path().join("syu.yaml"),
        concat!(
            "schema: syu/config/v1\n",
            "workspace:\n",
            "  spec_roots: [spec]\n",
            "  artifact_roots: [src, docs]\n",
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
            "    max_editable_files: 2\n",
            "    max_editable_symbols: 4\n",
            "    max_verification_targets: 1\n",
            "    max_readonly_targets: 4\n",
            "    max_total_bytes: 4096\n",
            "  context:\n",
            "    include_parent_principles: false\n",
            "    include_parent_rules: false\n",
            "adapters: { enabled: [rust, markdown] }\n",
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
            "  - id: REQ-DOC-001\n",
            "    title: Docs\n",
            "    description: Documentation is first-class.\n",
            "    priority: medium\n",
            "    status: planned\n",
            "    criteria:\n",
            "      - id: explain\n",
            "        kind: behavior\n",
            "        statement: Users can read the architecture note.\n",
            "        governed_by: []\n",
        ),
    )
    .unwrap();
    fs::write(
        temp.path().join("spec/feature.yaml"),
        concat!(
            "schema: syu/spec/v1\n",
            "kind: features\n",
            "namespace: sample\n",
            "category: Sample\n",
            "features:\n",
            "  - id: FEAT-DOC-001\n",
            "    title: Docs\n",
            "    summary: Documentation is first-class.\n",
            "    status: implemented\n",
            "    bindings:\n",
            "      - id: impl\n",
            "        role: implementation\n",
            "        facet: backend\n",
            "        responsibility: Provide runtime behavior.\n",
            "        targets:\n",
            "          - { id: code, adapter: rust, path: src/lib.rs, selector: { kind: symbol, names: [alpha] } }\n",
            "        satisfies: [REQ-DOC-001#criterion.explain]\n",
            "      - id: guide\n",
            "        role: documentation\n",
            "        facet: docs\n",
            "        responsibility: Describe the architecture.\n",
            "        targets:\n",
            "          - { id: architecture, adapter: markdown, path: docs/guide.md, selector: { kind: heading, value: Architecture } }\n",
            "        documents: [REQ-DOC-001#criterion.explain]\n",
        ),
    )
    .unwrap();
    fs::write(temp.path().join("src/lib.rs"), "fn alpha() {}\n").unwrap();
    fs::write(
        temp.path().join("docs/guide.md"),
        "# Architecture\n\nDetails.\n",
    )
    .unwrap();
    let request = temp.path().join("work.yaml");
    fs::write(
        &request,
        concat!(
            "schema: syu/work-request/v1\n",
            "id: WORK-DOC-001\n",
            "summary: Update the documentation.\n",
            "operation: document\n",
            "seeds: []\n",
            "requested_targets:\n",
            "  - ref: FEAT-DOC-001#binding.guide/target.architecture\n",
            "    transition: readonly\n",
            "constraints: { include_facets: [], exclude_paths: [], max_slices: 2 }\n",
        ),
    )
    .unwrap();
    init_workspace_repo(temp.path());
    let plan = temp.path().join("plan.yaml");
    Command::cargo_bin("syu")
        .unwrap()
        .args(["work", "plan", "--request"])
        .arg(&request)
        .args(["--out"])
        .arg(&plan)
        .args(["--workspace"])
        .arg(temp.path())
        .assert()
        .success();
    let text = fs::read_to_string(plan).unwrap();
    assert!(text.contains("status: ready"));
    assert!(text.contains("resolved_path: docs/guide.md"));
    assert!(text.contains("kind: diff-within-scope"));
    assert!(text.contains("no-code-drift"));
}

#[test]
fn plan_blocks_when_context_pack_serialized_budget_would_overflow() {
    let temp = tempdir().unwrap();
    fs::create_dir_all(temp.path().join("spec")).unwrap();
    fs::create_dir_all(temp.path().join("src")).unwrap();
    fs::create_dir_all(temp.path().join("docs")).unwrap();
    fs::write(
        temp.path().join("syu.yaml"),
        concat!(
            "schema: syu/config/v1\n",
            "workspace:\n",
            "  spec_roots: [spec]\n",
            "  artifact_roots: [src, docs]\n",
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
            "    max_editable_files: 2\n",
            "    max_editable_symbols: 2\n",
            "    max_verification_targets: 1\n",
            "    max_readonly_targets: 2\n",
            "    max_total_bytes: 256\n",
            "  context:\n",
            "    include_parent_principles: false\n",
            "    include_parent_rules: false\n",
            "adapters: { enabled: [rust, markdown] }\n",
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
            "  - id: REQ-DOC-BUDGET-001\n",
            "    title: Docs\n",
            "    description: Documentation is first-class.\n",
            "    priority: medium\n",
            "    status: planned\n",
            "    criteria:\n",
            "      - id: explain\n",
            "        kind: behavior\n",
            "        statement: Users can read the architecture note.\n",
            "        governed_by: []\n",
        ),
    )
    .unwrap();
    fs::write(
        temp.path().join("spec/feature.yaml"),
        concat!(
            "schema: syu/spec/v1\n",
            "kind: features\n",
            "namespace: sample\n",
            "category: Sample\n",
            "features:\n",
            "  - id: FEAT-DOC-BUDGET-001\n",
            "    title: Docs\n",
            "    summary: Documentation is first-class.\n",
            "    status: implemented\n",
            "    bindings:\n",
            "      - id: impl\n",
            "        role: implementation\n",
            "        facet: backend\n",
            "        responsibility: Provide runtime behavior.\n",
            "        targets:\n",
            "          - { id: code, adapter: rust, path: src/lib.rs, selector: { kind: symbol, names: [alpha] } }\n",
            "        satisfies: [REQ-DOC-BUDGET-001#criterion.explain]\n",
            "      - id: guide\n",
            "        role: documentation\n",
            "        facet: docs\n",
            "        responsibility: Describe the architecture in detail for humans.\n",
            "        targets:\n",
            "          - { id: architecture, adapter: markdown, path: docs/guide.md, selector: { kind: heading, value: Architecture } }\n",
            "        documents: [REQ-DOC-BUDGET-001#criterion.explain]\n",
        ),
    )
    .unwrap();
    fs::write(temp.path().join("src/lib.rs"), "fn alpha() {}\n").unwrap();
    fs::write(
        temp.path().join("docs/guide.md"),
        "# Architecture\n\nTiny.\n",
    )
    .unwrap();
    let request = temp.path().join("work.yaml");
    let plan = temp.path().join("plan.yaml");
    fs::write(
        &request,
        concat!(
            "schema: syu/work-request/v1\n",
            "id: WORK-DOC-BUDGET-001\n",
            "summary: Update the documentation.\n",
            "operation: document\n",
            "seeds: []\n",
            "requested_targets:\n",
            "  - ref: FEAT-DOC-BUDGET-001#binding.guide/target.architecture\n",
            "    transition: readonly\n",
            "constraints: { include_facets: [], exclude_paths: [], max_slices: 2 }\n",
        ),
    )
    .unwrap();
    init_workspace_repo(temp.path());
    Command::cargo_bin("syu")
        .unwrap()
        .args(["work", "plan", "--request"])
        .arg(&request)
        .args(["--out"])
        .arg(&plan)
        .args(["--workspace"])
        .arg(temp.path())
        .assert()
        .failure();
    let text = fs::read_to_string(plan).unwrap();
    assert!(text.contains("status: blocked"));
    assert!(text.contains("context pack exceeds configured budget"));
}

#[test]
fn refactor_plan_adds_contract_consistency_completion() {
    let temp = tempdir().unwrap();
    let request = temp.path().join("request.yaml");
    let plan = temp.path().join("plan.yaml");
    fs::write(
        &request,
        concat!(
            "schema: syu/work-request/v1\n",
            "id: WORK-REFACTOR-001\n",
            "summary: Refactor login flow.\n",
            "operation: refactor\n",
            "seeds: [REQ-AUTH-001#criterion.invalid-credentials]\n",
            "constraints: { include_facets: [ui, backend], exclude_paths: [], max_slices: 4 }\n",
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
    assert!(text.contains("kind: contract-consistent"));
    assert!(text.contains("kind: diff-within-scope"));
    assert!(text.contains("preserve-behavior"));
}

#[test]
fn workbench_project_exports_canonical_projection() {
    let temp = tempdir().unwrap();
    let request = temp.path().join("request.yaml");
    fs::write(
        &request,
        concat!(
            "schema: syu/work-request/v1\n",
            "id: WORK-WORKBENCH-001\n",
            "summary: Project workbench state.\n",
            "operation: modify\n",
            "seeds: [REQ-AUTH-001#criterion.invalid-credentials]\n",
            "constraints: { include_facets: [ui, backend], exclude_paths: [], max_slices: 4 }\n",
        ),
    )
    .unwrap();
    Command::cargo_bin("syu")
        .unwrap()
        .args([
            "workbench",
            "project",
            "--workspace",
            "fixtures/v1/valid-web-app",
            "--request",
        ])
        .arg(&request)
        .assert()
        .success();
    let output = Command::cargo_bin("syu")
        .unwrap()
        .args([
            "workbench",
            "project",
            "--workspace",
            "fixtures/v1/valid-web-app",
            "--request",
        ])
        .arg(&request)
        .output()
        .unwrap();
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("\"plan\""));
    assert!(text.contains("\"validation\""));
    assert!(text.contains("invalid-credentials"));
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
fn strict_preset_enables_changed_ownership_rules() {
    let temp = tempdir().unwrap();
    fs::create_dir_all(temp.path().join("spec")).unwrap();
    fs::create_dir_all(temp.path().join("src")).unwrap();
    fs::write(
        temp.path().join("spec/foundation.yaml"),
        concat!(
            "schema: syu/spec/v1\n",
            "kind: philosophies\n",
            "namespace: sample\n",
            "category: Sample\n",
            "philosophies:\n",
            "  - id: PHIL-SAMPLE-010\n",
            "    title: Sample\n",
            "    summary: Sample philosophy.\n",
            "    principles:\n",
            "      - { id: governed, statement: Keep ownership explicit., applies_to: [product] }\n",
            "    bindings: []\n",
        ),
    )
    .unwrap();
    fs::write(
        temp.path().join("spec/policy.yaml"),
        concat!(
            "schema: syu/spec/v1\n",
            "kind: policies\n",
            "namespace: sample\n",
            "category: Sample\n",
            "policies:\n",
            "  - id: POL-SAMPLE-010\n",
            "    title: Sample\n",
            "    summary: Sample policy.\n",
            "    description: Sample policy.\n",
            "    rules:\n",
            "      - id: governed\n",
            "        level: should\n",
            "        statement: Keep ownership explicit.\n",
            "        governed_by: [PHIL-SAMPLE-010#principle.governed]\n",
            "    bindings: []\n",
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
            "  - id: REQ-SAMPLE-010\n",
            "    title: Sample\n",
            "    description: Sample requirement.\n",
            "    priority: medium\n",
            "    status: planned\n",
            "    criteria:\n",
            "      - id: check\n",
            "        kind: behavior\n",
            "        statement: Sample criterion.\n",
            "        governed_by: [POL-SAMPLE-010#rule.governed]\n",
        ),
    )
    .unwrap();
    fs::write(temp.path().join("src/orphan.rs"), "fn orphan() {}\n").unwrap();
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
            "    require_owned_changes: true\n",
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
    init_workspace_repo(temp.path());
    fs::write(
        temp.path().join("src/orphan.rs"),
        "fn orphan() { let _x = 1; }\n",
    )
    .unwrap();
    Command::cargo_bin("syu")
        .unwrap()
        .args(["validate"])
        .arg(temp.path())
        .args(["--range", "HEAD"])
        .assert()
        .success();
    fs::write(
        temp.path().join("syu.yaml"),
        fs::read_to_string(temp.path().join("syu.yaml"))
            .unwrap()
            .replacen("preset: standard", "preset: strict", 1),
    )
    .unwrap();
    Command::cargo_bin("syu")
        .unwrap()
        .args(["validate"])
        .arg(temp.path())
        .args(["--range", "HEAD"])
        .assert()
        .failure();
}

#[test]
fn validate_rejects_unknown_rule_overrides() {
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
            "  rules:\n",
            "    UNKNOWN-RULE-001: off\n",
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
            "  - id: REQ-SAMPLE-011\n",
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
fn strict_preset_requires_changed_spec_impact_updates() {
    let temp = tempdir().unwrap();
    fs::create_dir_all(temp.path().join("spec")).unwrap();
    fs::create_dir_all(temp.path().join("src")).unwrap();
    fs::create_dir_all(temp.path().join("tests")).unwrap();
    fs::write(
        temp.path().join("syu.yaml"),
        concat!(
            "schema: syu/config/v1\n",
            "workspace:\n",
            "  spec_roots: [spec]\n",
            "  artifact_roots: [src, tests]\n",
            "  excludes: []\n",
            "profiles: { active: [], custom: {} }\n",
            "validation:\n",
            "  preset: strict\n",
            "  deny_warnings: false\n",
            "  rules: {}\n",
            "  changed:\n",
            "    baseline: { strategy: parent }\n",
            "    require_owned_changes: false\n",
            "work:\n",
            "  slicing:\n",
            "    max_editable_files: 1\n",
            "    max_editable_symbols: 1\n",
            "    max_verification_targets: 2\n",
            "    max_readonly_targets: 2\n",
            "    max_total_bytes: 2048\n",
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
            "  - id: REQ-SAMPLE-030\n",
            "    title: Sample\n",
            "    description: Sample requirement.\n",
            "    priority: high\n",
            "    status: implemented\n",
            "    criteria:\n",
            "      - id: check\n",
            "        kind: behavior\n",
            "        statement: Original criterion.\n",
            "        governed_by: []\n",
            "    bindings:\n",
            "      - id: verify\n",
            "        role: verification\n",
            "        facet: verification\n",
            "        responsibility: Verify behavior.\n",
            "        targets:\n",
            "          - { id: case, adapter: rust, path: tests/check.rs, selector: { kind: symbol, names: [check_behavior] } }\n",
            "        verifies: [REQ-SAMPLE-030#criterion.check]\n",
        ),
    )
    .unwrap();
    fs::write(
        temp.path().join("spec/feature.yaml"),
        concat!(
            "schema: syu/spec/v1\n",
            "kind: features\n",
            "namespace: sample\n",
            "category: Sample\n",
            "features:\n",
            "  - id: FEAT-SAMPLE-030\n",
            "    title: Sample\n",
            "    summary: Sample feature.\n",
            "    status: implemented\n",
            "    bindings:\n",
            "      - id: app\n",
            "        role: implementation\n",
            "        facet: backend\n",
            "        responsibility: Provide behavior.\n",
            "        targets:\n",
            "          - { id: code, adapter: rust, path: src/lib.rs, selector: { kind: symbol, names: [run] } }\n",
            "        satisfies: [REQ-SAMPLE-030#criterion.check]\n",
        ),
    )
    .unwrap();
    fs::write(temp.path().join("src/lib.rs"), "fn run() {}\n").unwrap();
    fs::write(
        temp.path().join("tests/check.rs"),
        "fn check_behavior() {}\n",
    )
    .unwrap();
    init_workspace_repo(temp.path());
    fs::write(
        temp.path().join("spec/requirement.yaml"),
        fs::read_to_string(temp.path().join("spec/requirement.yaml"))
            .unwrap()
            .replace("Original criterion.", "Updated criterion."),
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
fn strict_preset_only_flags_the_changed_criterion_in_a_shared_document() {
    let temp = tempdir().unwrap();
    fs::create_dir_all(temp.path().join("spec")).unwrap();
    fs::create_dir_all(temp.path().join("src")).unwrap();
    fs::create_dir_all(temp.path().join("tests")).unwrap();
    fs::write(
        temp.path().join("syu.yaml"),
        concat!(
            "schema: syu/config/v1\n",
            "workspace:\n",
            "  spec_roots: [spec]\n",
            "  artifact_roots: [src, tests]\n",
            "  excludes: []\n",
            "profiles: { active: [], custom: {} }\n",
            "validation:\n",
            "  preset: strict\n",
            "  deny_warnings: false\n",
            "  rules: {}\n",
            "  changed:\n",
            "    baseline: { strategy: parent }\n",
            "    require_owned_changes: false\n",
            "work:\n",
            "  slicing:\n",
            "    max_editable_files: 2\n",
            "    max_editable_symbols: 2\n",
            "    max_verification_targets: 2\n",
            "    max_readonly_targets: 2\n",
            "    max_total_bytes: 4096\n",
            "  context:\n",
            "    include_parent_principles: false\n",
            "    include_parent_rules: false\n",
            "adapters: { enabled: [rust] }\n",
        ),
    )
    .unwrap();
    fs::write(
        temp.path().join("spec/foundation.yaml"),
        concat!(
            "schema: syu/spec/v1\n",
            "kind: philosophies\n",
            "namespace: sample\n",
            "category: Sample\n",
            "philosophies:\n",
            "  - id: PHIL-SAMPLE-040\n",
            "    title: Sample\n",
            "    summary: Sample philosophy.\n",
            "    principles:\n",
            "      - { id: governed, statement: Keep changes governed., applies_to: [product] }\n",
            "    bindings: []\n",
        ),
    )
    .unwrap();
    fs::write(
        temp.path().join("spec/policy.yaml"),
        concat!(
            "schema: syu/spec/v1\n",
            "kind: policies\n",
            "namespace: sample\n",
            "category: Sample\n",
            "policies:\n",
            "  - id: POL-SAMPLE-040\n",
            "    title: Sample\n",
            "    summary: Sample policy.\n",
            "    description: Sample policy.\n",
            "    rules:\n",
            "      - id: governed\n",
            "        level: should\n",
            "        statement: Keep changes governed.\n",
            "        governed_by: [PHIL-SAMPLE-040#principle.governed]\n",
            "    bindings: []\n",
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
            "  - id: REQ-SAMPLE-040\n",
            "    title: Sample\n",
            "    description: Sample requirement.\n",
            "    priority: high\n",
            "    status: implemented\n",
            "    criteria:\n",
            "      - id: first\n",
            "        kind: behavior\n",
            "        statement: First criterion.\n",
            "        governed_by: [POL-SAMPLE-040#rule.governed]\n",
            "      - id: second\n",
            "        kind: behavior\n",
            "        statement: Second criterion.\n",
            "        governed_by: [POL-SAMPLE-040#rule.governed]\n",
            "    bindings:\n",
            "      - id: verify-first\n",
            "        role: verification\n",
            "        facet: verification\n",
            "        responsibility: Verify first behavior.\n",
            "        targets:\n",
            "          - { id: case, adapter: rust, path: tests/first.rs, selector: { kind: symbol, names: [check_first] } }\n",
            "        verifies: [REQ-SAMPLE-040#criterion.first]\n",
            "      - id: verify-second\n",
            "        role: verification\n",
            "        facet: verification\n",
            "        responsibility: Verify second behavior.\n",
            "        targets:\n",
            "          - { id: case, adapter: rust, path: tests/second.rs, selector: { kind: symbol, names: [check_second] } }\n",
            "        verifies: [REQ-SAMPLE-040#criterion.second]\n",
        ),
    )
    .unwrap();
    fs::write(
        temp.path().join("spec/feature.yaml"),
        concat!(
            "schema: syu/spec/v1\n",
            "kind: features\n",
            "namespace: sample\n",
            "category: Sample\n",
            "features:\n",
            "  - id: FEAT-SAMPLE-040\n",
            "    title: Sample\n",
            "    summary: Sample feature.\n",
            "    status: implemented\n",
            "    bindings:\n",
            "      - id: first\n",
            "        role: implementation\n",
            "        facet: backend\n",
            "        responsibility: Provide first behavior.\n",
            "        targets:\n",
            "          - { id: code, adapter: rust, path: src/first.rs, selector: { kind: symbol, names: [first] } }\n",
            "        satisfies: [REQ-SAMPLE-040#criterion.first]\n",
            "      - id: second\n",
            "        role: implementation\n",
            "        facet: backend\n",
            "        responsibility: Provide second behavior.\n",
            "        targets:\n",
            "          - { id: code, adapter: rust, path: src/second.rs, selector: { kind: symbol, names: [second] } }\n",
            "        satisfies: [REQ-SAMPLE-040#criterion.second]\n",
        ),
    )
    .unwrap();
    fs::write(temp.path().join("src/first.rs"), "fn first() {}\n").unwrap();
    fs::write(temp.path().join("src/second.rs"), "fn second() {}\n").unwrap();
    fs::write(temp.path().join("tests/first.rs"), "fn check_first() {}\n").unwrap();
    fs::write(
        temp.path().join("tests/second.rs"),
        "fn check_second() {}\n",
    )
    .unwrap();
    init_workspace_repo(temp.path());
    fs::write(
        temp.path().join("spec/requirement.yaml"),
        fs::read_to_string(temp.path().join("spec/requirement.yaml"))
            .unwrap()
            .replace("First criterion.", "Updated first criterion."),
    )
    .unwrap();
    let output = Command::cargo_bin("syu")
        .unwrap()
        .args(["validate"])
        .arg(temp.path())
        .args(["--range", "HEAD"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert_eq!(stdout.matches("SYU-CHANGE-003").count(), 1);
    assert!(stdout.contains("REQ-SAMPLE-040#criterion.first"));
    assert!(!stdout.contains("REQ-SAMPLE-040#criterion.second"));
}

#[test]
fn strict_preset_requires_all_contract_participants_to_change() {
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
            "  preset: strict\n",
            "  deny_warnings: false\n",
            "  rules: {}\n",
            "  changed:\n",
            "    baseline: { strategy: parent }\n",
            "    require_owned_changes: false\n",
            "work:\n",
            "  slicing:\n",
            "    max_editable_files: 2\n",
            "    max_editable_symbols: 2\n",
            "    max_verification_targets: 1\n",
            "    max_readonly_targets: 2\n",
            "    max_total_bytes: 4096\n",
            "  context:\n",
            "    include_parent_principles: false\n",
            "    include_parent_rules: false\n",
            "adapters: { enabled: [rust] }\n",
        ),
    )
    .unwrap();
    fs::write(
        temp.path().join("spec/foundation.yaml"),
        concat!(
            "schema: syu/spec/v1\n",
            "kind: philosophies\n",
            "namespace: sample\n",
            "category: Sample\n",
            "philosophies:\n",
            "  - id: PHIL-SAMPLE-050\n",
            "    title: Sample\n",
            "    summary: Sample philosophy.\n",
            "    principles:\n",
            "      - { id: governed, statement: Keep contract changes governed., applies_to: [product] }\n",
            "    bindings: []\n",
        ),
    )
    .unwrap();
    fs::write(
        temp.path().join("spec/policy.yaml"),
        concat!(
            "schema: syu/spec/v1\n",
            "kind: policies\n",
            "namespace: sample\n",
            "category: Sample\n",
            "policies:\n",
            "  - id: POL-SAMPLE-050\n",
            "    title: Sample\n",
            "    summary: Sample policy.\n",
            "    description: Sample policy.\n",
            "    rules:\n",
            "      - id: governed\n",
            "        level: should\n",
            "        statement: Keep contract changes governed.\n",
            "        governed_by: [PHIL-SAMPLE-050#principle.governed]\n",
            "    bindings: []\n",
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
            "  - id: REQ-SAMPLE-050\n",
            "    title: Sample\n",
            "    description: Sample requirement.\n",
            "    priority: high\n",
            "    status: implemented\n",
            "    criteria:\n",
            "      - id: bridge\n",
            "        kind: behavior\n",
            "        statement: Keep both sides aligned.\n",
            "        governed_by: [POL-SAMPLE-050#rule.governed]\n",
            "    bindings:\n",
            "      - id: verify\n",
            "        role: verification\n",
            "        facet: verification\n",
            "        responsibility: Verify both sides stay aligned.\n",
            "        targets:\n",
            "          - { id: case, adapter: rust, path: src/check.rs, selector: { kind: symbol, names: [check_bridge] } }\n",
            "        verifies: [REQ-SAMPLE-050#criterion.bridge]\n",
        ),
    )
    .unwrap();
    fs::write(
        temp.path().join("spec/feature.yaml"),
        concat!(
            "schema: syu/spec/v1\n",
            "kind: features\n",
            "namespace: sample\n",
            "category: Sample\n",
            "features:\n",
            "  - id: FEAT-SAMPLE-050\n",
            "    title: Sample\n",
            "    summary: Sample feature.\n",
            "    status: implemented\n",
            "    bindings:\n",
            "      - id: ui\n",
            "        role: implementation\n",
            "        facet: ui\n",
            "        responsibility: UI side.\n",
            "        targets:\n",
            "          - { id: code, adapter: rust, path: src/ui.rs, selector: { kind: symbol, names: [ui_side] } }\n",
            "        satisfies: [REQ-SAMPLE-050#criterion.bridge]\n",
            "      - id: backend\n",
            "        role: implementation\n",
            "        facet: backend\n",
            "        responsibility: Backend side.\n",
            "        targets:\n",
            "          - { id: code, adapter: rust, path: src/backend.rs, selector: { kind: symbol, names: [backend_side] } }\n",
            "        satisfies: [REQ-SAMPLE-050#criterion.bridge]\n",
            "      - id: schema\n",
            "        role: contract-source\n",
            "        facet: api\n",
            "        responsibility: Contract source.\n",
            "        targets:\n",
            "          - { id: code, adapter: rust, path: src/schema.rs, selector: { kind: symbol, names: [schema_side] } }\n",
            "    contracts:\n",
            "      - id: bridge-http\n",
            "        kind: http\n",
            "        source: FEAT-SAMPLE-050#binding.schema/target.code\n",
            "        participants:\n",
            "          - { binding: FEAT-SAMPLE-050#binding.backend, role: provider }\n",
            "          - { binding: FEAT-SAMPLE-050#binding.ui, role: consumer }\n",
        ),
    )
    .unwrap();
    fs::write(temp.path().join("src/ui.rs"), "fn ui_side() {}\n").unwrap();
    fs::write(temp.path().join("src/backend.rs"), "fn backend_side() {}\n").unwrap();
    fs::write(temp.path().join("src/schema.rs"), "fn schema_side() {}\n").unwrap();
    fs::write(temp.path().join("src/check.rs"), "fn check_bridge() {}\n").unwrap();
    init_workspace_repo(temp.path());
    fs::write(
        temp.path().join("spec/feature.yaml"),
        fs::read_to_string(temp.path().join("spec/feature.yaml"))
            .unwrap()
            .replace("role: consumer", "role: client"),
    )
    .unwrap();
    fs::write(
        temp.path().join("src/backend.rs"),
        "fn backend_side() { let changed = 1; assert_eq!(changed, 1); }\n",
    )
    .unwrap();
    let output = Command::cargo_bin("syu")
        .unwrap()
        .args(["validate"])
        .arg(temp.path())
        .args(["--range", "HEAD"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("SYU-CHANGE-004"));
    assert!(stdout.contains("FEAT-SAMPLE-050#contract.bridge-http"));
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
