---
title: "Delivery"
description: "Generated reference for docs/spec/errors/delivery.yaml"
---

> Generated from `docs/spec/errors/delivery.yaml`.

## Parsed content

### Genre

- Delivery State

### Version

- 1

### Rules

- **code**: invalid-status
  - **severity**: error
  - **title**: Delivery state values must be supported
  - **summary**: Requirements and features need a status vocabulary the validator understands.
  - **description**:
    - |
      Delivery-state automation only works when the state machine is explicit and
      finite. If contributors invent ad hoc status names, `syu` can no longer
      tell whether traces should exist, be absent, or be considered complete.
      This rule keeps workflow semantics predictable by limiting the state
      vocabulary to supported values.
- **code**: planned-status-disallowed
  - **severity**: error
  - **title**: Planned items can be forbidden by project policy
  - **summary**: Some repositories want the spec to describe only already-delivered work.
  - **description**:
    - |
      Teams reach moments when backlog-style entries should no longer remain in
      the committed specification for a branch, release line, or audited
      workspace. This rule lets a project tighten that policy without rewriting
      the entire validation engine. The goal is to support both exploratory
      workflows and locked-down release workflows through explicit config.
- **code**: planned-trace-present
  - **severity**: error
  - **title**: Planned items must not claim delivered traces
  - **summary**: Declaring tests or implementations for planned work blurs intent and completion.
  - **description**:
    - |
      `planned` means the work is accepted but not yet delivered. Once a planned
      item already points to tests or implementations, the status and the
      evidence disagree, which makes reports and roadmap conversations harder to
      trust. This rule preserves the meaning of planning by keeping future work
      distinct from delivered work.
- **code**: implemented-trace-missing
  - **severity**: error
  - **title**: Implemented items must prove delivery
  - **summary**: A delivered requirement or feature needs explicit evidence in tests or code.
  - **description**:
    - |
      Marking something as implemented is a promise that the repository can
      already demonstrate it. Without traces, that promise becomes a belief
      rather than evidence. This rule exists to make implementation status
      reviewable and to keep status labels from drifting away from reality.
- **code**: missing-trace
  - **severity**: warning
  - **title**: Undefined delivery state leaves trace expectations unclear
  - **summary**: If status is omitted or invalid, absent traces become a warning rather than silent drift.
  - **description**:
    - |
      Sometimes a repository is partway through adoption and has not yet settled
      on a clean delivery state for every entry. In those cases `syu` still
      warns when no traces are declared so the gap remains visible. The warning
      preserves forward pressure toward explicit, reviewable delivery semantics.

## Source YAML

```yaml
genre: Delivery State
version: 1

rules:
  - code: invalid-status
    severity: error
    title: Delivery state values must be supported
    summary: Requirements and features need a status vocabulary the validator understands.
    description: |
      Delivery-state automation only works when the state machine is explicit and
      finite. If contributors invent ad hoc status names, `syu` can no longer
      tell whether traces should exist, be absent, or be considered complete.
      This rule keeps workflow semantics predictable by limiting the state
      vocabulary to supported values.

  - code: planned-status-disallowed
    severity: error
    title: Planned items can be forbidden by project policy
    summary: Some repositories want the spec to describe only already-delivered work.
    description: |
      Teams reach moments when backlog-style entries should no longer remain in
      the committed specification for a branch, release line, or audited
      workspace. This rule lets a project tighten that policy without rewriting
      the entire validation engine. The goal is to support both exploratory
      workflows and locked-down release workflows through explicit config.

  - code: planned-trace-present
    severity: error
    title: Planned items must not claim delivered traces
    summary: Declaring tests or implementations for planned work blurs intent and completion.
    description: |
      `planned` means the work is accepted but not yet delivered. Once a planned
      item already points to tests or implementations, the status and the
      evidence disagree, which makes reports and roadmap conversations harder to
      trust. This rule preserves the meaning of planning by keeping future work
      distinct from delivered work.

  - code: implemented-trace-missing
    severity: error
    title: Implemented items must prove delivery
    summary: A delivered requirement or feature needs explicit evidence in tests or code.
    description: |
      Marking something as implemented is a promise that the repository can
      already demonstrate it. Without traces, that promise becomes a belief
      rather than evidence. This rule exists to make implementation status
      reviewable and to keep status labels from drifting away from reality.

  - code: missing-trace
    severity: warning
    title: Undefined delivery state leaves trace expectations unclear
    summary: If status is omitted or invalid, absent traces become a warning rather than silent drift.
    description: |
      Sometimes a repository is partway through adoption and has not yet settled
      on a clean delivery state for every entry. In those cases `syu` still
      warns when no traces are declared so the gap remains visible. The warning
      preserves forward pressure toward explicit, reviewable delivery semantics.
```
