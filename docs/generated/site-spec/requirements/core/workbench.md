---
title: "Core Workbench / Workbench"
description: "Generated reference for docs/syu/requirements/core/workbench.yaml"
---

> Generated from `docs/syu/requirements/core/workbench.yaml`.

## Parsed content

### Category

- Core Workbench

### Prefix

- REQ-WORKBENCH

### Requirements

- **id**: REQ-WORKBENCH-001
  - **title**: Command-palette-first Workbench
  - **description**:
    - |
      The Workbench MUST open around a command palette instead of a fixed tab
      strip. Users SHOULD be able to launch request, goal, scope, assignment,
      and evidence actions from the same keyboard-first surface, and the UI
      SHOULD keep the active goal centered in view.
  - **priority**: medium
  - **status**: implemented
  - **linked_policies**:
    - POL-005
  - **linked_features**:
    - FEAT-WORKBENCH-001
  - **tests**:
    - **markdown**:
      - **file**: docs/guide/workbench.md
        - **symbols**:
          - command-palette-first
          - goal-centered
          - browser and desktop
- **id**: REQ-WORKBENCH-002
  - **title**: Request, Goal, Evidence, and Assignment model
  - **description**:
    - |
      The Workbench MUST represent requests, goals, evidence, and assignments
      as explicit artifacts instead of hiding them inside a generic task list.
      The model SHOULD make it clear which request a goal came from, which
      evidence supports progress, and who is responsible for the current step.
  - **priority**: medium
  - **status**: implemented
  - **linked_policies**:
    - POL-005
  - **linked_features**:
    - FEAT-WORKBENCH-002
    - FEAT-WORKBENCH-003
    - FEAT-WORKBENCH-005
  - **tests**:
    - **markdown**:
      - **file**: docs/guide/workbench.md
        - **symbols**:
          - Request
          - Goal Plan
          - assignment
          - evidence
- **id**: REQ-WORKBENCH-003
  - **title**: Goal splitting for large change requests
  - **description**:
    - |
      The Workbench MUST help break large requests into smaller scoped goals
      before execution starts. It SHOULD preserve the parent request, keep the
      split goals reviewable, and let each goal carry its own temporary Goal
      Plan so delivery stays bounded.
  - **priority**: medium
  - **status**: implemented
  - **linked_policies**:
    - POL-005
  - **linked_features**:
    - FEAT-WORKBENCH-003
  - **tests**:
    - **markdown**:
      - **file**: docs/guide/workbench.md
        - **symbols**:
          - scaffold preview
          - Goal Plan
          - assignment
- **id**: REQ-WORKBENCH-004
  - **title**: Spec impact and branch scope visualization
  - **description**:
    - |
      The Workbench MUST show which specifications, files, and branch scope are
      likely to change before the user commits to implementation. It SHOULD
      make the impact of a request visible early enough that the user can
      refine scope before work starts.
  - **priority**: medium
  - **status**: implemented
  - **linked_policies**:
    - POL-005
  - **linked_features**:
    - FEAT-WORKBENCH-004
  - **tests**:
    - **markdown**:
      - **file**: docs/guide/workbench.md
        - **symbols**:
          - scope
          - branch scope
          - scaffold preview
- **id**: REQ-WORKBENCH-005
  - **title**: Human and AI assignment with explicit scope and evidence
  - **description**:
    - |
      The Workbench MUST support assigning a scoped goal to a human or AI with
      explicit scope, expected evidence, and a clear handoff boundary. The
      assignment SHOULD be readable as a durable part of the goal rather than a
      hidden runtime choice.
  - **priority**: medium
  - **status**: implemented
  - **linked_policies**:
    - POL-005
  - **linked_features**:
    - FEAT-WORKBENCH-005
    - FEAT-WORKBENCH-006
  - **tests**:
    - **markdown**:
      - **file**: docs/guide/workbench.md
        - **symbols**:
          - assignment
          - evidence
          - completion check
- **id**: REQ-WORKBENCH-006
  - **title**: Shared browser and desktop Workbench behavior
  - **description**:
    - |
      The Workbench MUST behave consistently in browser and desktop contexts.
      The same request, goal, assignment, and evidence flow SHOULD work in both
      environments so users do not have to learn two separate products.
  - **priority**: medium
  - **status**: implemented
  - **linked_policies**:
    - POL-005
  - **linked_features**:
    - FEAT-WORKBENCH-001
    - FEAT-WORKBENCH-007
  - **tests**:
    - **markdown**:
      - **file**: docs/guide/workbench.md
        - **symbols**:
          - browser and desktop
          - same way
