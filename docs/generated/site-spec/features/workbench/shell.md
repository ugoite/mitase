---
title: "Workbench / Shell"
description: "Generated reference for docs/syu/features/workbench/shell.yaml"
---

> Generated from `docs/syu/features/workbench/shell.yaml`.

## Parsed content

### Category

- Workbench

### Version

- 1

### Features

- **id**: FEAT-WORKBENCH-SHELL-001
  - **title**: Command-palette Workbench shell
  - **summary**: Open the Workbench around a command palette and goal-centered surface that launches request, goal, scope, assignment, and evidence actions without a fixed tab strip, using the typed Workbench state machine and action registry as the source of truth.
  - **status**: implemented
  - **linked_requirements**:
    - REQ-WORKBENCH-001
    - REQ-WORKBENCH-006
    - REQ-WORKBENCH-007
  - **implementations**:
    - **rust**:
      - **file**: crates/syu-workbench/src/lib.rs
        - **symbols**:
          - WorkbenchState
          - CommandPaletteState
          - WorkbenchActionRegistry
          - WorkbenchActionAvailability
    - **markdown**:
      - **file**: docs/guide/workbench.md
        - **symbols**:
          - command palette registry
          - WorkbenchState
          - CommandPaletteState
          - request.classify

## Source YAML

```yaml
category: Workbench
version: 1

features:
  - id: FEAT-WORKBENCH-SHELL-001
    title: Command-palette Workbench shell
    summary: Open the Workbench around a command palette and goal-centered surface that launches request, goal, scope, assignment, and evidence actions without a fixed tab strip, using the typed Workbench state machine and action registry as the source of truth.
    status: implemented
    linked_requirements:
      - REQ-WORKBENCH-001
      - REQ-WORKBENCH-006
      - REQ-WORKBENCH-007
    implementations:
      rust:
        - file: crates/syu-workbench/src/lib.rs
          symbols:
            - WorkbenchState
            - CommandPaletteState
            - WorkbenchActionRegistry
            - WorkbenchActionAvailability
      markdown:
        - file: docs/guide/workbench.md
          symbols:
            - command palette registry
            - WorkbenchState
            - CommandPaletteState
            - request.classify
```
