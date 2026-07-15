---
title: "Work planning / Work"
description: "Generated reference for docs/syu/work.yaml"
---

> Generated from `docs/syu/work.yaml`.

## Parsed content

### Schema

- syu/spec/v1

### Kind

- requirements

### Namespace

- work

### Category

- Work planning

### Requirements

- **id**: REQ-WORK-001
  - **title**: Plan exact executable work
  - **description**: A caller can derive bounded slices from an exact criterion, binding, or contract.
  - **priority**: critical
  - **status**: implemented
  - **criteria**:
    - **id**: exact-slice
      - **kind**: behavior
      - **statement**: An exact criterion seed produces explicit editable, verification, and readonly targets.
      - **governed_by**:
        - POL-DELIVERY-001#rule.exact-ownership
  - **bindings**:
    - **id**: planner-test
      - **role**: verification
      - **facet**: verification
      - **responsibility**: Verify stable anchor and exact target reference behavior used by planning.
      - **targets**:
        - **id**: anchor-test
          - **adapter**: rust
          - **path**: crates/syu-spec-model/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: anchors_roundtrip
          - **claims**:
            - **kind**: verifies
              - **criterion**: REQ-WORK-001#criterion.exact-slice
              - **covers**:
                - FEAT-PLANNER-001#binding.implementation/target.canonical-plan
              - **runner**:
                - **runner**: cargo-test
                - **arguments**:
                  - **package**: syu-spec-model
                  - **test**: tests::anchors_roundtrip
- **id**: REQ-PUBLIC-001
  - **title**: Public entrypoint traceability
  - **description**: Every discovered public entrypoint has an exact target and bounded canonical work plan.
  - **priority**: low
  - **status**: planned
  - **criteria**:
    - **id**: entrypoint
      - **kind**: quality
      - **statement**: A public entrypoint resolves through an exact target to a canonical plan.
      - **governed_by**:
        - POL-DELIVERY-001#rule.exact-ownership

## Source YAML

```yaml
schema: syu/spec/v1
kind: requirements
namespace: work
category: Work planning
requirements:
  - id: REQ-WORK-001
    title: Plan exact executable work
    description: A caller can derive bounded slices from an exact criterion, binding, or contract.
    priority: critical
    status: implemented
    criteria:
      - id: exact-slice
        kind: behavior
        statement: An exact criterion seed produces explicit editable, verification, and readonly targets.
        governed_by: [POL-DELIVERY-001#rule.exact-ownership]
    bindings:
      - id: planner-test
        role: verification
        facet: verification
        responsibility: Verify stable anchor and exact target reference behavior used by planning.
        targets:
          - id: anchor-test
            adapter: rust
            path: crates/syu-spec-model/src/lib.rs
            selector: { kind: symbol, name: anchors_roundtrip }
            claims:
              - kind: verifies
                criterion: REQ-WORK-001#criterion.exact-slice
                covers: [FEAT-PLANNER-001#binding.implementation/target.canonical-plan]
                runner: { runner: cargo-test, arguments: { package: syu-spec-model, test: tests::anchors_roundtrip } }
  - id: REQ-PUBLIC-001
    title: Public entrypoint traceability
    description: Every discovered public entrypoint has an exact target and bounded canonical work plan.
    priority: low
    status: planned
    criteria:
      - id: entrypoint
        kind: quality
        statement: A public entrypoint resolves through an exact target to a canonical plan.
        governed_by: [POL-DELIVERY-001#rule.exact-ownership]
```
