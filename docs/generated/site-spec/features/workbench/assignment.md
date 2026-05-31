---
title: "Workbench / Assignment"
description: "Generated reference for docs/syu/features/workbench/assignment.yaml"
---

> Generated from `docs/syu/features/workbench/assignment.yaml`.

## Parsed content

### Category

- Workbench

### Version

- 1

### Features

- **id**: FEAT-WORKBENCH-ASSIGNMENT-001
  - **title**: Explicit human and AI assignment
  - **summary**: Assign a scoped Goal Plan to a human or command-adapter AI runner with visible scope, blockers, dry-run output, and evidence capture.
  - **status**: implemented
  - **linked_requirements**:
    - REQ-WORKBENCH-005
  - **implementations**:
    - **rust**:
      - **file**: crates/syu-workbench/src/lib.rs
        - **symbols**:
          - Assignment
          - ScopeGuard
          - CommandAgentAdapter
          - EvidenceCollector
      - **file**: crates/syu-app-ui/src/components/shell.rs
        - **symbols**:
          - AssignGoalDialog
          - ScopeGuardPreview
          - AgentRunPanel
      - **file**: crates/syu-workbench/src/lib.rs
        - **symbols**:
          - assignment_blocker_logic_rejects_ambiguous_ai_scope
          - dry_run_command_adapter_captures_stdout_stderr_and_evidence
      - **file**: tests/workbench_smoke.rs
        - **symbols**:
          - assignment_preview_renders_blocked_state_with_scope_tokens
          - assignment_actions_are_exposed_in_the_command_palette
    - **markdown**:
      - **file**: docs/guide/workbench.md
        - **symbols**:
          - assignment
          - evidence
          - human or AI

## Source YAML

```yaml
category: Workbench
version: 1

features:
  - id: FEAT-WORKBENCH-ASSIGNMENT-001
    title: Explicit human and AI assignment
    summary: Assign a scoped Goal Plan to a human or command-adapter AI runner with visible scope, blockers, dry-run output, and evidence capture.
    status: implemented
    linked_requirements:
      - REQ-WORKBENCH-005
    implementations:
      rust:
        - file: crates/syu-workbench/src/lib.rs
          symbols:
            - Assignment
            - ScopeGuard
            - CommandAgentAdapter
            - EvidenceCollector
        - file: crates/syu-app-ui/src/components/shell.rs
          symbols:
            - AssignGoalDialog
            - ScopeGuardPreview
            - AgentRunPanel
        - file: crates/syu-workbench/src/lib.rs
          symbols:
            - assignment_blocker_logic_rejects_ambiguous_ai_scope
            - dry_run_command_adapter_captures_stdout_stderr_and_evidence
        - file: tests/workbench_smoke.rs
          symbols:
            - assignment_preview_renders_blocked_state_with_scope_tokens
            - assignment_actions_are_exposed_in_the_command_palette
      markdown:
        - file: docs/guide/workbench.md
          symbols:
            - assignment
            - evidence
            - human or AI
```
