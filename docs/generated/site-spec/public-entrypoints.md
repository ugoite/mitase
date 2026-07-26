---
title: "Public entrypoint contracts / Public Entrypoints"
description: "Generated reference for docs/syu/public-entrypoints.yaml"
---

> Generated from `docs/syu/public-entrypoints.yaml`.

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

- **id**: FEAT-PUBLIC-WORKBENCH-TRANSPORT-001
  - **title**: Workbench client transport
  - **summary**: Govern canonical browser transport, session, projection, scope, and source reads.
  - **status**: implemented
  - **bindings**:
    - **id**: public-api-001
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint addressable by an exact target.
      - **targets**:
        - **id**: entrypoint-001-javascript-crates-syu-app-ui-assets-js-api-js-mutationb
          - **adapter**: javascript
          - **path**: crates/syu-app-ui/assets/js/api.js
          - **selector**:
            - **kind**: symbol
            - **name**: mutationBasis
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.workbench-client-transport
    - **id**: public-api-002
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint addressable by an exact target.
      - **targets**:
        - **id**: entrypoint-002-javascript-crates-syu-app-ui-assets-js-api-js-post
          - **adapter**: javascript
          - **path**: crates/syu-app-ui/assets/js/api.js
          - **selector**:
            - **kind**: symbol
            - **name**: post
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.workbench-client-transport
    - **id**: public-api-003
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint addressable by an exact target.
      - **targets**:
        - **id**: entrypoint-003-javascript-crates-syu-app-ui-assets-js-api-js-readproje
          - **adapter**: javascript
          - **path**: crates/syu-app-ui/assets/js/api.js
          - **selector**:
            - **kind**: symbol
            - **name**: readProjection
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.workbench-client-transport
    - **id**: public-api-004
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint addressable by an exact target.
      - **targets**:
        - **id**: entrypoint-004-javascript-crates-syu-app-ui-assets-js-api-js-request
          - **adapter**: javascript
          - **path**: crates/syu-app-ui/assets/js/api.js
          - **selector**:
            - **kind**: symbol
            - **name**: request
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.workbench-client-transport
    - **id**: public-api-establish-session
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Govern Workbench session establishment.
      - **targets**:
        - **id**: entrypoint-establish-session
          - **adapter**: javascript
          - **path**: crates/syu-app-ui/assets/js/api.js
          - **selector**:
            - **kind**: symbol
            - **name**: establishSession
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.workbench-client-transport
    - **id**: public-api-read-branch-scope
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Govern branch-scope reads.
      - **targets**:
        - **id**: entrypoint-read-branch-scope
          - **adapter**: javascript
          - **path**: crates/syu-app-ui/assets/js/api.js
          - **selector**:
            - **kind**: symbol
            - **name**: readBranchScope
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.workbench-client-transport
    - **id**: public-api-read-scope-diff
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Govern scope diff reads.
      - **targets**:
        - **id**: entrypoint-read-scope-diff
          - **adapter**: javascript
          - **path**: crates/syu-app-ui/assets/js/api.js
          - **selector**:
            - **kind**: symbol
            - **name**: readScopeDiff
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.workbench-client-transport
    - **id**: public-api-read-source
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Govern linked source reads.
      - **targets**:
        - **id**: entrypoint-read-source
          - **adapter**: javascript
          - **path**: crates/syu-app-ui/assets/js/api.js
          - **selector**:
            - **kind**: symbol
            - **name**: readSource
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.workbench-client-transport
    - **id**: public-api-read-target-source
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Govern exact target source reads.
      - **targets**:
        - **id**: entrypoint-read-target-source
          - **adapter**: javascript
          - **path**: crates/syu-app-ui/assets/js/api.js
          - **selector**:
            - **kind**: symbol
            - **name**: readTargetSource
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.workbench-client-transport
    - **id**: public-readiness
      - **role**: verification
      - **facet**: public
      - **responsibility**: Prove workbench client transport entrypoints have bounded canonical plans.
      - **targets**:
        - **id**: canonical-plans
          - **adapter**: rust
          - **path**: crates/syu-validation/src/readiness.rs
          - **selector**:
            - **kind**: symbol
            - **name**: tests::workbench_client_transport_public_entrypoints_have_canonical_plans
          - **claims**:
            - **kind**: verifies
              - **criterion**: REQ-PUBLIC-001#criterion.workbench-client-transport
              - **covers**:
                - FEAT-PUBLIC-WORKBENCH-TRANSPORT-001#binding.public-api-001/target.entrypoint-001-javascript-crates-syu-app-ui-assets-js-api-js-mutationb
                - FEAT-PUBLIC-WORKBENCH-TRANSPORT-001#binding.public-api-002/target.entrypoint-002-javascript-crates-syu-app-ui-assets-js-api-js-post
                - FEAT-PUBLIC-WORKBENCH-TRANSPORT-001#binding.public-api-003/target.entrypoint-003-javascript-crates-syu-app-ui-assets-js-api-js-readproje
                - FEAT-PUBLIC-WORKBENCH-TRANSPORT-001#binding.public-api-004/target.entrypoint-004-javascript-crates-syu-app-ui-assets-js-api-js-request
                - FEAT-PUBLIC-WORKBENCH-TRANSPORT-001#binding.public-api-establish-session/target.entrypoint-establish-session
                - FEAT-PUBLIC-WORKBENCH-TRANSPORT-001#binding.public-api-read-branch-scope/target.entrypoint-read-branch-scope
                - FEAT-PUBLIC-WORKBENCH-TRANSPORT-001#binding.public-api-read-scope-diff/target.entrypoint-read-scope-diff
                - FEAT-PUBLIC-WORKBENCH-TRANSPORT-001#binding.public-api-read-source/target.entrypoint-read-source
                - FEAT-PUBLIC-WORKBENCH-TRANSPORT-001#binding.public-api-read-target-source/target.entrypoint-read-target-source
              - **runner**:
                - **runner**: cargo-test
                - **arguments**:
                  - **package**: syu-validation
                  - **test**: tests::workbench_client_transport_public_entrypoints_have_canonical_plans
- **id**: FEAT-PUBLIC-WORKBENCH-ACTIONS-001
  - **title**: Workbench client actions
  - **summary**: Govern typed Workbench preview, approval, diagnostics, readiness, and search actions.
  - **status**: implemented
  - **bindings**:
    - **id**: public-api-005
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint addressable by an exact target.
      - **targets**:
        - **id**: entrypoint-005-javascript-crates-syu-app-ui-assets-js-api-js-runjourn
          - **adapter**: javascript
          - **path**: crates/syu-app-ui/assets/js/api.js
          - **selector**:
            - **kind**: symbol
            - **name**: runJourneyAction
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.workbench-client-actions
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
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.workbench-client-actions
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
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.workbench-client-actions
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
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.workbench-client-actions
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
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.workbench-client-actions
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
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.workbench-client-actions
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
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.workbench-client-actions
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
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.workbench-client-actions
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
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.workbench-client-actions
    - **id**: public-readiness
      - **role**: verification
      - **facet**: public
      - **responsibility**: Prove workbench client actions entrypoints have bounded canonical plans.
      - **targets**:
        - **id**: canonical-plans
          - **adapter**: rust
          - **path**: crates/syu-validation/src/readiness.rs
          - **selector**:
            - **kind**: symbol
            - **name**: tests::workbench_client_actions_public_entrypoints_have_canonical_plans
          - **claims**:
            - **kind**: verifies
              - **criterion**: REQ-PUBLIC-001#criterion.workbench-client-actions
              - **covers**:
                - FEAT-PUBLIC-WORKBENCH-ACTIONS-001#binding.public-api-005/target.entrypoint-005-javascript-crates-syu-app-ui-assets-js-api-js-runjourn
                - FEAT-PUBLIC-WORKBENCH-ACTIONS-001#binding.public-api-apply-specification-candidate/target.entrypoint-apply-specification-candidate
                - FEAT-PUBLIC-WORKBENCH-ACTIONS-001#binding.public-api-approve-target-suggestions/target.entrypoint-approve-target-suggestions
                - FEAT-PUBLIC-WORKBENCH-ACTIONS-001#binding.public-api-preview-specification-candidate/target.entrypoint-preview-specification-candidate
                - FEAT-PUBLIC-WORKBENCH-ACTIONS-001#binding.public-api-read-target-suggestions/target.entrypoint-read-target-suggestions
                - FEAT-PUBLIC-WORKBENCH-ACTIONS-001#binding.public-api-reject-target-suggestion/target.entrypoint-reject-target-suggestion
                - FEAT-PUBLIC-WORKBENCH-ACTIONS-001#binding.public-api-run-diagnostics/target.entrypoint-run-diagnostics
                - FEAT-PUBLIC-WORKBENCH-ACTIONS-001#binding.public-api-run-readiness/target.entrypoint-run-readiness
                - FEAT-PUBLIC-WORKBENCH-ACTIONS-001#binding.public-api-search-specification-candidates/target.entrypoint-search-specification-candidates
              - **runner**:
                - **runner**: cargo-test
                - **arguments**:
                  - **package**: syu-validation
                  - **test**: tests::workbench_client_actions_public_entrypoints_have_canonical_plans
- **id**: FEAT-PUBLIC-WORKBENCH-COMPONENTS-001
  - **title**: Workbench components
  - **summary**: Govern reusable Workbench rendering, translation, and diff components.
  - **status**: implemented
  - **bindings**:
    - **id**: public-api-006
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint addressable by an exact target.
      - **targets**:
        - **id**: entrypoint-006-javascript-crates-syu-app-ui-assets-js-components-actio
          - **adapter**: javascript
          - **path**: crates/syu-app-ui/assets/js/components/action.js
          - **selector**:
            - **kind**: symbol
            - **name**: actionCapability
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.workbench-components
    - **id**: public-api-007
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint addressable by an exact target.
      - **targets**:
        - **id**: entrypoint-007-javascript-crates-syu-app-ui-assets-js-components-diagn
          - **adapter**: javascript
          - **path**: crates/syu-app-ui/assets/js/components/diagnostic.js
          - **selector**:
            - **kind**: symbol
            - **name**: renderDiagnostic
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.workbench-components
    - **id**: public-api-008
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint addressable by an exact target.
      - **targets**:
        - **id**: entrypoint-008-javascript-crates-syu-app-ui-assets-js-components-edito
          - **adapter**: javascript
          - **path**: crates/syu-app-ui/assets/js/components/editor.js
          - **selector**:
            - **kind**: symbol
            - **name**: structuredEditor
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.workbench-components
    - **id**: public-api-009
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint addressable by an exact target.
      - **targets**:
        - **id**: entrypoint-009-javascript-crates-syu-app-ui-assets-js-components-readi
          - **adapter**: javascript
          - **path**: crates/syu-app-ui/assets/js/components/readiness.js
          - **selector**:
            - **kind**: symbol
            - **name**: renderReadiness
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.workbench-components
    - **id**: public-api-010
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint addressable by an exact target.
      - **targets**:
        - **id**: entrypoint-010-javascript-crates-syu-app-ui-assets-js-components-targe
          - **adapter**: javascript
          - **path**: crates/syu-app-ui/assets/js/components/target.js
          - **selector**:
            - **kind**: symbol
            - **name**: renderTarget
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.workbench-components
    - **id**: public-api-011
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint addressable by an exact target.
      - **targets**:
        - **id**: entrypoint-011-javascript-crates-syu-app-ui-assets-js-i18n-js-translat
          - **adapter**: javascript
          - **path**: crates/syu-app-ui/assets/js/i18n.js
          - **selector**:
            - **kind**: symbol
            - **name**: translate
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.workbench-components
    - **id**: public-api-render-diff
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Govern bounded diff rendering.
      - **targets**:
        - **id**: entrypoint-render-diff
          - **adapter**: javascript
          - **path**: crates/syu-app-ui/assets/js/components/diff.js
          - **selector**:
            - **kind**: symbol
            - **name**: renderDiff
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.workbench-components
    - **id**: public-readiness
      - **role**: verification
      - **facet**: public
      - **responsibility**: Prove workbench components entrypoints have bounded canonical plans.
      - **targets**:
        - **id**: canonical-plans
          - **adapter**: rust
          - **path**: crates/syu-validation/src/readiness.rs
          - **selector**:
            - **kind**: symbol
            - **name**: tests::workbench_components_public_entrypoints_have_canonical_plans
          - **claims**:
            - **kind**: verifies
              - **criterion**: REQ-PUBLIC-001#criterion.workbench-components
              - **covers**:
                - FEAT-PUBLIC-WORKBENCH-COMPONENTS-001#binding.public-api-006/target.entrypoint-006-javascript-crates-syu-app-ui-assets-js-components-actio
                - FEAT-PUBLIC-WORKBENCH-COMPONENTS-001#binding.public-api-007/target.entrypoint-007-javascript-crates-syu-app-ui-assets-js-components-diagn
                - FEAT-PUBLIC-WORKBENCH-COMPONENTS-001#binding.public-api-008/target.entrypoint-008-javascript-crates-syu-app-ui-assets-js-components-edito
                - FEAT-PUBLIC-WORKBENCH-COMPONENTS-001#binding.public-api-009/target.entrypoint-009-javascript-crates-syu-app-ui-assets-js-components-readi
                - FEAT-PUBLIC-WORKBENCH-COMPONENTS-001#binding.public-api-010/target.entrypoint-010-javascript-crates-syu-app-ui-assets-js-components-targe
                - FEAT-PUBLIC-WORKBENCH-COMPONENTS-001#binding.public-api-011/target.entrypoint-011-javascript-crates-syu-app-ui-assets-js-i18n-js-translat
                - FEAT-PUBLIC-WORKBENCH-COMPONENTS-001#binding.public-api-render-diff/target.entrypoint-render-diff
              - **runner**:
                - **runner**: cargo-test
                - **arguments**:
                  - **package**: syu-validation
                  - **test**: tests::workbench_components_public_entrypoints_have_canonical_plans
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
- **id**: FEAT-PUBLIC-WORKBENCH-NAVIGATION-001
  - **title**: Workbench navigation
  - **summary**: Govern Workbench routes, tabs, selected slices, and landing navigation.
  - **status**: implemented
  - **bindings**:
    - **id**: public-api-019
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint addressable by an exact target.
      - **targets**:
        - **id**: entrypoint-019-javascript-crates-syu-app-ui-assets-js-router-js-pages
          - **adapter**: javascript
          - **path**: crates/syu-app-ui/assets/js/router.js
          - **selector**:
            - **kind**: symbol
            - **name**: PAGES
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.workbench-navigation
    - **id**: public-api-020
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint addressable by an exact target.
      - **targets**:
        - **id**: entrypoint-020-javascript-crates-syu-app-ui-assets-js-router-js-tab-gr
          - **adapter**: javascript
          - **path**: crates/syu-app-ui/assets/js/router.js
          - **selector**:
            - **kind**: symbol
            - **name**: TAB_GROUPS
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.workbench-navigation
    - **id**: public-api-021
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint addressable by an exact target.
      - **targets**:
        - **id**: entrypoint-021-javascript-crates-syu-app-ui-assets-js-router-js-naviga
          - **adapter**: javascript
          - **path**: crates/syu-app-ui/assets/js/router.js
          - **selector**:
            - **kind**: symbol
            - **name**: navigate
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.workbench-navigation
    - **id**: public-api-022
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint addressable by an exact target.
      - **targets**:
        - **id**: entrypoint-022-javascript-crates-syu-app-ui-assets-js-state-js-creates
          - **adapter**: javascript
          - **path**: crates/syu-app-ui/assets/js/state.js
          - **selector**:
            - **kind**: symbol
            - **name**: createState
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.workbench-navigation
    - **id**: public-api-023
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint addressable by an exact target.
      - **targets**:
        - **id**: entrypoint-023-javascript-crates-syu-app-ui-assets-js-state-js-replace
          - **adapter**: javascript
          - **path**: crates/syu-app-ui/assets/js/state.js
          - **selector**:
            - **kind**: symbol
            - **name**: replaceProjection
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.workbench-navigation
    - **id**: public-api-024
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint addressable by an exact target.
      - **targets**:
        - **id**: entrypoint-024-javascript-crates-syu-app-ui-assets-js-state-js-selects
          - **adapter**: javascript
          - **path**: crates/syu-app-ui/assets/js/state.js
          - **selector**:
            - **kind**: symbol
            - **name**: selectSlice
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.workbench-navigation
    - **id**: public-api-025
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint addressable by an exact target.
      - **targets**:
        - **id**: entrypoint-025-javascript-website-src-pages-index-js-home
          - **adapter**: javascript
          - **path**: website/src/pages/index.js
          - **selector**:
            - **kind**: symbol
            - **name**: Home
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.workbench-navigation
    - **id**: public-readiness
      - **role**: verification
      - **facet**: public
      - **responsibility**: Prove workbench navigation entrypoints have bounded canonical plans.
      - **targets**:
        - **id**: canonical-plans
          - **adapter**: rust
          - **path**: crates/syu-validation/src/readiness.rs
          - **selector**:
            - **kind**: symbol
            - **name**: tests::workbench_navigation_public_entrypoints_have_canonical_plans
          - **claims**:
            - **kind**: verifies
              - **criterion**: REQ-PUBLIC-001#criterion.workbench-navigation
              - **covers**:
                - FEAT-PUBLIC-WORKBENCH-NAVIGATION-001#binding.public-api-019/target.entrypoint-019-javascript-crates-syu-app-ui-assets-js-router-js-pages
                - FEAT-PUBLIC-WORKBENCH-NAVIGATION-001#binding.public-api-020/target.entrypoint-020-javascript-crates-syu-app-ui-assets-js-router-js-tab-gr
                - FEAT-PUBLIC-WORKBENCH-NAVIGATION-001#binding.public-api-021/target.entrypoint-021-javascript-crates-syu-app-ui-assets-js-router-js-naviga
                - FEAT-PUBLIC-WORKBENCH-NAVIGATION-001#binding.public-api-022/target.entrypoint-022-javascript-crates-syu-app-ui-assets-js-state-js-creates
                - FEAT-PUBLIC-WORKBENCH-NAVIGATION-001#binding.public-api-023/target.entrypoint-023-javascript-crates-syu-app-ui-assets-js-state-js-replace
                - FEAT-PUBLIC-WORKBENCH-NAVIGATION-001#binding.public-api-024/target.entrypoint-024-javascript-crates-syu-app-ui-assets-js-state-js-selects
                - FEAT-PUBLIC-WORKBENCH-NAVIGATION-001#binding.public-api-025/target.entrypoint-025-javascript-website-src-pages-index-js-home
              - **runner**:
                - **runner**: cargo-test
                - **arguments**:
                  - **package**: syu-validation
                  - **test**: tests::workbench_navigation_public_entrypoints_have_canonical_plans
