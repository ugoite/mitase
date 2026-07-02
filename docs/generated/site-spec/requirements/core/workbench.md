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
  - **title**: Page-oriented, human-readable Workbench
  - **description**:
    - |
      The Workbench MUST keep the command palette as its keyboard-first action
      surface while exposing only four persistent role menus in this order:
      Work, Scope, Items, and Diagnostics. Work MUST be the default. A selected
      palette command MUST resolve to a page, section, entity, stable component
      anchor, focus intent, and safe execution policy instead of opening a
      generic command result. Items MUST provide linked-item navigation,
      source-preserving item editing with mandatory diff review, reciprocal
      adjacent-layer link updates, Item-driven Work and scope entry points, and
      workspace initialization when specs do not exist. Diagnostics MUST use a
      single action to queue all checks and update item status through jobs and
      SSE. Settings MUST be a full-page utility opened from the gear and MUST
      preview, validate, stale-check, and source-preserve syu.yaml changes.
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
          - Stable pages
          - Explorer Frame
          - CommandTarget navigation
          - Live state and APIs
    - **rust**:
      - **file**: tests/workbench_smoke.rs
        - **symbols**:
          - sidebar_has_only_four_roles_in_required_order
          - work_is_human_readable_default_page
          - item_draft_uses_the_same_detail_canvas
          - palette_history_targets_work_evidence
          - diagnostics_and_settings_have_localized_japanese_copy
      - **file**: tests/workbench_command.rs
        - **symbols**:
          - workbench_help_lists_browser_launch_options
      - **file**: crates/syu-workbench-server/src/tests.rs
        - **symbols**:
          - index_renders_new_page_contract_and_navigation_script
          - diagnostics_run_exposes_job_lifecycle_and_evidence
          - item_driven_work_records_item_source_and_goal
          - settings_preview_and_apply_preserve_comments_and_unknown_fields
      - **file**: src/command/workbench.rs
        - **symbols**:
          - uninitialized_directory_resolves_to_a_workbench_launch
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
          - work_is_human_readable_default_page
          - palette_history_targets_work_evidence
      - **file**: crates/syu-app-ui/src/model/navigation.rs
        - **symbols**:
          - every_palette_command_has_one_page_target
          - legacy_page_slugs_are_rejected
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
          - work_is_human_readable_default_page
          - palette_history_targets_work_evidence
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
      - **file**: crates/syu-app-ui/src/components/pages/scope.rs
        - **symbols**:
          - ScopePage
          - SliceDetail
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
      - **file**: crates/syu-app-ui/src/components/pages/work.rs
        - **symbols**:
          - WorkDetails
          - VerificationSection
          - Evidence
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
          - Browser, desktop, and CI
          - same AppShell
    - **rust**:
      - **file**: crates/syu-desktop/src/lib.rs
        - **symbols**:
          - desktop_bridge_uses_the_shared_action_registry
          - desktop_shell_renders_the_shared_dioxus_workbench_ui
      - **file**: crates/syu-workbench-server/src/tests.rs
        - **symbols**:
          - index_renders_new_page_contract_and_navigation_script
          - css_and_event_stream_are_available
          - health_and_actions_endpoints_report_live_server_state
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
          - Rust-native Dioxus
          - Browser, desktop, and CI
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
    title: Page-oriented, human-readable Workbench
    description: |
      The Workbench MUST keep the command palette as its keyboard-first action
      surface while exposing only four persistent role menus in this order:
      Work, Scope, Items, and Diagnostics. Work MUST be the default. A selected
      palette command MUST resolve to a page, section, entity, stable component
      anchor, focus intent, and safe execution policy instead of opening a
      generic command result. Items MUST provide linked-item navigation,
      source-preserving item editing with mandatory diff review, reciprocal
      adjacent-layer link updates, Item-driven Work and scope entry points, and
      workspace initialization when specs do not exist. Diagnostics MUST use a
      single action to queue all checks and update item status through jobs and
      SSE. Settings MUST be a full-page utility opened from the gear and MUST
      preview, validate, stale-check, and source-preserve syu.yaml changes.
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
            - Stable pages
            - Explorer Frame
            - CommandTarget navigation
            - Live state and APIs
      rust:
        - file: tests/workbench_smoke.rs
          symbols:
            - sidebar_has_only_four_roles_in_required_order
            - work_is_human_readable_default_page
            - item_draft_uses_the_same_detail_canvas
            - palette_history_targets_work_evidence
            - diagnostics_and_settings_have_localized_japanese_copy
        - file: tests/workbench_command.rs
          symbols:
            - workbench_help_lists_browser_launch_options
        - file: crates/syu-workbench-server/src/tests.rs
          symbols:
            - index_renders_new_page_contract_and_navigation_script
            - diagnostics_run_exposes_job_lifecycle_and_evidence
            - item_driven_work_records_item_source_and_goal
            - settings_preview_and_apply_preserve_comments_and_unknown_fields
        - file: src/command/workbench.rs
          symbols:
            - uninitialized_directory_resolves_to_a_workbench_launch
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
            - work_is_human_readable_default_page
            - palette_history_targets_work_evidence
        - file: crates/syu-app-ui/src/model/navigation.rs
          symbols:
            - every_palette_command_has_one_page_target
            - legacy_page_slugs_are_rejected
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
            - work_is_human_readable_default_page
            - palette_history_targets_work_evidence
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
        - file: crates/syu-app-ui/src/components/pages/scope.rs
          symbols:
            - ScopePage
            - SliceDetail
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
        - file: crates/syu-app-ui/src/components/pages/work.rs
          symbols:
            - WorkDetails
            - VerificationSection
            - Evidence
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
            - Browser, desktop, and CI
            - same AppShell
      rust:
        - file: crates/syu-desktop/src/lib.rs
          symbols:
            - desktop_bridge_uses_the_shared_action_registry
            - desktop_shell_renders_the_shared_dioxus_workbench_ui
        - file: crates/syu-workbench-server/src/tests.rs
          symbols:
            - index_renders_new_page_contract_and_navigation_script
            - css_and_event_stream_are_available
            - health_and_actions_endpoints_report_live_server_state
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
            - Rust-native Dioxus
            - Browser, desktop, and CI
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
