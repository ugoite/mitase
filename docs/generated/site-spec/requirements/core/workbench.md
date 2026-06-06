---
title: "Core Workbench / Workbench"
description: "Generated reference for docs/syu/requirements/core/workbench.yaml"
---

> Generated from `docs/syu/requirements/core/workbench.yaml`.

## Parsed content

### Category

- Core Workbench

### Prefix

- REQ-WORKBENCH

### Requirements

- **id**: REQ-WORKBENCH-001
  - **title**: Command-palette-first Workbench
  - **description**:
    - |
      The Workbench MUST open around a command palette instead of a fixed tab
      strip. Users SHOULD be able to launch request, goal, scope, assignment,
      and evidence actions from the same keyboard-first surface, and the UI
      SHOULD keep the active goal centered in view while rendering actions from
      the registry rather than hardcoded buttons. The first Workbench shell
      SHOULD be visually intentional from the start, using centralized design
      tokens and reusable classes rather than throwaway handwritten CSS.
  - **priority**: medium
  - **status**: implemented
  - **linked_policies**:
    - POL-005
  - **linked_features**:
    - FEAT-WORKBENCH-SHELL-001
    - FEAT-WORKBENCH-DESIGN-TOKENS-001
  - **tests**:
    - **markdown**:
      - **file**: docs/guide/workbench.md
        - **symbols**:
          - command-palette-first
          - command palette registry
          - WorkbenchActionRegistry
    - **rust**:
      - **file**: tests/workbench_smoke.rs
        - **symbols**:
          - app_shell_renders_command_palette_first_shell
          - command_palette_renders_disabled_reason_for_unavailable_actions
          - goal_canvas_renders_a_read_only_action_preview_placeholder
          - evidence_panel_renders_placeholder_when_empty
- **id**: REQ-WORKBENCH-002
  - **title**: Request, Goal, Evidence, and Assignment model
  - **description**:
    - |
      The Workbench MUST represent requests, goals, evidence, and assignments
      as explicit artifacts instead of hiding them inside a generic task list.
      The model SHOULD make it clear which request a goal came from, which
      evidence supports progress, who is responsible for the current step, and
      which typed action produced the current transition. Evidence status SHOULD
      have stable UI tokens and reusable presentation hooks so later timeline
      views can reuse the same visual language without inventing a second set of
      colors or badge semantics.
  - **priority**: medium
  - **status**: implemented
  - **linked_policies**:
    - POL-005
  - **linked_features**:
    - FEAT-WORKBENCH-COMMAND-PALETTE-001
    - FEAT-WORKBENCH-DESIGN-TOKENS-001
    - FEAT-WORKBENCH-003
    - FEAT-WORKBENCH-EVIDENCE-001
  - **tests**:
    - **markdown**:
      - **file**: docs/guide/workbench.md
        - **symbols**:
          - WorkbenchState
          - ActiveRequestState
          - ActiveGoalState
          - AssignmentState
    - **rust**:
      - **file**: tests/workbench_smoke.rs
        - **symbols**:
          - filters_actions_by_query
          - read_only_action_returns_placeholder_preview
          - registry_loaded_from_server_payload
          - evidence_panel_renders_goal_scoped_timeline
      - **file**: crates/syu-app-ui/src/model/tests.rs
        - **symbols**:
          - cli_catalog_exposes_top_level_and_task_commands
          - filters_cli_commands_by_query_and_previews_invocation
- **id**: REQ-WORKBENCH-003
  - **title**: Goal splitting for large change requests
  - **description**:
    - |
      The Workbench MUST help break large requests into smaller scoped goals
      before execution starts. It SHOULD preserve the parent request, keep the
      split goals reviewable, and let each goal carry its own temporary Goal
      Plan so delivery stays bounded. Request Intake MUST distinguish temporary
      Workbench planning artifacts under `.syu/workbench/` from persistent spec
      items, and exported Goal Plan YAML MUST remain compatible with
      `syu task check` while preserving non-goals, scope, tests, completion
      commands, and required evidence.
  - **priority**: medium
  - **status**: implemented
  - **linked_policies**:
    - POL-005
  - **linked_features**:
    - FEAT-WORKBENCH-003
    - FEAT-WORKBENCH-REQUEST-INTAKE-001
    - FEAT-WORKBENCH-GOAL-SPLITTER-001
  - **tests**:
    - **markdown**:
      - **file**: docs/guide/workbench.md
        - **symbols**:
          - Request Intake
          - Goal Splitter
          - scaffold preview
          - Goal Plan
          - assignment
    - **rust**:
      - **file**: tests/workbench_smoke.rs
        - **symbols**:
          - request_intake_flow_renders_generated_goal_plan
          - request_flow_actions_are_exposed_in_the_command_palette
          - goal_plan_export_panel_marks_yaml_as_temporary_artifact
