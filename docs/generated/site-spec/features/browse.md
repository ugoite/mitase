---
title: "Interactive CLI / Browse"
description: "Generated reference for docs/spec/features/browse.yaml"
---

> Generated from `docs/spec/features/browse.yaml`.

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
```
