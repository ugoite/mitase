---
title: "Public entrypoint contracts / Workbench Pages"
description: "Generated reference for docs/syu/features/public-entrypoints/workbench-pages.yaml"
---

> Generated from `docs/syu/features/public-entrypoints/workbench-pages.yaml`.

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

- **id**: FEAT-PUBLIC-WORKBENCH-PAGES-001
  - **title**: Workbench pages
  - **summary**: Govern page initialization and canonical projection rendering.
  - **status**: implemented
  - **bindings**:
    - **id**: public-api-013
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint addressable by an exact target.
      - **targets**:
        - **id**: entrypoint-013-javascript-crates-syu-app-ui-assets-js-pages-diagnostic
          - **adapter**: javascript
          - **path**: crates/syu-app-ui/assets/js/pages/diagnostics.js
          - **selector**:
            - **kind**: symbol
            - **name**: renderDiagnostics
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.workbench-pages
    - **id**: public-api-014
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint addressable by an exact target.
      - **targets**:
        - **id**: entrypoint-014-javascript-crates-syu-app-ui-assets-js-pages-readiness
          - **adapter**: javascript
          - **path**: crates/syu-app-ui/assets/js/pages/readiness.js
          - **selector**:
            - **kind**: symbol
            - **name**: renderReadinessPage
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.workbench-pages
    - **id**: public-api-015
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint addressable by an exact target.
      - **targets**:
        - **id**: entrypoint-015-javascript-crates-syu-app-ui-assets-js-pages-scope-js-r
          - **adapter**: javascript
          - **path**: crates/syu-app-ui/assets/js/pages/scope.js
          - **selector**:
            - **kind**: symbol
            - **name**: renderScope
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.workbench-pages
    - **id**: public-api-016
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint addressable by an exact target.
      - **targets**:
        - **id**: entrypoint-016-javascript-crates-syu-app-ui-assets-js-pages-settings-j
          - **adapter**: javascript
          - **path**: crates/syu-app-ui/assets/js/pages/settings.js
          - **selector**:
            - **kind**: symbol
            - **name**: renderSettings
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.workbench-pages
    - **id**: public-api-017
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint addressable by an exact target.
      - **targets**:
        - **id**: entrypoint-017-javascript-crates-syu-app-ui-assets-js-pages-specificat
          - **adapter**: javascript
          - **path**: crates/syu-app-ui/assets/js/pages/specifications.js
          - **selector**:
            - **kind**: symbol
            - **name**: renderSpecifications
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.workbench-pages
    - **id**: public-api-init-diagnostics
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Govern diagnostics page initialization.
      - **targets**:
        - **id**: entrypoint-init-diagnostics
          - **adapter**: javascript
          - **path**: crates/syu-app-ui/assets/js/pages/diagnostics.js
          - **selector**:
            - **kind**: symbol
            - **name**: initDiagnostics
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.workbench-pages
    - **id**: public-api-init-readiness
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Govern readiness page initialization.
      - **targets**:
        - **id**: entrypoint-init-readiness
          - **adapter**: javascript
          - **path**: crates/syu-app-ui/assets/js/pages/readiness.js
          - **selector**:
            - **kind**: symbol
            - **name**: initReadiness
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.workbench-pages
    - **id**: public-api-init-scope
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Govern scope page initialization.
      - **targets**:
        - **id**: entrypoint-init-scope
          - **adapter**: javascript
          - **path**: crates/syu-app-ui/assets/js/pages/scope.js
          - **selector**:
            - **kind**: symbol
            - **name**: initScope
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.workbench-pages
    - **id**: public-api-init-specifications
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Govern specifications page initialization.
      - **targets**:
        - **id**: entrypoint-init-specifications
          - **adapter**: javascript
          - **path**: crates/syu-app-ui/assets/js/pages/specifications.js
          - **selector**:
            - **kind**: symbol
            - **name**: initSpecifications
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.workbench-pages
    - **id**: public-api-render-source-detail
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Govern linked source detail rendering.
      - **targets**:
        - **id**: entrypoint-render-source-detail
          - **adapter**: javascript
          - **path**: crates/syu-app-ui/assets/js/pages/specifications.js
          - **selector**:
            - **kind**: symbol
            - **name**: renderSourceDetail
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.workbench-pages
    - **id**: public-api-render-specification-detail
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Govern specification detail rendering.
      - **targets**:
        - **id**: entrypoint-render-specification-detail
          - **adapter**: javascript
          - **path**: crates/syu-app-ui/assets/js/pages/specifications.js
          - **selector**:
            - **kind**: symbol
            - **name**: renderSpecificationDetail
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.workbench-pages
    - **id**: public-readiness
      - **role**: verification
      - **facet**: public
      - **responsibility**: Prove workbench pages entrypoints have bounded canonical plans.
      - **targets**:
        - **id**: canonical-plans
          - **adapter**: rust
          - **path**: crates/syu-validation/src/readiness.rs
          - **selector**:
            - **kind**: symbol
            - **name**: tests::workbench_pages_public_entrypoints_have_canonical_plans
          - **claims**:
            - **kind**: verifies
              - **criterion**: REQ-PUBLIC-001#criterion.workbench-pages
              - **covers**:
                - FEAT-PUBLIC-WORKBENCH-PAGES-001#binding.public-api-013/target.entrypoint-013-javascript-crates-syu-app-ui-assets-js-pages-diagnostic
                - FEAT-PUBLIC-WORKBENCH-PAGES-001#binding.public-api-014/target.entrypoint-014-javascript-crates-syu-app-ui-assets-js-pages-readiness
                - FEAT-PUBLIC-WORKBENCH-PAGES-001#binding.public-api-015/target.entrypoint-015-javascript-crates-syu-app-ui-assets-js-pages-scope-js-r
                - FEAT-PUBLIC-WORKBENCH-PAGES-001#binding.public-api-016/target.entrypoint-016-javascript-crates-syu-app-ui-assets-js-pages-settings-j
                - FEAT-PUBLIC-WORKBENCH-PAGES-001#binding.public-api-017/target.entrypoint-017-javascript-crates-syu-app-ui-assets-js-pages-specificat
                - FEAT-PUBLIC-WORKBENCH-PAGES-001#binding.public-api-init-diagnostics/target.entrypoint-init-diagnostics
                - FEAT-PUBLIC-WORKBENCH-PAGES-001#binding.public-api-init-readiness/target.entrypoint-init-readiness
                - FEAT-PUBLIC-WORKBENCH-PAGES-001#binding.public-api-init-scope/target.entrypoint-init-scope
                - FEAT-PUBLIC-WORKBENCH-PAGES-001#binding.public-api-init-specifications/target.entrypoint-init-specifications
                - FEAT-PUBLIC-WORKBENCH-PAGES-001#binding.public-api-render-source-detail/target.entrypoint-render-source-detail
                - FEAT-PUBLIC-WORKBENCH-PAGES-001#binding.public-api-render-specification-detail/target.entrypoint-render-specification-detail
              - **runner**:
                - **runner**: cargo-test
                - **arguments**:
                  - **package**: syu-validation
                  - **test**: tests::workbench_pages_public_entrypoints_have_canonical_plans

