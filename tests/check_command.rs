use assert_cmd::cargo::CommandCargoExt;
use std::{fs, path::Path, path::PathBuf, process::Command};
use tempfile::tempdir;

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/workspaces")
        .join(name)
}

fn expected_workspace_arg(path: &Path) -> String {
    let rendered = path.display().to_string();
    if rendered.is_empty() {
        return if cfg!(windows) {
            "\"\"".to_string()
        } else {
            "''".to_string()
        };
    }

    let is_shell_safe = rendered.chars().all(|ch| {
        ch.is_ascii_alphanumeric()
            || if cfg!(windows) {
                "/\\\\:._-".contains(ch)
            } else {
                "/._-".contains(ch)
            }
    });
    if is_shell_safe {
        rendered
    } else if cfg!(windows) {
        format!("\"{rendered}\"")
    } else {
        format!("'{}'", rendered.replace('\'', "'\\''"))
    }
}

fn write_trace_path_workspace(root: &Path, trace_path: &str) {
    fs::create_dir_all(root.join("docs/syu/philosophy")).expect("philosophy dir");
    fs::create_dir_all(root.join("docs/syu/policies")).expect("policies dir");
    fs::create_dir_all(root.join("docs/syu/requirements")).expect("requirements dir");
    fs::create_dir_all(root.join("docs/syu/features")).expect("features dir");
    fs::create_dir_all(root.join("src")).expect("src dir");

    fs::write(
        root.join("syu.yaml"),
        format!(
            "version: {version}\nspec:\n  root: docs/syu\nvalidate:\n  default_fix: false\n  allow_planned: true\nruntimes:\n  python:\n    command: auto\n  node:\n    command: auto\n",
            version = env!("CARGO_PKG_VERSION"),
        ),
    )
    .expect("config");

    fs::write(
        root.join("docs/syu/philosophy/foundation.yaml"),
        "category: Philosophy\nversion: 1\nlanguage: en\n\nphilosophies:\n  - id: PHIL-001\n    title: Clear repository paths\n    product_design_principle: Keep trace paths portable.\n    coding_guideline: Prefer canonical repository-relative file paths.\n    linked_policies:\n      - POL-001\n",
    )
    .expect("philosophy");

    fs::write(
        root.join("docs/syu/policies/policies.yaml"),
        "category: Policies\nversion: 1\nlanguage: en\n\npolicies:\n  - id: POL-001\n    title: Keep traces portable\n    summary: Trace file paths should stay reviewable across environments.\n    description: Checked-in trace paths should use one portable repository-relative form.\n    linked_philosophies:\n      - PHIL-001\n    linked_requirements:\n      - REQ-001\n",
    )
    .expect("policy");

    fs::write(
        root.join("docs/syu/requirements/core.yaml"),
        format!(
            "category: Core Requirements\nprefix: REQ\n\nrequirements:\n  - id: REQ-001\n    title: Verify canonical trace paths\n    description: Requirement traces should stay portable across reviewers and CI.\n    priority: high\n    status: implemented\n    linked_policies:\n      - POL-001\n    linked_features:\n      - FEAT-001\n    tests:\n      rust:\n        - file: {trace_path}\n          symbols:\n            - trace_symbol\n",
        ),
    )
    .expect("requirement");

    fs::write(
        root.join("docs/syu/features/features.yaml"),
        format!(
            "version: \"{}\"\nfiles:\n  - kind: core\n    file: core.yaml\n",
            env!("CARGO_PKG_VERSION")
        ),
    )
    .expect("feature registry");

    fs::write(
        root.join("docs/syu/features/core.yaml"),
        "category: Core Features\nversion: 1\n\nfeatures:\n  - id: FEAT-001\n    title: Verify canonical trace paths\n    summary: Feature implementations should keep portable trace paths.\n    status: implemented\n    linked_requirements:\n      - REQ-001\n    implementations:\n      rust:\n        - file: src/trace.rs\n          symbols:\n            - trace_symbol\n",
    )
    .expect("feature");

    fs::write(root.join("src/trace.rs"), "pub fn trace_symbol() {}\n").expect("trace source");
}

