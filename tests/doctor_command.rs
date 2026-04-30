// REQ-CORE-026

use assert_cmd::cargo::CommandCargoExt;
use serde_json::Value;
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};
use tempfile::tempdir;

fn write_doctor_workspace() -> tempfile::TempDir {
    let tempdir = tempdir().expect("tempdir should exist");
    let docs_root = tempdir.path().join("docs/syu");
    fs::create_dir_all(docs_root.join("philosophy")).expect("philosophy dir");
    fs::create_dir_all(docs_root.join("policies")).expect("policies dir");
    fs::create_dir_all(docs_root.join("requirements")).expect("requirements dir");
    fs::create_dir_all(docs_root.join("features/cli")).expect("feature dir");
    fs::create_dir_all(tempdir.path().join("app/node_modules")).expect("app node_modules");
    fs::create_dir_all(tempdir.path().join("website/node_modules")).expect("website node_modules");
    fs::create_dir_all(tempdir.path().join("nested/work/tree")).expect("nested dir");

    fs::write(
        tempdir.path().join("syu.yaml"),
        format!("version: {}\nspec:\n  root: docs/syu\nvalidate:\n  default_fix: false\n  allow_planned: true\n  require_non_orphaned_items: true\n  require_reciprocal_links: true\n  require_symbol_trace_coverage: false\nruntimes:\n  python:\n    command: auto\n  node:\n    command: auto\n", env!("CARGO_PKG_VERSION")),
    )
    .expect("syu config");
    fs::write(
        docs_root.join("philosophy/foundation.yaml"),
        "category: Philosophy\nversion: 1\nlanguage: en\nphilosophies:\n  - id: PHIL-DOCTOR-001\n    title: Doctor checks should be explicit.\n    product_design_principle: Keep local readiness visible before contributor checks run.\n    coding_guideline: Prefer one command that explains what is missing.\n    linked_policies:\n      - POL-DOCTOR-001\n",
    )
    .expect("philosophy");
    fs::write(
        docs_root.join("policies/policies.yaml"),
        "category: Policies\nversion: 1\nlanguage: en\npolicies:\n  - id: POL-DOCTOR-001\n    title: Contributor setup should stay visible.\n    summary: Local prerequisites should be obvious before commands fail.\n    description: Readiness checks should point contributors to the next command.\n    linked_philosophies:\n      - PHIL-DOCTOR-001\n    linked_requirements:\n      - REQ-DOCTOR-001\n",
    )
    .expect("policy");
    fs::write(
        docs_root.join("requirements/core.yaml"),
        "category: Core\nprefix: REQ-DOCTOR\n\nrequirements:\n  - id: REQ-DOCTOR-001\n    title: Doctor output should stay structured.\n    description: The workspace only exists to let the command resolve a root in tests.\n    priority: medium\n    status: implemented\n    linked_policies:\n      - POL-DOCTOR-001\n    linked_features:\n      - FEAT-DOCTOR-LOCAL-001\n",
    )
    .expect("requirement");
    fs::write(
        docs_root.join("features/features.yaml"),
        "version: \"1\"\nfiles:\n  - kind: doctor\n    file: cli/doctor.yaml\n",
    )
    .expect("feature registry");
    fs::write(
        docs_root.join("features/cli/doctor.yaml"),
        "category: Doctor\nversion: 1\nfeatures:\n  - id: FEAT-DOCTOR-LOCAL-001\n    title: Local doctor fixture\n    summary: Test-only feature registry entry.\n    status: implemented\n    linked_requirements:\n      - REQ-DOCTOR-001\n    implementations:\n      rust:\n        - file: src/doctor_fixture.rs\n          symbols:\n            - doctor_fixture\n",
    )
    .expect("feature");

    fs::write(
        tempdir.path().join("Cargo.toml"),
        "[package]\nrust-version = \"1.88\"\n",
    )
    .expect("cargo toml");
    fs::write(
        tempdir.path().join("app/package.json"),
        "{\n  \"name\": \"doctor-app\",\n  \"private\": true,\n  \"packageManager\": \"npm@11.8.0\"\n}\n",
    )
    .expect("app package");
    fs::write(tempdir.path().join("app/.nvmrc"), "25\n").expect("app nvmrc");
    fs::write(tempdir.path().join("app/package-lock.json"), "{}\n").expect("app lockfile");
    fs::write(
        tempdir.path().join("app/node_modules/.package-lock.json"),
        "{}\n",
    )
    .expect("app marker");

    fs::write(
        tempdir.path().join("website/package.json"),
        "{\n  \"name\": \"doctor-website\",\n  \"private\": true,\n  \"packageManager\": \"npm@11.8.0\"\n}\n",
    )
    .expect("website package");
    fs::write(tempdir.path().join("website/.nvmrc"), "20\n").expect("website nvmrc");
    fs::write(tempdir.path().join("website/package-lock.json"), "{}\n").expect("website lockfile");
    fs::write(
        tempdir
            .path()
            .join("website/node_modules/.package-lock.json"),
        "{}\n",
    )
    .expect("website marker");

    tempdir
}