## Source YAML

```yaml
schema: syu/spec/v1
kind: features
namespace: public
category: Public entrypoint contracts
features:
- id: FEAT-PUBLIC-WORKBENCH-PAGES-001
  title: Workbench pages
  summary: Govern page initialization and canonical projection rendering.
  status: implemented
  bindings:
  - id: public-api-013
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint addressable by an exact target.
    targets:
    - id: entrypoint-013-javascript-crates-syu-app-ui-assets-js-pages-diagnostic
      adapter: javascript
      path: crates/syu-app-ui/assets/js/pages/diagnostics.js
      selector:
        kind: symbol
        name: renderDiagnostics
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.workbench-pages
  - id: public-api-014
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint addressable by an exact target.
    targets:
    - id: entrypoint-014-javascript-crates-syu-app-ui-assets-js-pages-readiness
      adapter: javascript
      path: crates/syu-app-ui/assets/js/pages/readiness.js
      selector:
        kind: symbol
        name: renderReadinessPage
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.workbench-pages
  - id: public-api-015
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint addressable by an exact target.
    targets:
    - id: entrypoint-015-javascript-crates-syu-app-ui-assets-js-pages-scope-js-r
      adapter: javascript
      path: crates/syu-app-ui/assets/js/pages/scope.js
      selector:
        kind: symbol
        name: renderScope
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.workbench-pages
  - id: public-api-016
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint addressable by an exact target.
    targets:
    - id: entrypoint-016-javascript-crates-syu-app-ui-assets-js-pages-settings-j
      adapter: javascript
      path: crates/syu-app-ui/assets/js/pages/settings.js
      selector:
        kind: symbol
        name: renderSettings
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.workbench-pages
  - id: public-api-017
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint addressable by an exact target.
    targets:
    - id: entrypoint-017-javascript-crates-syu-app-ui-assets-js-pages-specificat
      adapter: javascript
      path: crates/syu-app-ui/assets/js/pages/specifications.js
      selector:
        kind: symbol
        name: renderSpecifications
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.workbench-pages
  - id: public-api-init-diagnostics
    role: implementation
    facet: public
    responsibility: Govern diagnostics page initialization.
    targets:
    - id: entrypoint-init-diagnostics
      adapter: javascript
      path: crates/syu-app-ui/assets/js/pages/diagnostics.js
      selector:
        kind: symbol
        name: initDiagnostics
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.workbench-pages
  - id: public-api-init-readiness
    role: implementation
    facet: public
    responsibility: Govern readiness page initialization.
    targets:
    - id: entrypoint-init-readiness
      adapter: javascript
      path: crates/syu-app-ui/assets/js/pages/readiness.js
      selector:
        kind: symbol
        name: initReadiness
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.workbench-pages
  - id: public-api-init-scope
    role: implementation
    facet: public
    responsibility: Govern scope page initialization.
    targets:
    - id: entrypoint-init-scope
      adapter: javascript
      path: crates/syu-app-ui/assets/js/pages/scope.js
      selector:
        kind: symbol
        name: initScope
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.workbench-pages
  - id: public-api-init-specifications
    role: implementation
    facet: public
    responsibility: Govern specifications page initialization.
    targets:
    - id: entrypoint-init-specifications
      adapter: javascript
      path: crates/syu-app-ui/assets/js/pages/specifications.js
      selector:
        kind: symbol
        name: initSpecifications
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.workbench-pages
  - id: public-api-render-source-detail
    role: implementation
    facet: public
    responsibility: Govern linked source detail rendering.
    targets:
    - id: entrypoint-render-source-detail
      adapter: javascript
      path: crates/syu-app-ui/assets/js/pages/specifications.js
      selector:
        kind: symbol
        name: renderSourceDetail
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.workbench-pages
  - id: public-api-render-specification-detail
    role: implementation
    facet: public
    responsibility: Govern specification detail rendering.
    targets:
    - id: entrypoint-render-specification-detail
      adapter: javascript
      path: crates/syu-app-ui/assets/js/pages/specifications.js
      selector:
        kind: symbol
        name: renderSpecificationDetail
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.workbench-pages
  - id: public-readiness
    role: verification
    facet: public
    responsibility: Prove workbench pages entrypoints have bounded canonical plans.
    targets:
    - id: canonical-plans
      adapter: rust
      path: crates/syu-validation/src/readiness.rs
      selector:
        kind: symbol
        name: tests::workbench_pages_public_entrypoints_have_canonical_plans
      claims:
      - kind: verifies
        criterion: REQ-PUBLIC-001#criterion.workbench-pages
        covers:
        - FEAT-PUBLIC-WORKBENCH-PAGES-001#binding.public-api-013/target.entrypoint-013-javascript-crates-syu-app-ui-assets-js-pages-diagnostic
        - FEAT-PUBLIC-WORKBENCH-PAGES-001#binding.public-api-014/target.entrypoint-014-javascript-crates-syu-app-ui-assets-js-pages-readiness
        - FEAT-PUBLIC-WORKBENCH-PAGES-001#binding.public-api-015/target.entrypoint-015-javascript-crates-syu-app-ui-assets-js-pages-scope-js-r
        - FEAT-PUBLIC-WORKBENCH-PAGES-001#binding.public-api-016/target.entrypoint-016-javascript-crates-syu-app-ui-assets-js-pages-settings-j
        - FEAT-PUBLIC-WORKBENCH-PAGES-001#binding.public-api-017/target.entrypoint-017-javascript-crates-syu-app-ui-assets-js-pages-specificat
        - FEAT-PUBLIC-WORKBENCH-PAGES-001#binding.public-api-init-diagnostics/target.entrypoint-init-diagnostics
        - FEAT-PUBLIC-WORKBENCH-PAGES-001#binding.public-api-init-readiness/target.entrypoint-init-readiness
        - FEAT-PUBLIC-WORKBENCH-PAGES-001#binding.public-api-init-scope/target.entrypoint-init-scope
        - FEAT-PUBLIC-WORKBENCH-PAGES-001#binding.public-api-init-specifications/target.entrypoint-init-specifications
        - FEAT-PUBLIC-WORKBENCH-PAGES-001#binding.public-api-render-source-detail/target.entrypoint-render-source-detail
        - FEAT-PUBLIC-WORKBENCH-PAGES-001#binding.public-api-render-specification-detail/target.entrypoint-render-specification-detail
        runner:
          runner: cargo-test
          arguments:
            package: syu-validation
            test: tests::workbench_pages_public_entrypoints_have_canonical_plans
```
