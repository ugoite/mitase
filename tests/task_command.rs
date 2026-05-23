// REQ-CORE-028
// REQ-CORE-029
// REQ-CORE-030
// REQ-CORE-031
// REQ-CORE-032

use std::{fs, path::Path, process::Command};

use assert_cmd::cargo::CommandCargoExt;
use tempfile::tempdir;

fn write_workspace(root: &std::path::Path) {
    fs::write(
        root.join("syu.yaml"),
        "version: 1\nspec:\n  root: docs/syu\n",
    )
    .expect("workspace config");
    fs::create_dir_all(root.join("docs/syu/philosophy")).expect("philosophy dir");
    fs::create_dir_all(root.join("docs/syu/policies")).expect("policy dir");
    fs::create_dir_all(root.join("docs/syu/requirements/core")).expect("requirements dir");
    fs::create_dir_all(root.join("docs/syu/features/core")).expect("features dir");

    fs::write(
        root.join("docs/syu/philosophy/foundation.yaml"),
        "category: Philosophy\nversion: 1\nlanguage: en\nphilosophies:\n  - id: PHIL-001\n    title: Keep planning explicit\n    product_design_principle: Request artifacts should stay reviewable.\n    coding_guideline: Prefer explicit request classification.\n    linked_policies:\n      - POL-001\n",
    )
    .expect("philosophy doc");
    fs::write(
        root.join("docs/syu/policies/policies.yaml"),
        "category: Policies\nversion: 1\nlanguage: en\npolicies:\n  - id: POL-001\n    title: Keep request workflows visible\n    summary: Keep intake and planning separate.\n    description: Request artifacts should be classified against the current graph.\n    linked_philosophies:\n      - PHIL-001\n    linked_requirements:\n      - REQ-CORE-028\n      - REQ-CORE-029\n      - REQ-CORE-030\n      - REQ-CORE-031\n      - REQ-CORE-032\n",
    )
    .expect("policy doc");
    fs::write(
        root.join("docs/syu/requirements/core/classify.yaml"),
        "category: Core Workspace\nprefix: REQ-CORE\nrequirements:\n  - id: REQ-CORE-028\n    title: Classify request artifacts into requirement actions\n    description: The task classifier should decide whether a request creates, changes, or deletes a requirement.\n    priority: medium\n    status: implemented\n    linked_policies:\n      - POL-001\n    linked_features:\n      - FEAT-TASK-001\n    tests:\n      rust:\n        - file: src/command/task.rs\n          symbols:\n            - '*'\n",
    )
    .expect("requirement doc");
    fs::write(
        root.join("docs/syu/requirements/core/scaffold.yaml"),
        "category: Core Workspace\nprefix: REQ-CORE\nrequirements:\n  - id: REQ-CORE-029\n    title: Scaffold planned requirement and feature updates from task planning\n    description: The scaffold command should turn request planning results into reviewable planned requirement and feature updates.\n    priority: medium\n    status: implemented\n    linked_policies:\n      - POL-001\n    linked_features:\n      - FEAT-TASK-002\n    tests:\n      rust:\n        - file: src/command/task.rs\n          symbols:\n            - '*'\n",
    )
    .expect("scaffold requirement doc");
    fs::write(
        root.join("docs/syu/requirements/core/scope.yaml"),
        "category: Core Workspace\nprefix: REQ-CORE\nrequirements:\n  - id: REQ-CORE-030\n    title: Scope requests against requirements, policies, philosophies, and features\n    description: The task scope command should map a request artifact onto nearby spec items before planning starts.\n    priority: medium\n    status: implemented\n    linked_policies:\n      - POL-001\n    linked_features:\n      - FEAT-TASK-003\n    tests:\n      rust:\n        - file: src/command/task.rs\n          symbols:\n            - '*'\n",
    )
    .expect("scope requirement doc");
    fs::write(
        root.join("docs/syu/requirements/core/plan.yaml"),
        "category: Core Workspace\nprefix: REQ-CORE\nrequirements:\n  - id: REQ-CORE-031\n    title: Generate temporary Goal Plans from scoped requests\n    description: The task plan command should turn a scoped request artifact into a temporary Goal Plan while keeping persistent spec files untouched.\n    priority: medium\n    status: implemented\n    linked_policies:\n      - POL-001\n    linked_features:\n      - FEAT-TASK-004\n    tests:\n      rust:\n        - file: src/command/task.rs\n          symbols:\n            - '*'\n",
    )
    .expect("plan requirement doc");
    fs::write(
        root.join("docs/syu/requirements/core/check.yaml"),
        "category: Core Workspace\nprefix: REQ-CORE\nrequirements:\n  - id: REQ-CORE-032\n    title: Validate temporary Goal Plans against the current spec graph and git range\n    description: The task check command should validate Goal Plan conformance against changed files, linked spec IDs, required tests, and completion commands.\n    priority: medium\n    status: implemented\n    linked_policies:\n      - POL-001\n    linked_features:\n      - FEAT-TASK-005\n    tests:\n      rust:\n        - file: tests/task_command.rs\n          symbols:\n            - task_check_reports_pass_fail_results_for_goal_plans\n",
    )
    .expect("check requirement doc");
    fs::write(
        root.join("docs/syu/features/features.yaml"),
        "version: 1\nupdated: \"2026-05\"\nfiles:\n  - kind: task\n    file: core/task.yaml\n  - kind: task\n    file: core/scaffold.yaml\n  - kind: task\n    file: core/scope.yaml\n  - kind: task\n    file: core/plan.yaml\n",
    )
    .expect("feature registry");
    fs::write(
        root.join("docs/syu/features/core/task.yaml"),
        "category: Task Planning CLI\nversion: 1\nfeatures:\n  - id: FEAT-TASK-001\n    title: Request artifact classification\n    summary: Classify captured request artifacts into create, change, or delete decisions using the current spec graph, with a short explanation and text or JSON output.\n    status: implemented\n    linked_requirements:\n      - REQ-CORE-028\n    implementations:\n      rust:\n        - file: src/command/task.rs\n          symbols:\n            - run_task_command\n            - run_task_classify_command\n  - id: FEAT-TASK-003\n    title: Request artifact scoping\n    summary: Map request artifacts onto candidate requirements, policies, philosophies, and features before planning begins.\n    status: implemented\n    linked_requirements:\n      - REQ-CORE-030\n    implementations:\n      rust:\n        - file: src/command/task.rs\n          symbols:\n            - run_task_command\n            - run_task_scope_command\n  - id: FEAT-TASK-005\n    title: Goal Plan conformance checking\n    summary: Validate temporary Goal Plan artifacts against changed files, linked spec IDs, required tests, and declared completion commands before review.\n    status: implemented\n    linked_requirements:\n      - REQ-CORE-032\n    implementations:\n      rust:\n        - file: src/command/task.rs\n          symbols:\n            - run_task_command\n            - run_task_check_command\n            - load_goal_plan_artifact\n        - file: src/cli.rs\n          symbols:\n            - TaskArgs\n            - TaskCheckArgs\n        - file: src/lib.rs\n          symbols:\n            - dispatch\n            - run_dispatch\n",
    )
    .expect("feature doc");
    fs::write(
        root.join("docs/syu/features/core/plan.yaml"),
        "category: Core Workspace\nversion: 1\nfeatures:\n  - id: FEAT-TASK-004\n    title: Goal Plan generation\n    summary: Turn scoped request artifacts into temporary Goal Plans with implementation, test, coverage, and completion sections outside the persistent spec tree.\n    status: implemented\n    linked_requirements:\n      - REQ-CORE-031\n    implementations:\n      rust:\n        - file: src/command/task.rs\n          symbols:\n            - run_task_command\n            - run_task_plan_command\n",
    )
    .expect("plan feature doc");
    fs::write(
        root.join("docs/syu/features/core/scaffold.yaml"),
        "category: Core Workspace\nversion: 1\nfeatures:\n  - id: FEAT-TASK-002\n    title: Planned task scaffold preview\n    summary: Preview reviewable planned requirement and feature updates that follow the existing add and registry conventions.\n    status: implemented\n    linked_requirements:\n      - REQ-CORE-029\n    implementations:\n      rust:\n        - file: src/command/task.rs\n          symbols:\n            - run_task_command\n            - run_task_scaffold_command\n",
    )
    .expect("scaffold feature doc");
    fs::write(
        root.join("docs/syu/features/core/scope.yaml"),
        "category: Core Workspace\nversion: 1\nfeatures:\n  - id: FEAT-TASK-003\n    title: Request artifact scoping\n    summary: Map request artifacts onto candidate requirements, policies, philosophies, and features before planning begins.\n    status: implemented\n    linked_requirements:\n      - REQ-CORE-030\n    implementations:\n      rust:\n        - file: src/command/task.rs\n          symbols:\n            - run_task_command\n            - run_task_scope_command\n",
    )
    .expect("scope feature doc");
}

