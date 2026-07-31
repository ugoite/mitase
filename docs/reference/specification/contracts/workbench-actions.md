---
title: "Public entrypoint contracts / Workbench Actions"
description: "Generated reference for docs/syu/features/public-entrypoints/workbench-actions.yaml"
---

> Generated from `docs/syu/features/public-entrypoints/workbench-actions.yaml`.

## Parsed content

### Schema

- syu/spec/v1

### Kind

- features

### Namespace

- public

### Category

- Public entrypoint contracts

### Features

- **id**: FEAT-PUBLIC-WORKBENCH-ACTIONS-001
  - **title**: Workbench client actions
  - **summary**: Govern typed Workbench preview, approval, diagnostics, readiness, and search actions.
  - **status**: implemented
  - **bindings**:
    - **id**: public-api-005
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint exactly addressable and linked to its verified capability boundary.
      - **targets**:
        - **id**: entrypoint-005-javascript-crates-syu-app-ui-assets-js-api-js-runjourn
          - **adapter**: javascript
          - **path**: crates/syu-app-ui/assets/js/api.js
          - **selector**:
            - **kind**: symbol
            - **name**: runJourneyAction
          - **claims**:
            - **kind**: exposes
              - **target**: FEAT-WORKBENCH-WORK-UI-001#binding.work/target.plan
    - **id**: public-api-apply-specification-candidate
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Govern the typed specification apply entrypoint.
      - **targets**:
        - **id**: entrypoint-apply-specification-candidate
          - **adapter**: javascript
          - **path**: crates/syu-app-ui/assets/js/api.js
          - **selector**:
            - **kind**: symbol
            - **name**: applySpecificationCandidate
          - **claims**:
            - **kind**: exposes
              - **target**: FEAT-WORKBENCH-WORK-UI-001#binding.work/target.plan
    - **id**: public-api-approve-target-suggestions
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Govern the target suggestion approval entrypoint.
      - **targets**:
        - **id**: entrypoint-approve-target-suggestions
          - **adapter**: javascript
          - **path**: crates/syu-app-ui/assets/js/api.js
          - **selector**:
            - **kind**: symbol
            - **name**: approveTargetSuggestions
          - **claims**:
            - **kind**: exposes
              - **target**: FEAT-WORKBENCH-WORK-UI-001#binding.work/target.plan
    - **id**: public-api-preview-specification-candidate
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Govern typed specification preview.
      - **targets**:
        - **id**: entrypoint-preview-specification-candidate
          - **adapter**: javascript
          - **path**: crates/syu-app-ui/assets/js/api.js
          - **selector**:
            - **kind**: symbol
            - **name**: previewSpecificationCandidate
          - **claims**:
            - **kind**: exposes
              - **target**: FEAT-WORKBENCH-WORK-UI-001#binding.work/target.plan
    - **id**: public-api-read-target-suggestions
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Govern target suggestion reads.
      - **targets**:
        - **id**: entrypoint-read-target-suggestions
          - **adapter**: javascript
          - **path**: crates/syu-app-ui/assets/js/api.js
          - **selector**:
            - **kind**: symbol
            - **name**: readTargetSuggestions
          - **claims**:
            - **kind**: exposes
              - **target**: FEAT-WORKBENCH-WORK-UI-001#binding.work/target.plan
    - **id**: public-api-reject-target-suggestion
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Govern target suggestion rejection.
      - **targets**:
        - **id**: entrypoint-reject-target-suggestion
          - **adapter**: javascript
          - **path**: crates/syu-app-ui/assets/js/api.js
          - **selector**:
            - **kind**: symbol
            - **name**: rejectTargetSuggestion
          - **claims**:
            - **kind**: exposes
              - **target**: FEAT-WORKBENCH-WORK-UI-001#binding.work/target.plan
    - **id**: public-api-run-diagnostics
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Govern canonical diagnostic execution.
      - **targets**:
        - **id**: entrypoint-run-diagnostics
          - **adapter**: javascript
          - **path**: crates/syu-app-ui/assets/js/api.js
          - **selector**:
            - **kind**: symbol
            - **name**: runDiagnostics
          - **claims**:
            - **kind**: exposes
              - **target**: FEAT-WORKBENCH-WORK-UI-001#binding.work/target.plan
    - **id**: public-api-run-readiness
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Govern canonical readiness execution.
      - **targets**:
        - **id**: entrypoint-run-readiness
          - **adapter**: javascript
          - **path**: crates/syu-app-ui/assets/js/api.js
          - **selector**:
            - **kind**: symbol
            - **name**: runReadiness
          - **claims**:
            - **kind**: exposes
              - **target**: FEAT-WORKBENCH-WORK-UI-001#binding.work/target.plan
    - **id**: public-api-search-specification-candidates
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Govern typed specification search.
      - **targets**:
        - **id**: entrypoint-search-specification-candidates
          - **adapter**: javascript
          - **path**: crates/syu-app-ui/assets/js/api.js
          - **selector**:
            - **kind**: symbol
            - **name**: searchSpecificationCandidates
          - **claims**:
            - **kind**: exposes
              - **target**: FEAT-WORKBENCH-WORK-UI-001#binding.work/target.plan

