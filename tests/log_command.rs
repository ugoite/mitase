// REQ-CORE-024

use assert_cmd::cargo::CommandCargoExt;
use serde_json::Value;
use std::{
    fs,
    path::Path,
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
};
use tempfile::{TempDir, tempdir};

static COMMIT_TIMESTAMP: AtomicU64 = AtomicU64::new(1_776_355_200);

fn write_history_workspace() -> TempDir {
    let tempdir = tempdir().expect("tempdir should exist");
    let docs_root = tempdir.path().join("docs/syu");
    fs::create_dir_all(docs_root.join("philosophy")).expect("philosophy dir");
    fs::create_dir_all(docs_root.join("policies")).expect("policies dir");
    fs::create_dir_all(docs_root.join("requirements")).expect("requirements dir");
    fs::create_dir_all(docs_root.join("features/cli")).expect("features dir");
    fs::create_dir_all(tempdir.path().join("src")).expect("src dir");

    fs::write(
        docs_root.join("philosophy/foundation.yaml"),
        "category: Philosophy\nversion: 1\nlanguage: en\nphilosophies:\n  - id: PHIL-HIST-001\n    title: History should stay explorable.\n    product_design_principle: Keep commit history close to trace links.\n    coding_guideline: Prefer one-command repository history lookups.\n    linked_policies:\n      - POL-HIST-001\n",
    )
    .expect("philosophy file");
    fs::write(
        docs_root.join("policies/policies.yaml"),
        "category: Policies\nversion: 1\nlanguage: en\npolicies:\n  - id: POL-HIST-001\n    title: History should be reachable from traces.\n    summary: Git history is useful when it is derived from checked-in trace metadata.\n    description: The repository history should be explorable from requirement and feature traces.\n    linked_philosophies:\n      - PHIL-HIST-001\n    linked_requirements:\n      - REQ-HIST-001\n",
    )
    .expect("policy file");
    fs::write(
        docs_root.join("requirements/core.yaml"),
        "category: Core\nprefix: REQ-HIST\n\nrequirements:\n  - id: REQ-HIST-001\n    title: Requirement history lookup\n    description: Requirement history should show the traced test and checked-in definition.\n    priority: medium\n    status: implemented\n    linked_policies:\n      - POL-HIST-001\n    linked_features:\n      - FEAT-HIST-001\n    tests:\n      rust:\n        - file: src/history_tests.rs\n          symbols:\n            - requirement_history_test\n",
    )
    .expect("requirement file");
    fs::write(
        docs_root.join("features/features.yaml"),
        "version: \"1\"\nfiles:\n  - kind: history\n    file: cli/history.yaml\n",
    )
    .expect("feature registry");
    fs::write(
        docs_root.join("features/cli/history.yaml"),
        "category: History\nversion: 1\nfeatures:\n  - id: FEAT-HIST-001\n    title: Feature history lookup\n    summary: Feature history should show the traced implementation and checked-in definition.\n    status: implemented\n    linked_requirements:\n      - REQ-HIST-001\n    implementations:\n      rust:\n        - file: src/history_feature.rs\n          symbols:\n            - feature_history\n",
    )
    .expect("feature file");
    fs::write(
        tempdir.path().join("src/history_tests.rs"),
        "// REQ-HIST-001\nfn requirement_history_test() {}\n",
    )
    .expect("history test file");
    fs::write(
        tempdir.path().join("src/history_feature.rs"),
        "// FEAT-HIST-001\nfn feature_history() {}\n",
    )
    .expect("history feature file");

    init_git_repository(tempdir.path());
    update_file(
        &tempdir.path().join("docs/syu/requirements/core.yaml"),
        "category: Core\nprefix: REQ-HIST\n\nrequirements:\n  - id: REQ-HIST-001\n    title: Requirement history lookup\n    description: Requirement history should show the traced test, checked-in definition, and maintenance story.\n    priority: medium\n    status: implemented\n    linked_policies:\n      - POL-HIST-001\n    linked_features:\n      - FEAT-HIST-001\n    tests:\n      rust:\n        - file: src/history_tests.rs\n          symbols:\n            - requirement_history_test\n",
    );
    git_commit(tempdir.path(), "docs: refine traced requirement history");

    update_file(
        &tempdir.path().join("src/history_tests.rs"),
        "// REQ-HIST-001\nfn requirement_history_test() {\n    // traced test adjustment\n}\n",
    );
    git_commit(tempdir.path(), "test: adjust traced requirement coverage");

    update_file(
        &tempdir.path().join("src/history_feature.rs"),
        "// FEAT-HIST-001\nfn feature_history() {\n    // traced implementation adjustment\n}\n",
    );
    git_commit(tempdir.path(), "feat: update traced implementation");

    tempdir
}

