---
title: "Core Workspace / Workspace"
description: "Generated reference for docs/syu/requirements/core/workspace.yaml"
---

> Generated from `docs/syu/requirements/core/workspace.yaml`.

## Parsed content

### Category

- Core Workspace

### Prefix

- REQ-CORE

### Requirements

- **id**: REQ-CORE-009
  - **title**: Bootstrap a workspace with syu init and syu.yaml
  - **description**:
    - |
      The `init` command MUST create a starter `syu.yaml` whose version matches
      the running CLI and a valid `docs/syu/` tree whose starter requirements
      and features begin as `planned` so users can begin from a working
      structure instead of manually creating directories and placeholder YAML
      files.
  - **priority**: high
  - **status**: implemented
  - **linked_policies**:
    - POL-001
    - POL-004
  - **linked_features**:
    - FEAT-INIT-001
  - **tests**:
    - **rust**:
      - **file**: tests/init_command.rs
        - **symbols**:
          - *
      - **file**: src/command/init.rs
        - **symbols**:
          - *
      - **file**: src/config.rs
        - **symbols**:
          - *
- **id**: REQ-CORE-015
  - **title**: Provide a resilient interactive browse CLI
  - **description**:
    - |
      Running `syu` without a subcommand in a terminal MUST open an interactive
      browser that shows philosophy, policy, feature, requirement, and error
      counts; allows drilling into linked definitions; and still explains the
      workspace when validation issues exist. When standard input/output are not
      terminals, `syu` SHOULD fall back to help text instead of crashing.
  - **priority**: medium
  - **status**: implemented
  - **linked_policies**:
    - POL-001
    - POL-002
    - POL-004
  - **linked_features**:
    - FEAT-BROWSE-001
  - **tests**:
    - **rust**:
      - **file**: tests/browse_command.rs
        - **symbols**:
          - *
      - **file**: src/lib.rs
        - **symbols**:
          - dispatches_interactive_bare_invocations_to_browse_defaults
      - **file**: src/command/browse.rs
        - **symbols**:
          - *
- **id**: REQ-CORE-017
  - **title**: Serve a local browser app backed by shared Rust and WebAssembly logic
  - **description**:
    - |
      The `app` command MUST start a local server that lets contributors inspect
      the current workspace in a browser. The UI MUST expose tabs for
      philosophy, policies, features, and requirements; keep file- and
      folder-oriented subnavigation for each layer; show linked items and
      current validation issues even when the workspace is imperfect; and reuse
      browser-safe Rust logic through WebAssembly instead of reimplementing the
      layered model only in JavaScript. When `syu.yaml` defines app defaults,
      `syu app` MUST use `app.bind` and `app.port` unless CLI flags override
      them.
  - **priority**: medium
  - **status**: implemented
  - **linked_policies**:
    - POL-002
    - POL-004
    - POL-005
  - **linked_features**:
    - FEAT-APP-001
  - **tests**:
    - **rust**:
      - **file**: src/lib.rs
        - **symbols**:
          - dispatches_app_subcommands_without_rewriting_them
      - **file**: tests/repository_quality.rs
        - **symbols**:
          - repository_ships_browser_app
      - **file**: tests/app_command.rs
        - **symbols**:
          - *
      - **file**: src/command/app.rs
        - **symbols**:
          - *
    - **typescript**:
      - **file**: app/tests/browser-app.spec.ts
        - **symbols**:
          - *
- **id**: REQ-CORE-018
  - **title**: Provide non-interactive list and show CLI commands
  - **description**:
    - |
      The CLI MUST provide one-command lookup flows that let users list
      philosophies, policies, requirements, or features and show the details
      for a known item by ID without entering interactive browse mode. These
      commands SHOULD keep working when validation issues exist so long as the
      workspace itself can still load, and SHOULD offer JSON output for
      automation.
  - **priority**: medium
  - **status**: implemented
  - **linked_policies**:
    - POL-001
    - POL-002
    - POL-004
  - **linked_features**:
    - FEAT-LIST-001
    - FEAT-SHOW-001
  - **tests**:
    - **rust**:
      - **file**: tests/list_command.rs
        - **symbols**:
          - *
      - **file**: tests/show_command.rs
        - **symbols**:
          - *
      - **file**: src/lib.rs
        - **symbols**:
          - dispatches_lookup_subcommands_without_rewriting_them

