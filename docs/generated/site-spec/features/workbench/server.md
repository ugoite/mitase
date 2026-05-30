---
title: "Workbench / Server"
description: "Generated reference for docs/syu/features/workbench/server.yaml"
---

> Generated from `docs/syu/features/workbench/server.yaml`.

## Parsed content

### Category

- Workbench

### Version

- 1

### Features

- **id**: FEAT-WORKBENCH-SERVER-001
  - **title**: Workbench server and event bus
  - **summary**: Serve the typed Workbench API from a Rust-native Axum server with local bind safety, filesystem reload events, and a thin CLI wrapper so browser and desktop clients can share one source of truth.
  - **status**: implemented
  - **linked_requirements**:
    - REQ-WORKBENCH-006
    - REQ-WORKBENCH-007
  - **implementations**:
    - **rust**:
      - **file**: crates/syu-workbench-server/src/lib.rs
        - **symbols**:
          - WorkbenchServer
          - WorkbenchLaunchConfig
          - WorkbenchEvent
          - WorkbenchHealth
          - WorkbenchActionCatalog
      - **file**: src/command/workbench.rs
        - **symbols**:
          - run_workbench_command
    - **markdown**:
      - **file**: docs/guide/workbench.md
        - **symbols**:
          - browser and desktop
          - server architecture
          - Workbench server

## Source YAML

```yaml
category: Workbench
version: 1

features:
  - id: FEAT-WORKBENCH-SERVER-001
    title: Workbench server and event bus
    summary: Serve the typed Workbench API from a Rust-native Axum server with local bind safety, filesystem reload events, and a thin CLI wrapper so browser and desktop clients can share one source of truth.
    status: implemented
    linked_requirements:
      - REQ-WORKBENCH-006
      - REQ-WORKBENCH-007
    implementations:
      rust:
        - file: crates/syu-workbench-server/src/lib.rs
          symbols:
            - WorkbenchServer
            - WorkbenchLaunchConfig
            - WorkbenchEvent
            - WorkbenchHealth
            - WorkbenchActionCatalog
        - file: src/command/workbench.rs
          symbols:
            - run_workbench_command
      markdown:
        - file: docs/guide/workbench.md
          symbols:
            - browser and desktop
            - server architecture
            - Workbench server
```
