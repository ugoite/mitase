use crate::output::{ColorStyle, TerminalCapabilities};
use mitase_diagnostics::{Diagnostic, Severity, ValidationResult};
use std::{fmt::Write as _, time::Duration};

pub(crate) struct HumanRenderer {
    capabilities: TerminalCapabilities,
}

impl HumanRenderer {
    pub(crate) fn new() -> Self {
        Self {
            capabilities: TerminalCapabilities::detect(),
        }
    }

    pub(crate) fn new_for_stderr() -> Self {
        Self {
            capabilities: TerminalCapabilities::detect_stderr(),
        }
    }

    #[cfg(test)]
    fn with_capabilities(capabilities: TerminalCapabilities) -> Self {
        Self { capabilities }
    }

    pub(crate) fn render_validation_result(
        &self,
        result: &ValidationResult,
        operation: &str,
        elapsed: Duration,
    ) -> String {
        self.render_validation_result_with_scope(result, operation, elapsed, None)
    }

    pub(crate) fn render_change_validation_result(
        &self,
        result: &ValidationResult,
        elapsed: Duration,
        scope: &str,
    ) -> String {
        self.render_validation_result_with_scope(result, "validate change", elapsed, Some(scope))
    }

    fn render_validation_result_with_scope(
        &self,
        result: &ValidationResult,
        operation: &str,
        elapsed: Duration,
        scope: Option<&str>,
    ) -> String {
        let mut output = String::new();
        let summary = Summary::from_result(result, elapsed);
        let outcome = if result.is_valid() {
            "passed"
        } else {
            "failed"
        };
        let outcome_marker = if result.is_valid() { "✓" } else { "×" };
        let outcome_style = if result.is_valid() {
            ColorStyle::Success
        } else {
            ColorStyle::Error
        };
        let width = usize::from(self.capabilities.width()).max(2);
        let summary_lines = wrap_text(
            &format!("{operation} {outcome} · {}", summary.render()),
            width - 2,
        );
        writeln!(
            output,
            "{} {}",
            self.capabilities.paint(outcome_style, outcome_marker),
            summary_lines[0]
        )
        .expect("writing to a String cannot fail");
        for line in summary_lines.iter().skip(1) {
            writeln!(output, "  {line}").expect("writing to a String cannot fail");
        }
        if let Some(scope) = scope {
            let scope_lines = wrap_text(&format!("Scope: {scope}"), width - 2);
            for line in scope_lines {
                writeln!(output, "  {line}").expect("writing to a String cannot fail");
            }
        }

        for (index, diagnostic) in result.diagnostics.iter().enumerate() {
            if index > 0 {
                output.push('\n');
            }
            self.render_diagnostic(&mut output, diagnostic);
        }

        output
    }

    fn render_diagnostic(&self, output: &mut String, diagnostic: &Diagnostic) {
        let (marker, style) = match diagnostic.severity {
            Severity::Error => ("×", ColorStyle::Error),
            Severity::Warning => ("!", ColorStyle::Warning),
            Severity::Info => ("·", ColorStyle::Metadata),
        };
        let prefix_width = marker.chars().count() + 1;
        let width = usize::from(self.capabilities.width()).max(prefix_width);
        let content_width = width.saturating_sub(prefix_width).max(1);
        let lines = wrap_text(&diagnostic.render_text(), content_width);

        write!(
            output,
            "{} {}",
            self.capabilities.paint(style, marker),
            lines.first().map_or("", String::as_str)
        )
        .expect("writing to a String cannot fail");
        for line in lines.iter().skip(1) {
            write!(output, "\n  {line}").expect("writing to a String cannot fail");
        }
        output.push('\n');
    }
}

pub(crate) struct CompactRenderer;

impl CompactRenderer {
    pub(crate) fn render_validation_result(
        result: &ValidationResult,
        operation: &str,
        elapsed: Duration,
        scope: Option<&str>,
    ) -> String {
        let summary = Summary::from_result(result, elapsed);
        let outcome = if result.is_valid() {
            "passed"
        } else {
            "failed"
        };
        let mut output = format!(
            "{operation} {outcome} | errors={} | warnings={} | infos={} | elapsed={:.2}s",
            summary.errors,
            summary.warnings,
            summary.infos,
            summary.elapsed.as_secs_f64()
        );
        if let Some(scope) = scope {
            write!(output, " | scope={}", compact_text(scope)).expect("writing to String");
        }
        output.push('\n');
        for diagnostic in &result.diagnostics {
            writeln!(output, "{}", compact_diagnostic(diagnostic))
                .expect("writing to a String cannot fail");
        }
        output
    }
}