## Source YAML

```yaml
category: Core Workspace
prefix: REQ-CORE
requirements:
  - id: REQ-CORE-009
    title: Bootstrap a workspace with syu init and syu.yaml
    description: |
      The `init` command MUST create a starter `syu.yaml` whose version matches
      the running CLI and a valid `docs/syu/` tree whose starter requirements
      and features begin as `planned` so users can begin from a working
      structure instead of manually creating directories and placeholder YAML
      files.
    priority: high
    status: implemented
    linked_policies:
      - POL-001
      - POL-004
    linked_features:
      - FEAT-INIT-001
    tests:
      rust:
        - file: tests/init_command.rs
          symbols:
            - '*'
        - file: src/command/init.rs
          symbols:
            - '*'
        - file: src/config.rs
          symbols:
            - '*'
  - id: REQ-CORE-015
    title: Provide a resilient interactive browse CLI
    description: |
      Running `syu` without a subcommand in a terminal MUST open an interactive
      browser that shows philosophy, policy, feature, requirement, and error
      counts; allows drilling into linked definitions; and still explains the
      workspace when validation issues exist. When standard input/output are not
      terminals, `syu` SHOULD fall back to help text instead of crashing.
    priority: medium
    status: implemented
    linked_policies:
      - POL-001
      - POL-002
      - POL-004
    linked_features:
      - FEAT-BROWSE-001
    tests:
      rust:
        - file: tests/browse_command.rs
          symbols:
            - '*'
        - file: src/lib.rs
          symbols:
            - dispatches_interactive_bare_invocations_to_browse_defaults
        - file: src/command/browse.rs
          symbols:
            - '*'
  - id: REQ-CORE-017
    title: Serve a local browser app backed by shared Rust and WebAssembly logic
    description: |
      The `app` command MUST start a local server that lets contributors inspect
      the current workspace in a browser. The UI MUST expose tabs for
      philosophy, policies, features, and requirements; keep file- and
      folder-oriented subnavigation for each layer; show linked items and
      current validation issues even when the workspace is imperfect; and reuse
      browser-safe Rust logic through WebAssembly instead of reimplementing the
      layered model only in JavaScript. When `syu.yaml` defines app defaults,
      `syu app` MUST use `app.bind` and `app.port` unless CLI flags override
      them.
    priority: medium
    status: implemented
    linked_policies:
      - POL-002
      - POL-004
      - POL-005
    linked_features:
      - FEAT-APP-001
    tests:
      rust:
        - file: src/lib.rs
          symbols:
            - dispatches_app_subcommands_without_rewriting_them
        - file: tests/repository_quality.rs
          symbols:
            - repository_ships_browser_app
        - file: tests/app_command.rs
          symbols:
            - '*'
        - file: src/command/app.rs
          symbols:
            - '*'
      typescript:
        - file: app/tests/browser-app.spec.ts
          symbols:
            - '*'
  - id: REQ-CORE-018
    title: Provide non-interactive list and show CLI commands
    description: |
      The CLI MUST provide one-command lookup flows that let users list
      philosophies, policies, requirements, or features and show the details
      for a known item by ID without entering interactive browse mode. These
      commands SHOULD keep working when validation issues exist so long as the
      workspace itself can still load, and SHOULD offer JSON output for
      automation.
    priority: medium
    status: implemented
    linked_policies:
      - POL-001
      - POL-002
      - POL-004
    linked_features:
      - FEAT-LIST-001
      - FEAT-SHOW-001
    tests:
      rust:
        - file: tests/list_command.rs
          symbols:
            - '*'
        - file: tests/show_command.rs
          symbols:
            - '*'
        - file: src/lib.rs
          symbols:
            - dispatches_lookup_subcommands_without_rewriting_them
```
