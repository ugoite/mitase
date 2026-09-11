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
```

Requirement and criterion meaning, binding responsibilities, claims, and the
verification runner are explicit. The normalizer may only infer mechanics:

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