fn write_request(root: &std::path::Path, request: &str, affected_area: &str, linked_ids: &[&str]) {
    let linked_ids_block = if linked_ids.is_empty() {
        "  linked_ids: []\n".to_string()
    } else {
        let list = linked_ids
            .iter()
            .map(|id| format!("    - {id}\n"))
            .collect::<String>();
        format!("  linked_ids:\n{list}")
    };
    fs::write(
        root.join("request.yaml"),
        format!(
            "version: 1\nrequest: >\n  {request}\ncontext:\n  affected_area: {affected_area}\n  repository_constraints:\n    - keep text and JSON output\n{linked_ids_block}",
        ),
    )
    .expect("request");
}

fn git_output(root: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .current_dir(root)
        .args(args)
        .output()
        .expect("git command should run");
    assert!(output.status.success(), "git {:?} failed", args);
    String::from_utf8(output.stdout)
        .expect("git output should be valid utf-8")
        .trim()
        .to_string()
}

fn git(root: &Path, args: &[&str]) {
    let status = Command::new("git")
        .current_dir(root)
        .args(args)
        .status()
        .expect("git command should run");
    assert!(status.success(), "git {:?} failed", args);
}

fn init_git_repo(root: &Path) {
    git(root, &["init", "-b", "main"]);
    git(root, &["config", "user.email", "syu@example.com"]);
    git(root, &["config", "user.name", "syu"]);
}

