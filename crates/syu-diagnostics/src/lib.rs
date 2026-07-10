#![forbid(unsafe_code)]
use serde::{Deserialize, Serialize};
use syu_spec_model::{BoundTargetRef, SpecAnchor};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Error,
    Warning,
    Info,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Location {
    pub path: String,
    #[serde(default)]
    pub line: Option<u32>,
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
pub struct ItemCoverage {
    pub item: String,
    pub kind: String,
    pub level: String,
    pub required: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CoverageSummary {
    pub target: String,
    pub required_items: usize,
    pub covered_items: usize,
    pub items: Vec<ItemCoverage>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Diagnostic {
    pub rule_id: String,
    pub severity: Severity,
    pub message: String,
    pub primary: Location,
    #[serde(default)]
    pub related: Vec<RelatedLocation>,
    #[serde(default)]
    pub anchor: Option<SpecAnchor>,
    #[serde(default)]
    pub target: Option<BoundTargetRef>,
    #[serde(default)]
    pub evidence: Vec<Evidence>,
    #[serde(default)]
    pub help: Option<String>,
    #[serde(default)]
    pub fix: Option<SafeFix>,
}
impl Diagnostic {
    pub fn error(rule: &str, message: impl Into<String>, path: impl Into<String>) -> Self {
        Self {
            rule_id: rule.into(),
            severity: Severity::Error,
            message: message.into(),
            primary: Location {
                path: path.into(),
                line: None,
                label: None,
            },
            related: vec![],
            anchor: None,
            target: None,
            evidence: vec![],
            help: None,
            fix: None,
        }
    }
}
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ValidationResult {
    pub diagnostics: Vec<Diagnostic>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub coverage: Option<CoverageSummary>,
}
impl ValidationResult {
    pub fn is_valid(&self) -> bool {
        !self
            .diagnostics
            .iter()
            .any(|d| d.severity == Severity::Error)
    }
}
