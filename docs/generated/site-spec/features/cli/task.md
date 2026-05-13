---
title: "Task Planning CLI / Task"
description: "Generated reference for docs/syu/features/cli/task.yaml"
---

> Generated from `docs/syu/features/cli/task.yaml`.

## Parsed content

### Category

- Task Planning CLI

### Version

- 1

### Features

- **id**: FEAT-TASK-001
  - **title**: Request artifact classification
  - **summary**: Classify captured request artifacts into requirement create, change, or delete decisions using the current spec graph, with a short explanation and text or JSON output.
  - **status**: implemented
  - **linked_requirements**:
    - REQ-CORE-028
  - **implementations**:
    - **rust**:
      - **file**: src/command/task.rs
        - **symbols**:
          - run_task_command
          - run_task_classify_command
      - **file**: src/cli.rs
        - **symbols**:
          - TaskArgs
          - TaskClassifyArgs
      - **file**: src/lib.rs
        - **symbols**:
          - dispatches_task_subcommands_without_rewriting_them
    - **markdown**:
      - **file**: docs/guide/request-artifact-format.md
        - **symbols**:
          - syu task classify

## Source YAML

```yaml
category: Task Planning CLI
version: 1

features:
  - id: FEAT-TASK-001
    title: Request artifact classification
    summary: Classify captured request artifacts into requirement create, change, or delete decisions using the current spec graph, with a short explanation and text or JSON output.
    status: implemented
    linked_requirements:
      - REQ-CORE-028
    implementations:
      rust:
        - file: src/command/task.rs
          symbols:
            - run_task_command
            - run_task_classify_command
        - file: src/cli.rs
          symbols:
            - TaskArgs
            - TaskClassifyArgs
        - file: src/lib.rs
          symbols:
            - dispatches_task_subcommands_without_rewriting_them
      markdown:
        - file: docs/guide/request-artifact-format.md
          symbols:
            - syu task classify
```
