---
title: "Spec quality diagnostics"
description: "Stable Q001-Q005 contract for structurally unnatural specifications."
sidebar_position: 2
---

# Spec quality diagnostics

<!-- FEAT-DOCS-001 -->

`mitase check .` stays green on a structurally unnatural specification. The
Q001–Q005 family reports **bad-but-valid** shapes with evidence so authors can
decide whether each layer carries its own meaning. Quality diagnostics never
become new hard errors in the release that introduces them, and they never
weaken the existing safety errors.

All five diagnostics use the shared diagnostic shape (`code`, `phase`,
`severity`, `primary`, `related_spans`, `subject`, `reference`, `evidence`,
`suggested_action`). `phase` is `graph`. Output order and content are
deterministic for the same input. Only code-prefix filtering
(`MITASE-QUALITY-*`) is needed to select the quality-only subset in text, JSON,
and LSP output. Mitase offers no automatic fix.

## Rule table

| Code | Name | Fires when | Severity |
| --- | --- | --- | --- |
| `MITASE-QUALITY-001` | Layer Echo | A Criterion's direct `governed_by` Policy rule `statement` and the Criterion `statement` are equal after conservative normalization, with sufficient length. | Warning |
| `MITASE-QUALITY-002` | One-off Policy | A Policy shows at least 3 of 4 signs, including mandatory single downstream Criterion: (a) exactly one downstream Criterion, (b) shared concrete number, API, or file token, (c) high statement similarity, (d) strongly overlapping evidence and implementation targets. | Warning |
| `MITASE-QUALITY-003` | Fragmented Requirement | A Requirement's Criterion–Feature bipartite graph has 5+ Criteria, 3+ connected components, the largest component holds 40% or fewer of the Criteria, and implementation relations are sufficiently resolved. | Warning |
| `MITASE-QUALITY-004` | Duplicate Obligation | Criteria in different Requirements have equal normalized normative text, share a governing Policy rule, and share the same exact implementation target (plus verification target where applicable). | Warning |
| `MITASE-QUALITY-005` | Redundant Rule Description | A non-empty Policy `description` equals one of its `rule.statement` values after normalization. | Info |

## Comparison rules

- Normalization is limited to Unicode compatibility normalization, case,
  leading/trailing and repeated whitespace, and obvious presentational
  punctuation. It never removes must/should modality, negation, comparison
  operators, numbers, or target names. It is deterministic in Japanese and
  English.
- Only Q002 may use language-independent character n-gram-style similarity.
  Deterministic decisions (Q001/Q004/Q005) never fire on fuzzy similarity
  alone.
- Q003 uses only direct `satisfies` Criterion-to-Feature relations, joined
  within the same Feature. Facets never split one Feature or Criterion.
  `exposes` is never treated as `satisfies`. Planned, absent, or unresolved
  relations are never counted as independence; when relations are too sparse,
  the existing coverage diagnostics own the finding and Q003 stays silent.
- Q004 requires the full conjunction of same text, common Policy, and same
  exact target. Parallel Criteria that legitimately describe the same behavior
  on different surfaces must stay silent.
- Q005 ignores Policies with an empty description.

## Evidence and messages

Each diagnostic carries stable string key/value evidence (for example
`normalized_statement_equal=true`, `governed_by=direct`, matched signs,
reference counts, shared tokens or targets, component sizes and ratios,
scores and thresholds where applicable). The primary location is the
Criterion (Q001–Q004) or Policy (Q005) line and column; the related span is
the counterpart anchor. File plus anchor identity is always preserved when an
exact position is unavailable.

Q001 example shape:

```text
MITASE-QUALITY-001 [warning]
Policy POL-EXAMPLE-001#rule.governance and Criterion
REQ-EXAMPLE-001#criterion.bound express the same normative statement.
Evidence: normalized_statement_equal=true; governed_by=direct.
Suggested action: Keep the local acceptance condition in Criterion.
Either generalize the Policy into a reusable decision rule, or remove/merge
that Policy if it has no independent meaning.
```

## Negative controls

The following shapes are intentionally silent and are covered by regression
tests:

- short shared phrases below the minimum length, negated or numerically
  distinct statements, and indirect (non-`governed_by`) topic overlap;
- a single-Criterion Policy that states a genuine reusable invariant (for
  example a security boundary) without the remaining Q002 signs;
- one Feature shared across Facets (`core`/`backend`/`frontend`/MCP) without
  fragmentation;
- legitimately parallel Criteria on distinct surfaces without a common Policy
  and exact target;
- distinct Policy descriptions that merely share vocabulary.

## What v0.2.5 does not decide mechanically

- Whether a `governed_by` derivation is causally correct.
- Whether a Criterion sentence is truly falsifiable as natural language.
- Whether a named test semantically proves the Criterion. An exact `verifies`
  claim existing and a test proving intent are different facts.

These remain authoring-guide and human-review concerns. Future diagnostics may
cover them only when a verifiable structural condition is found; guessing
never becomes a hard gate.

## Continue with these pages

- [Spec anti-patterns](./spec-antipatterns.md) for the authoring principles
  behind these signals
- [Diagnostics](../../workflows/repository/diagnostics.md) for consuming
  diagnostics in CLI and CI
