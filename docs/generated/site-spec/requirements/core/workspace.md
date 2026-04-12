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
      the running CLI and a valid starter specification tree whose starter
      requirements and features begin as `planned` so users can begin from a
      working structure instead of manually creating directories and
      placeholder YAML files. By default the tree lives under `docs/syu/`.
      `syu init --spec-root` MUST support scaffolding the same layout into
      another repository-relative specification root while writing the matching
      `spec.root` value into `syu.yaml`, and `syu init --template` MUST support
      small `rust-only`, `python-only`, and `polyglot` starter layouts so
      adopters can begin closer to their repository style without copying
      example files by hand. `syu init --id-prefix` MUST support seeding a
      shared project-specific stem into the starter philosophy, policy,
      requirement, and feature IDs, and the per-layer `--philosophy-prefix`,
      `--policy-prefix`, `--requirement-prefix`, and `--feature-prefix` flags
      MUST allow narrower overrides when one shared stem is not enough.
  - **priority**: high
  - **status**: implemented
  - **linked_policies**:
    - POL-001
    - POL-004
  - **linked_features**:
    - FEAT-INIT-001
    - FEAT-INIT-002
    - FEAT-INIT-003
    - FEAT-INIT-004
    - FEAT-INIT-005
  - **tests**:
    - **rust**:
      - **file**: tests/init_command.rs
        - **symbols**:
          - *
      - **file**: src/command/init.rs
        - **symbols**:
          - *
      - **file**: src/command/mod.rs
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
    - FEAT-BROWSE-002
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
      them. The startup output MUST also tell users which local URL to open in a
      browser and how to stop the server cleanly.
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
    - FEAT-LIST-002
    - FEAT-SHOW-001
  - **tests**:
    - **rust**:
      - **file**: tests/list_command.rs
        - **symbols**:
          - *
      - **file**: tests/show_command.rs
        - **symbols**:
          - *
      - **file**: src/command/show.rs
        - **symbols**:
          - *
      - **file**: src/lib.rs
        - **symbols**:
          - dispatches_lookup_subcommands_without_rewriting_them
      - **file**: src/command/list.rs
        - **symbols**:
          - *
      - **file**: src/command/lookup.rs
        - **symbols**:
          - *
- **id**: REQ-CORE-019
  - **title**: Provide a terminal-first search CLI command
  - **description**:
    - |
      The CLI MUST provide a lightweight `search` command that matches
      philosophies, policies, requirements, and features by ID, title, summary,
      or description without requiring the browser app. The command SHOULD
      support optional kind scoping, SHOULD offer JSON output for automation,
      and SHOULD continue working when validation issues exist so long as the
      workspace itself still loads.
  - **priority**: medium
  - **status**: implemented
  - **linked_policies**:
    - POL-001
    - POL-002
    - POL-004
  - **linked_features**:
    - FEAT-SEARCH-001
  - **tests**:
    - **rust**:
      - **file**: tests/search_command.rs
        - **symbols**:
          - *
      - **file**: tests/help_command.rs
        - **symbols**:
          - search_help_mentions_kind_scoping_and_json_output
      - **file**: src/lib.rs
        - **symbols**:
          - dispatches_search_subcommands_without_rewriting_them
      - **file**: src/command/search.rs
        - **symbols**:
          - *
      - **file**: src/command/lookup.rs
        - **symbols**:
          - search
          - extend_search_results
          - field_matches_query
- **id**: REQ-CORE-020
  - **title**: Provide a follow-up authoring scaffold command after syu init
  - **description**:
    - |
      The CLI MUST provide an `add` command that scaffolds a new philosophy,
      policy, requirement, or feature stub after a workspace already exists.
      The command SHOULD infer a sensible default title and document path from
      the requested ID, MUST honor the configured `spec.root`, and MUST update
      the explicit feature registry when creating a new feature document. Output
      SHOULD stay concise enough for normal code review and hand-edited follow-up
      work.
  - **priority**: medium
  - **status**: implemented
  - **linked_policies**:
    - POL-001
    - POL-004
  - **linked_features**:
    - FEAT-ADD-001
  - **tests**:
    - **rust**:
      - **file**: tests/add_command.rs
        - **symbols**:
          - *
      - **file**: tests/help_command.rs
        - **symbols**:
          - add_help_mentions_explicit_file_and_feature_kind
      - **file**: src/lib.rs
        - **symbols**:
          - dispatches_add_subcommands_without_rewriting_them
      - **file**: src/command/add.rs
        - **symbols**:
          - *