fn write_unregistered_feature_workspace(root: &Path) {
    fs::create_dir_all(root.join("docs/syu/philosophy")).expect("philosophy dir");
    fs::create_dir_all(root.join("docs/syu/policies")).expect("policies dir");
    fs::create_dir_all(root.join("docs/syu/requirements/core")).expect("requirements dir");
    fs::create_dir_all(root.join("docs/syu/features/core")).expect("core features dir");
    fs::create_dir_all(root.join("docs/syu/features/extra")).expect("extra features dir");

    fs::write(
        root.join("syu.yaml"),
        format!(
            "version: {version}\nspec:\n  root: docs/syu\nvalidate:\n  default_fix: false\n  allow_planned: true\n  require_non_orphaned_items: true\n  require_reciprocal_links: true\n  require_symbol_trace_coverage: false\napp:\n  bind: 127.0.0.1\n  port: 3000\nruntimes:\n  python:\n    command: auto\n  node:\n    command: auto\n",
            version = env!("CARGO_PKG_VERSION"),
        ),
    )
    .expect("config");

    fs::write(
        root.join("docs/syu/philosophy/foundation.yaml"),
        "category: Philosophy\nversion: 1\nlanguage: en\n\nphilosophies:\n  - id: PHIL-001\n    title: Keep feature discovery explicit\n    product_design_principle: Feature documents should not hide from review.\n    coding_guideline: Keep the feature registry aligned with the checked-in file tree.\n    linked_policies:\n      - POL-001\n",
    )
    .expect("philosophy");

    fs::write(
        root.join("docs/syu/policies/policies.yaml"),
        "category: Policies\nversion: 1\nlanguage: en\n\npolicies:\n  - id: POL-001\n    title: Register every feature document\n    summary: Feature discovery should remain explicit.\n    description: syu should catch feature YAML files that drift away from the registry.\n    linked_philosophies:\n      - PHIL-001\n    linked_requirements:\n      - REQ-001\n",
    )
    .expect("policy");

    fs::write(
        root.join("docs/syu/requirements/core/core.yaml"),
        "category: Core Requirements\nprefix: REQ\n\nrequirements:\n  - id: REQ-001\n    title: Registered feature files stay discoverable\n    description: Feature docs should remain visible to list and browse flows.\n    priority: high\n    status: planned\n    linked_policies:\n      - POL-001\n    linked_features:\n      - FEAT-001\n    tests: {}\n",
    )
    .expect("requirement");

    fs::write(
        root.join("docs/syu/features/features.yaml"),
        format!(
            "version: \"{}\"\nupdated: \"generated by test\"\n\nfiles:\n  - kind: core\n    file: core/core.yaml\n",
            env!("CARGO_PKG_VERSION")
        ),
    )
    .expect("feature registry");

    fs::write(
        root.join("docs/syu/features/core/core.yaml"),
        "category: Core Features\nversion: 1\n\nfeatures:\n  - id: FEAT-001\n    title: Registered feature\n    summary: This feature is declared in the registry.\n    status: planned\n    linked_requirements:\n      - REQ-001\n    implementations: {}\n",
    )
    .expect("registered feature");

    fs::write(
        root.join("docs/syu/features/extra/stray.yaml"),
        "category: Extra Features\nversion: 1\n\nfeatures:\n  - id: FEAT-EXTRA-001\n    title: Stray feature file\n    summary: This file exists on disk but is missing from the registry.\n    status: planned\n    linked_requirements:\n      - REQ-001\n    implementations: {}\n",
    )
    .expect("unregistered feature");
}

#[test]
// REQ-CORE-001
fn check_command_accepts_passing_workspace() {
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("validate")
        .arg(fixture_path("passing"))
        .output()
        .expect("command should run");

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("syu validate passed"));
    assert!(stdout.contains("checks:"));
    assert!(stdout.contains("workspace items"));
    assert!(stdout.contains("(workspace, graph, delivery, trace)"));
    assert!(stdout.contains("validate.require_symbol_trace_coverage=false"));
    assert!(stdout.contains(
        "traceability: requirements=6/6 traces validated; features=6/6 traces validated"
    ));
    assert!(
        stdout.contains("What to do next:"),
        "success output should include next-step guidance: {stdout}"
    );
    assert!(
        stdout.contains("syu app "),
        "next-step block should mention syu app: {stdout}"
    );
}

#[test]
// REQ-CORE-001
fn check_command_preserves_default_workspace_dot_in_next_steps() {
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .current_dir(fixture_path("passing"))
        .arg("validate")
        .output()
        .expect("command should run");

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("syu app ."));
    assert!(stdout.contains("syu browse ."));
    assert!(stdout.contains("syu report ."));
    assert!(stdout.contains("syu show <ID> ."));
}

#[test]
// REQ-CORE-001
fn check_command_prints_workspace_aware_next_steps_for_explicit_paths() {
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("validate")
        .arg(fixture_path("passing"))
        .output()
        .expect("command should run");

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let workspace_arg = expected_workspace_arg(&fixture_path("passing"));
    assert!(stdout.contains(&format!("syu app {workspace_arg}")));
    assert!(stdout.contains(&format!("syu browse {workspace_arg}")));
    assert!(stdout.contains(&format!("syu report {workspace_arg}")));
    assert!(stdout.contains(&format!("syu show <ID> {workspace_arg}")));
}

