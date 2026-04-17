#[cfg(unix)]
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::Command,
};

#[cfg(unix)]
use tempfile::tempdir;

#[cfg(unix)]
fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[cfg(unix)]
fn write_executable(path: &Path, content: &str) {
    fs::write(path, content).expect("mock executable should be written");
    let permissions = fs::Permissions::from_mode(0o755);
    fs::set_permissions(path, permissions).expect("mock executable should be executable");
}

#[cfg(unix)]
#[test]
// REQ-CORE-013
fn install_precommit_reports_lookup_paths_when_hook_bootstrap_fails() {
    let tempdir = tempdir().expect("tempdir should exist");
    let bin_dir = tempdir.path().join("bin");
    let user_base = tempdir.path().join("user-base");
    let pipx_bin = tempdir.path().join("pipx-bin");
    fs::create_dir_all(&bin_dir).expect("mock bin dir should exist");
    fs::create_dir_all(user_base.join("bin")).expect("mock user-base bin dir should exist");
    fs::create_dir_all(&pipx_bin).expect("mock pipx bin dir should exist");

    write_executable(
        &bin_dir.join("python3"),
        &format!(
            "#!/usr/bin/env bash\nset -euo pipefail\nif [ \"$#\" -ge 2 ] && [ \"$1\" = \"-m\" ] && [ \"$2\" = \"pip\" ]; then\n  exit 0\nfi\nif [ \"$#\" -eq 3 ] && [ \"$1\" = \"-m\" ] && [ \"$2\" = \"site\" ] && [ \"$3\" = \"--user-base\" ]; then\n  echo 'mock python warning' >&2\n  printf '%s\\n' '{}'\n  exit 0\nfi\nif [ \"$#\" -ge 3 ] && [ \"$1\" = \"-m\" ] && [ \"$2\" = \"pre_commit\" ] && [ \"$3\" = \"install\" ]; then\n  echo 'mock pre_commit install failed' >&2\n  exit 23\nfi\necho \"unexpected python args: $*\" >&2\nexit 99\n",
            user_base.display()
        ),
    );
    write_executable(
        &bin_dir.join("pipx"),
        &format!(
            "#!/usr/bin/env bash\nset -euo pipefail\nif [ \"$#\" -ge 3 ] && [ \"$1\" = \"install\" ] && [ \"$2\" = \"--force\" ] && [ \"$3\" = \"pre-commit\" ]; then\n  exit 0\nfi\nif [ \"$#\" -eq 3 ] && [ \"$1\" = \"environment\" ] && [ \"$2\" = \"--value\" ] && [ \"$3\" = \"PIPX_BIN_DIR\" ]; then\n  echo 'mock pipx warning' >&2\n  printf '%s\\n' '{}'\n  exit 0\nfi\necho \"unexpected pipx args: $*\" >&2\nexit 99\n",
            pipx_bin.display()
        ),
    );

    let output = Command::new("/usr/bin/bash")
        .current_dir(repo_root())
        .arg(repo_root().join("scripts/install-precommit.sh"))
        .env_clear()
        .env("BASH_ENV", "")
        .env("ENV", "")
        .env("HOME", tempdir.path())
        .env("PATH", format!("{}:/usr/bin:/bin", bin_dir.display()))
        .output()
        .expect("install-precommit script should run");

    assert!(
        !output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(stdout.contains("Installing pre-commit with pipx."));
    assert!(stdout.contains("Falling back to"));
    assert!(stderr.contains("Checked Python user-base path:"));
    assert!(stderr.contains(&format!("{}/bin/pre-commit", user_base.display())));
    assert!(stderr.contains("Checked pipx bin path:"));
    assert!(stderr.contains(&format!("{}/pre-commit", pipx_bin.display())));
    assert!(stderr.contains("Fallback hook installation via"));
    assert!(stderr.contains("mock pre_commit install failed"));
    assert!(stderr.contains(&format!(
        "Troubleshooting: compare '{}/python3 -m site --user-base' with your PATH",
        bin_dir.display()
    )));
    assert!(stderr.contains(
        "If you installed pre-commit with pipx, also compare 'pipx environment --value PIPX_BIN_DIR' with your PATH."
    ));
    assert!(stderr.contains("See CONTRIBUTING.md#local-checks"));
    assert!(!stderr.contains("mock python warning"));
    assert!(!stderr.contains("mock pipx warning"));
}
