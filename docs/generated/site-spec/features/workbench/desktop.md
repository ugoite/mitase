---
title: "Workbench / Desktop"
description: "Generated reference for docs/syu/features/workbench/desktop.yaml"
---

> Generated from `docs/syu/features/workbench/desktop.yaml`.

## Parsed content

### Category

- Workbench

### Version

- 1

### Features

- **id**: FEAT-WORKBENCH-TAURI-001
  - **title**: Tauri shell for the shared Workbench
  - **summary**: Provide a Tauri 2 desktop target that launches against the local Workbench server, renders the shared Dioxus Workbench UI crate, uses the shared Tailwind asset, and exposes desktop commands only as bridges to typed Workbench actions.
  - **status**: implemented
  - **linked_requirements**:
    - REQ-WORKBENCH-006
  - **implementations**:
    - **rust**:
      - **file**: crates/syu-desktop/src/lib.rs
        - **symbols**:
          - DesktopLaunchConfig
          - desktop_action_bridge
          - render_shared_workbench_shell
      - **file**: crates/syu-desktop/src/main.rs
        - **symbols**:
          - main
    - **json**:
      - **file**: crates/syu-desktop/tauri.conf.json
        - **symbols**:
          - *
- **id**: FEAT-WORKBENCH-007
  - **title**: Shared browser and desktop Workbench
  - **summary**: Keep the browser and desktop Workbench aligned through a Rust-native UI and server architecture that share one request, goal, assignment, and evidence model.
  - **status**: implemented
  - **linked_requirements**:
    - REQ-WORKBENCH-006
    - REQ-WORKBENCH-007
  - **implementations**:
    - **rust**:
      - **file**: crates/syu-workbench-server/src/lib.rs
        - **symbols**:
          - workbench_index
          - workbench_css
    - **markdown**:
      - **file**: docs/guide/workbench.md
        - **symbols**:
          - browser and desktop
          - Rust-native UI
          - server architecture

## Source YAML

```yaml
category: Workbench
version: 1

features:
  - id: FEAT-WORKBENCH-TAURI-001
    title: Tauri shell for the shared Workbench
    summary: Provide a Tauri 2 desktop target that launches against the local Workbench server, renders the shared Dioxus Workbench UI crate, uses the shared Tailwind asset, and exposes desktop commands only as bridges to typed Workbench actions.
    status: implemented
    linked_requirements:
      - REQ-WORKBENCH-006
    implementations:
      rust:
        - file: crates/syu-desktop/src/lib.rs
          symbols:
            - DesktopLaunchConfig
            - desktop_action_bridge
            - render_shared_workbench_shell
        - file: crates/syu-desktop/src/main.rs
          symbols:
            - main
      json:
        - file: crates/syu-desktop/tauri.conf.json
          symbols:
            - "*"
  - id: FEAT-WORKBENCH-007
    title: Shared browser and desktop Workbench
    summary: Keep the browser and desktop Workbench aligned through a Rust-native UI and server architecture that share one request, goal, assignment, and evidence model.
    status: implemented
    linked_requirements:
      - REQ-WORKBENCH-006
      - REQ-WORKBENCH-007
    implementations:
      rust:
        - file: crates/syu-workbench-server/src/lib.rs
          symbols:
            - workbench_index
            - workbench_css
      markdown:
        - file: docs/guide/workbench.md
          symbols:
            - browser and desktop
            - Rust-native UI
            - server architecture
```
