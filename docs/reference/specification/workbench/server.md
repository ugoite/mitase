---
title: "Workbench implementation / Server"
description: "Generated reference for docs/syu/features/workbench/server.yaml"
---

> Generated from `docs/syu/features/workbench/server.yaml`.

## Parsed content

### Schema

- syu/spec/v1

### Kind

- features

### Namespace

- workbench

### Category

- Workbench implementation

### Features

- **id**: FEAT-WORKBENCH-SERVER-001
  - **title**: Workbench server
  - **summary**: Expose canonical read, exact-origin Work creation and slice selection, validation, edit, and security APIs.
  - **status**: implemented
  - **bindings**:
    - **id**: server
      - **role**: implementation
      - **facet**: server
      - **responsibility**: Serve canonical Workbench HTTP routes and mutation guards.
      - **owns**:
        - **id**: server-homogeneous-transitions
          - **adapter**: rust
          - **path**: crates/syu-workbench-server/src/lib.rs
          - **selector**:
            - **kind**: module
            - **name**: lib::ensure_homogeneous_approved_transitions
        - **id**: server-journey-steps
          - **adapter**: rust
          - **path**: crates/syu-workbench-server/src/lib.rs
          - **selector**:
            - **kind**: module
            - **name**: lib::journey_steps
        - **id**: server-readiness-view
          - **adapter**: rust
          - **path**: crates/syu-workbench-server/src/lib.rs
          - **selector**:
            - **kind**: module
            - **name**: lib::readiness_view
        - **id**: server-resolve-approved
          - **adapter**: rust
          - **path**: crates/syu-workbench-server/src/lib.rs
          - **selector**:
            - **kind**: module
            - **name**: lib::resolve_approved_target_candidates
        - **id**: server-resolve-requested
          - **adapter**: rust
          - **path**: crates/syu-workbench-server/src/lib.rs
          - **selector**:
            - **kind**: module
            - **name**: lib::resolve_requested_targets
        - **id**: server-approved-resolution-test
          - **adapter**: rust
          - **path**: crates/syu-workbench-server/src/lib.rs
          - **selector**:
            - **kind**: module
            - **name**: lib::tests::approved_target_resolution_fails_closed_and_filters_stale_evidence
        - **id**: server-copy-workspace-test
          - **adapter**: rust
          - **path**: crates/syu-workbench-server/src/lib.rs
          - **selector**:
            - **kind**: module
            - **name**: lib::tests::copy_workspace_tree
        - **id**: server-approved-work-test
          - **adapter**: rust
          - **path**: crates/syu-workbench-server/src/lib.rs
          - **selector**:
            - **kind**: module
            - **name**: lib::tests::create_approved_work_request
        - **id**: server-journey-verify-test
          - **adapter**: rust
          - **path**: crates/syu-workbench-server/src/lib.rs
          - **selector**:
            - **kind**: module
            - **name**: lib::tests::journey_verify_returns_a_fresh_projection_after_editable_change
        - **id**: server-fixture-post-state-test
          - **adapter**: rust
          - **path**: crates/syu-workbench-server/src/lib.rs
          - **selector**:
            - **kind**: module
            - **name**: lib::tests::run_fixture_post_state_flow
        - **id**: server-agent-start-test
          - **adapter**: rust
          - **path**: crates/syu-workbench-server/src/lib.rs
          - **selector**:
            - **kind**: module
            - **name**: lib::tests::start_lifecycle_agent
        - **id**: server-agent-request-test
          - **adapter**: rust
          - **path**: crates/syu-workbench-server/src/lib.rs
          - **selector**:
            - **kind**: module
            - **name**: lib::tests::start_lifecycle_agent_with_request
        - **id**: server-tests-module
          - **adapter**: rust
          - **path**: crates/syu-workbench-server/src/lib.rs
          - **selector**:
            - **kind**: module
            - **name**: lib::tests[cfg(test)]
        - **id**: server-origin-capabilities
          - **adapter**: rust
          - **path**: crates/syu-workbench-server/src/lib.rs
          - **selector**:
            - **kind**: module
            - **name**: lib::origin_capabilities
        - **id**: server-validate-origin
          - **adapter**: rust
          - **path**: crates/syu-workbench-server/src/lib.rs
          - **selector**:
            - **kind**: module
            - **name**: lib::validate_work_origin
        - **id**: server-split-recovery
          - **adapter**: rust
          - **path**: crates/syu-workbench-server/src/lib.rs
          - **selector**:
            - **kind**: module
            - **name**: lib::split_recovery_view
        - **id**: server-select-slice
          - **adapter**: rust
          - **path**: crates/syu-workbench-server/src/lib.rs
          - **selector**:
            - **kind**: module
            - **name**: lib::select_slice
        - **id**: server-origin-validation-test
          - **adapter**: rust
          - **path**: crates/syu-workbench-server/src/lib.rs
          - **selector**:
            - **kind**: module
            - **name**: lib::tests::create_work_requires_an_exact_implemented_requirement_criterion
      - **targets**:
        - **id**: mutation-guard
          - **adapter**: rust
          - **path**: crates/syu-workbench-server/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: mutation_guard
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-WORKBENCH-005#criterion.secure-local-server
        - **id**: workspace-snapshot
          - **adapter**: rust
          - **path**: crates/syu-workbench-server/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: snapshot
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-WORKBENCH-012#criterion.exact-snapshot-reuse

