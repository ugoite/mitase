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
      small `rust-only`, `python-only`, `go-only`, and `polyglot` starter layouts so
      adopters can begin closer to their repository style without copying
      example files by hand. The CLI MUST also provide `syu templates` so users
      can discover the available starter layouts, short descriptions, and any
      matching checked-in examples before they scaffold, and the root help plus
      closely related `syu init` guidance MUST surface that discovery command
      alongside `syu init` without sending newcomers to docs first.
      `syu init --id-prefix` MUST support seeding a
       shared project-specific stem into the starter philosophy, policy,
       requirement, and feature IDs, and the per-layer `--philosophy-prefix`,
       `--policy-prefix`, `--requirement-prefix`, and `--feature-prefix` flags
       MUST allow narrower overrides when one shared stem is not enough.
       `syu init --interactive` MUST guide first-run users through the starter
       template, `spec.root`, shared ID stem, and stricter validation defaults
       in a terminal while still writing the same checked-in scaffold.
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
    - FEAT-INIT-006
    - FEAT-INIT-007
  - **tests**:
    - **rust**:
      - **file**: tests/init_command.rs
        - **symbols**:
          - *
      - **file**: src/command/init.rs
        - **symbols**:
          - *
      - **file**: src/command/templates.rs
        - **symbols**:
          - *
      - **file**: src/command/prompt.rs
        - **symbols**:
          - *
      - **file**: src/command/mod.rs
        - **symbols**:
          - *
      - **file**: src/config.rs
        - **symbols**:
          - *
      - **file**: tests/templates_command.rs
        - **symbols**:
          - *
      - **file**: tests/help_command.rs
        - **symbols**:
          - templates_help_mentions_json_and_init_follow_up
- **id**: REQ-CORE-015
  - **title**: Provide a resilient interactive browse CLI
  - **description**:
    - |
      Running `syu` without a subcommand in a terminal MUST open an interactive
      browser that shows philosophy, policy, feature, requirement, and error
      counts; allows drilling into linked definitions; and still explains the
      workspace when validation issues exist. When standard input/output are not
      terminals, `syu` SHOULD fall back to help text instead of crashing. When
      the workspace argument (or default current directory) points inside a
      workspace, `browse` MUST walk parent directories until it finds
      `syu.yaml`, then show the resolved workspace root in its summary output.
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
      - **file**: tests/workspace_discovery_command.rs
        - **symbols**:
          - browse_command_discovers_workspace_from_child_directory
      - **file**: src/lib.rs
        - **symbols**:
          - dispatches_interactive_bare_invocations_to_browse_defaults
          - print_help_dispatch_renders_successfully
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
      them. When `app.bind` or `--bind` selects a non-loopback address, `syu
      app` MUST require an explicit `--allow-remote` opt-in before it starts.
      When the workspace argument (or default current directory) points inside a
      workspace, `syu app` MUST walk parent directories until it finds
      `syu.yaml`. The startup output MUST also tell users which workspace root
      was selected, which local URL to open in a browser, and how to stop the
      server cleanly.
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
      - **file**: tests/help_command.rs
        - **symbols**:
          - app_help_mentions_remote_bind_opt_in
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
      automation. When the workspace argument points at a child directory,
      `list` and `show` MUST walk parent directories until they find
      `syu.yaml`.
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
      - **file**: tests/workspace_discovery_command.rs
        - **symbols**:
          - list_command_discovers_workspace_from_child_directory
          - show_command_discovers_workspace_from_child_directory
      - **file**: tests/help_command.rs
        - **symbols**:
          - list_help_mentions_spec_root_and_child_directory_examples
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
      workspace itself still loads. When the workspace argument points at a
      child directory, `search` MUST walk parent directories until it finds
      `syu.yaml`.
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
      - **file**: tests/workspace_discovery_command.rs
        - **symbols**:
          - search_command_discovers_workspace_from_child_directory
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
      work while explicitly guiding contributors toward the reciprocal links they
      still need before the next validation run, including concrete scaffold
      commands for adjacent definitions when those documents do not exist yet.
      When contributors omit the ID in a terminal, `syu add` SHOULD prompt for
      it, and `syu add --interactive` SHOULD let them confirm or override the
      default feature kind and target YAML path before writing the stub.
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
- **id**: REQ-CORE-021
  - **title**: Provide a source-first trace lookup CLI command
  - **description**:
    - |
      The CLI MUST provide a `trace` command that starts from a repository file
      path and optional symbol, then resolves the linked requirements, features,
      policies, and philosophies for that source artifact without requiring a
      known spec ID first. The command SHOULD offer text and JSON output, SHOULD
      keep working when the workspace still loads despite validation issues, and
      SHOULD explain whether the source is owned, partially traced, or unowned.
  - **priority**: medium
  - **status**: implemented
  - **linked_policies**:
    - POL-001
    - POL-002
    - POL-003
    - POL-004
  - **linked_features**:
    - FEAT-TRACE-001
  - **tests**:
    - **rust**:
      - **file**: tests/trace_command.rs
        - **symbols**:
          - *
      - **file**: tests/help_command.rs
        - **symbols**:
          - trace_help_mentions_symbol_lookup_and_json_output
      - **file**: src/lib.rs
        - **symbols**:
          - dispatches_trace_subcommands_without_rewriting_them
      - **file**: src/command/trace.rs
        - **symbols**:
          - *