fn init_git_repository(workspace: &Path) {
    git(workspace, &["init"]);
    fs::create_dir_all(workspace.join(".git-hooks")).expect("hooks dir");
    git(workspace, &["config", "core.hooksPath", ".git-hooks"]);
    git(workspace, &["config", "user.name", "Test User"]);
    git(workspace, &["config", "user.email", "test@example.com"]);
    git(workspace, &["add", "."]);
    git_commit_with_timestamp(workspace, "chore: initial history fixture");
}

fn git_commit(workspace: &Path, summary: &str) {
    git(workspace, &["add", "."]);
    git_commit_with_timestamp(workspace, summary);
}

fn git_commit_with_timestamp(workspace: &Path, summary: &str) {
    let timestamp = COMMIT_TIMESTAMP.fetch_add(1, Ordering::Relaxed);
    let timestamp = format!("{timestamp} +0000");

    let mut command = Command::new("git");
    command
        .arg("-C")
        .arg(workspace)
        .args(["commit", "-m", summary])
        .env("GIT_AUTHOR_DATE", &timestamp)
        .env("GIT_COMMITTER_DATE", &timestamp);
    for key in [
        "GIT_ALTERNATE_OBJECT_DIRECTORIES",
        "GIT_CEILING_DIRECTORIES",
        "GIT_COMMON_DIR",
        "GIT_DIR",
        "GIT_INDEX_FILE",
        "GIT_OBJECT_DIRECTORY",
        "GIT_PREFIX",
        "GIT_WORK_TREE",
    ] {
        command.env_remove(key);
    }

    let output = command.output().expect("git should run");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "git commit failed\nstdout:\n{}\nstderr:\n{}",
        stdout,
        stderr
    );
}

fn git(workspace: &Path, args: &[&str]) {
    let mut command = Command::new("git");
    command.arg("-C").arg(workspace).args(args);
    for key in [
        "GIT_ALTERNATE_OBJECT_DIRECTORIES",
        "GIT_CEILING_DIRECTORIES",
        "GIT_COMMON_DIR",
        "GIT_DIR",
        "GIT_INDEX_FILE",
        "GIT_OBJECT_DIRECTORY",
        "GIT_PREFIX",
        "GIT_WORK_TREE",
    ] {
        command.env_remove(key);
    }
    let output = command.output().expect("git should run");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "git {:?} failed\nstdout:\n{}\nstderr:\n{}",
        args,
        stdout,
        stderr
    );
}

fn update_file(path: &Path, contents: &str) {
    fs::write(path, contents).expect("file should update");
}

fn git_stdout(workspace: &Path, args: &[&str]) -> String {
    let mut command = Command::new("git");
    command.arg("-C").arg(workspace).args(args);
    for key in [
        "GIT_ALTERNATE_OBJECT_DIRECTORIES",
        "GIT_CEILING_DIRECTORIES",
        "GIT_COMMON_DIR",
        "GIT_DIR",
        "GIT_INDEX_FILE",
        "GIT_OBJECT_DIRECTORY",
        "GIT_PREFIX",
        "GIT_WORK_TREE",
    ] {
        command.env_remove(key);
    }
    let output = command.output().expect("git should run");
    assert!(
        output.status.success(),
        "git {:?} failed\nstdout:\n{}\nstderr:\n{}",
        args,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout)
        .expect("git output should be valid utf8")
        .trim()
        .to_string()
}

fn write_fake_git_for_log_failure(script_dir: &Path) {
    let script_path = script_dir.join("git");
    fs::write(
        &script_path,
        "#!/bin/sh\nset -eu\nworkspace=\"$2\"\ncommand_name=\"$3\"\nif [ \"$command_name\" = \"rev-parse\" ]; then\n  printf '%s\\n' \"$workspace\"\n  exit 0\nfi\nif [ \"$command_name\" = \"log\" ]; then\n  echo 'synthetic git log failure' >&2\n  exit 1\nfi\necho 'unexpected git invocation' >&2\nexit 1\n",
    )
    .expect("fake git script");

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let mut permissions = fs::metadata(&script_path)
            .expect("fake git metadata")
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&script_path, permissions).expect("fake git permissions");
    }
}

