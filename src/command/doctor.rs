// FEAT-DOCTOR-001
// REQ-CORE-026

use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
    time::SystemTime,
};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::{
    cli::{DoctorArgs, OutputFormat},
    config::CONFIG_FILE_NAME,
    runtime::command_exists,
    workspace::resolve_workspace_root,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum DoctorStatus {
    Ok,
    Warning,
    Error,
    Skipped,
}

impl DoctorStatus {
    const fn label(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::Warning => "warning",
            Self::Error => "error",
            Self::Skipped => "skipped",
        }
    }
}

#[derive(Debug, Serialize)]
struct DoctorReport {
    workspace_root: PathBuf,
    summary: DoctorSummary,
    checks: Vec<DoctorCheck>,
}

#[derive(Debug, Default, Serialize)]
struct DoctorSummary {
    ok: usize,
    warning: usize,
    error: usize,
    skipped: usize,
}

#[derive(Debug, Serialize)]
struct DoctorCheck {
    id: &'static str,
    label: &'static str,
    status: DoctorStatus,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    fix: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct PackageMetadata {
    package_manager: Option<String>,
}

trait DeserializePackageMetadata: Sized {
    fn from_path(path: &Path) -> Result<Self>;
}

impl DeserializePackageMetadata for PackageMetadata {
    fn from_path(path: &Path) -> Result<Self> {
        let raw = fs::read_to_string(path).with_context(|| {
            format!("failed to read package metadata from `{}`", path.display())
        })?;
        serde_json::from_str(&raw)
            .with_context(|| format!("failed to parse package metadata from `{}`", path.display()))
    }
}

impl DoctorSummary {
    fn from_checks(checks: &[DoctorCheck]) -> Self {
        let mut summary = Self::default();
        for check in checks {
            match check.status {
                DoctorStatus::Ok => summary.ok += 1,
                DoctorStatus::Warning => summary.warning += 1,
                DoctorStatus::Error => summary.error += 1,
                DoctorStatus::Skipped => summary.skipped += 1,
            }
        }
        summary
    }
}

pub fn run_doctor_command(args: &DoctorArgs) -> Result<i32> {
    let report = build_doctor_report(&args.workspace)?;

    match args.format {
        OutputFormat::Json => println!(
            "{}",
            serde_json::to_string_pretty(&report).context("failed to serialize doctor output")?
        ),
        OutputFormat::Text => print_text_report(&report),
    }

    Ok(if report.summary.error > 0 { 1 } else { 0 })
}

fn build_doctor_report(workspace: &Path) -> Result<DoctorReport> {
    let workspace_root = resolve_workspace_root(workspace)?;
    let mut checks = Vec::new();

    checks.push(workspace_config_check(&workspace_root));
    checks.extend(rust_toolchain_checks(&workspace_root));
    checks.extend(surface_checks(
        &workspace_root,
        "app",
        "Browser app",
        "scripts/ci/pinned-npm.sh install app && scripts/ci/pinned-npm.sh exec app -- ci",
    ));
    checks.extend(surface_checks(
        &workspace_root,
        "website",
        "Docs site",
        "scripts/ci/pinned-npm.sh install website && scripts/ci/pinned-npm.sh exec website -- ci",
    ));
    checks.push(playwright_check(&workspace_root));

    let summary = DoctorSummary::from_checks(&checks);
    Ok(DoctorReport {
        workspace_root,
        summary,
        checks,
    })
}

fn workspace_config_check(workspace_root: &Path) -> DoctorCheck {
    let config_path = workspace_root.join(CONFIG_FILE_NAME);
    if config_path.is_file() {
        DoctorCheck {
            id: "workspace-config",
            label: "Workspace config",
            status: DoctorStatus::Ok,
            message: format!("found `{}`", config_path.display()),
            fix: None,
        }
    } else {
        DoctorCheck {
            id: "workspace-config",
            label: "Workspace config",
            status: DoctorStatus::Warning,
            message: format!(
                "`{}` is missing, so workspace-specific readiness checks are limited",
                config_path.display()
            ),
            fix: Some(format!(
                "Run `syu init {}` first.",
                shell_workspace_arg(workspace_root)
            )),
        }
    }
}

fn rust_toolchain_checks(workspace_root: &Path) -> Vec<DoctorCheck> {
    let mut checks = Vec::new();

    let rustc_check = match command_version("rustc", &["--version"]) {
        Ok(version) => DoctorCheck {
            id: "rustc-version",
            label: "Rust compiler",
            status: DoctorStatus::Ok,
            message: format!("found {version}"),
            fix: None,
        },
        Err(message) => DoctorCheck {
            id: "rustc-version",
            label: "Rust compiler",
            status: DoctorStatus::Error,
            message,
            fix: Some(
                "Install Rust from https://rustup.rs before running contributor checks."
                    .to_string(),
            ),
        },
    };
    checks.push(rustc_check);

    let cargo_check = match command_version(cargo_executable(), &["--version"]) {
        Ok(version) => DoctorCheck {
            id: "cargo-version",
            label: "Cargo",
            status: DoctorStatus::Ok,
            message: format!("found {version}"),
            fix: None,
        },
        Err(message) => DoctorCheck {
            id: "cargo-version",
            label: "Cargo",
            status: DoctorStatus::Error,
            message,
            fix: Some(
                "Install Rust/Cargo from https://rustup.rs before running contributor checks."
                    .to_string(),
            ),
        },
    };
    checks.push(cargo_check);

    checks.push(rust_msrv_check(workspace_root));
    checks
}

fn rust_msrv_check(workspace_root: &Path) -> DoctorCheck {
    let cargo_toml = workspace_root.join("Cargo.toml");
    build_rust_msrv_check(
        &cargo_toml,
        read_rust_version(&cargo_toml).as_deref(),
        command_version("rustc", &["--version"]),
    )
}

fn surface_checks(
    workspace_root: &Path,
    relative_dir: &'static str,
    label_prefix: &'static str,
    install_fix: &'static str,
) -> Vec<DoctorCheck> {
    let surface_root = workspace_root.join(relative_dir);
    let package_json = surface_root.join("package.json");
    let nvmrc = surface_root.join(".nvmrc");

    if !package_json.is_file() {
        return vec![
            DoctorCheck {
                id: surface_check_id(relative_dir, "node"),
                label: surface_label(label_prefix, "Node"),
                status: DoctorStatus::Skipped,
                message: format!("`{}` is not present", package_json.display()),
                fix: None,
            },
            DoctorCheck {
                id: surface_check_id(relative_dir, "npm"),
                label: surface_label(label_prefix, "npm"),
                status: DoctorStatus::Skipped,
                message: format!("`{}` is not present", package_json.display()),
                fix: None,
            },
            DoctorCheck {
                id: surface_check_id(relative_dir, "deps"),
                label: surface_label(label_prefix, "dependencies"),
                status: DoctorStatus::Skipped,
                message: format!("`{}` is not present", package_json.display()),
                fix: None,
            },
        ];
    }

    let expected_npm_version = PackageMetadata::from_path(&package_json)
        .map(|metadata| expected_npm_version(&metadata))
        .map_err(|error| error.to_string());
    let expected_node_major = fs::read_to_string(&nvmrc)
        .ok()
        .map(|raw| raw.trim().to_string())
        .filter(|value| !value.is_empty());

    vec![
        surface_node_check(
            label_prefix,
            relative_dir,
            expected_node_major.as_deref(),
            &nvmrc,
        ),
        surface_npm_check(
            label_prefix,
            relative_dir,
            expected_npm_version,
            &package_json,
        ),
        surface_dependency_check(label_prefix, relative_dir, &surface_root, install_fix),
    ]
}

fn surface_node_check(
    label_prefix: &'static str,
    relative_dir: &'static str,
    expected_major: Option<&str>,
    nvmrc_path: &Path,
) -> DoctorCheck {
    match command_version("node", &["--version"]) {
        Ok(current) => build_node_check(
            label_prefix,
            relative_dir,
            expected_major,
            nvmrc_path,
            &current,
        ),
        Err(message) => DoctorCheck {
            id: surface_check_id(relative_dir, "node"),
            label: surface_label(label_prefix, "Node"),
            status: DoctorStatus::Error,
            message,
            fix: Some("Install Node.js before running browser or docs-site workflows.".to_string()),
        },
    }
}

fn surface_npm_check(
    label_prefix: &'static str,
    relative_dir: &'static str,
    expected_version: std::result::Result<Option<String>, String>,
    package_json_path: &Path,
) -> DoctorCheck {
    let expected_version = match expected_version {
        Ok(expected_version) => expected_version,
        Err(message) => {
            return DoctorCheck {
                id: surface_check_id(relative_dir, "npm"),
                label: surface_label(label_prefix, "npm"),
                status: DoctorStatus::Error,
                message,
                fix: Some(format!(
                    "Fix the JSON syntax in `{}` so the pinned npm requirement can be read.",
                    package_json_path.display()
                )),
            };
        }
    };

    match command_version(npm_executable(), &["--version"]) {
        Ok(current) => build_npm_check(
            label_prefix,
            relative_dir,
            expected_version.as_deref(),
            package_json_path,
            &current,
        ),
        Err(message) => DoctorCheck {
            id: surface_check_id(relative_dir, "npm"),
            label: surface_label(label_prefix, "npm"),
            status: DoctorStatus::Error,
            message,
            fix: Some("Install npm before running browser or docs-site workflows.".to_string()),
        },
    }
}

fn surface_dependency_check(
    label_prefix: &'static str,
    relative_dir: &'static str,
    surface_root: &Path,
    install_fix: &'static str,
) -> DoctorCheck {
    let node_modules = surface_root.join("node_modules");
    let install_marker = node_modules.join(".package-lock.json");
    let lockfile = surface_root.join("package-lock.json");

    if !lockfile.is_file() {
        return DoctorCheck {
            id: surface_check_id(relative_dir, "deps"),
            label: surface_label(label_prefix, "dependencies"),
            status: DoctorStatus::Skipped,
            message: format!("`{}` is not present", lockfile.display()),
            fix: None,
        };
    }

    let needs_install = if !node_modules.is_dir() || !install_marker.is_file() {
        true
    } else {
        dependency_install_needs_refresh(modified_time(&lockfile), modified_time(&install_marker))
    };

    if needs_install {
        DoctorCheck {
            id: surface_check_id(relative_dir, "deps"),
            label: surface_label(label_prefix, "dependencies"),
            status: DoctorStatus::Warning,
            message: format!(
                "`{}` is missing or stale compared with `{}`",
                node_modules.display(),
                lockfile.display()
            ),
            fix: Some(format!("Run `{install_fix}`.")),
        }
    } else {
        DoctorCheck {
            id: surface_check_id(relative_dir, "deps"),
            label: surface_label(label_prefix, "dependencies"),
            status: DoctorStatus::Ok,
            message: format!(
                "`{}` is present and newer than the lockfile marker",
                node_modules.display()
            ),
            fix: None,
        }
    }
}

fn playwright_check(workspace_root: &Path) -> DoctorCheck {
    let app_root = workspace_root.join("app");
    let chromium_path = find_playwright_chromium();
    build_playwright_check(&app_root, chromium_path.as_deref())
}

fn print_text_report(report: &DoctorReport) {
    println!("workspace: {}", report.workspace_root.display());
    println!(
        "summary: {} ok, {} warning, {} error, {} skipped",
        report.summary.ok, report.summary.warning, report.summary.error, report.summary.skipped
    );
    println!();

    for check in &report.checks {
        println!(
            "{:<8} {:<24} {}",
            check.status.label(),
            check.label,
            check.message
        );
        if let Some(fix) = &check.fix {
            println!("         fix: {fix}");
        }
    }
}

fn command_version(command: &str, args: &[&str]) -> Result<String, String> {
    if !command_exists(command) {
        return Err(format!("`{command}` was not found on PATH"));
    }

    let output = Command::new(command)
        .args(args)
        .output()
        .map_err(|error| format!("failed to run `{command}`: {error}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let detail = if stderr.is_empty() {
            format!("exit status {}", output.status)
        } else {
            stderr
        };
        return Err(format!("`{command}` failed: {detail}"));
    }

    String::from_utf8(output.stdout)
        .map(|stdout| stdout.trim().to_string())
        .map_err(|error| format!("failed to decode `{command}` output: {error}"))
}

fn read_rust_version(cargo_toml: &Path) -> Option<String> {
    let raw = fs::read_to_string(cargo_toml).ok()?;
    raw.lines().find_map(|line| {
        let trimmed = line.trim();
        trimmed
            .strip_prefix("rust-version = ")
            .map(|value| value.trim().trim_matches('"').to_string())
            .filter(|value| !value.is_empty())
    })
}

fn version_at_least(current: &str, minimum: &str) -> bool {
    let mut current_parts = parse_numeric_version(current).into_iter();
    let mut minimum_parts = parse_numeric_version(minimum).into_iter();

    loop {
        match (current_parts.next(), minimum_parts.next()) {
            (Some(current), Some(minimum)) if current > minimum => return true,
            (Some(current), Some(minimum)) if current < minimum => return false,
            (Some(_), Some(_)) => {}
            (_, None) => return true,
            (None, Some(minimum)) => return minimum == 0 && minimum_parts.all(|part| part == 0),
        }
    }
}

fn parse_numeric_version(raw: &str) -> Vec<u32> {
    let cleaned = raw.trim().trim_start_matches('v');
    cleaned
        .split(|ch: char| !(ch.is_ascii_digit() || ch == '.'))
        .find(|segment| segment.chars().any(|ch| ch.is_ascii_digit()))
        .unwrap_or_default()
        .split('.')
        .filter_map(|segment| segment.parse::<u32>().ok())
        .collect()
}

fn parse_major_version(raw: &str) -> Option<String> {
    parse_numeric_version(raw).first().map(ToString::to_string)
}

fn modified_time(path: &Path) -> Option<SystemTime> {
    fs::metadata(path).ok()?.modified().ok()
}

fn dependency_install_needs_refresh(
    lockfile_time: Option<SystemTime>,
    marker_time: Option<SystemTime>,
) -> bool {
    match (lockfile_time, marker_time) {
        (Some(lockfile_time), Some(marker_time)) => lockfile_time > marker_time,
        _ => true,
    }
}

fn find_playwright_chromium() -> Option<PathBuf> {
    playwright_cache_roots()
        .into_iter()
        .find(|root| root.is_dir())
        .and_then(|root| find_playwright_chromium_in(&root))
}

fn playwright_cache_roots() -> Vec<PathBuf> {
    playwright_cache_roots_from(
        env::var_os("PLAYWRIGHT_BROWSERS_PATH")
            .filter(|value| !value.is_empty())
            .map(PathBuf::from),
        home_dir(),
        env::var_os("LOCALAPPDATA").map(PathBuf::from),
    )
}

fn home_dir() -> Option<PathBuf> {
    home_dir_from(
        env::var_os("HOME").map(PathBuf::from),
        env::var_os("USERPROFILE").map(PathBuf::from),
    )
}

const fn cargo_executable() -> &'static str {
    if cfg!(windows) { "cargo.exe" } else { "cargo" }
}

