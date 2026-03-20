// REQ-CORE-004

use std::{collections::BTreeSet, fmt::Write};

use crate::model::{CheckResult, Issue, Severity};

// FEAT-REPORT-001
pub fn render_markdown_report(result: &CheckResult) -> String {
    let mut output = String::new();
    let status = if result.is_success() { "PASS" } else { "FAIL" };

    writeln!(&mut output, "# syu validation report").expect("writing to String must succeed");
    writeln!(&mut output).expect("writing to String must succeed");
    writeln!(&mut output, "## Status").expect("writing to String must succeed");
    writeln!(&mut output).expect("writing to String must succeed");
    writeln!(&mut output, "- Result: **{status}**").expect("writing to String must succeed");
    writeln!(
        &mut output,
        "- Workspace: `{}`",
        result.workspace_root.display()
    )
    .expect("writing to String must succeed");
    writeln!(&mut output).expect("writing to String must succeed");

    writeln!(&mut output, "## Definitions").expect("writing to String must succeed");
    writeln!(&mut output).expect("writing to String must succeed");
    writeln!(
        &mut output,
        "- Philosophies: {}",
        result.definition_counts.philosophies
    )
    .expect("writing to String must succeed");
    writeln!(
        &mut output,
        "- Policies: {}",
        result.definition_counts.policies
    )
    .expect("writing to String must succeed");
    writeln!(
        &mut output,
        "- Requirements: {}",
        result.definition_counts.requirements
    )
    .expect("writing to String must succeed");
    writeln!(
        &mut output,
        "- Features: {}",
        result.definition_counts.features
    )
    .expect("writing to String must succeed");
    writeln!(&mut output).expect("writing to String must succeed");

    writeln!(&mut output, "## Traceability").expect("writing to String must succeed");
    writeln!(&mut output).expect("writing to String must succeed");
    writeln!(
        &mut output,
        "- Requirement-to-test traceability: {}/{}",
        result.trace_summary.requirement_traces.validated,
        result.trace_summary.requirement_traces.declared
    )
    .expect("writing to String must succeed");
    writeln!(
        &mut output,
        "- Feature-to-implementation traceability: {}/{}",
        result.trace_summary.feature_traces.validated, result.trace_summary.feature_traces.declared
    )
    .expect("writing to String must succeed");
    writeln!(&mut output).expect("writing to String must succeed");

    writeln!(&mut output, "## Issues").expect("writing to String must succeed");
    writeln!(&mut output).expect("writing to String must succeed");
    if result.issues.is_empty() {
        writeln!(&mut output, "No issues found.").expect("writing to String must succeed");
    } else {
        writeln!(
            &mut output,
            "| Severity | Code | Rule | Subject | Location | Message |"
        )
        .expect("writing to String must succeed");
        writeln!(&mut output, "| --- | --- | --- | --- | --- | --- |")
            .expect("writing to String must succeed");
        for issue in &result.issues {
            writeln!(
                &mut output,
                "| {} | {} | {} | {} | {} | {} |",
                severity_label(&issue.severity),
                escape_cell(&issue.code),
                escape_cell(
                    &crate::rules::rule_by_code(&issue.code)
                        .map(|rule| rule.title.clone())
                        .unwrap_or_else(|| "-".to_string())
                ),
                escape_cell(&issue.subject),
                escape_cell(issue.location.as_deref().unwrap_or("-")),
                escape_cell(&issue.message)
            )
            .expect("writing to String must succeed");
        }
    }
    writeln!(&mut output).expect("writing to String must succeed");

    if !result.referenced_rules.is_empty() {
        writeln!(&mut output, "## Referenced rules").expect("writing to String must succeed");
        writeln!(&mut output).expect("writing to String must succeed");
        for rule in &result.referenced_rules {
            writeln!(&mut output, "### `{}` — {}", rule.code, rule.title)
                .expect("writing to String must succeed");
            writeln!(&mut output).expect("writing to String must succeed");
            writeln!(&mut output, "- Genre: {}", rule.genre)
                .expect("writing to String must succeed");
            writeln!(&mut output, "- Severity: {}", rule.severity)
                .expect("writing to String must succeed");
            writeln!(
                &mut output,
                "- Summary: {}",
                collapse_whitespace(&rule.summary)
            )
            .expect("writing to String must succeed");
            writeln!(
                &mut output,
                "- Description: {}",
                collapse_whitespace(&rule.description)
            )
            .expect("writing to String must succeed");
            writeln!(&mut output).expect("writing to String must succeed");
        }
    }

    writeln!(&mut output, "## Suggested next actions").expect("writing to String must succeed");
    writeln!(&mut output).expect("writing to String must succeed");
    let suggestions = collect_suggestions(&result.issues);
    if suggestions.is_empty() {
        writeln!(&mut output, "- No action needed.").expect("writing to String must succeed");
    } else {
        for suggestion in suggestions {
            writeln!(&mut output, "- {suggestion}").expect("writing to String must succeed");
        }
    }

    output
}

