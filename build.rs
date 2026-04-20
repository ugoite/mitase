// FEAT-APP-001

use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
    time::SystemTime,
};

fn main() {
    let manifest_dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("manifest dir"));
    let app_dir = manifest_dir.join("app");
    let shared_core_dir = manifest_dir.join("crates").join("syu-core");
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR")).join("syu-app-dist");

    println!("cargo:rerun-if-changed=build.rs");
    emit_watch(&manifest_dir.join("Cargo.lock"));
    emit_watch(&app_dir.join("index.html"));
    emit_watch(&app_dir.join("package.json"));
    emit_watch(&app_dir.join("package-lock.json"));
    emit_watch(&app_dir.join("vite.config.ts"));
    emit_watch(&app_dir.join("tsconfig.json"));
    emit_watch(&app_dir.join("tsconfig.app.json"));
    emit_watch(&app_dir.join("tsconfig.node.json"));
    emit_watch_recursive(&app_dir.join("public"));
    emit_watch_recursive(&app_dir.join("scripts"));
    emit_watch_recursive(&app_dir.join("src"));
    emit_watch_recursive(&app_dir.join("wasm"));
    emit_watch(&shared_core_dir.join("Cargo.toml"));
    emit_watch_recursive(&shared_core_dir.join("src"));

    if let Err(error) = required_npm_version(&app_dir).and_then(|required_npm| {
        ensure_app_dependencies(&app_dir, &required_npm)
            .and_then(|_| rebuild_browser_wasm_bindings(&manifest_dir, &app_dir, &required_npm))
            .and_then(|_| build_browser_bundle(&app_dir, &out_dir, &required_npm))
    }) {
        panic!("{error}");
    }
}

fn emit_watch(path: &Path) {
    if path.exists() {
        println!("cargo:rerun-if-changed={}", path.display());
    }
}

fn emit_watch_recursive(path: &Path) {
    if !path.exists() {
        return;
    }

    if path.is_file() {
        emit_watch(path);
        return;
    }

    for entry in fs::read_dir(path).expect("watch directory should be readable") {
        let child = entry.expect("watch directory entry should exist").path();
        if child.is_dir() {
            emit_watch_recursive(&child);
        } else {
            emit_watch(&child);
        }
    }
}

fn npm_executable() -> &'static str {
    if cfg!(windows) { "npm.cmd" } else { "npm" }
}

fn npx_executable() -> &'static str {
    if cfg!(windows) { "npx.cmd" } else { "npx" }
}

fn node_executable() -> &'static str {
    "node"
}

fn package_manager_for(package_json: &Path) -> Result<String, String> {
    let output = Command::new(node_executable())
        .arg("-p")
        .arg("JSON.parse(require('node:fs').readFileSync(process.argv[1], 'utf8')).packageManager ?? ''")
        .arg(package_json)
        .output()
        .map_err(|error| format!("failed to read {}: {error}", package_json.display()))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        let detail = if stderr.is_empty() {
            format!("exit status {}", output.status)
        } else {
            stderr
        };
        return Err(format!(
            "failed to read {} packageManager: {detail}",
            package_json.display()
        ));
    }

    String::from_utf8(output.stdout)
        .map_err(|error| {
            format!(
                "failed to decode {} packageManager: {error}",
                package_json.display()
            )
        })
        .map(|stdout| stdout.trim().to_owned())
}

fn required_npm_version(app_dir: &Path) -> Result<String, String> {
    let package_json = app_dir.join("package.json");
    let package_manager = package_manager_for(&package_json)?;

    package_manager
        .strip_prefix("npm@")
        .filter(|version| !version.is_empty())
        .map(str::to_owned)
        .ok_or_else(|| {
            format!(
                "Expected app/package.json to declare packageManager: npm@<version>, found `{package_manager}`."
            )
        })
}

