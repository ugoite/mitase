---
title: "Relation Inspection CLI / Relate"
description: "Generated reference for docs/syu/features/cli/relate.yaml"
---

> Generated from `docs/syu/features/cli/relate.yaml`.

## Parsed content

### Category

- Relation Inspection CLI

### Version

- 1

### Features

- **id**: FEAT-RELATE-001
  - **title**: Cross-layer relation inspection
  - **summary**: Follow one ID, path, or traced source symbol through the connected specification graph, including evidence and suspicious gaps.
  - **status**: implemented
  - **linked_requirements**:
    - REQ-CORE-023
  - **implementations**:
    - **rust**:
      - **file**: src/command/relate.rs
        - **symbols**:
          - run_relate_command
          - build_relation_report
          - resolve_selection
          - expand_related_ids
          - collect_related_traces
          - collect_gaps
      - **file**: src/cli.rs
        - **symbols**:
          - RelateArgs
      - **file**: src/lib.rs
        - **symbols**:
          - dispatches_relate_subcommands_without_rewriting_them

## Source YAML

```yaml
# FEAT-RELATE-001

category: Relation Inspection CLI
version: 1

features:
  - id: FEAT-RELATE-001
    title: Cross-layer relation inspection
    summary: Follow one ID, path, or traced source symbol through the connected specification graph, including evidence and suspicious gaps.
    status: implemented
    linked_requirements:
      - REQ-CORE-023
    implementations:
      rust:
        - file: src/command/relate.rs
          symbols:
            - run_relate_command
            - build_relation_report
            - resolve_selection
            - expand_related_ids
            - collect_related_traces
            - collect_gaps
        - file: src/cli.rs
          symbols:
            - RelateArgs
        - file: src/lib.rs
          symbols:
            - dispatches_relate_subcommands_without_rewriting_them
```
