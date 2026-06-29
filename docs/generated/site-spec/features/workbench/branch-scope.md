---
title: "Workbench / Branch Scope"
description: "Generated reference for docs/syu/features/workbench/branch-scope.yaml"
---

> Generated from `docs/syu/features/workbench/branch-scope.yaml`.

## Parsed content

### Category

- Workbench

### Version

- 1

### Features

- **id**: FEAT-WORKBENCH-SPEC-GRAPH-001
  - **title**: Spec Impact Graph
  - **summary**: Render a simple typed graph that connects philosophy, policy, requirement, feature, changed file or symbol, and test nodes so request and branch impact can be inspected without a separate graph engine.
  - **status**: implemented
  - **linked_requirements**:
    - REQ-WORKBENCH-004
  - **implementations**:
    - **rust**:
      - **file**: crates/syu-code-intel/src/branch_scope.rs
        - **symbols**:
          - SpecImpactGraphReport
          - SpecImpactGraphNode
          - SpecImpactGraphEdge
          - BranchScopeReport
      - **file**: crates/syu-app-ui/src/components/pages/scope.rs
        - **symbols**:
          - ScopePage
          - SliceDetail
    - **markdown**:
      - **file**: docs/guide/workbench.md
        - **symbols**:
          - Scope
          - Implementation Slices
- **id**: FEAT-WORKBENCH-BRANCH-SCOPE-001
  - **title**: Branch Scope Lens
  - **summary**: Show branch range, changed files, traced ownership, affected specs, out-of-scope changes, goal split suggestions, tests, and strict review status from typed Workbench Branch Scope data.
  - **status**: implemented
  - **linked_requirements**:
    - REQ-WORKBENCH-004
  - **implementations**:
    - **rust**:
      - **file**: crates/syu-code-intel/src/branch_scope.rs
        - **symbols**:
          - BranchScopeReport
          - BranchScopeEvidence
          - SpecImpactReport
      - **file**: crates/syu-workbench/src/lib.rs
        - **symbols**:
          - WorkbenchActionId
          - WorkbenchEvidenceKind
          - BranchScopeState
      - **file**: crates/syu-app-ui/src/model/scope.rs
        - **symbols**:
          - ImplementationSlice
          - implementation_slices
      - **file**: crates/syu-app-ui/src/components/pages/scope.rs
        - **symbols**:
          - ScopePage
          - SliceDetail
    - **markdown**:
      - **file**: docs/guide/workbench.md
        - **symbols**:
          - Scope
          - Implementation Slices
- **id**: FEAT-WORKBENCH-004
  - **title**: Spec impact and branch scope view
  - **summary**: Show the likely spec, file, and branch scope impact of a request before implementation begins so the user can refine scope early.
  - **status**: implemented
  - **linked_requirements**:
    - REQ-WORKBENCH-004
  - **implementations**:
    - **markdown**:
      - **file**: docs/guide/workbench.md
        - **symbols**:
          - Scope
          - Implementation Slices

## Source YAML

```yaml
category: Workbench
version: 1

features:
  - id: FEAT-WORKBENCH-SPEC-GRAPH-001
    title: Spec Impact Graph
    summary: Render a simple typed graph that connects philosophy, policy, requirement, feature, changed file or symbol, and test nodes so request and branch impact can be inspected without a separate graph engine.
    status: implemented
    linked_requirements:
      - REQ-WORKBENCH-004
    implementations:
      rust:
        - file: crates/syu-code-intel/src/branch_scope.rs
          symbols:
            - SpecImpactGraphReport
            - SpecImpactGraphNode
            - SpecImpactGraphEdge
            - BranchScopeReport
        - file: crates/syu-app-ui/src/components/pages/scope.rs
          symbols:
            - ScopePage
            - SliceDetail
      markdown:
        - file: docs/guide/workbench.md
          symbols:
            - Scope
            - Implementation Slices
  - id: FEAT-WORKBENCH-BRANCH-SCOPE-001
    title: Branch Scope Lens
    summary: Show branch range, changed files, traced ownership, affected specs, out-of-scope changes, goal split suggestions, tests, and strict review status from typed Workbench Branch Scope data.
    status: implemented
    linked_requirements:
      - REQ-WORKBENCH-004
    implementations:
      rust:
        - file: crates/syu-code-intel/src/branch_scope.rs
          symbols:
            - BranchScopeReport
            - BranchScopeEvidence
            - SpecImpactReport
        - file: crates/syu-workbench/src/lib.rs
          symbols:
            - WorkbenchActionId
            - WorkbenchEvidenceKind
            - BranchScopeState
        - file: crates/syu-app-ui/src/model/scope.rs
          symbols:
            - ImplementationSlice
            - implementation_slices
        - file: crates/syu-app-ui/src/components/pages/scope.rs
          symbols:
            - ScopePage
            - SliceDetail
      markdown:
        - file: docs/guide/workbench.md
          symbols:
            - Scope
            - Implementation Slices
  - id: FEAT-WORKBENCH-004
    title: Spec impact and branch scope view
    summary: Show the likely spec, file, and branch scope impact of a request before implementation begins so the user can refine scope early.
    status: implemented
    linked_requirements:
      - REQ-WORKBENCH-004
    implementations:
      markdown:
        - file: docs/guide/workbench.md
          symbols:
            - Scope
            - Implementation Slices
```