fn write_fake_git_for_merge_base_failure(script_dir: &Path) {
    let script_path = script_dir.join("git");
    fs::write(
        &script_path,
        "#!/bin/sh\nset -eu\nworkspace=\"$2\"\ncommand_name=\"$3\"\nif [ \"$command_name\" = \"rev-parse\" ]; then\n  printf '%s\\n' \"$workspace\"\n  exit 0\nfi\nif [ \"$command_name\" = \"log\" ]; then\n  printf '\\036sha\\000short\\000author\\0002026-04-13T00:00:00+00:00\\000subject\\000src/history_tests.rs\\000'\n  exit 0\nfi\nif [ \"$command_name\" = \"merge-base\" ]; then\n  echo 'synthetic git merge-base failure' >&2\n  exit 2\nfi\necho 'unexpected git invocation' >&2\nexit 1\n",
    )
    .expect("fake git script");

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let mut permissions = fs::metadata(&script_path)
            .expect("fake git metadata")
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&script_path, permissions).expect("fake git permissions");
    }
}

fn write_fake_git_for_merge_base_spawn_failure(script_dir: &Path) {
    let script_path = script_dir.join("git");
    fs::write(
        &script_path,
        "#!/bin/sh\nset -eu\nworkspace=\"$2\"\ncommand_name=\"$3\"\nif [ \"$command_name\" = \"rev-parse\" ]; then\n  printf '%s\\n' \"$workspace\"\n  exit 0\nfi\nif [ \"$command_name\" = \"log\" ]; then\n  printf '\\036sha\\000short\\000author\\0002026-04-13T00:00:00+00:00\\000subject\\000src/history_tests.rs\\000'\n  /bin/rm -- \"$0\"\n  exit 0\nfi\necho 'unexpected git invocation' >&2\nexit 1\n",
    )
    .expect("fake git script");

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let mut permissions = fs::metadata(&script_path)
            .expect("fake git metadata")
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&script_path, permissions).expect("fake git permissions");
    }
}

#[test]
fn log_command_renders_requirement_definition_and_test_history() {
    let workspace = write_history_workspace();
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("log")
        .arg("REQ-HIST-001")
        .arg(workspace.path())
        .output()
        .expect("command should run");

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("History: requirement REQ-HIST-001"));
    assert!(stdout.contains("definition\tdocs/syu/requirements/core.yaml"));
    assert!(stdout.contains("test\tsrc/history_tests.rs\trust\t[`requirement_history_test`]"));
    assert!(stdout.contains("docs: refine traced requirement history"));
    assert!(stdout.contains("test: adjust traced requirement coverage"));
    assert!(
        !stdout.contains("feat: update traced implementation"),
        "requirement history should not include feature implementation commits"
    );
}

#[test]
fn log_command_supports_json_kind_and_path_filters() {
    let workspace = write_history_workspace();
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .args([
            "log",
            "FEAT-HIST-001",
            workspace.path().to_str().expect("utf8 path"),
            "--kind",
            "implementation",
            "--path",
            "src",
            "--format",
            "json",
        ])
        .output()
        .expect("command should run");

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let json: Value = serde_json::from_slice(&output.stdout).expect("output should be valid JSON");
    assert_eq!(json["id"], "FEAT-HIST-001");
    assert_eq!(json["kind"], "implementation");
    assert_eq!(json["path_filter"], "src");
    let commits = json["commits"].as_array().expect("commit array");
    assert_eq!(commits.len(), 2);
    assert_eq!(
        json["commits"][0]["summary"],
        "feat: update traced implementation"
    );
    assert_eq!(json["commits"][0]["reasons"][0]["kind"], "implementation");
    assert_eq!(
        json["commits"][0]["reasons"][0]["path"],
        "src/history_feature.rs"
    );
    assert_eq!(
        json["commits"][1]["summary"],
        "chore: initial history fixture"
    );
}

