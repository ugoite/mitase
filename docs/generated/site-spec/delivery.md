---
title: "Delivery governance / Delivery"
description: "Generated reference for docs/syu/delivery.yaml"
---

> Generated from `docs/syu/delivery.yaml`.

## Parsed content

### Schema

- syu/spec/v1

### Kind

- policies

### Namespace

- delivery

### Category

- Delivery governance

### Policies

- **id**: POL-DELIVERY-001
  - **title**: Exact executable work
  - **summary**: Executable work is derived only from explicit bindings.
  - **description**: The shared validator rejects incomplete graph and work scope.
  - **rules**:
    - **id**: exact-ownership
      - **level**: must
      - **statement**: Executable targets must have one explicit specification owner.
      - **governed_by**:
        - PHIL-001#principle.exact-intent
      - **applies_to**:
        - **roles**:
          - implementation
          - verification
  - **bindings**:
    - **id**: shared-validator
      - **role**: enforcement
      - **facet**: tooling
      - **responsibility**: Enforce graph integrity, exact targets, and plan scope through one validation engine.
      - **targets**:
        - **id**: validation-entry
          - **adapter**: rust
          - **path**: crates/syu-validation/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **names**:
              - validate
              - validate_plan
      - **enforces**:
        - POL-DELIVERY-001#rule.exact-ownership

## Source YAML

```yaml
schema: syu/spec/v1
kind: policies
namespace: delivery
category: Delivery governance
policies:
  - id: POL-DELIVERY-001
    title: Exact executable work
    summary: Executable work is derived only from explicit bindings.
    description: The shared validator rejects incomplete graph and work scope.
    rules:
      - id: exact-ownership
        level: must
        statement: Executable targets must have one explicit specification owner.
        governed_by: [PHIL-001#principle.exact-intent]
        applies_to: { roles: [implementation, verification] }
    bindings:
      - id: shared-validator
        role: enforcement
        facet: tooling
        responsibility: Enforce graph integrity, exact targets, and plan scope through one validation engine.
        targets:
          - id: validation-entry
            adapter: rust
            path: crates/syu-validation/src/lib.rs
            selector: { kind: symbol, names: [validate, validate_plan] }
        enforces: [POL-DELIVERY-001#rule.exact-ownership]
```