fn npm_version(app_dir: &Path) -> Result<String, String> {
    let output = Command::new(npm_executable())
        .arg("--version")
        .current_dir(app_dir)
        .output()
        .map_err(|error| format!("failed to read npm version: {error}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        let detail = if stderr.is_empty() {
            format!("exit status {}", output.status)
        } else {
            stderr
        };
        return Err(format!("failed to read npm version: {detail}"));
    }

    String::from_utf8(output.stdout)
        .map_err(|error| format!("failed to decode npm version: {error}"))
        .map(|stdout| stdout.trim().to_owned())
}

fn uses_required_npm(app_dir: &Path, required: &str) -> Result<bool, String> {
    Ok(npm_version(app_dir)? == required)
}

fn modified_time(path: &Path) -> Option<SystemTime> {
    fs::metadata(path).ok()?.modified().ok()
}

fn needs_npm_ci(app_dir: &Path) -> bool {
    let node_modules = app_dir.join("node_modules");
    let install_marker = node_modules.join(".package-lock.json");
    let lockfile = app_dir.join("package-lock.json");

    if !node_modules.is_dir() || !install_marker.is_file() {
        return true;
    }

    match (modified_time(&lockfile), modified_time(&install_marker)) {
        (Some(lockfile_time), Some(marker_time)) => lockfile_time > marker_time,
        _ => true,
    }
}

fn run_npm(
    app_dir: &Path,
    required_npm: &str,
    args: &[String],
    action: &str,
    extra_env: &[(&str, String)],
) -> Result<(), String> {
    let mut command;
    let command_display;
    if uses_required_npm(app_dir, required_npm)? {
        command = Command::new(npm_executable());
        command.args(args);
        command_display = format!("{} {}", npm_executable(), args.join(" "));
    } else {
        command = Command::new(npx_executable());
        command
            .arg("-y")
            .arg(format!("npm@{required_npm}"))
            .args(args);
        command_display = format!(
            "{} -y npm@{} {}",
            npx_executable(),
            required_npm,
            args.join(" ")
        );
    }

    let status = command
        .current_dir(app_dir)
        .envs(extra_env.iter().map(|(key, value)| (*key, value)))
        .status()
        .map_err(|error| format!("failed to {action}: {error}"))?;

    if status.success() {
        return Ok(());
    }

    Err(format!(
        "failed to {action}: `{command_display}` exited with status {status}",
    ))
}

fn ensure_app_dependencies(app_dir: &Path, required_npm: &str) -> Result<(), String> {
    if !needs_npm_ci(app_dir) {
        return Ok(());
    }

    Err(missing_app_dependencies_message(required_npm))
}

fn missing_app_dependencies_message(required_npm: &str) -> String {
    format!(
        concat!(
            "browser app dependencies are not ready.\n\n",
            "This usually means you are in a fresh clone or fresh worktree that does not have `app/node_modules` yet.\n",
            "Cargo intentionally does not run a networked npm install for you during embedded browser-app builds.\n\n",
            "From the repository root, run:\n",
            "  scripts/ci/pinned-npm.sh install app\n",
            "  npm --prefix app ci\n\n",
            "Then rerun the Cargo command. The pinned npm workflow expects npm {}."
        ),
        required_npm
    )
}

fn remove_dir_if_exists(path: &Path, description: &str) -> Result<(), String> {
    if !path.exists() {
        return Ok(());
    }

    fs::remove_dir_all(path).map_err(|error| format!("failed to clear {description}: {error}"))
}

fn rebuild_browser_wasm_bindings(
    manifest_dir: &Path,
    app_dir: &Path,
    required_npm: &str,
) -> Result<(), String> {
    remove_dir_if_exists(
        &app_dir.join("src").join("wasm"),
        "generated browser app Wasm bindings",
    )?;

    let wasm_target_dir = default_wasm_target_dir(manifest_dir)
        .to_string_lossy()
        .into_owned();

    run_npm(
        app_dir,
        required_npm,
        &[String::from("run"), String::from("build:wasm")],
        "generate the browser app Wasm bridge",
        &[("CARGO_TARGET_DIR", wasm_target_dir)],
    )
}