#[test]
fn log_command_can_include_related_history_surface() {
    let workspace = write_history_workspace();
    update_file(
        &workspace.path().join("docs/syu/features/cli/history.yaml"),
        "category: History\nversion: 1\nfeatures:\n  - id: FEAT-HIST-001\n    title: Feature history lookup\n    summary: Feature history should show the traced implementation, linked requirement, and checked-in definition.\n    status: implemented\n    linked_requirements:\n      - REQ-HIST-001\n    implementations:\n      rust:\n        - file: src/history_feature.rs\n          symbols:\n            - feature_history\n",
    );
    git_commit(workspace.path(), "docs: refine related feature definition");

    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .args([
            "log",
            "REQ-HIST-001",
            workspace.path().to_str().expect("utf8 path"),
            "--include-related",
            "--format",
            "json",
        ])
        .output()
        .expect("command should run");

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let json: Value = serde_json::from_slice(&output.stdout).expect("output should be valid JSON");
    assert_eq!(json["include_related"], true);
    let tracked_paths = json["tracked_paths"].as_array().expect("tracked paths");
    assert!(tracked_paths.iter().any(|path| {
        path["owner_id"] == "FEAT-HIST-001"
            && path["kind"] == "definition"
            && path["source"] == "related"
    }));
    assert!(tracked_paths.iter().any(|path| {
        path["owner_id"] == "FEAT-HIST-001"
            && path["kind"] == "implementation"
            && path["source"] == "related"
    }));
    let summaries = json["commits"]
        .as_array()
        .expect("commit array")
        .iter()
        .map(|commit| commit["summary"].as_str().expect("summary"))
        .collect::<Vec<_>>();
    assert!(summaries.contains(&"docs: refine related feature definition"));
    assert!(summaries.contains(&"feat: update traced implementation"));
}

#[test]
fn log_command_keeps_related_definitions_when_path_scopes_code_history() {
    let workspace = write_history_workspace();
    update_file(
        &workspace.path().join("docs/syu/features/cli/history.yaml"),
        "category: History\nversion: 1\nfeatures:\n  - id: FEAT-HIST-001\n    title: Feature history lookup\n    summary: Feature history should show the traced implementation, linked requirement, and checked-in definition.\n    status: implemented\n    linked_requirements:\n      - REQ-HIST-001\n    implementations:\n      rust:\n        - file: src/history_feature.rs\n          symbols:\n            - feature_history\n",
    );
    git_commit(workspace.path(), "docs: refine related feature definition");

    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .args([
            "log",
            "REQ-HIST-001",
            workspace.path().to_str().expect("utf8 path"),
            "--include-related",
            "--path",
            "src",
            "--format",
            "json",
        ])
        .output()
        .expect("command should run");

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let json: Value = serde_json::from_slice(&output.stdout).expect("output should be valid JSON");
    let tracked_paths = json["tracked_paths"].as_array().expect("tracked paths");
    assert!(tracked_paths.iter().any(|path| {
        path["owner_id"] == "FEAT-HIST-001"
            && path["kind"] == "definition"
            && path["source"] == "related"
            && path["path"] == "docs/syu/features/cli/history.yaml"
    }));
    let summaries = json["commits"]
        .as_array()
        .expect("commit array")
        .iter()
        .map(|commit| commit["summary"].as_str().expect("summary"))
        .collect::<Vec<_>>();
    assert!(summaries.contains(&"docs: refine related feature definition"));
    assert!(summaries.contains(&"feat: update traced implementation"));
}

#[test]
fn log_command_can_limit_history_to_an_explicit_range() {
    let workspace = write_history_workspace();
    let merge_base = git_stdout(workspace.path(), &["rev-parse", "HEAD~2"]);

    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .args([
            "log",
            "REQ-HIST-001",
            workspace.path().to_str().expect("utf8 path"),
            "--include-related",
            "--range",
            &format!("{merge_base}..HEAD"),
            "--format",
            "json",
        ])
        .output()
        .expect("command should run");

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let json: Value = serde_json::from_slice(&output.stdout).expect("output should be valid JSON");
    assert_eq!(
        json["scope"]["label"],
        format!("range `{merge_base}..HEAD`")
    );
    let summaries = json["commits"]
        .as_array()
        .expect("commit array")
        .iter()
        .map(|commit| commit["summary"].as_str().expect("summary"))
        .collect::<Vec<_>>();
    assert!(summaries.contains(&"test: adjust traced requirement coverage"));
    assert!(summaries.contains(&"feat: update traced implementation"));
    assert!(!summaries.contains(&"chore: initial history fixture"));
}

#[test]
fn log_command_rejects_ids_without_supported_prefixes() {
    let workspace = write_history_workspace();
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .args([
            "log",
            "NOTE-HIST-001",
            workspace.path().to_str().expect("utf8 path"),
        ])
        .output()
        .expect("command should run");

    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("was not found"));
}