fn commit_all(root: &Path, message: &str) {
    git(root, &["add", "-A"]);
    git(root, &["commit", "--quiet", "-m", message]);
}

fn write_goal_plan(path: &Path, contents: &str) {
    fs::write(path, contents).expect("goal plan");
}

fn goal_plan_yaml(
    linked_requirement: &str,
    linked_feature: &str,
    test_file: &str,
    test_symbol: &str,
    confidence: &str,
) -> String {
    format!(
        "version: 1\nkind: syu.goal_plan\nsource:\n  mode: diff_inferred\n  range: origin/main...HEAD\n  confidence: {confidence}\ngoal:\n  id: GOAL-001\n  title: Keep temporary planning explicit\n  statement: Capture implementation intent without creating a fifth persistent spec layer.\n  non_goals:\n    - Add persistent task specs under spec.root\nspec_mapping:\n  persistent_items:\n    philosophies:\n      - PHIL-001\n    policies:\n      - POL-001\n    requirements:\n      - {linked_requirement}\n    features:\n      - {linked_feature}\n  spec_updates:\n    required: false\n    expected_updates: []\nimplementation_plan:\n  scope:\n    include:\n      - src/command/task.rs\n    exclude:\n      - docs/syu/**\n  steps:\n    - add a Goal Plan model\ntest_plan:\n  selection_mode: affected\n  required_tests:\n    rust:\n      - file: {test_file}\n        symbols:\n          - {test_symbol}\n  suggested_tests: {{}}\ncoverage:\n  mode: changed_lines\n  threshold: 100\n  include:\n    - src/command/task.rs\n  exclude: []\ncompletion:\n  must_pass:\n    - syu validate .\n"
    )
}

fn prepare_git_workspace(root: &Path, test_symbol: &str) {
    write_workspace(root);
    fs::create_dir_all(root.join("src/command")).expect("src dir");
    fs::create_dir_all(root.join("tests")).expect("tests dir");
    fs::write(
        root.join("src/command/task.rs"),
        "pub fn run_task_command() {}\n",
    )
    .expect("task source");
    fs::write(
        root.join("src/command/report.rs"),
        "pub fn run_report_command() {}\n",
    )
    .expect("report source");
    fs::write(
        root.join("tests/task_command.rs"),
        format!("fn {test_symbol}() {{}}\n"),
    )
    .expect("task tests");

    init_git_repo(root);
    commit_all(root, "base");
    let base = git_output(root, &["rev-parse", "HEAD"]);
    git(root, &["update-ref", "refs/remotes/origin/main", &base]);
}

#[test]
fn task_classify_prints_text_output_with_related_items() {
    let tempdir = tempdir().expect("tempdir");
    write_workspace(tempdir.path());
    write_request(
        tempdir.path(),
        "Request artifact classification",
        "core",
        &[],
    );

    let output = {
        let mut command = std::process::Command::cargo_bin("syu").expect("binary should build");
        command.current_dir(tempdir.path());
        command.args(["task", "classify", "request.yaml"]);
        command.output().expect("command should run")
    };

    assert!(output.status.success(), "task classify should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("classification: requirement_create"));
    assert!(stdout.contains("related items:"));
    assert!(stdout.contains("FEAT-TASK-001"));
    assert!(stdout.contains("closest spec graph matches are"));
}

#[test]
fn task_classify_prints_json_output_with_explicit_items() {
    let tempdir = tempdir().expect("tempdir");
    write_workspace(tempdir.path());
    write_request(
        tempdir.path(),
        "Update PHIL-001, POL-001, REQ-CORE-028, and FEAT-TASK-001 together.",
        "core",
        &["PHIL-001", "POL-001", "REQ-CORE-028", "FEAT-TASK-001"],
    );

    let output = {
        let mut command = std::process::Command::cargo_bin("syu").expect("binary should build");
        command.current_dir(tempdir.path());
        command.args(["task", "classify", "request.yaml", "--format", "json"]);
        command.output().expect("command should run")
    };

    assert!(output.status.success(), "task classify should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("\"classification\": \"requirement_change\""));
    assert!(stdout.contains("\"id\": \"PHIL-001\""));
    assert!(stdout.contains("\"id\": \"POL-001\""));
    assert!(stdout.contains("\"id\": \"REQ-CORE-028\""));
    assert!(stdout.contains("\"id\": \"FEAT-TASK-001\""));
}

