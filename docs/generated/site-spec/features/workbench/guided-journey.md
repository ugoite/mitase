---
title: "Workbench implementation / Guided Journey"
description: "Generated reference for docs/syu/features/workbench/guided-journey.yaml"
---

> Generated from `docs/syu/features/workbench/guided-journey.yaml`.

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

- **id**: FEAT-WORKBENCH-GUIDED-JOURNEY-001
  - **title**: Guided non-programmer journey
  - **summary**: Project a safe change lifecycle as one explained next action at a time.
  - **status**: implemented
  - **bindings**:
    - **id**: journey
      - **role**: implementation
      - **facet**: workbench-journey
      - **responsibility**: Build the server-owned guided work projection and typed action boundary.
      - **targets**:
        - **id**: journey-projection
          - **adapter**: rust
          - **path**: crates/syu-workbench-server/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: WorkJourneyView
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-WORKBENCH-013#criterion.guided-journey
            - **kind**: satisfies
              - **criterion**: REQ-WORKBENCH-013#criterion.linked-specification-context
        - **id**: journey-action
          - **adapter**: rust
          - **path**: crates/syu-workbench-server/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: api_journey_action
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-WORKBENCH-013#criterion.guided-journey
            - **kind**: satisfies
              - **criterion**: REQ-WORKBENCH-013#criterion.linked-specification-context
        - **id**: journey-source
          - **adapter**: rust
          - **path**: crates/syu-workbench-server/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: api_source
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-WORKBENCH-013#criterion.linked-specification-context
        - **id**: journey-browser
          - **adapter**: declared
          - **path**: crates/syu-app-ui/assets/js/pages/work.js
          - **selector**:
            - **kind**: file
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-WORKBENCH-013#criterion.guided-journey
            - **kind**: satisfies
              - **criterion**: REQ-WORKBENCH-013#criterion.linked-specification-context
    - **id**: journey-verification
      - **role**: verification
      - **facet**: workbench-journey
      - **responsibility**: Verify guided action state and cancellation.
      - **targets**:
        - **id**: journey-test
          - **adapter**: rust
          - **path**: crates/syu-workbench-server/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: tests::journey_action_exposes_one_friendly_next_step_and_can_cancel
          - **claims**:
            - **kind**: verifies
              - **criterion**: REQ-WORKBENCH-013#criterion.guided-journey
              - **covers**:
                - FEAT-WORKBENCH-GUIDED-JOURNEY-001#binding.journey/target.journey-projection
                - FEAT-WORKBENCH-GUIDED-JOURNEY-001#binding.journey/target.journey-action
                - FEAT-WORKBENCH-GUIDED-JOURNEY-001#binding.journey/target.journey-browser
              - **runner**:
                - **runner**: cargo-test
                - **arguments**:
                  - **package**: syu-workbench-server
                  - **test**: tests::journey_action_exposes_one_friendly_next_step_and_can_cancel
            - **kind**: verifies
              - **criterion**: REQ-WORKBENCH-013#criterion.linked-specification-context
              - **covers**:
                - FEAT-WORKBENCH-GUIDED-JOURNEY-001#binding.journey/target.journey-projection
                - FEAT-WORKBENCH-GUIDED-JOURNEY-001#binding.journey/target.journey-action
                - FEAT-WORKBENCH-GUIDED-JOURNEY-001#binding.journey/target.journey-source
                - FEAT-WORKBENCH-GUIDED-JOURNEY-001#binding.journey/target.journey-browser
              - **runner**:
                - **runner**: cargo-test
                - **arguments**:
                  - **package**: syu-workbench-server
                  - **test**: tests::journey_action_exposes_one_friendly_next_step_and_can_cancel

## Source YAML

```yaml
schema: syu/spec/v1
kind: features
namespace: workbench
category: Workbench implementation
features:
- id: FEAT-WORKBENCH-GUIDED-JOURNEY-001
  title: Guided non-programmer journey
  summary: Project a safe change lifecycle as one explained next action at a time.
  status: implemented
  bindings:
  - id: journey
    role: implementation
    facet: workbench-journey
    responsibility: Build the server-owned guided work projection and typed action boundary.
    targets:
    - id: journey-projection
      adapter: rust
      path: crates/syu-workbench-server/src/lib.rs
      selector: { kind: symbol, name: WorkJourneyView }
      claims:
      - kind: satisfies
        criterion: REQ-WORKBENCH-013#criterion.guided-journey
      - kind: satisfies
        criterion: REQ-WORKBENCH-013#criterion.linked-specification-context
    - id: journey-action
      adapter: rust
      path: crates/syu-workbench-server/src/lib.rs
      selector: { kind: symbol, name: api_journey_action }
      claims:
      - kind: satisfies
        criterion: REQ-WORKBENCH-013#criterion.guided-journey
      - kind: satisfies
        criterion: REQ-WORKBENCH-013#criterion.linked-specification-context
    - id: journey-source
      adapter: rust
      path: crates/syu-workbench-server/src/lib.rs
      selector: { kind: symbol, name: api_source }
      claims:
      - kind: satisfies
        criterion: REQ-WORKBENCH-013#criterion.linked-specification-context
    - id: journey-browser
      adapter: declared
      path: crates/syu-app-ui/assets/js/pages/work.js
      selector: { kind: file }
      claims:
      - kind: satisfies
        criterion: REQ-WORKBENCH-013#criterion.guided-journey
      - kind: satisfies
        criterion: REQ-WORKBENCH-013#criterion.linked-specification-context
  - id: journey-verification
    role: verification
    facet: workbench-journey
    responsibility: Verify guided action state and cancellation.
    targets:
    - id: journey-test
      adapter: rust
      path: crates/syu-workbench-server/src/lib.rs
      selector: { kind: symbol, name: tests::journey_action_exposes_one_friendly_next_step_and_can_cancel }
      claims:
      - kind: verifies
        criterion: REQ-WORKBENCH-013#criterion.guided-journey
        covers:
        - FEAT-WORKBENCH-GUIDED-JOURNEY-001#binding.journey/target.journey-projection
        - FEAT-WORKBENCH-GUIDED-JOURNEY-001#binding.journey/target.journey-action
        - FEAT-WORKBENCH-GUIDED-JOURNEY-001#binding.journey/target.journey-browser
        runner: { runner: cargo-test, arguments: { package: syu-workbench-server, test: tests::journey_action_exposes_one_friendly_next_step_and_can_cancel } }
      - kind: verifies
        criterion: REQ-WORKBENCH-013#criterion.linked-specification-context
        covers:
        - FEAT-WORKBENCH-GUIDED-JOURNEY-001#binding.journey/target.journey-projection
        - FEAT-WORKBENCH-GUIDED-JOURNEY-001#binding.journey/target.journey-action
        - FEAT-WORKBENCH-GUIDED-JOURNEY-001#binding.journey/target.journey-source
        - FEAT-WORKBENCH-GUIDED-JOURNEY-001#binding.journey/target.journey-browser
        runner: { runner: cargo-test, arguments: { package: syu-workbench-server, test: tests::journey_action_exposes_one_friendly_next_step_and_can_cancel } }
```
