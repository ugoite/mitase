// FEAT-AUDIT-001
// REQ-CORE-025

use anyhow::Result;
use serde::Serialize;
use std::collections::{BTreeSet, HashSet};

use crate::{
    cli::{AuditArgs, OutputFormat},
    model::{Feature, Philosophy, Policy, Requirement},
    workspace::{Workspace, load_workspace},
};

const OVERLAP_THRESHOLD: f32 = 0.45;
const STOP_WORDS: &[&str] = &[
    "a", "an", "and", "as", "be", "by", "for", "from", "in", "into", "is", "it", "must", "of",
    "on", "or", "should", "that", "the", "to", "when", "with",
];
const OPPOSITE_TERMS: &[(&str, &str)] = &[
    ("manual", "automatic"),
    ("manual", "automated"),
    ("always", "never"),
    ("required", "optional"),
    ("strict", "lightweight"),
    ("interactive", "noninteractive"),
    ("coupled", "decoupled"),
    ("opaque", "explainable"),
];

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
enum AuditFindingKind {
    Overlap,
    Tension,
    OrphanedPolicy,
}

#[derive(Debug, Clone, Serialize)]
struct AuditFinding {
    kind: AuditFindingKind,
    summary: String,
    details: String,
    related_ids: Vec<String>,
    shared_terms: Vec<String>,
    score: f32,
}

#[derive(Debug, Clone, Serialize)]
struct AuditSummary {
    overlap_candidates: usize,
    tension_candidates: usize,
    orphaned_policies: usize,
}

#[derive(Debug, Clone, Serialize)]
struct JsonAuditOutput {
    workspace: String,
    summary: AuditSummary,
    findings: Vec<AuditFinding>,
}

pub fn run_audit_command(args: &AuditArgs) -> Result<i32> {
    let workspace = load_workspace(&args.workspace)?;
    let findings = collect_findings(&workspace);
    match args.format {
        OutputFormat::Text => print_text_results(&workspace.root.display().to_string(), &findings),
        OutputFormat::Json => println!(
            "{}",
            serde_json::to_string_pretty(&JsonAuditOutput {
                workspace: workspace.root.display().to_string(),
                summary: summarize_findings(&findings),
                findings,
            })
            .expect("serializing audit output should succeed")
        ),
    }
    Ok(0)
}

fn collect_findings(workspace: &Workspace) -> Vec<AuditFinding> {
    let mut findings = overlap_findings(&workspace.requirements);
    findings.extend(tension_findings(workspace));
    findings.extend(orphaned_policy_findings(
        &workspace.policies,
        &workspace.requirements,
    ));
    findings
}

fn summarize_findings(findings: &[AuditFinding]) -> AuditSummary {
    AuditSummary {
        overlap_candidates: findings
            .iter()
            .filter(|finding| matches!(finding.kind, AuditFindingKind::Overlap))
            .count(),
        tension_candidates: findings
            .iter()
            .filter(|finding| matches!(finding.kind, AuditFindingKind::Tension))
            .count(),
        orphaned_policies: findings
            .iter()
            .filter(|finding| matches!(finding.kind, AuditFindingKind::OrphanedPolicy))
            .count(),
    }
}

fn print_text_results(workspace: &str, findings: &[AuditFinding]) {
    print!("{}", render_text_results(workspace, findings));
}

fn render_text_results(workspace: &str, findings: &[AuditFinding]) -> String {
    let summary = summarize_findings(findings);
    let mut output = format!("audit workspace: {workspace}\n");
    output.push_str(&format!(
        "summary: {} overlap candidate(s), {} tension candidate(s), {} orphaned policy candidate(s)",
        summary.overlap_candidates, summary.tension_candidates, summary.orphaned_policies
    ));
    if findings.is_empty() {
        output.push('\n');
        output.push_str("no audit findings\n");
        return output;
    }
    for finding in findings {
        let kind = match finding.kind {
            AuditFindingKind::Overlap => "overlap",
            AuditFindingKind::Tension => "tension",
            AuditFindingKind::OrphanedPolicy => "orphaned-policy",
        };
        output.push_str("\n\n");
        output.push_str(&format!("[{kind}] {}\n", finding.summary));
        output.push_str(&format!("  ids: {}\n", finding.related_ids.join(", ")));
        if !finding.shared_terms.is_empty() {
            output.push_str(&format!(
                "  shared terms: {}\n",
                finding.shared_terms.join(", ")
            ));
        }
        if finding.score > 0.0 {
            output.push_str(&format!("  score: {:.2}\n", finding.score));
        }
        output.push_str(&format!("  {}\n", finding.details));
    }
    output
}