const fn npm_executable() -> &'static str {
    if cfg!(windows) { "npm.cmd" } else { "npm" }
}

fn shell_workspace_arg(workspace_root: &Path) -> String {
    let rendered = workspace_root.display().to_string();
    if rendered == "." {
        rendered
    } else {
        format!("'{}'", rendered.replace('\'', "'\\''"))
    }
}

fn build_rust_msrv_check(
    cargo_toml: &Path,
    required: Option<&str>,
    current: std::result::Result<String, String>,
) -> DoctorCheck {
    let Some(required) = required else {
        return DoctorCheck {
            id: "rust-msrv",
            label: "Rust MSRV",
            status: DoctorStatus::Skipped,
            message: format!("`{}` does not declare `rust-version`", cargo_toml.display()),
            fix: None,
        };
    };

    match current {
        Ok(current) => {
            let status = if version_at_least(&current, required) {
                DoctorStatus::Ok
            } else {
                DoctorStatus::Warning
            };
            let fix = (status == DoctorStatus::Warning).then(|| {
                format!("Run `rustup toolchain install {required}` and `cargo +{required} check --all-targets`.")
            });
            DoctorCheck {
                id: "rust-msrv",
                label: "Rust MSRV",
                status,
                message: format!("current {current}; repository expects at least Rust {required}"),
                fix,
            }
        }
        Err(message) => DoctorCheck {
            id: "rust-msrv",
            label: "Rust MSRV",
            status: DoctorStatus::Error,
            message,
            fix: Some(format!(
                "Install Rust {required} or newer from https://rustup.rs."
            )),
        },
    }
}