- **id**: REQ-WORKBENCH-004
  - **title**: Spec impact and branch scope visualization
  - **description**:
    - |
      The Workbench MUST show which specifications, files, and branch scope are
      likely to change before the user commits to implementation. It SHOULD
      make the impact of a request visible early enough that the user can
      refine scope before work starts. Scope status MUST distinguish in-scope,
      out-of-scope, ambiguous, owned, unowned, and ownership-ambiguous states,
      and visual graph state MUST use named Workbench tokens rather than
      arbitrary styling. Spec impact and branch scope reports SHOULD stay
      compatible with future evidence timeline records.
  - **priority**: medium
  - **status**: implemented
  - **linked_policies**:
    - POL-005
  - **linked_features**:
    - FEAT-WORKBENCH-004
    - FEAT-WORKBENCH-SPEC-GRAPH-001
    - FEAT-WORKBENCH-BRANCH-SCOPE-001
  - **tests**:
    - **markdown**:
      - **file**: docs/guide/workbench.md
        - **symbols**:
          - scope
          - branch scope
          - scaffold preview
    - **rust**:
      - **file**: crates/syu-code-intel/src/branch_scope.rs
        - **symbols**:
          - branch_scope_report_includes_typed_graph_nodes_and_edges
      - **file**: tests/workbench_smoke.rs
        - **symbols**:
          - branch_scope_lens_renders_scope_ownership_and_tests
          - spec_impact_graph_renders_typed_nodes_edges_and_legend
- **id**: REQ-WORKBENCH-005
  - **title**: Human and AI assignment with explicit scope and evidence
  - **description**:
    - |
      The Workbench MUST support assigning a scoped Goal Plan to a human or AI
      command adapter with explicit include/exclude scope, non-goals, linked
      spec context, required tests, completion commands, expected evidence, and
      a clear handoff boundary. Automated assignment MUST be blocked when scope
      is ambiguous or required constraints are missing, and dry-run output MUST
      be captured as evidence.
  - **priority**: medium
  - **status**: implemented
  - **linked_policies**:
    - POL-005
  - **linked_features**:
    - FEAT-WORKBENCH-EVIDENCE-001
    - FEAT-WORKBENCH-ASSIGNMENT-001
  - **tests**:
    - **markdown**:
      - **file**: docs/guide/workbench.md
        - **symbols**:
          - assignment
          - evidence
          - completion check
    - **rust**:
      - **file**: crates/syu-workbench/src/lib.rs
        - **symbols**:
          - assignment_blocker_logic_rejects_ambiguous_ai_scope
          - dry_run_command_adapter_captures_stdout_stderr_and_evidence
      - **file**: tests/workbench_smoke.rs
        - **symbols**:
          - assignment_preview_renders_blocked_state_with_scope_tokens
          - assignment_actions_are_exposed_in_the_command_palette
          - scope_guard_preview_renders_out_of_scope_changes
