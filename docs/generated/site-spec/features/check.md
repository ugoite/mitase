---
title: "Validate Command / Check"
description: "Generated reference for docs/spec/features/check.yaml"
---

> Generated from `docs/spec/features/check.yaml`.

## Parsed content

### Category

- Validate Command

### Version

- 1

### Features

- **id**: FEAT-CHECK-001
  - **title**: Unified validation command
  - **summary**: Validate graph links, rule-backed diagnostics, trace ownership, optional strict coverage, and safe autofix with one command.
  - **status**: implemented
  - **linked_requirements**:
    - REQ-CORE-001
    - REQ-CORE-002
    - REQ-CORE-003
    - REQ-CORE-005
  - **implementations**:
    - **rust**:
      - **file**: src/command/check.rs
        - **symbols**:
          - *
      - **file**: src/config.rs
        - **symbols**:
          - *
      - **file**: src/coverage.rs
        - **symbols**:
          - *
      - **file**: src/inspect.rs
        - **symbols**:
          - *
      - **file**: src/language.rs
        - **symbols**:
          - *
      - **file**: src/model.rs
        - **symbols**:
          - *
      - **file**: src/rules.rs
        - **symbols**:
          - *
      - **file**: src/runtime.rs
        - **symbols**:
          - *
      - **file**: src/workspace.rs
        - **symbols**:
          - *
    - **yaml**:
      - **file**: syu.yaml
        - **symbols**:
          - FEAT-CHECK-001
          - require_non_orphaned_items
          - require_symbol_trace_coverage

## Source YAML

```yaml
category: Validate Command
version: 1

features:
  - id: FEAT-CHECK-001
    title: Unified validation command
    summary: Validate graph links, rule-backed diagnostics, trace ownership, optional strict coverage, and safe autofix with one command.
    status: implemented
    linked_requirements:
      - REQ-CORE-001
      - REQ-CORE-002
      - REQ-CORE-003
      - REQ-CORE-005
    implementations:
      rust:
        - file: src/command/check.rs
          symbols:
            - "*"
        - file: src/config.rs
          symbols:
            - "*"
        - file: src/coverage.rs
          symbols:
            - "*"
        - file: src/inspect.rs
          symbols:
            - "*"
        - file: src/language.rs
          symbols:
            - "*"
        - file: src/model.rs
          symbols:
            - "*"
        - file: src/rules.rs
          symbols:
            - "*"
        - file: src/runtime.rs
          symbols:
            - "*"
        - file: src/workspace.rs
          symbols:
            - "*"
      yaml:
        - file: syu.yaml
          symbols:
            - FEAT-CHECK-001
            - require_non_orphaned_items
            - require_symbol_trace_coverage
```