## Source YAML

```yaml
schema: syu/spec/v1
kind: features
namespace: public
category: Public entrypoint contracts
features:
- id: FEAT-PUBLIC-WORKBENCH-ACTIONS-001
  title: Workbench client actions
  summary: Govern typed Workbench preview, approval, diagnostics, readiness, and search
    actions.
  status: implemented
  bindings:
  - id: public-api-005
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint exactly addressable and linked to its verified capability boundary.
    targets:
    - id: entrypoint-005-javascript-crates-syu-app-ui-assets-js-api-js-runjourn
      adapter: javascript
      path: crates/syu-app-ui/assets/js/api.js
      selector:
        kind: symbol
        name: runJourneyAction
      claims:
      - kind: exposes
        target: FEAT-WORKBENCH-WORK-UI-001#binding.work/target.plan
  - id: public-api-apply-specification-candidate
    role: implementation
    facet: public
    responsibility: Govern the typed specification apply entrypoint.
    targets:
    - id: entrypoint-apply-specification-candidate
      adapter: javascript
      path: crates/syu-app-ui/assets/js/api.js
      selector:
        kind: symbol
        name: applySpecificationCandidate
      claims:
      - kind: exposes
        target: FEAT-WORKBENCH-WORK-UI-001#binding.work/target.plan
  - id: public-api-approve-target-suggestions
    role: implementation
    facet: public
    responsibility: Govern the target suggestion approval entrypoint.
    targets:
    - id: entrypoint-approve-target-suggestions
      adapter: javascript
      path: crates/syu-app-ui/assets/js/api.js
      selector:
        kind: symbol
        name: approveTargetSuggestions
      claims:
      - kind: exposes
        target: FEAT-WORKBENCH-WORK-UI-001#binding.work/target.plan
  - id: public-api-preview-specification-candidate
    role: implementation
    facet: public
    responsibility: Govern typed specification preview.
    targets:
    - id: entrypoint-preview-specification-candidate
      adapter: javascript
      path: crates/syu-app-ui/assets/js/api.js
      selector:
        kind: symbol
        name: previewSpecificationCandidate
      claims:
      - kind: exposes
        target: FEAT-WORKBENCH-WORK-UI-001#binding.work/target.plan
  - id: public-api-read-target-suggestions
    role: implementation
    facet: public
    responsibility: Govern target suggestion reads.
    targets:
    - id: entrypoint-read-target-suggestions
      adapter: javascript
      path: crates/syu-app-ui/assets/js/api.js
      selector:
        kind: symbol
        name: readTargetSuggestions
      claims:
      - kind: exposes
        target: FEAT-WORKBENCH-WORK-UI-001#binding.work/target.plan
  - id: public-api-reject-target-suggestion
    role: implementation
    facet: public
    responsibility: Govern target suggestion rejection.
    targets:
    - id: entrypoint-reject-target-suggestion
      adapter: javascript
      path: crates/syu-app-ui/assets/js/api.js
      selector:
        kind: symbol
        name: rejectTargetSuggestion
      claims:
      - kind: exposes
        target: FEAT-WORKBENCH-WORK-UI-001#binding.work/target.plan
  - id: public-api-run-diagnostics
    role: implementation
    facet: public
    responsibility: Govern canonical diagnostic execution.
    targets:
    - id: entrypoint-run-diagnostics
      adapter: javascript
      path: crates/syu-app-ui/assets/js/api.js
      selector:
        kind: symbol
        name: runDiagnostics
      claims:
      - kind: exposes
        target: FEAT-WORKBENCH-WORK-UI-001#binding.work/target.plan
  - id: public-api-run-readiness
    role: implementation
    facet: public
    responsibility: Govern canonical readiness execution.
    targets:
    - id: entrypoint-run-readiness
      adapter: javascript
      path: crates/syu-app-ui/assets/js/api.js
      selector:
        kind: symbol
        name: runReadiness
      claims:
      - kind: exposes
        target: FEAT-WORKBENCH-WORK-UI-001#binding.work/target.plan
  - id: public-api-search-specification-candidates
    role: implementation
    facet: public
    responsibility: Govern typed specification search.
    targets:
    - id: entrypoint-search-specification-candidates
      adapter: javascript
      path: crates/syu-app-ui/assets/js/api.js
      selector:
        kind: symbol
        name: searchSpecificationCandidates
      claims:
      - kind: exposes
        target: FEAT-WORKBENCH-WORK-UI-001#binding.work/target.plan
```
