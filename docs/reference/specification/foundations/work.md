---
title: "Work planning / Work"
description: "Generated reference for docs/syu/requirements/work.yaml"
---

> Generated from `docs/syu/requirements/work.yaml`.

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
    - **id**: completion-evidence
      - **kind**: behavior
      - **statement**: A verified slice reports exactly which acceptance criteria and completion checks are demonstrated.
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
        - **id**: lifecycle-plan-test
          - **adapter**: rust
          - **path**: crates/syu-planner/src/lib.rs
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
                  - **package**: syu-planner
                  - **test**: tests::explicit_add_transition_plans_missing_target_as_ensure_present
- **id**: REQ-WORK-002
  - **title**: Durable completion delivery
  - **description**: Completion verification is preserved as immutable attempts and can be finalized only after explicit plan approval.
  - **priority**: critical
  - **status**: implemented
  - **criteria**:
    - **id**: store-boundary
      - **kind**: behavior
      - **statement**: Delivery data stays outside the worktree in a stable repository-local store with explicit approval and attempt paths.
      - **governed_by**:
        - POL-DELIVERY-001#rule.exact-ownership
    - **id**: immutable-attempt
      - **kind**: behavior
      - **statement**: Failed and successful verification attempts are digest-checked, immutable, and queryable without widening their approved scope.
      - **governed_by**:
        - POL-DELIVERY-001#rule.exact-ownership
    - **id**: approval-scope
      - **kind**: behavior
      - **statement**: An approval and its stored plan preserve the reviewed workspace, targets, and status transition without hidden expansion.
      - **governed_by**:
        - POL-DELIVERY-001#rule.exact-ownership
    - **id**: finalization-handoff
      - **kind**: behavior
      - **statement**: A complete attempt can promote only its exact planned specification items after a fresh overlay validation.
      - **governed_by**:
        - POL-DELIVERY-001#rule.exact-ownership
- **id**: REQ-WORK-003
  - **title**: Scoped implementation agent
  - **description**: An implementation tool can perform only approved Modify, Add, or Remove transitions for editable targets in one WorkPlan slice and must preserve inspectable evidence for every decision.
  - **priority**: critical
  - **status**: implemented
  - **criteria**:
    - **id**: scoped-write
      - **kind**: security
      - **statement**: A scoped agent write is checked against the approved plan, slice, target digest, transition lifecycle, access mode, and budget before application; Add and Remove preconditions reject newly existing or stale targets.
      - **governed_by**:
        - POL-DELIVERY-001#rule.exact-ownership
    - **id**: expansion-request
      - **kind**: behavior
      - **statement**: A tool can request explicit scope expansion without changing its current write permissions.
      - **governed_by**:
        - POL-DELIVERY-001#rule.exact-ownership
    - **id**: agent-evidence
      - **kind**: behavior
      - **statement**: Accepted, rejected, blocked, and scope-expansion agent events remain inspectable with a precise next action.
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
      - id: completion-evidence
        kind: behavior
        statement: A verified slice reports exactly which acceptance criteria and completion checks are demonstrated.
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
          - id: lifecycle-plan-test
            adapter: rust
            path: crates/syu-planner/src/lib.rs
            selector: { kind: symbol, name: tests::explicit_add_transition_plans_missing_target_as_ensure_present }
            claims:
              - kind: verifies
                criterion: REQ-WORK-001#criterion.exact-slice
                covers: [FEAT-PLANNER-001#binding.implementation/target.lifecycle-plan]
                runner: { runner: cargo-test, arguments: { package: syu-planner, test: tests::explicit_add_transition_plans_missing_target_as_ensure_present } }
  - id: REQ-WORK-002
    title: Durable completion delivery
    description: Completion verification is preserved as immutable attempts and can be finalized only after explicit plan approval.
    priority: critical
    status: implemented
    criteria:
      - id: store-boundary
        kind: behavior
        statement: Delivery data stays outside the worktree in a stable repository-local store with explicit approval and attempt paths.
        governed_by: [POL-DELIVERY-001#rule.exact-ownership]
      - id: immutable-attempt
        kind: behavior
        statement: Failed and successful verification attempts are digest-checked, immutable, and queryable without widening their approved scope.
        governed_by: [POL-DELIVERY-001#rule.exact-ownership]
      - id: approval-scope
        kind: behavior
        statement: An approval and its stored plan preserve the reviewed workspace, targets, and status transition without hidden expansion.
        governed_by: [POL-DELIVERY-001#rule.exact-ownership]
      - id: finalization-handoff
        kind: behavior
        statement: A complete attempt can promote only its exact planned specification items after a fresh overlay validation.
        governed_by: [POL-DELIVERY-001#rule.exact-ownership]
  - id: REQ-WORK-003
    title: Scoped implementation agent
    description: An implementation tool can perform only approved Modify, Add, or Remove transitions for editable targets in one WorkPlan slice and must preserve inspectable evidence for every decision.
    priority: critical
    status: implemented
    criteria:
      - id: scoped-write
        kind: security
        statement: A scoped agent write is checked against the approved plan, slice, target digest, transition lifecycle, access mode, and budget before application; Add and Remove preconditions reject newly existing or stale targets.
        governed_by: [POL-DELIVERY-001#rule.exact-ownership]
      - id: expansion-request
        kind: behavior
        statement: A tool can request explicit scope expansion without changing its current write permissions.
        governed_by: [POL-DELIVERY-001#rule.exact-ownership]
      - id: agent-evidence
        kind: behavior
        statement: Accepted, rejected, blocked, and scope-expansion agent events remain inspectable with a precise next action.
        governed_by: [POL-DELIVERY-001#rule.exact-ownership]
```
