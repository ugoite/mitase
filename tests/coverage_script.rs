use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::Command,
};

use serde_json::Value;
use tempfile::tempdir;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn write_executable(path: &Path, contents: &str) {
    fs::write(path, contents).expect("write executable");
    let mut permissions = fs::metadata(path).expect("metadata").permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).expect("chmod");
}

fn make_executable(path: &Path) {
    let mut permissions = fs::metadata(path).expect("metadata").permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).expect("chmod");
}

fn copy_coverage_scripts(root: &Path) {
    fs::create_dir_all(root.join("scripts/ci")).expect("scripts dir");
    fs::copy(
        repo_root().join("scripts/ci/coverage.sh"),
        root.join("scripts/ci/coverage.sh"),
    )
    .expect("copy coverage script");
    fs::copy(
        repo_root().join("scripts/ci/write-spec-coverage-summary.py"),
        root.join("scripts/ci/write-spec-coverage-summary.py"),
    )
    .expect("copy coverage summary script");
    make_executable(&root.join("scripts/ci/coverage.sh"));
}

fn write_fixture_workspace(root: &Path) {
    fs::create_dir_all(root.join("src")).expect("src dir");
    fs::create_dir_all(root.join("tests")).expect("tests dir");
    fs::create_dir_all(root.join("docs/generated")).expect("generated docs dir");
    fs::create_dir_all(root.join("docs/syu/features/core")).expect("features dir");
    fs::create_dir_all(root.join("docs/syu/requirements/core")).expect("requirements dir");

    let yaml_shim = "import json\n\n\ndef safe_load(text):\n    return json.loads(text)\n";
    fs::write(root.join("yaml.py"), yaml_shim).expect("yaml shim");
    fs::write(root.join("scripts/ci/yaml.py"), yaml_shim).expect("yaml shim");
    fs::write(
        root.join("syu.yaml"),
        r#"{"version":1,"spec":{"root":"docs/syu"}}"#,
    )
    .expect("syu config");
    fs::write(
        root.join("docs/syu/features/core/feature.yaml"),
        r#"{"category":"Core Workspace","version":1,"features":[{"id":"FEAT-CORE-001","title":"Coverage feature","summary":"Track rust implementation coverage.","status":"implemented","linked_requirements":["REQ-CORE-001"],"implementations":{"rust":[{"file":"src/lib.rs","symbols":["*"]}]}}]}"#,
    )
    .expect("feature doc");
    fs::write(
        root.join("docs/syu/requirements/core/requirement.yaml"),
        r#"{"category":"Core Workspace","prefix":"REQ-CORE","requirements":[{"id":"REQ-CORE-001","title":"Coverage requirement","description":"Track selected test coverage.","priority":"medium","status":"implemented","linked_policies":[],"linked_features":["FEAT-CORE-001"],"tests":{"rust":[{"file":"tests/helper.rs","symbols":["*"]}]}}]}"#,
    )
    .expect("requirement doc");

    fs::write(root.join("src/lib.rs"), "pub fn covered() -> u32 { 1 }\n").expect("lib base");
    fs::write(root.join("src/other.rs"), "pub fn outside() -> u32 { 1 }\n").expect("other base");
    fs::write(
        root.join("tests/helper.rs"),
        "pub fn helper() -> u32 { 1 }\n",
    )
    .expect("test base");
    fs::write(
        root.join("docs/generated/generated.rs"),
        "pub fn generated() -> u32 { 1 }\n",
    )
    .expect("generated base");
}

