// REQ-CORE-023

use assert_cmd::cargo::CommandCargoExt;
use serde_json::Value;
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
};
use tempfile::tempdir;

static COMMIT_TIMESTAMP: AtomicU64 = AtomicU64::new(1_776_355_200);

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/workspaces")
        .join(name)
}

fn copy_dir_recursive(source: &Path, destination: &Path) {
    fs::create_dir_all(destination).expect("destination dir");
    for entry in fs::read_dir(source).expect("source dir") {
        let entry = entry.expect("dir entry");
        let entry_type = entry.file_type().expect("entry type");
        let destination_path = destination.join(entry.file_name());
        if entry_type.is_dir() {
            copy_dir_recursive(&entry.path(), &destination_path);
        } else {
            fs::copy(entry.path(), destination_path).expect("file copy");
        }
    }
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
    assert!(
        output.status.success(),
        "git {:?} failed\nstdout:\n{}\nstderr:\n{}",
        args,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn git_commit(workspace: &Path, summary: &str) {
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
    let output = command.output().expect("git commit should run");
    assert!(
        output.status.success(),
        "git commit failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn init_git_fixture_workspace() -> tempfile::TempDir {
    let tempdir = tempdir().expect("tempdir should exist");
    let workspace = tempdir.path().join("workspace");
    copy_dir_recursive(&fixture_path("passing"), &workspace);

    git(&workspace, &["init"]);
    git(&workspace, &["config", "user.name", "Test User"]);
    git(&workspace, &["config", "user.email", "test@example.com"]);
    git(&workspace, &["add", "."]);
    git_commit(&workspace, "chore: seed traced workspace");

    fs::write(
        workspace.join("src/rust_feature.rs"),
        "// FEAT-TRACE-001\npub fn feature_trace_rust() {\n    println!(\"changed\");\n}\n",
    )
    .expect("changed traced file");
    git(&workspace, &["add", "."]);
    git_commit(&workspace, "feat: update traced file");

    tempdir
}

fn write_gap_fixture_workspace() -> tempfile::TempDir {
    let tempdir = tempdir().expect("tempdir should exist");
    let docs_root = tempdir.path().join("docs/syu");
    fs::create_dir_all(docs_root.join("philosophy")).expect("philosophy dir");
    fs::create_dir_all(docs_root.join("policies")).expect("policies dir");
    fs::create_dir_all(docs_root.join("requirements")).expect("requirements dir");
    fs::create_dir_all(docs_root.join("features")).expect("features dir");

    fs::write(
        docs_root.join("philosophy/foundation.yaml"),
        "category: Philosophy\nversion: 1\nlanguage: en\nphilosophies:\n  - id: PHIL-GAP-001\n    title: Gap philosophy\n    product_design_principle: Keep gaps visible.\n    coding_guideline: Prefer inspection commands.\n    linked_policies: []\n",
    )
    .expect("philosophy file");
    fs::write(
        docs_root.join("policies/policies.yaml"),
        "category: Policies\nversion: 1\nlanguage: en\npolicies:\n  - id: POL-GAP-001\n    title: Gap policy\n    summary: Leave one sparse requirement.\n    description: Gap policy.\n    linked_philosophies:\n      - PHIL-GAP-001\n    linked_requirements: []\n",
    )
    .expect("policy file");
    fs::write(
        docs_root.join("requirements/core.yaml"),
        "category: Core\nprefix: REQ-GAP\n\nrequirements:\n  - id: REQ-GAP-001\n    title: Sparse requirement\n    description: This requirement intentionally leaves links empty.\n    priority: medium\n    status: planned\n    linked_policies: []\n    linked_features: []\n    tests: {}\n",
    )
    .expect("requirement file");
    fs::write(
        docs_root.join("features/features.yaml"),
        "version: \"1\"\nfiles:\n  - kind: dummy\n    file: dummy.yaml\n",
    )
    .expect("feature registry");
    fs::write(
        docs_root.join("features/dummy.yaml"),
        "category: Dummy\nversion: 1\nfeatures:\n  - id: FEAT-DUMMY-001\n    title: Dummy feature\n    summary: Keeps the workspace loadable.\n    status: planned\n    linked_requirements: []\n    implementations: {}\n",
    )
    .expect("feature file");

    tempdir
}

fn write_root_file_fixture_workspace() -> tempfile::TempDir {
    let tempdir = tempdir().expect("tempdir should exist");
    let docs_root = tempdir.path().join("docs/syu");
    fs::create_dir_all(docs_root.join("philosophy")).expect("philosophy dir");
    fs::create_dir_all(docs_root.join("policies")).expect("policies dir");
    fs::create_dir_all(docs_root.join("requirements")).expect("requirements dir");
    fs::create_dir_all(docs_root.join("features")).expect("features dir");

    fs::write(
        docs_root.join("philosophy/foundation.yaml"),
        "category: Philosophy\nversion: 1\nlanguage: en\nphilosophies:\n  - id: PHIL-ROOT-001\n    title: Root file paths stay explorable\n    product_design_principle: Root-level docs can still be traced before they exist.\n    coding_guideline: Treat repository paths as first-class selectors.\n    linked_policies:\n      - POL-ROOT-001\n",
    )
    .expect("philosophy file");
    fs::write(
        docs_root.join("policies/policies.yaml"),
        "category: Policies\nversion: 1\nlanguage: en\npolicies:\n  - id: POL-ROOT-001\n    title: Root files can be traced\n    summary: Root-level repository files should stay selectable.\n    description: This policy keeps root files queryable even when the file is not present yet.\n    linked_philosophies:\n      - PHIL-ROOT-001\n    linked_requirements:\n      - REQ-ROOT-001\n",
    )
    .expect("policy file");
    fs::write(
        docs_root.join("requirements/core.yaml"),
        "category: Core\nprefix: REQ-ROOT\n\nrequirements:\n  - id: REQ-ROOT-001\n    title: Root file trace\n    description: Root-level files should resolve as path selectors.\n    priority: medium\n    status: planned\n    linked_policies:\n      - POL-ROOT-001\n    linked_features:\n      - FEAT-ROOT-001\n    tests: {}\n",
    )
    .expect("requirement file");
    fs::write(
        docs_root.join("features/features.yaml"),
        "version: \"1\"\nfiles:\n  - kind: root\n    file: root.yaml\n",
    )
    .expect("feature registry");
    fs::write(
        docs_root.join("features/root.yaml"),
        "category: Root\nversion: 1\nfeatures:\n  - id: FEAT-ROOT-001\n    title: Root file selector\n    summary: The relate command should treat README.md and LICENSE as path selectors.\n    status: planned\n    linked_requirements:\n      - REQ-ROOT-001\n    implementations:\n      markdown:\n        - file: README.md\n          symbols: []\n        - file: LICENSE\n          symbols: []\n",
    )
    .expect("feature file");
    fs::write(tempdir.path().join("LICENSE"), "fixture license\n").expect("license file");

    tempdir
}

#[test]
fn relate_command_traverses_the_connected_graph_from_a_requirement() {
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("relate")
        .arg("REQ-TRACE-001")
        .arg(fixture_path("passing"))
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Selection: definition REQ-TRACE-001"));
    assert!(stdout.contains("philosophy PHIL-TRACE-001"));
    assert!(stdout.contains("policy POL-TRACE-001"));
    assert!(stdout.contains("policy POL-TRACE-002"));
    assert!(stdout.contains("feature FEAT-TRACE-001"));
    assert!(!stdout.contains("requirement REQ-TRACE-002"));
    assert!(!stdout.contains("requirement REQ-TRACE-003"));
    assert!(!stdout.contains("feature FEAT-TRACE-002"));
    assert!(!stdout.contains("feature FEAT-TRACE-003"));
    assert!(stdout.contains("src/rust_trace_tests.rs"));
    assert!(stdout.contains("src/rust_feature.rs"));
    assert!(stdout.contains("Gaps:\n- none"));
}

#[test]
fn relate_command_supports_json_output_for_path_selection() {
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .args([
            "relate",
            "src/rust_feature.rs",
            fixture_path("passing").to_str().expect("utf8 path"),
            "--format",
            "json",
        ])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("json output");
    assert_eq!(json["selection"]["kind"], "path");
    assert_eq!(json["selection"]["query"], "src/rust_feature.rs");
    assert_eq!(
        json["direct_matches"]["traces"][0]["owner_id"],
        "FEAT-TRACE-001"
    );
    assert_eq!(json["features"][0]["id"], "FEAT-TRACE-001");
    assert_eq!(json["requirements"][0]["id"], "REQ-TRACE-001");
    assert_eq!(json["features"].as_array().expect("feature array").len(), 1);
    assert_eq!(
        json["requirements"]
            .as_array()
            .expect("requirement array")
            .len(),
        1
    );
}

#[test]
fn relate_command_matches_source_symbols() {
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("relate")
        .arg("feature_trace_rust")
        .arg(fixture_path("passing"))
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Selection: symbol feature_trace_rust"));
    assert!(stdout.contains("feature FEAT-TRACE-001 implementation rust\tsrc/rust_feature.rs"));
    assert!(stdout.contains("(direct match)"));
}

#[test]
fn relate_command_treats_missing_root_level_files_as_path_selectors() {
    let workspace = write_root_file_fixture_workspace();
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .args([
            "relate",
            "README.md",
            workspace.path().to_str().expect("utf8 path"),
            "--format",
            "json",
        ])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("json output");
    assert_eq!(json["selection"]["kind"], "path");
    assert_eq!(json["selection"]["query"], "README.md");
    assert_eq!(
        json["direct_matches"]["traces"][0]["owner_id"],
        "FEAT-ROOT-001"
    );
    assert_eq!(json["features"][0]["id"], "FEAT-ROOT-001");
    assert_eq!(json["requirements"][0]["id"], "REQ-ROOT-001");
}

#[test]
fn relate_command_treats_existing_extensionless_root_files_as_path_selectors() {
    let workspace = write_root_file_fixture_workspace();
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .args([
            "relate",
            "LICENSE",
            workspace.path().to_str().expect("utf8 path"),
            "--format",
            "json",
        ])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("json output");
    assert_eq!(json["selection"]["kind"], "path");
    assert_eq!(json["selection"]["query"], "LICENSE");
    assert_eq!(
        json["direct_matches"]["traces"][0]["owner_id"],
        "FEAT-ROOT-001"
    );
    assert_eq!(json["features"][0]["id"], "FEAT-ROOT-001");
    assert_eq!(json["requirements"][0]["id"], "REQ-ROOT-001");
}

#[test]
fn relate_command_surfaces_sparse_graph_gaps() {
    let workspace = write_gap_fixture_workspace();
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("relate")
        .arg("REQ-GAP-001")
        .arg(workspace.path())
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("requirement `REQ-GAP-001` does not link to any policies"));
    assert!(stdout.contains("requirement `REQ-GAP-001` does not link to any features"));
    assert!(stdout.contains("requirement `REQ-GAP-001` does not declare any test traces"));
}

#[test]
fn relate_command_rejects_parent_directory_path_selectors() {
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("relate")
        .arg("../src/rust_feature.rs")
        .arg(fixture_path("passing"))
        .output()
        .expect("command should run");

    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("must stay inside workspace"));
}

#[test]
fn relate_command_rejects_unknown_selectors() {
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("relate")
        .arg("missing_selector")
        .arg(fixture_path("passing"))
        .output()
        .expect("command should run");

    assert_eq!(output.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("did not match any definition ID, traced path, or traced source symbol")
    );
}

#[test]
fn relate_command_reports_git_range_summary_for_changed_files() {
    let workspace = init_git_fixture_workspace();
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .current_dir(workspace.path().join("workspace"))
        .args(["relate", "--range", "HEAD~1..HEAD"])
        .output()
        .expect("relate range command should run");

    assert!(output.status.success(), "relate range should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Git range: HEAD~1..HEAD"));
    assert!(stdout.contains("Changed files: 1"));
    assert!(stdout.contains("Summary:"));
    assert!(stdout.contains("Philosophies:\n- philosophy PHIL-TRACE-001"));
    assert!(stdout.contains("Policies:\n- policy POL-TRACE-001"));
    assert!(stdout.contains("Requirements:\n- requirement REQ-TRACE-001"));
    assert!(stdout.contains("Features:\n- feature FEAT-TRACE-001"));
}

