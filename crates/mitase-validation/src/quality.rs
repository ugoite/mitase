//! Read-only specification-quality checks (Q001/Q005).
//!
//! This module implements the deterministic subset of the quality-diagnostic
//! contract described in `docs/understand/quality/spec-quality-diagnostics.md`:
//!
//! - Q001 Layer Echo: a Criterion repeats the normative statement of a Policy
//!   rule it is directly governed by.
//! - Q005 Redundant Rule Description: a Policy description repeats one of its
//!   own rule statements.
//!
//! Every function here is pure and read-only: checks inspect the already built
//! [`SpecIndex`] and [`SpecWorkspace`], perform no I/O, and offer no automatic
//! fix. Findings are Warning (Q001) or Info (Q005) prompts for the author, not
//! proof that the specified meaning is wrong.

use mitase_diagnostics::{
    Diagnostic, DiagnosticSubject, Evidence, Location, ReadOnlyHint, RelatedLocation, RelationRef,
};
use mitase_spec_model::{ItemStatus, LocalAnchorKind, SemanticDocument, SpecAnchor};
use mitase_workspace::{SpecIndex, SpecWorkspace};
use std::collections::{BTreeMap, BTreeSet};
use unicode_normalization::UnicodeNormalization;

/// Minimum length, in Unicode scalar values, of a normalized statement that
/// may raise Q001 or Q005.
///
/// Rationale: below roughly twenty characters, generic fragments such as
/// "must be secure" or "安全であること" collide across unrelated items by
/// accident. The deterministic echo rules must stay silent there; the boundary
/// is pinned by unit tests (`short_shared_fragments_stay_silent`).
pub(crate) const MIN_NORMALIZED_STATEMENT_LEN: usize = 20;

/// Presentational punctuation ignored by statement comparison.
///
/// The set is deliberately narrow: sentence-level punctuation in English and
/// Japanese that never carries normative meaning. Modality (`must`/`should`),
/// negation words, comparison operators, digits, target names, hyphens, and
/// symbols such as `!*-+/ <>=&|%$#@_` are preserved exactly, so statements
/// that differ only in those positions never compare equal.
const STRIPPED_PUNCTUATION: &[char] = &[
    '.', ',', ';', ':', '?', '\'', '"', '‘', '’', '“', '”', '«', '»', '‹', '›', '(', ')', '[', ']',
    '{', '}', '、', '。', '，', '．', '「', '」', '『', '』', '【', '】', '〈', '〉', '《', '》',
    '…', '—', '–', '·', '•', '`', '~',
];

