---
title: v0.2 short authoring
---

Mitase v0.2 introduces a short authoring contract at the frontend boundary.
It normalizes into the existing `mitase/spec/v1` canonical graph; it does not
replace the semantic model.

During the 0.1.x dogfood period, normal workspace loading accepts both
`mitase/spec/v1` documents and `mitase/authoring/v2` documents. The latter are
parsed and normalized before indexing and validation, so downstream consumers
always receive the canonical graph.

The loader derives the source policy from the Mitase release line: 0.1.x uses
the dual-source policy, and 0.2.x uses the v2-only policy. The 0.2 policy
rejects v1 source at the workspace boundary with an explicit migration action.

The first short form describes one requirement, one criterion, one
implementation target, and one verification target:

```yaml
schema: mitase/authoring/v2
kind: requirement
namespace: demo
category: Demo
requirement:
  id: REQ-DEMO-001
  title: The requirement intent is explicit
  description: The requirement description is explicit.
  priority: high
  status: implemented
  criterion:
    id: behavior
    kind: behavior
    statement: The criterion statement is explicit.
    governed_by: []
  implementation:
    facet: delivery
    responsibility: Own the exact implementation.
    target:
      path: src/example.rs
      satisfies: behavior
  verification:
    facet: verification
    responsibility: Verify the exact behavior.
    target:
      path: tests/example.rs
      verifies:
        criterion: behavior
        covers: [source]
        runner: cargo-test
        arguments: { package: demo, test: example_behavior }
```

Requirement and criterion meaning, binding responsibilities, claims, and the
verification runner metadata are explicit. A standalone short requirement is
a valid seed workspace: its implementation and verification bindings are
normalized into the canonical requirement graph, and an empty
`criterion.governed_by` relation means that the slice has not yet introduced a
policy layer. The normalizer may only infer mechanics:

- binding IDs default to `implementation` and `verification`;
- target IDs default to `source` and `test`;
- an omitted selector defaults to the exact `file` selector;
- an omitted adapter is inferred from the target extension only when there is
  exactly one candidate.

Inference is fail-closed: zero candidates and multiple candidates are errors.
For example, JSON, YAML, and Markdown extensions overlap multiple inventory
adapters and therefore require an explicit adapter.
The normalized result exposes `applied_defaults` and `inferred` provenance so
the authoring result is fully inspectable.

Authoring frontend failures use the shared diagnostic object used by semantic
validation. A v1 source rejected by the v0.2 loader is reported as
`MITASE-SOURCE-001`; malformed v2 input as `MITASE-AUTHORING-001`; and failed
normalization as `MITASE-AUTHORING-002` (no candidate), `003` (ambiguous
candidates), or `004` (unknown exact reference). Each diagnostic carries a
path/span, a reason, any candidates, and a suggested next action. `check` and
`validate` serialize that same object in JSON and render it in text; the LSP
publishes it in `publishDiagnostics.data` without introducing a separate
editor-specific semantic model.

## Corpus boundary

The Mitase corpus is the measurement boundary for this contract. The current
corpus contains 11 authoring-v2 documents and 30 top-level semantic items:
3 philosophies, 7 policies, 2 requirements, and 18 features. Neither
requirement fits the minimal short shape because the first has 13 criteria and
two verification bindings, while the second has three criteria and six
verification targets. The corpus therefore stays in the explicit collection
forms; it is not rewritten into a short form that would discard relations or
binding responsibilities.

The only additional omission supported by the corpus audit is an empty
`criterion.governed_by` list. An absent list means exactly the same empty
relation set and does not infer a governing policy. Requirement text, status,
priority, criterion kind and statement, binding facets and responsibilities,
claims, coverage, and runners remain required. Adapter inference remains
fail-closed: Rust paths have one candidate, while overlapping formats such as
Markdown and JSON require an explicit adapter.