#[test]
fn relate_command_reports_empty_git_ranges_as_json() {
    let workspace = init_git_fixture_workspace();
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .current_dir(workspace.path().join("workspace"))
        .args(["relate", "--range", "HEAD..HEAD", "--format", "json"])
        .output()
        .expect("relate range command should run");

    assert!(output.status.success(), "empty relate range should succeed");
    let json: Value = serde_json::from_slice(&output.stdout).expect("json output");
    assert_eq!(json["range"], "HEAD..HEAD");
    assert_eq!(json["summary"]["total_files"], 0);
    assert!(
        json["features"]
            .as_array()
            .expect("feature array")
            .is_empty()
    );
}

#[test]
fn relate_command_reports_empty_git_ranges_as_text() {
    let workspace = init_git_fixture_workspace();
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .current_dir(workspace.path().join("workspace"))
        .args(["relate", "--range", "HEAD..HEAD"])
        .output()
        .expect("relate range command should run");

    assert!(output.status.success(), "empty relate range should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Git range: HEAD..HEAD"));
    assert!(stdout.contains("No files changed in range"));
}

#[test]
fn relate_command_supports_git_range_json_output() {
    let workspace = init_git_fixture_workspace();
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .current_dir(workspace.path().join("workspace"))
        .args(["relate", "--range", "HEAD~1..HEAD", "--format", "json"])
        .output()
        .expect("relate range command should run");

    assert!(output.status.success(), "relate range should succeed");
    let json: Value = serde_json::from_slice(&output.stdout).expect("json output");
    assert_eq!(json["summary"]["total_files"], 1);
    assert_eq!(json["summary"]["total_features"], 1);
    assert_eq!(json["features"][0]["id"], "FEAT-TRACE-001");
}