#[test]
fn log_command_normalizes_non_canonical_trace_paths_and_filters() {
    let workspace = write_history_workspace();
    update_file(
        &workspace.path().join("docs/syu/requirements/core.yaml"),
        "category: Core\nprefix: REQ-HIST\n\nrequirements:\n  - id: REQ-HIST-001\n    title: Requirement history lookup\n    description: Requirement history should show the traced test, checked-in definition, and maintenance story.\n    priority: medium\n    status: implemented\n    linked_policies:\n      - POL-HIST-001\n    linked_features:\n      - FEAT-HIST-001\n    tests:\n      rust:\n        - file: ./src/../src/history_tests.rs\n          symbols:\n            - requirement_history_test\n",
    );
    git_commit(workspace.path(), "docs: rewrite requirement trace path");

    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .args([
            "log",
            "REQ-HIST-001",
            workspace.path().to_str().expect("utf8 path"),
            "--kind",
            "test",
            "--path",
            "./src/../src",
            "--format",
            "json",
        ])
        .output()
        .expect("command should run");

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let json: Value = serde_json::from_slice(&output.stdout).expect("output should be valid JSON");
    assert_eq!(json["path_filter"], "src");
    assert_eq!(
        json["tracked_paths"][0]["path"],
        "./src/../src/history_tests.rs"
    );
    let summaries = json["commits"]
        .as_array()
        .expect("commit array")
        .iter()
        .map(|commit| commit["summary"].as_str().expect("summary"))
        .collect::<Vec<_>>();
    assert!(summaries.contains(&"test: adjust traced requirement coverage"));
    assert!(summaries.contains(&"chore: initial history fixture"));
    assert!(!summaries.contains(&"docs: rewrite requirement trace path"));
}

#[test]
fn log_command_follows_tracked_file_renames() {
    let workspace = write_history_workspace();
    fs::rename(
        workspace.path().join("src/history_feature.rs"),
        workspace.path().join("src/history_feature_renamed.rs"),
    )
    .expect("feature implementation should rename");
    update_file(
        &workspace.path().join("docs/syu/features/cli/history.yaml"),
        "category: History\nversion: 1\nfeatures:\n  - id: FEAT-HIST-001\n    title: Feature history lookup\n    summary: Feature history should show the traced implementation and checked-in definition.\n    status: implemented\n    linked_requirements:\n      - REQ-HIST-001\n    implementations:\n      rust:\n        - file: src/history_feature_renamed.rs\n          symbols:\n            - feature_history\n",
    );
    git_commit(workspace.path(), "feat: rename traced implementation file");

    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .args([
            "log",
            "FEAT-HIST-001",
            workspace.path().to_str().expect("utf8 path"),
            "--kind",
            "implementation",
            "--format",
            "json",
        ])
        .output()
        .expect("command should run");

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let json: Value = serde_json::from_slice(&output.stdout).expect("output should be valid JSON");
    let summaries = json["commits"]
        .as_array()
        .expect("commit array")
        .iter()
        .map(|commit| commit["summary"].as_str().expect("summary"))
        .collect::<Vec<_>>();
    assert_eq!(summaries[0], "feat: rename traced implementation file");
    assert!(summaries.contains(&"feat: update traced implementation"));
    assert!(summaries.contains(&"chore: initial history fixture"));
}

#[test]
fn log_command_rejects_ambiguous_duplicate_ids() {
    let workspace = write_history_workspace();
    fs::write(
        workspace.path().join("docs/syu/requirements/duplicate.yaml"),
        "category: Core\nprefix: REQ-HIST\n\nrequirements:\n  - id: REQ-HIST-001\n    title: Duplicate history lookup\n    description: Duplicate copy.\n    priority: medium\n    status: implemented\n    linked_policies: []\n    linked_features: []\n    tests: {}\n",
    )
    .expect("duplicate requirement file should write");

    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .args([
            "log",
            "REQ-HIST-001",
            workspace.path().to_str().expect("utf8 path"),
        ])
        .output()
        .expect("command should run");

    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("ambiguous because it appears in multiple documents"));
    assert!(stderr.contains("docs/syu/requirements/core.yaml"));
    assert!(stderr.contains("docs/syu/requirements/duplicate.yaml"));
}