#[test]
fn task_scope_prints_text_output_with_candidate_requirements_and_flags() {
    let tempdir = tempdir().expect("tempdir");
    write_workspace(tempdir.path());
    write_request(
        tempdir.path(),
        "Update PHIL-001, POL-001, REQ-CORE-028, and FEAT-TASK-001 so the planning flow stays explainable.",
        "core",
        &["PHIL-001", "POL-001", "REQ-CORE-028", "FEAT-TASK-001"],
    );

    let output = {
        let mut command = std::process::Command::cargo_bin("syu").expect("binary should build");
        command.current_dir(tempdir.path());
        command.args(["task", "scope", "request.yaml"]);
        command.output().expect("command should run")
    };

    assert!(output.status.success(), "task scope should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("candidate requirements:"));
    assert!(stdout.contains("candidate features:"));
    assert!(stdout.contains("scope signals:"));
    assert!(stdout.contains("policy discussion likely: yes"));
    assert!(stdout.contains("philosophy discussion likely: yes"));
    assert!(stdout.contains("FEAT-TASK-001"));
    assert!(stdout.contains("candidate feature planned-state updates: yes"));
}

#[test]
fn task_scope_prints_text_output_without_scope_notes_when_no_scope_keywords_match() {
    let tempdir = tempdir().expect("tempdir");
    write_workspace(tempdir.path());
    write_request(
        tempdir.path(),
        "Update REQ-CORE-028 to clarify request intake.",
        "core",
        &["REQ-CORE-028"],
    );

    let output = {
        let mut command = std::process::Command::cargo_bin("syu").expect("binary should build");
        command.current_dir(tempdir.path());
        command.args(["task", "scope", "request.yaml"]);
        command.output().expect("command should run")
    };

    assert!(output.status.success(), "task scope should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("candidate features:\n- none"));
    assert!(stdout.contains("policy discussion likely: no"));
    assert!(stdout.contains("philosophy discussion likely: no"));
    assert!(stdout.contains("candidate feature planned-state updates: no"));
    assert!(!stdout.contains("scope notes:"));
}

#[test]
fn task_scope_prints_keyword_based_notes_without_matched_policy_or_philosophy_items() {
    let tempdir = tempdir().expect("tempdir");
    write_workspace(tempdir.path());
    write_request(
        tempdir.path(),
        "Need approval and ethos updates for the request flow.",
        "core",
        &[],
    );

    let output = {
        let mut command = std::process::Command::cargo_bin("syu").expect("binary should build");
        command.current_dir(tempdir.path());
        command.args(["task", "scope", "request.yaml"]);
        command.output().expect("command should run")
    };

    assert!(output.status.success(), "task scope should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("policy discussion likely: yes"));
    assert!(stdout.contains("philosophy discussion likely: yes"));
    assert!(stdout.contains("request uses policy-oriented language: approval"));
    assert!(stdout.contains("request uses philosophy-oriented language: ethos"));
}

#[test]
fn task_scope_prints_feature_candidates_without_planned_state_updates_for_delete_requests() {
    let tempdir = tempdir().expect("tempdir");
    write_workspace(tempdir.path());
    write_request(
        tempdir.path(),
        "Delete FEAT-TASK-001 from the planning flow.",
        "core",
        &["FEAT-TASK-001"],
    );

    let output = {
        let mut command = std::process::Command::cargo_bin("syu").expect("binary should build");
        command.current_dir(tempdir.path());
        command.args(["task", "scope", "request.yaml"]);
        command.output().expect("command should run")
    };

    assert!(output.status.success(), "task scope should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("candidate features:"));
    assert!(stdout.contains("FEAT-TASK-001"));
    assert!(!stdout.contains("planned-state update suggested"));
}

#[test]
fn task_scope_prints_json_output_with_planning_signals() {
    let tempdir = tempdir().expect("tempdir");
    write_workspace(tempdir.path());
    write_request(
        tempdir.path(),
        "Update PHIL-001, POL-001, REQ-CORE-028, and FEAT-TASK-001 so the planning flow stays explainable.",
        "core",
        &["PHIL-001", "POL-001", "REQ-CORE-028", "FEAT-TASK-001"],
    );

    let output = {
        let mut command = std::process::Command::cargo_bin("syu").expect("binary should build");
        command.current_dir(tempdir.path());
        command.args(["task", "scope", "request.yaml", "--format", "json"]);
        command.output().expect("command should run")
    };

    assert!(output.status.success(), "task scope should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("\"classification\": \"requirement_change\""));
    assert!(stdout.contains("\"signals\": {"));
    assert!(stdout.contains("\"policy_discussion\": true"));
    assert!(stdout.contains("\"philosophy_discussion\": true"));
    assert!(stdout.contains("\"planned_feature_updates\": true"));
    assert!(stdout.contains("\"requirements\": ["));
    assert!(stdout.contains("\"features\": ["));
    assert!(stdout.contains("\"id\": \"REQ-CORE-028\""));
    assert!(stdout.contains("\"id\": \"FEAT-TASK-001\""));
}

#[test]
fn task_plan_prints_text_output_and_writes_goal_plan_file() {
    let tempdir = tempdir().expect("tempdir");
    write_workspace(tempdir.path());
    write_request(
        tempdir.path(),
        "Generate a plan for the current request-driven workflow.",
        "core",
        &["REQ-CORE-030", "FEAT-TASK-003"],
    );

    let output = {
        let mut command = std::process::Command::cargo_bin("syu").expect("binary should build");
        command.current_dir(tempdir.path());
        command.args([
            "task",
            "plan",
            "request.yaml",
            "--output",
            ".syu/tasks/current.yaml",
        ]);
        command.output().expect("command should run")
    };

    assert!(output.status.success(), "task plan should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("wrote goal plan to"));
    let rendered = fs::read_to_string(tempdir.path().join(".syu/tasks/current.yaml"))
        .expect("goal plan should be written");
    assert!(rendered.contains("kind: syu.goal_plan"));
    assert!(rendered.contains("goal:"));
    assert!(rendered.contains("implementation plan:"));
    assert!(rendered.contains("test plan:"));
    assert!(rendered.contains("coverage: changed_lines (threshold 100)"));
    assert!(rendered.contains("completion checks:"));
    assert!(rendered.contains("syu task check .syu/tasks/current.yaml --range origin/main...HEAD"));
}

#[test]
fn task_plan_prints_json_output_with_scope_and_test_sections() {
    let tempdir = tempdir().expect("tempdir");
    write_workspace(tempdir.path());
    write_request(
        tempdir.path(),
        "Generate a plan for the current request-driven workflow.",
        "core",
        &["REQ-CORE-030", "FEAT-TASK-003"],
    );

    let output = {
        let mut command = std::process::Command::cargo_bin("syu").expect("binary should build");
        command.current_dir(tempdir.path());
        command.args(["task", "plan", "request.yaml", "--format", "json"]);
        command.output().expect("command should run")
    };

    assert!(output.status.success(), "task plan should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("\"kind\": \"syu.goal_plan\""));
    assert!(stdout.contains("\"goal\": {"));
    assert!(stdout.contains("\"implementation_plan\": {"));
    assert!(stdout.contains("\"test_plan\": {"));
    assert!(stdout.contains("\"coverage\": {"));
    assert!(stdout.contains("\"completion\": {"));
}

#[test]
fn task_plan_warns_when_output_is_inside_spec_root() {
    let tempdir = tempdir().expect("tempdir");
    write_workspace(tempdir.path());
    write_request(
        tempdir.path(),
        "Generate a plan for the current request-driven workflow.",
        "core",
        &["REQ-CORE-030", "FEAT-TASK-003"],
    );

    let output = {
        let mut command = std::process::Command::cargo_bin("syu").expect("binary should build");
        command.current_dir(tempdir.path());
        command.args([
            "task",
            "plan",
            "request.yaml",
            "--output",
            "docs/syu/plans/current.yaml",
        ]);
        command.output().expect("command should run")
    };

    assert!(output.status.success(), "task plan should succeed");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("warning: task plan output"));
    assert!(stderr.contains("inside spec.root"));
    assert!(tempdir.path().join("docs/syu/plans/current.yaml").exists());
}

#[test]
fn task_scaffold_prints_text_preview_for_new_planned_updates() {
    let tempdir = tempdir().expect("tempdir");
    write_workspace(tempdir.path());
    write_request(
        tempdir.path(),
        "Create a new checkout planning flow for reviewers.",
        "checkout",
        &[],
    );

    let output = {
        let mut command = std::process::Command::cargo_bin("syu").expect("binary should build");
        command.current_dir(tempdir.path());
        command.args(["task", "scaffold", "request.yaml"]);
        command.output().expect("command should run")
    };

    assert!(output.status.success(), "task scaffold should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("planned updates:"));
    assert!(stdout.contains("create requirement docs/syu/requirements/checkout/checkout.yaml"));
    assert!(stdout.contains("create feature docs/syu/features/checkout/checkout.yaml"));
    assert!(stdout.contains("append feature registry docs/syu/features/features.yaml"));
    assert!(stdout.contains("Generated from `syu task scaffold`"));
}

#[test]
fn task_scaffold_prints_json_preview_for_existing_ids() {
    let tempdir = tempdir().expect("tempdir");
    write_workspace(tempdir.path());
    write_request(
        tempdir.path(),
        "Update REQ-CORE-028 and FEAT-TASK-001 so planning remains explainable.",
        "core",
        &["REQ-CORE-028", "FEAT-TASK-001"],
    );

    let output = {
        let mut command = std::process::Command::cargo_bin("syu").expect("binary should build");
        command.current_dir(tempdir.path());
        command.args(["task", "scaffold", "request.yaml", "--format", "json"]);
        command.output().expect("command should run")
    };

    assert!(output.status.success(), "task scaffold should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("\"classification\": \"requirement_change\""));
    assert!(stdout.contains("\"action\": \"update\""));
    assert!(stdout.contains("\"path\": \"docs/syu/requirements/core/classify.yaml\""));
    assert!(stdout.contains("\"path\": \"docs/syu/features/core/task.yaml\""));
}

#[test]
fn task_scope_prints_text_summary_with_candidate_items() {
    let tempdir = tempdir().expect("tempdir");
    write_workspace(tempdir.path());
    write_request(
        tempdir.path(),
        "Scope REQ-CORE-028 with FEAT-TASK-001 and ask whether policy wording needs refinement.",
        "core",
        &["REQ-CORE-028", "FEAT-TASK-001"],
    );

    let output = {
        let mut command = std::process::Command::cargo_bin("syu").expect("binary should build");
        command.current_dir(tempdir.path());
        command.args(["task", "scope", "request.yaml"]);
        command.output().expect("command should run")
    };

    assert!(output.status.success(), "task scope should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("candidate requirements:"));
    assert!(stdout.contains("REQ-CORE-028"));
    assert!(stdout.contains("candidate features:"));
    assert!(stdout.contains("FEAT-TASK-001"));
    assert!(stdout.contains("scope signals:"));
    assert!(stdout.contains("policy discussion likely: yes"));
    assert!(stdout.contains("philosophy discussion likely: no"));
    assert!(stdout.contains("candidate feature planned-state updates: yes"));
}

#[test]
fn task_scope_prints_json_summary_for_candidate_items() {
    let tempdir = tempdir().expect("tempdir");
    write_workspace(tempdir.path());
    write_request(
        tempdir.path(),
        "Scope REQ-CORE-028 with FEAT-TASK-001 and ask whether policy wording needs refinement.",
        "core",
        &["REQ-CORE-028", "FEAT-TASK-001"],
    );

    let output = {
        let mut command = std::process::Command::cargo_bin("syu").expect("binary should build");
        command.current_dir(tempdir.path());
        command.args(["task", "scope", "request.yaml", "--format", "json"]);
        command.output().expect("command should run")
    };

    assert!(output.status.success(), "task scope should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("\"signals\": {"));
    assert!(stdout.contains("\"policy_discussion\": true"));
    assert!(stdout.contains("\"philosophy_discussion\": false"));
    assert!(stdout.contains("\"planned_feature_updates\": true"));
    assert!(stdout.contains("\"requirements\": ["));
    assert!(stdout.contains("\"features\": ["));
}

#[test]
fn task_scaffold_rejects_delete_requests() {
    let tempdir = tempdir().expect("tempdir");
    write_workspace(tempdir.path());
    write_request(
        tempdir.path(),
        "Delete REQ-CORE-028 because the workflow is obsolete.",
        "core",
        &["REQ-CORE-028"],
    );

    let output = {
        let mut command = std::process::Command::cargo_bin("syu").expect("binary should build");
        command.current_dir(tempdir.path());
        command.args(["task", "scaffold", "request.yaml"]);
        command.output().expect("command should run")
    };

    assert!(!output.status.success(), "delete scaffold should fail");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("only supports request artifacts"));
}

#[test]
fn task_check_reports_pass_fail_results_for_goal_plans() {
    let tempdir = tempdir().expect("tempdir");
    prepare_git_workspace(tempdir.path(), "task_plan_generates_goal_from_request");

    fs::write(
        tempdir.path().join("src/command/task.rs"),
        "pub fn run_task_command() {}\n// updated for goal plan check\n",
    )
    .expect("updated task source");
    commit_all(tempdir.path(), "update task");

    let plan = tempdir.path().join("goal-plan.yaml");
    write_goal_plan(
        &plan,
        &goal_plan_yaml(
            "REQ-CORE-031",
            "FEAT-TASK-004",
            "tests/task_command.rs",
            "task_plan_generates_goal_from_request",
            "high",
        ),
    );

    let output = {
        let mut command = Command::cargo_bin("syu").expect("binary should build");
        command.current_dir(tempdir.path());
        command.args([
            "task",
            "check",
            "goal-plan.yaml",
            "--range",
            "origin/main...HEAD",
        ]);
        command.output().expect("command should run")
    };

    assert!(output.status.success(), "task check should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("goal plan: goal-plan.yaml"));
    assert!(stdout.contains("git range: origin/main...HEAD"));
    assert!(stdout.contains("status: passed"));
    assert!(stdout.contains("findings: none"));
}

#[test]
fn task_check_prints_json_output_for_goal_plans() {
    let tempdir = tempdir().expect("tempdir");
    prepare_git_workspace(tempdir.path(), "task_plan_generates_goal_from_request");

    fs::write(
        tempdir.path().join("src/command/task.rs"),
        "pub fn run_task_command() {}\n// updated for goal plan check\n",
    )
    .expect("updated task source");
    commit_all(tempdir.path(), "update task");

    let plan = tempdir.path().join("goal-plan.yaml");
    write_goal_plan(
        &plan,
        &goal_plan_yaml(
            "REQ-CORE-031",
            "FEAT-TASK-004",
            "tests/task_command.rs",
            "task_plan_generates_goal_from_request",
            "high",
        ),
    );

    let output = {
        let mut command = Command::cargo_bin("syu").expect("binary should build");
        command.current_dir(tempdir.path());
        command.args([
            "task",
            "check",
            "goal-plan.yaml",
            "--range",
            "origin/main...HEAD",
            "--format",
            "json",
        ]);
        command.output().expect("command should run")
    };

    assert!(output.status.success(), "task check should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("\"plan_path\": \"goal-plan.yaml\""));
    assert!(stdout.contains("\"range\": \"origin/main...HEAD\""));
    assert!(stdout.contains("\"passed\": true"));
    assert!(stdout.contains("\"issue_count\": 0"));
    assert!(stdout.contains("\"warning_count\": 0"));
    assert!(stdout.contains("\"error_count\": 0"));
}

#[test]
fn task_check_fails_for_out_of_scope_changed_files() {
    let tempdir = tempdir().expect("tempdir");
    prepare_git_workspace(tempdir.path(), "task_plan_generates_goal_from_request");

    fs::write(
        tempdir.path().join("src/command/report.rs"),
        "pub fn run_report_command() {}\n// changed outside scope\n",
    )
    .expect("updated report source");
    commit_all(tempdir.path(), "update report");

    let plan = tempdir.path().join("goal-plan.yaml");
    write_goal_plan(
        &plan,
        &goal_plan_yaml(
            "REQ-CORE-031",
            "FEAT-TASK-004",
            "tests/task_command.rs",
            "task_plan_generates_goal_from_request",
            "high",
        ),
    );

    let output = {
        let mut command = Command::cargo_bin("syu").expect("binary should build");
        command.current_dir(tempdir.path());
        command.args([
            "task",
            "check",
            "goal-plan.yaml",
            "--range",
            "origin/main...HEAD",
        ]);
        command.output().expect("command should run")
    };

    assert!(!output.status.success(), "task check should fail");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("status: failed"));
    assert!(stdout.contains("changed production file is outside the implementation scope"));
    assert!(stdout.contains("src/command/report.rs"));
}

