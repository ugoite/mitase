// REQ-CORE-004

use std::{
    fs,
    path::{Component, Path, PathBuf},
};

use anyhow::{Result, bail};

use crate::{
    cli::ReportArgs, command::check::collect_check_result_from_workspace,
    coverage::normalize_relative_path, model::CheckResult, report::render_markdown_report,
    rules::attach_referenced_rules, workspace::load_workspace,
};

// FEAT-REPORT-001
pub fn run_report_command(args: &ReportArgs) -> Result<i32> {
    let (result, output) = match load_workspace(&args.workspace) {
        Ok(workspace) => (
            collect_check_result_from_workspace(&workspace),
            resolve_report_output(
                &workspace.root,
                workspace.config.report.output.as_deref(),
                args.output.as_deref(),
            )?,
        ),
        Err(error) => (
            attach_referenced_rules(CheckResult::from_load_error(
                args.workspace.to_path_buf(),
                error.to_string(),
            )),
            args.output.clone(),
        ),
    };
    let markdown = render_markdown_report(&result);

    if let Some(output) = &output {
        if let Some(parent) = output.parent()
            && !parent.as_os_str().is_empty()
        {
            fs::create_dir_all(parent)?;
        }
        fs::write(output, markdown)?;
        println!("wrote report to {}", output.display());
    } else {
        println!("{markdown}");
    }

    Ok(if result.is_success() { 0 } else { 1 })
}

fn resolve_report_output(
    workspace_root: &Path,
    configured_output: Option<&Path>,
    cli_output: Option<&Path>,
) -> Result<Option<PathBuf>> {
    if let Some(output) = cli_output {
        return Ok(Some(output.to_path_buf()));
    }

    configured_output
        .map(|output| resolve_configured_report_output(workspace_root, output))
        .transpose()
}

