use serde::{Deserialize, Serialize};
use std::{
    fmt,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpecKind {
    Philosophy,
    Policy,
    Requirement,
    Feature,
}

impl SpecKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Philosophy => "philosophy",
            Self::Policy => "policy",
            Self::Requirement => "requirement",
            Self::Feature => "feature",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SpecId(pub String);

impl From<String> for SpecId {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for SpecId {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

impl fmt::Display for SpecId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl AsRef<str> for SpecId {
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct WorkspaceRoot(pub PathBuf);

impl From<PathBuf> for WorkspaceRoot {
    fn from(value: PathBuf) -> Self {
        Self(value)
    }
}

impl From<&Path> for WorkspaceRoot {
    fn from(value: &Path) -> Self {
        Self(value.to_path_buf())
    }
}

impl AsRef<Path> for WorkspaceRoot {
    fn as_ref(&self) -> &Path {
        self.0.as_path()
    }
}

impl fmt::Display for WorkspaceRoot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.display().fmt(f)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct GitRange(pub String);

impl From<String> for GitRange {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for GitRange {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

impl AsRef<str> for GitRange {
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}

impl fmt::Display for GitRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct LanguageName(pub String);

impl From<String> for LanguageName {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for LanguageName {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

impl AsRef<str> for LanguageName {
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}

impl fmt::Display for LanguageName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TraceReference {
    pub file: PathBuf,
    #[serde(default, alias = "tests", alias = "functions")]
    pub symbols: Vec<String>,
    #[serde(default, alias = "docs", alias = "docstrings")]
    pub doc_contains: Vec<String>,
    #[serde(default)]
    pub method: Option<String>,
    #[serde(default)]
    pub path: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::{
        GitRange, Issue, LanguageName, Severity, SpecId, SpecKind, TraceReference, WorkspaceRoot,
    };
    use std::path::PathBuf;

    #[test]
    fn issue_constructors_set_expected_severity() {
        let error = Issue::error("e", "subject", Some("loc".to_string()), "message", None);
        let warning = Issue::warning("w", "subject", None, "message", Some("fix".to_string()));

        assert_eq!(error.severity, Severity::Error);
        assert_eq!(warning.severity, Severity::Warning);
        assert_eq!(warning.suggestion.as_deref(), Some("fix"));
    }

    #[test]
    fn trace_reference_roundtrips_through_yaml_and_json() {
        let reference = TraceReference {
            file: PathBuf::from("tests/task.rs"),
            symbols: vec!["smoke".to_string()],
            doc_contains: vec!["goal plan".to_string()],
            method: Some("get".to_string()),
            path: Some("/tasks".to_string()),
        };

        let yaml = serde_yaml::to_string(&reference).expect("yaml");
        let from_yaml: TraceReference = serde_yaml::from_str(&yaml).expect("yaml parse");
        assert_eq!(reference, from_yaml);

        let json = serde_json::to_string(&reference).expect("json");
        let from_json: TraceReference = serde_json::from_str(&json).expect("json parse");
        assert_eq!(reference, from_json);
    }

    #[test]
    fn wrapper_types_serialize_as_strings() {
        let spec_id = SpecId::from("REQ-001");
        let workspace_root = WorkspaceRoot::from(PathBuf::from("/repo"));
        let git_range = GitRange::from("origin/main...HEAD");
        let language = LanguageName::from("rust");

        assert_eq!(
            serde_json::to_string(&spec_id).expect("json"),
            "\"REQ-001\""
        );
        assert_eq!(
            serde_json::to_string(&workspace_root).expect("json"),
            "\"/repo\""
        );
        assert_eq!(
            serde_json::to_string(&git_range).expect("json"),
            "\"origin/main...HEAD\""
        );
        assert_eq!(serde_json::to_string(&language).expect("json"), "\"rust\"");
    }

    #[test]
    fn spec_kind_labels_are_stable() {
        assert_eq!(SpecKind::Philosophy.label(), "philosophy");
        assert_eq!(SpecKind::Policy.label(), "policy");
        assert_eq!(SpecKind::Requirement.label(), "requirement");
        assert_eq!(SpecKind::Feature.label(), "feature");
    }
}
