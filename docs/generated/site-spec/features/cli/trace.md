---
title: "Trace Lookup CLI / Trace"
description: "Generated reference for docs/syu/features/cli/trace.yaml"
---

> Generated from `docs/syu/features/cli/trace.yaml`.

## Parsed content

### Category

- Trace Lookup CLI

### Version

- 1

### Features

- **id**: FEAT-TRACE-001
  - **title**: Source-first trace lookup
  - **summary**: Start from a repository file path and optional symbol, then resolve linked requirements, features, policies, and philosophies from trace ownership.
  - **status**: implemented
  - **linked_requirements**:
    - REQ-CORE-021
  - **implementations**:
    - **rust**:
      - **file**: src/cli.rs
        - **symbols**:
          - TraceArgs
      - **file**: src/command/trace.rs
        - **symbols**:
          - *

## Source YAML

```yaml
category: Trace Lookup CLI
version: 1

features:
  - id: FEAT-TRACE-001
    title: Source-first trace lookup
    summary: Start from a repository file path and optional symbol, then resolve linked requirements, features, policies, and philosophies from trace ownership.
    status: implemented
    linked_requirements:
      - REQ-CORE-021
    implementations:
      rust:
        - file: src/cli.rs
          symbols:
            - TraceArgs
        - file: src/command/trace.rs
          symbols:
            - "*"
```