fn write_mock_cargo(bin_dir: &Path, repo_root: &Path, lcov_contents: &str, list_json: &str) {
    let cargo_path = bin_dir.join("cargo");
    let repo_root = repo_root.display().to_string();
    let script = format!(
        r#"#!/usr/bin/env bash
set -euo pipefail
repo_root={repo_root:?}
lcov_contents={lcov_contents:?}
list_json={list_json:?}

if [[ "$1" == "llvm-cov" ]]; then
  shift
  if [[ "$1" == "nextest" ]]; then
    shift
  elif [[ "$1" == "test" ]]; then
    shift
  else
    printf 'unexpected cargo llvm-cov invocation: %s\n' "$*" >&2
    exit 1
  fi
  output_path=""
  while [[ $# -gt 0 ]]; do
    case "$1" in
      --output-path)
        output_path="$2"
        shift 2
        ;;
      *)
        shift
        ;;
    esac
  done
  mkdir -p "$(dirname "$output_path")"
  printf '%b' "$lcov_contents" >"$output_path"
  exit 0
fi

if [[ "$1" == "run" ]]; then
  shift
  while [[ $# -gt 0 && "$1" != "--" ]]; do
    shift
  done
  if [[ "$1" == "--" ]]; then
    shift
  fi
  if [[ "$1" == "list" && "$2" == "--with-path" && "$3" == "--format" && "$4" == "json" ]]; then
    printf '%s' "$list_json"
    exit 0
  fi
fi

printf 'unexpected cargo invocation: %s\n' "$*" >&2
exit 1
"#,
    );
    write_executable(&cargo_path, &script);

    let cargo_llvm_cov_path = bin_dir.join("cargo-llvm-cov");
    write_executable(&cargo_llvm_cov_path, "#!/usr/bin/env bash\nexit 0\n");
}

fn init_git_repo(root: &Path) {
    let status = Command::new("git")
        .current_dir(root)
        .args(["init", "-b", "main"])
        .status()
        .expect("git command should run");
    assert!(status.success(), "git init failed");

    let status = Command::new("git")
        .current_dir(root)
        .args(["config", "user.email", "syu@example.com"])
        .status()
        .expect("git command should run");
    assert!(status.success(), "git config user.email failed");

    let status = Command::new("git")
        .current_dir(root)
        .args(["config", "user.name", "syu"])
        .status()
        .expect("git command should run");
    assert!(status.success(), "git config user.name failed");
}

fn commit_all(root: &Path, message: &str) {
    let status = Command::new("git")
        .current_dir(root)
        .args(["add", "-A"])
        .status()
        .expect("git command should run");
    assert!(status.success(), "git add failed");

    let status = Command::new("git")
        .current_dir(root)
        .args(["commit", "--quiet", "-m", message])
        .status()
        .expect("git command should run");
    assert!(status.success(), "git commit failed");
}

fn update_origin_main(root: &Path) {
    let status = Command::new("git")
        .current_dir(root)
        .args(["update-ref", "refs/remotes/origin/main", "HEAD"])
        .status()
        .expect("git command should run");
    assert!(status.success(), "git update-ref failed");
}

fn run_coverage(root: &Path, bin_dir: &Path, args: &[&str]) -> std::process::Output {
    Command::new("bash")
        .current_dir(root)
        .arg(root.join("scripts/ci/coverage.sh"))
        .args(args)
        .env("HOME", root)
        .env(
            "PATH",
            format!("{}:{}", bin_dir.display(), std::env::var("PATH").unwrap()),
        )
        .output()
        .expect("coverage script should run")
}

#[test]
// REQ-CORE-006
fn pr_goal_coverage_reports_json_success_for_covered_in_scope_changes() {
    let tempdir = tempdir().expect("tempdir");
    copy_coverage_scripts(tempdir.path());
    write_fixture_workspace(tempdir.path());
    init_git_repo(tempdir.path());
    commit_all(tempdir.path(), "base");
    update_origin_main(tempdir.path());

    fs::write(
        tempdir.path().join("src/lib.rs"),
        "pub fn covered() -> u32 { 2 }\n",
    )
    .expect("update lib");
    commit_all(tempdir.path(), "goal coverage");

    let bin_dir = tempdir.path().join("bin");
    fs::create_dir_all(&bin_dir).expect("bin dir");
    let lcov_contents = format!(
        "TN:\nSF:{}\nDA:1,1\nend_of_record\n",
        tempdir.path().join("src/lib.rs").display()
    );
    write_mock_cargo(
        &bin_dir,
        tempdir.path(),
        &lcov_contents,
        r#"{"requirement":[{"id":"REQ-CORE-001","document_path":"docs/syu/requirements/core/requirement.yaml"}],"feature":[{"id":"FEAT-CORE-001","document_path":"docs/syu/features/core/feature.yaml"}]}"#,
    );

    let goal_plan = tempdir.path().join("target/syu/goal.yaml");
    fs::create_dir_all(goal_plan.parent().expect("goal plan parent")).expect("goal dir");
    fs::write(
        &goal_plan,
        r#"{"version":1,"kind":"syu.goal_plan","goal":{"id":"GOAL-001","title":"Keep coverage goal explicit","statement":"Keep PR coverage tied to a Goal Plan."},"implementation_plan":{"scope":{"include":[{"file":"src/lib.rs","symbols":[]}],"exclude":[]},"steps":["keep the changed line covered"]},"test_plan":{"selection_mode":"affected","required_tests":{"rust":[{"file":"tests/helper.rs","symbols":["*"]}]},"suggested_tests":{}},"coverage":{"mode":"changed_lines","threshold":100,"include":["src/lib.rs"],"exclude":[]},"completion":{"must_pass":["syu validate ."]}}"#,
    )
    .expect("goal plan");

    let output = run_coverage(
        tempdir.path(),
        &bin_dir,
        &[
            "pr",
            "--goal",
            goal_plan.to_str().expect("utf-8"),
            "--format",
            "json",
        ],
    );

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let json: Value = serde_json::from_slice(&output.stdout).expect("json output");
    assert_eq!(json["status"], "passed");
    assert_eq!(json["mode"], "goal_scoped");
    assert_eq!(json["goal"]["id"], "GOAL-001");
    assert!(
        json["missing_changed_line_coverage"]
            .as_array()
            .expect("array")
            .is_empty()
    );
    assert!(
        json["changed_files_outside_goal_scope"]
            .as_array()
            .expect("array")
            .is_empty()
    );
}

#[test]
// REQ-CORE-006
fn pr_goal_coverage_fails_for_uncovered_and_out_of_scope_changes() {
    let tempdir = tempdir().expect("tempdir");
    copy_coverage_scripts(tempdir.path());
    write_fixture_workspace(tempdir.path());
    init_git_repo(tempdir.path());
    commit_all(tempdir.path(), "base");
    update_origin_main(tempdir.path());

    fs::write(
        tempdir.path().join("src/lib.rs"),
        "pub fn covered() -> u32 { 2 }\npub fn uncovered() -> u32 { 3 }\n",
    )
    .expect("update lib");
    fs::write(
        tempdir.path().join("src/other.rs"),
        "pub fn outside() -> u32 { 2 }\n",
    )
    .expect("update other");
    fs::write(
        tempdir.path().join("tests/helper.rs"),
        "pub fn helper() -> u32 { 2 }\n",
    )
    .expect("update test");
    fs::write(
        tempdir.path().join("docs/generated/generated.rs"),
        "pub fn generated() -> u32 { 2 }\n",
    )
    .expect("update generated");
    commit_all(tempdir.path(), "goal coverage failure");

    let bin_dir = tempdir.path().join("bin");
    fs::create_dir_all(&bin_dir).expect("bin dir");
    let lcov_contents = format!(
        "TN:\nSF:{}\nDA:1,1\nDA:2,0\nend_of_record\n",
        tempdir.path().join("src/lib.rs").display()
    );
    write_mock_cargo(
        &bin_dir,
        tempdir.path(),
        &lcov_contents,
        r#"{"requirement":[{"id":"REQ-CORE-001","document_path":"docs/syu/requirements/core/requirement.yaml"}],"feature":[{"id":"FEAT-CORE-001","document_path":"docs/syu/features/core/feature.yaml"}]}"#,
    );

    let goal_plan = tempdir.path().join("target/syu/goal.yaml");
    fs::create_dir_all(goal_plan.parent().expect("goal plan parent")).expect("goal dir");
    fs::write(
        &goal_plan,
        r#"{"version":1,"kind":"syu.goal_plan","goal":{"id":"GOAL-001","title":"Keep coverage goal explicit","statement":"Keep PR coverage tied to a Goal Plan."},"implementation_plan":{"scope":{"include":[{"file":"src/lib.rs","symbols":[]}],"exclude":["docs/generated/**"]},"steps":["keep the changed line covered","keep outside-scope files out of the scope"]},"test_plan":{"selection_mode":"affected","required_tests":{"rust":[{"file":"tests/helper.rs","symbols":["*"]}]},"suggested_tests":{}},"coverage":{"mode":"changed_lines","threshold":100,"include":["src/lib.rs"],"exclude":[]},"completion":{"must_pass":["syu validate ."]}}"#,
    )
    .expect("goal plan");

    let output = run_coverage(
        tempdir.path(),
        &bin_dir,
        &["pr", "--goal", goal_plan.to_str().expect("utf-8")],
    );

    assert!(
        !output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("goal-scoped coverage failed: GOAL-001"));
    assert!(stdout.contains("Goal: Keep coverage goal explicit"));
    assert!(stdout.contains("Missing changed-line coverage:"));
    assert!(stdout.contains("src/lib.rs"));
    assert!(stdout.contains("2"));
    assert!(stdout.contains("Changed production files outside goal scope:"));
    assert!(stdout.contains("src/other.rs"));
    assert!(!stdout.contains("tests/helper.rs"));
    assert!(!stdout.contains("docs/generated/generated.rs"));
}