fn overlap_findings(requirements: &[Requirement]) -> Vec<AuditFinding> {
    let mut findings = Vec::new();
    for (index, left) in requirements.iter().enumerate() {
        let left_terms = requirement_terms(left);
        for right in requirements.iter().skip(index + 1) {
            let right_terms = requirement_terms(right);
            let shared_terms = sorted_terms(left_terms.intersection(&right_terms));
            let score = jaccard_score(&left_terms, &right_terms);
            if shared_terms.len() < 3 || score < OVERLAP_THRESHOLD {
                continue;
            }
            findings.push(AuditFinding {
                kind: AuditFindingKind::Overlap,
                summary: format!(
                    "{} and {} may describe overlapping obligations",
                    left.id, right.id
                ),
                details: "Both requirements reuse a similar set of terms and may be worth reviewing together before they drift independently.".to_string(),
                related_ids: vec![left.id.clone(), right.id.clone()],
                shared_terms,
                score,
            });
        }
    }
    findings
}

fn tension_findings(workspace: &Workspace) -> Vec<AuditFinding> {
    let mut findings = Vec::new();
    for feature in &workspace.features {
        let feature_terms = feature_terms(feature);
        if feature_terms.is_empty() {
            continue;
        }
        for requirement in workspace
            .requirements
            .iter()
            .filter(|requirement| feature.linked_requirements.contains(&requirement.id))
        {
            let requirement_terms = requirement_terms(requirement);
            for policy in workspace
                .policies
                .iter()
                .filter(|policy| requirement.linked_policies.contains(&policy.id))
            {
                let policy_terms = policy_terms(policy);
                if let Some((feature_term, policy_term)) =
                    first_opposing_term_pair(&feature_terms, &policy_terms)
                {
                    if linked_requirement_explicitly_scopes_term_pair(
                        &requirement_terms,
                        feature_term,
                        policy_term,
                    ) {
                        continue;
                    }
                    let mut related_ids = vec![
                        feature.id.clone(),
                        requirement.id.clone(),
                        policy.id.clone(),
                    ];
                    let philosophies = linked_philosophies(workspace, policy);
                    related_ids.extend(philosophies.iter().map(|item| item.id.clone()));
                    findings.push(AuditFinding {
                        kind: AuditFindingKind::Tension,
                        summary: format!(
                            "{} may pull against {} through {}",
                            feature.id, policy.id, requirement.id
                        ),
                        details: format!(
                            "Feature wording uses `{feature_term}` while the linked policy language emphasizes `{policy_term}`. Review whether the feature intent still supports the upstream rule."
                        ),
                        related_ids,
                        shared_terms: vec![feature_term.to_string(), policy_term.to_string()],
                        score: 1.0,
                    });
                }
            }
        }
    }
    findings
}

fn linked_requirement_explicitly_scopes_term_pair(
    requirement_terms: &HashSet<String>,
    left_term: &str,
    right_term: &str,
) -> bool {
    requirement_terms.contains(left_term) && requirement_terms.contains(right_term)
}

fn orphaned_policy_findings(
    policies: &[Policy],
    requirements: &[Requirement],
) -> Vec<AuditFinding> {
    policies
        .iter()
        .filter_map(|policy| {
            let downstream = requirements
                .iter()
                .filter(|requirement| requirement.linked_policies.contains(&policy.id))
                .count();
            if downstream > 0 {
                return None;
            }
            Some(AuditFinding {
                kind: AuditFindingKind::OrphanedPolicy,
                summary: format!("{} has no concrete downstream requirements", policy.id),
                details: "The policy text is present, but no checked-in requirement currently turns it into an actionable obligation.".to_string(),
                related_ids: vec![policy.id.clone()],
                shared_terms: Vec::new(),
                score: 0.0,
            })
        })
        .collect()
}

fn requirement_terms(requirement: &Requirement) -> HashSet<String> {
    tokenize(&format!(
        "{} {}",
        requirement.title, requirement.description
    ))
}

fn feature_terms(feature: &Feature) -> HashSet<String> {
    tokenize(&format!("{} {}", feature.title, feature.summary))
}

