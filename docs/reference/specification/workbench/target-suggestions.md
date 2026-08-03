---
title: "Workbench implementation / Target Suggestions"
description: "Generated reference for docs/syu/features/workbench/target-suggestions.yaml"
---

> Generated from `docs/syu/features/workbench/target-suggestions.yaml`.

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

- **id**: FEAT-WORKBENCH-TARGET-SUGGESTIONS-001
  - **title**: Workbench target suggestions
  - **summary**: Rank exact targets with evidence, persist reviewed approvals, and consume them into WorkRequest scope only through a server-projected exact-origin Create Work capability.
  - **status**: implemented
  - **bindings**:
    - **id**: suggestions
      - **role**: implementation
      - **facet**: planning
      - **responsibility**: Derive, review, reject, and approve exact target candidates without silently widening executable scope.
      - **owns**:
        - **id**: planner-implemented-missing-target-test
          - **adapter**: rust
          - **path**: crates/syu-planner/src/lib.rs
          - **selector**:
            - **kind**: module
            - **name**: lib::tests::implemented_missing_exact_target_is_not_reframed_as_add
        - **id**: server-target-suggestions-api
          - **adapter**: rust
          - **path**: crates/syu-workbench-server/src/lib.rs
          - **selector**:
            - **kind**: module
            - **name**: lib::api_target_suggestions
      - **targets**:
        - **id**: rank-candidates
          - **adapter**: rust
          - **path**: crates/syu-planner/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: suggest_targets
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-WORKBENCH-008#criterion.reviewed-target-suggestions
        - **id**: approve-candidates
          - **adapter**: rust
          - **path**: crates/syu-workbench-server/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: api_target_suggestions_approve
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-WORKBENCH-008#criterion.reviewed-target-suggestions
        - **id**: suggestion-review-ui
          - **adapter**: javascript
          - **path**: crates/syu-app-ui/assets/js/pages/specifications.js
          - **selector**:
            - **kind**: symbol
            - **name**: renderTargetSuggestions
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-WORKBENCH-008#criterion.reviewed-target-suggestions

## Source YAML

```yaml
schema: syu/spec/v1
kind: features
namespace: workbench
category: Workbench implementation
features:
- id: FEAT-WORKBENCH-TARGET-SUGGESTIONS-001
  title: Workbench target suggestions
  summary: Rank exact targets with evidence, persist reviewed approvals, and consume them into WorkRequest scope only through a server-projected exact-origin Create Work capability.
  status: implemented
  bindings:
  - id: suggestions
    role: implementation
    facet: planning
    responsibility: Derive, review, reject, and approve exact target candidates without silently widening executable scope.
    owns:
    - id: planner-implemented-missing-target-test
      adapter: rust
      path: crates/syu-planner/src/lib.rs
      selector: { kind: module, name: 'lib::tests::implemented_missing_exact_target_is_not_reframed_as_add' }
    - id: server-target-suggestions-api
      adapter: rust
      path: crates/syu-workbench-server/src/lib.rs
      selector: { kind: module, name: 'lib::api_target_suggestions' }
    targets:
    - id: rank-candidates
      adapter: rust
      path: crates/syu-planner/src/lib.rs
      selector:
        kind: symbol
        name: suggest_targets
      claims:
      - kind: satisfies
        criterion: REQ-WORKBENCH-008#criterion.reviewed-target-suggestions
    - id: approve-candidates
      adapter: rust
      path: crates/syu-workbench-server/src/lib.rs
      selector:
        kind: symbol
        name: api_target_suggestions_approve
      claims:
      - kind: satisfies
        criterion: REQ-WORKBENCH-008#criterion.reviewed-target-suggestions
    - id: suggestion-review-ui
      adapter: javascript
      path: crates/syu-app-ui/assets/js/pages/specifications.js
      selector:
        kind: symbol
        name: renderTargetSuggestions
      claims:
      - kind: satisfies
        criterion: REQ-WORKBENCH-008#criterion.reviewed-target-suggestions
```