fn compact_diagnostic(diagnostic: &Diagnostic) -> String {
    let severity = match diagnostic.severity {
        Severity::Error => "error",
        Severity::Warning => "warning",
        Severity::Info => "info",
    };
    let mut output = format!(
        "{}: {severity}[{}]: {}",
        compact_location(&diagnostic.primary),
        diagnostic.rule_id,
        compact_text(&diagnostic.message)
    );
    if let Some(relation) = &diagnostic.relation {
        let targets = relation
            .targets
            .iter()
            .map(|target| format!("{}={}", target.kind, compact_text(&target.value)))
            .collect::<Vec<_>>()
            .join(",");
        write!(
            output,
            " | relation={} {}={} -> {}",
            relation.relation,
            relation.source.kind,
            compact_text(&relation.source.value),
            targets
        )
        .expect("writing to String");
    }
    if !diagnostic.next.is_empty() {
        let next = diagnostic
            .next
            .iter()
            .map(|hint| format!("{}={}", hint.kind, compact_text(&hint.value)))
            .collect::<Vec<_>>()
            .join(",");
        write!(output, " | next={next}").expect("writing to String");
    }
    output
}

fn compact_location(location: &mitase_diagnostics::Location) -> String {
    let mut output = location.path.clone();
    if let Some(line) = location.line {
        write!(output, ":{line}").expect("writing to String");
        if let Some(column) = location.column {
            write!(output, ":{column}").expect("writing to String");
        }
    }
    output
}

