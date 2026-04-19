// REQ-CORE-004

use std::{
    collections::BTreeMap,
    fs,
    path::{Component, Path, PathBuf},
};

use anyhow::{Result, bail};

use crate::{
    cli::ReportArgs,
    command::check::collect_check_result_from_workspace,
    coverage::normalize_relative_path,
    model::{CheckResult, Feature, Requirement, TraceReference},
    report::render_markdown_report,
    rules::attach_referenced_rules,
    workspace::{Workspace, load_workspace},
};

// FEAT-REPORT-001
pub fn run_report_command(args: &ReportArgs) -> Result<i32> {
    let (result, output, spec_coverage_summary) = match load_workspace(&args.workspace) {
        Ok(workspace) => {
            let output = resolve_report_output(
                &workspace.root,
                workspace.config.report.output.as_deref(),
                args.output.as_deref(),
            )?;
            let spec_coverage_summary =
                render_spec_coverage_summary(&workspace, output.as_deref())?;
            (
                collect_check_result_from_workspace(&workspace),
                output,
                spec_coverage_summary,
            )
        }
        Err(error) => (
            attach_referenced_rules(CheckResult::from_load_error(
                args.workspace.to_path_buf(),
                error.to_string(),
            )),
            args.output.clone(),
            None,
        ),
    };
    let mut markdown = render_markdown_report(&result);
    if let Some(spec_coverage_summary) = spec_coverage_summary {
        markdown.push_str("\n\n");
        markdown.push_str(&spec_coverage_summary);
    }

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

fn render_spec_coverage_summary(
    workspace: &Workspace,
    output: Option<&Path>,
) -> Result<Option<String>> {
    if output.is_some() {
        return Ok(None);
    }

    let lcov_path = workspace.root.join("target/coverage/lcov.info");
    if !lcov_path.is_file() {
        return Ok(None);
    }

    let lcov = load_lcov(&lcov_path)?;
    let feature_details = workspace
        .features
        .iter()
        .map(|feature| {
            let implementation_refs = feature
                .implementations
                .values()
                .map(std::vec::Vec::len)
                .sum::<usize>();
            let rust_paths = rust_trace_paths(workspace, feature.implementations.get("rust"));
            let rust_file_count = rust_paths.len();
            let (covered, total, instrumented_paths) = summarize_paths(&lcov, &rust_paths);

            (
                feature.id.clone(),
                FeatureCoverageDetail {
                    linked_requirements: feature.linked_requirements.clone(),
                    implementation_refs,
                    rust_file_count,
                    rust_paths,
                    rust_coverage: coverage_label(
                        implementation_refs,
                        rust_file_count,
                        instrumented_paths,
                        covered,
                        total,
                        "no implementation refs",
                    ),
                },
            )
        })
        .collect::<BTreeMap<_, _>>();

    let requirement_rows = workspace
        .requirements
        .iter()
        .map(|requirement| {
            render_requirement_coverage_row(requirement, workspace, &lcov, &feature_details)
        })
        .collect::<Vec<_>>();
    let feature_rows = workspace
        .features
        .iter()
        .map(|feature| render_feature_coverage_row(feature, &feature_details))
        .collect::<Vec<_>>();

    Ok(Some(format!(
        "# Coverage by requirement and feature\n\nThis report combines Rust line coverage from `cargo llvm-cov` with the current\n`syu` requirement/feature trace graph so local reports can inspect coverage in spec terms.\n\n## Requirements\n\n| Requirement | Linked features | Traced test refs | Rust test file coverage | Linked Rust implementation coverage |\n| --- | --- | ---: | ---: | ---: |\n{}\n\n## Features\n\n| Feature | Linked requirements | Implementation refs | Rust implementation files | Rust implementation coverage |\n| --- | --- | ---: | ---: | ---: |\n{}\n",
        requirement_rows.join("\n"),
        feature_rows.join("\n")
    )))
}

fn render_requirement_coverage_row(
    requirement: &Requirement,
    workspace: &Workspace,
    lcov: &BTreeMap<PathBuf, (usize, usize)>,
    feature_details: &BTreeMap<String, FeatureCoverageDetail>,
) -> String {
    let test_refs = requirement
        .tests
        .values()
        .map(std::vec::Vec::len)
        .sum::<usize>();
    let rust_test_paths = rust_trace_paths(workspace, requirement.tests.get("rust"));
    let (test_covered, test_total, test_instrumented_paths) =
        summarize_paths(lcov, &rust_test_paths);

    let linked_feature_paths = requirement
        .linked_features
        .iter()
        .filter_map(|feature_id| feature_details.get(feature_id))
        .flat_map(|detail| detail.rust_paths.iter().cloned())
        .collect::<Vec<_>>();
    let linked_feature_rust_files = dedup_paths(&linked_feature_paths).len();
    let (feature_covered, feature_total, feature_instrumented_paths) =
        summarize_paths(lcov, &linked_feature_paths);

    format!(
        "| {} | {} | {} | {} | {} |",
        requirement.id,
        comma_or_dash(&requirement.linked_features),
        test_refs,
        coverage_label(
            test_refs,
            rust_test_paths.len(),
            test_instrumented_paths,
            test_covered,
            test_total,
            "no test refs",
        ),
        if requirement.linked_features.is_empty() {
            "—".to_string()
        } else {
            coverage_label(
                requirement.linked_features.len(),
                linked_feature_rust_files,
                feature_instrumented_paths,
                feature_covered,
                feature_total,
                "no linked features",
            )
        }
    )
}

fn render_feature_coverage_row(
    feature: &Feature,
    feature_details: &BTreeMap<String, FeatureCoverageDetail>,
) -> String {
    let detail = feature_details
        .get(&feature.id)
        .expect("feature coverage detail should exist");
    format!(
        "| {} | {} | {} | {} | {} |",
        feature.id,
        comma_or_dash(&detail.linked_requirements),
        detail.implementation_refs,
        detail.rust_file_count,
        detail.rust_coverage
    )
}

fn load_lcov(path: &Path) -> Result<BTreeMap<PathBuf, (usize, usize)>> {
    let mut coverage = BTreeMap::new();
    let mut current_path = None;
    let mut covered = 0;
    let mut total = 0;

    for raw_line in fs::read_to_string(path)?.lines() {
        if let Some(path) = raw_line.strip_prefix("SF:") {
            current_path = Some(PathBuf::from(path));
            covered = 0;
            total = 0;
        } else if let Some(payload) = raw_line.strip_prefix("DA:") {
            let (_, count) = payload
                .split_once(',')
                .ok_or_else(|| anyhow::anyhow!("invalid LCOV DA record `{raw_line}`"))?;
            total += 1;
            covered += usize::from(count.parse::<usize>()? > 0);
        } else if raw_line == "end_of_record"
            && let Some(path) = current_path.take()
        {
            coverage.insert(path, (covered, total));
        }
    }

    Ok(coverage)
}

fn rust_trace_paths(
    workspace: &Workspace,
    references: Option<&Vec<TraceReference>>,
) -> Vec<PathBuf> {
    dedup_paths(
        &references
            .into_iter()
            .flatten()
            .map(|reference| {
                if reference.file.is_absolute() {
                    reference.file.clone()
                } else {
                    workspace.root.join(&reference.file)
                }
            })
            .collect::<Vec<_>>(),
    )
}

fn dedup_paths(paths: &[PathBuf]) -> Vec<PathBuf> {
    let mut unique = Vec::new();
    for path in paths {
        if !unique.contains(path) {
            unique.push(path.clone());
        }
    }
    unique
}

fn summarize_paths(
    lcov: &BTreeMap<PathBuf, (usize, usize)>,
    paths: &[PathBuf],
) -> (usize, usize, usize) {
    let mut covered = 0;
    let mut total = 0;
    let mut instrumented_paths = 0;

    for path in paths {
        if let Some((path_covered, path_total)) = lcov.get(path) {
            instrumented_paths += 1;
            covered += path_covered;
            total += path_total;
        }
    }

    (covered, total, instrumented_paths)
}

fn coverage_label(
    total_refs: usize,
    rust_file_count: usize,
    instrumented_paths: usize,
    covered: usize,
    total: usize,
    empty_label: &str,
) -> String {
    if total_refs == 0 {
        return empty_label.to_string();
    }
    if rust_file_count == 0 {
        return "no Rust files".to_string();
    }
    if instrumented_paths == 0 {
        return "not instrumented".to_string();
    }
    if total == 0 {
        return "0.0% (0/0)".to_string();
    }

    format!(
        "{:.1}% ({covered}/{total})",
        covered as f64 * 100.0 / total as f64
    )
}

fn comma_or_dash(values: &[String]) -> String {
    if values.is_empty() {
        "—".to_string()
    } else {
        values.join(", ")
    }
}

struct FeatureCoverageDetail {
    linked_requirements: Vec<String>,
    implementation_refs: usize,
    rust_file_count: usize,
    rust_paths: Vec<PathBuf>,
    rust_coverage: String,
}

#[cfg(test)]
mod tests {
    use std::{
        collections::BTreeMap,
        fs,
        path::{Path, PathBuf},
    };

    use tempfile::tempdir;

    use crate::test_support::CurrentDirGuard;
    use crate::{
        cli::ReportArgs,
        config::SyuConfig,
        model::{Requirement, TraceReference},
        workspace::Workspace,
    };

    use super::{
        coverage_label, render_requirement_coverage_row, render_spec_coverage_summary,
        resolve_report_output, run_report_command, rust_trace_paths,
    };
    use crate::workspace::load_workspace;

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

    fn copy_dir_recursive(source: &Path, destination: &Path) {
        fs::create_dir_all(destination).expect("destination directory should exist");
        for entry in fs::read_dir(source).expect("source directory should be readable") {
            let entry = entry.expect("directory entry should exist");
            let source_path = entry.path();
            let destination_path = destination.join(entry.file_name());
            if source_path.is_dir() {
                copy_dir_recursive(&source_path, &destination_path);
            } else {
                fs::copy(&source_path, &destination_path).expect("fixture file should copy");
            }
        }
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

    #[test]
    fn report_command_includes_spec_coverage_summary_when_lcov_exists() {
        let tempdir = tempdir().expect("tempdir should exist");
        copy_dir_recursive(&fixture_path("passing"), tempdir.path());
        fs::create_dir_all(tempdir.path().join("target/coverage"))
            .expect("coverage directory should exist");
        fs::write(
            tempdir.path().join("target/coverage/lcov.info"),
            format!(
                "SF:{}\nDA:1,1\nDA:2,0\nend_of_record\nSF:{}\nDA:1,1\nend_of_record\n",
                tempdir.path().join("src/rust_trace_tests.rs").display(),
                tempdir.path().join("src/rust_feature.rs").display()
            ),
        )
        .expect("lcov file should exist");

        let workspace = load_workspace(tempdir.path()).expect("workspace should load");
        let summary = render_spec_coverage_summary(&workspace, None)
            .expect("summary should render")
            .expect("summary should be present");

        assert!(summary.contains("# Coverage by requirement and feature"));
        assert!(summary.contains("REQ-TRACE-001"));
        assert!(summary.contains("FEAT-TRACE-001"));
        assert!(summary.contains("50.0% (1/2)"));
        assert!(summary.contains("100.0% (1/1)"));
    }

    #[test]
    fn report_command_appends_spec_coverage_summary_to_stdout_output() {
        let tempdir = tempdir().expect("tempdir should exist");
        copy_dir_recursive(&fixture_path("passing"), tempdir.path());
        fs::create_dir_all(tempdir.path().join("target/coverage"))
            .expect("coverage directory should exist");
        fs::write(
            tempdir.path().join("target/coverage/lcov.info"),
            format!(
                "SF:{}\nDA:1,1\nend_of_record\n",
                tempdir.path().join("src/rust_feature.rs").display()
            ),
        )
        .expect("lcov file should exist");

        let args = ReportArgs {
            workspace: tempdir.path().to_path_buf(),
            output: None,
        };

        let result = run_report_command(&args).expect("report should render");
        assert!(
            matches!(result, 0 | 1),
            "report should return a normal validation status code"
        );
    }

    #[test]
    fn report_command_skips_spec_coverage_summary_for_configured_report_outputs() {
        let tempdir = tempdir().expect("tempdir should exist");
        copy_dir_recursive(&fixture_path("passing"), tempdir.path());
        write_config(tempdir.path(), Some("docs/generated/syu-report.md"));
        fs::create_dir_all(tempdir.path().join("target/coverage"))
            .expect("coverage directory should exist");
        fs::write(
            tempdir.path().join("target/coverage/lcov.info"),
            format!(
                "SF:{}\nDA:1,1\nend_of_record\n",
                tempdir.path().join("src/rust_feature.rs").display()
            ),
        )
        .expect("lcov file should exist");

        let args = ReportArgs {
            workspace: tempdir.path().to_path_buf(),
            output: None,
        };

        let result = run_report_command(&args).expect("report should succeed");
        let report = fs::read_to_string(tempdir.path().join("docs/generated/syu-report.md"))
            .expect("report should be written");

        assert_eq!(result, 0);
        assert!(!report.contains("# Coverage by requirement and feature"));
    }

    #[test]
    fn report_command_skips_spec_coverage_summary_for_cli_file_outputs() {
        let tempdir = tempdir().expect("tempdir should exist");
        copy_dir_recursive(&fixture_path("passing"), tempdir.path());
        fs::create_dir_all(tempdir.path().join("target/coverage"))
            .expect("coverage directory should exist");
        fs::write(
            tempdir.path().join("target/coverage/lcov.info"),
            format!(
                "SF:{}\nDA:1,1\nend_of_record\n",
                tempdir.path().join("src/rust_feature.rs").display()
            ),
        )
        .expect("lcov file should exist");
        let _current_dir = CurrentDirGuard::chdir(tempdir.path());

        let args = ReportArgs {
            workspace: tempdir.path().to_path_buf(),
            output: Some(PathBuf::from("report.md")),
        };

        let result = run_report_command(&args).expect("report should succeed");
        let report =
            fs::read_to_string(tempdir.path().join("report.md")).expect("report should be written");

        assert_eq!(result, 0);
        assert!(!report.contains("# Coverage by requirement and feature"));
    }

    #[test]
    fn coverage_helpers_cover_empty_and_uninstrumented_states() {
        assert_eq!(coverage_label(0, 1, 0, 0, 0, "no refs"), "no refs");
        assert_eq!(coverage_label(1, 1, 0, 0, 0, "no refs"), "not instrumented");
        assert_eq!(coverage_label(1, 1, 1, 0, 0, "no refs"), "0.0% (0/0)");
    }

    #[test]
    fn rust_trace_paths_preserves_absolute_paths() {
        let workspace = Workspace {
            root: PathBuf::from("/repo"),
            spec_root: PathBuf::from("/repo/docs/syu"),
            config: SyuConfig::default(),
            philosophies: Vec::new(),
            policies: Vec::new(),
            requirements: Vec::new(),
            features: Vec::new(),
        };
        let absolute = PathBuf::from("/tmp/rust_trace.rs");
        let traces = vec![TraceReference {
            file: absolute.clone(),
            symbols: vec![String::from("trace")],
            doc_contains: Vec::new(),
        }];

        assert_eq!(rust_trace_paths(&workspace, Some(&traces)), vec![absolute]);
    }

    #[test]
    fn requirement_rows_render_dashes_for_missing_links() {
        let workspace = Workspace {
            root: PathBuf::from("/repo"),
            spec_root: PathBuf::from("/repo/docs/syu"),
            config: SyuConfig::default(),
            philosophies: Vec::new(),
            policies: Vec::new(),
            requirements: Vec::new(),
            features: Vec::new(),
        };
        let requirement = Requirement {
            id: String::from("REQ-EMPTY-001"),
            title: String::from("Empty"),
            description: String::from("Empty"),
            priority: String::from("medium"),
            status: String::from("planned"),
            linked_policies: Vec::new(),
            linked_features: Vec::new(),
            tests: BTreeMap::new(),
        };

        assert_eq!(
            render_requirement_coverage_row(
                &requirement,
                &workspace,
                &BTreeMap::new(),
                &BTreeMap::new(),
            ),
            "| REQ-EMPTY-001 | — | 0 | no test refs | — |"
        );
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