#[test]
fn task_check_reports_unknown_linked_requirement_and_feature_ids() {
    let tempdir = tempdir().expect("tempdir");
    prepare_git_workspace(tempdir.path(), "task_plan_generates_goal_from_request");

    fs::write(
        tempdir.path().join("src/command/task.rs"),
        "pub fn run_task_command() {}\n// updated for unknown-id check\n",
    )
    .expect("updated task source");
    commit_all(tempdir.path(), "update task");

    let plan = tempdir.path().join("goal-plan.yaml");
    write_goal_plan(
        &plan,
        &goal_plan_yaml(
            "REQ-MISSING-001",
            "FEAT-MISSING-001",
            "tests/task_command.rs",
            "task_plan_generates_goal_from_request",
            "high",
        ),
    );

    let output = {
        let mut command = Command::cargo_bin("syu").expect("binary should build");
        command.current_dir(tempdir.path());
        command.args([
            "task",
            "check",
            "goal-plan.yaml",
            "--range",
            "origin/main...HEAD",
        ]);
        command.output().expect("command should run")
    };

    assert!(!output.status.success(), "task check should fail");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("linked persistent spec ID does not exist"));
    assert!(stdout.contains("REQ-MISSING-001"));
    assert!(stdout.contains("FEAT-MISSING-001"));
}

