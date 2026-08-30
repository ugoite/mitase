---
title: "Workbench implementation / Verification"
description: "Generated reference for docs/mitase/features/workbench/verification.yaml"
---

> Generated from `docs/mitase/features/workbench/verification.yaml`.

## Parsed content

### Schema

- mitase/spec/v1

### Kind

- features

### Namespace

- workbench

### Category

- Workbench implementation

### Features

- **id**: FEAT-WORKBENCH-VERIFICATION-001
  - **title**: Workbench verification tests
  - **summary**: Exercise the real Workbench HTTP server, exact origins, split recovery, and verification targets.
  - **status**: implemented
  - **bindings**:
    - **id**: verification-harness
      - **role**: implementation
      - **facet**: verification
      - **responsibility**: Maintain the executable Workbench HTTP verification harness.
      - **targets**:
        - **id**: verification-endpoint
          - **adapter**: rust
          - **path**: crates/mitase-workbench-server/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: api_verify
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-WORKBENCH-001#criterion.canonical-projection
      - **owns**:
        - **id**: cli-test-module
          - **adapter**: rust
          - **path**: tests/v1_cli.rs
          - **selector**:
            - **kind**: module
            - **name**: v1_cli
        - **id**: workbench-command-test-module
          - **adapter**: rust
          - **path**: tests/workbench_command.rs
          - **selector**:
            - **kind**: module
            - **name**: workbench_command
        - **id**: workbench-smoke-test-module
          - **adapter**: rust
          - **path**: tests/workbench_smoke.rs
          - **selector**:
            - **kind**: module
            - **name**: workbench_smoke
        - **id**: workbench-visual-test-module
          - **adapter**: rust
          - **path**: tests/workbench_visual.rs
          - **selector**:
            - **kind**: module
            - **name**: workbench_visual
        - **id**: test-support-module
          - **adapter**: rust
          - **path**: tests/support/mod.rs
          - **selector**:
            - **kind**: module
            - **name**: mod
    - **id**: test-exposure
      - **role**: verification
      - **facet**: verification
      - **responsibility**: Expose every executable Workbench test as an exact verification target.
      - **targets**:
        - **id**: server-e2e
          - **adapter**: rust
          - **path**: crates/mitase-workbench-server/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: tests::workbench_http_closed_loop_flow
          - **claims**:
            - **kind**: verifies
              - **criterion**: REQ-WORKBENCH-001#criterion.canonical-projection
              - **covers**:
                - FEAT-WORKBENCH-PROJECTION-001#binding.projection/target.project
                - FEAT-WORKBENCH-VERIFICATION-001#binding.verification-harness/target.verification-endpoint
              - **runner**:
                - **runner**: cargo-test
                - **arguments**:
                  - **package**: mitase-workbench-server
                  - **test**: tests::workbench_http_closed_loop_flow
        - **id**: server-transport
          - **adapter**: rust
          - **path**: crates/mitase-workbench-server/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: tests::workbench_http_server_transport_flow
          - **claims**:
            - **kind**: verifies
              - **criterion**: REQ-WORKBENCH-001#criterion.canonical-projection
              - **covers**:
                - FEAT-WORKBENCH-PROJECTION-001#binding.projection/target.project
                - FEAT-WORKBENCH-VERIFICATION-001#binding.verification-harness/target.verification-endpoint
              - **runner**:
                - **runner**: cargo-test
                - **arguments**:
                  - **package**: mitase-workbench-server
                  - **test**: tests::workbench_http_server_transport_flow
        - **id**: command-help
          - **adapter**: rust
          - **path**: tests/workbench_command.rs
          - **selector**:
            - **kind**: symbol
            - **name**: root_cli_does_not_expose_transitional_workbench
          - **claims**:
            - **kind**: verifies
              - **criterion**: REQ-WORKBENCH-002#criterion.work-session
              - **covers**:
                - FEAT-WORKBENCH-WORK-UI-001#binding.work/target.plan
              - **runner**:
                - **runner**: cargo-test-integration
                - **arguments**:
                  - **package**: mitase
                  - **harness**: workbench_command
                  - **test**: root_cli_does_not_expose_transitional_workbench
        - **id**: smoke-projection
          - **adapter**: rust
          - **path**: tests/workbench_smoke.rs
          - **selector**:
            - **kind**: symbol
            - **name**: workbench_projection_is_server_owned_and_starts_not_run
          - **claims**:
            - **kind**: verifies
              - **criterion**: REQ-WORKBENCH-001#criterion.canonical-projection
              - **covers**:
                - FEAT-WORKBENCH-PROJECTION-001#binding.projection/target.project
              - **runner**:
                - **runner**: cargo-test-integration
                - **arguments**:
                  - **package**: mitase
                  - **harness**: workbench_smoke
                  - **test**: workbench_projection_is_server_owned_and_starts_not_run
        - **id**: smoke-module-contract
          - **adapter**: rust
          - **path**: tests/workbench_smoke.rs
          - **selector**:
            - **kind**: symbol
            - **name**: rendered_workbench_uses_external_module_assets_and_specifications_route
          - **claims**:
            - **kind**: verifies
              - **criterion**: REQ-WORKBENCH-006#criterion.accessible-navigation
              - **covers**:
                - FEAT-WORKBENCH-NAVIGATION-001#binding.navigation/target.html-navigation
                - FEAT-WORKBENCH-NAVIGATION-001#binding.navigation/target.accessibility-attributes
                - FEAT-WORKBENCH-NAVIGATION-001#binding.navigation/target.keyboard-navigation
                - FEAT-WORKBENCH-NAVIGATION-001#binding.navigation/target.javascript-navigation
              - **runner**:
                - **runner**: cargo-test-integration
                - **arguments**:
                  - **package**: mitase
                  - **harness**: workbench_smoke
                  - **test**: rendered_workbench_uses_external_module_assets_and_specifications_route
        - **id**: smoke-dto-contract
          - **adapter**: rust
          - **path**: tests/workbench_smoke.rs
          - **selector**:
            - **kind**: symbol
            - **name**: browser_modules_render_dtos_without_model_inference
          - **claims**:
            - **kind**: verifies
              - **criterion**: REQ-WORKBENCH-001#criterion.canonical-projection
              - **covers**:
                - FEAT-WORKBENCH-PROJECTION-001#binding.projection/target.project
              - **runner**:
                - **runner**: cargo-test-integration
                - **arguments**:
                  - **package**: mitase
                  - **harness**: workbench_smoke
                  - **test**: browser_modules_render_dtos_without_model_inference
        - **id**: smoke-keyboard
          - **adapter**: rust
          - **path**: tests/workbench_smoke.rs
          - **selector**:
            - **kind**: symbol
            - **name**: workbench_tabs_are_keyboard_navigable
          - **claims**:
            - **kind**: verifies
              - **criterion**: REQ-WORKBENCH-006#criterion.accessible-navigation
              - **covers**:
                - FEAT-WORKBENCH-NAVIGATION-001#binding.navigation/target.javascript-navigation
                - FEAT-WORKBENCH-NAVIGATION-001#binding.navigation/target.keyboard-navigation
              - **runner**:
                - **runner**: cargo-test-integration
                - **arguments**:
                  - **package**: mitase
                  - **harness**: workbench_smoke
                  - **test**: workbench_tabs_are_keyboard_navigable
        - **id**: visual-dom
          - **adapter**: rust
          - **path**: tests/workbench_visual.rs
          - **selector**:
            - **kind**: symbol
            - **name**: workbench_rendered_dom_uses_projection_driven_placeholders
          - **claims**:
            - **kind**: verifies
              - **criterion**: REQ-WORKBENCH-006#criterion.accessible-navigation
              - **covers**:
                - FEAT-WORKBENCH-NAVIGATION-001#binding.navigation/target.html-navigation
                - FEAT-WORKBENCH-NAVIGATION-001#binding.navigation/target.accessibility-attributes
              - **runner**:
                - **runner**: cargo-test-integration
                - **arguments**:
                  - **package**: mitase
                  - **harness**: workbench_visual
                  - **test**: workbench_rendered_dom_uses_projection_driven_placeholders