fn resolve_configured_report_output(workspace_root: &Path, output: &Path) -> Result<PathBuf> {
    if output.is_absolute() {
        return Ok(output.to_path_buf());
    }

    let normalized = normalize_relative_path(output);
    if normalized.as_os_str().is_empty()
        || normalized.is_absolute()
        || normalized
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        bail!(
            "`report.output` in `syu.yaml` must stay within workspace root `{}`: `{}`",
            workspace_root.display(),
            output.display()
        );
    }

    Ok(workspace_root.join(normalized))
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::{Path, PathBuf},
    };

    use tempfile::tempdir;

    use crate::cli::ReportArgs;
    use crate::test_support::CurrentDirGuard;

    use super::{resolve_report_output, run_report_command};

    fn fixture_path(name: &str) -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/workspaces")
            .join(name)
    }

    fn write_config(root: &Path, output: Option<&str>) {
        let report_section = output
            .map(|output| format!("report:\n  output: {output}\n"))
            .unwrap_or_default();
        fs::write(
            root.join("syu.yaml"),
            format!(
                "version: {}\nspec:\n  root: docs/syu\nvalidate:\n  default_fix: false\n  allow_planned: true\n  require_non_orphaned_items: true\n  require_reciprocal_links: true\n  require_symbol_trace_coverage: false\n{report_section}runtimes:\n  python:\n    command: auto\n  node:\n    command: auto\n",
                env!("CARGO_PKG_VERSION"),
            ),
        )
        .expect("config");
    }

    #[test]
    fn report_command_returns_zero_for_passing_workspace() {
        let args = ReportArgs {
            workspace: fixture_path("passing"),
            output: None,
        };

        let code = run_report_command(&args).expect("report should succeed");
        assert_eq!(code, 0);
    }

    #[test]
    fn resolve_report_output_defaults_to_stdout_when_config_is_missing() {
        let tempdir = tempdir().expect("tempdir should exist");
        let output = resolve_report_output(tempdir.path(), None, None)
            .expect("default output should resolve");
        assert_eq!(output, None);
    }

    #[test]
    fn resolve_report_output_uses_workspace_relative_config_paths() {
        let tempdir = tempdir().expect("tempdir should exist");
        write_config(tempdir.path(), Some("docs/generated/syu-report.md"));

        let output = resolve_report_output(
            tempdir.path(),
            Some(Path::new("docs/generated/syu-report.md")),
            None,
        )
        .expect("configured output should resolve");

        assert_eq!(
            output,
            Some(tempdir.path().join("docs/generated/syu-report.md"))
        );
    }

    #[test]
    fn resolve_report_output_preserves_absolute_config_paths() {
        let tempdir = tempdir().expect("tempdir should exist");
        let absolute = tempdir.path().join("report.md");
        write_config(tempdir.path(), Some(absolute.to_string_lossy().as_ref()));

        let output = resolve_report_output(tempdir.path(), Some(&absolute), None)
            .expect("absolute output should resolve");

        assert_eq!(output, Some(absolute));
    }

    #[test]
    fn resolve_report_output_prefers_cli_paths_over_config() {
        let tempdir = tempdir().expect("tempdir should exist");
        write_config(tempdir.path(), Some("docs/generated/syu-report.md"));

        let output = resolve_report_output(
            tempdir.path(),
            Some(Path::new("docs/generated/syu-report.md")),
            Some(Path::new("custom/report.md")),
        )
        .expect("cli output should win");

        assert_eq!(output, Some(PathBuf::from("custom/report.md")));
    }

    #[test]
    fn resolve_report_output_rejects_config_paths_that_escape_the_workspace() {
        let tempdir = tempdir().expect("tempdir should exist");

        let error = resolve_report_output(tempdir.path(), Some(Path::new("../report.md")), None)
            .expect_err("escaping config output should fail");

        assert!(
            error
                .to_string()
                .contains("must stay within workspace root")
        );
    }

    #[test]
    fn report_command_returns_one_for_failing_workspace() {
        let args = ReportArgs {
            workspace: fixture_path("failing"),
            output: None,
        };

        let code = run_report_command(&args).expect("report should still render");
        assert_eq!(code, 1);
    }

    #[test]
    fn report_command_renders_load_errors_as_failing_reports() {
        let tempdir = tempdir().expect("tempdir should exist");
        let args = ReportArgs {
            workspace: tempdir.path().join("missing"),
            output: None,
        };

        let code = run_report_command(&args).expect("load errors should still render a report");
        assert_eq!(code, 1);
    }

    #[test]
    fn report_command_errors_when_output_parent_is_a_file() {
        let tempdir = tempdir().expect("tempdir should exist");
        let occupied = tempdir.path().join("occupied");
        fs::write(&occupied, "not a directory").expect("occupied file should exist");

        let args = ReportArgs {
            workspace: fixture_path("passing"),
            output: Some(occupied.join("report.md")),
        };

        let error = run_report_command(&args).expect_err("writing through file parent should fail");
        assert!(
            error.to_string().contains("Not a directory")
                || error.to_string().contains("File exists")
        );
    }

    #[test]
    fn report_command_writes_to_relative_paths_without_parent_directories() {
        let tempdir = tempdir().expect("tempdir should exist");
        let _current_dir = CurrentDirGuard::chdir(tempdir.path());

        let args = ReportArgs {
            workspace: fixture_path("passing"),
            output: Some(std::path::PathBuf::from("report.md")),
        };

        let result = run_report_command(&args).expect("relative report path should work");
        let report = fs::read_to_string(tempdir.path().join("report.md"))
            .expect("relative report should be written");

        assert_eq!(result, 0);
        assert!(report.contains("# syu validation report"));
    }

    #[cfg(unix)]
    #[test]
    fn report_command_handles_root_paths_without_parent() {
        let args = ReportArgs {
            workspace: fixture_path("passing"),
            output: Some(std::path::PathBuf::from("/")),
        };

        let error = run_report_command(&args).expect_err("writing to root directory should fail");
        assert!(!error.to_string().is_empty());
    }
}