#[test]
fn task_check_reports_missing_required_test_files() {
    let tempdir = tempdir().expect("tempdir");
    prepare_git_workspace(tempdir.path(), "task_plan_generates_goal_from_request");

    fs::write(
        tempdir.path().join("src/command/task.rs"),
        "pub fn run_task_command() {}\n// updated for missing-file check\n",
    )
    .expect("updated task source");
    commit_all(tempdir.path(), "update task");

    let plan = tempdir.path().join("goal-plan.yaml");
    write_goal_plan(
        &plan,
        &goal_plan_yaml(
            "REQ-CORE-031",
            "FEAT-TASK-004",
            "tests/missing.rs",
            "missing_test",
            "high",
        ),
    );

    let output = {
        let mut command = Command::cargo_bin("syu").expect("binary should build");
        command.current_dir(tempdir.path());
        command.args([
            "task",
            "check",
            "goal-plan.yaml",
            "--range",
            "origin/main...HEAD",
        ]);
        command.output().expect("command should run")
    };

    assert!(!output.status.success(), "task check should fail");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("required test file is missing"));
    assert!(stdout.contains("tests/missing.rs"));
}

#[test]
fn task_check_reports_missing_required_test_symbols() {
    let tempdir = tempdir().expect("tempdir");
    prepare_git_workspace(tempdir.path(), "task_plan_generates_goal_from_request");

    fs::write(
        tempdir.path().join("src/command/task.rs"),
        "pub fn run_task_command() {}\n// updated for missing-symbol check\n",
    )
    .expect("updated task source");
    commit_all(tempdir.path(), "update task");

    let plan = tempdir.path().join("goal-plan.yaml");
    fs::write(
        tempdir.path().join("tests/task_command.rs"),
        "fn actual_test_name() {}\n// task_plan_missing_symbol\n",
    )
    .expect("updated task tests");
    write_goal_plan(
        &plan,
        &goal_plan_yaml(
            "REQ-CORE-031",
            "FEAT-TASK-004",
            "tests/task_command.rs",
            "task_plan_missing_symbol",
            "high",
        ),
    );

    let output = {
        let mut command = Command::cargo_bin("syu").expect("binary should build");
        command.current_dir(tempdir.path());
        command.args([
            "task",
            "check",
            "goal-plan.yaml",
            "--range",
            "origin/main...HEAD",
        ]);
        command.output().expect("command should run")
    };

    assert!(!output.status.success(), "task check should fail");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("required test symbol is missing"));
    assert!(stdout.contains("task_plan_missing_symbol"));
}

