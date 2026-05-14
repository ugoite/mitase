// REQ-CORE-028
// REQ-CORE-029
// REQ-CORE-030

use std::fs;

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
        "category: Policies\nversion: 1\nlanguage: en\npolicies:\n  - id: POL-001\n    title: Keep request workflows visible\n    summary: Keep intake and planning separate.\n    description: Request artifacts should be classified against the current graph.\n    linked_philosophies:\n      - PHIL-001\n    linked_requirements:\n      - REQ-CORE-028\n      - REQ-CORE-029\n",
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
        root.join("docs/syu/features/features.yaml"),
        "version: 1\nupdated: \"2026-05\"\nfiles:\n  - kind: task\n    file: core/task.yaml\n  - kind: task\n    file: core/scaffold.yaml\n  - kind: task\n    file: core/scope.yaml\n",
    )
    .expect("feature registry");
    fs::write(
        root.join("docs/syu/features/core/task.yaml"),
        "category: Task Planning CLI\nversion: 1\nfeatures:\n  - id: FEAT-TASK-001\n    title: Request artifact classification\n    summary: Classify captured request artifacts into create, change, or delete decisions using the current spec graph, with a short explanation and text or JSON output.\n    status: implemented\n    linked_requirements:\n      - REQ-CORE-028\n    implementations:\n      rust:\n        - file: src/command/task.rs\n          symbols:\n            - run_task_command\n            - run_task_classify_command\n  - id: FEAT-TASK-003\n    title: Request artifact scoping\n    summary: Map request artifacts onto candidate requirements, policies, philosophies, and features before planning begins.\n    status: implemented\n    linked_requirements:\n      - REQ-CORE-030\n    implementations:\n      rust:\n        - file: src/command/task.rs\n          symbols:\n            - run_task_command\n            - run_task_scope_command\n",
    )
    .expect("feature doc");
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
    assert!(stdout.contains("policy discussion likely: yes"));
    assert!(stdout.contains("philosophy discussion likely: yes"));
    assert!(stdout.contains("FEAT-TASK-001"));
    assert!(stdout.contains("planned-state update suggested"));
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
    assert!(stdout.contains("\"policy_discussion\": true"));
    assert!(stdout.contains("\"philosophy_discussion\": true"));
    assert!(stdout.contains("\"planned_feature_updates\": true"));
    assert!(stdout.contains("\"id\": \"REQ-CORE-028\""));
    assert!(stdout.contains("\"id\": \"FEAT-TASK-001\""));
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
