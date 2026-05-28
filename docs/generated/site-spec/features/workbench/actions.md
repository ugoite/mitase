---
title: "Workbench / Actions"
description: "Generated reference for docs/syu/features/workbench/actions.yaml"
---

> Generated from `docs/syu/features/workbench/actions.yaml`.

## Parsed content

### Category

- Workbench

### Version

- 1

### Features

- **id**: FEAT-WORKBENCH-002
  - **title**: Workbench action model
  - **summary**: Keep request, goal, evidence, and assignment actions explicit so users can see which artifact is being created, updated, or handed off.
  - **status**: implemented
  - **linked_requirements**:
    - REQ-WORKBENCH-002
  - **implementations**:
    - **rust**:
      - **file**: crates/syu-actions/src/lib.rs
        - **symbols**:
          - classify_request
          - scope_request
          - scaffold_request
          - generate_goal_plan
          - infer_goal_plan_from_diff
          - select_goal_tests
          - check_goal_plan
          - validate_workspace
          - browse_workspace
          - list_items
          - show_item
          - search_items
          - audit_workspace
          - trace_selector
          - trace_range
          - relate_selector
          - relate_range
          - explain_selector
          - history_for_item
          - doctor_workspace
    - **markdown**:
      - **file**: docs/guide/workbench.md
        - **symbols**:
          - Request
          - Goal Plan
          - assignment
          - evidence

## Source YAML

```yaml
category: Workbench
version: 1

features:
  - id: FEAT-WORKBENCH-002
    title: Workbench action model
    summary: Keep request, goal, evidence, and assignment actions explicit so users can see which artifact is being created, updated, or handed off.
    status: implemented
    linked_requirements:
      - REQ-WORKBENCH-002
    implementations:
      rust:
        - file: crates/syu-actions/src/lib.rs
          symbols:
            - classify_request
            - scope_request
            - scaffold_request
            - generate_goal_plan
            - infer_goal_plan_from_diff
            - select_goal_tests
            - check_goal_plan
            - validate_workspace
            - browse_workspace
            - list_items
            - show_item
            - search_items
            - audit_workspace
            - trace_selector
            - trace_range
            - relate_selector
            - relate_range
            - explain_selector
            - history_for_item
            - doctor_workspace
      markdown:
        - file: docs/guide/workbench.md
          symbols:
            - Request
            - Goal Plan
            - assignment
            - evidence
```
