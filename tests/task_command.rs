// REQ-CORE-028

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
        "category: Policies\nversion: 1\nlanguage: en\npolicies:\n  - id: POL-001\n    title: Keep request workflows visible\n    summary: Keep intake and planning separate.\n    description: Request artifacts should be classified against the current graph.\n    linked_philosophies:\n      - PHIL-001\n    linked_requirements:\n      - REQ-CORE-028\n",
    )
    .expect("policy doc");
    fs::write(
        root.join("docs/syu/requirements/core/classify.yaml"),
        "category: Core Workspace\nprefix: REQ-CORE\nrequirements:\n  - id: REQ-CORE-028\n    title: Classify request artifacts into requirement actions\n    description: The task classifier should decide whether a request creates, changes, or deletes a requirement.\n    priority: medium\n    status: implemented\n    linked_policies:\n      - POL-001\n    linked_features:\n      - FEAT-TASK-001\n    tests:\n      rust:\n        - file: src/command/task.rs\n          symbols:\n            - '*'\n",
    )
    .expect("requirement doc");
    fs::write(
        root.join("docs/syu/features/features.yaml"),
        "version: 1\nupdated: \"2026-05\"\nfiles:\n  - kind: task\n    file: core/task.yaml\n",
    )
    .expect("feature registry");
    fs::write(
        root.join("docs/syu/features/core/task.yaml"),
        "category: Task Planning CLI\nversion: 1\nfeatures:\n  - id: FEAT-TASK-001\n    title: Request artifact classification\n    summary: Classify captured request artifacts into create, change, or delete decisions using the current spec graph, with a short explanation and text or JSON output.\n    status: implemented\n    linked_requirements:\n      - REQ-CORE-028\n    implementations:\n      rust:\n        - file: src/command/task.rs\n          symbols:\n            - run_task_command\n            - run_task_classify_command\n        - file: src/cli.rs\n          symbols:\n            - TaskArgs\n            - TaskClassifyArgs\n        - file: src/lib.rs\n          symbols:\n            - dispatches_task_subcommands_without_rewriting_them\n",
    )
    .expect("feature doc");
}

fn write_request(root: &std::path::Path, request: &str, linked_ids: &[&str]) {
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
            "version: 1\nrequest: >\n  {request}\ncontext:\n  affected_area: core\n  repository_constraints:\n    - keep text and JSON output\n{linked_ids_block}",
        ),
    )
    .expect("request");
}

#[test]
fn task_classify_prints_text_output() {
    let tempdir = tempdir().expect("tempdir");
    write_workspace(tempdir.path());
    write_request(
        tempdir.path(),
        "Update REQ-CORE-028 so the request classifier stays explainable.",
        &["REQ-CORE-028"],
    );

    let output = {
        let mut command = std::process::Command::cargo_bin("syu").expect("binary should build");
        command.current_dir(tempdir.path());
        command.args(["task", "classify", "request.yaml"]);
        command.output().expect("command should run")
    };

    assert!(output.status.success(), "task classify should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("classification: requirement_change"));
    assert!(stdout.contains("REQ-CORE-028"));
}

#[test]
fn task_classify_prints_json_output() {
    let tempdir = tempdir().expect("tempdir");
    write_workspace(tempdir.path());
    write_request(
        tempdir.path(),
        "Create a new request summary for planning.",
        &[],
    );

    let output = {
        let mut command = std::process::Command::cargo_bin("syu").expect("binary should build");
        command.current_dir(tempdir.path());
        command.args(["task", "classify", "request.yaml", "--format", "json"]);
        command.output().expect("command should run")
    };

    assert!(output.status.success(), "task classify should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("\"classification\": \"requirement_create\""));
    assert!(stdout.contains("\"request_path\": \"request.yaml\""));
}

#[test]
// REQ-CORE-028
fn task_classify_handles_requests_without_matches() {
    let tempdir = tempdir().expect("tempdir");
    write_workspace(tempdir.path());
    fs::write(
        tempdir.path().join("request.yaml"),
        "version: 1\nrequest: >\n  Blorf zqxw 123.\ncontext: {}\n",
    )
    .expect("request");

    let output = {
        let mut command = std::process::Command::cargo_bin("syu").expect("binary should build");
        command.current_dir(tempdir.path());
        command.args(["task", "classify", "request.yaml"]);
        command.output().expect("command should run")
    };

    assert!(output.status.success(), "task classify should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("classification: requirement_create"));
    assert!(stdout.contains("- none"));
    assert!(stdout.contains("request does not use a strong create/change/delete verb"));
    assert!(stdout.contains("no existing spec item was named and the request reads like new work"));
}