- **id**: REQ-WORKBENCH-007
  - **title**: Rust-native UI and server architecture
  - **description**:
    - |
      The Workbench MUST use a Rust-native UI and server architecture so the
      product can share one source of truth for request intake, goal tracking,
      evidence capture, and assignment state. The implementation SHOULD keep
      the UI and server layers close enough that browser and desktop clients
      stay in sync.
  - **priority**: medium
  - **status**: implemented
  - **linked_policies**:
    - POL-005
  - **linked_features**:
    - FEAT-WORKBENCH-001
    - FEAT-WORKBENCH-007
  - **tests**:
    - **markdown**:
      - **file**: docs/guide/workbench.md
        - **symbols**:
          - Rust-native UI
          - server architecture
          - browser and desktop

## Source YAML

```yaml
category: Core Workbench
prefix: REQ-WORKBENCH
requirements:
  - id: REQ-WORKBENCH-001
    title: Command-palette-first Workbench
    description: |
      The Workbench MUST open around a command palette instead of a fixed tab
      strip. Users SHOULD be able to launch request, goal, scope, assignment,
      and evidence actions from the same keyboard-first surface, and the UI
      SHOULD keep the active goal centered in view.
    priority: medium
    status: implemented
    linked_policies:
      - POL-005
    linked_features:
      - FEAT-WORKBENCH-001
    tests:
      markdown:
        - file: docs/guide/workbench.md
          symbols:
            - command-palette-first
            - goal-centered
            - browser and desktop
  - id: REQ-WORKBENCH-002
    title: Request, Goal, Evidence, and Assignment model
    description: |
      The Workbench MUST represent requests, goals, evidence, and assignments
      as explicit artifacts instead of hiding them inside a generic task list.
      The model SHOULD make it clear which request a goal came from, which
      evidence supports progress, and who is responsible for the current step.
    priority: medium
    status: implemented
    linked_policies:
      - POL-005
    linked_features:
      - FEAT-WORKBENCH-002
      - FEAT-WORKBENCH-003
      - FEAT-WORKBENCH-005
    tests:
      markdown:
        - file: docs/guide/workbench.md
          symbols:
            - Request
            - Goal Plan
            - assignment
            - evidence
  - id: REQ-WORKBENCH-003
    title: Goal splitting for large change requests
    description: |
      The Workbench MUST help break large requests into smaller scoped goals
      before execution starts. It SHOULD preserve the parent request, keep the
      split goals reviewable, and let each goal carry its own temporary Goal
      Plan so delivery stays bounded.
    priority: medium
    status: implemented
    linked_policies:
      - POL-005
    linked_features:
      - FEAT-WORKBENCH-003
    tests:
      markdown:
        - file: docs/guide/workbench.md
          symbols:
            - scaffold preview
            - Goal Plan
            - assignment
  - id: REQ-WORKBENCH-004
    title: Spec impact and branch scope visualization
    description: |
      The Workbench MUST show which specifications, files, and branch scope are
      likely to change before the user commits to implementation. It SHOULD
      make the impact of a request visible early enough that the user can
      refine scope before work starts.
    priority: medium
    status: implemented
    linked_policies:
      - POL-005
    linked_features:
      - FEAT-WORKBENCH-004
    tests:
      markdown:
        - file: docs/guide/workbench.md
          symbols:
            - scope
            - branch scope
            - scaffold preview
  - id: REQ-WORKBENCH-005
    title: Human and AI assignment with explicit scope and evidence
    description: |
      The Workbench MUST support assigning a scoped goal to a human or AI with
      explicit scope, expected evidence, and a clear handoff boundary. The
      assignment SHOULD be readable as a durable part of the goal rather than a
      hidden runtime choice.
    priority: medium
    status: implemented
    linked_policies:
      - POL-005
    linked_features:
      - FEAT-WORKBENCH-005
      - FEAT-WORKBENCH-006
    tests:
      markdown:
        - file: docs/guide/workbench.md
          symbols:
            - assignment
            - evidence
            - completion check
  - id: REQ-WORKBENCH-006
    title: Shared browser and desktop Workbench behavior
    description: |
      The Workbench MUST behave consistently in browser and desktop contexts.
      The same request, goal, assignment, and evidence flow SHOULD work in both
      environments so users do not have to learn two separate products.
    priority: medium
    status: implemented
    linked_policies:
      - POL-005
    linked_features:
      - FEAT-WORKBENCH-001
      - FEAT-WORKBENCH-007
    tests:
      markdown:
        - file: docs/guide/workbench.md
          symbols:
            - browser and desktop
            - same way
  - id: REQ-WORKBENCH-007
    title: Rust-native UI and server architecture
    description: |
      The Workbench MUST use a Rust-native UI and server architecture so the
      product can share one source of truth for request intake, goal tracking,
      evidence capture, and assignment state. The implementation SHOULD keep
      the UI and server layers close enough that browser and desktop clients
      stay in sync.
    priority: medium
    status: implemented
    linked_policies:
      - POL-005
    linked_features:
      - FEAT-WORKBENCH-001
      - FEAT-WORKBENCH-007
    tests:
      markdown:
        - file: docs/guide/workbench.md
          symbols:
            - Rust-native UI
            - server architecture
            - browser and desktop
```
