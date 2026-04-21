use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::Command,
};

use tempfile::tempdir;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn write_executable(path: &Path, contents: &str) {
    fs::write(path, contents).expect("write file");
    let mut perms = fs::metadata(path).expect("metadata").permissions();
    perms.set_mode(0o755);
    fs::set_permissions(path, perms).expect("chmod");
}

fn write_fixture_repo(root: &Path) {
    fs::create_dir_all(root.join("scripts/ci")).expect("scripts dir");
    fs::create_dir_all(root.join("app")).expect("app dir");
    fs::create_dir_all(root.join("website")).expect("website dir");
    fs::create_dir_all(root.join("editors/vscode")).expect("vscode dir");

    let bootstrap =
        fs::read_to_string(repo_root().join("scripts/ci/bootstrap-contributor-tooling.sh"))
            .expect("read bootstrap script");
    write_executable(
        &root.join("scripts/ci/bootstrap-contributor-tooling.sh"),
        &bootstrap,
    );
    write_executable(
        &root.join("scripts/ci/pinned-npm.sh"),
        r#"#!/usr/bin/env bash
set -euo pipefail
printf 'PINNED %s\n' "$*" >>"$BOOTSTRAP_LOG"
"#,
    );
    write_executable(
        &root.join("scripts/ci/install-docs-site-deps.sh"),
        r#"#!/usr/bin/env bash
set -euo pipefail
printf 'INSTALL_DOCS_SITE_DEPS\n' >>"$BOOTSTRAP_LOG"
"#,
    );
    fs::write(root.join("app/.nvmrc"), "25\n").expect("app nvmrc");
    fs::write(root.join("website/.nvmrc"), "20\n").expect("website nvmrc");
    fs::write(root.join("editors/vscode/.nvmrc"), "20\n").expect("vscode nvmrc");
}

fn write_mock_bin(root: &Path) -> PathBuf {
    let bin_dir = root.join("bin");
    fs::create_dir_all(&bin_dir).expect("bin dir");
    write_executable(
        &bin_dir.join("node"),
        r#"#!/usr/bin/env bash
set -euo pipefail
if [[ "$1" == "-p" ]]; then
  printf '%s\n' "$NODE_MAJOR"
  exit 0
fi
printf 'unexpected node invocation: %s\n' "$*" >&2
exit 1
"#,
    );
    write_executable(
        &bin_dir.join("npm"),
        r#"#!/usr/bin/env bash
set -euo pipefail
printf 'NPM %s\n' "$*" >>"$BOOTSTRAP_LOG"
"#,
    );
    write_executable(
        &bin_dir.join("npx"),
        r#"#!/usr/bin/env bash
set -euo pipefail
printf 'NPX %s\n' "$*" >>"$BOOTSTRAP_LOG"
"#,
    );
    bin_dir
}

#[test]
// REQ-CORE-013
fn bootstrap_default_on_node_25_installs_app_only() {
    let tempdir = tempdir().expect("tempdir");
    write_fixture_repo(tempdir.path());
    let bin_dir = write_mock_bin(tempdir.path());
    let log_path = tempdir.path().join("bootstrap.log");

    let output = Command::new("bash")
        .arg(
            tempdir
                .path()
                .join("scripts/ci/bootstrap-contributor-tooling.sh"),
        )
        .current_dir(tempdir.path())
        .env(
            "PATH",
            format!("{}:{}", bin_dir.display(), std::env::var("PATH").unwrap()),
        )
        .env("BOOTSTRAP_LOG", &log_path)
        .env("NODE_MAJOR", "25")
        .output()
        .expect("script should run");

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let log = fs::read_to_string(&log_path).expect("bootstrap log");
    assert!(log.contains("PINNED install app"));
    assert!(log.contains("NPM --prefix app ci"));
    assert!(!log.contains("website"));
    assert!(!log.contains("editors/vscode"));
    assert!(!log.contains("INSTALL_DOCS_SITE_DEPS"));
}

#[test]
// REQ-CORE-013
fn bootstrap_default_on_node_20_installs_docs_and_vscode() {
    let tempdir = tempdir().expect("tempdir");
    write_fixture_repo(tempdir.path());
    let bin_dir = write_mock_bin(tempdir.path());
    let log_path = tempdir.path().join("bootstrap.log");

    let output = Command::new("bash")
        .arg(
            tempdir
                .path()
                .join("scripts/ci/bootstrap-contributor-tooling.sh"),
        )
        .current_dir(tempdir.path())
        .env(
            "PATH",
            format!("{}:{}", bin_dir.display(), std::env::var("PATH").unwrap()),
        )
        .env("BOOTSTRAP_LOG", &log_path)
        .env("NODE_MAJOR", "20")
        .output()
        .expect("script should run");

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let log = fs::read_to_string(&log_path).expect("bootstrap log");
    assert!(!log.contains("PINNED install app"));
    assert!(log.contains("PINNED install website"));
    assert!(log.contains("INSTALL_DOCS_SITE_DEPS"));
    assert!(log.contains("PINNED install editors/vscode"));
    assert!(log.contains("NPM --prefix editors/vscode ci"));
}

#[test]
// REQ-CORE-013
fn bootstrap_default_rejects_unmatched_node_major() {
    let tempdir = tempdir().expect("tempdir");
    write_fixture_repo(tempdir.path());
    let bin_dir = write_mock_bin(tempdir.path());
    let log_path = tempdir.path().join("bootstrap.log");

    let output = Command::new("bash")
        .arg(
            tempdir
                .path()
                .join("scripts/ci/bootstrap-contributor-tooling.sh"),
        )
        .current_dir(tempdir.path())
        .env(
            "PATH",
            format!("{}:{}", bin_dir.display(), std::env::var("PATH").unwrap()),
        )
        .env("BOOTSTRAP_LOG", &log_path)
        .env("NODE_MAJOR", "18")
        .output()
        .expect("script should run");

    assert!(
        !output.status.success(),
        "unexpected success\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Current Node major 18 does not match"));
    assert!(stderr.contains("Node 25"));
    assert!(stderr.contains("Node 20"));
}