#[test]
fn log_command_respects_limit_after_ordering_history() {
    let workspace = write_history_workspace();
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .args([
            "log",
            "FEAT-HIST-001",
            workspace.path().to_str().expect("utf8 path"),
            "--kind",
            "implementation",
            "--limit",
            "1",
            "--format",
            "json",
        ])
        .output()
        .expect("command should run");

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let json: Value = serde_json::from_slice(&output.stdout).expect("output should be valid JSON");
    let commits = json["commits"].as_array().expect("commit array");
    assert_eq!(commits.len(), 1);
    assert_eq!(commits[0]["summary"], "feat: update traced implementation");
}

#[test]
fn log_command_rejects_zero_limit() {
    let workspace = write_history_workspace();
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .args([
            "log",
            "REQ-HIST-001",
            workspace.path().to_str().expect("utf8 path"),
            "--limit",
            "0",
        ])
        .output()
        .expect("command should run");

    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("greater than zero"));
}

#[test]
fn log_command_rejects_unknown_ids() {
    let workspace = write_history_workspace();
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .args([
            "log",
            "REQ-HIST-404",
            workspace.path().to_str().expect("utf8 path"),
        ])
        .output()
        .expect("command should run");

    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("was not found"));
}

#[test]
fn log_command_renders_philosophy_definition_history() {
    let workspace = write_history_workspace();
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .args([
            "log",
            "PHIL-HIST-001",
            workspace.path().to_str().expect("utf8 path"),
        ])
        .output()
        .expect("command should run");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("History: philosophy PHIL-HIST-001"));
    assert!(stdout.contains("definition\tdocs/syu/philosophy/foundation.yaml"));
    assert!(stdout.contains("chore: initial history fixture"));
    assert!(!stdout.contains("test: adjust traced requirement coverage"));
}

#[test]
fn log_command_renders_policy_definition_history() {
    let workspace = write_history_workspace();
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .args([
            "log",
            "POL-HIST-001",
            workspace.path().to_str().expect("utf8 path"),
        ])
        .output()
        .expect("command should run");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("History: policy POL-HIST-001"));
    assert!(stdout.contains("definition\tdocs/syu/policies/policies.yaml"));
    assert!(stdout.contains("chore: initial history fixture"));
    assert!(!stdout.contains("feat: update traced implementation"));
}

#[test]
fn log_command_rejects_incompatible_requirement_kinds() {
    let workspace = write_history_workspace();
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .args([
            "log",
            "REQ-HIST-001",
            workspace.path().to_str().expect("utf8 path"),
            "--kind",
            "implementation",
        ])
        .output()
        .expect("command should run");

    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("--kind implementation"));
}

#[test]
fn log_command_rejects_incompatible_feature_kinds() {
    let workspace = write_history_workspace();
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .args([
            "log",
            "FEAT-HIST-001",
            workspace.path().to_str().expect("utf8 path"),
            "--kind",
            "test",
        ])
        .output()
        .expect("command should run");

    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("--kind test"));
}

#[test]
fn log_command_rejects_incompatible_policy_kinds() {
    let workspace = write_history_workspace();
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .args([
            "log",
            "POL-HIST-001",
            workspace.path().to_str().expect("utf8 path"),
            "--kind",
            "implementation",
        ])
        .output()
        .expect("command should run");

    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("--kind implementation"));
}

#[test]
fn log_command_rejects_incompatible_philosophy_kinds() {
    let workspace = write_history_workspace();
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .args([
            "log",
            "PHIL-HIST-001",
            workspace.path().to_str().expect("utf8 path"),
            "--kind",
            "test",
        ])
        .output()
        .expect("command should run");

    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("--kind test"));
}

#[test]
fn log_command_rejects_empty_path_selections() {
    let workspace = write_history_workspace();
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .args([
            "log",
            "FEAT-HIST-001",
            workspace.path().to_str().expect("utf8 path"),
            "--kind",
            "implementation",
            "--path",
            "docs",
        ])
        .output()
        .expect("command should run");

    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("no tracked history paths remain"));
}

#[test]
fn log_command_rejects_absolute_path_filters_outside_workspace() {
    let workspace = write_history_workspace();
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .args([
            "log",
            "REQ-HIST-001",
            workspace.path().to_str().expect("utf8 path"),
            "--path",
            "/tmp/outside-history-path",
        ])
        .output()
        .expect("command should run");

    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("must stay inside workspace"));
}