#[test]
// REQ-CORE-001
fn check_command_quiet_suppresses_success_summary_and_next_step_guidance() {
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("validate")
        .arg(fixture_path("passing"))
        .arg("--quiet")
        .output()
        .expect("command should run");

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("syu validate passed"));
    assert!(!stdout.contains("workspace:"));
    assert!(!stdout.contains("definitions:"));
    assert!(!stdout.contains("checks:"));
    assert!(!stdout.contains("traceability:"));
    assert!(
        !stdout.contains("What to do next:"),
        "quiet mode should suppress next-step guidance: {stdout}"
    );
}

#[test]
// REQ-CORE-001
fn check_command_quiet_suppresses_failure_guidance() {
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("validate")
        .arg(fixture_path("failing"))
        .arg("--quiet")
        .output()
        .expect("command should run");

    assert!(
        !output.status.success(),
        "failing fixture should fail quietly"
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("syu validate failed"));
    assert!(!stdout.contains("What to inspect next:"));
    assert!(!stdout.contains("syu show <ID>"));
    assert!(!stdout.contains("--severity error"));
    assert!(!stdout.contains("--genre graph"));
}

#[test]
// REQ-CORE-001
fn check_command_quiet_suppresses_filtered_footer_on_success() {
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("validate")
        .arg(fixture_path("passing"))
        .arg("--quiet")
        .arg("--severity")
        .arg("error")
        .output()
        .expect("command should run");

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.trim(), "syu validate passed (filtered view)");
}

#[test]
// REQ-CORE-001
fn check_command_reports_missing_definition_links() {
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("validate")
        .arg(fixture_path("failing"))
        .output()
        .expect("command should run");

    assert!(!output.status.success(), "failing fixture should fail");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("SYU-graph-reference-001"));
    assert!(stdout.contains("referenced rules:"));
    assert!(stdout.contains("Linked definitions must exist"));
    assert!(stdout.contains("REQ-MISSING-999"));
    assert!(stdout.contains("What to inspect next:"));
    assert!(stdout.contains("syu show <ID>"));
    assert!(
        stdout.contains("inspect a specific requirement, feature, policy, or philosophy by ID")
    );
    assert!(stdout.contains("--severity error"));
    assert!(stdout.contains("--genre graph"));
    assert!(stdout.contains("syu app "));
}

#[test]
// REQ-CORE-001
fn check_command_failure_guidance_avoids_named_above_wording_for_empty_filtered_views() {
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("validate")
        .arg(fixture_path("failing"))
        .arg("--id")
        .arg("REQ-NOT-FOUND")
        .output()
        .expect("command should run");

    assert!(
        !output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("syu validate failed (filtered view)"));
    assert!(stdout.contains("no issues matched the active filters."));
    assert!(
        stdout.contains("inspect a specific requirement, feature, policy, or philosophy by ID")
    );
    assert!(!stdout.contains("named above"));
}

#[test]
// REQ-CORE-001
fn check_command_reports_feature_documents_missing_from_registry() {
    let tempdir = tempdir().expect("tempdir should exist");
    write_unregistered_feature_workspace(tempdir.path());

    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("validate")
        .arg(tempdir.path())
        .output()
        .expect("command should run");

    assert!(
        !output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("SYU-workspace-registry-001"),
        "stdout:\n{stdout}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        stdout.contains("docs/syu/features/features.yaml"),
        "stdout:\n{stdout}"
    );
    assert!(
        stdout.contains("docs/syu/features/extra/stray.yaml"),
        "stdout:\n{stdout}"
    );
    assert!(
        stdout.contains("Add a `files` entry for `extra/stray.yaml`"),
        "stdout:\n{stdout}"
    );
}

#[test]
// REQ-CORE-001
fn check_command_reports_malformed_unregistered_feature_documents_on_candidate_path() {
    let tempdir = tempdir().expect("tempdir should exist");
    write_unregistered_feature_workspace(tempdir.path());
    fs::write(
        tempdir.path().join("docs/syu/features/extra/stray.yaml"),
        "features: [",
    )
    .expect("invalid feature candidate should exist");

    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("validate")
        .arg(tempdir.path())
        .output()
        .expect("command should run");

    assert!(
        !output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("SYU-workspace-registry-001"),
        "stdout:\n{stdout}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        stdout.contains("docs/syu/features/extra/stray.yaml"),
        "stdout:\n{stdout}"
    );
    assert!(
        stdout.contains("failed to parse feature candidate"),
        "stdout:\n{stdout}"
    );
    assert!(
        stdout.contains("Fix `docs/syu/features/extra/stray.yaml`"),
        "stdout:\n{stdout}"
    );
    assert!(
        !stdout
            .contains("Failed to compare feature files against `docs/syu/features/features.yaml`"),
        "stdout:\n{stdout}"
    );
}

