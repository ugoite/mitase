---
title: "Workbench / Design Tokens"
description: "Generated reference for docs/syu/features/workbench/design-tokens.yaml"
---

> Generated from `docs/syu/features/workbench/design-tokens.yaml`.

## Parsed content

### Category

- Workbench

### Version

- 1

### Features

- **id**: FEAT-WORKBENCH-DESIGN-TOKENS-001
  - **title**: Workbench design tokens and Tailwind shell
  - **summary**: Define Workbench-specific design tokens and centralized class constants for the Dioxus UI crate so the shell can be visually intentional from the first implementation while keeping Tailwind constrained to the UI layer.
  - **status**: implemented
  - **linked_requirements**:
    - REQ-WORKBENCH-001
    - REQ-WORKBENCH-002
  - **implementations**:
    - **rust**:
      - **file**: crates/syu-app-ui/src/design/tokens.rs
        - **symbols**:
          - BACKGROUND
          - FOREGROUND
          - PANEL
          - BORDER
          - COMMAND
          - COMMAND_ACTIVE
          - GOAL
          - GOAL_ACTIVE
          - SPEC_LINKED
          - CODE_LINKED
          - TEST_LINKED
          - SCOPE_IN
          - SCOPE_OUT
          - SCOPE_AMBIGUOUS
          - OWNERSHIP_KNOWN
          - OWNERSHIP_MISSING
          - OWNERSHIP_AMBIGUOUS
          - EVIDENCE_PASS
          - EVIDENCE_WARN
          - EVIDENCE_FAIL
          - EVIDENCE_PENDING
      - **file**: crates/syu-app-ui/src/design/classes.rs
        - **symbols**:
          - APP_SHELL
          - PANEL
          - COMMAND_ITEM
          - EVIDENCE_CARD
          - EMPTY_STATE
      - **file**: tests/workbench_smoke.rs
        - **symbols**:
          - app_shell_renders_command_palette_first_shell
          - goal_canvas_renders_a_read_only_action_preview_placeholder

## Source YAML

```yaml
category: Workbench
version: 1

features:
  - id: FEAT-WORKBENCH-DESIGN-TOKENS-001
    title: Workbench design tokens and Tailwind shell
    summary: Define Workbench-specific design tokens and centralized class constants for the Dioxus UI crate so the shell can be visually intentional from the first implementation while keeping Tailwind constrained to the UI layer.
    status: implemented
    linked_requirements:
      - REQ-WORKBENCH-001
      - REQ-WORKBENCH-002
    implementations:
      rust:
        - file: crates/syu-app-ui/src/design/tokens.rs
          symbols:
            - BACKGROUND
            - FOREGROUND
            - PANEL
            - BORDER
            - COMMAND
            - COMMAND_ACTIVE
            - GOAL
            - GOAL_ACTIVE
            - SPEC_LINKED
            - CODE_LINKED
            - TEST_LINKED
            - SCOPE_IN
            - SCOPE_OUT
            - SCOPE_AMBIGUOUS
            - OWNERSHIP_KNOWN
            - OWNERSHIP_MISSING
            - OWNERSHIP_AMBIGUOUS
            - EVIDENCE_PASS
            - EVIDENCE_WARN
            - EVIDENCE_FAIL
            - EVIDENCE_PENDING
        - file: crates/syu-app-ui/src/design/classes.rs
          symbols:
            - APP_SHELL
            - PANEL
            - COMMAND_ITEM
            - EVIDENCE_CARD
            - EMPTY_STATE
        - file: tests/workbench_smoke.rs
          symbols:
            - app_shell_renders_command_palette_first_shell
            - goal_canvas_renders_a_read_only_action_preview_placeholder
```