## Source YAML

```yaml
schema: mitase/spec/v1
kind: features
namespace: workbench
category: Workbench implementation
features:
- id: FEAT-WORKBENCH-VERIFICATION-001
  title: Workbench verification tests
  summary: Exercise the real Workbench HTTP server, exact origins, split recovery, and verification targets.
  status: implemented
  bindings:
  - id: verification-harness
    role: implementation
    facet: verification
    responsibility: Maintain the executable Workbench HTTP verification harness.
    targets:
    - id: verification-endpoint
      adapter: rust
      path: crates/mitase-workbench-server/src/lib.rs
      selector:
        kind: symbol
        name: api_verify
      claims:
      - kind: satisfies
        criterion: REQ-WORKBENCH-001#criterion.canonical-projection
    owns:
    - id: cli-test-module
      adapter: rust
      path: tests/v1_cli.rs
      selector:
        kind: module
        name: v1_cli
    - id: workbench-command-test-module
      adapter: rust
      path: tests/workbench_command.rs
      selector:
        kind: module
        name: workbench_command
    - id: workbench-smoke-test-module
      adapter: rust
      path: tests/workbench_smoke.rs
      selector:
        kind: module
        name: workbench_smoke
    - id: workbench-visual-test-module
      adapter: rust
      path: tests/workbench_visual.rs
      selector:
        kind: module
        name: workbench_visual
    - id: test-support-module
      adapter: rust
      path: tests/support/mod.rs
      selector:
        kind: module
        name: mod
  - id: test-exposure
    role: verification
    facet: verification
    responsibility: Expose every executable Workbench test as an exact verification target.
    targets:
    - id: server-e2e
      adapter: rust
      path: crates/mitase-workbench-server/src/lib.rs
      selector:
        kind: symbol
        name: tests::workbench_http_closed_loop_flow
      claims:
      - kind: verifies
        criterion: REQ-WORKBENCH-001#criterion.canonical-projection
        covers:
        - FEAT-WORKBENCH-PROJECTION-001#binding.projection/target.project
        - FEAT-WORKBENCH-VERIFICATION-001#binding.verification-harness/target.verification-endpoint
        runner:
          runner: cargo-test
          arguments:
            package: mitase-workbench-server
            test: tests::workbench_http_closed_loop_flow
    - id: server-transport
      adapter: rust
      path: crates/mitase-workbench-server/src/lib.rs
      selector:
        kind: symbol
        name: tests::workbench_http_server_transport_flow
      claims:
      - kind: verifies
        criterion: REQ-WORKBENCH-001#criterion.canonical-projection
        covers:
        - FEAT-WORKBENCH-PROJECTION-001#binding.projection/target.project
        - FEAT-WORKBENCH-VERIFICATION-001#binding.verification-harness/target.verification-endpoint
        runner:
          runner: cargo-test
          arguments:
            package: mitase-workbench-server
            test: tests::workbench_http_server_transport_flow
    - id: command-help
      adapter: rust
      path: tests/workbench_command.rs
      selector:
        kind: symbol
        name: root_cli_does_not_expose_transitional_workbench
      claims:
      - kind: verifies
        criterion: REQ-WORKBENCH-002#criterion.work-session
        covers:
        - FEAT-WORKBENCH-WORK-UI-001#binding.work/target.plan
        runner:
          runner: cargo-test-integration
          arguments:
            package: mitase
            harness: workbench_command
            test: root_cli_does_not_expose_transitional_workbench
    - id: smoke-projection
      adapter: rust
      path: tests/workbench_smoke.rs
      selector:
        kind: symbol
        name: workbench_projection_is_server_owned_and_starts_not_run
      claims:
      - kind: verifies
        criterion: REQ-WORKBENCH-001#criterion.canonical-projection
        covers:
        - FEAT-WORKBENCH-PROJECTION-001#binding.projection/target.project
        runner:
          runner: cargo-test-integration
          arguments:
            package: mitase
            harness: workbench_smoke
            test: workbench_projection_is_server_owned_and_starts_not_run
    - id: smoke-module-contract
      adapter: rust
      path: tests/workbench_smoke.rs
      selector:
        kind: symbol
        name: rendered_workbench_uses_external_module_assets_and_specifications_route
      claims:
      - kind: verifies
        criterion: REQ-WORKBENCH-006#criterion.accessible-navigation
        covers:
        - FEAT-WORKBENCH-NAVIGATION-001#binding.navigation/target.html-navigation
        - FEAT-WORKBENCH-NAVIGATION-001#binding.navigation/target.accessibility-attributes
        - FEAT-WORKBENCH-NAVIGATION-001#binding.navigation/target.keyboard-navigation
        - FEAT-WORKBENCH-NAVIGATION-001#binding.navigation/target.javascript-navigation
        runner:
          runner: cargo-test-integration
          arguments:
            package: mitase
            harness: workbench_smoke
            test: rendered_workbench_uses_external_module_assets_and_specifications_route
    - id: smoke-dto-contract
      adapter: rust
      path: tests/workbench_smoke.rs
      selector:
        kind: symbol
        name: browser_modules_render_dtos_without_model_inference
      claims:
      - kind: verifies
        criterion: REQ-WORKBENCH-001#criterion.canonical-projection
        covers:
        - FEAT-WORKBENCH-PROJECTION-001#binding.projection/target.project
        runner:
          runner: cargo-test-integration
          arguments:
            package: mitase
            harness: workbench_smoke
            test: browser_modules_render_dtos_without_model_inference
    - id: smoke-keyboard
      adapter: rust
      path: tests/workbench_smoke.rs
      selector:
        kind: symbol
        name: workbench_tabs_are_keyboard_navigable
      claims:
      - kind: verifies
        criterion: REQ-WORKBENCH-006#criterion.accessible-navigation
        covers:
        - FEAT-WORKBENCH-NAVIGATION-001#binding.navigation/target.javascript-navigation
        - FEAT-WORKBENCH-NAVIGATION-001#binding.navigation/target.keyboard-navigation
        runner:
          runner: cargo-test-integration
          arguments:
            package: mitase
            harness: workbench_smoke
            test: workbench_tabs_are_keyboard_navigable
    - id: visual-dom
      adapter: rust
      path: tests/workbench_visual.rs
      selector:
        kind: symbol
        name: workbench_rendered_dom_uses_projection_driven_placeholders
      claims:
      - kind: verifies
        criterion: REQ-WORKBENCH-006#criterion.accessible-navigation
        covers:
        - FEAT-WORKBENCH-NAVIGATION-001#binding.navigation/target.html-navigation
        - FEAT-WORKBENCH-NAVIGATION-001#binding.navigation/target.accessibility-attributes
        runner:
          runner: cargo-test-integration
          arguments:
            package: mitase
            harness: workbench_visual
            test: workbench_rendered_dom_uses_projection_driven_placeholders
```