- **id**: REQ-WORKBENCH-006
  - **title**: Shared browser and desktop Workbench behavior
  - **description**:
    - |
      The Workbench MUST behave consistently in browser and desktop contexts.
      The same request, goal, assignment, and evidence flow SHOULD work in both
      environments so users do not have to learn two separate products. The
      browser root served by `syu workbench` MUST render the shared Dioxus
      Workbench shell instead of an API placeholder page. Tauri MUST remain a
      desktop shell around the shared Workbench server, typed action registry,
      Dioxus UI crate, and Tailwind-generated CSS asset rather than becoming a
      second Workbench implementation.
  - **priority**: medium
  - **status**: implemented
  - **linked_policies**:
    - POL-005
  - **linked_features**:
    - FEAT-WORKBENCH-SHELL-001
    - FEAT-WORKBENCH-007
    - FEAT-WORKBENCH-TAURI-001
    - FEAT-WORKBENCH-SERVER-001
  - **tests**:
    - **markdown**:
      - **file**: docs/guide/workbench.md
        - **symbols**:
          - browser and desktop
          - same way
          - shared Dioxus Workbench UI
    - **rust**:
      - **file**: crates/syu-desktop/src/lib.rs
        - **symbols**:
          - desktop_bridge_uses_the_shared_action_registry
          - desktop_shell_renders_the_shared_dioxus_workbench_ui
      - **file**: crates/syu-workbench-server/src/tests.rs
        - **symbols**:
          - index_route_renders_workbench_browser_entrypoint_and_css_asset
          - css_route_serves_the_shared_tailwind_asset
          - server_smoke_covers_root_css_health_and_actions
- **id**: REQ-WORKBENCH-007
  - **title**: Rust-native UI and server architecture
  - **description**:
    - |
      The Workbench MUST use a Rust-native UI and server architecture so the
      product can share one source of truth for request intake, goal tracking,
      evidence capture, and assignment state. The implementation SHOULD keep
      the UI and server layers close enough that browser and desktop clients
      stay in sync. CI MUST validate this architecture with Rust-native
      Workbench model, action, server, UI, smoke, and installed-binary API
      checks instead of old browser-app jobs. The browser/server entrypoint
      MUST server-render the shared UI crate with the shared CSS asset while
      keeping localhost-first server security defaults. Tailwind CSS is allowed
      only as a constrained stylesheet build layer for `crates/syu-app-ui/`.
  - **priority**: medium
  - **status**: implemented
  - **linked_policies**:
    - POL-005
  - **linked_features**:
    - FEAT-WORKBENCH-SHELL-001
    - FEAT-WORKBENCH-007
    - FEAT-WORKBENCH-SERVER-001
  - **tests**:
    - **markdown**:
      - **file**: docs/guide/workbench.md
        - **symbols**:
          - Rust-native UI
          - server architecture
          - browser and desktop
          - Workbench CI
    - **shell**:
      - **file**: scripts/ci/check-workbench.sh
        - **symbols**:
          - run_workbench_checks
      - **file**: scripts/ci/check-ui-assets.sh
        - **symbols**:
          - check_ui_assets
      - **file**: scripts/ci/installed-binary-smoke.sh
        - **symbols**:
          - /api/health
          - /api/actions

## Source YAML