// REQ-CORE-031
#[test]
fn task_check_rejects_empty_required_test_symbols() {
    let tempdir = tempdir().expect("tempdir");
    prepare_git_workspace(tempdir.path(), "task_plan_generates_goal_from_request");

    fs::write(
        tempdir.path().join("src/command/task.rs"),
        "pub fn run_task_command() {}\n// updated for empty-symbol check\n",
    )
    .expect("updated task source");
    commit_all(tempdir.path(), "update task");

    let plan = tempdir.path().join("goal-plan.yaml");
    write_goal_plan(
        &plan,
        "version: 1\nkind: syu.goal_plan\nsource:\n  mode: diff_inferred\n  range: origin/main...HEAD\n  confidence: high\ngoal:\n  id: GOAL-001\n  title: Keep temporary planning explicit\n  statement: Capture implementation intent without creating a fifth persistent spec layer.\nspec_mapping:\n  persistent_items:\n    philosophies:\n      - PHIL-001\n    policies:\n      - POL-001\n    requirements:\n      - REQ-CORE-031\n    features:\n      - FEAT-TASK-004\n  spec_updates:\n    required: false\n    expected_updates: []\nimplementation_plan:\n  scope:\n    include:\n      - src/command/task.rs\n    exclude:\n      - docs/syu/**\n  steps:\n    - add a Goal Plan model\ntest_plan:\n  selection_mode: affected\n  required_tests:\n    rust:\n      - file: tests/task_command.rs\n        symbols:\n          - \"\"\n  suggested_tests: {}\ncoverage:\n  mode: changed_lines\n  threshold: 100\n  include:\n    - src/command/task.rs\n  exclude: []\ncompletion:\n  must_pass:\n    - syu validate .\n",
    );

    let output = {
        let mut command = Command::cargo_bin("syu").expect("binary should build");
        command.current_dir(tempdir.path());
        command.args([
            "task",
            "check",
            "goal-plan.yaml",
            "--range",
            "origin/main...HEAD",
        ]);
        command.output().expect("command should run")
    };

    assert!(!output.status.success(), "task check should fail");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("required test symbol is empty"));
}

