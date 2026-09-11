# Tutorial

This tutorial creates a minimal v2 authoring workspace by hand. The v2
documents are normalized into Mitase's canonical specification graph before
validation.

1. Add `mitase.yaml` with `schema: mitase/config/v1`.
2. Create `docs/mitase/philosophy/foundation.yaml`.
3. Create `docs/mitase/policies/policies.yaml`.
4. Create `docs/mitase/requirements/core/core.yaml`.
5. Create `docs/mitase/features/core/core.yaml`.
6. Run `cargo run --quiet -- validate workspace .`.

A minimal requirement/feature connection looks like this:

```yaml
schema: mitase/authoring/v2
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
            claims:
              - kind: verifies
                criterion: REQ-DEMO-001#criterion.exact-behavior
                covers: []
                runner: { runner: cargo-test, arguments: {} }
```

```yaml
schema: mitase/authoring/v2
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
            claims:
              - kind: satisfies
                criterion: REQ-DEMO-001#criterion.exact-behavior
```

Existing v1 source can still be converted during the 0.1.x dogfood period with
the read-only `mitase migrate <source> --stdout` command. Use the checked-in
examples and the [short authoring contract](../../workflows/repository/authoring-v2.md)
for larger v2 layouts.