fn expected_npm_version(metadata: &PackageMetadata) -> Option<String> {
    metadata
        .package_manager
        .as_deref()
        .and_then(|value| value.strip_prefix("npm@"))
        .map(ToOwned::to_owned)
}

fn build_node_check(
    label_prefix: &'static str,
    relative_dir: &'static str,
    expected_major: Option<&str>,
    nvmrc_path: &Path,
    current: &str,
) -> DoctorCheck {
    let status = match (expected_major, parse_major_version(current)) {
        (Some(expected), Some(current_major)) if current_major != expected => DoctorStatus::Warning,
        _ => DoctorStatus::Ok,
    };
    let message = match expected_major {
        Some(expected) => format!(
            "current {current}; `{}` expects Node {expected}",
            nvmrc_path.display()
        ),
        None => format!("found {current}"),
    };
    let fix = (status == DoctorStatus::Warning)
        .then(|| format!("Switch runtimes with `nvm use \"$(cat {relative_dir}/.nvmrc)\"`."));
    DoctorCheck {
        id: surface_check_id(relative_dir, "node"),
        label: surface_label(label_prefix, "Node"),
        status,
        message,
        fix,
    }
}

fn build_npm_check(
    label_prefix: &'static str,
    relative_dir: &'static str,
    expected_version: Option<&str>,
    package_json_path: &Path,
    current: &str,
) -> DoctorCheck {
    let status = match expected_version {
        Some(expected) if current != expected => DoctorStatus::Warning,
        _ => DoctorStatus::Ok,
    };
    let message = match expected_version {
        Some(expected) => format!(
            "current npm {current}; `{}` expects npm {expected}",
            package_json_path.display()
        ),
        None => format!("found npm {current}"),
    };
    let fix = (status == DoctorStatus::Warning).then(|| {
        format!(
            "Use the pinned npm release declared in `{}`.",
            package_json_path.display()
        )
    });
    DoctorCheck {
        id: surface_check_id(relative_dir, "npm"),
        label: surface_label(label_prefix, "npm"),
        status,
        message,
        fix,
    }
}