fn collect_suggestions(issues: &[Issue]) -> BTreeSet<String> {
    issues
        .iter()
        .filter_map(|issue| issue.suggestion.clone())
        .collect()
}

fn severity_label(severity: &Severity) -> &'static str {
    match severity {
        Severity::Error => "error",
        Severity::Warning => "warning",
    }
}

fn escape_cell(value: &str) -> String {
    value.replace('|', "\\|")
}

fn collapse_whitespace(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::model::{CheckResult, DefinitionCounts, Issue, Severity, TraceCount, TraceSummary};

    use super::render_markdown_report;

    #[test]
    fn report_renders_successful_result() {
        let result = CheckResult {
            workspace_root: PathBuf::from("/tmp/workspace"),
            definition_counts: DefinitionCounts {
                philosophies: 1,
                policies: 2,
                requirements: 3,
                features: 4,
            },
            trace_summary: TraceSummary {
                requirement_traces: TraceCount {
                    declared: 3,
                    validated: 3,
                },
                feature_traces: TraceCount {
                    declared: 4,
                    validated: 4,
                },
            },
            issues: Vec::new(),
            referenced_rules: Vec::new(),
        };

        let report = render_markdown_report(&result);
        assert!(report.contains("Result: **PASS**"));
        assert!(report.contains("No issues found."));
        assert!(report.contains("- No action needed."));
    }

    #[test]
    fn report_renders_issues_and_deduplicated_suggestions() {
        let result = CheckResult {
            workspace_root: PathBuf::from("/tmp/workspace"),
            definition_counts: DefinitionCounts::default(),
            trace_summary: TraceSummary::default(),
            issues: vec![
                Issue {
                    code: "SYU-workspace-duplicate-003".to_string(),
                    severity: Severity::Error,
                    subject: "feature|subject".to_string(),
                    location: Some("yaml:file.yml".to_string()),
                    message: "message|with pipe".to_string(),
                    suggestion: Some("fix it".to_string()),
                },
                Issue::warning(
                    "warn",
                    "workspace",
                    None,
                    "warning",
                    Some("fix it".to_string()),
                ),
            ],
            referenced_rules: crate::rules::referenced_rules(&[
                Issue {
                    code: "SYU-workspace-duplicate-003".to_string(),
                    severity: Severity::Error,
                    subject: "feature|subject".to_string(),
                    location: Some("yaml:file.yml".to_string()),
                    message: "message|with pipe".to_string(),
                    suggestion: Some("fix it".to_string()),
                },
                Issue::warning(
                    "warn",
                    "workspace",
                    None,
                    "warning",
                    Some("fix it".to_string()),
                ),
            ]),
        };

        let report = render_markdown_report(&result);
        assert!(report.contains("Result: **FAIL**"));
        assert!(report.contains("error"));
        assert!(report.contains("warning"));
        assert!(report.contains("SYU-workspace-duplicate-003"));
        assert!(report.contains("feature\\|subject"));
        assert!(report.contains("message\\|with pipe"));
        assert!(report.contains("## Referenced rules"));
        assert_eq!(report.matches("- fix it").count(), 1);
    }
}