#[test]
// REQ-CORE-001
fn check_command_filters_visible_issues_by_spec_id() {
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("validate")
        .arg(fixture_path("failing"))
        .arg("--id")
        .arg("REQ-FAIL-001")
        .output()
        .expect("command should run");

    assert!(
        !output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("syu validate failed (filtered view)"));
    assert!(stdout.contains("filters: id=REQ-FAIL-001"));
    assert!(stdout.contains("showing 1 of 3 issues after filtering"));
    assert!(stdout.contains("requirement REQ-FAIL-001"));
    assert!(!stdout.contains("feature FEAT-FAIL-001"));
    assert!(!stdout.contains("policy POL-FAIL-001"));
}

#[test]
fn check_command_suggests_init_for_uninitialized_workspaces() {
    let tempdir = tempdir().expect("tempdir should exist");
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("validate")
        .arg(tempdir.path())
        .output()
        .expect("command should run");

    assert!(!output.status.success(), "empty workspace should fail");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("SYU-workspace-load-001"));
    assert!(stdout.contains("suggestion:"));
    assert!(stdout.contains("syu init ."));
}

#[test]
// REQ-CORE-002
fn check_command_verifies_requirement_test_traceability_in_all_supported_languages() {
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("validate")
        .arg(fixture_path("passing"))
        .arg("--format")
        .arg("json")
        .output()
        .expect("command should run");

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("output should be valid JSON");
    assert_eq!(json["trace_summary"]["requirement_traces"]["declared"], 6);
    assert_eq!(json["trace_summary"]["requirement_traces"]["validated"], 6);
}

#[test]
// REQ-CORE-002
fn check_command_verifies_feature_implementation_traceability_in_all_supported_languages() {
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("validate")
        .arg(fixture_path("passing"))
        .arg("--format")
        .arg("json")
        .output()
        .expect("command should run");

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("output should be valid JSON");
    assert_eq!(json["trace_summary"]["feature_traces"]["declared"], 6);
    assert_eq!(json["trace_summary"]["feature_traces"]["validated"], 6);
}

#[test]
fn check_alias_still_invokes_validate() {
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("check")
        .arg(fixture_path("passing"))
        .output()
        .expect("command should run");

    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("syu validate passed"));
}

#[test]
// REQ-CORE-001
fn check_command_reports_disabled_validation_toggles_from_config() {
    let tempdir = tempdir().expect("tempdir should exist");
    write_trace_path_workspace(tempdir.path(), "src/trace.rs");
    fs::write(
        tempdir.path().join("syu.yaml"),
        format!(
            "version: {version}\nspec:\n  root: docs/syu\nvalidate:\n  default_fix: false\n  allow_planned: true\n  require_non_orphaned_items: false\n  require_reciprocal_links: false\n  require_symbol_trace_coverage: false\nruntimes:\n  python:\n    command: auto\n  node:\n    command: auto\n",
            version = env!("CARGO_PKG_VERSION"),
        ),
    )
    .expect("config");

    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("validate")
        .arg(tempdir.path())
        .output()
        .expect("command should run");

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("validate.require_non_orphaned_items=false"));
    assert!(stdout.contains("validate.require_reciprocal_links=false"));
    assert!(stdout.contains("validate.require_symbol_trace_coverage=false"));
}

#[test]
// REQ-CORE-002
fn check_command_accepts_trace_workspaces_without_inline_spec_ids() {
    let tempdir = tempdir().expect("tempdir should exist");
    write_trace_path_workspace(tempdir.path(), "src/trace.rs");

    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("validate")
        .arg(tempdir.path())
        .output()
        .expect("command should run");

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
// REQ-CORE-002
fn check_command_warns_for_non_canonical_relative_trace_paths() {
    let tempdir = tempdir().expect("tempdir should exist");
    write_trace_path_workspace(tempdir.path(), "./src/../src/trace.rs");

    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("validate")
        .arg(tempdir.path())
        .output()
        .expect("command should run");

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("SYU-trace-file-003"));
    assert!(stdout.contains("src/trace.rs"));
}

#[test]
// REQ-CORE-002
fn check_command_warns_for_backslash_trace_paths() {
    let tempdir = tempdir().expect("tempdir should exist");
    write_trace_path_workspace(tempdir.path(), "src\\\\trace.rs");

    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("validate")
        .arg(tempdir.path())
        .output()
        .expect("command should run");

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("SYU-trace-file-003"));
    assert!(stdout.contains("src/trace.rs"));
}