fn build_playwright_check(app_root: &Path, chromium_path: Option<&Path>) -> DoctorCheck {
    if !app_root.join("package.json").is_file() {
        return DoctorCheck {
            id: "playwright-chromium",
            label: "Playwright Chromium",
            status: DoctorStatus::Skipped,
            message: format!(
                "`{}` is not present",
                app_root.join("package.json").display()
            ),
            fix: None,
        };
    }

    if !app_root.join("node_modules").is_dir() {
        return DoctorCheck {
            id: "playwright-chromium",
            label: "Playwright Chromium",
            status: DoctorStatus::Warning,
            message: "app dependencies are missing, so browser readiness cannot be checked yet"
                .to_string(),
            fix: Some(
                "Run `scripts/ci/pinned-npm.sh install app && scripts/ci/pinned-npm.sh exec app -- ci` first."
                    .to_string(),
            ),
        };
    }

    if let Some(path) = chromium_path {
        DoctorCheck {
            id: "playwright-chromium",
            label: "Playwright Chromium",
            status: DoctorStatus::Ok,
            message: format!(
                "found an installed Chromium bundle under `{}`",
                path.display()
            ),
            fix: None,
        }
    } else {
        DoctorCheck {
            id: "playwright-chromium",
            label: "Playwright Chromium",
            status: DoctorStatus::Warning,
            message:
                "no installed Playwright Chromium bundle was found in the standard cache locations"
                    .to_string(),
            fix: Some(
                "Run `scripts/ci/pinned-npm.sh install app && scripts/ci/pinned-npm.sh exec app -- exec playwright install --with-deps chromium`.".to_string(),
            ),
        }
    }
}

