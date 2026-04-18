---
title: "Lookup Search CLI / Search"
description: "Generated reference for docs/syu/features/cli/search.yaml"
---

> Generated from `docs/syu/features/cli/search.yaml`.

## Parsed content

### Category

- Lookup Search CLI

### Version

- 1

### Features

- **id**: FEAT-SEARCH-001
  - **title**: Terminal-first definition search
  - **summary**: Match spec items by ID, title, summary, or description from the CLI with optional kind scoping, JSON output, and workspace discovery from child directories.
  - **status**: implemented
  - **linked_requirements**:
    - REQ-CORE-019
  - **implementations**:
    - **rust**:
      - **file**: src/command/search.rs
        - **symbols**:
          - run_search_command
      - **file**: src/command/lookup.rs
        - **symbols**:
          - search
          - extend_search_results
          - field_matches_query
      - **file**: src/cli.rs
        - **symbols**:
          - SearchArgs

## Source YAML

```yaml
category: Lookup Search CLI
version: 1

features:
  - id: FEAT-SEARCH-001
    title: Terminal-first definition search
    summary: Match spec items by ID, title, summary, or description from the CLI with optional kind scoping, JSON output, and workspace discovery from child directories.
    status: implemented
    linked_requirements:
      - REQ-CORE-019
    implementations:
      rust:
        - file: src/command/search.rs
          symbols:
            - run_search_command
        - file: src/command/lookup.rs
          symbols:
            - search
            - extend_search_results
            - field_matches_query
        - file: src/cli.rs
          symbols:
            - SearchArgs
```
