// FEAT-CHECK-001
// REQ-CORE-001

use std::{
    collections::{BTreeMap, BTreeSet},
    sync::LazyLock,
};

use serde::{Deserialize, Serialize};

use crate::model::{CheckResult, Issue};

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct RuleDocument {
    version: u32,
    rules: Vec<RuleDefinition>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct RuleDefinition {
    code: String,
    genre: String,
    severity: String,
    title: String,
    summary: String,
    description: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ReferencedRule {
    pub genre: String,
    pub code: String,
    pub severity: String,
    pub title: String,
    pub summary: String,
    pub description: String,
}

static RULE_FILE: &str = include_str!("../docs/syu/features/validation/validation.yaml");

static RULES_BY_CODE: LazyLock<BTreeMap<String, ReferencedRule>> = LazyLock::new(load_rules);

pub fn all_rules() -> Vec<ReferencedRule> {
    RULES_BY_CODE.values().cloned().collect()
}

pub fn rule_by_code(code: &str) -> Option<&'static ReferencedRule> {
    RULES_BY_CODE.get(code)
}

pub fn attach_referenced_rules(mut result: CheckResult) -> CheckResult {
    result.referenced_rules = referenced_rules(&result.issues);
    result
}

pub fn referenced_rules(issues: &[Issue]) -> Vec<ReferencedRule> {
    let mut seen = BTreeSet::new();
    let mut rules = Vec::new();

    for issue in issues {
        if !seen.insert(issue.code.clone()) {
            continue;
        }

        if let Some(rule) = rule_by_code(&issue.code) {
            rules.push(rule.clone());
        }
    }

    rules
}

fn load_rules() -> BTreeMap<String, ReferencedRule> {
    let mut rules = BTreeMap::new();

    let document: RuleDocument =
        serde_yaml::from_str(RULE_FILE).expect("built-in validation rules must parse");
    assert!(
        document.version >= 1,
        "built-in validation rule documents must declare a supported version"
    );

    for rule in document.rules {
        assert!(
            is_structured_rule_code(&rule.code),
            "built-in validation rule code must use SYU-[genre]-[content]-[number]: {}",
            rule.code
        );
        let entry = ReferencedRule {
            genre: rule.genre,
            code: rule.code.clone(),
            severity: rule.severity,
            title: rule.title,
            summary: rule.summary,
            description: rule.description,
        };

        let replaced = rules.insert(rule.code, entry);
        assert!(
            replaced.is_none(),
            "duplicate built-in validation rule code detected"
        );
    }

    rules
}

fn is_structured_rule_code(code: &str) -> bool {
    let parts: Vec<_> = code.split('-').collect();
    if parts.len() != 4 || parts[0] != "SYU" {
        return false;
    }

    parts[1..3]
        .iter()
        .all(|part| !part.is_empty() && part.chars().all(|ch| ch.is_ascii_lowercase()))
        && parts[3].len() == 3
        && parts[3].chars().all(|ch| ch.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use crate::model::{CheckResult, DefinitionCounts, Issue, TraceSummary};

    use super::{
        all_rules, attach_referenced_rules, is_structured_rule_code, referenced_rules, rule_by_code,
    };

    #[test]
    fn built_in_rule_catalog_loads_expected_entries() {
        let rules = all_rules();
        assert!(
            rules
                .iter()
                .any(|rule| rule.code == "SYU-workspace-load-001")
        );
        assert!(
            rules
                .iter()
                .any(|rule| rule.code == "SYU-graph-orphaned-001")
        );
        assert!(
            rules
                .iter()
                .any(|rule| rule.code == "SYU-coverage-public-001")
        );
    }

    #[test]
    fn referenced_rules_follow_issue_codes_without_duplicates() {
        let issues = vec![
            Issue::error(
                "SYU-workspace-duplicate-001",
                "subject",
                None,
                "message",
                None,
            ),
            Issue::warning(
                "SYU-workspace-duplicate-001",
                "subject",
                None,
                "message",
                None,
            ),
            Issue::error("SYU-graph-reference-001", "subject", None, "message", None),
            Issue::warning("warn", "subject", None, "message", None),
        ];

        let rules = referenced_rules(&issues);
        assert_eq!(rules.len(), 2);
        assert_eq!(rules[0].code, "SYU-workspace-duplicate-001");
        assert_eq!(rules[1].code, "SYU-graph-reference-001");
    }

    #[test]
    fn attach_referenced_rules_enriches_check_results() {
        let result = CheckResult {
            workspace_root: ".".into(),
            definition_counts: DefinitionCounts::default(),
            trace_summary: TraceSummary::default(),
            issues: vec![Issue::error(
                "SYU-workspace-load-001",
                "workspace",
                None,
                "boom",
                None,
            )],
            referenced_rules: Vec::new(),
        };

        let attached = attach_referenced_rules(result);
        assert_eq!(attached.referenced_rules.len(), 1);
        assert_eq!(attached.referenced_rules[0].code, "SYU-workspace-load-001");
    }

    #[test]
    fn rule_lookup_returns_none_for_unknown_codes() {
        assert!(rule_by_code("warn").is_none());
    }

    #[test]
    fn structured_rule_code_validation_rejects_malformed_codes() {
        assert!(is_structured_rule_code("SYU-graph-reference-001"));
        assert!(!is_structured_rule_code("graph-reference-001"));
        assert!(!is_structured_rule_code("SYU-Graph-reference-001"));
        assert!(!is_structured_rule_code("SYU-graph-reference-1"));
    }
}