fn build_browser_bundle(app_dir: &Path, out_dir: &Path, required_npm: &str) -> Result<(), String> {
    remove_dir_if_exists(out_dir, "generated browser bundle")?;
    fs::create_dir_all(out_dir)
        .map_err(|error| format!("failed to create browser bundle output directory: {error}"))?;

    let out_dir_arg = out_dir.to_string_lossy().into_owned();
    run_npm(
        app_dir,
        required_npm,
        &[
            String::from("run"),
            String::from("build"),
            String::from("--"),
            String::from("--outDir"),
            out_dir_arg,
        ],
        "build the embedded browser app bundle",
        &[],
    )?;

    let index = out_dir.join("index.html");
    if index.is_file() {
        return Ok(());
    }

    Err(format!(
        "embedded browser app build succeeded but `{}` was not produced",
        index.display()
    ))
}

fn default_wasm_target_dir(manifest_dir: &Path) -> PathBuf {
    default_wasm_target_dir_from_common_dir(
        manifest_dir,
        git_common_dir(manifest_dir).as_deref(),
        env::var_os("CARGO_TARGET_DIR")
            .map(PathBuf::from)
            .as_deref(),
    )
}

fn default_wasm_target_dir_from_common_dir(
    manifest_dir: &Path,
    git_common_dir: Option<&Path>,
    configured_target_dir: Option<&Path>,
) -> PathBuf {
    if let Some(configured_target_dir) = configured_target_dir {
        return if configured_target_dir.is_absolute() {
            configured_target_dir.to_path_buf()
        } else {
            manifest_dir.join(configured_target_dir)
        };
    }

    if let Some(repo_root) = git_common_dir.and_then(Path::parent) {
        return repo_root.join("target").join("app-wasm");
    }

    manifest_dir.join("target").join("app-wasm")
}

fn git_common_dir(manifest_dir: &Path) -> Option<PathBuf> {
    let output = Command::new("git")
        .args(["rev-parse", "--git-common-dir"])
        .current_dir(manifest_dir)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }

    let raw = String::from_utf8(output.stdout).ok()?;
    let path = PathBuf::from(raw.trim());
    Some(if path.is_absolute() {
        path
    } else {
        manifest_dir.join(path)
    })
}

#[cfg(test)]
mod tests {
    use super::default_wasm_target_dir_from_common_dir;
    use std::path::Path;

    #[test]
    fn default_wasm_target_dir_uses_configured_target_dir_first() {
        assert_eq!(
            default_wasm_target_dir_from_common_dir(
                Path::new("/repo/.worktrees/impl"),
                Some(Path::new("/repo/.git")),
                Some(Path::new("/shared/target"))
            ),
            Path::new("/shared/target")
        );
    }

    #[test]
    fn default_wasm_target_dir_resolves_relative_configured_target_dir_from_manifest_dir() {
        assert_eq!(
            default_wasm_target_dir_from_common_dir(
                Path::new("/repo/.worktrees/impl"),
                Some(Path::new("/repo/.git")),
                Some(Path::new("target"))
            ),
            Path::new("/repo/.worktrees/impl/target")
        );
    }

    #[test]
    fn default_wasm_target_dir_uses_git_common_dir_parent_when_available() {
        assert_eq!(
            default_wasm_target_dir_from_common_dir(
                Path::new("/repo/.worktrees/impl"),
                Some(Path::new("/repo/.git")),
                None
            ),
            Path::new("/repo/target/app-wasm")
        );
    }

    #[test]
    fn default_wasm_target_dir_falls_back_to_manifest_target() {
        assert_eq!(
            default_wasm_target_dir_from_common_dir(Path::new("/repo/.worktrees/impl"), None, None),
            Path::new("/repo/.worktrees/impl/target/app-wasm")
        );
    }

    #[test]
    fn missing_app_dependencies_message_guides_fresh_worktrees() {
        let message = super::missing_app_dependencies_message("11.8.0");

        assert!(message.contains("fresh clone or fresh worktree"));
        assert!(message.contains("Cargo intentionally does not run a networked npm install"));
        assert!(message.contains("scripts/ci/pinned-npm.sh install app"));
        assert!(message.contains("npm --prefix app ci"));
        assert!(message.contains("npm 11.8.0"));
    }
}
