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
            - **names**:
              - tests::anchors_roundtrip
              - tests::target_refs_do_not_use_array_positions
      - **verifies**:
        - REQ-WORK-001#criterion.exact-slice
- **id**: REQ-WORK-002
  - **title**: Require qualitative item coverage
  - **description**: A repository can require every implemented specification item to reach a named Syu benefit level without using a line-coverage percentage.
  - **priority**: high
  - **status**: implemented
  - **criteria**:
    - **id**: qualitative-coverage
      - **kind**: quality
      - **statement**: Validate fails when an implemented item does not reach the coverage target configured for the whole workspace.
      - **governed_by**:
        - POL-DELIVERY-001#rule.exact-ownership
  - **bindings**:
    - **id**: coverage-test
      - **role**: verification
      - **facet**: verification
      - **responsibility**: Verify qualitative coverage targets, defaults, and whole-workspace enforcement.
      - **targets**:
        - **id**: coverage-cli
          - **adapter**: rust
          - **path**: tests/v1_cli.rs
          - **selector**:
            - **kind**: symbol
            - **names**:
              - validates_qualitative_item_coverage
      - **verifies**:
        - REQ-WORK-002#criterion.qualitative-coverage

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
            selector: { kind: symbol, names: [tests::anchors_roundtrip, tests::target_refs_do_not_use_array_positions] }
        verifies: [REQ-WORK-001#criterion.exact-slice]
  - id: REQ-WORK-002
    title: Require qualitative item coverage
    description: A repository can require every implemented specification item to reach a named Syu benefit level without using a line-coverage percentage.
    priority: high
    status: implemented
    criteria:
      - id: qualitative-coverage
        kind: quality
        statement: Validate fails when an implemented item does not reach the coverage target configured for the whole workspace.
        governed_by: [POL-DELIVERY-001#rule.exact-ownership]
    bindings:
      - id: coverage-test
        role: verification
        facet: verification
        responsibility: Verify qualitative coverage targets, defaults, and whole-workspace enforcement.
        targets:
          - id: coverage-cli
            adapter: rust
            path: tests/v1_cli.rs
            selector: { kind: symbol, names: [validates_qualitative_item_coverage] }
        verifies: [REQ-WORK-002#criterion.qualitative-coverage]
```