#[test]
fn doctor_command_reports_known_checks_in_json_output() {
    let workspace = write_doctor_workspace();
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .args([
            "doctor",
            workspace
                .path()
                .join("nested/work/tree")
                .to_str()
                .expect("utf8 path"),
            "--format",
            "json",
        ])
        .output()
        .expect("doctor command should run");

    let json: Value =
        serde_json::from_slice(&output.stdout).expect("doctor output should be valid JSON");
    let checks = json["checks"]
        .as_array()
        .expect("checks should be an array");
    let ids = checks
        .iter()
        .filter_map(|entry| entry["id"].as_str())
        .collect::<Vec<_>>();

    assert_eq!(
        json["workspace_root"].as_str(),
        workspace
            .path()
            .canonicalize()
            .ok()
            .and_then(|path| path.to_str().map(str::to_string))
            .as_deref()
    );
    for expected in [
        "workspace-config",
        "rustc-version",
        "cargo-version",
        "rust-msrv",
        "app-node",
        "app-npm",
        "app-deps",
        "website-node",
        "website-npm",
        "website-deps",
        "playwright-chromium",
    ] {
        assert!(
            ids.contains(&expected),
            "missing check id `{expected}` in {ids:?}"
        );
    }
}

#[test]
fn doctor_command_renders_text_summary() {
    let workspace = write_doctor_workspace();
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .args(["doctor", workspace.path().to_str().expect("utf8 path")])
        .output()
        .expect("doctor command should run");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("workspace:"));
    assert!(stdout.contains("summary:"));
    assert!(stdout.contains("Browser app Node"));
    assert!(stdout.contains("Docs site dependencies"));
    assert!(stdout.contains("Playwright Chromium"));
}

#[test]
fn doctor_command_uses_subprocess_env_for_runtime_checks() {
    let workspace = write_doctor_workspace();
    let tools_dir = workspace.path().join("fake-tools");
    fs::create_dir_all(&tools_dir).expect("fake tools dir");
    write_mock_command(
        &tools_dir,
        "rustc",
        "printf 'rustc 1.87.0 (abc)\\n'\nexit 0\n",
    );
    write_mock_command(
        &tools_dir,
        cargo_command_name(),
        "printf 'cargo 1.88.1 (abc)\\n'\nexit 0\n",
    );
    write_mock_command(&tools_dir, "node", "printf 'v25.0.0\\n'\nexit 0\n");
    write_mock_command(
        &tools_dir,
        npm_command_name(),
        "printf '11.8.0\\n'\nexit 0\n",
    );

    let cache_root = workspace.path().join("pw-cache");
    fs::create_dir_all(cache_root.join("chromium-1000")).expect("chromium cache");

    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .args([
            "doctor",
            workspace.path().to_str().expect("utf8 path"),
            "--format",
            "json",
        ])
        .env("PATH", &tools_dir)
        .env("PLAYWRIGHT_BROWSERS_PATH", &cache_root)
        .output()
        .expect("doctor command should run");

    let json: Value =
        serde_json::from_slice(&output.stdout).expect("doctor output should be valid JSON");
    assert_eq!(
        status_for_check(&json, "rust-msrv"),
        Some("warning"),
        "{json:#}"
    );
    assert_eq!(
        status_for_check(&json, "playwright-chromium"),
        Some("ok"),
        "{json:#}"
    );
    assert_eq!(status_for_check(&json, "app-node"), Some("ok"), "{json:#}");
}

#[test]
fn doctor_command_reports_missing_toolchains_without_leaking_env() {
    let workspace = write_doctor_workspace();
    let empty_bin = workspace.path().join("empty-bin");
    fs::create_dir_all(&empty_bin).expect("empty bin dir");

    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .args([
            "doctor",
            workspace.path().to_str().expect("utf8 path"),
            "--format",
            "json",
        ])
        .env("PATH", &empty_bin)
        .env_remove("PLAYWRIGHT_BROWSERS_PATH")
        .output()
        .expect("doctor command should run");

    let json: Value =
        serde_json::from_slice(&output.stdout).expect("doctor output should be valid JSON");
    assert_eq!(
        status_for_check(&json, "rustc-version"),
        Some("error"),
        "{json:#}"
    );
    assert_eq!(
        status_for_check(&json, "cargo-version"),
        Some("error"),
        "{json:#}"
    );
    assert_eq!(
        status_for_check(&json, "rust-msrv"),
        Some("error"),
        "{json:#}"
    );
}

fn status_for_check<'a>(json: &'a Value, id: &str) -> Option<&'a str> {
    json["checks"].as_array().and_then(|checks| {
        checks.iter().find_map(|check| {
            (check["id"].as_str() == Some(id))
                .then(|| check["status"].as_str())
                .flatten()
        })
    })
}

fn write_mock_command(dir: &Path, name: &str, body: &str) -> PathBuf {
    let path = dir.join(name);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::write(&path, format!("#!/bin/sh\nset -eu\n{body}")).expect("mock command");
        let mut permissions = fs::metadata(&path).expect("metadata").permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&path, permissions).expect("permissions");
    }
    #[cfg(windows)]
    {
        fs::write(&path, format!("@echo off\r\n{body}")).expect("mock command");
    }
    path
}

fn npm_command_name() -> &'static str {
    if cfg!(windows) { "npm.cmd" } else { "npm" }
}

fn cargo_command_name() -> &'static str {
    if cfg!(windows) { "cargo.exe" } else { "cargo" }
}
