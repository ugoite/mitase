---
title: "Init Command / Init"
description: "Generated reference for docs/syu/features/init.yaml"
---

> Generated from `docs/syu/features/init.yaml`.

## Parsed content

### Category

- Init Command

### Version

- 1

### Features

- **id**: FEAT-INIT-001
  - **title**: Workspace bootstrap
  - **summary**: Scaffold version-matched syu.yaml and a valid starter spec tree with planned entries.
  - **status**: implemented
  - **linked_requirements**:
    - REQ-CORE-009
  - **implementations**:
    - **rust**:
      - **file**: src/command/init.rs
        - **symbols**:
          - run_init_command
          - scaffold_files
      - **file**: src/config.rs
        - **symbols**:
          - render_config
          - current_cli_version

## Source YAML

```yaml
category: Init Command
version: 1

features:
  - id: FEAT-INIT-001
    title: Workspace bootstrap
    summary: Scaffold version-matched syu.yaml and a valid starter spec tree with planned entries.
    status: implemented
    linked_requirements:
      - REQ-CORE-009
    implementations:
      rust:
        - file: src/command/init.rs
          symbols:
            - run_init_command
            - scaffold_files
        - file: src/config.rs
          symbols:
            - render_config
            - current_cli_version
```