- **id**: FEAT-PUBLIC-WORKBENCH-RENDERING-001
  - **title**: Workbench shell rendering
  - **summary**: Govern localized server-rendered Workbench shell construction.
  - **status**: implemented
  - **bindings**:
    - **id**: public-api-026
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint addressable by an exact target.
      - **targets**:
        - **id**: entrypoint-026-rust-crates-syu-app-ui-src-lib-rs-workbenchview-new
          - **adapter**: rust
          - **path**: crates/syu-app-ui/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: WorkbenchView::new
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.workbench-rendering
    - **id**: public-api-027
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint addressable by an exact target.
      - **targets**:
        - **id**: entrypoint-027-rust-crates-syu-app-ui-src-lib-rs-workbenchview-render
          - **adapter**: rust
          - **path**: crates/syu-app-ui/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: WorkbenchView::render_html
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.workbench-rendering
    - **id**: public-api-028
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint addressable by an exact target.
      - **targets**:
        - **id**: entrypoint-028-rust-crates-syu-app-ui-src-lib-rs-locale-catalog-script
          - **adapter**: rust
          - **path**: crates/syu-app-ui/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: locale_catalog_script
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.workbench-rendering
    - **id**: public-readiness
      - **role**: verification
      - **facet**: public
      - **responsibility**: Prove workbench shell rendering entrypoints have bounded canonical plans.
      - **targets**:
        - **id**: canonical-plans
          - **adapter**: rust
          - **path**: crates/syu-validation/src/readiness.rs
          - **selector**:
            - **kind**: symbol
            - **name**: tests::workbench_rendering_public_entrypoints_have_canonical_plans
          - **claims**:
            - **kind**: verifies
              - **criterion**: REQ-PUBLIC-001#criterion.workbench-rendering
              - **covers**:
                - FEAT-PUBLIC-WORKBENCH-RENDERING-001#binding.public-api-026/target.entrypoint-026-rust-crates-syu-app-ui-src-lib-rs-workbenchview-new
                - FEAT-PUBLIC-WORKBENCH-RENDERING-001#binding.public-api-027/target.entrypoint-027-rust-crates-syu-app-ui-src-lib-rs-workbenchview-render
                - FEAT-PUBLIC-WORKBENCH-RENDERING-001#binding.public-api-028/target.entrypoint-028-rust-crates-syu-app-ui-src-lib-rs-locale-catalog-script
              - **runner**:
                - **runner**: cargo-test
                - **arguments**:
                  - **package**: syu-validation
                  - **test**: tests::workbench_rendering_public_entrypoints_have_canonical_plans
- **id**: FEAT-PUBLIC-CODE-DIAGNOSTICS-001
  - **title**: Code diagnostics
  - **summary**: Govern symbol resolution and diagnostic result entrypoints.
  - **status**: implemented
  - **bindings**:
    - **id**: public-api-029
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint addressable by an exact target.
      - **targets**:
        - **id**: entrypoint-029-rust-crates-syu-code-intel-src-lib-rs-resolve-symbol
          - **adapter**: rust
          - **path**: crates/syu-code-intel/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: resolve_symbol
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.code-diagnostics
    - **id**: public-api-030
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint addressable by an exact target.
      - **targets**:
        - **id**: entrypoint-030-rust-crates-syu-diagnostics-src-lib-rs-diagnostic-error
          - **adapter**: rust
          - **path**: crates/syu-diagnostics/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: Diagnostic::error
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.code-diagnostics
    - **id**: public-api-031
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint addressable by an exact target.
      - **targets**:
        - **id**: entrypoint-031-rust-crates-syu-diagnostics-src-lib-rs-validationresult
          - **adapter**: rust
          - **path**: crates/syu-diagnostics/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: ValidationResult::is_valid
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.code-diagnostics
    - **id**: public-readiness
      - **role**: verification
      - **facet**: public
      - **responsibility**: Prove code diagnostics entrypoints have bounded canonical plans.
      - **targets**:
        - **id**: canonical-plans
          - **adapter**: rust
          - **path**: crates/syu-validation/src/readiness.rs
          - **selector**:
            - **kind**: symbol
            - **name**: tests::code_diagnostics_public_entrypoints_have_canonical_plans
          - **claims**:
            - **kind**: verifies
              - **criterion**: REQ-PUBLIC-001#criterion.code-diagnostics
              - **covers**:
                - FEAT-PUBLIC-CODE-DIAGNOSTICS-001#binding.public-api-029/target.entrypoint-029-rust-crates-syu-code-intel-src-lib-rs-resolve-symbol
                - FEAT-PUBLIC-CODE-DIAGNOSTICS-001#binding.public-api-030/target.entrypoint-030-rust-crates-syu-diagnostics-src-lib-rs-diagnostic-error
                - FEAT-PUBLIC-CODE-DIAGNOSTICS-001#binding.public-api-031/target.entrypoint-031-rust-crates-syu-diagnostics-src-lib-rs-validationresult
              - **runner**:
                - **runner**: cargo-test
                - **arguments**:
                  - **package**: syu-validation
                  - **test**: tests::code_diagnostics_public_entrypoints_have_canonical_plans
- **id**: FEAT-PUBLIC-INVENTORY-001
  - **title**: Inventory discovery
  - **summary**: Govern artifact discovery and adapter-output combination entrypoints.
  - **status**: implemented
  - **bindings**:
    - **id**: public-api-032
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint addressable by an exact target.
      - **targets**:
        - **id**: entrypoint-032-rust-crates-syu-inventory-src-lib-rs-inventoryregistry
          - **adapter**: rust
          - **path**: crates/syu-inventory/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: InventoryRegistry::discover
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.inventory-discovery
    - **id**: public-api-033
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint addressable by an exact target.
      - **targets**:
        - **id**: entrypoint-033-rust-crates-syu-inventory-src-lib-rs-read-bytes
          - **adapter**: rust
          - **path**: crates/syu-inventory/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: read_bytes
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.inventory-discovery
    - **id**: public-api-034
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint addressable by an exact target.
      - **targets**:
        - **id**: entrypoint-034-rust-crates-syu-inventory-src-lib-rs-union
          - **adapter**: rust
          - **path**: crates/syu-inventory/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: union
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.inventory-discovery
    - **id**: public-readiness
      - **role**: verification
      - **facet**: public
      - **responsibility**: Prove inventory discovery entrypoints have bounded canonical plans.
      - **targets**:
        - **id**: canonical-plans
          - **adapter**: rust
          - **path**: crates/syu-validation/src/readiness.rs
          - **selector**:
            - **kind**: symbol
            - **name**: tests::inventory_discovery_public_entrypoints_have_canonical_plans
          - **claims**:
            - **kind**: verifies
              - **criterion**: REQ-PUBLIC-001#criterion.inventory-discovery
              - **covers**:
                - FEAT-PUBLIC-INVENTORY-001#binding.public-api-032/target.entrypoint-032-rust-crates-syu-inventory-src-lib-rs-inventoryregistry
                - FEAT-PUBLIC-INVENTORY-001#binding.public-api-033/target.entrypoint-033-rust-crates-syu-inventory-src-lib-rs-read-bytes
                - FEAT-PUBLIC-INVENTORY-001#binding.public-api-034/target.entrypoint-034-rust-crates-syu-inventory-src-lib-rs-union
              - **runner**:
                - **runner**: cargo-test
                - **arguments**:
                  - **package**: syu-validation
                  - **test**: tests::inventory_discovery_public_entrypoints_have_canonical_plans
- **id**: FEAT-PUBLIC-SPEC-MODEL-001
  - **title**: Specification model
  - **summary**: Govern repository-path, anchor, and schema entrypoints.
  - **status**: implemented
  - **bindings**:
    - **id**: public-api-035
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint addressable by an exact target.
      - **targets**:
        - **id**: entrypoint-036-rust-crates-syu-spec-model-src-lib-rs-localanchorkind-l
          - **adapter**: rust
          - **path**: crates/syu-spec-model/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: LocalAnchorKind::label
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.specification-model
    - **id**: public-api-036
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint addressable by an exact target.
      - **targets**:
        - **id**: entrypoint-037-rust-crates-syu-spec-model-src-lib-rs-repopath-as-path
          - **adapter**: rust
          - **path**: crates/syu-spec-model/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: RepoPath::as_path
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.specification-model
    - **id**: public-api-037
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint addressable by an exact target.
      - **targets**:
        - **id**: entrypoint-038-rust-crates-syu-spec-model-src-lib-rs-repopath-display
          - **adapter**: rust
          - **path**: crates/syu-spec-model/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: RepoPath::display
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.specification-model
    - **id**: public-api-038
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint addressable by an exact target.
      - **targets**:
        - **id**: entrypoint-039-rust-crates-syu-spec-model-src-lib-rs-repopath-new
          - **adapter**: rust
          - **path**: crates/syu-spec-model/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: RepoPath::new
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.specification-model
    - **id**: public-api-039
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint addressable by an exact target.
      - **targets**:
        - **id**: entrypoint-040-rust-crates-syu-spec-model-src-lib-rs-repopath-to-strin
          - **adapter**: rust
          - **path**: crates/syu-spec-model/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: RepoPath::to_string_lossy
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.specification-model
    - **id**: public-api-040
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint addressable by an exact target.
      - **targets**:
        - **id**: entrypoint-041-rust-crates-syu-spec-model-src-lib-rs-specdocument-sche
          - **adapter**: rust
          - **path**: crates/syu-spec-model/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: SpecDocument::schema
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.specification-model
    - **id**: public-api-repo-path-from-path
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Govern repository path conversion.
      - **targets**:
        - **id**: entrypoint-repo-path-from-path
          - **adapter**: rust
          - **path**: crates/syu-spec-model/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: RepoPath::from_path
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.specification-model
    - **id**: public-readiness
      - **role**: verification
      - **facet**: public
      - **responsibility**: Prove specification model entrypoints have bounded canonical plans.
      - **targets**:
        - **id**: canonical-plans
          - **adapter**: rust
          - **path**: crates/syu-validation/src/readiness.rs
          - **selector**:
            - **kind**: symbol
            - **name**: tests::specification_model_public_entrypoints_have_canonical_plans
          - **claims**:
            - **kind**: verifies
              - **criterion**: REQ-PUBLIC-001#criterion.specification-model
              - **covers**:
                - FEAT-PUBLIC-SPEC-MODEL-001#binding.public-api-035/target.entrypoint-036-rust-crates-syu-spec-model-src-lib-rs-localanchorkind-l
                - FEAT-PUBLIC-SPEC-MODEL-001#binding.public-api-036/target.entrypoint-037-rust-crates-syu-spec-model-src-lib-rs-repopath-as-path
                - FEAT-PUBLIC-SPEC-MODEL-001#binding.public-api-037/target.entrypoint-038-rust-crates-syu-spec-model-src-lib-rs-repopath-display
                - FEAT-PUBLIC-SPEC-MODEL-001#binding.public-api-038/target.entrypoint-039-rust-crates-syu-spec-model-src-lib-rs-repopath-new
                - FEAT-PUBLIC-SPEC-MODEL-001#binding.public-api-039/target.entrypoint-040-rust-crates-syu-spec-model-src-lib-rs-repopath-to-strin
                - FEAT-PUBLIC-SPEC-MODEL-001#binding.public-api-040/target.entrypoint-041-rust-crates-syu-spec-model-src-lib-rs-specdocument-sche
                - FEAT-PUBLIC-SPEC-MODEL-001#binding.public-api-repo-path-from-path/target.entrypoint-repo-path-from-path
              - **runner**:
                - **runner**: cargo-test
                - **arguments**:
                  - **package**: syu-validation
                  - **test**: tests::specification_model_public_entrypoints_have_canonical_plans