fn policy_terms(policy: &Policy) -> HashSet<String> {
    tokenize(&format!(
        "{} {} {}",
        policy.title, policy.summary, policy.description
    ))
}

fn linked_philosophies<'a>(workspace: &'a Workspace, policy: &Policy) -> Vec<&'a Philosophy> {
    workspace
        .philosophies
        .iter()
        .filter(|item| policy.linked_philosophies.contains(&item.id))
        .collect()
}

fn tokenize(text: &str) -> HashSet<String> {
    text.split(|ch: char| !ch.is_ascii_alphanumeric())
        .map(|token| token.trim().to_lowercase())
        .filter(|token| token.len() >= 4)
        .filter(|token| !STOP_WORDS.contains(&token.as_str()))
        .collect()
}

fn sorted_terms<'a>(terms: impl Iterator<Item = &'a String>) -> Vec<String> {
    let mut unique = BTreeSet::new();
    for term in terms {
        unique.insert(term.clone());
    }
    unique.into_iter().collect()
}

fn jaccard_score(left: &HashSet<String>, right: &HashSet<String>) -> f32 {
    let union = left.union(right).count();
    if union == 0 {
        return 0.0;
    }
    left.intersection(right).count() as f32 / union as f32
}

fn first_opposing_term_pair<'a>(
    left: &'a HashSet<String>,
    right: &'a HashSet<String>,
) -> Option<(&'a str, &'a str)> {
    OPPOSITE_TERMS.iter().find_map(|(left_term, right_term)| {
        if left.contains(*left_term) && right.contains(*right_term) {
            Some((*left_term, *right_term))
        } else if left.contains(*right_term) && right.contains(*left_term) {
            Some((*right_term, *left_term))
        } else {
            None
        }
    })
}

#[cfg(test)]
mod tests {
    use super::{
        AuditFindingKind, first_opposing_term_pair, jaccard_score,
        linked_requirement_explicitly_scopes_term_pair, orphaned_policy_findings,
        render_text_results, tension_findings, tokenize,
    };
    use crate::{
        model::{Feature, Philosophy, Policy, Requirement},
        workspace::Workspace,
    };
    use std::{
        collections::{BTreeMap, HashSet},
        path::PathBuf,
    };

    #[test]
    fn tokenize_drops_short_and_common_terms() {
        let tokens = tokenize("The terminal should stay explainable and low ceremony.");
        assert!(tokens.contains("terminal"));
        assert!(tokens.contains("explainable"));
        assert!(!tokens.contains("the"));
        assert!(!tokens.contains("and"));
    }

    #[test]
    fn jaccard_score_returns_zero_for_empty_sets() {
        assert_eq!(jaccard_score(&HashSet::new(), &HashSet::new()), 0.0);
    }

    #[test]
    fn detects_opposing_term_pairs() {
        let left = HashSet::from(["manual".to_string(), "workflow".to_string()]);
        let right = HashSet::from(["automatic".to_string(), "checks".to_string()]);
        assert_eq!(
            first_opposing_term_pair(&left, &right),
            Some(("manual", "automatic"))
        );
    }

    #[test]
    fn detects_reversed_opposing_term_pairs() {
        let left = HashSet::from(["automatic".to_string(), "checks".to_string()]);
        let right = HashSet::from(["manual".to_string(), "workflow".to_string()]);
        assert_eq!(
            first_opposing_term_pair(&left, &right),
            Some(("automatic", "manual"))
        );
    }

    #[test]
    fn linked_requirement_terms_can_suppress_known_surface_pairs() {
        let requirement_terms = HashSet::from([
            "browser".to_string(),
            "terminal".to_string(),
            "workflow".to_string(),
        ]);
        assert!(linked_requirement_explicitly_scopes_term_pair(
            &requirement_terms,
            "browser",
            "terminal",
        ));
        assert!(!linked_requirement_explicitly_scopes_term_pair(
            &requirement_terms,
            "manual",
            "automatic",
        ));
    }