fn compact_text(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[derive(Debug, Clone, Copy)]
struct Summary {
    errors: usize,
    warnings: usize,
    infos: usize,
    elapsed: Duration,
}

impl Summary {
    fn from_result(result: &ValidationResult, elapsed: Duration) -> Self {
        let mut summary = Self {
            errors: 0,
            warnings: 0,
            infos: 0,
            elapsed,
        };
        for diagnostic in &result.diagnostics {
            match diagnostic.severity {
                Severity::Error => summary.errors += 1,
                Severity::Warning => summary.warnings += 1,
                Severity::Info => summary.infos += 1,
            }
        }
        summary
    }

    fn render(self) -> String {
        let diagnostics = self.errors + self.warnings + self.infos;
        let counts = if diagnostics == 0 {
            "0 diagnostics".to_string()
        } else {
            format!(
                "{} · {} · {}",
                count_label(self.errors, "error", "errors"),
                count_label(self.warnings, "warning", "warnings"),
                count_label(self.infos, "info", "infos")
            )
        };
        format!("{counts} · {:.2}s", self.elapsed.as_secs_f64())
    }
}

fn count_label(count: usize, singular: &str, plural: &str) -> String {
    if count == 1 {
        format!("1 {singular}")
    } else {
        format!("{count} {plural}")
    }
}

pub(crate) fn wrap_text(value: &str, width: usize) -> Vec<String> {
    let width = width.max(1);
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in value.split_whitespace() {
        let word_width = word.chars().count();
        if word_width > width {
            if !current.is_empty() {
                lines.push(std::mem::take(&mut current));
            }
            let mut remainder = word;
            while remainder.chars().count() > width {
                let split_at = remainder
                    .char_indices()
                    .nth(width)
                    .map_or(remainder.len(), |(index, _)| index);
                lines.push(remainder[..split_at].to_string());
                remainder = &remainder[split_at..];
            }
            current.push_str(remainder);
        } else if current.is_empty() {
            current.push_str(word);
        } else if current.chars().count() + 1 + word_width <= width {
            current.push(' ');
            current.push_str(word);
        } else {
            lines.push(std::mem::take(&mut current));
            current.push_str(word);
        }
    }
    if !current.is_empty() || lines.is_empty() {
        lines.push(current);
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::output::ColorPolicy;

    fn result_with(diagnostics: Vec<Diagnostic>) -> ValidationResult {
        ValidationResult {
            diagnostics,
            readiness: None,
        }
    }

    #[test]
    fn summary_puts_success_and_zero_count_first() {
        let renderer = HumanRenderer::with_capabilities(TerminalCapabilities::from_environment(
            false,
            Some("80"),
            false,
        ));
        let rendered = renderer.render_validation_result(
            &result_with(Vec::new()),
            "check",
            Duration::from_millis(420),
        );

        assert_eq!(rendered, "✓ check passed · 0 diagnostics · 0.42s\n");
        assert_eq!(renderer.capabilities.color_policy(), ColorPolicy::Disabled);
    }

    #[test]
    fn change_validation_renders_scope_after_outcome() {
        let renderer = HumanRenderer::with_capabilities(TerminalCapabilities::from_environment(
            false,
            Some("80"),
            false,
        ));
        let rendered = renderer.render_change_validation_result(
            &result_with(Vec::new()),
            Duration::from_millis(420),
            "working tree changes · baseline: abc123 (default fallback chain)",
        );

        assert_eq!(
            rendered,
            "✓ validate change passed · 0 diagnostics · 0.42s\n  Scope: working tree changes · baseline: abc123 (default fallback chain)\n"
        );
    }

    #[test]
    fn compact_validation_is_one_line_per_diagnostic_without_ansi() {
        let mut diagnostic = Diagnostic::error(
            "MITASE-TARGET-002",
            "target resolution\nis ambiguous",
            "spec/requirements.yaml",
        )
        .with_span(12, 5, 12, 19)
        .with_relation(
            "verifies",
            mitase_diagnostics::DiagnosticSubject {
                kind: "bound-target".into(),
                value: "FEAT-DEMO-001#binding.verification/target.contract".into(),
            },
            mitase_diagnostics::DiagnosticSubject {
                kind: "spec-anchor".into(),
                value: "REQ-DEMO-001#criterion.acceptance".into(),
            },
        );
        diagnostic = diagnostic.with_next_read("show", "REQ-DEMO-001");

        let rendered = CompactRenderer::render_validation_result(
            &result_with(vec![diagnostic]),
            "check",
            Duration::from_millis(420),
            None,
        );

        assert_eq!(
            rendered,
            "check failed | errors=1 | warnings=0 | infos=0 | elapsed=0.42s\nspec/requirements.yaml:12:5: error[MITASE-TARGET-002]: target resolution is ambiguous | relation=verifies bound-target=FEAT-DEMO-001#binding.verification/target.contract -> spec-anchor=REQ-DEMO-001#criterion.acceptance | next=show=REQ-DEMO-001\n"
        );
        assert!(!rendered.contains('\x1b'));
        assert!(rendered.lines().all(|line| !line.contains('\n')));
    }

    #[test]
    fn mixed_diagnostics_have_markers_and_blank_boundaries() {
        let renderer = HumanRenderer::with_capabilities(TerminalCapabilities::from_environment(
            false,
            Some("120"),
            false,
        ));
        let rendered = renderer.render_validation_result(
            &result_with(vec![
                Diagnostic::error("MITASE-E001", "failed", "spec.yaml"),
                Diagnostic::warning("MITASE-W001", "check this", "other.yaml"),
            ]),
            "check",
            Duration::ZERO,
        );

        assert!(rendered.starts_with("× check failed · 1 error · 1 warning · 0 infos · 0.00s\n"));
        assert!(rendered.contains("× Error MITASE-E001"));
        assert!(rendered.contains("\n\n! Warning MITASE-W001"));
        assert!(!rendered.contains('\x1b'));
    }

    #[test]
    fn enabled_color_is_limited_to_markers() {
        let renderer = HumanRenderer::with_capabilities(TerminalCapabilities::from_environment(
            true,
            Some("80"),
            false,
        ));
        let rendered = renderer.render_validation_result(
            &result_with(vec![Diagnostic::error(
                "MITASE-E001",
                "failed",
                "spec.yaml",
            )]),
            "check",
            Duration::ZERO,
        );

        assert!(rendered.contains("\x1b[31m×\x1b[0m"));
        assert!(rendered.contains("Error MITASE-E001"));
    }

    #[test]
    fn diagnostics_wrap_to_the_configured_width() {
        let renderer = HumanRenderer::with_capabilities(TerminalCapabilities::from_environment(
            false,
            Some("24"),
            false,
        ));
        let rendered = renderer.render_validation_result(
            &result_with(vec![Diagnostic::error(
                "MITASE-E001",
                "a diagnostic with enough text to wrap",
                "spec/very-long-file-name.yaml",
            )]),
            "check",
            Duration::ZERO,
        );

        assert!(
            rendered
                .lines()
                .skip(1)
                .all(|line| line.chars().count() <= 24)
        );
        assert!(rendered.contains("\n  "));
    }

    #[test]
    fn diagnostics_at_tiny_columns_use_the_clamped_width() {
        let renderer = HumanRenderer::with_capabilities(TerminalCapabilities::from_environment(
            false,
            Some("1"),
            false,
        ));
        let rendered = renderer.render_validation_result(
            &result_with(vec![Diagnostic::error(
                "MITASE-E001",
                "a diagnostic with enough text to wrap",
                "spec.yaml",
            )]),
            "check",
            Duration::ZERO,
        );

        assert!(rendered.lines().all(|line| line.chars().count() <= 20));
    }
}
