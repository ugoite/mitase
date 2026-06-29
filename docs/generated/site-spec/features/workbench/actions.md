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
  - **title**: Page-targeted Workbench command palette
  - **summary**: Keep typed action availability while resolving each palette entry to a page, section, entity, stable component anchor, focus intent, and safe execution policy instead of a generic result surface.
  - **status**: implemented
  - **linked_requirements**:
    - REQ-WORKBENCH-002
  - **implementations**:
    - **rust**:
      - **file**: crates/syu-workbench/src/lib.rs
        - **symbols**:
          - WorkbenchAction
          - WorkbenchActionAvailability
          - WorkbenchActionRegistry
          - WorkbenchActionResult
      - **file**: crates/syu-app-ui/src/model/navigation.rs
        - **symbols**:
          - CommandTarget
          - CommandExecution
          - target_for_command
          - target_for_action
          - every_palette_command_has_one_page_target
      - **file**: crates/syu-app-ui/src/components/shell.rs
        - **symbols**:
          - CommandPalette
      - **file**: crates/syu-workbench-server/src/browser.rs
        - **symbols**:
          - render_workbench
          - workbench_document
    - **markdown**:
      - **file**: docs/guide/workbench.md
        - **symbols**:
          - CommandTarget navigation
          - Live state and APIs

## Source YAML

```yaml
category: Workbench
version: 1

features:
  - id: FEAT-WORKBENCH-COMMAND-PALETTE-001
    title: Page-targeted Workbench command palette
    summary: Keep typed action availability while resolving each palette entry to a page, section, entity, stable component anchor, focus intent, and safe execution policy instead of a generic result surface.
    status: implemented
    linked_requirements:
      - REQ-WORKBENCH-002
    implementations:
      rust:
        - file: crates/syu-workbench/src/lib.rs
          symbols:
            - WorkbenchAction
            - WorkbenchActionAvailability
            - WorkbenchActionRegistry
            - WorkbenchActionResult
        - file: crates/syu-app-ui/src/model/navigation.rs
          symbols:
            - CommandTarget
            - CommandExecution
            - target_for_command
            - target_for_action
            - every_palette_command_has_one_page_target
        - file: crates/syu-app-ui/src/components/shell.rs
          symbols:
            - CommandPalette
        - file: crates/syu-workbench-server/src/browser.rs
          symbols:
            - render_workbench
            - workbench_document
      markdown:
        - file: docs/guide/workbench.md
          symbols:
            - CommandTarget navigation
            - Live state and APIs
```
