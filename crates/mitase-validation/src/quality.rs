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
use mitase_spec_model::{LocalAnchorKind, SemanticDocument, SpecAnchor};
use mitase_workspace::{SpecIndex, SpecWorkspace};
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
}
