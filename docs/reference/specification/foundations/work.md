---
title: "Work planning / Work"
description: "Generated reference for docs/mitase/requirements/work.yaml"
---

> Generated from `docs/mitase/requirements/work.yaml`.

## Parsed content

### Schema

- mitase/spec/v1

### Kind

- requirements

### Namespace

- work

### Category

- Work planning

### Requirements

- **id**: REQ-WORK-001
  - **title**: Plan exact executable work
  - **description**: A caller can derive bounded slices from one exact Work origin and select one canonical execution boundary before delivery.
  - **priority**: critical
  - **status**: implemented
  - **criteria**:
    - **id**: exact-slice
      - **kind**: behavior
      - **statement**: An exact Requirement criterion or validated Feature implementation origin produces explicit editable, verification, and readonly targets with one typed semantic identity.
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
          - **path**: crates/mitase-spec-model/src/lib.rs
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
                  - **package**: mitase-spec-model
                  - **test**: tests::anchors_roundtrip
        - **id**: lifecycle-plan-test
          - **adapter**: rust
          - **path**: crates/mitase-planner/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: tests::explicit_add_transition_plans_missing_target_as_ensure_present
          - **claims**:
            - **kind**: verifies
              - **criterion**: REQ-WORK-001#criterion.exact-slice
              - **covers**:
                - FEAT-PLANNER-001#binding.implementation/target.lifecycle-plan
              - **runner**:
                - **runner**: cargo-test
                - **arguments**:
                  - **package**: mitase-planner
                  - **test**: tests::explicit_add_transition_plans_missing_target_as_ensure_present

## Source YAML

```yaml
schema: mitase/spec/v1
kind: requirements
namespace: work
category: Work planning
requirements:
  - id: REQ-WORK-001
    title: Plan exact executable work
    description: A caller can derive bounded slices from one exact Work origin and select one canonical execution boundary before delivery.
    priority: critical
    status: implemented
    criteria:
      - id: exact-slice
        kind: behavior
        statement: An exact Requirement criterion or validated Feature implementation origin produces explicit editable, verification, and readonly targets with one typed semantic identity.
        governed_by: [POL-DELIVERY-001#rule.exact-ownership]
    bindings:
      - id: planner-test
        role: verification
        facet: verification
        responsibility: Verify stable anchor and exact target reference behavior used by planning.
        targets:
          - id: anchor-test
            adapter: rust
            path: crates/mitase-spec-model/src/lib.rs
            selector: { kind: symbol, name: anchors_roundtrip }
            claims:
              - kind: verifies
                criterion: REQ-WORK-001#criterion.exact-slice
                covers: [FEAT-PLANNER-001#binding.implementation/target.canonical-plan]
                runner: { runner: cargo-test, arguments: { package: mitase-spec-model, test: tests::anchors_roundtrip } }
          - id: lifecycle-plan-test
            adapter: rust
            path: crates/mitase-planner/src/lib.rs
            selector: { kind: symbol, name: tests::explicit_add_transition_plans_missing_target_as_ensure_present }
            claims:
              - kind: verifies
                criterion: REQ-WORK-001#criterion.exact-slice
                covers: [FEAT-PLANNER-001#binding.implementation/target.lifecycle-plan]
                runner: { runner: cargo-test, arguments: { package: mitase-planner, test: tests::explicit_add_transition_plans_missing_target_as_ensure_present } }
```
