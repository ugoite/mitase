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
    genre: String,
    version: u32,
    rules: Vec<RuleDefinition>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct RuleDefinition {
    code: String,
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

static RULE_FILES: &[&str] = &[
    include_str!("../docs/syu/errors/workspace.yaml"),
    include_str!("../docs/syu/errors/graph.yaml"),
    include_str!("../docs/syu/errors/delivery.yaml"),
    include_str!("../docs/syu/errors/traceability.yaml"),
    include_str!("../docs/syu/errors/coverage.yaml"),
];

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

    for raw in RULE_FILES {
        let document: RuleDocument =
            serde_yaml::from_str(raw).expect("built-in validation rules must parse");
        assert!(
            document.version >= 1,
            "built-in validation rule documents must declare a supported version"
        );

        for rule in document.rules {
            let entry = ReferencedRule {
                genre: document.genre.clone(),
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
    }

    rules
}

#[cfg(test)]
mod tests {
    use crate::model::{CheckResult, DefinitionCounts, Issue, TraceSummary};

    use super::{all_rules, attach_referenced_rules, referenced_rules, rule_by_code};

    #[test]
    fn built_in_rule_catalog_loads_expected_entries() {
        let rules = all_rules();
        assert!(rules.iter().any(|rule| rule.code == "load-failed"));
        assert!(rules.iter().any(|rule| rule.code == "orphaned-definition"));
        assert!(
            rules
                .iter()
                .any(|rule| rule.code == "public-symbol-untracked")
        );
    }

    #[test]
    fn referenced_rules_follow_issue_codes_without_duplicates() {
        let issues = vec![
            Issue::error("duplicate-id", "subject", None, "message", None),
            Issue::warning("duplicate-id", "subject", None, "message", None),
            Issue::error("missing-reference", "subject", None, "message", None),
            Issue::warning("warn", "subject", None, "message", None),
        ];

        let rules = referenced_rules(&issues);
        assert_eq!(rules.len(), 2);
        assert_eq!(rules[0].code, "duplicate-id");
        assert_eq!(rules[1].code, "missing-reference");
    }

    #[test]
    fn attach_referenced_rules_enriches_check_results() {
        let result = CheckResult {
            workspace_root: ".".into(),
            definition_counts: DefinitionCounts::default(),
            trace_summary: TraceSummary::default(),
            issues: vec![Issue::error("load-failed", "workspace", None, "boom", None)],
            referenced_rules: Vec::new(),
        };

        let attached = attach_referenced_rules(result);
        assert_eq!(attached.referenced_rules.len(), 1);
        assert_eq!(attached.referenced_rules[0].code, "load-failed");
    }

    #[test]
    fn rule_lookup_returns_none_for_unknown_codes() {
        assert!(rule_by_code("warn").is_none());
    }
}