fn playwright_cache_roots_from(
    custom: Option<PathBuf>,
    home: Option<PathBuf>,
    local_app_data: Option<PathBuf>,
) -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Some(custom) = custom {
        roots.push(custom);
    }

    if let Some(home) = home {
        roots.push(home.join(".cache/ms-playwright"));
        roots.push(home.join("Library/Caches/ms-playwright"));
    }

    if let Some(local_app_data) = local_app_data {
        roots.push(local_app_data.join("ms-playwright"));
    }

    roots
}

fn home_dir_from(home: Option<PathBuf>, userprofile: Option<PathBuf>) -> Option<PathBuf> {
    home.or(userprofile)
}

fn find_playwright_chromium_in(root: &Path) -> Option<PathBuf> {
    fs::read_dir(root).ok()?.find_map(|entry| {
        let path = entry.ok()?.path();
        let name = path.file_name()?.to_str()?;
        (path.is_dir() && name.starts_with("chromium")).then_some(path)
    })
}

fn surface_check_id(surface: &'static str, kind: &'static str) -> &'static str {
    match (surface, kind) {
        ("app", "node") => "app-node",
        ("app", "npm") => "app-npm",
        ("app", "deps") => "app-deps",
        ("website", "node") => "website-node",
        ("website", "npm") => "website-npm",
        ("website", "deps") => "website-deps",
        _ => "surface-check",
    }
}