// REQ-CORE-031
#[test]
fn task_check_rejects_absolute_required_test_files_outside_workspace() {
    let tempdir = tempdir().expect("tempdir");
    prepare_git_workspace(tempdir.path(), "task_plan_generates_goal_from_request");

    fs::write(
        tempdir.path().join("src/command/task.rs"),
        "pub fn run_task_command() {}\n// updated for absolute-path check\n",
    )
    .expect("updated task source");
    commit_all(tempdir.path(), "update task");

    let external = tempfile::tempdir().expect("external tempdir");
    let external_test = external.path().join("outside.rs");
    fs::write(
        &external_test,
        "fn task_plan_generates_goal_from_request() {}\n",
    )
    .expect("external test file");

    let plan = tempdir.path().join("goal-plan.yaml");
    write_goal_plan(
        &plan,
        &goal_plan_yaml(
            "REQ-CORE-031",
            "FEAT-TASK-004",
            &external_test.display().to_string(),
            "task_plan_generates_goal_from_request",
            "high",
        ),
    );

    let output = {
        let mut command = Command::cargo_bin("syu").expect("binary should build");
        command.current_dir(tempdir.path());
        command.args([
            "task",
            "check",
            "goal-plan.yaml",
            "--range",
            "origin/main...HEAD",
        ]);
        command.output().expect("command should run")
    };

    assert!(!output.status.success(), "task check should fail");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("required test file must stay within the workspace"));
    assert!(stdout.contains(&external_test.display().to_string()));
}

#[test]
fn task_check_rejects_malformed_goal_plans() {
    let tempdir = tempdir().expect("tempdir");
    write_workspace(tempdir.path());

    let plan = tempdir.path().join("goal-plan.yaml");
    write_goal_plan(
        &plan,
        "version: 1\nkind: syu.goal_plan\nsource:\n  mode: diff_inferred\n",
    );

    let output = {
        let mut command = Command::cargo_bin("syu").expect("binary should build");
        command.current_dir(tempdir.path());
        command.args([
            "task",
            "check",
            "goal-plan.yaml",
            "--range",
            "origin/main...HEAD",
        ]);
        command.output().expect("command should run")
    };

    assert!(!output.status.success(), "malformed goal plan should fail");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("failed to parse goal plan artifact"));
}

#[test]
// REQ-CORE-031
fn goal_plan_artifact_supports_request_driven_and_diff_inferred_sources() {
    let guide = fs::read_to_string("docs/guide/goal-plan-format.md")
        .expect("goal plan guide should be readable");

    assert!(guide.contains("syu.goal_plan"));
    assert!(guide.contains("request_driven"));
    assert!(guide.contains("diff-inferred"));
    assert!(guide.contains("confidence"));
}

#[test]
// REQ-CORE-031
fn goal_plan_artifact_requires_the_goal_plan_marker() {
    let guide = fs::read_to_string("docs/guide/goal-plan-format.md")
        .expect("goal plan guide should be readable");

    assert!(guide.contains("kind: syu.goal_plan"));
    assert!(guide.contains("Goal Plans are"));
    assert!(guide.contains("planning artifacts"));
}

#[test]
// REQ-CORE-031
fn goal_plan_format_is_documented_as_temporary() {
    let guide = fs::read_to_string("docs/guide/implementation-planning.md")
        .expect("implementation planning guide should be readable");

    assert!(guide.contains("goal plan format"));
    assert!(guide.contains("temporary"));
}
