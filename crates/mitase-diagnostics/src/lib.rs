#![forbid(unsafe_code)]
use mitase_spec_model::{BoundTargetRef, SpecAnchor};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Error,
    Warning,
    Info,
}

/// The validation engine owns diagnostic categorisation. Consumers such as the
/// CLI must render this value rather than deriving a phase from a rule
/// identifier.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ValidationPhase {
    Config,
    Graph,
    Targets,
    Scope,
    #[default]
    Readiness,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Location {
    pub path: String,
    #[serde(default)]
    pub line: Option<u32>,
    #[serde(default)]
    pub column: Option<u32>,
    #[serde(default)]
    pub end_line: Option<u32>,
    #[serde(default)]
    pub end_column: Option<u32>,
    #[serde(default)]
    pub label: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RelatedLocation {
    pub location: Location,
    pub message: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DiagnosticSubject {
    pub kind: String,
    pub value: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Evidence {
    pub kind: String,
    pub value: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SafeFix {
    pub description: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Diagnostic {
    #[serde(rename = "code")]
    pub rule_id: String,
    #[serde(default)]
    pub phase: ValidationPhase,
    pub severity: Severity,
    #[serde(rename = "reason")]
    pub message: String,
    pub primary: Location,
    #[serde(rename = "related_spans")]
    #[serde(default)]
    pub related: Vec<RelatedLocation>,
    #[serde(default)]
    pub subject: Option<DiagnosticSubject>,
    #[serde(default)]
    pub reference: Option<DiagnosticSubject>,
    #[serde(default)]
    pub candidates: Vec<String>,
    #[serde(default)]
    pub evidence: Vec<Evidence>,
    #[serde(rename = "suggested_action")]
    #[serde(default)]
    pub help: Option<String>,
    #[serde(default)]
    pub fix: Option<SafeFix>,
}
impl Diagnostic {
    pub fn error(rule: &str, message: impl Into<String>, path: impl Into<String>) -> Self {
        Self {
            rule_id: rule.into(),
            phase: ValidationPhase::default(),
            severity: Severity::Error,
            message: message.into(),
            primary: Location {
                path: path.into(),
                line: None,
                column: None,
                end_line: None,
                end_column: None,
                label: None,
            },
            related: vec![],
            subject: None,
            reference: None,
            candidates: vec![],
            evidence: vec![],
            help: None,
            fix: None,
        }
    }

    pub fn set_subject_anchor(&mut self, anchor: &SpecAnchor) {
        self.subject = Some(DiagnosticSubject {
            kind: "spec-anchor".into(),
            value: anchor.to_string(),
        });
    }

    pub fn set_reference_target(&mut self, target: &BoundTargetRef) {
        self.reference = Some(DiagnosticSubject {
            kind: "bound-target".into(),
            value: target.to_string(),
        });
    }

    /// Render the stable human-readable representation used by CLI consumers.
    /// Structured consumers should serialize the diagnostic instead.
    pub fn render_text(&self) -> String {
        use std::fmt::Write as _;

        let mut output = format!(
            "{:?} {} {}: {}",
            self.severity, self.rule_id, self.primary.path, self.message
        );
        if !self.candidates.is_empty() {
            write!(output, " [candidates: {}]", self.candidates.join(", "))
                .expect("writing to a String cannot fail");
        }
        if let Some(action) = &self.help {
            write!(output, " [suggested action: {action}]")
                .expect("writing to a String cannot fail");
        }
        output
    }
}
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ValidationResult {
    pub diagnostics: Vec<Diagnostic>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub readiness: Option<serde_json::Value>,
}
impl ValidationResult {
    pub fn is_valid(&self) -> bool {
        !self
            .diagnostics
            .iter()
            .any(|d| d.severity == Severity::Error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn structured_fields_round_trip_through_json() {
        let anchor: SpecAnchor = "REQ-DIAGNOSTICS-001#criterion.contract".parse().unwrap();
        let target: BoundTargetRef = "FEAT-DIAGNOSTICS-001#binding.implementation/target.source"
            .parse()
            .unwrap();
        let mut diagnostic = Diagnostic::error(
            "MITASE-TARGET-002",
            "target resolution is ambiguous",
            "spec/requirements.yaml",
        );
        diagnostic.phase = ValidationPhase::Targets;
        diagnostic.primary.line = Some(12);
        diagnostic.primary.column = Some(5);
        diagnostic.primary.end_line = Some(12);
        diagnostic.primary.end_column = Some(19);
        diagnostic.primary.label = Some("target path".into());
        diagnostic.related.push(RelatedLocation {
            location: Location {
                path: "spec/features.yaml".into(),
                line: Some(8),
                column: Some(3),
                end_line: Some(8),
                end_column: Some(15),
                label: Some("related target".into()),
            },
            message: "another target has the same selector".into(),
        });
        diagnostic.set_subject_anchor(&anchor);
        diagnostic.set_reference_target(&target);
        diagnostic.candidates = vec!["src/one.rs".into(), "src/two.rs".into()];
        diagnostic.help = Some("make the selector unique".into());

        let value = serde_json::to_value(&diagnostic).unwrap();
        assert_eq!(value["code"], "MITASE-TARGET-002");
        assert_eq!(value["reason"], "target resolution is ambiguous");
        assert_eq!(value["primary"]["column"], 5);
        assert_eq!(value["primary"]["end_column"], 19);
        assert!(value["related_spans"].is_array());
        assert_eq!(value["subject"]["kind"], "spec-anchor");
        assert_eq!(value["reference"]["kind"], "bound-target");
        assert_eq!(value["candidates"][0], "src/one.rs");
        assert_eq!(value["suggested_action"], "make the selector unique");
        assert!(value.get("rule_id").is_none());
        assert!(value.get("message").is_none());

        let decoded: Diagnostic = serde_json::from_value(value).unwrap();
        assert_eq!(decoded, diagnostic);
        assert_eq!(
            diagnostic.render_text(),
            "Error MITASE-TARGET-002 spec/requirements.yaml: target resolution is ambiguous [candidates: src/one.rs, src/two.rs] [suggested action: make the selector unique]"
        );
    }
}