#[test]
fn relate_command_rejects_invalid_git_ranges() {
    let workspace = init_git_fixture_workspace();
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .current_dir(workspace.path().join("workspace"))
        .args(["relate", "--range", "definitely-not-a-range"])
        .output()
        .expect("relate range command should run");

    assert_eq!(output.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("git range `definitely-not-a-range` is not valid")
    );
}

#[test]
fn relate_command_skips_invalid_git_diff_paths_and_collects_definition_matches() {
    let tempdir = tempdir().expect("tempdir should exist");
    let script_dir = tempdir.path().join("bin");
    fs::create_dir_all(&script_dir).expect("script dir");
    let script_path = script_dir.join("git");
    fs::write(
        &script_path,
        "#!/bin/sh\nset -eu\nprintf '../outside.rs\\ndocs/syu/features/features.yaml\\nsrc/rust_feature.rs\\n'\n",
    )
    .expect("fake git");

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let mut permissions = fs::metadata(&script_path).expect("metadata").permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&script_path, permissions).expect("permissions");
    }

    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .current_dir(fixture_path("passing"))
        .env(
            "PATH",
            format!(
                "{}:{}",
                script_dir.display(),
                std::env::var("PATH").expect("PATH env")
            ),
        )
        .args(["relate", "--range", "HEAD~1..HEAD", "--format", "json"])
        .output()
        .expect("relate range command should run");

    assert!(
        output.status.success(),
        "range should ignore invalid diff paths"
    );
    let json: Value = serde_json::from_slice(&output.stdout).expect("json output");
    assert_eq!(json["summary"]["total_files"], 3);
    assert_eq!(json["summary"]["total_features"], 1);
    assert_eq!(json["features"][0]["id"], "FEAT-TRACE-001");
}
