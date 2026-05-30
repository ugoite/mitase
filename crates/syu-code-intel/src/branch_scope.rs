use serde::Serialize;
use syu_task_model::SearchResult;

use crate::{OwnershipStatus, confidence_for_branch_scope, flatten_symbols};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum BranchScopeConfidence {
    High,
    Medium,
    Low,
}

impl BranchScopeConfidence {
    pub const fn label(self) -> &'static str {
        match self {
            Self::High => "high",
            Self::Medium => "medium",
            Self::Low => "low",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AffectedSpecItem {
    pub kind: String,
    pub id: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document_path: Option<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub direct: bool,
}

fn is_false(value: &bool) -> bool {
    !*value
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ChangedSymbolReport {
    pub file: String,
    pub symbol: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub owners: Vec<AffectedSpecItem>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ChangedFileReport {
    pub file: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub symbols: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub owners: Vec<AffectedSpecItem>,
    pub status: OwnershipStatus,
    #[serde(default, skip_serializing_if = "is_false")]
    pub is_spec_file: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UnownedChange {
    pub file: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AmbiguousOwnership {
    pub file: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub owners: Vec<AffectedSpecItem>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OutOfScopeChange {
    pub file: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub allowed_ids: Vec<String>,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TraceOwnershipReport {
    pub changed_files_total: usize,
    pub owned_files: usize,
    pub partial_files: usize,
    pub unowned_files: usize,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub changed_symbols: Vec<ChangedSymbolReport>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub unowned_changes: Vec<UnownedChange>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub ambiguous_ownership: Vec<AmbiguousOwnership>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SpecImpactReport {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub affected_items: Vec<AffectedSpecItem>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub unowned_changes: Vec<UnownedChange>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub ambiguous_ownership: Vec<AmbiguousOwnership>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub out_of_scope_changes: Vec<OutOfScopeChange>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TestInventoryReport {
    pub total_tests: usize,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub required_tests: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub linked_tests: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SuggestedGoalSplit {
    pub confidence: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub include: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub exclude: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RepoRiskSummary {
    pub level: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BranchScopeEvidence {
    pub range: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub changed_files: Vec<ChangedFileReport>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub trace_ownership: Vec<ChangedFileReport>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub spec_items: Vec<AffectedSpecItem>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub required_tests: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub linked_tests: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub include_patterns: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub exclude_patterns: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub allowed_ids: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub unowned_files: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub ambiguous_files: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub spec_files: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub out_of_scope_changes: Vec<OutOfScopeChange>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub direct_items: Vec<SearchResult>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub related_items: Vec<SearchResult>,
    pub has_planned_features: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BranchScopeReport {
    pub range: String,
    pub confidence: BranchScopeConfidence,
    pub changed_files: Vec<ChangedFileReport>,
    pub changed_symbols: Vec<ChangedSymbolReport>,
    pub trace_ownership: TraceOwnershipReport,
    pub spec_impact: SpecImpactReport,
    pub test_inventory: TestInventoryReport,
    pub suggested_goal_split: SuggestedGoalSplit,
    pub repo_risk: RepoRiskSummary,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
}

impl BranchScopeReport {
    pub fn from_evidence(evidence: BranchScopeEvidence) -> Self {
        let changed_files = if evidence.changed_files.is_empty() {
            evidence.trace_ownership.clone()
        } else {
            evidence.changed_files.clone()
        };

        let mut changed_symbols = Vec::new();
        for file in &changed_files {
            changed_symbols.extend(flatten_symbols(&file.file, &file.symbols));
        }

        let confidence = confidence_for_branch_scope(
            &changed_files
                .iter()
                .map(|file| file.file.clone())
                .collect::<Vec<_>>(),
            &evidence.unowned_files,
            &evidence.ambiguous_files,
            &evidence.spec_files,
            evidence.has_planned_features,
        );

        let mut affected_items = evidence.spec_items.clone();
        if affected_items.is_empty() {
            affected_items.extend(evidence.direct_items.iter().map(|item| AffectedSpecItem {
                kind: item.kind.clone(),
                id: item.id.clone(),
                title: item.title.clone(),
                document_path: None,
                direct: true,
            }));
        }

        let trace_ownership = TraceOwnershipReport {
            changed_files_total: changed_files.len(),
            owned_files: changed_files
                .iter()
                .filter(|file| file.status == OwnershipStatus::Owned)
                .count(),
            partial_files: changed_files
                .iter()
                .filter(|file| file.status == OwnershipStatus::Partial)
                .count(),
            unowned_files: changed_files
                .iter()
                .filter(|file| file.status == OwnershipStatus::Unowned)
                .count(),
            changed_symbols: changed_symbols.clone(),
            unowned_changes: evidence
                .unowned_files
                .iter()
                .map(|file| UnownedChange {
                    file: file.clone(),
                    reason: "no trace ownership was found".to_string(),
                })
                .collect(),
            ambiguous_ownership: evidence
                .ambiguous_files
                .iter()
                .map(|file| AmbiguousOwnership {
                    file: file.clone(),
                    owners: Vec::new(),
                })
                .collect(),
        };

        let spec_impact = SpecImpactReport {
            affected_items,
            unowned_changes: trace_ownership.unowned_changes.clone(),
            ambiguous_ownership: trace_ownership.ambiguous_ownership.clone(),
            out_of_scope_changes: evidence.out_of_scope_changes.clone(),
        };

        let test_inventory = TestInventoryReport {
            total_tests: evidence.required_tests.len() + evidence.linked_tests.len(),
            required_tests: evidence.required_tests.clone(),
            linked_tests: evidence.linked_tests.clone(),
        };

        let suggested_goal_split = SuggestedGoalSplit {
            confidence: confidence.label().to_string(),
            include: evidence.include_patterns.clone(),
            exclude: evidence.exclude_patterns.clone(),
            reasons: build_split_reasons(
                confidence,
                &trace_ownership,
                &spec_impact,
                &test_inventory,
            ),
        };

        let repo_risk = RepoRiskSummary {
            level: confidence.label().to_string(),
            reasons: build_repo_risk_reasons(confidence, &trace_ownership, &spec_impact),
        };
        let warnings = build_warnings(confidence, &trace_ownership, &spec_impact);

        Self {
            range: evidence.range,
            confidence,
            changed_files,
            changed_symbols,
            trace_ownership,
            spec_impact,
            test_inventory,
            suggested_goal_split,
            repo_risk,
            warnings,
        }
    }
}

fn build_split_reasons(
    confidence: BranchScopeConfidence,
    trace_ownership: &TraceOwnershipReport,
    spec_impact: &SpecImpactReport,
    test_inventory: &TestInventoryReport,
) -> Vec<String> {
    let mut reasons = Vec::new();
    if confidence == BranchScopeConfidence::Low {
        reasons.push("branch scope remains weakly owned".to_string());
    }
    if trace_ownership.unowned_files > 0 {
        reasons.push("unowned files should be isolated before assignment".to_string());
    }
    if !spec_impact.ambiguous_ownership.is_empty() {
        reasons.push("ambiguous ownership makes a split safer".to_string());
    }
    if test_inventory.total_tests == 0 {
        reasons.push("no test inventory was linked to the diff".to_string());
    }
    reasons
}

fn build_repo_risk_reasons(
    confidence: BranchScopeConfidence,
    trace_ownership: &TraceOwnershipReport,
    spec_impact: &SpecImpactReport,
) -> Vec<String> {
    let mut reasons = Vec::new();
    if confidence == BranchScopeConfidence::Low {
        reasons.push("low confidence branch scope".to_string());
    }
    if trace_ownership.unowned_files > 0 {
        reasons.push("unowned files reduce assignment confidence".to_string());
    }
    if !spec_impact.out_of_scope_changes.is_empty() {
        reasons.push("scope guard violations are present".to_string());
    }
    reasons
}

fn build_warnings(
    confidence: BranchScopeConfidence,
    trace_ownership: &TraceOwnershipReport,
    spec_impact: &SpecImpactReport,
) -> Vec<String> {
    let mut warnings = Vec::new();
    if confidence == BranchScopeConfidence::Low {
        if trace_ownership.unowned_files > 0 {
            warnings.push(format!(
                "Low confidence: no trace ownership was found for {}.",
                trace_ownership
                    .unowned_changes
                    .iter()
                    .map(|item| item.file.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        if !spec_impact.ambiguous_ownership.is_empty() {
            warnings.push(format!(
                "Low confidence: ambiguous ownership remains for {}.",
                spec_impact
                    .ambiguous_ownership
                    .iter()
                    .map(|item| item.file.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
    }
    warnings
}

#[cfg(test)]
mod tests {
    use super::{
        AffectedSpecItem, BranchScopeConfidence, BranchScopeEvidence, BranchScopeReport,
        ChangedFileReport,
    };
    use crate::OwnershipStatus;

    #[test]
    fn spec_files_do_not_become_out_of_scope_without_explicit_violations() {
        let report = BranchScopeReport::from_evidence(BranchScopeEvidence {
            range: "HEAD~1..HEAD".to_string(),
            changed_files: vec![ChangedFileReport {
                file: "docs/syu/spec.md".to_string(),
                symbols: Vec::new(),
                owners: Vec::new(),
                status: OwnershipStatus::Owned,
                is_spec_file: true,
            }],
            trace_ownership: Vec::new(),
            spec_items: vec![AffectedSpecItem {
                kind: "spec".to_string(),
                id: "spec-1".to_string(),
                title: "Spec".to_string(),
                document_path: Some("docs/syu/spec.md".to_string()),
                direct: true,
            }],
            required_tests: Vec::new(),
            linked_tests: Vec::new(),
            include_patterns: Vec::new(),
            exclude_patterns: Vec::new(),
            allowed_ids: Vec::new(),
            unowned_files: Vec::new(),
            ambiguous_files: Vec::new(),
            spec_files: vec!["docs/syu/spec.md".to_string()],
            out_of_scope_changes: Vec::new(),
            direct_items: Vec::new(),
            related_items: Vec::new(),
            has_planned_features: false,
        });

        assert_eq!(report.confidence, BranchScopeConfidence::Medium);
        assert!(report.spec_impact.out_of_scope_changes.is_empty());
    }
}