```yaml
category: Core Workbench
prefix: REQ-WORKBENCH
requirements:
  - id: REQ-WORKBENCH-001
    title: Command-palette-first Workbench
    description: |
      The Workbench MUST open around a command palette instead of a fixed tab
      strip. Users SHOULD be able to launch request, goal, scope, assignment,
      and evidence actions from the same keyboard-first surface, and the UI
      SHOULD keep the active goal centered in view while rendering actions from
      the registry rather than hardcoded buttons. The first Workbench shell
      SHOULD be visually intentional from the start, using centralized design
      tokens and reusable classes rather than throwaway handwritten CSS.
    priority: medium
    status: implemented
    linked_policies:
      - POL-005
    linked_features:
      - FEAT-WORKBENCH-SHELL-001
      - FEAT-WORKBENCH-DESIGN-TOKENS-001
    tests:
      markdown:
        - file: docs/guide/workbench.md
          symbols:
            - command-palette-first
            - command palette registry
            - WorkbenchActionRegistry
      rust:
        - file: tests/workbench_smoke.rs
          symbols:
            - app_shell_renders_command_palette_first_shell
            - command_palette_renders_disabled_reason_for_unavailable_actions
            - goal_canvas_renders_a_read_only_action_preview_placeholder
            - evidence_panel_renders_placeholder_when_empty
  - id: REQ-WORKBENCH-002
    title: Request, Goal, Evidence, and Assignment model
    description: |
      The Workbench MUST represent requests, goals, evidence, and assignments
      as explicit artifacts instead of hiding them inside a generic task list.
      The model SHOULD make it clear which request a goal came from, which
      evidence supports progress, who is responsible for the current step, and
      which typed action produced the current transition. Evidence status SHOULD
      have stable UI tokens and reusable presentation hooks so later timeline
      views can reuse the same visual language without inventing a second set of
      colors or badge semantics.
    priority: medium
    status: implemented
    linked_policies:
      - POL-005
    linked_features:
      - FEAT-WORKBENCH-COMMAND-PALETTE-001
      - FEAT-WORKBENCH-DESIGN-TOKENS-001
      - FEAT-WORKBENCH-003
      - FEAT-WORKBENCH-EVIDENCE-001
    tests:
      markdown:
        - file: docs/guide/workbench.md
          symbols:
            - WorkbenchState
            - ActiveRequestState
            - ActiveGoalState
            - AssignmentState
      rust:
        - file: tests/workbench_smoke.rs
          symbols:
            - filters_actions_by_query
            - read_only_action_returns_placeholder_preview
            - registry_loaded_from_server_payload
            - evidence_panel_renders_goal_scoped_timeline
        - file: crates/syu-app-ui/src/model/tests.rs
          symbols:
            - cli_catalog_exposes_top_level_and_task_commands
            - filters_cli_commands_by_query_and_previews_invocation
  - id: REQ-WORKBENCH-003
    title: Goal splitting for large change requests
    description: |
      The Workbench MUST help break large requests into smaller scoped goals
      before execution starts. It SHOULD preserve the parent request, keep the
      split goals reviewable, and let each goal carry its own temporary Goal
      Plan so delivery stays bounded. Request Intake MUST distinguish temporary
      Workbench planning artifacts under `.syu/workbench/` from persistent spec
      items, and exported Goal Plan YAML MUST remain compatible with
      `syu task check` while preserving non-goals, scope, tests, completion
      commands, and required evidence.
    priority: medium
    status: implemented
    linked_policies:
      - POL-005
    linked_features:
      - FEAT-WORKBENCH-003
      - FEAT-WORKBENCH-REQUEST-INTAKE-001
      - FEAT-WORKBENCH-GOAL-SPLITTER-001
    tests:
      markdown:
        - file: docs/guide/workbench.md
          symbols:
            - Request Intake
            - Goal Splitter
            - scaffold preview
            - Goal Plan
            - assignment
      rust:
        - file: tests/workbench_smoke.rs
          symbols:
            - request_intake_flow_renders_generated_goal_plan
            - request_flow_actions_are_exposed_in_the_command_palette
            - goal_plan_export_panel_marks_yaml_as_temporary_artifact
  - id: REQ-WORKBENCH-004
    title: Spec impact and branch scope visualization
    description: |
      The Workbench MUST show which specifications, files, and branch scope are
      likely to change before the user commits to implementation. It SHOULD
      make the impact of a request visible early enough that the user can
      refine scope before work starts. Scope status MUST distinguish in-scope,
      out-of-scope, ambiguous, owned, unowned, and ownership-ambiguous states,
      and visual graph state MUST use named Workbench tokens rather than
      arbitrary styling. Spec impact and branch scope reports SHOULD stay
      compatible with future evidence timeline records.
    priority: medium
    status: implemented
    linked_policies:
      - POL-005
    linked_features:
      - FEAT-WORKBENCH-004
      - FEAT-WORKBENCH-SPEC-GRAPH-001
      - FEAT-WORKBENCH-BRANCH-SCOPE-001
    tests:
      markdown:
        - file: docs/guide/workbench.md
          symbols:
            - scope
            - branch scope
            - scaffold preview
      rust:
        - file: crates/syu-code-intel/src/branch_scope.rs
          symbols:
            - branch_scope_report_includes_typed_graph_nodes_and_edges
        - file: tests/workbench_smoke.rs
          symbols:
            - branch_scope_lens_renders_scope_ownership_and_tests
            - spec_impact_graph_renders_typed_nodes_edges_and_legend
  - id: REQ-WORKBENCH-005
    title: Human and AI assignment with explicit scope and evidence
    description: |
      The Workbench MUST support assigning a scoped Goal Plan to a human or AI
      command adapter with explicit include/exclude scope, non-goals, linked
      spec context, required tests, completion commands, expected evidence, and
      a clear handoff boundary. Automated assignment MUST be blocked when scope
      is ambiguous or required constraints are missing, and dry-run output MUST
      be captured as evidence.
    priority: medium
    status: implemented
    linked_policies:
      - POL-005
    linked_features:
      - FEAT-WORKBENCH-EVIDENCE-001
      - FEAT-WORKBENCH-ASSIGNMENT-001
    tests:
      markdown:
        - file: docs/guide/workbench.md
          symbols:
            - assignment
            - evidence
            - completion check
      rust:
        - file: crates/syu-workbench/src/lib.rs
          symbols:
            - assignment_blocker_logic_rejects_ambiguous_ai_scope
            - dry_run_command_adapter_captures_stdout_stderr_and_evidence
        - file: tests/workbench_smoke.rs
          symbols:
            - assignment_preview_renders_blocked_state_with_scope_tokens
            - assignment_actions_are_exposed_in_the_command_palette
            - scope_guard_preview_renders_out_of_scope_changes
  - id: REQ-WORKBENCH-006
    title: Shared browser and desktop Workbench behavior
    description: |
      The Workbench MUST behave consistently in browser and desktop contexts.
      The same request, goal, assignment, and evidence flow SHOULD work in both
      environments so users do not have to learn two separate products. The
      browser root served by `syu workbench` MUST render the shared Dioxus
      Workbench shell instead of an API placeholder page. Tauri MUST remain a
      desktop shell around the shared Workbench server, typed action registry,
      Dioxus UI crate, and Tailwind-generated CSS asset rather than becoming a
      second Workbench implementation.
    priority: medium
    status: implemented
    linked_policies:
      - POL-005
    linked_features:
      - FEAT-WORKBENCH-SHELL-001
      - FEAT-WORKBENCH-007
      - FEAT-WORKBENCH-TAURI-001
      - FEAT-WORKBENCH-SERVER-001
    tests:
      markdown:
        - file: docs/guide/workbench.md
          symbols:
            - browser and desktop
            - same way
            - shared Dioxus Workbench UI
      rust:
        - file: crates/syu-desktop/src/lib.rs
          symbols:
            - desktop_bridge_uses_the_shared_action_registry
            - desktop_shell_renders_the_shared_dioxus_workbench_ui
        - file: crates/syu-workbench-server/src/tests.rs
          symbols:
            - index_route_renders_workbench_browser_entrypoint_and_css_asset
            - css_route_serves_the_shared_tailwind_asset
            - server_smoke_covers_root_css_health_and_actions
  - id: REQ-WORKBENCH-007
    title: Rust-native UI and server architecture
    description: |
      The Workbench MUST use a Rust-native UI and server architecture so the
      product can share one source of truth for request intake, goal tracking,
      evidence capture, and assignment state. The implementation SHOULD keep
      the UI and server layers close enough that browser and desktop clients
      stay in sync. CI MUST validate this architecture with Rust-native
      Workbench model, action, server, UI, smoke, and installed-binary API
      checks instead of old browser-app jobs. The browser/server entrypoint
      MUST server-render the shared UI crate with the shared CSS asset while
      keeping localhost-first server security defaults. Tailwind CSS is allowed
      only as a constrained stylesheet build layer for `crates/syu-app-ui/`.
    priority: medium
    status: implemented
    linked_policies:
      - POL-005
    linked_features:
      - FEAT-WORKBENCH-SHELL-001
      - FEAT-WORKBENCH-007
      - FEAT-WORKBENCH-SERVER-001
    tests:
      markdown:
        - file: docs/guide/workbench.md
          symbols:
            - Rust-native UI
            - server architecture
            - browser and desktop
            - Workbench CI
      shell:
        - file: scripts/ci/check-workbench.sh
          symbols:
            - run_workbench_checks
        - file: scripts/ci/check-ui-assets.sh
          symbols:
            - check_ui_assets
        - file: scripts/ci/installed-binary-smoke.sh
          symbols:
            - /api/health
            - /api/actions
```