fn surface_label(prefix: &'static str, suffix: &'static str) -> &'static str {
    match (prefix, suffix) {
        ("Browser app", "Node") => "Browser app Node",
        ("Browser app", "npm") => "Browser app npm",
        ("Browser app", "dependencies") => "Browser app dependencies",
        ("Docs site", "Node") => "Docs site Node",
        ("Docs site", "npm") => "Docs site npm",
        ("Docs site", "dependencies") => "Docs site dependencies",
        _ => "Surface check",
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::{Path, PathBuf},
        time::Duration,
    };

    use tempfile::tempdir;

    use super::{
        DeserializePackageMetadata, DoctorArgs, DoctorCheck, DoctorStatus, DoctorSummary,
        PackageMetadata, build_doctor_report, build_node_check, build_npm_check,
        build_playwright_check, build_rust_msrv_check, cargo_executable, command_version,
        dependency_install_needs_refresh, expected_npm_version, find_playwright_chromium_in,
        home_dir_from, modified_time, npm_executable, parse_major_version, parse_numeric_version,
        playwright_cache_roots_from, read_rust_version, run_doctor_command, shell_workspace_arg,
        surface_check_id, surface_checks, surface_dependency_check, surface_label,
        version_at_least, workspace_config_check,
    };

    #[test]
    fn summary_counts_each_status() {
        let summary = DoctorSummary::from_checks(&[
            DoctorCheck {
                id: "a",
                label: "A",
                status: DoctorStatus::Ok,
                message: String::new(),
                fix: None,
            },
            DoctorCheck {
                id: "b",
                label: "B",
                status: DoctorStatus::Warning,
                message: String::new(),
                fix: None,
            },
            DoctorCheck {
                id: "c",
                label: "C",
                status: DoctorStatus::Error,
                message: String::new(),
                fix: None,
            },
            DoctorCheck {
                id: "d",
                label: "D",
                status: DoctorStatus::Skipped,
                message: String::new(),
                fix: None,
            },
        ]);

        assert_eq!(summary.ok, 1);
        assert_eq!(summary.warning, 1);
        assert_eq!(summary.error, 1);
        assert_eq!(summary.skipped, 1);
    }

    #[test]
    fn doctor_status_labels_cover_all_variants() {
        assert_eq!(DoctorStatus::Ok.label(), "ok");
        assert_eq!(DoctorStatus::Warning.label(), "warning");
        assert_eq!(DoctorStatus::Error.label(), "error");
        assert_eq!(DoctorStatus::Skipped.label(), "skipped");
    }

    #[test]
    fn parses_major_versions_from_prefixed_strings() {
        assert_eq!(parse_major_version("v25.1.0").as_deref(), Some("25"));
        assert_eq!(
            parse_major_version("rustc 1.88.1 (abc)").as_deref(),
            Some("1")
        );
    }

    #[test]
    fn parses_numeric_version_segments() {
        assert_eq!(parse_numeric_version("npm 11.8.0"), vec![11, 8, 0]);
        assert_eq!(parse_numeric_version("1.88"), vec![1, 88]);
        assert!(parse_numeric_version("not-a-version").is_empty());
    }

    #[test]
    fn compares_versions_by_numeric_segments() {
        assert!(version_at_least("rustc 1.88.1 (abc)", "1.88"));
        assert!(version_at_least("11.8.0", "11.8.0"));
        assert!(!version_at_least("1.87.0", "1.88"));
        assert!(version_at_least("1.88", "1.88.0"));
    }

    #[test]
    fn reads_rust_version_from_cargo_toml() {
        let tempdir = tempdir().expect("tempdir");
        let cargo_toml = tempdir.path().join("Cargo.toml");
        fs::write(&cargo_toml, "[package]\nrust-version = \"1.88\"\n").expect("cargo toml");
        assert_eq!(read_rust_version(&cargo_toml).as_deref(), Some("1.88"));
        assert_eq!(
            read_rust_version(&tempdir.path().join("missing.toml")),
            None
        );
    }

    #[test]
    fn shell_workspace_arg_quotes_special_paths() {
        assert_eq!(shell_workspace_arg(Path::new(".")), ".");
        assert_eq!(
            shell_workspace_arg(Path::new("/tmp/doctor's workspace")),
            "'/tmp/doctor'\\''s workspace'"
        );
    }

    #[test]
    fn surface_ids_and_labels_cover_known_surfaces() {
        assert_eq!(surface_check_id("app", "node"), "app-node");
        assert_eq!(surface_check_id("website", "deps"), "website-deps");
        assert_eq!(surface_check_id("other", "node"), "surface-check");
        assert_eq!(surface_label("Browser app", "Node"), "Browser app Node");
        assert_eq!(
            surface_label("Docs site", "dependencies"),
            "Docs site dependencies"
        );
        assert_eq!(surface_label("Other", "Node"), "Surface check");
    }

    #[test]
    fn workspace_config_check_warns_when_missing() {
        let tempdir = tempdir().expect("tempdir");
        let warning = workspace_config_check(tempdir.path());
        assert_eq!(warning.status, DoctorStatus::Warning);
        assert!(
            warning
                .fix
                .as_deref()
                .is_some_and(|fix| fix.contains("syu init"))
        );

        fs::write(tempdir.path().join("syu.yaml"), "version: 1\n").expect("syu config");
        let ok = workspace_config_check(tempdir.path());
        assert_eq!(ok.status, DoctorStatus::Ok);
    }

    #[test]
    fn package_metadata_defaults_when_missing_fields() {
        let tempdir = tempdir().expect("tempdir");
        let package_json = tempdir.path().join("package.json");
        fs::write(&package_json, "{ \"name\": \"doctor\" }\n").expect("package");
        let metadata = PackageMetadata::from_path(&package_json).expect("metadata should parse");
        assert_eq!(metadata.package_manager, None);
    }

    #[test]
    fn package_metadata_reports_read_and_parse_failures() {
        let tempdir = tempdir().expect("tempdir");
        let missing = PackageMetadata::from_path(&tempdir.path().join("missing.json"))
            .expect_err("missing package should fail");
        assert!(
            missing
                .to_string()
                .contains("failed to read package metadata")
        );

        let invalid = tempdir.path().join("invalid.json");
        fs::write(&invalid, "{invalid").expect("invalid package");
        let invalid_error =
            PackageMetadata::from_path(&invalid).expect_err("invalid package should fail");
        assert!(
            invalid_error
                .to_string()
                .contains("failed to parse package metadata")
        );
    }

    #[test]
    fn expected_npm_version_only_accepts_npm_prefixes() {
        assert_eq!(
            expected_npm_version(&PackageMetadata {
                package_manager: Some("npm@11.8.0".to_string()),
            }),
            Some("11.8.0".to_string())
        );
        assert_eq!(
            expected_npm_version(&PackageMetadata {
                package_manager: Some("pnpm@9.0.0".to_string()),
            }),
            None
        );
    }

    #[test]
    fn surface_dependency_check_covers_skipped_warning_and_ok_paths() {
        let tempdir = tempdir().expect("tempdir");
        let surface_root = tempdir.path().join("app");
        fs::create_dir_all(&surface_root).expect("surface root");

        let skipped = surface_dependency_check("Browser app", "app", &surface_root, "install");
        assert_eq!(skipped.status, DoctorStatus::Skipped);

        fs::write(surface_root.join("package-lock.json"), "{}\n").expect("lockfile");
        let warning = surface_dependency_check("Browser app", "app", &surface_root, "install");
        assert_eq!(warning.status, DoctorStatus::Warning);
        assert!(
            warning
                .fix
                .as_deref()
                .is_some_and(|fix| fix.contains("install"))
        );

        fs::create_dir_all(surface_root.join("node_modules")).expect("node_modules");
        std::thread::sleep(Duration::from_millis(5));
        fs::write(surface_root.join("node_modules/.package-lock.json"), "{}\n").expect("marker");
        let ok = surface_dependency_check("Browser app", "app", &surface_root, "install");
        assert_eq!(ok.status, DoctorStatus::Ok);
    }

    #[test]
    fn surface_checks_skip_when_package_json_is_missing() {
        let tempdir = tempdir().expect("tempdir");
        let checks = surface_checks(tempdir.path(), "app", "Browser app", "install");
        assert_eq!(checks.len(), 3);
        assert!(
            checks
                .iter()
                .all(|check| check.status == DoctorStatus::Skipped)
        );
    }

    #[test]
    fn surface_checks_report_invalid_package_metadata_as_an_error() {
        let tempdir = tempdir().expect("tempdir");
        let app_root = tempdir.path().join("app");
        fs::create_dir_all(&app_root).expect("app root");
        fs::write(app_root.join("package.json"), "{invalid").expect("invalid package");
        fs::write(app_root.join(".nvmrc"), "25\n").expect("nvmrc");

        let checks = surface_checks(tempdir.path(), "app", "Browser app", "install");
        let npm_check = checks
            .iter()
            .find(|check| check.id == "app-npm")
            .expect("npm check should exist");

        assert_eq!(npm_check.status, DoctorStatus::Error);
        assert!(
            npm_check
                .message
                .contains("failed to parse package metadata")
        );
        assert!(
            npm_check
                .fix
                .as_deref()
                .is_some_and(|fix| fix.contains("pinned npm requirement"))
        );
    }

    #[test]
    fn command_version_reports_missing_failure_and_success() {
        let tempdir = tempdir().expect("tempdir");
        let success =
            write_mock_command(tempdir.path(), "doctor-ok", "printf '1.2.3\\n'\nexit 0\n");
        let failure =
            write_mock_command(tempdir.path(), "doctor-fail", "echo 'boom' >&2\nexit 7\n");

        assert_eq!(
            command_version(success.to_str().expect("utf8 path"), &[]).as_deref(),
            Ok("1.2.3")
        );
        assert!(
            command_version("definitely-missing-doctor-command", &[])
                .err()
                .is_some_and(|message| message.contains("was not found"))
        );
        assert!(
            command_version(failure.to_str().expect("utf8 path"), &[])
                .err()
                .is_some_and(|message| message.contains("boom"))
        );
    }

    #[test]
    fn command_version_covers_spawn_exit_status_and_decode_errors() {
        let tempdir = tempdir().expect("tempdir");
        let non_executable = tempdir.path().join("doctor-noexec");
        fs::write(&non_executable, "not executable\n").expect("plain file");
        assert!(
            command_version(non_executable.to_str().expect("utf8 path"), &[])
                .err()
                .is_some_and(|message| message.contains("failed to run"))
        );

        let no_stderr = write_mock_command(tempdir.path(), "doctor-status", "exit 7\n");
        assert!(
            command_version(no_stderr.to_str().expect("utf8 path"), &[])
                .err()
                .is_some_and(|message| message.contains("exit status"))
        );

        let invalid_utf8 = write_mock_command(
            tempdir.path(),
            "doctor-bytes",
            "printf '\\377\\376'\nexit 0\n",
        );
        assert!(
            command_version(invalid_utf8.to_str().expect("utf8 path"), &[])
                .err()
                .is_some_and(|message| message.contains("failed to decode"))
        );
    }

    #[test]
    fn version_comparison_handles_missing_non_zero_tail() {
        assert!(!version_at_least("1", "1.0.1"));
    }

    #[test]
    fn helper_builders_cover_remaining_status_paths() {
        let tempdir = tempdir().expect("tempdir");
        let cargo_toml = tempdir.path().join("Cargo.toml");
        let skipped = build_rust_msrv_check(&cargo_toml, None, Ok("rustc 1.88.0".to_string()));
        assert_eq!(skipped.status, DoctorStatus::Skipped);

        let error = build_rust_msrv_check(&cargo_toml, Some("1.88"), Err("missing".to_string()));
        assert_eq!(error.status, DoctorStatus::Error);

        let node_without_expectation = build_node_check(
            "Browser app",
            "app",
            None,
            Path::new("app/.nvmrc"),
            "v25.0.0",
        );
        assert_eq!(node_without_expectation.status, DoctorStatus::Ok);
        assert_eq!(node_without_expectation.message, "found v25.0.0");

        let npm_warning = build_npm_check(
            "Docs site",
            "website",
            Some("10.0.0"),
            Path::new("website/package.json"),
            "11.8.0",
        );
        assert_eq!(npm_warning.status, DoctorStatus::Warning);
        assert!(
            npm_warning
                .fix
                .as_deref()
                .is_some_and(|fix| fix.contains("website/package.json"))
        );

        let npm_without_expectation = build_npm_check(
            "Browser app",
            "app",
            None,
            Path::new("app/package.json"),
            "11.8.0",
        );
        assert_eq!(npm_without_expectation.status, DoctorStatus::Ok);
        assert_eq!(npm_without_expectation.message, "found npm 11.8.0");
    }

    #[test]
    fn playwright_helpers_cover_cache_and_status_variants() {
        let tempdir = tempdir().expect("tempdir");
        let app_root = tempdir.path().join("app");

        let skipped = build_playwright_check(&app_root, None);
        assert_eq!(skipped.status, DoctorStatus::Skipped);

        fs::create_dir_all(&app_root).expect("app root");
        fs::write(app_root.join("package.json"), "{ \"name\": \"doctor\" }\n").expect("package");
        let warning = build_playwright_check(&app_root, None);
        assert_eq!(warning.status, DoctorStatus::Warning);
        assert!(warning.message.contains("app dependencies are missing"));
        assert!(
            warning
                .fix
                .as_deref()
                .is_some_and(|fix| fix.contains("pinned-npm.sh exec app -- ci"))
        );

        fs::create_dir_all(app_root.join("node_modules")).expect("node_modules");
        let no_cache = build_playwright_check(&app_root, None);
        assert_eq!(no_cache.status, DoctorStatus::Warning);
        assert!(
            no_cache
                .message
                .contains("no installed Playwright Chromium bundle")
        );
        assert!(
            no_cache
                .fix
                .as_deref()
                .is_some_and(|fix| fix.contains("pinned-npm.sh exec app -- exec playwright"))
        );

        let chromium = tempdir.path().join("pw-cache/chromium-1000");
        fs::create_dir_all(&chromium).expect("chromium");
        let ok = build_playwright_check(&app_root, Some(&chromium));
        assert_eq!(ok.status, DoctorStatus::Ok);

        let broken_marker = app_root.join("node_modules/.package-lock.json");
        #[cfg(unix)]
        std::os::unix::fs::symlink(app_root.join("missing"), &broken_marker).expect("symlink");
        #[cfg(windows)]
        fs::write(&broken_marker, "{}\n").expect("marker");
        let _ = modified_time(&app_root.join("missing"));
    }

    #[test]
    fn filesystem_and_env_helpers_cover_fallback_paths() {
        let custom = PathBuf::from("/tmp/playwright");
        let home = PathBuf::from("/tmp/home");
        let local_app_data = PathBuf::from("/tmp/local");
        assert_eq!(
            playwright_cache_roots_from(
                Some(custom.clone()),
                Some(home.clone()),
                Some(local_app_data.clone())
            ),
            vec![
                custom,
                home.join(".cache/ms-playwright"),
                home.join("Library/Caches/ms-playwright"),
                local_app_data.join("ms-playwright")
            ]
        );
        assert_eq!(
            home_dir_from(None, Some(PathBuf::from("/tmp/profile"))),
            Some(PathBuf::from("/tmp/profile"))
        );
        assert_eq!(home_dir_from(None, None), None);
        assert!(modified_time(Path::new("/definitely/missing")).is_none());
        assert!(dependency_install_needs_refresh(None, None));
        assert!(!cargo_executable().is_empty());
        assert!(!npm_executable().is_empty());
    }

    #[test]
    fn report_builders_run_without_env_mutation() {
        let tempdir = tempdir().expect("tempdir");
        fs::write(tempdir.path().join("syu.yaml"), "version: 1\n").expect("config");
        let report = build_doctor_report(tempdir.path()).expect("report should build");
        assert_eq!(
            report.checks.first().map(|check| check.id),
            Some("workspace-config")
        );

        let exit_code = run_doctor_command(&DoctorArgs {
            workspace: tempdir.path().to_path_buf(),
            format: crate::cli::OutputFormat::Json,
        })
        .expect("doctor command should run");
        assert!(exit_code >= 0);
    }

    #[test]
    fn surface_dependency_and_playwright_finders_cover_edge_cases() {
        let tempdir = tempdir().expect("tempdir");
        let surface_root = tempdir.path().join("app");
        fs::create_dir_all(surface_root.join("node_modules")).expect("node_modules");
        fs::write(surface_root.join("package-lock.json"), "{}\n").expect("lockfile");
        #[cfg(unix)]
        std::os::unix::fs::symlink(
            surface_root.join("missing-marker"),
            surface_root.join("node_modules/.package-lock.json"),
        )
        .expect("symlink");
        #[cfg(windows)]
        fs::write(surface_root.join("node_modules/.package-lock.json"), "{}\n").expect("marker");
        let warning = surface_dependency_check("Browser app", "app", &surface_root, "install");
        assert_eq!(warning.status, DoctorStatus::Warning);

        let root = tempdir.path().join("pw-root");
        fs::create_dir_all(&root).expect("root");
        fs::write(root.join("notes.txt"), "ignore\n").expect("file");
        fs::create_dir_all(root.join("chromium-1234")).expect("chromium");
        assert_eq!(
            find_playwright_chromium_in(&root),
            Some(root.join("chromium-1234"))
        );
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
}
