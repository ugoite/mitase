# Tutorial

This tutorial creates a minimal v1 workspace by hand.

1. Add `mitase.yaml` with `schema: mitase/config/v1`.
2. Create `docs/mitase/philosophy/foundation.yaml`.
3. Create `docs/mitase/policies/policies.yaml`.
4. Create `docs/mitase/requirements/core/core.yaml`.
5. Create `docs/mitase/features/core/core.yaml`.
6. Run `cargo run --quiet -- validate .`.

A minimal requirement/feature connection looks like this:

```yaml
schema: mitase/spec/v1
kind: requirements
namespace: demo
category: Demo
requirements:
  - id: REQ-DEMO-001
    title: Keep one behavior exact
    description: Example requirement.
    priority: high
    status: implemented
    criteria:
      - id: exact-behavior
        kind: behavior
        statement: One feature owns one implementation path and one verification path.
        governed_by: [POL-DEMO-001#rule.traceable-delivery]
    bindings:
      - id: verification
        role: verification
        facet: verification
        responsibility: Verify the example behavior.
        targets:
          - id: test
            adapter: rust
            path: tests/example.rs
            selector: { kind: file }
        verifies: [REQ-DEMO-001#criterion.exact-behavior]
```

```yaml
schema: mitase/spec/v1
kind: features
namespace: demo
category: Demo
features:
  - id: FEAT-DEMO-001
    title: Example delivery
    summary: Example feature.
    status: implemented
    bindings:
      - id: implementation
        role: implementation
        facet: delivery
        responsibility: Implement the example behavior.
        targets:
          - id: source
            adapter: rust
            path: src/example.rs
            selector: { kind: file }
        satisfies: [REQ-DEMO-001#criterion.exact-behavior]
```

Use the checked-in examples for larger layouts.
