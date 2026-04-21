use assert_cmd::cargo::CommandCargoExt;
use std::{fs, path::Path, process::Command};
use tempfile::tempdir;

fn write_workspace(
    root: &Path,
    allow_planned: bool,
    requirement_status: &str,
    feature_status: &str,
    include_requirement_traces: bool,
    include_feature_traces: bool,
) {
    fs::create_dir_all(root.join("docs/syu/philosophy")).expect("philosophy dir");
    fs::create_dir_all(root.join("docs/syu/policies")).expect("policies dir");
    fs::create_dir_all(root.join("docs/syu/requirements")).expect("requirements dir");
    fs::create_dir_all(root.join("docs/syu/features")).expect("features dir");
    if include_requirement_traces || include_feature_traces {
        fs::create_dir_all(root.join("src")).expect("src dir");
        fs::write(
            root.join("src/trace.rs"),
            "// REQ-001\n// FEAT-001\npub fn req_trace() {}\n",
        )
        .expect("trace source");
    }

    fs::write(
        root.join("syu.yaml"),
        format!(
            "version: {version}\nspec:\n  root: docs/syu\nvalidate:\n  default_fix: false\n  allow_planned: {allow_planned}\nruntimes:\n  python:\n    command: auto\n  node:\n    command: auto\n",
            version = env!("CARGO_PKG_VERSION"),
            allow_planned = if allow_planned { "true" } else { "false" },
        ),
    )
    .expect("config");

    fs::write(
        root.join("docs/syu/philosophy/foundation.yaml"),
        "category: Philosophy\nversion: 1\nlanguage: en\n\nphilosophies:\n  - id: PHIL-001\n    title: Executable agreement\n    product_design_principle: Keep change traceable.\n    coding_guideline: Prefer explicit links.\n    linked_policies:\n      - POL-001\n",
    )
    .expect("philosophy");

    fs::write(
        root.join("docs/syu/policies/policies.yaml"),
        "category: Policies\nversion: 1\nlanguage: en\n\npolicies:\n  - id: POL-001\n    title: Keep delivery states explicit\n    summary: Planned items stay unimplemented until traces exist.\n    description: Delivery state controls whether traces must be absent or present.\n    linked_philosophies:\n      - PHIL-001\n    linked_requirements:\n      - REQ-001\n",
    )
    .expect("policy");

    let requirement_traces = if include_requirement_traces {
        "    tests:\n      rust:\n        - file: src/trace.rs\n          symbols:\n            - req_trace\n"
    } else {
        "    tests: {}\n"
    };
    fs::write(
        root.join("docs/syu/requirements/core.yaml"),
        format!(
            "category: Core Requirements\nprefix: REQ\n\nrequirements:\n  - id: REQ-001\n    title: Keep requirement delivery state explicit\n    description: Requirement delivery state must match trace expectations.\n    priority: high\n    status: {status}\n    linked_policies:\n      - POL-001\n    linked_features:\n      - FEAT-001\n{traces}",
            status = requirement_status,
            traces = requirement_traces,
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

    let feature_traces = if include_feature_traces {
        "    implementations:\n      rust:\n        - file: src/trace.rs\n          symbols:\n            - req_trace\n"
    } else {
        "    implementations: {}\n"
    };
    fs::write(
        root.join("docs/syu/features/core.yaml"),
        format!(
            "category: Core Features\nversion: 1\n\nfeatures:\n  - id: FEAT-001\n    title: Keep feature delivery state explicit\n    summary: Feature delivery state must match implementation traces.\n    status: {status}\n    linked_requirements:\n      - REQ-001\n{traces}",
            status = feature_status,
            traces = feature_traces,
        ),
    )
    .expect("feature");
}

#[test]
// REQ-CORE-001
fn validate_accepts_planned_entries_without_traces() {
    let tempdir = tempdir().expect("tempdir should exist");
    write_workspace(tempdir.path(), true, "planned", "planned", false, false);

    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("validate")
        .arg(tempdir.path())
        .output()
        .expect("validate should run");

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
// REQ-CORE-001
fn validate_rejects_implemented_entries_without_traces() {
    let tempdir = tempdir().expect("tempdir should exist");
    write_workspace(
        tempdir.path(),
        true,
        "implemented",
        "implemented",
        false,
        false,
    );

    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("validate")
        .arg(tempdir.path())
        .output()
        .expect("validate should run");

    assert!(!output.status.success(), "implemented entries should fail");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("SYU-delivery-implemented-001"));
}

#[test]
// REQ-CORE-001
fn validate_spec_only_accepts_implemented_entries_without_traces() {
    let tempdir = tempdir().expect("tempdir should exist");
    write_workspace(
        tempdir.path(),
        true,
        "implemented",
        "implemented",
        false,
        false,
    );

    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .args(["validate", "--spec-only"])
        .arg(tempdir.path())
        .output()
        .expect("validate should run");

    assert!(
        output.status.success(),
        "spec-only validation should skip trace enforcement\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains("SYU-delivery-implemented-001"));
    assert!(stdout.contains(
        "traceability: skipped (--spec-only disables requirement and feature trace enforcement)"
    ));
    assert!(!stdout.contains(
        "traceability: requirements=0/0 traces validated; features=0/0 traces validated"
    ));
}

#[test]
// REQ-CORE-001
fn validate_rejects_planned_entries_with_traces() {
    let tempdir = tempdir().expect("tempdir should exist");
    write_workspace(tempdir.path(), true, "planned", "planned", true, true);

    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("validate")
        .arg(tempdir.path())
        .output()
        .expect("validate should run");

    assert!(
        !output.status.success(),
        "planned entries with traces should fail"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("SYU-delivery-planned-002"));
}

#[test]
// REQ-CORE-001
fn validate_rejects_planned_entries_when_config_disallows_them() {
    let tempdir = tempdir().expect("tempdir should exist");
    write_workspace(tempdir.path(), false, "planned", "planed", false, false);

    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("validate")
        .arg(tempdir.path())
        .output()
        .expect("validate should run");

    assert!(!output.status.success(), "planned entries should fail");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("SYU-delivery-planned-001"));
}

#[test]
// REQ-CORE-001
fn validate_warns_when_planned_requirement_links_to_implemented_feature() {
    let tempdir = tempdir().expect("tempdir should exist");
    write_workspace(tempdir.path(), true, "planned", "implemented", false, true);

    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("validate")
        .arg(tempdir.path())
        .output()
        .expect("validate should run");

    assert!(
        output.status.success(),
        "warning-only validation should pass\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("SYU-delivery-agreement-001"));
    assert!(stdout.contains("planned"));
    assert!(stdout.contains("implemented features"));
    assert!(stdout.contains("FEAT-001"));
}

#[test]
// REQ-CORE-001
fn validate_warns_when_implemented_feature_links_only_to_planned_requirements() {
    let tempdir = tempdir().expect("tempdir should exist");
    write_workspace(tempdir.path(), true, "planned", "implemented", false, true);

    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("validate")
        .arg(tempdir.path())
        .output()
        .expect("validate should run");

    assert!(
        output.status.success(),
        "warning-only validation should pass\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("SYU-delivery-agreement-001"));
    assert!(stdout.contains("only to planned requirements"));
    assert!(stdout.contains("REQ-001"));
}

#[test]
// REQ-CORE-001
fn validate_can_use_a_custom_warning_exit_code_for_warning_only_runs() {
    let tempdir = tempdir().expect("tempdir should exist");
    write_workspace(tempdir.path(), true, "planned", "implemented", false, true);

    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("validate")
        .arg(tempdir.path())
        .arg("--warning-exit-code")
        .arg("3")
        .output()
        .expect("validate should run");

    assert_eq!(
        output.status.code(),
        Some(3),
        "warning-only validation should use the configured warning exit code\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("SYU-delivery-agreement-001"));
}

#[test]
// REQ-CORE-001
fn validate_keeps_error_exit_code_when_warning_exit_code_is_configured() {
    let tempdir = tempdir().expect("tempdir should exist");
    write_workspace(
        tempdir.path(),
        true,
        "implemented",
        "implemented",
        false,
        false,
    );

    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("validate")
        .arg(tempdir.path())
        .arg("--warning-exit-code")
        .arg("3")
        .output()
        .expect("validate should run");

    assert_eq!(
        output.status.code(),
        Some(1),
        "error-bearing validation should keep the normal failure exit code\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("SYU-delivery-implemented-001"));
}

#[test]
// REQ-CORE-001
fn validate_rejects_zero_warning_exit_code() {
    let tempdir = tempdir().expect("tempdir should exist");
    write_workspace(tempdir.path(), true, "planned", "implemented", false, true);

    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("validate")
        .arg(tempdir.path())
        .arg("--warning-exit-code")
        .arg("0")
        .output()
        .expect("validate should run");

    assert_eq!(output.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("invalid value '0' for '--warning-exit-code <CODE>'"),
        "zero warning exit code should be rejected\nstderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