- **id**: FEAT-PUBLIC-VALIDATION-001
  - **title**: Validation engine
  - **summary**: Govern canonical validation, readiness, change, and verification entrypoints.
  - **status**: implemented
  - **bindings**:
    - **id**: public-api-041
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint addressable by an exact target.
      - **targets**:
        - **id**: entrypoint-044-rust-crates-syu-validation-src-lib-rs-phase-for-rule
          - **adapter**: rust
          - **path**: crates/syu-validation/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: phase_for_rule
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.validation-engine
    - **id**: public-api-042
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint addressable by an exact target.
      - **targets**:
        - **id**: entrypoint-047-rust-crates-syu-validation-src-lib-rs-validate-without
          - **adapter**: rust
          - **path**: crates/syu-validation/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: validate_without_readiness
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.validation-engine
    - **id**: public-api-078
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint addressable by an exact target.
      - **targets**:
        - **id**: entrypoint-078-rust-crates-syu-validation-src-lib-rs-readinessaxis
          - **adapter**: rust
          - **path**: crates/syu-validation/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: ReadinessAxis
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.validation-engine
    - **id**: public-api-079
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint addressable by an exact target.
      - **targets**:
        - **id**: entrypoint-079-rust-crates-syu-validation-src-lib-rs-readinessaxisid
          - **adapter**: rust
          - **path**: crates/syu-validation/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: ReadinessAxisId
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.validation-engine
    - **id**: public-api-080
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint addressable by an exact target.
      - **targets**:
        - **id**: entrypoint-080-rust-crates-syu-validation-src-lib-rs-readinessreport
          - **adapter**: rust
          - **path**: crates/syu-validation/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: ReadinessReport
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.validation-engine
    - **id**: public-api-081
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint addressable by an exact target.
      - **targets**:
        - **id**: entrypoint-081-rust-crates-syu-validation-src-lib-rs-evaluate-readiness
          - **adapter**: rust
          - **path**: crates/syu-validation/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: evaluate_readiness
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.validation-engine
    - **id**: public-api-082
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint addressable by an exact target.
      - **targets**:
        - **id**: entrypoint-082-rust-crates-syu-validation-src-lib-rs-required-axes
          - **adapter**: rust
          - **path**: crates/syu-validation/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: required_axes
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.validation-engine
    - **id**: public-api-changed-files-against-revision
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Govern canonical changed-file collection.
      - **targets**:
        - **id**: entrypoint-changed-files-against-revision
          - **adapter**: rust
          - **path**: crates/syu-validation/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: changed_files_against_revision
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.validation-engine
    - **id**: public-api-execute-verification-attempt
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Govern durable exact verification attempts.
      - **targets**:
        - **id**: entrypoint-execute-verification-attempt
          - **adapter**: rust
          - **path**: crates/syu-validation/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: execute_verification_attempt
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.validation-engine
    - **id**: public-api-validate-workspace
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Govern canonical workspace validation.
      - **targets**:
        - **id**: entrypoint-validate-workspace
          - **adapter**: rust
          - **path**: crates/syu-validation/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: validate_workspace
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.validation-engine
    - **id**: public-readiness
      - **role**: verification
      - **facet**: public
      - **responsibility**: Prove validation engine entrypoints have bounded canonical plans.
      - **targets**:
        - **id**: canonical-plans
          - **adapter**: rust
          - **path**: crates/syu-validation/src/readiness.rs
          - **selector**:
            - **kind**: symbol
            - **name**: tests::validation_engine_public_entrypoints_have_canonical_plans
          - **claims**:
            - **kind**: verifies
              - **criterion**: REQ-PUBLIC-001#criterion.validation-engine
              - **covers**:
                - FEAT-PUBLIC-VALIDATION-001#binding.public-api-041/target.entrypoint-044-rust-crates-syu-validation-src-lib-rs-phase-for-rule
                - FEAT-PUBLIC-VALIDATION-001#binding.public-api-042/target.entrypoint-047-rust-crates-syu-validation-src-lib-rs-validate-without
                - FEAT-PUBLIC-VALIDATION-001#binding.public-api-078/target.entrypoint-078-rust-crates-syu-validation-src-lib-rs-readinessaxis
                - FEAT-PUBLIC-VALIDATION-001#binding.public-api-079/target.entrypoint-079-rust-crates-syu-validation-src-lib-rs-readinessaxisid
                - FEAT-PUBLIC-VALIDATION-001#binding.public-api-080/target.entrypoint-080-rust-crates-syu-validation-src-lib-rs-readinessreport
                - FEAT-PUBLIC-VALIDATION-001#binding.public-api-081/target.entrypoint-081-rust-crates-syu-validation-src-lib-rs-evaluate-readiness
                - FEAT-PUBLIC-VALIDATION-001#binding.public-api-082/target.entrypoint-082-rust-crates-syu-validation-src-lib-rs-required-axes
                - FEAT-PUBLIC-VALIDATION-001#binding.public-api-changed-files-against-revision/target.entrypoint-changed-files-against-revision
                - FEAT-PUBLIC-VALIDATION-001#binding.public-api-execute-verification-attempt/target.entrypoint-execute-verification-attempt
                - FEAT-PUBLIC-VALIDATION-001#binding.public-api-validate-workspace/target.entrypoint-validate-workspace
              - **runner**:
                - **runner**: cargo-test
                - **arguments**:
                  - **package**: syu-validation
                  - **test**: tests::validation_engine_public_entrypoints_have_canonical_plans
- **id**: FEAT-PUBLIC-WORK-PLANNING-001
  - **title**: Work planning
  - **summary**: Govern requested-target, plan-identity, and split-guidance entrypoints.
  - **status**: implemented
  - **bindings**:
    - **id**: public-api-043
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint addressable by an exact target.
      - **targets**:
        - **id**: entrypoint-048-rust-crates-syu-work-model-src-lib-rs-requestedtarget-c
          - **adapter**: rust
          - **path**: crates/syu-work-model/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: RequestedTarget::criterion
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.work-planning
    - **id**: public-api-044
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint addressable by an exact target.
      - **targets**:
        - **id**: entrypoint-049-rust-crates-syu-work-model-src-lib-rs-requestedtarget-r
          - **adapter**: rust
          - **path**: crates/syu-work-model/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: RequestedTarget::reference
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.work-planning
    - **id**: public-api-045
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint addressable by an exact target.
      - **targets**:
        - **id**: entrypoint-050-rust-crates-syu-work-model-src-lib-rs-requestedtarget-t
          - **adapter**: rust
          - **path**: crates/syu-work-model/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: RequestedTarget::transition
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.work-planning
    - **id**: public-api-046
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint addressable by an exact target.
      - **targets**:
        - **id**: entrypoint-051-rust-crates-syu-work-model-src-lib-rs-work-plan-digest
          - **adapter**: rust
          - **path**: crates/syu-work-model/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: work_plan_digest
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.work-planning
    - **id**: public-api-split-work-recommendation
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Govern bounded split recommendations.
      - **targets**:
        - **id**: entrypoint-split-work-recommendation
          - **adapter**: rust
          - **path**: crates/syu-planner/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: split_work_recommendation
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.work-planning
    - **id**: public-api-readonly-targets-fingerprint
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Govern readonly target fingerprints.
      - **targets**:
        - **id**: entrypoint-readonly-targets-fingerprint
          - **adapter**: rust
          - **path**: crates/syu-work-model/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: readonly_targets_fingerprint
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.work-planning
    - **id**: public-readiness
      - **role**: verification
      - **facet**: public
      - **responsibility**: Prove work planning entrypoints have bounded canonical plans.
      - **targets**:
        - **id**: canonical-plans
          - **adapter**: rust
          - **path**: crates/syu-validation/src/readiness.rs
          - **selector**:
            - **kind**: symbol
            - **name**: tests::work_planning_public_entrypoints_have_canonical_plans
          - **claims**:
            - **kind**: verifies
              - **criterion**: REQ-PUBLIC-001#criterion.work-planning
              - **covers**:
                - FEAT-PUBLIC-WORK-PLANNING-001#binding.public-api-043/target.entrypoint-048-rust-crates-syu-work-model-src-lib-rs-requestedtarget-c
                - FEAT-PUBLIC-WORK-PLANNING-001#binding.public-api-044/target.entrypoint-049-rust-crates-syu-work-model-src-lib-rs-requestedtarget-r
                - FEAT-PUBLIC-WORK-PLANNING-001#binding.public-api-045/target.entrypoint-050-rust-crates-syu-work-model-src-lib-rs-requestedtarget-t
                - FEAT-PUBLIC-WORK-PLANNING-001#binding.public-api-046/target.entrypoint-051-rust-crates-syu-work-model-src-lib-rs-work-plan-digest
                - FEAT-PUBLIC-WORK-PLANNING-001#binding.public-api-split-work-recommendation/target.entrypoint-split-work-recommendation
                - FEAT-PUBLIC-WORK-PLANNING-001#binding.public-api-readonly-targets-fingerprint/target.entrypoint-readonly-targets-fingerprint
              - **runner**:
                - **runner**: cargo-test
                - **arguments**:
                  - **package**: syu-validation
                  - **test**: tests::work_planning_public_entrypoints_have_canonical_plans
- **id**: FEAT-PUBLIC-WORKBENCH-SERVER-001
  - **title**: Workbench server lifecycle
  - **summary**: Govern Workbench server construction, projection, routing, and execution.
  - **status**: implemented
  - **bindings**:
    - **id**: public-api-047
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint addressable by an exact target.
      - **targets**:
        - **id**: entrypoint-052-rust-crates-syu-workbench-server-src-lib-rs-branchscope
          - **adapter**: rust
          - **path**: crates/syu-workbench-server/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: BranchScopeView::not_applicable
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.workbench-server-lifecycle
    - **id**: public-api-048
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint addressable by an exact target.
      - **targets**:
        - **id**: entrypoint-053-rust-crates-syu-workbench-server-src-lib-rs-validationr
          - **adapter**: rust
          - **path**: crates/syu-workbench-server/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: ValidationRunView::completed
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.workbench-server-lifecycle
    - **id**: public-api-049
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint addressable by an exact target.
      - **targets**:
        - **id**: entrypoint-054-rust-crates-syu-workbench-server-src-lib-rs-validationr
          - **adapter**: rust
          - **path**: crates/syu-workbench-server/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: ValidationRunView::failed
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.workbench-server-lifecycle
    - **id**: public-api-050
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint addressable by an exact target.
      - **targets**:
        - **id**: entrypoint-055-rust-crates-syu-workbench-server-src-lib-rs-validationr
          - **adapter**: rust
          - **path**: crates/syu-workbench-server/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: ValidationRunView::not_applicable
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.workbench-server-lifecycle
    - **id**: public-api-051
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint addressable by an exact target.
      - **targets**:
        - **id**: entrypoint-056-rust-crates-syu-workbench-server-src-lib-rs-validationr
          - **adapter**: rust
          - **path**: crates/syu-workbench-server/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: ValidationRunView::not_run
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.workbench-server-lifecycle
    - **id**: public-api-052
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint addressable by an exact target.
      - **targets**:
        - **id**: entrypoint-057-rust-crates-syu-workbench-server-src-lib-rs-workbenchse
          - **adapter**: rust
          - **path**: crates/syu-workbench-server/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: WorkbenchServer::new
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.workbench-server-lifecycle
    - **id**: public-api-053
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint addressable by an exact target.
      - **targets**:
        - **id**: entrypoint-058-rust-crates-syu-workbench-server-src-lib-rs-workbenchse
          - **adapter**: rust
          - **path**: crates/syu-workbench-server/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: WorkbenchServer::projection
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.workbench-server-lifecycle
    - **id**: public-api-054
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint addressable by an exact target.
      - **targets**:
        - **id**: entrypoint-059-rust-crates-syu-workbench-server-src-lib-rs-workbenchse
          - **adapter**: rust
          - **path**: crates/syu-workbench-server/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: WorkbenchServer::router
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.workbench-server-lifecycle
    - **id**: public-api-055
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint addressable by an exact target.
      - **targets**:
        - **id**: entrypoint-060-rust-crates-syu-workbench-server-src-lib-rs-workbenchse
          - **adapter**: rust
          - **path**: crates/syu-workbench-server/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: WorkbenchServer::run
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.workbench-server-lifecycle
    - **id**: public-api-056
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint addressable by an exact target.
      - **targets**:
        - **id**: entrypoint-061-rust-crates-syu-workbench-server-src-lib-rs-workbenchse
          - **adapter**: rust
          - **path**: crates/syu-workbench-server/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: WorkbenchServer::with_launch
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.workbench-server-lifecycle
    - **id**: public-api-057
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint addressable by an exact target.
      - **targets**:
        - **id**: entrypoint-062-rust-crates-syu-workbench-server-src-lib-rs-workbenchse
          - **adapter**: rust
          - **path**: crates/syu-workbench-server/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: WorkbenchServer::with_request
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.workbench-server-lifecycle
    - **id**: public-readiness
      - **role**: verification
      - **facet**: public
      - **responsibility**: Prove workbench server lifecycle entrypoints have bounded canonical plans.
      - **targets**:
        - **id**: canonical-plans
          - **adapter**: rust
          - **path**: crates/syu-validation/src/readiness.rs
          - **selector**:
            - **kind**: symbol
            - **name**: tests::workbench_server_lifecycle_public_entrypoints_have_canonical_plans
          - **claims**:
            - **kind**: verifies
              - **criterion**: REQ-PUBLIC-001#criterion.workbench-server-lifecycle
              - **covers**:
                - FEAT-PUBLIC-WORKBENCH-SERVER-001#binding.public-api-047/target.entrypoint-052-rust-crates-syu-workbench-server-src-lib-rs-branchscope
                - FEAT-PUBLIC-WORKBENCH-SERVER-001#binding.public-api-048/target.entrypoint-053-rust-crates-syu-workbench-server-src-lib-rs-validationr
                - FEAT-PUBLIC-WORKBENCH-SERVER-001#binding.public-api-049/target.entrypoint-054-rust-crates-syu-workbench-server-src-lib-rs-validationr
                - FEAT-PUBLIC-WORKBENCH-SERVER-001#binding.public-api-050/target.entrypoint-055-rust-crates-syu-workbench-server-src-lib-rs-validationr
                - FEAT-PUBLIC-WORKBENCH-SERVER-001#binding.public-api-051/target.entrypoint-056-rust-crates-syu-workbench-server-src-lib-rs-validationr
                - FEAT-PUBLIC-WORKBENCH-SERVER-001#binding.public-api-052/target.entrypoint-057-rust-crates-syu-workbench-server-src-lib-rs-workbenchse
                - FEAT-PUBLIC-WORKBENCH-SERVER-001#binding.public-api-053/target.entrypoint-058-rust-crates-syu-workbench-server-src-lib-rs-workbenchse
                - FEAT-PUBLIC-WORKBENCH-SERVER-001#binding.public-api-054/target.entrypoint-059-rust-crates-syu-workbench-server-src-lib-rs-workbenchse
                - FEAT-PUBLIC-WORKBENCH-SERVER-001#binding.public-api-055/target.entrypoint-060-rust-crates-syu-workbench-server-src-lib-rs-workbenchse
                - FEAT-PUBLIC-WORKBENCH-SERVER-001#binding.public-api-056/target.entrypoint-061-rust-crates-syu-workbench-server-src-lib-rs-workbenchse
                - FEAT-PUBLIC-WORKBENCH-SERVER-001#binding.public-api-057/target.entrypoint-062-rust-crates-syu-workbench-server-src-lib-rs-workbenchse
              - **runner**:
                - **runner**: cargo-test
                - **arguments**:
                  - **package**: syu-validation
                  - **test**: tests::workbench_server_lifecycle_public_entrypoints_have_canonical_plans
- **id**: FEAT-PUBLIC-WORKBENCH-VALIDATION-001
  - **title**: Workbench server validation
  - **summary**: Govern branch-scope and verification-receipt entrypoints.
  - **status**: implemented
  - **bindings**:
    - **id**: public-api-058
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint addressable by an exact target.
      - **targets**:
        - **id**: entrypoint-063-rust-crates-syu-workbench-server-src-lib-rs-branch-scop
          - **adapter**: rust
          - **path**: crates/syu-workbench-server/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: branch_scope_view
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.workbench-server-validation
    - **id**: public-api-059
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint addressable by an exact target.
      - **targets**:
        - **id**: entrypoint-064-rust-crates-syu-workbench-server-src-lib-rs-execute-ver
          - **adapter**: rust
          - **path**: crates/syu-workbench-server/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: execute_verification
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.workbench-server-validation
    - **id**: public-api-060
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint addressable by an exact target.
      - **targets**:
        - **id**: entrypoint-065-rust-crates-syu-workbench-server-src-lib-rs-validate-ve
          - **adapter**: rust
          - **path**: crates/syu-workbench-server/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: validate_verification_receipt
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.workbench-server-validation
    - **id**: public-readiness
      - **role**: verification
      - **facet**: public
      - **responsibility**: Prove workbench server validation entrypoints have bounded canonical plans.
      - **targets**:
        - **id**: canonical-plans
          - **adapter**: rust
          - **path**: crates/syu-validation/src/readiness.rs
          - **selector**:
            - **kind**: symbol
            - **name**: tests::workbench_server_validation_public_entrypoints_have_canonical_plans
          - **claims**:
            - **kind**: verifies
              - **criterion**: REQ-PUBLIC-001#criterion.workbench-server-validation
              - **covers**:
                - FEAT-PUBLIC-WORKBENCH-VALIDATION-001#binding.public-api-058/target.entrypoint-063-rust-crates-syu-workbench-server-src-lib-rs-branch-scop
                - FEAT-PUBLIC-WORKBENCH-VALIDATION-001#binding.public-api-059/target.entrypoint-064-rust-crates-syu-workbench-server-src-lib-rs-execute-ver
                - FEAT-PUBLIC-WORKBENCH-VALIDATION-001#binding.public-api-060/target.entrypoint-065-rust-crates-syu-workbench-server-src-lib-rs-validate-ve
              - **runner**:
                - **runner**: cargo-test
                - **arguments**:
                  - **package**: syu-validation
                  - **test**: tests::workbench_server_validation_public_entrypoints_have_canonical_plans
