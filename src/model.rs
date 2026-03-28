// FEAT-CHECK-001
// REQ-CORE-001

use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::PathBuf};

use crate::rules::ReferencedRule;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PhilosophyDocument {
    pub category: String,
    pub version: u32,
    pub language: Option<String>,
    pub philosophies: Vec<Philosophy>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Philosophy {
    pub id: String,
    pub title: String,
    pub product_design_principle: String,
    pub coding_guideline: String,
    #[serde(default)]
    pub linked_policies: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PolicyDocument {
    pub category: String,
    pub version: u32,
    pub language: Option<String>,
    pub policies: Vec<Policy>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Policy {
    pub id: String,
    pub title: String,
    pub summary: String,
    pub description: String,
    #[serde(default)]
    pub linked_philosophies: Vec<String>,
    #[serde(default)]
    pub linked_requirements: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RequirementDocument {
    pub category: String,
    pub prefix: String,
    pub requirements: Vec<Requirement>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Requirement {
    pub id: String,
    pub title: String,
    pub description: String,
    pub priority: String,
    pub status: String,
    #[serde(default)]
    pub linked_policies: Vec<String>,
    #[serde(default)]
    pub linked_features: Vec<String>,
    #[serde(default)]
    pub tests: BTreeMap<String, Vec<TraceReference>>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FeatureRegistryDocument {
    pub version: String,
    pub updated: Option<String>,
    pub files: Vec<FeatureRegistryEntry>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FeatureRegistryEntry {
    pub kind: String,
    pub file: PathBuf,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FeatureDocument {
    pub category: String,
    pub version: u32,
    pub features: Vec<Feature>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Feature {
    pub id: String,
    pub title: String,
    pub summary: String,
    pub status: String,
    #[serde(default)]
    pub linked_requirements: Vec<String>,
    #[serde(default)]
    pub implementations: BTreeMap<String, Vec<TraceReference>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TraceReference {
    pub file: PathBuf,
    #[serde(default, alias = "tests", alias = "functions")]
    pub symbols: Vec<String>,
    #[serde(default, alias = "docs", alias = "docstrings")]
    pub doc_contains: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct DefinitionCounts {
    pub philosophies: usize,
    pub policies: usize,
    pub requirements: usize,
    pub features: usize,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct TraceSummary {
    pub requirement_traces: TraceCount,
    pub feature_traces: TraceCount,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct TraceCount {
    pub declared: usize,
    pub validated: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct CheckResult {
    pub workspace_root: PathBuf,
    pub definition_counts: DefinitionCounts,
    pub trace_summary: TraceSummary,
    pub issues: Vec<Issue>,
    pub referenced_rules: Vec<ReferencedRule>,
}

impl CheckResult {
    pub fn from_load_error(workspace_root: PathBuf, message: impl Into<String>) -> Self {
        Self {
            workspace_root,
            definition_counts: DefinitionCounts::default(),
            trace_summary: TraceSummary::default(),
            issues: vec![Issue::error(
                "SYU-workspace-load-001",
                "workspace",
                None,
                message.into(),
                Some(
                    "New workspace? Run `syu init .` in the repository root. Otherwise make sure `syu.yaml` and `docs/syu/` exist under the selected workspace."
                        .to_string(),
                ),
            )],
            referenced_rules: Vec::new(),
        }
    }

    pub fn is_success(&self) -> bool {
        self.issues
            .iter()
            .all(|issue| issue.severity != Severity::Error)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Debug, Clone, Serialize)]
pub struct Issue {
    pub code: String,
    pub severity: Severity,
    pub subject: String,
    pub location: Option<String>,
    pub message: String,
    pub suggestion: Option<String>,
}

impl Issue {
    pub fn error(
        code: impl Into<String>,
        subject: impl Into<String>,
        location: Option<String>,
        message: impl Into<String>,
        suggestion: Option<String>,
    ) -> Self {
        Self {
            code: code.into(),
            severity: Severity::Error,
            subject: subject.into(),
            location,
            message: message.into(),
            suggestion,
        }
    }

    pub fn warning(
        code: impl Into<String>,
        subject: impl Into<String>,
        location: Option<String>,
        message: impl Into<String>,
        suggestion: Option<String>,
    ) -> Self {
        Self {
            code: code.into(),
            severity: Severity::Warning,
            subject: subject.into(),
            location,
            message: message.into(),
            suggestion,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{CheckResult, Issue, Severity};

    #[test]
    fn load_error_result_is_unsuccessful() {
        let result = CheckResult::from_load_error(PathBuf::from("."), "boom");
        assert!(!result.is_success());
        assert_eq!(result.issues.len(), 1);
        assert_eq!(result.issues[0].code, "SYU-workspace-load-001");
    }

    #[test]
    fn warning_only_result_is_successful() {
        let result = CheckResult {
            workspace_root: PathBuf::from("."),
            definition_counts: Default::default(),
            trace_summary: Default::default(),
            issues: vec![Issue::warning(
                "warn",
                "workspace",
                None,
                "only warning",
                None,
            )],
            referenced_rules: Vec::new(),
        };

        assert!(result.is_success());
    }

    #[test]
    fn issue_constructors_set_expected_severity() {
        let error = Issue::error("e", "subject", Some("loc".to_string()), "message", None);
        let warning = Issue::warning("w", "subject", None, "message", Some("fix".to_string()));

        assert_eq!(error.severity, Severity::Error);
        assert_eq!(warning.severity, Severity::Warning);
        assert_eq!(warning.suggestion.as_deref(), Some("fix"));
    }
}