## Source YAML

```yaml
category: Core Workspace
prefix: REQ-CORE
requirements:
  - id: REQ-CORE-009
    title: Bootstrap a workspace with syu init and syu.yaml
    description: |
      The `init` command MUST create a starter `syu.yaml` whose version matches
      the running CLI and a valid starter specification tree whose starter
      requirements and features begin as `planned` so users can begin from a
      working structure instead of manually creating directories and
      placeholder YAML files. By default the tree lives under `docs/syu/`.
      `syu init --spec-root` MUST support scaffolding the same layout into
      another repository-relative specification root while writing the matching
      `spec.root` value into `syu.yaml`, and `syu init --template` MUST support
      small `rust-only`, `python-only`, and `polyglot` starter layouts so
      adopters can begin closer to their repository style without copying
      example files by hand. `syu init --id-prefix` MUST support seeding a
      shared project-specific stem into the starter philosophy, policy,
      requirement, and feature IDs, and the per-layer `--philosophy-prefix`,
      `--policy-prefix`, `--requirement-prefix`, and `--feature-prefix` flags
      MUST allow narrower overrides when one shared stem is not enough.
    priority: high
    status: implemented
    linked_policies:
      - POL-001
      - POL-004
    linked_features:
      - FEAT-INIT-001
      - FEAT-INIT-002
      - FEAT-INIT-003
      - FEAT-INIT-004
      - FEAT-INIT-005
    tests:
      rust:
        - file: tests/init_command.rs
          symbols:
            - '*'
        - file: src/command/init.rs
          symbols:
            - '*'
        - file: src/command/mod.rs
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
      - FEAT-BROWSE-002
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
      them. The startup output MUST also tell users which local URL to open in a
      browser and how to stop the server cleanly.
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
      - FEAT-LIST-002
      - FEAT-SHOW-001
    tests:
      rust:
        - file: tests/list_command.rs
          symbols:
            - '*'
        - file: tests/show_command.rs
          symbols:
            - '*'
        - file: src/command/show.rs
          symbols:
            - '*'
        - file: src/lib.rs
          symbols:
            - dispatches_lookup_subcommands_without_rewriting_them
        - file: src/command/list.rs
          symbols:
            - '*'
        - file: src/command/lookup.rs
          symbols:
            - '*'
  - id: REQ-CORE-019
    title: Provide a terminal-first search CLI command
    description: |
      The CLI MUST provide a lightweight `search` command that matches
      philosophies, policies, requirements, and features by ID, title, summary,
      or description without requiring the browser app. The command SHOULD
      support optional kind scoping, SHOULD offer JSON output for automation,
      and SHOULD continue working when validation issues exist so long as the
      workspace itself still loads.
    priority: medium
    status: implemented
    linked_policies:
      - POL-001
      - POL-002
      - POL-004
    linked_features:
      - FEAT-SEARCH-001
    tests:
      rust:
        - file: tests/search_command.rs
          symbols:
            - '*'
        - file: tests/help_command.rs
          symbols:
            - search_help_mentions_kind_scoping_and_json_output
        - file: src/lib.rs
          symbols:
            - dispatches_search_subcommands_without_rewriting_them
        - file: src/command/search.rs
          symbols:
            - '*'
        - file: src/command/lookup.rs
          symbols:
            - search
            - extend_search_results
            - field_matches_query
  - id: REQ-CORE-020
    title: Provide a follow-up authoring scaffold command after syu init
    description: |
      The CLI MUST provide an `add` command that scaffolds a new philosophy,
      policy, requirement, or feature stub after a workspace already exists.
      The command SHOULD infer a sensible default title and document path from
      the requested ID, MUST honor the configured `spec.root`, and MUST update
      the explicit feature registry when creating a new feature document. Output
      SHOULD stay concise enough for normal code review and hand-edited follow-up
      work.
    priority: medium
    status: implemented
    linked_policies:
      - POL-001
      - POL-004
    linked_features:
      - FEAT-ADD-001
    tests:
      rust:
        - file: tests/add_command.rs
          symbols:
            - '*'
        - file: tests/help_command.rs
          symbols:
            - add_help_mentions_explicit_file_and_feature_kind
        - file: src/lib.rs
          symbols:
            - dispatches_add_subcommands_without_rewriting_them
        - file: src/command/add.rs
          symbols:
            - '*'
```
