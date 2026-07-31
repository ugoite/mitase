---
title: "Delivery governance / Delivery"
description: "Generated reference for docs/syu/policies/delivery.yaml"
---

> Generated from `docs/syu/policies/delivery.yaml`.

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
            - **name**: validate
          - **claims**:
            - **kind**: enforces
              - **rule**: POL-DELIVERY-001#rule.exact-ownership
- **id**: POL-ADOPTION-001
  - **title**: Capability-bounded self-hosting
  - **summary**: Adopt repository governance only when one coherent capability has exact intent, ownership, and evidence.
  - **description**: Readiness is earned capability by capability; catalog size and planned work never substitute for current evidence.
  - **rules**:
    - **id**: active-status
      - **level**: must
      - **statement**: Planned Features must not own active artifacts or satisfy the current readiness denominator.
      - **governed_by**:
        - PHIL-001#principle.exact-intent
      - **applies_to**:
        - **roles**:
          - implementation
          - verification
    - **id**: feature-evidence
      - **level**: must
      - **statement**: Every implemented Feature target must satisfy an implemented acceptance criterion and have exact configured verification covering that target.
      - **governed_by**:
        - PHIL-001#principle.exact-intent
      - **applies_to**:
        - **roles**:
          - implementation
          - verification
    - **id**: bounded-rollout
      - **level**: must
      - **statement**: Repository adoption advances explicit capability and public-entrypoint probes without repository-wide catch-all ownership.
      - **governed_by**:
        - PHIL-001#principle.exact-intent
      - **applies_to**:
        - **roles**:
          - implementation
          - verification
          - configuration
  - **bindings**:
    - **id**: feature-governance
      - **role**: enforcement
      - **facet**: self-hosting
      - **responsibility**: Reject planned ownership and implemented Features without exact acceptance and verification closure.
      - **targets**:
        - **id**: feature-shape-validation
          - **adapter**: rust
          - **path**: crates/syu-validation/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: validate_document_shapes
          - **claims**:
            - **kind**: enforces
              - **rule**: POL-ADOPTION-001#rule.active-status
            - **kind**: enforces
              - **rule**: POL-ADOPTION-001#rule.feature-evidence
    - **id**: readiness-governance
      - **role**: enforcement
      - **facet**: self-hosting
      - **responsibility**: Evaluate only explicit capability and public-entrypoint readiness subjects.
      - **targets**:
        - **id**: public-readiness
          - **adapter**: rust
          - **path**: crates/syu-validation/src/readiness.rs
          - **selector**:
            - **kind**: symbol
            - **name**: public_entrypoint_subjects
          - **claims**:
            - **kind**: enforces
              - **rule**: POL-ADOPTION-001#rule.bounded-rollout

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
            selector: { kind: symbol, name: validate }
            claims:
              - kind: enforces
                rule: POL-DELIVERY-001#rule.exact-ownership

  - id: POL-ADOPTION-001
    title: Capability-bounded self-hosting
    summary: Adopt repository governance only when one coherent capability has exact intent, ownership, and evidence.
    description: Readiness is earned capability by capability; catalog size and planned work never substitute for current evidence.
    rules:
      - id: active-status
        level: must
        statement: Planned Features must not own active artifacts or satisfy the current readiness denominator.
        governed_by: [PHIL-001#principle.exact-intent]
        applies_to: { roles: [implementation, verification] }
      - id: feature-evidence
        level: must
        statement: Every implemented Feature target must satisfy an implemented acceptance criterion and have exact configured verification covering that target.
        governed_by: [PHIL-001#principle.exact-intent]
        applies_to: { roles: [implementation, verification] }
      - id: bounded-rollout
        level: must
        statement: Repository adoption advances explicit capability and public-entrypoint probes without repository-wide catch-all ownership.
        governed_by: [PHIL-001#principle.exact-intent]
        applies_to: { roles: [implementation, verification, configuration] }
    bindings:
      - id: feature-governance
        role: enforcement
        facet: self-hosting
        responsibility: Reject planned ownership and implemented Features without exact acceptance and verification closure.
        targets:
          - id: feature-shape-validation
            adapter: rust
            path: crates/syu-validation/src/lib.rs
            selector: { kind: symbol, name: validate_document_shapes }
            claims:
              - kind: enforces
                rule: POL-ADOPTION-001#rule.active-status
              - kind: enforces
                rule: POL-ADOPTION-001#rule.feature-evidence
      - id: readiness-governance
        role: enforcement
        facet: self-hosting
        responsibility: Evaluate only explicit capability and public-entrypoint readiness subjects.
        targets:
          - id: public-readiness
            adapter: rust
            path: crates/syu-validation/src/readiness.rs
            selector: { kind: symbol, name: public_entrypoint_subjects }
            claims:
              - kind: enforces
                rule: POL-ADOPTION-001#rule.bounded-rollout
```