## Source YAML

```yaml
schema: syu/spec/v1
kind: features
namespace: workbench
category: Workbench implementation
features:
- id: FEAT-WORKBENCH-SERVER-001
  title: Workbench server
  summary: Expose canonical read, exact-origin Work creation and slice selection, validation, edit, and security APIs.
  status: implemented
  bindings:
  - id: server
    role: implementation
    facet: server
    responsibility: Serve canonical Workbench HTTP routes and mutation guards.
    owns:
    - id: server-homogeneous-transitions
      adapter: rust
      path: crates/syu-workbench-server/src/lib.rs
      selector: { kind: module, name: 'lib::ensure_homogeneous_approved_transitions' }
    - id: server-journey-steps
      adapter: rust
      path: crates/syu-workbench-server/src/lib.rs
      selector: { kind: module, name: 'lib::journey_steps' }
    - id: server-readiness-view
      adapter: rust
      path: crates/syu-workbench-server/src/lib.rs
      selector: { kind: module, name: 'lib::readiness_view' }
    - id: server-resolve-approved
      adapter: rust
      path: crates/syu-workbench-server/src/lib.rs
      selector: { kind: module, name: 'lib::resolve_approved_target_candidates' }
    - id: server-resolve-requested
      adapter: rust
      path: crates/syu-workbench-server/src/lib.rs
      selector: { kind: module, name: 'lib::resolve_requested_targets' }
    - id: server-approved-resolution-test
      adapter: rust
      path: crates/syu-workbench-server/src/lib.rs
      selector: { kind: module, name: 'lib::tests::approved_target_resolution_fails_closed_and_filters_stale_evidence' }
    - id: server-copy-workspace-test
      adapter: rust
      path: crates/syu-workbench-server/src/lib.rs
      selector: { kind: module, name: 'lib::tests::copy_workspace_tree' }
    - id: server-approved-work-test
      adapter: rust
      path: crates/syu-workbench-server/src/lib.rs
      selector: { kind: module, name: 'lib::tests::create_approved_work_request' }
    - id: server-journey-verify-test
      adapter: rust
      path: crates/syu-workbench-server/src/lib.rs
      selector: { kind: module, name: 'lib::tests::journey_verify_returns_a_fresh_projection_after_editable_change' }
    - id: server-fixture-post-state-test
      adapter: rust
      path: crates/syu-workbench-server/src/lib.rs
      selector: { kind: module, name: 'lib::tests::run_fixture_post_state_flow' }
    - id: server-agent-start-test
      adapter: rust
      path: crates/syu-workbench-server/src/lib.rs
      selector: { kind: module, name: 'lib::tests::start_lifecycle_agent' }
    - id: server-agent-request-test
      adapter: rust
      path: crates/syu-workbench-server/src/lib.rs
      selector: { kind: module, name: 'lib::tests::start_lifecycle_agent_with_request' }
    - id: server-tests-module
      adapter: rust
      path: crates/syu-workbench-server/src/lib.rs
      selector: { kind: module, name: 'lib::tests[cfg(test)]' }
    - id: server-origin-capabilities
      adapter: rust
      path: crates/syu-workbench-server/src/lib.rs
      selector: { kind: module, name: 'lib::origin_capabilities' }
    - id: server-validate-origin
      adapter: rust
      path: crates/syu-workbench-server/src/lib.rs
      selector: { kind: module, name: 'lib::validate_work_origin' }
    - id: server-split-recovery
      adapter: rust
      path: crates/syu-workbench-server/src/lib.rs
      selector: { kind: module, name: 'lib::split_recovery_view' }
    - id: server-select-slice
      adapter: rust
      path: crates/syu-workbench-server/src/lib.rs
      selector: { kind: module, name: 'lib::select_slice' }
    - id: server-origin-validation-test
      adapter: rust
      path: crates/syu-workbench-server/src/lib.rs
      selector: { kind: module, name: 'lib::tests::create_work_requires_an_exact_implemented_requirement_criterion' }
    targets:
    - id: mutation-guard
      adapter: rust
      path: crates/syu-workbench-server/src/lib.rs
      selector:
        kind: symbol
        name: mutation_guard
      claims:
      - kind: satisfies
        criterion: REQ-WORKBENCH-005#criterion.secure-local-server
    - id: workspace-snapshot
      adapter: rust
      path: crates/syu-workbench-server/src/lib.rs
      selector:
        kind: symbol
        name: snapshot
      claims:
      - kind: satisfies
        criterion: REQ-WORKBENCH-012#criterion.exact-snapshot-reuse
```
