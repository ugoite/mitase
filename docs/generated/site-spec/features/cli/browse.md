---
title: "Interactive CLI / Browse"
description: "Generated reference for docs/syu/features/cli/browse.yaml"
---

> Generated from `docs/syu/features/cli/browse.yaml`.

## Parsed content

### Category

- Interactive CLI

### Version

- 1

### Features

- **id**: FEAT-BROWSE-001
  - **title**: Interactive specification browser
  - **summary**: Let users explore counts, definitions, links, and validation errors from the terminal without needing a passing workspace first.
  - **status**: implemented
  - **linked_requirements**:
    - REQ-CORE-015
  - **implementations**:
    - **rust**:
      - **file**: src/lib.rs
        - **symbols**:
          - *
      - **file**: src/cli.rs
        - **symbols**:
          - *
      - **file**: src/command/mod.rs
        - **symbols**:
          - *
      - **file**: src/command/browse.rs
        - **symbols**:
          - *
- **id**: FEAT-BROWSE-002
  - **title**: Non-interactive spec tree output
  - **summary**: Add a --non-interactive flag to `syu browse` that prints the spec tree (all kinds grouped with counts and IDs) to stdout and exits, suitable for CI and scripting.
  - **status**: implemented
  - **linked_requirements**:
    - REQ-CORE-015
  - **implementations**:
    - **rust**:
      - **file**: src/command/browse.rs
        - **symbols**:
          - run_browse_command
          - print_non_interactive
      - **file**: src/cli.rs
        - **symbols**:
          - BrowseArgs

## Source YAML

```yaml
category: Interactive CLI
version: 1

features:
  - id: FEAT-BROWSE-001
    title: Interactive specification browser
    summary: Let users explore counts, definitions, links, and validation errors from the terminal without needing a passing workspace first.
    status: implemented
    linked_requirements:
      - REQ-CORE-015
    implementations:
      rust:
        - file: src/lib.rs
          symbols:
            - "*"
        - file: src/cli.rs
          symbols:
            - "*"
        - file: src/command/mod.rs
          symbols:
            - "*"
        - file: src/command/browse.rs
          symbols:
            - "*"

  - id: FEAT-BROWSE-002
    title: Non-interactive spec tree output
    summary: Add a --non-interactive flag to `syu browse` that prints the spec tree (all kinds grouped with counts and IDs) to stdout and exits, suitable for CI and scripting.
    status: implemented
    linked_requirements:
      - REQ-CORE-015
    implementations:
      rust:
        - file: src/command/browse.rs
          symbols:
            - run_browse_command
            - print_non_interactive
        - file: src/cli.rs
          symbols:
            - BrowseArgs
```
