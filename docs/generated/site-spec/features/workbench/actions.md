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

- **id**: FEAT-WORKBENCH-COMMAND-PALETTE-001
  - **title**: Workbench command palette registry
  - **summary**: Keep request, goal, evidence, and assignment actions explicit so users can see which artifact is being created, updated, or handed off, and drive them from a typed registry instead of hardcoded UI flow.
  - **status**: implemented
  - **linked_requirements**:
    - REQ-WORKBENCH-002
  - **implementations**:
    - **rust**:
      - **file**: crates/syu-workbench/src/lib.rs
        - **symbols**:
          - WorkbenchAction
          - WorkbenchActionAvailability
          - WorkbenchActionContext
          - WorkbenchActionId
          - WorkbenchActionInput
          - WorkbenchActionRegistry
          - WorkbenchActionResult
          - WorkbenchState
          - CommandPaletteState
      - **file**: crates/syu-app-ui/src/model.rs
        - **symbols**:
          - WorkbenchUiState
          - CommandPaletteEntry
      - **file**: crates/syu-app-ui/src/components/shell.rs
        - **symbols**:
          - CommandPalette
          - GoalCanvas
      - **file**: tests/workbench_smoke.rs
        - **symbols**:
          - filters_actions_by_query
          - read_only_action_returns_placeholder_preview
          - registry_loaded_from_server_payload
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
          - command palette registry
          - WorkbenchState
          - request.scope
          - goal.test_select
          - branch.scope
          - branch.infer_goal
          - spec.impact
          - trace.range
          - relate.range

## Source YAML

```yaml
category: Workbench
version: 1

features:
  - id: FEAT-WORKBENCH-COMMAND-PALETTE-001
    title: Workbench command palette registry
    summary: Keep request, goal, evidence, and assignment actions explicit so users can see which artifact is being created, updated, or handed off, and drive them from a typed registry instead of hardcoded UI flow.
    status: implemented
    linked_requirements:
      - REQ-WORKBENCH-002
    implementations:
      rust:
        - file: crates/syu-workbench/src/lib.rs
          symbols:
            - WorkbenchAction
            - WorkbenchActionAvailability
            - WorkbenchActionContext
            - WorkbenchActionId
            - WorkbenchActionInput
            - WorkbenchActionRegistry
            - WorkbenchActionResult
            - WorkbenchState
            - CommandPaletteState
        - file: crates/syu-app-ui/src/model.rs
          symbols:
            - WorkbenchUiState
            - CommandPaletteEntry
        - file: crates/syu-app-ui/src/components/shell.rs
          symbols:
            - CommandPalette
            - GoalCanvas
        - file: tests/workbench_smoke.rs
          symbols:
            - filters_actions_by_query
            - read_only_action_returns_placeholder_preview
            - registry_loaded_from_server_payload
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
            - command palette registry
            - WorkbenchState
            - request.scope
            - goal.test_select
            - branch.scope
            - branch.infer_goal
            - spec.impact
            - trace.range
            - relate.range
```