- **id**: REQ-CORE-022
  - **title**: Provide a VS Code extension for diagnostics and source-first navigation
  - **description**:
    - |
      The repository MUST ship a VS Code extension that keeps common `syu`
      workflows inside the editor. The extension MUST surface validation
      diagnostics in the Problems panel, MUST let users jump from spec IDs to
      their YAML documents and traced files, and SHOULD show the current file's
      related requirements, features, policies, and philosophies without
      requiring manual terminal commands for each lookup. The first cut MAY use
      the existing CLI plus checked-in spec files directly, but the integration
      contract SHOULD stay explicit enough to support richer editor clients
      later.
  - **priority**: medium
  - **status**: implemented
  - **linked_policies**:
    - POL-002
    - POL-003
    - POL-004
    - POL-005
  - **linked_features**:
    - FEAT-VSCODE-001
  - **tests**:
    - **rust**:
      - **file**: tests/repository_quality.rs
        - **symbols**:
          - repository_ships_vscode_extension
    - **javascript**:
      - **file**: editors/vscode/test/model.test.js
        - **symbols**:
          - *
- **id**: REQ-CORE-023
  - **title**: Provide a cross-layer relation inspection CLI command
  - **description**:
    - |
      The CLI MUST provide a `relate` command that accepts a philosophy, policy,
      requirement, or feature ID, plus repository-relative paths or traced
      source symbols, and returns the connected graph that reviewers should
      inspect next. The command MUST walk upstream and downstream across the
      layered links, MUST include traced files and symbols for connected
      requirements and features, SHOULD surface sparse or suspicious gaps that
      weaken explainability, and SHOULD emit both text and JSON output.
  - **priority**: medium
  - **status**: implemented
  - **linked_policies**:
    - POL-001
    - POL-002
    - POL-004
  - **linked_features**:
    - FEAT-RELATE-001
  - **tests**:
    - **rust**:
      - **file**: tests/relate_command.rs
        - **symbols**:
          - *
      - **file**: tests/help_command.rs
        - **symbols**:
          - relate_help_mentions_ids_paths_symbols_and_json_output
      - **file**: src/lib.rs
        - **symbols**:
          - dispatches_relate_subcommands_without_rewriting_them
      - **file**: src/command/relate.rs
        - **symbols**:
          - *