/// Normalize a normative statement for deterministic comparison.
///
/// The pipeline is Unicode compatibility normalization (NFKC, so full-width
/// alphanumerics and the ideographic space compare equal to their ASCII
/// counterparts), Unicode case folding, removal of [`STRIPPED_PUNCTUATION`],
/// and collapsing of all whitespace runs to single ASCII spaces. The function
/// is total and deterministic in Japanese and English.
pub(crate) fn normalize_statement(value: &str) -> String {
    let folded: String = value.nfkc().flat_map(|c| c.to_lowercase()).collect();
    let stripped: String = folded
        .chars()
        .filter(|c| !STRIPPED_PUNCTUATION.contains(c))
        .collect();
    stripped.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Length of a normalized statement in Unicode scalar values.
pub(crate) fn normalized_len(normalized: &str) -> usize {
    normalized.chars().count()
}

/// True when two statements are deterministically the same normative text:
/// equal after [`normalize_statement`] and long enough to rule out generic
/// fragments. Fuzzy similarity never counts here.
pub(crate) fn statements_match(first: &str, second: &str) -> bool {
    let normalized = normalize_statement(first);
    normalized_len(&normalized) >= MIN_NORMALIZED_STATEMENT_LEN
        && normalized == normalize_statement(second)
}

fn item_path(index: &SpecIndex, anchor: &SpecAnchor) -> String {
    index
        .item_paths
        .get(&anchor.item)
        .map(|path| path.to_string_lossy().into_owned())
        .unwrap_or_default()
}

fn anchor_subject(anchor: &SpecAnchor) -> DiagnosticSubject {
    DiagnosticSubject {
        kind: "spec-anchor".into(),
        value: anchor.to_string(),
    }
}

/// Run the deterministic quality checks and push findings onto `out`.
///
/// Callers run this after graph validation, when every anchor referenced here
/// has already been resolved. Pairs whose governing anchor is missing or has
/// an unexpected kind are skipped: the structural diagnostics own that
/// finding and quality must not duplicate it.
pub(crate) fn validate_quality(
    workspace: &SpecWorkspace,
    index: &SpecIndex,
    out: &mut Vec<Diagnostic>,
) {
    validate_layer_echo(index, out);
    validate_redundant_rule_description(workspace, out);
    validate_fragmented_requirement(workspace, index, out);
    validate_duplicate_obligation(index, out);
}

/// Q001 Layer Echo: a Criterion directly governed by a Policy rule states the
/// same normative text as that rule.
fn validate_layer_echo(index: &SpecIndex, out: &mut Vec<Diagnostic>) {
    let mut reported = std::collections::BTreeSet::new();
    for (anchor, value) in &index.anchors {
        let mitase_workspace::AnchorValue::Criterion(criterion) = value else {
            continue;
        };
        for governor in &criterion.governed_by {
            if governor.kind != LocalAnchorKind::Rule {
                continue;
            }
            let Some(mitase_workspace::AnchorValue::Rule(rule)) = index.anchors.get(governor)
            else {
                continue;
            };
            if !reported.insert((anchor.to_string(), governor.to_string())) {
                continue;
            }
            if !statements_match(&criterion.statement, &rule.statement) {
                continue;
            }
            let normalized_length =
                normalized_len(&normalize_statement(&criterion.statement)).to_string();
            let mut diagnostic = Diagnostic::warning(
                "MITASE-QUALITY-001",
                format!(
                    "Policy {governor} and Criterion {anchor} express the same normative statement"
                ),
                item_path(index, anchor),
            );
            diagnostic.set_subject_anchor(anchor);
            diagnostic.reference = Some(anchor_subject(governor));
            diagnostic.relation = Some(RelationRef {
                relation: "governed-by".into(),
                source: anchor_subject(anchor),
                targets: vec![anchor_subject(governor)],
            });
            diagnostic.related.push(RelatedLocation {
                location: Location {
                    path: item_path(index, governor),
                    line: None,
                    column: None,
                    end_line: None,
                    end_column: None,
                    label: Some("governing rule states the same normative statement".into()),
                },
                message: format!("governing rule {governor} states the same normative statement"),
            });
            diagnostic.evidence = vec![
                Evidence {
                    kind: "normalized_statement_equal".into(),
                    value: "true".into(),
                },
                Evidence {
                    kind: "governed_by".into(),
                    value: "direct".into(),
                },
                Evidence {
                    kind: "normalized_length".into(),
                    value: normalized_length,
                },
            ];
            diagnostic.help = Some(
                "Keep the local acceptance condition in Criterion. Either generalize the Policy \
                 into a reusable decision rule, or remove/merge that Policy if it has no \
                 independent meaning."
                    .into(),
            );
            diagnostic.next.push(ReadOnlyHint {
                kind: "show".into(),
                value: anchor.item.to_string(),
            });
            out.push(diagnostic);
        }
    }
}

/// Q005 Redundant Rule Description: a non-empty Policy description repeats one
/// of its own rule statements. Policies without a description stay silent.
fn validate_redundant_rule_description(workspace: &SpecWorkspace, out: &mut Vec<Diagnostic>) {
    for loaded in &workspace.documents {
        let SemanticDocument::Policies { policies, .. } = &loaded.document else {
            continue;
        };
        let path = loaded.path.to_string_lossy().into_owned();
        for policy in policies {
            if policy.description.trim().is_empty() {
                continue;
            }
            for rule in &policy.rules {
                if !statements_match(&policy.description, &rule.statement) {
                    continue;
                }
                let rule_anchor = SpecAnchor {
                    item: policy.id.clone(),
                    kind: LocalAnchorKind::Rule,
                    local_id: rule.id.clone(),
                };
                let mut diagnostic = Diagnostic::info(
                    "MITASE-QUALITY-005",
                    format!(
                        "Policy {} description repeats rule {} statement",
                        policy.id, rule_anchor
                    ),
                    path.clone(),
                );
                diagnostic.set_subject_anchor(&rule_anchor);
                diagnostic.evidence = vec![
                    Evidence {
                        kind: "description_statement_equal".into(),
                        value: "true".into(),
                    },
                    Evidence {
                        kind: "normalized_length".into(),
                        value: normalized_len(&normalize_statement(&policy.description))
                            .to_string(),
                    },
                ];
                diagnostic.help = Some(
                    "Keep a description that explains the rule's intent, or remove the \
                     redundant description text."
                        .into(),
                );
                diagnostic.next.push(ReadOnlyHint {
                    kind: "show".into(),
                    value: policy.id.to_string(),
                });
                out.push(diagnostic);
            }
        }
    }
}

/// Minimum Criterion count for a Requirement to be a fragmentation candidate.
///
/// Rationale: below five Criteria, a Requirement is small enough to review by
/// eye; splitting pressure from component counting would be noise. Pinned by
/// the fragmented-fixture integration test.
pub(crate) const MIN_FRAGMENTED_CRITERIA: usize = 5;

/// Minimum connected-component count for a fragmentation finding.
///
/// Rationale: two implementation groups often reflect a natural primary plus
/// auxiliary split. Three or more groups signal scattered responsibilities.
pub(crate) const MIN_FRAGMENTED_COMPONENTS: usize = 3;

/// A fragmentation finding requires the largest component to hold at most
/// this share of the Requirement's Criteria, expressed as `largest * 5 <=
/// total * 2` (40%). A dominant group means the Requirement still has a
/// center; scattered small groups do not.
fn largest_component_within_limit(largest: usize, total: usize) -> bool {
    largest * 5 <= total * 2
}

/// Connected Criterion groups of one Requirement over direct `satisfies`
/// relations.
///
/// Criteria are joined when they share one owning item (usually a Feature)
/// through authored direct-satisfies bindings. Facets never split a group:
/// the union key is the owner item, not the binding or facet. `exposes`
/// relations are never consulted. Bindings that no longer exist are skipped;
/// the structural diagnostics own that finding.
pub(crate) fn criterion_components(
    index: &SpecIndex,
    criteria: &[SpecAnchor],
) -> Vec<Vec<SpecAnchor>> {
    fn find(parent: &mut [usize], mut node: usize) -> usize {
        while parent[node] != node {
            parent[node] = parent[parent[node]];
            node = parent[node];
        }
        node
    }
    let mut parent: Vec<usize> = (0..criteria.len()).collect();
    let mut owner_groups: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for (position, criterion) in criteria.iter().enumerate() {
        let mut owners = BTreeSet::new();
        if let Some(bindings) = index.criteria_to_implementations.get(criterion) {
            for binding in bindings {
                if index.bindings.contains_key(binding) {
                    owners.insert(binding.item.to_string());
                }
            }
        }
        for owner in owners {
            owner_groups.entry(owner).or_default().push(position);
        }
    }
    for positions in owner_groups.values() {
        for window in positions.windows(2) {
            let left = find(&mut parent, window[0]);
            let right = find(&mut parent, window[1]);
            if left != right {
                parent[left] = right;
            }
        }
    }
    let mut groups: BTreeMap<usize, Vec<SpecAnchor>> = BTreeMap::new();
    for (position, criterion) in criteria.iter().enumerate() {
        let root = find(&mut parent, position);
        groups.entry(root).or_default().push(criterion.clone());
    }
    let mut components: Vec<Vec<SpecAnchor>> = groups.into_values().collect();
    for component in &mut components {
        component.sort();
    }
    components.sort();
    components
}

/// Q003 Fragmented Requirement: one Requirement's Criteria split into many
/// small implementation groups with no dominant center.
///
/// All of the Requirement's Criteria must currently resolve to at least one
/// exact implementation target. Planned, absent, or unresolved relations
/// never count as independence: when relations are too sparse the existing
/// coverage diagnostics own the finding and Q003 stays silent.
fn validate_fragmented_requirement(
    workspace: &SpecWorkspace,
    index: &SpecIndex,
    out: &mut Vec<Diagnostic>,
) {
    for loaded in &workspace.documents {
        let SemanticDocument::Requirements { requirements, .. } = &loaded.document else {
            continue;
        };
        let path = loaded.path.to_string_lossy().into_owned();
        for requirement in requirements {
            if index.item_status.get(&requirement.id) != Some(&ItemStatus::Implemented) {
                continue;
            }
            let criteria: Vec<SpecAnchor> = requirement
                .criteria
                .iter()
                .map(|criterion| SpecAnchor {
                    item: requirement.id.clone(),
                    kind: LocalAnchorKind::Criterion,
                    local_id: criterion.id.clone(),
                })
                .collect();
            if criteria.len() < MIN_FRAGMENTED_CRITERIA {
                continue;
            }
            let resolved = criteria.iter().all(|criterion| {
                index
                    .criteria_to_implementation_targets
                    .get(criterion)
                    .is_some_and(|targets| !targets.is_empty())
            });
            if !resolved {
                continue;
            }
            let components = criterion_components(index, &criteria);
            if components.len() < MIN_FRAGMENTED_COMPONENTS {
                continue;
            }
            let largest = components.iter().map(Vec::len).max().unwrap_or(0);
            if !largest_component_within_limit(largest, criteria.len()) {
                continue;
            }
            let mut owners = BTreeSet::new();
            for criterion in &criteria {
                if let Some(bindings) = index.criteria_to_implementations.get(criterion) {
                    for binding in bindings {
                        if index.bindings.contains_key(binding) {
                            owners.insert(binding.item.to_string());
                        }
                    }
                }
            }
            let mut diagnostic = Diagnostic::warning(
                "MITASE-QUALITY-003",
                format!(
                    "Requirement {} has {} criteria in {} disconnected implementation groups; \
                     the largest holds {} of {}",
                    requirement.id,
                    criteria.len(),
                    components.len(),
                    largest,
                    criteria.len()
                ),
                path.clone(),
            );
            diagnostic.subject = Some(DiagnosticSubject {
                kind: "spec-item".into(),
                value: requirement.id.to_string(),
            });
            diagnostic.evidence = vec![
                Evidence {
                    kind: "criteria_count".into(),
                    value: criteria.len().to_string(),
                },
                Evidence {
                    kind: "component_count".into(),
                    value: components.len().to_string(),
                },
                Evidence {
                    kind: "largest_component".into(),
                    value: format!("{largest}/{}", criteria.len()),
                },
                Evidence {
                    kind: "implementation_owners".into(),
                    value: owners.into_iter().collect::<Vec<_>>().join(","),
                },
            ];
            diagnostic.help = Some(
                "Consider splitting this Requirement by independent change reason so each \
                 item keeps one coherent implementation story. Keep the breadth only when \
                 it is the real design choice."
                    .into(),
            );
            diagnostic.next.push(ReadOnlyHint {
                kind: "show".into(),
                value: requirement.id.to_string(),
            });
            out.push(diagnostic);
        }
    }
}

/// Current exact implementation artifact identities for one Criterion.
///
/// Only non-planned, present, uniquely resolved targets count. An empty set
/// means the Criterion has no current implementation evidence.
fn current_implementation_identities(
    index: &SpecIndex,
    criterion: &SpecAnchor,
) -> BTreeSet<String> {
    index
        .criteria_to_implementation_targets
        .get(criterion)
        .into_iter()
        .flatten()
        .filter_map(|target| index.target_to_artifact.get(target))
        .cloned()
        .collect()
}

/// Q004 Duplicate Obligation: Criteria in different Requirements state the
/// same normative text under a common governing Policy rule and share one
/// exact implementation target.
///
/// All three conditions are required together. Matching text alone, or text
/// plus only a common target, stays silent so legitimate parallel Criteria on
/// different surfaces are never pushed toward hasty consolidation.
fn validate_duplicate_obligation(index: &SpecIndex, out: &mut Vec<Diagnostic>) {
    let mut groups: BTreeMap<String, Vec<SpecAnchor>> = BTreeMap::new();
    for (anchor, value) in &index.anchors {
        let mitase_workspace::AnchorValue::Criterion(criterion) = value else {
            continue;
        };
        if index.criterion_status.get(anchor) != Some(&ItemStatus::Implemented) {
            continue;
        }
        let normalized = normalize_statement(&criterion.statement);
        if normalized_len(&normalized) < MIN_NORMALIZED_STATEMENT_LEN {
            continue;
        }
        groups.entry(normalized).or_default().push(anchor.clone());
    }
    for members in groups.values() {
        if members.len() < 2 {
            continue;
        }
        // Members iterate in anchor order, so every unordered pair is
        // visited exactly once in a deterministic sequence.
        for (position, first) in members.iter().enumerate() {
            for second in members.iter().skip(position + 1) {
                if first.item == second.item {
                    continue;
                }
                let shared_rule = index
                    .criteria_to_rules
                    .get(first)
                    .map(|rules| {
                        rules
                            .iter()
                            .filter(|rule| {
                                index.anchors.get(rule).is_some_and(|value| {
                                    matches!(value, mitase_workspace::AnchorValue::Rule(_))
                                })
                            })
                            .collect::<BTreeSet<_>>()
                    })
                    .unwrap_or_default()
                    .intersection(
                        &index
                            .criteria_to_rules
                            .get(second)
                            .map(|rules| {
                                rules
                                    .iter()
                                    .filter(|rule| {
                                        index.anchors.get(rule).is_some_and(|value| {
                                            matches!(value, mitase_workspace::AnchorValue::Rule(_))
                                        })
                                    })
                                    .collect::<BTreeSet<_>>()
                            })
                            .unwrap_or_default(),
                    )
                    .next()
                    .cloned()
                    .cloned();
                let Some(shared_rule) = shared_rule else {
                    continue;
                };
                let shared_target = current_implementation_identities(index, first)
                    .intersection(&current_implementation_identities(index, second))
                    .next()
                    .cloned();
                let Some(shared_target) = shared_target else {
                    continue;
                };
                let mut diagnostic = Diagnostic::warning(
                    "MITASE-QUALITY-004",
                    format!(
                        "Criteria {first} and {second} express the same obligation under \
                         {shared_rule} with the same implementation target"
                    ),
                    item_path(index, first),
                );
                diagnostic.set_subject_anchor(first);
                diagnostic.reference = Some(anchor_subject(second));
                diagnostic.relation = Some(RelationRef {
                    relation: "duplicates".into(),
                    source: anchor_subject(first),
                    targets: vec![anchor_subject(second)],
                });
                diagnostic.related.push(RelatedLocation {
                    location: Location {
                        path: item_path(index, &shared_rule),
                        line: None,
                        column: None,
                        end_line: None,
                        end_column: None,
                        label: Some("common governing rule".into()),
                    },
                    message: format!("common governing rule {shared_rule}"),
                });
                diagnostic.evidence = vec![
                    Evidence {
                        kind: "normalized_statement_equal".into(),
                        value: "true".into(),
                    },
                    Evidence {
                        kind: "common_policy_rule".into(),
                        value: shared_rule.to_string(),
                    },
                    Evidence {
                        kind: "common_implementation_target".into(),
                        value: shared_target,
                    },
                ];
                diagnostic.help = Some(
                    "Keep both Criteria only when different surfaces genuinely need \
                     parallel acceptance, and record that reason. Otherwise consolidate \
                     the obligation and share the implementation target."
                        .into(),
                );
                diagnostic.next.push(ReadOnlyHint {
                    kind: "show".into(),
                    value: first.item.to_string(),
                });
                out.push(diagnostic);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalization_is_deterministic_across_case_space_and_punctuation() {
        assert_eq!(
            normalize_statement("  Login   failures, expose ONE generic response.  "),
            "login failures expose one generic response"
        );
        assert_eq!(
            normalize_statement("「ログイン失敗は、汎用的な応答を返す」。"),
            "ログイン失敗は汎用的な応答を返す"
        );
    }

    #[test]
    fn normalization_folds_fullwidth_characters() {
        assert_eq!(
            normalize_statement("Ｌｏｇｉｎ　failures　expose　１２　generic responses。"),
            "login failures expose 12 generic responses"
        );
    }

    #[test]
    fn normalization_preserves_negation_numbers_and_operators() {
        let kept = normalize_statement("must not delete data when count >= 4: retries = 0!");
        assert!(kept.contains("must not"), "negation words stay: {kept}");
        assert!(kept.contains(">="), "operators stay: {kept}");
        assert!(kept.contains('4'), "numbers stay: {kept}");
        assert!(kept.contains('='), "assignment stays: {kept}");
        assert_ne!(
            normalize_statement("Data must not be deleted under any condition whatsoever"),
            normalize_statement("Data must be deleted under any condition whatsoever"),
            "negation must keep statements distinct"
        );
    }

    #[test]
    fn short_shared_fragments_stay_silent() {
        assert!(!statements_match("must be secure", "Must be secure."));
        assert!(!statements_match("安全であること", "安全であること。"));
        let boundary: String = "a".repeat(MIN_NORMALIZED_STATEMENT_LEN);
        assert!(
            statements_match(&boundary, &boundary.to_uppercase()),
            "exactly {MIN_NORMALIZED_STATEMENT_LEN} characters fires"
        );
        let below: String = "a".repeat(MIN_NORMALIZED_STATEMENT_LEN - 1);
        assert!(
            !statements_match(&below, &below),
            "one character below the minimum stays silent"
        );
    }

    #[test]
    fn matching_ignores_wrapping_but_not_meaning() {
        assert!(statements_match(
            "Login failures expose one generic response to all callers.",
            "「Login failures expose one generic response to all callers」。"
        ));
        assert!(!statements_match(
            "Login failures expose one generic response to all callers.",
            "Login failures expose two generic responses to all callers."
        ));
    }

    #[test]
    fn largest_component_limit_pins_the_forty_percent_boundary() {
        assert!(largest_component_within_limit(2, 5));
        assert!(!largest_component_within_limit(3, 5));
        assert!(largest_component_within_limit(2, 6));
        assert!(!largest_component_within_limit(3, 6));
        assert!(largest_component_within_limit(4, 10));
        assert!(!largest_component_within_limit(5, 10));
    }

    fn test_binding(facet: &str) -> mitase_spec_model::ArtifactBinding {
        mitase_spec_model::ArtifactBinding {
            id: "test".into(),
            role: mitase_spec_model::BindingRole::Implementation,
            facet: facet.into(),
            responsibility: "test responsibility".into(),
            owns: Vec::new(),
            targets: Vec::new(),
        }
    }

    #[test]
    fn components_join_one_feature_across_facets() {
        let a1: SpecAnchor = "REQ-T-001#criterion.a1".parse().unwrap();
        let a2: SpecAnchor = "REQ-T-001#criterion.a2".parse().unwrap();
        let b1: SpecAnchor = "REQ-T-001#criterion.b1".parse().unwrap();
        let c1: SpecAnchor = "REQ-T-001#criterion.c1".parse().unwrap();
        let binding_one: SpecAnchor = "FEAT-T-001#binding.one".parse().unwrap();
        let binding_two: SpecAnchor = "FEAT-T-001#binding.two".parse().unwrap();
        let binding_other: SpecAnchor = "FEAT-T-002#binding.one".parse().unwrap();
        let binding_gone: SpecAnchor = "FEAT-T-003#binding.gone".parse().unwrap();
        let mut index = SpecIndex::default();
        index
            .bindings
            .insert(binding_one.clone(), test_binding("backend"));
        index
            .bindings
            .insert(binding_two.clone(), test_binding("frontend"));
        index
            .bindings
            .insert(binding_other.clone(), test_binding("backend"));
        index
            .criteria_to_implementations
            .insert(a1.clone(), vec![binding_one]);
        index
            .criteria_to_implementations
            .insert(a2.clone(), vec![binding_two]);
        index
            .criteria_to_implementations
            .insert(b1.clone(), vec![binding_other]);
        index
            .criteria_to_implementations
            .insert(c1.clone(), vec![binding_gone]);
        // `binding_gone` is absent from `index.bindings`: the structural
        // diagnostics own that finding, and the Criterion stays a singleton
        // instead of joining a phantom group.
        let components = criterion_components(&index, &[a1, a2, b1, c1]);
        assert_eq!(components.len(), 3);
        assert_eq!(
            components[0]
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>(),
            ["REQ-T-001#criterion.a1", "REQ-T-001#criterion.a2"]
        );
        assert_eq!(components[1].len(), 1);
        assert_eq!(components[2].len(), 1);
    }

    #[test]
    fn duplicate_pairs_scale_linearithmically_with_group_size() {
        use mitase_spec_model::{Criterion, CriterionKind, Rule, RuleLevel};
        const MEMBERS: usize = 120;
        let statement =
            "Scale fixture entries must remain exactly comparable across every surface.";
        let rule: SpecAnchor = "POL-S-001#rule.shared".parse().unwrap();
        let shared_target: mitase_spec_model::BoundTargetRef =
            "FEAT-S-001#binding.impl/target.main".parse().unwrap();
        let mut index = SpecIndex::default();
        index.anchors.insert(
            rule.clone(),
            mitase_workspace::AnchorValue::Rule(Rule {
                id: "shared".into(),
                level: RuleLevel::Should,
                statement: "Unrelated rule text that stays distinct.".into(),
                governed_by: Vec::new(),
                applies_to: Default::default(),
                enforcement: None,
            }),
        );
        index
            .target_to_artifact
            .insert(shared_target.clone(), "rust:src/lib.rs::lib::shared".into());
        let mut members = Vec::new();
        for number in 0..MEMBERS {
            let anchor: SpecAnchor = format!("REQ-S-{number:03}#criterion.dup").parse().unwrap();
            index.anchors.insert(
                anchor.clone(),
                mitase_workspace::AnchorValue::Criterion(Criterion {
                    id: "dup".into(),
                    kind: CriterionKind::Behavior,
                    statement: statement.into(),
                    governed_by: vec![rule.clone()],
                }),
            );
            index
                .criterion_status
                .insert(anchor.clone(), ItemStatus::Implemented);
            index
                .criteria_to_rules
                .insert(anchor.clone(), vec![rule.clone()]);
            index
                .criteria_to_implementation_targets
                .insert(anchor.clone(), vec![shared_target.clone()]);
            members.push(anchor);
        }
        let mut out = Vec::new();
        validate_duplicate_obligation(&index, &mut out);
        assert_eq!(out.len(), MEMBERS * (MEMBERS - 1) / 2);
        assert!(
            out.iter()
                .all(|diagnostic| diagnostic.rule_id == "MITASE-QUALITY-004")
        );
        let mut signatures: Vec<String> = out
            .iter()
            .map(|diagnostic| {
                format!(
                    "{}|{}",
                    diagnostic
                        .subject
                        .as_ref()
                        .map(|subject| subject.value.as_str())
                        .unwrap_or(""),
                    diagnostic
                        .reference
                        .as_ref()
                        .map(|reference| reference.value.as_str())
                        .unwrap_or("")
                )
            })
            .collect();
        signatures.sort();
        signatures.dedup();
        assert_eq!(signatures.len(), out.len(), "every pair is reported once");
    }
}
