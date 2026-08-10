use serde::Serialize;
use mitase_task_model::SearchResult;

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
pub struct SpecImpactGraphNode {
    pub id: String,
    pub label: String,
    pub kind: String,
    pub state: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SpecImpactGraphEdge {
    pub from: String,
    pub to: String,
    pub state: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SpecImpactGraphReport {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub nodes: Vec<SpecImpactGraphNode>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub edges: Vec<SpecImpactGraphEdge>,
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
    pub spec_impact_graph: SpecImpactGraphReport,
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
        let spec_impact_graph =
            build_spec_impact_graph(&changed_files, &spec_impact, &test_inventory);

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
            spec_impact_graph,
            test_inventory,
            suggested_goal_split,
            repo_risk,
            warnings,
        }
    }
}

fn build_spec_impact_graph(
    changed_files: &[ChangedFileReport],
    spec_impact: &SpecImpactReport,
    test_inventory: &TestInventoryReport,
) -> SpecImpactGraphReport {
    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    let mut previous_spec_id: Option<String> = None;

    for item in &spec_impact.affected_items {
        let state = match item.kind.as_str() {
            "philosophy" => "spec-linked",
            "policy" => "spec-linked",
            "requirement" => "scope-in",
            "feature" => "scope-in",
            _ => "spec-linked",
        };
        nodes.push(SpecImpactGraphNode {
            id: item.id.clone(),
            label: item.title.clone(),
            kind: item.kind.clone(),
            state: state.to_string(),
        });
        if let Some(previous) = &previous_spec_id {
            edges.push(SpecImpactGraphEdge {
                from: previous.clone(),
                to: item.id.clone(),
                state: "spec-linked".to_string(),
            });
        }
        previous_spec_id = Some(item.id.clone());
    }

    for file in changed_files {
        let state = match file.status {
            OwnershipStatus::Owned => "ownership-known",
            OwnershipStatus::Partial => "ownership-ambiguous",
            OwnershipStatus::Unowned => "ownership-missing",
        };
        nodes.push(SpecImpactGraphNode {
            id: file.file.clone(),
            label: file.file.clone(),
            kind: if file.symbols.is_empty() {
                "file".to_string()
            } else {
                "file/symbol".to_string()
            },
            state: state.to_string(),
        });
        if let Some(owner) = file
            .owners
            .first()
            .or_else(|| spec_impact.affected_items.first())
        {
            edges.push(SpecImpactGraphEdge {
                from: owner.id.clone(),
                to: file.file.clone(),
                state: "code-linked".to_string(),
            });
        }
    }

    for test in test_inventory
        .required_tests
        .iter()
        .chain(test_inventory.linked_tests.iter())
    {
        nodes.push(SpecImpactGraphNode {
            id: test.clone(),
            label: test.clone(),
            kind: "test".to_string(),
            state: "test-linked".to_string(),
        });
        if let Some(file) = changed_files.first() {
            edges.push(SpecImpactGraphEdge {
                from: file.file.clone(),
                to: test.clone(),
                state: "test-linked".to_string(),
            });
        }
    }

    SpecImpactGraphReport { nodes, edges }
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
                file: "docs/mitase/spec.md".to_string(),
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
                document_path: Some("docs/mitase/spec.md".to_string()),
                direct: true,
            }],
            required_tests: Vec::new(),
            linked_tests: Vec::new(),
            include_patterns: Vec::new(),
            exclude_patterns: Vec::new(),
            allowed_ids: Vec::new(),
            unowned_files: Vec::new(),
            ambiguous_files: Vec::new(),
            spec_files: vec!["docs/mitase/spec.md".to_string()],
            out_of_scope_changes: Vec::new(),
            direct_items: Vec::new(),
            related_items: Vec::new(),
            has_planned_features: false,
        });

        assert_eq!(report.confidence, BranchScopeConfidence::Medium);
        assert!(report.spec_impact.out_of_scope_changes.is_empty());
    }

    #[test]
    fn branch_scope_report_includes_typed_graph_nodes_and_edges() {
        let report = BranchScopeReport::from_evidence(BranchScopeEvidence {
            range: "main..HEAD".to_string(),
            changed_files: vec![ChangedFileReport {
                file: "src/workbench.rs".to_string(),
                symbols: vec!["SpecImpactGraph".to_string()],
                owners: vec![AffectedSpecItem {
                    kind: "feature".to_string(),
                    id: "FEAT-WORKBENCH-SPEC-GRAPH-001".to_string(),
                    title: "Spec Impact Graph".to_string(),
                    document_path: None,
                    direct: true,
                }],
                status: OwnershipStatus::Owned,
                is_spec_file: false,
            }],
            trace_ownership: Vec::new(),
            spec_items: vec![
                AffectedSpecItem {
                    kind: "requirement".to_string(),
                    id: "REQ-WORKBENCH-004".to_string(),
                    title: "Spec impact and branch scope visualization".to_string(),
                    document_path: None,
                    direct: true,
                },
                AffectedSpecItem {
                    kind: "feature".to_string(),
                    id: "FEAT-WORKBENCH-SPEC-GRAPH-001".to_string(),
                    title: "Spec Impact Graph".to_string(),
                    document_path: None,
                    direct: true,
                },
            ],
            required_tests: vec!["tests/workbench_smoke.rs".to_string()],
            linked_tests: Vec::new(),
            include_patterns: vec!["crates/mitase-app-ui/src/**".to_string()],
            exclude_patterns: Vec::new(),
            allowed_ids: Vec::new(),
            unowned_files: Vec::new(),
            ambiguous_files: Vec::new(),
            spec_files: Vec::new(),
            out_of_scope_changes: Vec::new(),
            direct_items: Vec::new(),
            related_items: Vec::new(),
            has_planned_features: true,
        });

        assert!(report.spec_impact_graph.nodes.iter().any(|node| {
            node.id == "FEAT-WORKBENCH-SPEC-GRAPH-001" && node.state == "scope-in"
        }));
        assert!(
            report
                .spec_impact_graph
                .nodes
                .iter()
                .any(|node| { node.id == "src/workbench.rs" && node.state == "ownership-known" })
        );
        assert!(report.spec_impact_graph.edges.iter().any(|edge| {
            edge.from == "REQ-WORKBENCH-004"
                && edge.to == "FEAT-WORKBENCH-SPEC-GRAPH-001"
                && edge.state == "spec-linked"
        }));
    }
}