#[test]
fn log_command_reports_git_spawn_errors() {
    let workspace = write_history_workspace();
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .env("PATH", "")
        .args([
            "log",
            "REQ-HIST-001",
            workspace.path().to_str().expect("utf8 path"),
        ])
        .output()
        .expect("command should run");

    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("failed to run `git rev-parse`"));
}

#[test]
fn log_command_reports_git_log_failures() {
    let workspace = write_history_workspace();
    let fake_bin = tempdir().expect("tempdir should exist");
    write_fake_git_for_log_failure(fake_bin.path());

    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .env("PATH", fake_bin.path())
        .args([
            "log",
            "REQ-HIST-001",
            workspace.path().to_str().expect("utf8 path"),
            "--kind",
            "definition",
        ])
        .output()
        .expect("command should run");

    assert_eq!(output.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("failed to read git history"),
        "stderr should explain git log failures"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("synthetic git log failure"),
        "stderr should preserve git stderr"
    );
}

#[test]
fn log_command_falls_back_when_git_merge_base_fails() {
    let workspace = write_history_workspace();
    let fake_bin = tempdir().expect("tempdir should exist");
    write_fake_git_for_merge_base_failure(fake_bin.path());

    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .env("PATH", fake_bin.path())
        .args([
            "log",
            "REQ-HIST-001",
            workspace.path().to_str().expect("utf8 path"),
            "--kind",
            "definition",
        ])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("subject"));
}

#[test]
fn log_command_falls_back_when_git_merge_base_cannot_spawn() {
    let workspace = write_history_workspace();
    let fake_bin = tempdir().expect("tempdir should exist");
    write_fake_git_for_merge_base_spawn_failure(fake_bin.path());

    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .env("PATH", fake_bin.path())
        .args([
            "log",
            "REQ-HIST-001",
            workspace.path().to_str().expect("utf8 path"),
            "--kind",
            "definition",
        ])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("subject"));
}

#[test]
fn log_command_requires_a_git_repository() {
    let workspace = tempdir().expect("tempdir should exist");
    fs::create_dir_all(workspace.path().join("docs/syu/philosophy")).expect("philosophy dir");
    fs::create_dir_all(workspace.path().join("docs/syu/policies")).expect("policies dir");
    fs::create_dir_all(workspace.path().join("docs/syu/requirements")).expect("requirements dir");
    fs::create_dir_all(workspace.path().join("docs/syu/features")).expect("features dir");
    fs::write(
        workspace.path().join("docs/syu/philosophy/foundation.yaml"),
        "category: Philosophy\nversion: 1\nlanguage: en\nphilosophies:\n  - id: PHIL-001\n    title: Minimal\n    product_design_principle: Minimal.\n    coding_guideline: Minimal.\n    linked_policies: []\n",
    )
    .expect("philosophy file");
    fs::write(
        workspace.path().join("docs/syu/policies/policies.yaml"),
        "category: Policies\nversion: 1\nlanguage: en\npolicies:\n  - id: POL-001\n    title: Minimal\n    summary: Minimal.\n    description: Minimal.\n    linked_philosophies: []\n    linked_requirements: []\n",
    )
    .expect("policy file");
    fs::write(
        workspace.path().join("docs/syu/requirements/core.yaml"),
        "category: Core\nprefix: REQ\n\nrequirements:\n  - id: REQ-001\n    title: Minimal\n    description: Minimal.\n    priority: medium\n    status: implemented\n    linked_policies: []\n    linked_features: []\n    tests:\n      rust:\n        - file: src/minimal.rs\n          symbols:\n            - minimal_requirement\n",
    )
    .expect("requirement file");
    fs::write(
        workspace.path().join("docs/syu/features/features.yaml"),
        "version: \"1\"\nfiles:\n  - kind: minimal\n    file: minimal.yaml\n",
    )
    .expect("feature registry");
    fs::write(
        workspace.path().join("docs/syu/features/minimal.yaml"),
        "category: Minimal\nversion: 1\nfeatures:\n  - id: FEAT-001\n    title: Minimal\n    summary: Minimal.\n    status: implemented\n    linked_requirements: []\n    implementations: {}\n",
    )
    .expect("feature file");

    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("log")
        .arg("REQ-001")
        .arg(workspace.path())
        .output()
        .expect("command should run");

    assert_eq!(output.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("is not inside a Git repository"),
        "stderr should explain the git repository requirement"
    );
}