    #[test]
    fn flags_policies_without_downstream_requirements() {
        let findings = orphaned_policy_findings(
            &[Policy {
                id: "POL-001".to_string(),
                title: "Keep it explained".to_string(),
                summary: "summary".to_string(),
                description: "description".to_string(),
                linked_philosophies: vec![],
                linked_requirements: vec![],
            }],
            &[Requirement {
                id: "REQ-001".to_string(),
                title: "Req".to_string(),
                description: "description".to_string(),
                priority: "medium".to_string(),
                status: "implemented".to_string(),
                linked_policies: vec!["POL-002".to_string()],
                linked_features: vec![],
                tests: BTreeMap::new(),
            }],
        );

        assert_eq!(findings.len(), 1);
        assert!(matches!(findings[0].kind, AuditFindingKind::OrphanedPolicy));
    }

    #[test]
    fn render_text_results_reports_empty_audits() {
        let rendered = render_text_results("workspace", &[]);
        assert!(rendered.contains("summary: 0 overlap candidate(s), 0 tension candidate(s), 0 orphaned policy candidate(s)"));
        assert!(rendered.contains("no audit findings"));
    }

    #[test]
    fn skips_tension_checks_for_features_without_searchable_terms() {
        let workspace = Workspace {
            root: PathBuf::from("."),
            spec_root: PathBuf::from("docs/syu"),
            config: crate::config::SyuConfig::default(),
            philosophies: vec![Philosophy {
                id: "PHIL-001".to_string(),
                title: "Explainable".to_string(),
                product_design_principle: "Explainable".to_string(),
                coding_guideline: "Explicit".to_string(),
                linked_policies: vec!["POL-001".to_string()],
            }],
            policies: vec![Policy {
                id: "POL-001".to_string(),
                title: "Prefer automatic checks".to_string(),
                summary: "Automatic checks".to_string(),
                description: "Automatic checks".to_string(),
                linked_philosophies: vec!["PHIL-001".to_string()],
                linked_requirements: vec!["REQ-001".to_string()],
            }],
            requirements: vec![Requirement {
                id: "REQ-001".to_string(),
                title: "Keep review flow".to_string(),
                description: "Keep review flow explainable.".to_string(),
                priority: "medium".to_string(),
                status: "implemented".to_string(),
                linked_policies: vec!["POL-001".to_string()],
                linked_features: vec!["FEAT-001".to_string()],
                tests: BTreeMap::new(),
            }],
            features: vec![Feature {
                id: "FEAT-001".to_string(),
                title: "and the".to_string(),
                summary: "to of in".to_string(),
                status: "implemented".to_string(),
                linked_requirements: vec!["REQ-001".to_string()],
                implementations: BTreeMap::new(),
            }],
        };

        assert!(tension_findings(&workspace).is_empty());
    }

    #[test]
    fn skips_tension_when_requirement_already_names_both_surface_terms() {
        let workspace = Workspace {
            root: PathBuf::from("."),
            spec_root: PathBuf::from("docs/syu"),
            config: crate::config::SyuConfig::default(),
            philosophies: vec![Philosophy {
                id: "PHIL-001".to_string(),
                title: "Keep automation choices explicit".to_string(),
                product_design_principle:
                    "Manual and automatic workflows should stay easy to compare."
                        .to_string(),
                coding_guideline: "Prefer explicit workflow trade-offs.".to_string(),
                linked_policies: vec!["POL-001".to_string()],
            }],
            policies: vec![Policy {
                id: "POL-001".to_string(),
                title: "Prefer automatic checks".to_string(),
                summary: "Automatic checks should stay trustworthy.".to_string(),
                description: "Automatic checks should remain the default repository path."
                    .to_string(),
                linked_philosophies: vec!["PHIL-001".to_string()],
                linked_requirements: vec!["REQ-001".to_string()],
            }],
            requirements: vec![Requirement {
                id: "REQ-001".to_string(),
                title: "Clarify manual and automatic review workflows".to_string(),
                description:
                    "Reviewers should be able to compare manual and automatic review workflows without losing context."
                        .to_string(),
                priority: "medium".to_string(),
                status: "implemented".to_string(),
                linked_policies: vec!["POL-001".to_string()],
                linked_features: vec!["FEAT-001".to_string()],
                tests: BTreeMap::new(),
            }],
            features: vec![Feature {
                id: "FEAT-001".to_string(),
                title: "Manual review overview".to_string(),
                summary: "Manual review overview keeps linked context visible.".to_string(),
                status: "implemented".to_string(),
                linked_requirements: vec!["REQ-001".to_string()],
                implementations: BTreeMap::new(),
            }],
        };

        assert!(tension_findings(&workspace).is_empty());
    }
}
