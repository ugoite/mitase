---
title: "Audit CLI / Audit"
description: "Generated reference for docs/syu/features/cli/audit.yaml"
---

> Generated from `docs/syu/features/cli/audit.yaml`.

## Parsed content

### Category

- Audit CLI

### Version

- 1

### Features

- **id**: FEAT-AUDIT-001
  - **title**: Heuristic four-layer consistency audit
  - **summary**: Surface review-friendly overlap, tension, and orphaned-policy candidates from the checked-in spec without turning heuristics into hard validation failures.
  - **status**: implemented
  - **linked_requirements**:
    - REQ-CORE-025
  - **implementations**:
    - **rust**:
      - **file**: src/command/audit.rs
        - **symbols**:
          - *
      - **file**: src/cli.rs
        - **symbols**:
          - AuditArgs
      - **file**: src/lib.rs
        - **symbols**:
          - dispatches_audit_subcommands_without_rewriting_them

## Source YAML

```yaml
# FEAT-AUDIT-001

category: Audit CLI
version: 1

features:
  - id: FEAT-AUDIT-001
    title: Heuristic four-layer consistency audit
    summary: Surface review-friendly overlap, tension, and orphaned-policy candidates from the checked-in spec without turning heuristics into hard validation failures.
    status: implemented
    linked_requirements:
      - REQ-CORE-025
    implementations:
      rust:
        - file: src/command/audit.rs
          symbols:
            - '*'
        - file: src/cli.rs
          symbols:
            - AuditArgs
        - file: src/lib.rs
          symbols:
            - dispatches_audit_subcommands_without_rewriting_them
```