- **id**: FEAT-PUBLIC-WORKSPACE-LOADING-001
  - **title**: Workspace loading
  - **summary**: Govern workspace loading, overlays, index access, and content reads.
  - **status**: implemented
  - **bindings**:
    - **id**: public-api-061
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint addressable by an exact target.
      - **targets**:
        - **id**: entrypoint-066-rust-crates-syu-workspace-src-lib-rs-specindex-anchor
          - **adapter**: rust
          - **path**: crates/syu-workspace/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: SpecIndex::anchor
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.workspace-loading
    - **id**: public-api-062
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint addressable by an exact target.
      - **targets**:
        - **id**: entrypoint-067-rust-crates-syu-workspace-src-lib-rs-specindex-build
          - **adapter**: rust
          - **path**: crates/syu-workspace/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: SpecIndex::build
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.workspace-loading
    - **id**: public-api-063
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint addressable by an exact target.
      - **targets**:
        - **id**: entrypoint-068-rust-crates-syu-workspace-src-lib-rs-specindex-target
          - **adapter**: rust
          - **path**: crates/syu-workspace/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: SpecIndex::target
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.workspace-loading
    - **id**: public-api-064
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint addressable by an exact target.
      - **targets**:
        - **id**: entrypoint-069-rust-crates-syu-workspace-src-lib-rs-specworkspace-fing
          - **adapter**: rust
          - **path**: crates/syu-workspace/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: SpecWorkspace::fingerprint
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.workspace-loading
    - **id**: public-api-065
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint addressable by an exact target.
      - **targets**:
        - **id**: entrypoint-070-rust-crates-syu-workspace-src-lib-rs-specworkspace-inde
          - **adapter**: rust
          - **path**: crates/syu-workspace/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: SpecWorkspace::index
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.workspace-loading
    - **id**: public-api-066
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint addressable by an exact target.
      - **targets**:
        - **id**: entrypoint-071-rust-crates-syu-workspace-src-lib-rs-specworkspace-load
          - **adapter**: rust
          - **path**: crates/syu-workspace/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: SpecWorkspace::load
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.workspace-loading
    - **id**: public-api-067
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint addressable by an exact target.
      - **targets**:
        - **id**: entrypoint-072-rust-crates-syu-workspace-src-lib-rs-specworkspace-over
          - **adapter**: rust
          - **path**: crates/syu-workspace/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: SpecWorkspace::overlay_config
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.workspace-loading
    - **id**: public-api-068
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint addressable by an exact target.
      - **targets**:
        - **id**: entrypoint-073-rust-crates-syu-workspace-src-lib-rs-specworkspace-over
          - **adapter**: rust
          - **path**: crates/syu-workspace/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: SpecWorkspace::overlay_document
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.workspace-loading
    - **id**: public-api-072
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint addressable by an exact target.
      - **targets**:
        - **id**: entrypoint-077-rust-crates-syu-workspace-src-lib-rs-specworkspace-read
          - **adapter**: rust
          - **path**: crates/syu-workspace/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: SpecWorkspace::read_bytes
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.workspace-loading
    - **id**: public-api-073
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint addressable by an exact target.
      - **targets**:
        - **id**: entrypoint-078-rust-crates-syu-workspace-src-lib-rs-specworkspace-read
          - **adapter**: rust
          - **path**: crates/syu-workspace/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: SpecWorkspace::read_to_string
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.workspace-loading
    - **id**: public-readiness
      - **role**: verification
      - **facet**: public
      - **responsibility**: Prove workspace loading entrypoints have bounded canonical plans.
      - **targets**:
        - **id**: canonical-plans
          - **adapter**: rust
          - **path**: crates/syu-validation/src/readiness.rs
          - **selector**:
            - **kind**: symbol
            - **name**: tests::workspace_loading_public_entrypoints_have_canonical_plans
          - **claims**:
            - **kind**: verifies
              - **criterion**: REQ-PUBLIC-001#criterion.workspace-loading
              - **covers**:
                - FEAT-PUBLIC-WORKSPACE-LOADING-001#binding.public-api-061/target.entrypoint-066-rust-crates-syu-workspace-src-lib-rs-specindex-anchor
                - FEAT-PUBLIC-WORKSPACE-LOADING-001#binding.public-api-062/target.entrypoint-067-rust-crates-syu-workspace-src-lib-rs-specindex-build
                - FEAT-PUBLIC-WORKSPACE-LOADING-001#binding.public-api-063/target.entrypoint-068-rust-crates-syu-workspace-src-lib-rs-specindex-target
                - FEAT-PUBLIC-WORKSPACE-LOADING-001#binding.public-api-064/target.entrypoint-069-rust-crates-syu-workspace-src-lib-rs-specworkspace-fing
                - FEAT-PUBLIC-WORKSPACE-LOADING-001#binding.public-api-065/target.entrypoint-070-rust-crates-syu-workspace-src-lib-rs-specworkspace-inde
                - FEAT-PUBLIC-WORKSPACE-LOADING-001#binding.public-api-066/target.entrypoint-071-rust-crates-syu-workspace-src-lib-rs-specworkspace-load
                - FEAT-PUBLIC-WORKSPACE-LOADING-001#binding.public-api-067/target.entrypoint-072-rust-crates-syu-workspace-src-lib-rs-specworkspace-over
                - FEAT-PUBLIC-WORKSPACE-LOADING-001#binding.public-api-068/target.entrypoint-073-rust-crates-syu-workspace-src-lib-rs-specworkspace-over
                - FEAT-PUBLIC-WORKSPACE-LOADING-001#binding.public-api-072/target.entrypoint-077-rust-crates-syu-workspace-src-lib-rs-specworkspace-read
                - FEAT-PUBLIC-WORKSPACE-LOADING-001#binding.public-api-073/target.entrypoint-078-rust-crates-syu-workspace-src-lib-rs-specworkspace-read
              - **runner**:
                - **runner**: cargo-test
                - **arguments**:
                  - **package**: syu-validation
                  - **test**: tests::workspace_loading_public_entrypoints_have_canonical_plans
- **id**: FEAT-PUBLIC-WORKSPACE-RESOLUTION-001
  - **title**: Workspace resolution
  - **summary**: Govern selector resolution and stable workspace fingerprints.
  - **status**: implemented
  - **bindings**:
    - **id**: public-api-069
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint addressable by an exact target.
      - **targets**:
        - **id**: entrypoint-074-rust-crates-syu-workspace-src-lib-rs-specworkspace-path
          - **adapter**: rust
          - **path**: crates/syu-workspace/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: SpecWorkspace::path_is_artifact
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.workspace-resolution
    - **id**: public-api-070
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint addressable by an exact target.
      - **targets**:
        - **id**: entrypoint-075-rust-crates-syu-workspace-src-lib-rs-specworkspace-path
          - **adapter**: rust
          - **path**: crates/syu-workspace/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: SpecWorkspace::path_is_excluded
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.workspace-resolution
    - **id**: public-api-071
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint addressable by an exact target.
      - **targets**:
        - **id**: entrypoint-076-rust-crates-syu-workspace-src-lib-rs-specworkspace-path
          - **adapter**: rust
          - **path**: crates/syu-workspace/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: SpecWorkspace::path_is_spec
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.workspace-resolution
    - **id**: public-api-074
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint addressable by an exact target.
      - **targets**:
        - **id**: entrypoint-079-rust-crates-syu-workspace-src-lib-rs-specworkspace-try
          - **adapter**: rust
          - **path**: crates/syu-workspace/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: SpecWorkspace::try_fingerprint
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.workspace-resolution
    - **id**: public-api-075
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint addressable by an exact target.
      - **targets**:
        - **id**: entrypoint-080-rust-crates-syu-workspace-src-lib-rs-resolve-target
          - **adapter**: rust
          - **path**: crates/syu-workspace/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: resolve_target
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.workspace-resolution
    - **id**: public-api-076
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint addressable by an exact target.
      - **targets**:
        - **id**: entrypoint-082-rust-crates-syu-workspace-src-lib-rs-resolve-target-wit
          - **adapter**: rust
          - **path**: crates/syu-workspace/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: resolve_target_with_adapters
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.workspace-resolution
    - **id**: public-api-077
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint addressable by an exact target.
      - **targets**:
        - **id**: entrypoint-083-rust-crates-syu-workspace-src-lib-rs-selector-supports
          - **adapter**: rust
          - **path**: crates/syu-workspace/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: selector_supports_editable
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.workspace-resolution
    - **id**: public-api-resolve-indexed-target
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint addressable by an exact target.
      - **targets**:
        - **id**: entrypoint-resolve-indexed-target
          - **adapter**: rust
          - **path**: crates/syu-workspace/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: resolve_indexed_target
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.workspace-resolution
    - **id**: public-api-ownership-fingerprint
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Govern ownership graph fingerprints.
      - **targets**:
        - **id**: entrypoint-ownership-fingerprint
          - **adapter**: rust
          - **path**: crates/syu-workspace/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: ownership_fingerprint
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.workspace-resolution
    - **id**: public-api-spec-fingerprint
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Govern specification graph fingerprints.
      - **targets**:
        - **id**: entrypoint-spec-fingerprint
          - **adapter**: rust
          - **path**: crates/syu-workspace/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: spec_fingerprint
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.workspace-resolution
    - **id**: public-readiness
      - **role**: verification
      - **facet**: public
      - **responsibility**: Prove workspace resolution entrypoints have bounded canonical plans.
      - **targets**:
        - **id**: canonical-plans
          - **adapter**: rust
          - **path**: crates/syu-validation/src/readiness.rs
          - **selector**:
            - **kind**: symbol
            - **name**: tests::workspace_resolution_public_entrypoints_have_canonical_plans
          - **claims**:
            - **kind**: verifies
              - **criterion**: REQ-PUBLIC-001#criterion.workspace-resolution
              - **covers**:
                - FEAT-PUBLIC-WORKSPACE-RESOLUTION-001#binding.public-api-069/target.entrypoint-074-rust-crates-syu-workspace-src-lib-rs-specworkspace-path
                - FEAT-PUBLIC-WORKSPACE-RESOLUTION-001#binding.public-api-070/target.entrypoint-075-rust-crates-syu-workspace-src-lib-rs-specworkspace-path
                - FEAT-PUBLIC-WORKSPACE-RESOLUTION-001#binding.public-api-071/target.entrypoint-076-rust-crates-syu-workspace-src-lib-rs-specworkspace-path
                - FEAT-PUBLIC-WORKSPACE-RESOLUTION-001#binding.public-api-074/target.entrypoint-079-rust-crates-syu-workspace-src-lib-rs-specworkspace-try
                - FEAT-PUBLIC-WORKSPACE-RESOLUTION-001#binding.public-api-075/target.entrypoint-080-rust-crates-syu-workspace-src-lib-rs-resolve-target
                - FEAT-PUBLIC-WORKSPACE-RESOLUTION-001#binding.public-api-076/target.entrypoint-082-rust-crates-syu-workspace-src-lib-rs-resolve-target-wit
                - FEAT-PUBLIC-WORKSPACE-RESOLUTION-001#binding.public-api-077/target.entrypoint-083-rust-crates-syu-workspace-src-lib-rs-selector-supports
                - FEAT-PUBLIC-WORKSPACE-RESOLUTION-001#binding.public-api-resolve-indexed-target/target.entrypoint-resolve-indexed-target
                - FEAT-PUBLIC-WORKSPACE-RESOLUTION-001#binding.public-api-ownership-fingerprint/target.entrypoint-ownership-fingerprint
                - FEAT-PUBLIC-WORKSPACE-RESOLUTION-001#binding.public-api-spec-fingerprint/target.entrypoint-spec-fingerprint
              - **runner**:
                - **runner**: cargo-test
                - **arguments**:
                  - **package**: syu-validation
                  - **test**: tests::workspace_resolution_public_entrypoints_have_canonical_plans
- **id**: FEAT-PUBLIC-AGENT-DELIVERY-001
  - **title**: Agent delivery evidence
  - **summary**: Govern scoped-agent and durable delivery evidence entrypoints.
  - **status**: implemented
  - **bindings**:
    - **id**: public-api-agent-current-run
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Govern current scoped-agent run reads.
      - **targets**:
        - **id**: entrypoint-agent-current-run
          - **adapter**: rust
          - **path**: crates/syu-agent/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: current_run
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.agent-delivery
    - **id**: public-api-agent-events
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Govern scoped-agent event reads.
      - **targets**:
        - **id**: entrypoint-agent-events
          - **adapter**: rust
          - **path**: crates/syu-agent/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: events
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.agent-delivery
    - **id**: public-api-agent-record-blocker
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Govern scoped-agent blocker evidence.
      - **targets**:
        - **id**: entrypoint-agent-record-blocker
          - **adapter**: rust
          - **path**: crates/syu-agent/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: record_blocker
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.agent-delivery
    - **id**: public-api-agent-record-verification
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Govern scoped-agent verification evidence.
      - **targets**:
        - **id**: entrypoint-agent-record-verification
          - **adapter**: rust
          - **path**: crates/syu-agent/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: record_verification
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.agent-delivery
    - **id**: public-api-delivery-agent-events
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Govern durable agent event reads.
      - **targets**:
        - **id**: entrypoint-delivery-agent-events
          - **adapter**: rust
          - **path**: crates/syu-delivery/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: agent_events
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.agent-delivery
    - **id**: public-api-delivery-agent-run
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Govern durable agent run reads.
      - **targets**:
        - **id**: entrypoint-delivery-agent-run
          - **adapter**: rust
          - **path**: crates/syu-delivery/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: agent_run
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.agent-delivery
    - **id**: public-api-delivery-latest-agent-run
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Govern latest durable agent run reads.
      - **targets**:
        - **id**: entrypoint-delivery-latest-agent-run
          - **adapter**: rust
          - **path**: crates/syu-delivery/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: latest_agent_run
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.agent-delivery
    - **id**: public-readiness
      - **role**: verification
      - **facet**: public
      - **responsibility**: Prove agent delivery evidence entrypoints have bounded canonical plans.
      - **targets**:
        - **id**: canonical-plans
          - **adapter**: rust
          - **path**: crates/syu-validation/src/readiness.rs
          - **selector**:
            - **kind**: symbol
            - **name**: tests::agent_delivery_public_entrypoints_have_canonical_plans
          - **claims**:
            - **kind**: verifies
              - **criterion**: REQ-PUBLIC-001#criterion.agent-delivery
              - **covers**:
                - FEAT-PUBLIC-AGENT-DELIVERY-001#binding.public-api-agent-current-run/target.entrypoint-agent-current-run
                - FEAT-PUBLIC-AGENT-DELIVERY-001#binding.public-api-agent-events/target.entrypoint-agent-events
                - FEAT-PUBLIC-AGENT-DELIVERY-001#binding.public-api-agent-record-blocker/target.entrypoint-agent-record-blocker
                - FEAT-PUBLIC-AGENT-DELIVERY-001#binding.public-api-agent-record-verification/target.entrypoint-agent-record-verification
                - FEAT-PUBLIC-AGENT-DELIVERY-001#binding.public-api-delivery-agent-events/target.entrypoint-delivery-agent-events
                - FEAT-PUBLIC-AGENT-DELIVERY-001#binding.public-api-delivery-agent-run/target.entrypoint-delivery-agent-run
                - FEAT-PUBLIC-AGENT-DELIVERY-001#binding.public-api-delivery-latest-agent-run/target.entrypoint-delivery-latest-agent-run
              - **runner**:
                - **runner**: cargo-test
                - **arguments**:
                  - **package**: syu-validation
                  - **test**: tests::agent_delivery_public_entrypoints_have_canonical_plans