- **id**: REQ-CORE-024
  - **title**: Provide Git history lookup from traced requirements and features
  - **description**:
    - |
      The CLI MUST provide a `log` command that maps one requirement or feature
      onto checked-in Git history using the current definition path and traced
      test or implementation files from the workspace. The command MUST explain
      why each returned commit matched, SHOULD support narrowing the lookup to
      definition versus traced evidence and to one path subset, SHOULD offer
      a reviewer-oriented mode that can include related spec items and traced
      files from the relation graph, SHOULD support scoping the returned history
      to an explicit Git range or the current branch's merge-base with another
      ref, SHOULD offer JSON output for audit or release automation, and SHOULD
      continue working when validation issues exist so long as the workspace
      still loads and is inside a Git repository.
  - **priority**: medium
  - **status**: implemented
  - **linked_policies**:
    - POL-002
    - POL-003
    - POL-004
  - **linked_features**:
    - FEAT-LOG-001
  - **tests**:
    - **rust**:
      - **file**: tests/log_command.rs
        - **symbols**:
          - *
      - **file**: tests/help_command.rs
        - **symbols**:
          - log_help_mentions_kind_path_and_json_output
      - **file**: src/lib.rs
        - **symbols**:
          - dispatches_log_subcommands_without_rewriting_them
      - **file**: src/command/log.rs
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
      small `rust-only`, `python-only`, `go-only`, and `polyglot` starter layouts so
      adopters can begin closer to their repository style without copying
      example files by hand. The CLI MUST also provide `syu templates` so users
      can discover the available starter layouts, short descriptions, and any
      matching checked-in examples before they scaffold, and the root help plus
      closely related `syu init` guidance MUST surface that discovery command
      alongside `syu init` without sending newcomers to docs first.
      `syu init --id-prefix` MUST support seeding a
       shared project-specific stem into the starter philosophy, policy,
       requirement, and feature IDs, and the per-layer `--philosophy-prefix`,
       `--policy-prefix`, `--requirement-prefix`, and `--feature-prefix` flags
       MUST allow narrower overrides when one shared stem is not enough.
       `syu init --interactive` MUST guide first-run users through the starter
       template, `spec.root`, shared ID stem, and stricter validation defaults
       in a terminal while still writing the same checked-in scaffold.
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
      - FEAT-INIT-006
      - FEAT-INIT-007
    tests:
      rust:
        - file: tests/init_command.rs
          symbols:
            - '*'
        - file: src/command/init.rs
          symbols:
            - '*'
        - file: src/command/templates.rs
          symbols:
            - '*'
        - file: src/command/prompt.rs
          symbols:
            - '*'
        - file: src/command/mod.rs
          symbols:
            - '*'
        - file: src/config.rs
          symbols:
            - '*'
        - file: tests/templates_command.rs
          symbols:
            - '*'
        - file: tests/help_command.rs
          symbols:
            - templates_help_mentions_json_and_init_follow_up
  - id: REQ-CORE-015
    title: Provide a resilient interactive browse CLI
    description: |
      Running `syu` without a subcommand in a terminal MUST open an interactive
      browser that shows philosophy, policy, feature, requirement, and error
      counts; allows drilling into linked definitions; and still explains the
      workspace when validation issues exist. When standard input/output are not
      terminals, `syu` SHOULD fall back to help text instead of crashing. When
      the workspace argument (or default current directory) points inside a
      workspace, `browse` MUST walk parent directories until it finds
      `syu.yaml`, then show the resolved workspace root in its summary output.
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
        - file: tests/workspace_discovery_command.rs
          symbols:
            - browse_command_discovers_workspace_from_child_directory
        - file: src/lib.rs
          symbols:
            - dispatches_interactive_bare_invocations_to_browse_defaults
            - print_help_dispatch_renders_successfully
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
      them. When `app.bind` or `--bind` selects a non-loopback address, `syu
      app` MUST require an explicit `--allow-remote` opt-in before it starts.
      When the workspace argument (or default current directory) points inside a
      workspace, `syu app` MUST walk parent directories until it finds
      `syu.yaml`. The startup output MUST also tell users which workspace root
      was selected, which local URL to open in a browser, and how to stop the
      server cleanly.
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
        - file: tests/help_command.rs
          symbols:
            - app_help_mentions_remote_bind_opt_in
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
      automation. When the workspace argument points at a child directory,
      `list` and `show` MUST walk parent directories until they find
      `syu.yaml`.
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
        - file: tests/workspace_discovery_command.rs
          symbols:
            - list_command_discovers_workspace_from_child_directory
            - show_command_discovers_workspace_from_child_directory
        - file: tests/help_command.rs
          symbols:
            - list_help_mentions_spec_root_and_child_directory_examples
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
      workspace itself still loads. When the workspace argument points at a
      child directory, `search` MUST walk parent directories until it finds
      `syu.yaml`.
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
        - file: tests/workspace_discovery_command.rs
          symbols:
            - search_command_discovers_workspace_from_child_directory
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
      work while explicitly guiding contributors toward the reciprocal links they
      still need before the next validation run, including concrete scaffold
      commands for adjacent definitions when those documents do not exist yet.
      When contributors omit the ID in a terminal, `syu add` SHOULD prompt for
      it, and `syu add --interactive` SHOULD let them confirm or override the
      default feature kind and target YAML path before writing the stub.
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
  - id: REQ-CORE-021
    title: Provide a source-first trace lookup CLI command
    description: |
      The CLI MUST provide a `trace` command that starts from a repository file
      path and optional symbol, then resolves the linked requirements, features,
      policies, and philosophies for that source artifact without requiring a
      known spec ID first. The command SHOULD offer text and JSON output, SHOULD
      keep working when the workspace still loads despite validation issues, and
      SHOULD explain whether the source is owned, partially traced, or unowned.
    priority: medium
    status: implemented
    linked_policies:
      - POL-001
      - POL-002
      - POL-003
      - POL-004
    linked_features:
      - FEAT-TRACE-001
    tests:
      rust:
        - file: tests/trace_command.rs
          symbols:
            - '*'
        - file: tests/help_command.rs
          symbols:
            - trace_help_mentions_symbol_lookup_and_json_output
        - file: src/lib.rs
          symbols:
            - dispatches_trace_subcommands_without_rewriting_them
        - file: src/command/trace.rs
          symbols:
            - '*'
  - id: REQ-CORE-022
    title: Provide a VS Code extension for diagnostics and source-first navigation
    description: |
      The repository MUST ship a VS Code extension that keeps common `syu`
      workflows inside the editor. The extension MUST surface validation
      diagnostics in the Problems panel, MUST let users jump from spec IDs to
      their YAML documents and traced files, and SHOULD show the current file's
      related requirements, features, policies, and philosophies without
      requiring manual terminal commands for each lookup. The first cut MAY use
      the existing CLI plus checked-in spec files directly, but the integration
      contract SHOULD stay explicit enough to support richer editor clients
      later.
    priority: medium
    status: implemented
    linked_policies:
      - POL-002
      - POL-003
      - POL-004
      - POL-005
    linked_features:
      - FEAT-VSCODE-001
    tests:
      rust:
        - file: tests/repository_quality.rs
          symbols:
            - repository_ships_vscode_extension
      javascript:
        - file: editors/vscode/test/model.test.js
          symbols:
            - '*'
  - id: REQ-CORE-023
    title: Provide a cross-layer relation inspection CLI command
    description: |
      The CLI MUST provide a `relate` command that accepts a philosophy, policy,
      requirement, or feature ID, plus repository-relative paths or traced
      source symbols, and returns the connected graph that reviewers should
      inspect next. The command MUST walk upstream and downstream across the
      layered links, MUST include traced files and symbols for connected
      requirements and features, SHOULD surface sparse or suspicious gaps that
      weaken explainability, and SHOULD emit both text and JSON output.
    priority: medium
    status: implemented
    linked_policies:
      - POL-001
      - POL-002
      - POL-004
    linked_features:
      - FEAT-RELATE-001
    tests:
      rust:
        - file: tests/relate_command.rs
          symbols:
            - '*'
        - file: tests/help_command.rs
          symbols:
            - relate_help_mentions_ids_paths_symbols_and_json_output
        - file: src/lib.rs
          symbols:
            - dispatches_relate_subcommands_without_rewriting_them
        - file: src/command/relate.rs
          symbols:
            - '*'
  - id: REQ-CORE-024
    title: Provide Git history lookup from traced requirements and features
    description: |
      The CLI MUST provide a `log` command that maps one requirement or feature
      onto checked-in Git history using the current definition path and traced
      test or implementation files from the workspace. The command MUST explain
      why each returned commit matched, SHOULD support narrowing the lookup to
      definition versus traced evidence and to one path subset, SHOULD offer
      a reviewer-oriented mode that can include related spec items and traced
      files from the relation graph, SHOULD support scoping the returned history
      to an explicit Git range or the current branch's merge-base with another
      ref, SHOULD offer JSON output for audit or release automation, and SHOULD
      continue working when validation issues exist so long as the workspace
      still loads and is inside a Git repository.
    priority: medium
    status: implemented
    linked_policies:
      - POL-002
      - POL-003
      - POL-004
    linked_features:
      - FEAT-LOG-001
    tests:
      rust:
        - file: tests/log_command.rs
          symbols:
            - '*'
        - file: tests/help_command.rs
          symbols:
            - log_help_mentions_kind_path_and_json_output
        - file: src/lib.rs
          symbols:
            - dispatches_log_subcommands_without_rewriting_them
        - file: src/command/log.rs
          symbols:
            - '*'
```
