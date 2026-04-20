---
title: "Git History CLI / Log"
description: "Generated reference for docs/syu/features/cli/log.yaml"
---

> Generated from `docs/syu/features/cli/log.yaml`.

## Parsed content

### Category

- Git History CLI

### Version

- 1

### Features

- **id**: FEAT-LOG-001
  - **title**: Trace-aware Git history lookup
  - **summary**: Show reviewer-friendly commit history behind one requirement or feature by projecting the current trace graph, related spec surface, and optional review range onto checked-in Git paths.
  - **status**: implemented
  - **linked_requirements**:
    - REQ-CORE-024
  - **implementations**:
    - **rust**:
      - **file**: src/command/log.rs
        - **symbols**:
          - run_log_command
          - build_history_target
          - load_git_history
          - parse_git_history
      - **file**: src/command/lookup.rs
        - **symbols**:
          - document_path_for_id
      - **file**: src/cli.rs
        - **symbols**:
          - HistoryKind
          - LogArgs
      - **file**: src/lib.rs
        - **symbols**:
          - dispatches_log_subcommands_without_rewriting_them

## Source YAML

```yaml
# FEAT-LOG-001

category: Git History CLI
version: 1

features:
  - id: FEAT-LOG-001
    title: Trace-aware Git history lookup
    summary: Show reviewer-friendly commit history behind one requirement or feature by projecting the current trace graph, related spec surface, and optional review range onto checked-in Git paths.
    status: implemented
    linked_requirements:
      - REQ-CORE-024
    implementations:
      rust:
        - file: src/command/log.rs
          symbols:
            - run_log_command
            - build_history_target
            - load_git_history
            - parse_git_history
        - file: src/command/lookup.rs
          symbols:
            - document_path_for_id
        - file: src/cli.rs
          symbols:
            - HistoryKind
            - LogArgs
        - file: src/lib.rs
          symbols:
            - dispatches_log_subcommands_without_rewriting_them
```