## Source YAML

```yaml
schema: syu/spec/v1
kind: features
namespace: public
category: Public entrypoint contracts
features:
- id: FEAT-PUBLIC-WORKBENCH-TRANSPORT-001
  title: Workbench client transport
  summary: Govern canonical browser transport, session, projection, scope, and source
    reads.
  status: implemented
  bindings:
  - id: public-api-001
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint addressable by an exact target.
    targets:
    - id: entrypoint-001-javascript-crates-syu-app-ui-assets-js-api-js-mutationb
      adapter: javascript
      path: crates/syu-app-ui/assets/js/api.js
      selector:
        kind: symbol
        name: mutationBasis
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.workbench-client-transport
  - id: public-api-002
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint addressable by an exact target.
    targets:
    - id: entrypoint-002-javascript-crates-syu-app-ui-assets-js-api-js-post
      adapter: javascript
      path: crates/syu-app-ui/assets/js/api.js
      selector:
        kind: symbol
        name: post
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.workbench-client-transport
  - id: public-api-003
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint addressable by an exact target.
    targets:
    - id: entrypoint-003-javascript-crates-syu-app-ui-assets-js-api-js-readproje
      adapter: javascript
      path: crates/syu-app-ui/assets/js/api.js
      selector:
        kind: symbol
        name: readProjection
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.workbench-client-transport
  - id: public-api-004
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint addressable by an exact target.
    targets:
    - id: entrypoint-004-javascript-crates-syu-app-ui-assets-js-api-js-request
      adapter: javascript
      path: crates/syu-app-ui/assets/js/api.js
      selector:
        kind: symbol
        name: request
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.workbench-client-transport
  - id: public-api-establish-session
    role: implementation
    facet: public
    responsibility: Govern Workbench session establishment.
    targets:
    - id: entrypoint-establish-session
      adapter: javascript
      path: crates/syu-app-ui/assets/js/api.js
      selector:
        kind: symbol
        name: establishSession
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.workbench-client-transport
  - id: public-api-read-branch-scope
    role: implementation
    facet: public
    responsibility: Govern branch-scope reads.
    targets:
    - id: entrypoint-read-branch-scope
      adapter: javascript
      path: crates/syu-app-ui/assets/js/api.js
      selector:
        kind: symbol
        name: readBranchScope
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.workbench-client-transport
  - id: public-api-read-scope-diff
    role: implementation
    facet: public
    responsibility: Govern scope diff reads.
    targets:
    - id: entrypoint-read-scope-diff
      adapter: javascript
      path: crates/syu-app-ui/assets/js/api.js
      selector:
        kind: symbol
        name: readScopeDiff
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.workbench-client-transport
  - id: public-api-read-source
    role: implementation
    facet: public
    responsibility: Govern linked source reads.
    targets:
    - id: entrypoint-read-source
      adapter: javascript
      path: crates/syu-app-ui/assets/js/api.js
      selector:
        kind: symbol
        name: readSource
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.workbench-client-transport
  - id: public-api-read-target-source
    role: implementation
    facet: public
    responsibility: Govern exact target source reads.
    targets:
    - id: entrypoint-read-target-source
      adapter: javascript
      path: crates/syu-app-ui/assets/js/api.js
      selector:
        kind: symbol
        name: readTargetSource
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.workbench-client-transport
  - id: public-readiness
    role: verification
    facet: public
    responsibility: Prove workbench client transport entrypoints have bounded canonical
      plans.
    targets:
    - id: canonical-plans
      adapter: rust
      path: crates/syu-validation/src/readiness.rs
      selector:
        kind: symbol
        name: tests::workbench_client_transport_public_entrypoints_have_canonical_plans
      claims:
      - kind: verifies
        criterion: REQ-PUBLIC-001#criterion.workbench-client-transport
        covers:
        - FEAT-PUBLIC-WORKBENCH-TRANSPORT-001#binding.public-api-001/target.entrypoint-001-javascript-crates-syu-app-ui-assets-js-api-js-mutationb
        - FEAT-PUBLIC-WORKBENCH-TRANSPORT-001#binding.public-api-002/target.entrypoint-002-javascript-crates-syu-app-ui-assets-js-api-js-post
        - FEAT-PUBLIC-WORKBENCH-TRANSPORT-001#binding.public-api-003/target.entrypoint-003-javascript-crates-syu-app-ui-assets-js-api-js-readproje
        - FEAT-PUBLIC-WORKBENCH-TRANSPORT-001#binding.public-api-004/target.entrypoint-004-javascript-crates-syu-app-ui-assets-js-api-js-request
        - FEAT-PUBLIC-WORKBENCH-TRANSPORT-001#binding.public-api-establish-session/target.entrypoint-establish-session
        - FEAT-PUBLIC-WORKBENCH-TRANSPORT-001#binding.public-api-read-branch-scope/target.entrypoint-read-branch-scope
        - FEAT-PUBLIC-WORKBENCH-TRANSPORT-001#binding.public-api-read-scope-diff/target.entrypoint-read-scope-diff
        - FEAT-PUBLIC-WORKBENCH-TRANSPORT-001#binding.public-api-read-source/target.entrypoint-read-source
        - FEAT-PUBLIC-WORKBENCH-TRANSPORT-001#binding.public-api-read-target-source/target.entrypoint-read-target-source
        runner:
          runner: cargo-test
          arguments:
            package: syu-validation
            test: tests::workbench_client_transport_public_entrypoints_have_canonical_plans
- id: FEAT-PUBLIC-WORKBENCH-ACTIONS-001
  title: Workbench client actions
  summary: Govern typed Workbench preview, approval, diagnostics, readiness, and search
    actions.
  status: implemented
  bindings:
  - id: public-api-005
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint addressable by an exact target.
    targets:
    - id: entrypoint-005-javascript-crates-syu-app-ui-assets-js-api-js-runjourn
      adapter: javascript
      path: crates/syu-app-ui/assets/js/api.js
      selector:
        kind: symbol
        name: runJourneyAction
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.workbench-client-actions
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
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.workbench-client-actions
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
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.workbench-client-actions
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
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.workbench-client-actions
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
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.workbench-client-actions
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
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.workbench-client-actions
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
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.workbench-client-actions
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
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.workbench-client-actions
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
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.workbench-client-actions
  - id: public-readiness
    role: verification
    facet: public
    responsibility: Prove workbench client actions entrypoints have bounded canonical
      plans.
    targets:
    - id: canonical-plans
      adapter: rust
      path: crates/syu-validation/src/readiness.rs
      selector:
        kind: symbol
        name: tests::workbench_client_actions_public_entrypoints_have_canonical_plans
      claims:
      - kind: verifies
        criterion: REQ-PUBLIC-001#criterion.workbench-client-actions
        covers:
        - FEAT-PUBLIC-WORKBENCH-ACTIONS-001#binding.public-api-005/target.entrypoint-005-javascript-crates-syu-app-ui-assets-js-api-js-runjourn
        - FEAT-PUBLIC-WORKBENCH-ACTIONS-001#binding.public-api-apply-specification-candidate/target.entrypoint-apply-specification-candidate
        - FEAT-PUBLIC-WORKBENCH-ACTIONS-001#binding.public-api-approve-target-suggestions/target.entrypoint-approve-target-suggestions
        - FEAT-PUBLIC-WORKBENCH-ACTIONS-001#binding.public-api-preview-specification-candidate/target.entrypoint-preview-specification-candidate
        - FEAT-PUBLIC-WORKBENCH-ACTIONS-001#binding.public-api-read-target-suggestions/target.entrypoint-read-target-suggestions
        - FEAT-PUBLIC-WORKBENCH-ACTIONS-001#binding.public-api-reject-target-suggestion/target.entrypoint-reject-target-suggestion
        - FEAT-PUBLIC-WORKBENCH-ACTIONS-001#binding.public-api-run-diagnostics/target.entrypoint-run-diagnostics
        - FEAT-PUBLIC-WORKBENCH-ACTIONS-001#binding.public-api-run-readiness/target.entrypoint-run-readiness
        - FEAT-PUBLIC-WORKBENCH-ACTIONS-001#binding.public-api-search-specification-candidates/target.entrypoint-search-specification-candidates
        runner:
          runner: cargo-test
          arguments:
            package: syu-validation
            test: tests::workbench_client_actions_public_entrypoints_have_canonical_plans
- id: FEAT-PUBLIC-WORKBENCH-COMPONENTS-001
  title: Workbench components
  summary: Govern reusable Workbench rendering, translation, and diff components.
  status: implemented
  bindings:
  - id: public-api-006
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint addressable by an exact target.
    targets:
    - id: entrypoint-006-javascript-crates-syu-app-ui-assets-js-components-actio
      adapter: javascript
      path: crates/syu-app-ui/assets/js/components/action.js
      selector:
        kind: symbol
        name: actionCapability
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.workbench-components
  - id: public-api-007
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint addressable by an exact target.
    targets:
    - id: entrypoint-007-javascript-crates-syu-app-ui-assets-js-components-diagn
      adapter: javascript
      path: crates/syu-app-ui/assets/js/components/diagnostic.js
      selector:
        kind: symbol
        name: renderDiagnostic
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.workbench-components
  - id: public-api-008
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint addressable by an exact target.
    targets:
    - id: entrypoint-008-javascript-crates-syu-app-ui-assets-js-components-edito
      adapter: javascript
      path: crates/syu-app-ui/assets/js/components/editor.js
      selector:
        kind: symbol
        name: structuredEditor
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.workbench-components
  - id: public-api-009
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint addressable by an exact target.
    targets:
    - id: entrypoint-009-javascript-crates-syu-app-ui-assets-js-components-readi
      adapter: javascript
      path: crates/syu-app-ui/assets/js/components/readiness.js
      selector:
        kind: symbol
        name: renderReadiness
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.workbench-components
  - id: public-api-010
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint addressable by an exact target.
    targets:
    - id: entrypoint-010-javascript-crates-syu-app-ui-assets-js-components-targe
      adapter: javascript
      path: crates/syu-app-ui/assets/js/components/target.js
      selector:
        kind: symbol
        name: renderTarget
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.workbench-components
  - id: public-api-011
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint addressable by an exact target.
    targets:
    - id: entrypoint-011-javascript-crates-syu-app-ui-assets-js-i18n-js-translat
      adapter: javascript
      path: crates/syu-app-ui/assets/js/i18n.js
      selector:
        kind: symbol
        name: translate
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.workbench-components
  - id: public-api-render-diff
    role: implementation
    facet: public
    responsibility: Govern bounded diff rendering.
    targets:
    - id: entrypoint-render-diff
      adapter: javascript
      path: crates/syu-app-ui/assets/js/components/diff.js
      selector:
        kind: symbol
        name: renderDiff
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.workbench-components
  - id: public-readiness
    role: verification
    facet: public
    responsibility: Prove workbench components entrypoints have bounded canonical
      plans.
    targets:
    - id: canonical-plans
      adapter: rust
      path: crates/syu-validation/src/readiness.rs
      selector:
        kind: symbol
        name: tests::workbench_components_public_entrypoints_have_canonical_plans
      claims:
      - kind: verifies
        criterion: REQ-PUBLIC-001#criterion.workbench-components
        covers:
        - FEAT-PUBLIC-WORKBENCH-COMPONENTS-001#binding.public-api-006/target.entrypoint-006-javascript-crates-syu-app-ui-assets-js-components-actio
        - FEAT-PUBLIC-WORKBENCH-COMPONENTS-001#binding.public-api-007/target.entrypoint-007-javascript-crates-syu-app-ui-assets-js-components-diagn
        - FEAT-PUBLIC-WORKBENCH-COMPONENTS-001#binding.public-api-008/target.entrypoint-008-javascript-crates-syu-app-ui-assets-js-components-edito
        - FEAT-PUBLIC-WORKBENCH-COMPONENTS-001#binding.public-api-009/target.entrypoint-009-javascript-crates-syu-app-ui-assets-js-components-readi
        - FEAT-PUBLIC-WORKBENCH-COMPONENTS-001#binding.public-api-010/target.entrypoint-010-javascript-crates-syu-app-ui-assets-js-components-targe
        - FEAT-PUBLIC-WORKBENCH-COMPONENTS-001#binding.public-api-011/target.entrypoint-011-javascript-crates-syu-app-ui-assets-js-i18n-js-translat
        - FEAT-PUBLIC-WORKBENCH-COMPONENTS-001#binding.public-api-render-diff/target.entrypoint-render-diff
        runner:
          runner: cargo-test
          arguments:
            package: syu-validation
            test: tests::workbench_components_public_entrypoints_have_canonical_plans
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
- id: FEAT-PUBLIC-WORKBENCH-NAVIGATION-001
  title: Workbench navigation
  summary: Govern Workbench routes, tabs, selected slices, and landing navigation.
  status: implemented
  bindings:
  - id: public-api-019
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint addressable by an exact target.
    targets:
    - id: entrypoint-019-javascript-crates-syu-app-ui-assets-js-router-js-pages
      adapter: javascript
      path: crates/syu-app-ui/assets/js/router.js
      selector:
        kind: symbol
        name: PAGES
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.workbench-navigation
  - id: public-api-020
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint addressable by an exact target.
    targets:
    - id: entrypoint-020-javascript-crates-syu-app-ui-assets-js-router-js-tab-gr
      adapter: javascript
      path: crates/syu-app-ui/assets/js/router.js
      selector:
        kind: symbol
        name: TAB_GROUPS
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.workbench-navigation
  - id: public-api-021
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint addressable by an exact target.
    targets:
    - id: entrypoint-021-javascript-crates-syu-app-ui-assets-js-router-js-naviga
      adapter: javascript
      path: crates/syu-app-ui/assets/js/router.js
      selector:
        kind: symbol
        name: navigate
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.workbench-navigation
  - id: public-api-022
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint addressable by an exact target.
    targets:
    - id: entrypoint-022-javascript-crates-syu-app-ui-assets-js-state-js-creates
      adapter: javascript
      path: crates/syu-app-ui/assets/js/state.js
      selector:
        kind: symbol
        name: createState
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.workbench-navigation
  - id: public-api-023
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint addressable by an exact target.
    targets:
    - id: entrypoint-023-javascript-crates-syu-app-ui-assets-js-state-js-replace
      adapter: javascript
      path: crates/syu-app-ui/assets/js/state.js
      selector:
        kind: symbol
        name: replaceProjection
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.workbench-navigation
  - id: public-api-024
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint addressable by an exact target.
    targets:
    - id: entrypoint-024-javascript-crates-syu-app-ui-assets-js-state-js-selects
      adapter: javascript
      path: crates/syu-app-ui/assets/js/state.js
      selector:
        kind: symbol
        name: selectSlice
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.workbench-navigation
  - id: public-api-025
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint addressable by an exact target.
    targets:
    - id: entrypoint-025-javascript-website-src-pages-index-js-home
      adapter: javascript
      path: website/src/pages/index.js
      selector:
        kind: symbol
        name: Home
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.workbench-navigation
  - id: public-readiness
    role: verification
    facet: public
    responsibility: Prove workbench navigation entrypoints have bounded canonical
      plans.
    targets:
    - id: canonical-plans
      adapter: rust
      path: crates/syu-validation/src/readiness.rs
      selector:
        kind: symbol
        name: tests::workbench_navigation_public_entrypoints_have_canonical_plans
      claims:
      - kind: verifies
        criterion: REQ-PUBLIC-001#criterion.workbench-navigation
        covers:
        - FEAT-PUBLIC-WORKBENCH-NAVIGATION-001#binding.public-api-019/target.entrypoint-019-javascript-crates-syu-app-ui-assets-js-router-js-pages
        - FEAT-PUBLIC-WORKBENCH-NAVIGATION-001#binding.public-api-020/target.entrypoint-020-javascript-crates-syu-app-ui-assets-js-router-js-tab-gr
        - FEAT-PUBLIC-WORKBENCH-NAVIGATION-001#binding.public-api-021/target.entrypoint-021-javascript-crates-syu-app-ui-assets-js-router-js-naviga
        - FEAT-PUBLIC-WORKBENCH-NAVIGATION-001#binding.public-api-022/target.entrypoint-022-javascript-crates-syu-app-ui-assets-js-state-js-creates
        - FEAT-PUBLIC-WORKBENCH-NAVIGATION-001#binding.public-api-023/target.entrypoint-023-javascript-crates-syu-app-ui-assets-js-state-js-replace
        - FEAT-PUBLIC-WORKBENCH-NAVIGATION-001#binding.public-api-024/target.entrypoint-024-javascript-crates-syu-app-ui-assets-js-state-js-selects
        - FEAT-PUBLIC-WORKBENCH-NAVIGATION-001#binding.public-api-025/target.entrypoint-025-javascript-website-src-pages-index-js-home
        runner:
          runner: cargo-test
          arguments:
            package: syu-validation
            test: tests::workbench_navigation_public_entrypoints_have_canonical_plans
- id: FEAT-PUBLIC-WORKBENCH-RENDERING-001
  title: Workbench shell rendering
  summary: Govern localized server-rendered Workbench shell construction.
  status: implemented
  bindings:
  - id: public-api-026
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint addressable by an exact target.
    targets:
    - id: entrypoint-026-rust-crates-syu-app-ui-src-lib-rs-workbenchview-new
      adapter: rust
      path: crates/syu-app-ui/src/lib.rs
      selector:
        kind: symbol
        name: WorkbenchView::new
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.workbench-rendering
  - id: public-api-027
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint addressable by an exact target.
    targets:
    - id: entrypoint-027-rust-crates-syu-app-ui-src-lib-rs-workbenchview-render
      adapter: rust
      path: crates/syu-app-ui/src/lib.rs
      selector:
        kind: symbol
        name: WorkbenchView::render_html
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.workbench-rendering
  - id: public-api-028
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint addressable by an exact target.
    targets:
    - id: entrypoint-028-rust-crates-syu-app-ui-src-lib-rs-locale-catalog-script
      adapter: rust
      path: crates/syu-app-ui/src/lib.rs
      selector:
        kind: symbol
        name: locale_catalog_script
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.workbench-rendering
  - id: public-readiness
    role: verification
    facet: public
    responsibility: Prove workbench shell rendering entrypoints have bounded canonical
      plans.
    targets:
    - id: canonical-plans
      adapter: rust
      path: crates/syu-validation/src/readiness.rs
      selector:
        kind: symbol
        name: tests::workbench_rendering_public_entrypoints_have_canonical_plans
      claims:
      - kind: verifies
        criterion: REQ-PUBLIC-001#criterion.workbench-rendering
        covers:
        - FEAT-PUBLIC-WORKBENCH-RENDERING-001#binding.public-api-026/target.entrypoint-026-rust-crates-syu-app-ui-src-lib-rs-workbenchview-new
        - FEAT-PUBLIC-WORKBENCH-RENDERING-001#binding.public-api-027/target.entrypoint-027-rust-crates-syu-app-ui-src-lib-rs-workbenchview-render
        - FEAT-PUBLIC-WORKBENCH-RENDERING-001#binding.public-api-028/target.entrypoint-028-rust-crates-syu-app-ui-src-lib-rs-locale-catalog-script
        runner:
          runner: cargo-test
          arguments:
            package: syu-validation
            test: tests::workbench_rendering_public_entrypoints_have_canonical_plans
- id: FEAT-PUBLIC-CODE-DIAGNOSTICS-001
  title: Code diagnostics
  summary: Govern symbol resolution and diagnostic result entrypoints.
  status: implemented
  bindings:
  - id: public-api-029
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint addressable by an exact target.
    targets:
    - id: entrypoint-029-rust-crates-syu-code-intel-src-lib-rs-resolve-symbol
      adapter: rust
      path: crates/syu-code-intel/src/lib.rs
      selector:
        kind: symbol
        name: resolve_symbol
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.code-diagnostics
  - id: public-api-030
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint addressable by an exact target.
    targets:
    - id: entrypoint-030-rust-crates-syu-diagnostics-src-lib-rs-diagnostic-error
      adapter: rust
      path: crates/syu-diagnostics/src/lib.rs
      selector:
        kind: symbol
        name: Diagnostic::error
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.code-diagnostics
  - id: public-api-031
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint addressable by an exact target.
    targets:
    - id: entrypoint-031-rust-crates-syu-diagnostics-src-lib-rs-validationresult
      adapter: rust
      path: crates/syu-diagnostics/src/lib.rs
      selector:
        kind: symbol
        name: ValidationResult::is_valid
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.code-diagnostics
  - id: public-readiness
    role: verification
    facet: public
    responsibility: Prove code diagnostics entrypoints have bounded canonical plans.
    targets:
    - id: canonical-plans
      adapter: rust
      path: crates/syu-validation/src/readiness.rs
      selector:
        kind: symbol
        name: tests::code_diagnostics_public_entrypoints_have_canonical_plans
      claims:
      - kind: verifies
        criterion: REQ-PUBLIC-001#criterion.code-diagnostics
        covers:
        - FEAT-PUBLIC-CODE-DIAGNOSTICS-001#binding.public-api-029/target.entrypoint-029-rust-crates-syu-code-intel-src-lib-rs-resolve-symbol
        - FEAT-PUBLIC-CODE-DIAGNOSTICS-001#binding.public-api-030/target.entrypoint-030-rust-crates-syu-diagnostics-src-lib-rs-diagnostic-error
        - FEAT-PUBLIC-CODE-DIAGNOSTICS-001#binding.public-api-031/target.entrypoint-031-rust-crates-syu-diagnostics-src-lib-rs-validationresult
        runner:
          runner: cargo-test
          arguments:
            package: syu-validation
            test: tests::code_diagnostics_public_entrypoints_have_canonical_plans
- id: FEAT-PUBLIC-INVENTORY-001
  title: Inventory discovery
  summary: Govern artifact discovery and adapter-output combination entrypoints.
  status: implemented
  bindings:
  - id: public-api-032
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint addressable by an exact target.
    targets:
    - id: entrypoint-032-rust-crates-syu-inventory-src-lib-rs-inventoryregistry
      adapter: rust
      path: crates/syu-inventory/src/lib.rs
      selector:
        kind: symbol
        name: InventoryRegistry::discover
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.inventory-discovery
  - id: public-api-033
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint addressable by an exact target.
    targets:
    - id: entrypoint-033-rust-crates-syu-inventory-src-lib-rs-read-bytes
      adapter: rust
      path: crates/syu-inventory/src/lib.rs
      selector:
        kind: symbol
        name: read_bytes
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.inventory-discovery
  - id: public-api-034
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint addressable by an exact target.
    targets:
    - id: entrypoint-034-rust-crates-syu-inventory-src-lib-rs-union
      adapter: rust
      path: crates/syu-inventory/src/lib.rs
      selector:
        kind: symbol
        name: union
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.inventory-discovery
  - id: public-readiness
    role: verification
    facet: public
    responsibility: Prove inventory discovery entrypoints have bounded canonical plans.
    targets:
    - id: canonical-plans
      adapter: rust
      path: crates/syu-validation/src/readiness.rs
      selector:
        kind: symbol
        name: tests::inventory_discovery_public_entrypoints_have_canonical_plans
      claims:
      - kind: verifies
        criterion: REQ-PUBLIC-001#criterion.inventory-discovery
        covers:
        - FEAT-PUBLIC-INVENTORY-001#binding.public-api-032/target.entrypoint-032-rust-crates-syu-inventory-src-lib-rs-inventoryregistry
        - FEAT-PUBLIC-INVENTORY-001#binding.public-api-033/target.entrypoint-033-rust-crates-syu-inventory-src-lib-rs-read-bytes
        - FEAT-PUBLIC-INVENTORY-001#binding.public-api-034/target.entrypoint-034-rust-crates-syu-inventory-src-lib-rs-union
        runner:
          runner: cargo-test
          arguments:
            package: syu-validation
            test: tests::inventory_discovery_public_entrypoints_have_canonical_plans
- id: FEAT-PUBLIC-SPEC-MODEL-001
  title: Specification model
  summary: Govern repository-path, anchor, and schema entrypoints.
  status: implemented
  bindings:
  - id: public-api-035
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint addressable by an exact target.
    targets:
    - id: entrypoint-036-rust-crates-syu-spec-model-src-lib-rs-localanchorkind-l
      adapter: rust
      path: crates/syu-spec-model/src/lib.rs
      selector:
        kind: symbol
        name: LocalAnchorKind::label
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.specification-model
  - id: public-api-036
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint addressable by an exact target.
    targets:
    - id: entrypoint-037-rust-crates-syu-spec-model-src-lib-rs-repopath-as-path
      adapter: rust
      path: crates/syu-spec-model/src/lib.rs
      selector:
        kind: symbol
        name: RepoPath::as_path
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.specification-model
  - id: public-api-037
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint addressable by an exact target.
    targets:
    - id: entrypoint-038-rust-crates-syu-spec-model-src-lib-rs-repopath-display
      adapter: rust
      path: crates/syu-spec-model/src/lib.rs
      selector:
        kind: symbol
        name: RepoPath::display
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.specification-model
  - id: public-api-038
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint addressable by an exact target.
    targets:
    - id: entrypoint-039-rust-crates-syu-spec-model-src-lib-rs-repopath-new
      adapter: rust
      path: crates/syu-spec-model/src/lib.rs
      selector:
        kind: symbol
        name: RepoPath::new
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.specification-model
  - id: public-api-039
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint addressable by an exact target.
    targets:
    - id: entrypoint-040-rust-crates-syu-spec-model-src-lib-rs-repopath-to-strin
      adapter: rust
      path: crates/syu-spec-model/src/lib.rs
      selector:
        kind: symbol
        name: RepoPath::to_string_lossy
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.specification-model
  - id: public-api-040
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint addressable by an exact target.
    targets:
    - id: entrypoint-041-rust-crates-syu-spec-model-src-lib-rs-specdocument-sche
      adapter: rust
      path: crates/syu-spec-model/src/lib.rs
      selector:
        kind: symbol
        name: SpecDocument::schema
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.specification-model
  - id: public-api-repo-path-from-path
    role: implementation
    facet: public
    responsibility: Govern repository path conversion.
    targets:
    - id: entrypoint-repo-path-from-path
      adapter: rust
      path: crates/syu-spec-model/src/lib.rs
      selector:
        kind: symbol
        name: RepoPath::from_path
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.specification-model
  - id: public-readiness
    role: verification
    facet: public
    responsibility: Prove specification model entrypoints have bounded canonical plans.
    targets:
    - id: canonical-plans
      adapter: rust
      path: crates/syu-validation/src/readiness.rs
      selector:
        kind: symbol
        name: tests::specification_model_public_entrypoints_have_canonical_plans
      claims:
      - kind: verifies
        criterion: REQ-PUBLIC-001#criterion.specification-model
        covers:
        - FEAT-PUBLIC-SPEC-MODEL-001#binding.public-api-035/target.entrypoint-036-rust-crates-syu-spec-model-src-lib-rs-localanchorkind-l
        - FEAT-PUBLIC-SPEC-MODEL-001#binding.public-api-036/target.entrypoint-037-rust-crates-syu-spec-model-src-lib-rs-repopath-as-path
        - FEAT-PUBLIC-SPEC-MODEL-001#binding.public-api-037/target.entrypoint-038-rust-crates-syu-spec-model-src-lib-rs-repopath-display
        - FEAT-PUBLIC-SPEC-MODEL-001#binding.public-api-038/target.entrypoint-039-rust-crates-syu-spec-model-src-lib-rs-repopath-new
        - FEAT-PUBLIC-SPEC-MODEL-001#binding.public-api-039/target.entrypoint-040-rust-crates-syu-spec-model-src-lib-rs-repopath-to-strin
        - FEAT-PUBLIC-SPEC-MODEL-001#binding.public-api-040/target.entrypoint-041-rust-crates-syu-spec-model-src-lib-rs-specdocument-sche
        - FEAT-PUBLIC-SPEC-MODEL-001#binding.public-api-repo-path-from-path/target.entrypoint-repo-path-from-path
        runner:
          runner: cargo-test
          arguments:
            package: syu-validation
            test: tests::specification_model_public_entrypoints_have_canonical_plans
- id: FEAT-PUBLIC-VALIDATION-001
  title: Validation engine
  summary: Govern canonical validation, readiness, change, and verification entrypoints.
  status: implemented
  bindings:
  - id: public-api-041
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint addressable by an exact target.
    targets:
    - id: entrypoint-044-rust-crates-syu-validation-src-lib-rs-phase-for-rule
      adapter: rust
      path: crates/syu-validation/src/lib.rs
      selector:
        kind: symbol
        name: phase_for_rule
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.validation-engine
  - id: public-api-042
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint addressable by an exact target.
    targets:
    - id: entrypoint-047-rust-crates-syu-validation-src-lib-rs-validate-without
      adapter: rust
      path: crates/syu-validation/src/lib.rs
      selector:
        kind: symbol
        name: validate_without_readiness
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.validation-engine
  - id: public-api-078
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint addressable by an exact target.
    targets:
    - id: entrypoint-078-rust-crates-syu-validation-src-lib-rs-readinessaxis
      adapter: rust
      path: crates/syu-validation/src/lib.rs
      selector:
        kind: symbol
        name: ReadinessAxis
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.validation-engine
  - id: public-api-079
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint addressable by an exact target.
    targets:
    - id: entrypoint-079-rust-crates-syu-validation-src-lib-rs-readinessaxisid
      adapter: rust
      path: crates/syu-validation/src/lib.rs
      selector:
        kind: symbol
        name: ReadinessAxisId
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.validation-engine
  - id: public-api-080
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint addressable by an exact target.
    targets:
    - id: entrypoint-080-rust-crates-syu-validation-src-lib-rs-readinessreport
      adapter: rust
      path: crates/syu-validation/src/lib.rs
      selector:
        kind: symbol
        name: ReadinessReport
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.validation-engine
  - id: public-api-081
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint addressable by an exact target.
    targets:
    - id: entrypoint-081-rust-crates-syu-validation-src-lib-rs-evaluate-readiness
      adapter: rust
      path: crates/syu-validation/src/lib.rs
      selector:
        kind: symbol
        name: evaluate_readiness
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.validation-engine
  - id: public-api-082
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint addressable by an exact target.
    targets:
    - id: entrypoint-082-rust-crates-syu-validation-src-lib-rs-required-axes
      adapter: rust
      path: crates/syu-validation/src/lib.rs
      selector:
        kind: symbol
        name: required_axes
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.validation-engine
  - id: public-api-changed-files-against-revision
    role: implementation
    facet: public
    responsibility: Govern canonical changed-file collection.
    targets:
    - id: entrypoint-changed-files-against-revision
      adapter: rust
      path: crates/syu-validation/src/lib.rs
      selector:
        kind: symbol
        name: changed_files_against_revision
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.validation-engine
  - id: public-api-execute-verification-attempt
    role: implementation
    facet: public
    responsibility: Govern durable exact verification attempts.
    targets:
    - id: entrypoint-execute-verification-attempt
      adapter: rust
      path: crates/syu-validation/src/lib.rs
      selector:
        kind: symbol
        name: execute_verification_attempt
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.validation-engine
  - id: public-api-validate-workspace
    role: implementation
    facet: public
    responsibility: Govern canonical workspace validation.
    targets:
    - id: entrypoint-validate-workspace
      adapter: rust
      path: crates/syu-validation/src/lib.rs
      selector:
        kind: symbol
        name: validate_workspace
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.validation-engine
  - id: public-readiness
    role: verification
    facet: public
    responsibility: Prove validation engine entrypoints have bounded canonical plans.
    targets:
    - id: canonical-plans
      adapter: rust
      path: crates/syu-validation/src/readiness.rs
      selector:
        kind: symbol
        name: tests::validation_engine_public_entrypoints_have_canonical_plans
      claims:
      - kind: verifies
        criterion: REQ-PUBLIC-001#criterion.validation-engine
        covers:
        - FEAT-PUBLIC-VALIDATION-001#binding.public-api-041/target.entrypoint-044-rust-crates-syu-validation-src-lib-rs-phase-for-rule
        - FEAT-PUBLIC-VALIDATION-001#binding.public-api-042/target.entrypoint-047-rust-crates-syu-validation-src-lib-rs-validate-without
        - FEAT-PUBLIC-VALIDATION-001#binding.public-api-078/target.entrypoint-078-rust-crates-syu-validation-src-lib-rs-readinessaxis
        - FEAT-PUBLIC-VALIDATION-001#binding.public-api-079/target.entrypoint-079-rust-crates-syu-validation-src-lib-rs-readinessaxisid
        - FEAT-PUBLIC-VALIDATION-001#binding.public-api-080/target.entrypoint-080-rust-crates-syu-validation-src-lib-rs-readinessreport
        - FEAT-PUBLIC-VALIDATION-001#binding.public-api-081/target.entrypoint-081-rust-crates-syu-validation-src-lib-rs-evaluate-readiness
        - FEAT-PUBLIC-VALIDATION-001#binding.public-api-082/target.entrypoint-082-rust-crates-syu-validation-src-lib-rs-required-axes
        - FEAT-PUBLIC-VALIDATION-001#binding.public-api-changed-files-against-revision/target.entrypoint-changed-files-against-revision
        - FEAT-PUBLIC-VALIDATION-001#binding.public-api-execute-verification-attempt/target.entrypoint-execute-verification-attempt
        - FEAT-PUBLIC-VALIDATION-001#binding.public-api-validate-workspace/target.entrypoint-validate-workspace
        runner:
          runner: cargo-test
          arguments:
            package: syu-validation
            test: tests::validation_engine_public_entrypoints_have_canonical_plans
- id: FEAT-PUBLIC-WORK-PLANNING-001
  title: Work planning
  summary: Govern requested-target, plan-identity, and split-guidance entrypoints.
  status: implemented
  bindings:
  - id: public-api-043
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint addressable by an exact target.
    targets:
    - id: entrypoint-048-rust-crates-syu-work-model-src-lib-rs-requestedtarget-c
      adapter: rust
      path: crates/syu-work-model/src/lib.rs
      selector:
        kind: symbol
        name: RequestedTarget::criterion
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.work-planning
  - id: public-api-044
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint addressable by an exact target.
    targets:
    - id: entrypoint-049-rust-crates-syu-work-model-src-lib-rs-requestedtarget-r
      adapter: rust
      path: crates/syu-work-model/src/lib.rs
      selector:
        kind: symbol
        name: RequestedTarget::reference
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.work-planning
  - id: public-api-045
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint addressable by an exact target.
    targets:
    - id: entrypoint-050-rust-crates-syu-work-model-src-lib-rs-requestedtarget-t
      adapter: rust
      path: crates/syu-work-model/src/lib.rs
      selector:
        kind: symbol
        name: RequestedTarget::transition
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.work-planning
  - id: public-api-046
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint addressable by an exact target.
    targets:
    - id: entrypoint-051-rust-crates-syu-work-model-src-lib-rs-work-plan-digest
      adapter: rust
      path: crates/syu-work-model/src/lib.rs
      selector:
        kind: symbol
        name: work_plan_digest
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.work-planning
  - id: public-api-split-work-recommendation
    role: implementation
    facet: public
    responsibility: Govern bounded split recommendations.
    targets:
    - id: entrypoint-split-work-recommendation
      adapter: rust
      path: crates/syu-planner/src/lib.rs
      selector:
        kind: symbol
        name: split_work_recommendation
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.work-planning
  - id: public-api-readonly-targets-fingerprint
    role: implementation
    facet: public
    responsibility: Govern readonly target fingerprints.
    targets:
    - id: entrypoint-readonly-targets-fingerprint
      adapter: rust
      path: crates/syu-work-model/src/lib.rs
      selector:
        kind: symbol
        name: readonly_targets_fingerprint
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.work-planning
  - id: public-readiness
    role: verification
    facet: public
    responsibility: Prove work planning entrypoints have bounded canonical plans.
    targets:
    - id: canonical-plans
      adapter: rust
      path: crates/syu-validation/src/readiness.rs
      selector:
        kind: symbol
        name: tests::work_planning_public_entrypoints_have_canonical_plans
      claims:
      - kind: verifies
        criterion: REQ-PUBLIC-001#criterion.work-planning
        covers:
        - FEAT-PUBLIC-WORK-PLANNING-001#binding.public-api-043/target.entrypoint-048-rust-crates-syu-work-model-src-lib-rs-requestedtarget-c
        - FEAT-PUBLIC-WORK-PLANNING-001#binding.public-api-044/target.entrypoint-049-rust-crates-syu-work-model-src-lib-rs-requestedtarget-r
        - FEAT-PUBLIC-WORK-PLANNING-001#binding.public-api-045/target.entrypoint-050-rust-crates-syu-work-model-src-lib-rs-requestedtarget-t
        - FEAT-PUBLIC-WORK-PLANNING-001#binding.public-api-046/target.entrypoint-051-rust-crates-syu-work-model-src-lib-rs-work-plan-digest
        - FEAT-PUBLIC-WORK-PLANNING-001#binding.public-api-split-work-recommendation/target.entrypoint-split-work-recommendation
        - FEAT-PUBLIC-WORK-PLANNING-001#binding.public-api-readonly-targets-fingerprint/target.entrypoint-readonly-targets-fingerprint
        runner:
          runner: cargo-test
          arguments:
            package: syu-validation
            test: tests::work_planning_public_entrypoints_have_canonical_plans
- id: FEAT-PUBLIC-WORKBENCH-SERVER-001
  title: Workbench server lifecycle
  summary: Govern Workbench server construction, projection, routing, and execution.
  status: implemented
  bindings:
  - id: public-api-047
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint addressable by an exact target.
    targets:
    - id: entrypoint-052-rust-crates-syu-workbench-server-src-lib-rs-branchscope
      adapter: rust
      path: crates/syu-workbench-server/src/lib.rs
      selector:
        kind: symbol
        name: BranchScopeView::not_applicable
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.workbench-server-lifecycle
  - id: public-api-048
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint addressable by an exact target.
    targets:
    - id: entrypoint-053-rust-crates-syu-workbench-server-src-lib-rs-validationr
      adapter: rust
      path: crates/syu-workbench-server/src/lib.rs
      selector:
        kind: symbol
        name: ValidationRunView::completed
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.workbench-server-lifecycle
  - id: public-api-049
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint addressable by an exact target.
    targets:
    - id: entrypoint-054-rust-crates-syu-workbench-server-src-lib-rs-validationr
      adapter: rust
      path: crates/syu-workbench-server/src/lib.rs
      selector:
        kind: symbol
        name: ValidationRunView::failed
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.workbench-server-lifecycle
  - id: public-api-050
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint addressable by an exact target.
    targets:
    - id: entrypoint-055-rust-crates-syu-workbench-server-src-lib-rs-validationr
      adapter: rust
      path: crates/syu-workbench-server/src/lib.rs
      selector:
        kind: symbol
        name: ValidationRunView::not_applicable
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.workbench-server-lifecycle
  - id: public-api-051
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint addressable by an exact target.
    targets:
    - id: entrypoint-056-rust-crates-syu-workbench-server-src-lib-rs-validationr
      adapter: rust
      path: crates/syu-workbench-server/src/lib.rs
      selector:
        kind: symbol
        name: ValidationRunView::not_run
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.workbench-server-lifecycle
  - id: public-api-052
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint addressable by an exact target.
    targets:
    - id: entrypoint-057-rust-crates-syu-workbench-server-src-lib-rs-workbenchse
      adapter: rust
      path: crates/syu-workbench-server/src/lib.rs
      selector:
        kind: symbol
        name: WorkbenchServer::new
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.workbench-server-lifecycle
  - id: public-api-053
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint addressable by an exact target.
    targets:
    - id: entrypoint-058-rust-crates-syu-workbench-server-src-lib-rs-workbenchse
      adapter: rust
      path: crates/syu-workbench-server/src/lib.rs
      selector:
        kind: symbol
        name: WorkbenchServer::projection
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.workbench-server-lifecycle
  - id: public-api-054
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint addressable by an exact target.
    targets:
    - id: entrypoint-059-rust-crates-syu-workbench-server-src-lib-rs-workbenchse
      adapter: rust
      path: crates/syu-workbench-server/src/lib.rs
      selector:
        kind: symbol
        name: WorkbenchServer::router
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.workbench-server-lifecycle
  - id: public-api-055
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint addressable by an exact target.
    targets:
    - id: entrypoint-060-rust-crates-syu-workbench-server-src-lib-rs-workbenchse
      adapter: rust
      path: crates/syu-workbench-server/src/lib.rs
      selector:
        kind: symbol
        name: WorkbenchServer::run
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.workbench-server-lifecycle
  - id: public-api-056
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint addressable by an exact target.
    targets:
    - id: entrypoint-061-rust-crates-syu-workbench-server-src-lib-rs-workbenchse
      adapter: rust
      path: crates/syu-workbench-server/src/lib.rs
      selector:
        kind: symbol
        name: WorkbenchServer::with_launch
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.workbench-server-lifecycle
  - id: public-api-057
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint addressable by an exact target.
    targets:
    - id: entrypoint-062-rust-crates-syu-workbench-server-src-lib-rs-workbenchse
      adapter: rust
      path: crates/syu-workbench-server/src/lib.rs
      selector:
        kind: symbol
        name: WorkbenchServer::with_request
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.workbench-server-lifecycle
  - id: public-readiness
    role: verification
    facet: public
    responsibility: Prove workbench server lifecycle entrypoints have bounded canonical
      plans.
    targets:
    - id: canonical-plans
      adapter: rust
      path: crates/syu-validation/src/readiness.rs
      selector:
        kind: symbol
        name: tests::workbench_server_lifecycle_public_entrypoints_have_canonical_plans
      claims:
      - kind: verifies
        criterion: REQ-PUBLIC-001#criterion.workbench-server-lifecycle
        covers:
        - FEAT-PUBLIC-WORKBENCH-SERVER-001#binding.public-api-047/target.entrypoint-052-rust-crates-syu-workbench-server-src-lib-rs-branchscope
        - FEAT-PUBLIC-WORKBENCH-SERVER-001#binding.public-api-048/target.entrypoint-053-rust-crates-syu-workbench-server-src-lib-rs-validationr
        - FEAT-PUBLIC-WORKBENCH-SERVER-001#binding.public-api-049/target.entrypoint-054-rust-crates-syu-workbench-server-src-lib-rs-validationr
        - FEAT-PUBLIC-WORKBENCH-SERVER-001#binding.public-api-050/target.entrypoint-055-rust-crates-syu-workbench-server-src-lib-rs-validationr
        - FEAT-PUBLIC-WORKBENCH-SERVER-001#binding.public-api-051/target.entrypoint-056-rust-crates-syu-workbench-server-src-lib-rs-validationr
        - FEAT-PUBLIC-WORKBENCH-SERVER-001#binding.public-api-052/target.entrypoint-057-rust-crates-syu-workbench-server-src-lib-rs-workbenchse
        - FEAT-PUBLIC-WORKBENCH-SERVER-001#binding.public-api-053/target.entrypoint-058-rust-crates-syu-workbench-server-src-lib-rs-workbenchse
        - FEAT-PUBLIC-WORKBENCH-SERVER-001#binding.public-api-054/target.entrypoint-059-rust-crates-syu-workbench-server-src-lib-rs-workbenchse
        - FEAT-PUBLIC-WORKBENCH-SERVER-001#binding.public-api-055/target.entrypoint-060-rust-crates-syu-workbench-server-src-lib-rs-workbenchse
        - FEAT-PUBLIC-WORKBENCH-SERVER-001#binding.public-api-056/target.entrypoint-061-rust-crates-syu-workbench-server-src-lib-rs-workbenchse
        - FEAT-PUBLIC-WORKBENCH-SERVER-001#binding.public-api-057/target.entrypoint-062-rust-crates-syu-workbench-server-src-lib-rs-workbenchse
        runner:
          runner: cargo-test
          arguments:
            package: syu-validation
            test: tests::workbench_server_lifecycle_public_entrypoints_have_canonical_plans
- id: FEAT-PUBLIC-WORKBENCH-VALIDATION-001
  title: Workbench server validation
  summary: Govern branch-scope and verification-receipt entrypoints.
  status: implemented
  bindings:
  - id: public-api-058
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint addressable by an exact target.
    targets:
    - id: entrypoint-063-rust-crates-syu-workbench-server-src-lib-rs-branch-scop
      adapter: rust
      path: crates/syu-workbench-server/src/lib.rs
      selector:
        kind: symbol
        name: branch_scope_view
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.workbench-server-validation
  - id: public-api-059
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint addressable by an exact target.
    targets:
    - id: entrypoint-064-rust-crates-syu-workbench-server-src-lib-rs-execute-ver
      adapter: rust
      path: crates/syu-workbench-server/src/lib.rs
      selector:
        kind: symbol
        name: execute_verification
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.workbench-server-validation
  - id: public-api-060
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint addressable by an exact target.
    targets:
    - id: entrypoint-065-rust-crates-syu-workbench-server-src-lib-rs-validate-ve
      adapter: rust
      path: crates/syu-workbench-server/src/lib.rs
      selector:
        kind: symbol
        name: validate_verification_receipt
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.workbench-server-validation
  - id: public-readiness
    role: verification
    facet: public
    responsibility: Prove workbench server validation entrypoints have bounded canonical
      plans.
    targets:
    - id: canonical-plans
      adapter: rust
      path: crates/syu-validation/src/readiness.rs
      selector:
        kind: symbol
        name: tests::workbench_server_validation_public_entrypoints_have_canonical_plans
      claims:
      - kind: verifies
        criterion: REQ-PUBLIC-001#criterion.workbench-server-validation
        covers:
        - FEAT-PUBLIC-WORKBENCH-VALIDATION-001#binding.public-api-058/target.entrypoint-063-rust-crates-syu-workbench-server-src-lib-rs-branch-scop
        - FEAT-PUBLIC-WORKBENCH-VALIDATION-001#binding.public-api-059/target.entrypoint-064-rust-crates-syu-workbench-server-src-lib-rs-execute-ver
        - FEAT-PUBLIC-WORKBENCH-VALIDATION-001#binding.public-api-060/target.entrypoint-065-rust-crates-syu-workbench-server-src-lib-rs-validate-ve
        runner:
          runner: cargo-test
          arguments:
            package: syu-validation
            test: tests::workbench_server_validation_public_entrypoints_have_canonical_plans
- id: FEAT-PUBLIC-WORKSPACE-LOADING-001
  title: Workspace loading
  summary: Govern workspace loading, overlays, index access, and content reads.
  status: implemented
  bindings:
  - id: public-api-061
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint addressable by an exact target.
    targets:
    - id: entrypoint-066-rust-crates-syu-workspace-src-lib-rs-specindex-anchor
      adapter: rust
      path: crates/syu-workspace/src/lib.rs
      selector:
        kind: symbol
        name: SpecIndex::anchor
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.workspace-loading
  - id: public-api-062
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint addressable by an exact target.
    targets:
    - id: entrypoint-067-rust-crates-syu-workspace-src-lib-rs-specindex-build
      adapter: rust
      path: crates/syu-workspace/src/lib.rs
      selector:
        kind: symbol
        name: SpecIndex::build
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.workspace-loading
  - id: public-api-063
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint addressable by an exact target.
    targets:
    - id: entrypoint-068-rust-crates-syu-workspace-src-lib-rs-specindex-target
      adapter: rust
      path: crates/syu-workspace/src/lib.rs
      selector:
        kind: symbol
        name: SpecIndex::target
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.workspace-loading
  - id: public-api-064
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint addressable by an exact target.
    targets:
    - id: entrypoint-069-rust-crates-syu-workspace-src-lib-rs-specworkspace-fing
      adapter: rust
      path: crates/syu-workspace/src/lib.rs
      selector:
        kind: symbol
        name: SpecWorkspace::fingerprint
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.workspace-loading
  - id: public-api-065
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint addressable by an exact target.
    targets:
    - id: entrypoint-070-rust-crates-syu-workspace-src-lib-rs-specworkspace-inde
      adapter: rust
      path: crates/syu-workspace/src/lib.rs
      selector:
        kind: symbol
        name: SpecWorkspace::index
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.workspace-loading
  - id: public-api-066
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint addressable by an exact target.
    targets:
    - id: entrypoint-071-rust-crates-syu-workspace-src-lib-rs-specworkspace-load
      adapter: rust
      path: crates/syu-workspace/src/lib.rs
      selector:
        kind: symbol
        name: SpecWorkspace::load
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.workspace-loading
  - id: public-api-067
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint addressable by an exact target.
    targets:
    - id: entrypoint-072-rust-crates-syu-workspace-src-lib-rs-specworkspace-over
      adapter: rust
      path: crates/syu-workspace/src/lib.rs
      selector:
        kind: symbol
        name: SpecWorkspace::overlay_config
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.workspace-loading
  - id: public-api-068
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint addressable by an exact target.
    targets:
    - id: entrypoint-073-rust-crates-syu-workspace-src-lib-rs-specworkspace-over
      adapter: rust
      path: crates/syu-workspace/src/lib.rs
      selector:
        kind: symbol
        name: SpecWorkspace::overlay_document
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.workspace-loading
  - id: public-api-072
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint addressable by an exact target.
    targets:
    - id: entrypoint-077-rust-crates-syu-workspace-src-lib-rs-specworkspace-read
      adapter: rust
      path: crates/syu-workspace/src/lib.rs
      selector:
        kind: symbol
        name: SpecWorkspace::read_bytes
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.workspace-loading
  - id: public-api-073
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint addressable by an exact target.
    targets:
    - id: entrypoint-078-rust-crates-syu-workspace-src-lib-rs-specworkspace-read
      adapter: rust
      path: crates/syu-workspace/src/lib.rs
      selector:
        kind: symbol
        name: SpecWorkspace::read_to_string
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.workspace-loading
  - id: public-readiness
    role: verification
    facet: public
    responsibility: Prove workspace loading entrypoints have bounded canonical plans.
    targets:
    - id: canonical-plans
      adapter: rust
      path: crates/syu-validation/src/readiness.rs
      selector:
        kind: symbol
        name: tests::workspace_loading_public_entrypoints_have_canonical_plans
      claims:
      - kind: verifies
        criterion: REQ-PUBLIC-001#criterion.workspace-loading
        covers:
        - FEAT-PUBLIC-WORKSPACE-LOADING-001#binding.public-api-061/target.entrypoint-066-rust-crates-syu-workspace-src-lib-rs-specindex-anchor
        - FEAT-PUBLIC-WORKSPACE-LOADING-001#binding.public-api-062/target.entrypoint-067-rust-crates-syu-workspace-src-lib-rs-specindex-build
        - FEAT-PUBLIC-WORKSPACE-LOADING-001#binding.public-api-063/target.entrypoint-068-rust-crates-syu-workspace-src-lib-rs-specindex-target
        - FEAT-PUBLIC-WORKSPACE-LOADING-001#binding.public-api-064/target.entrypoint-069-rust-crates-syu-workspace-src-lib-rs-specworkspace-fing
        - FEAT-PUBLIC-WORKSPACE-LOADING-001#binding.public-api-065/target.entrypoint-070-rust-crates-syu-workspace-src-lib-rs-specworkspace-inde
        - FEAT-PUBLIC-WORKSPACE-LOADING-001#binding.public-api-066/target.entrypoint-071-rust-crates-syu-workspace-src-lib-rs-specworkspace-load
        - FEAT-PUBLIC-WORKSPACE-LOADING-001#binding.public-api-067/target.entrypoint-072-rust-crates-syu-workspace-src-lib-rs-specworkspace-over
        - FEAT-PUBLIC-WORKSPACE-LOADING-001#binding.public-api-068/target.entrypoint-073-rust-crates-syu-workspace-src-lib-rs-specworkspace-over
        - FEAT-PUBLIC-WORKSPACE-LOADING-001#binding.public-api-072/target.entrypoint-077-rust-crates-syu-workspace-src-lib-rs-specworkspace-read
        - FEAT-PUBLIC-WORKSPACE-LOADING-001#binding.public-api-073/target.entrypoint-078-rust-crates-syu-workspace-src-lib-rs-specworkspace-read
        runner:
          runner: cargo-test
          arguments:
            package: syu-validation
            test: tests::workspace_loading_public_entrypoints_have_canonical_plans
- id: FEAT-PUBLIC-WORKSPACE-RESOLUTION-001
  title: Workspace resolution
  summary: Govern selector resolution and stable workspace fingerprints.
  status: implemented
  bindings:
  - id: public-api-069
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint addressable by an exact target.
    targets:
    - id: entrypoint-074-rust-crates-syu-workspace-src-lib-rs-specworkspace-path
      adapter: rust
      path: crates/syu-workspace/src/lib.rs
      selector:
        kind: symbol
        name: SpecWorkspace::path_is_artifact
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.workspace-resolution
  - id: public-api-070
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint addressable by an exact target.
    targets:
    - id: entrypoint-075-rust-crates-syu-workspace-src-lib-rs-specworkspace-path
      adapter: rust
      path: crates/syu-workspace/src/lib.rs
      selector:
        kind: symbol
        name: SpecWorkspace::path_is_excluded
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.workspace-resolution
  - id: public-api-071
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint addressable by an exact target.
    targets:
    - id: entrypoint-076-rust-crates-syu-workspace-src-lib-rs-specworkspace-path
      adapter: rust
      path: crates/syu-workspace/src/lib.rs
      selector:
        kind: symbol
        name: SpecWorkspace::path_is_spec
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.workspace-resolution
  - id: public-api-074
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint addressable by an exact target.
    targets:
    - id: entrypoint-079-rust-crates-syu-workspace-src-lib-rs-specworkspace-try
      adapter: rust
      path: crates/syu-workspace/src/lib.rs
      selector:
        kind: symbol
        name: SpecWorkspace::try_fingerprint
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.workspace-resolution
  - id: public-api-075
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint addressable by an exact target.
    targets:
    - id: entrypoint-080-rust-crates-syu-workspace-src-lib-rs-resolve-target
      adapter: rust
      path: crates/syu-workspace/src/lib.rs
      selector:
        kind: symbol
        name: resolve_target
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.workspace-resolution
  - id: public-api-076
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint addressable by an exact target.
    targets:
    - id: entrypoint-082-rust-crates-syu-workspace-src-lib-rs-resolve-target-wit
      adapter: rust
      path: crates/syu-workspace/src/lib.rs
      selector:
        kind: symbol
        name: resolve_target_with_adapters
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.workspace-resolution
  - id: public-api-077
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint addressable by an exact target.
    targets:
    - id: entrypoint-083-rust-crates-syu-workspace-src-lib-rs-selector-supports
      adapter: rust
      path: crates/syu-workspace/src/lib.rs
      selector:
        kind: symbol
        name: selector_supports_editable
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.workspace-resolution
  - id: public-api-resolve-indexed-target
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint addressable by an exact target.
    targets:
    - id: entrypoint-resolve-indexed-target
      adapter: rust
      path: crates/syu-workspace/src/lib.rs
      selector:
        kind: symbol
        name: resolve_indexed_target
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.workspace-resolution
  - id: public-api-ownership-fingerprint
    role: implementation
    facet: public
    responsibility: Govern ownership graph fingerprints.
    targets:
    - id: entrypoint-ownership-fingerprint
      adapter: rust
      path: crates/syu-workspace/src/lib.rs
      selector:
        kind: symbol
        name: ownership_fingerprint
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.workspace-resolution
  - id: public-api-spec-fingerprint
    role: implementation
    facet: public
    responsibility: Govern specification graph fingerprints.
    targets:
    - id: entrypoint-spec-fingerprint
      adapter: rust
      path: crates/syu-workspace/src/lib.rs
      selector:
        kind: symbol
        name: spec_fingerprint
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.workspace-resolution
  - id: public-readiness
    role: verification
    facet: public
    responsibility: Prove workspace resolution entrypoints have bounded canonical
      plans.
    targets:
    - id: canonical-plans
      adapter: rust
      path: crates/syu-validation/src/readiness.rs
      selector:
        kind: symbol
        name: tests::workspace_resolution_public_entrypoints_have_canonical_plans
      claims:
      - kind: verifies
        criterion: REQ-PUBLIC-001#criterion.workspace-resolution
        covers:
        - FEAT-PUBLIC-WORKSPACE-RESOLUTION-001#binding.public-api-069/target.entrypoint-074-rust-crates-syu-workspace-src-lib-rs-specworkspace-path
        - FEAT-PUBLIC-WORKSPACE-RESOLUTION-001#binding.public-api-070/target.entrypoint-075-rust-crates-syu-workspace-src-lib-rs-specworkspace-path
        - FEAT-PUBLIC-WORKSPACE-RESOLUTION-001#binding.public-api-071/target.entrypoint-076-rust-crates-syu-workspace-src-lib-rs-specworkspace-path
        - FEAT-PUBLIC-WORKSPACE-RESOLUTION-001#binding.public-api-074/target.entrypoint-079-rust-crates-syu-workspace-src-lib-rs-specworkspace-try
        - FEAT-PUBLIC-WORKSPACE-RESOLUTION-001#binding.public-api-075/target.entrypoint-080-rust-crates-syu-workspace-src-lib-rs-resolve-target
        - FEAT-PUBLIC-WORKSPACE-RESOLUTION-001#binding.public-api-076/target.entrypoint-082-rust-crates-syu-workspace-src-lib-rs-resolve-target-wit
        - FEAT-PUBLIC-WORKSPACE-RESOLUTION-001#binding.public-api-077/target.entrypoint-083-rust-crates-syu-workspace-src-lib-rs-selector-supports
        - FEAT-PUBLIC-WORKSPACE-RESOLUTION-001#binding.public-api-resolve-indexed-target/target.entrypoint-resolve-indexed-target
        - FEAT-PUBLIC-WORKSPACE-RESOLUTION-001#binding.public-api-ownership-fingerprint/target.entrypoint-ownership-fingerprint
        - FEAT-PUBLIC-WORKSPACE-RESOLUTION-001#binding.public-api-spec-fingerprint/target.entrypoint-spec-fingerprint
        runner:
          runner: cargo-test
          arguments:
            package: syu-validation
            test: tests::workspace_resolution_public_entrypoints_have_canonical_plans
- id: FEAT-PUBLIC-AGENT-DELIVERY-001
  title: Agent delivery evidence
  summary: Govern scoped-agent and durable delivery evidence entrypoints.
  status: implemented
  bindings:
  - id: public-api-agent-current-run
    role: implementation
    facet: public
    responsibility: Govern current scoped-agent run reads.
    targets:
    - id: entrypoint-agent-current-run
      adapter: rust
      path: crates/syu-agent/src/lib.rs
      selector:
        kind: symbol
        name: current_run
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.agent-delivery
  - id: public-api-agent-events
    role: implementation
    facet: public
    responsibility: Govern scoped-agent event reads.
    targets:
    - id: entrypoint-agent-events
      adapter: rust
      path: crates/syu-agent/src/lib.rs
      selector:
        kind: symbol
        name: events
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.agent-delivery
  - id: public-api-agent-record-blocker
    role: implementation
    facet: public
    responsibility: Govern scoped-agent blocker evidence.
    targets:
    - id: entrypoint-agent-record-blocker
      adapter: rust
      path: crates/syu-agent/src/lib.rs
      selector:
        kind: symbol
        name: record_blocker
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.agent-delivery
  - id: public-api-agent-record-verification
    role: implementation
    facet: public
    responsibility: Govern scoped-agent verification evidence.
    targets:
    - id: entrypoint-agent-record-verification
      adapter: rust
      path: crates/syu-agent/src/lib.rs
      selector:
        kind: symbol
        name: record_verification
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.agent-delivery
  - id: public-api-delivery-agent-events
    role: implementation
    facet: public
    responsibility: Govern durable agent event reads.
    targets:
    - id: entrypoint-delivery-agent-events
      adapter: rust
      path: crates/syu-delivery/src/lib.rs
      selector:
        kind: symbol
        name: agent_events
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.agent-delivery
  - id: public-api-delivery-agent-run
    role: implementation
    facet: public
    responsibility: Govern durable agent run reads.
    targets:
    - id: entrypoint-delivery-agent-run
      adapter: rust
      path: crates/syu-delivery/src/lib.rs
      selector:
        kind: symbol
        name: agent_run
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.agent-delivery
  - id: public-api-delivery-latest-agent-run
    role: implementation
    facet: public
    responsibility: Govern latest durable agent run reads.
    targets:
    - id: entrypoint-delivery-latest-agent-run
      adapter: rust
      path: crates/syu-delivery/src/lib.rs
      selector:
        kind: symbol
        name: latest_agent_run
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.agent-delivery
  - id: public-readiness
    role: verification
    facet: public
    responsibility: Prove agent delivery evidence entrypoints have bounded canonical
      plans.
    targets:
    - id: canonical-plans
      adapter: rust
      path: crates/syu-validation/src/readiness.rs
      selector:
        kind: symbol
        name: tests::agent_delivery_public_entrypoints_have_canonical_plans
      claims:
      - kind: verifies
        criterion: REQ-PUBLIC-001#criterion.agent-delivery
        covers:
        - FEAT-PUBLIC-AGENT-DELIVERY-001#binding.public-api-agent-current-run/target.entrypoint-agent-current-run
        - FEAT-PUBLIC-AGENT-DELIVERY-001#binding.public-api-agent-events/target.entrypoint-agent-events
        - FEAT-PUBLIC-AGENT-DELIVERY-001#binding.public-api-agent-record-blocker/target.entrypoint-agent-record-blocker
        - FEAT-PUBLIC-AGENT-DELIVERY-001#binding.public-api-agent-record-verification/target.entrypoint-agent-record-verification
        - FEAT-PUBLIC-AGENT-DELIVERY-001#binding.public-api-delivery-agent-events/target.entrypoint-delivery-agent-events
        - FEAT-PUBLIC-AGENT-DELIVERY-001#binding.public-api-delivery-agent-run/target.entrypoint-delivery-agent-run
        - FEAT-PUBLIC-AGENT-DELIVERY-001#binding.public-api-delivery-latest-agent-run/target.entrypoint-delivery-latest-agent-run
        runner:
          runner: cargo-test
          arguments:
            package: syu-validation
            test: tests::agent_delivery_public_entrypoints_have_canonical_plans
```
