---
title: "Explain CLI / Explain"
description: "Generated reference for docs/syu/features/cli/explain.yaml"
---

> Generated from `docs/syu/features/cli/explain.yaml`.

## Parsed content

### Category

- Explain CLI

### Version

- 1

### Features

- **id**: FEAT-EXPLAIN-001
  - **title**: Focused explainability for one selector
  - **summary**: Turn one spec ID, repository path, or traced symbol into a guided assessment with the connected chain, traces, and obvious gaps.
  - **status**: implemented
  - **linked_requirements**:
    - REQ-CORE-027
  - **implementations**:
    - **rust**:
      - **file**: src/command/explain.rs
        - **symbols**:
          - run_explain_command
      - **file**: src/cli.rs
        - **symbols**:
          - ExplainArgs
      - **file**: src/lib.rs
        - **symbols**:
          - dispatches_lookup_subcommands_without_rewriting_them

## Source YAML

```yaml
# FEAT-EXPLAIN-001

category: Explain CLI
version: 1

features:
  - id: FEAT-EXPLAIN-001
    title: Focused explainability for one selector
    summary: Turn one spec ID, repository path, or traced symbol into a guided assessment with the connected chain, traces, and obvious gaps.
    status: implemented
    linked_requirements:
      - REQ-CORE-027
    implementations:
      rust:
        - file: src/command/explain.rs
          symbols:
            - run_explain_command
        - file: src/cli.rs
          symbols:
            - ExplainArgs
        - file: src/lib.rs
          symbols:
            - dispatches_lookup_subcommands_without_rewriting_them
```
