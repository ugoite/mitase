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

- **id**: FEAT-PUBLIC-001
  - **title**: Public entrypoint contracts
  - **summary**: Keep discovered public entrypoints addressable by exact targets.
  - **status**: planned
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
              - **criterion**: REQ-PUBLIC-001#criterion.entrypoint
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
              - **criterion**: REQ-PUBLIC-001#criterion.entrypoint
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
              - **criterion**: REQ-PUBLIC-001#criterion.entrypoint
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
              - **criterion**: REQ-PUBLIC-001#criterion.entrypoint
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
              - **criterion**: REQ-PUBLIC-001#criterion.entrypoint
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
              - **criterion**: REQ-PUBLIC-001#criterion.entrypoint
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
              - **criterion**: REQ-PUBLIC-001#criterion.entrypoint
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
              - **criterion**: REQ-PUBLIC-001#criterion.entrypoint
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
              - **criterion**: REQ-PUBLIC-001#criterion.entrypoint
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
              - **criterion**: REQ-PUBLIC-001#criterion.entrypoint
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
              - **criterion**: REQ-PUBLIC-001#criterion.entrypoint
    - **id**: public-api-012
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint addressable by an exact target.
      - **targets**:
        - **id**: entrypoint-012-javascript-crates-syu-app-ui-assets-js-main-js-startwor
          - **adapter**: javascript
          - **path**: crates/syu-app-ui/assets/js/main.js
          - **selector**:
            - **kind**: symbol
            - **name**: startWorkbench
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.entrypoint
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
              - **criterion**: REQ-PUBLIC-001#criterion.entrypoint
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
              - **criterion**: REQ-PUBLIC-001#criterion.entrypoint
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
              - **criterion**: REQ-PUBLIC-001#criterion.entrypoint
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
              - **criterion**: REQ-PUBLIC-001#criterion.entrypoint
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
              - **criterion**: REQ-PUBLIC-001#criterion.entrypoint
    - **id**: public-api-018
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint addressable by an exact target.
      - **targets**:
        - **id**: entrypoint-018-javascript-crates-syu-app-ui-assets-js-pages-work-js-re
          - **adapter**: javascript
          - **path**: crates/syu-app-ui/assets/js/pages/work.js
          - **selector**:
            - **kind**: symbol
            - **name**: renderWork
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.entrypoint
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
              - **criterion**: REQ-PUBLIC-001#criterion.entrypoint
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
              - **criterion**: REQ-PUBLIC-001#criterion.entrypoint
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
              - **criterion**: REQ-PUBLIC-001#criterion.entrypoint
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
              - **criterion**: REQ-PUBLIC-001#criterion.entrypoint
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
              - **criterion**: REQ-PUBLIC-001#criterion.entrypoint
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
              - **criterion**: REQ-PUBLIC-001#criterion.entrypoint
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
              - **criterion**: REQ-PUBLIC-001#criterion.entrypoint
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
              - **criterion**: REQ-PUBLIC-001#criterion.entrypoint
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
              - **criterion**: REQ-PUBLIC-001#criterion.entrypoint
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
              - **criterion**: REQ-PUBLIC-001#criterion.entrypoint
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
              - **criterion**: REQ-PUBLIC-001#criterion.entrypoint
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
              - **criterion**: REQ-PUBLIC-001#criterion.entrypoint
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
              - **criterion**: REQ-PUBLIC-001#criterion.entrypoint
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
              - **criterion**: REQ-PUBLIC-001#criterion.entrypoint
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
              - **criterion**: REQ-PUBLIC-001#criterion.entrypoint
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
              - **criterion**: REQ-PUBLIC-001#criterion.entrypoint
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
              - **criterion**: REQ-PUBLIC-001#criterion.entrypoint
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
              - **criterion**: REQ-PUBLIC-001#criterion.entrypoint
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
              - **criterion**: REQ-PUBLIC-001#criterion.entrypoint
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
              - **criterion**: REQ-PUBLIC-001#criterion.entrypoint
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
              - **criterion**: REQ-PUBLIC-001#criterion.entrypoint
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
              - **criterion**: REQ-PUBLIC-001#criterion.entrypoint
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
              - **criterion**: REQ-PUBLIC-001#criterion.entrypoint
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
              - **criterion**: REQ-PUBLIC-001#criterion.entrypoint
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
              - **criterion**: REQ-PUBLIC-001#criterion.entrypoint
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
              - **criterion**: REQ-PUBLIC-001#criterion.entrypoint
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
              - **criterion**: REQ-PUBLIC-001#criterion.entrypoint
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
              - **criterion**: REQ-PUBLIC-001#criterion.entrypoint
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
              - **criterion**: REQ-PUBLIC-001#criterion.entrypoint
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
              - **criterion**: REQ-PUBLIC-001#criterion.entrypoint
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
              - **criterion**: REQ-PUBLIC-001#criterion.entrypoint
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
              - **criterion**: REQ-PUBLIC-001#criterion.entrypoint
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
              - **criterion**: REQ-PUBLIC-001#criterion.entrypoint
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
              - **criterion**: REQ-PUBLIC-001#criterion.entrypoint
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
              - **criterion**: REQ-PUBLIC-001#criterion.entrypoint
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
              - **criterion**: REQ-PUBLIC-001#criterion.entrypoint
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
              - **criterion**: REQ-PUBLIC-001#criterion.entrypoint
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
              - **criterion**: REQ-PUBLIC-001#criterion.entrypoint
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
              - **criterion**: REQ-PUBLIC-001#criterion.entrypoint
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
              - **criterion**: REQ-PUBLIC-001#criterion.entrypoint
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
              - **criterion**: REQ-PUBLIC-001#criterion.entrypoint
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
              - **criterion**: REQ-PUBLIC-001#criterion.entrypoint
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
              - **criterion**: REQ-PUBLIC-001#criterion.entrypoint
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
              - **criterion**: REQ-PUBLIC-001#criterion.entrypoint
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
              - **criterion**: REQ-PUBLIC-001#criterion.entrypoint
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
              - **criterion**: REQ-PUBLIC-001#criterion.entrypoint
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
              - **criterion**: REQ-PUBLIC-001#criterion.entrypoint
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
              - **criterion**: REQ-PUBLIC-001#criterion.entrypoint
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
              - **criterion**: REQ-PUBLIC-001#criterion.entrypoint
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
              - **criterion**: REQ-PUBLIC-001#criterion.entrypoint
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
              - **criterion**: REQ-PUBLIC-001#criterion.entrypoint
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
              - **criterion**: REQ-PUBLIC-001#criterion.entrypoint
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
              - **criterion**: REQ-PUBLIC-001#criterion.entrypoint
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
              - **criterion**: REQ-PUBLIC-001#criterion.entrypoint
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
              - **criterion**: REQ-PUBLIC-001#criterion.entrypoint
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
              - **criterion**: REQ-PUBLIC-001#criterion.entrypoint
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
              - **criterion**: REQ-PUBLIC-001#criterion.entrypoint
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
              - **criterion**: REQ-PUBLIC-001#criterion.entrypoint
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
              - **criterion**: REQ-PUBLIC-001#criterion.entrypoint
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
              - **criterion**: REQ-PUBLIC-001#criterion.entrypoint
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
              - **criterion**: REQ-PUBLIC-001#criterion.entrypoint
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
              - **criterion**: REQ-PUBLIC-001#criterion.entrypoint
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
              - **criterion**: REQ-PUBLIC-001#criterion.entrypoint
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
              - **criterion**: REQ-PUBLIC-001#criterion.entrypoint
    - **id**: public-api-083
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint addressable by an exact target.
      - **targets**:
        - **id**: entrypoint-083-rust-crates-syu-validation-src-lib-rs-evaluate-com
          - **adapter**: rust
          - **path**: crates/syu-validation/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: evaluate_completion
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.entrypoint
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
              - **criterion**: REQ-PUBLIC-001#criterion.entrypoint

## Source YAML

```yaml
schema: syu/spec/v1
kind: features
namespace: public
category: Public entrypoint contracts
features:
- id: FEAT-PUBLIC-001
  title: Public entrypoint contracts
  summary: Keep discovered public entrypoints addressable by exact targets.
  status: planned
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
        criterion: REQ-PUBLIC-001#criterion.entrypoint
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
        criterion: REQ-PUBLIC-001#criterion.entrypoint
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
        criterion: REQ-PUBLIC-001#criterion.entrypoint
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
        criterion: REQ-PUBLIC-001#criterion.entrypoint
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
        criterion: REQ-PUBLIC-001#criterion.entrypoint
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
        criterion: REQ-PUBLIC-001#criterion.entrypoint
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
        criterion: REQ-PUBLIC-001#criterion.entrypoint
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
        criterion: REQ-PUBLIC-001#criterion.entrypoint
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
        criterion: REQ-PUBLIC-001#criterion.entrypoint
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
        criterion: REQ-PUBLIC-001#criterion.entrypoint
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
        criterion: REQ-PUBLIC-001#criterion.entrypoint
  - id: public-api-012
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint addressable by an exact target.
    targets:
    - id: entrypoint-012-javascript-crates-syu-app-ui-assets-js-main-js-startwor
      adapter: javascript
      path: crates/syu-app-ui/assets/js/main.js
      selector:
        kind: symbol
        name: startWorkbench
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.entrypoint
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
        criterion: REQ-PUBLIC-001#criterion.entrypoint
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
        criterion: REQ-PUBLIC-001#criterion.entrypoint
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
        criterion: REQ-PUBLIC-001#criterion.entrypoint
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
        criterion: REQ-PUBLIC-001#criterion.entrypoint
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
        criterion: REQ-PUBLIC-001#criterion.entrypoint
  - id: public-api-018
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint addressable by an exact target.
    targets:
    - id: entrypoint-018-javascript-crates-syu-app-ui-assets-js-pages-work-js-re
      adapter: javascript
      path: crates/syu-app-ui/assets/js/pages/work.js
      selector:
        kind: symbol
        name: renderWork
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.entrypoint
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
        criterion: REQ-PUBLIC-001#criterion.entrypoint
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
        criterion: REQ-PUBLIC-001#criterion.entrypoint
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
        criterion: REQ-PUBLIC-001#criterion.entrypoint
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
        criterion: REQ-PUBLIC-001#criterion.entrypoint
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
        criterion: REQ-PUBLIC-001#criterion.entrypoint
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
        criterion: REQ-PUBLIC-001#criterion.entrypoint
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
        criterion: REQ-PUBLIC-001#criterion.entrypoint
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
        criterion: REQ-PUBLIC-001#criterion.entrypoint
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
        criterion: REQ-PUBLIC-001#criterion.entrypoint
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
        criterion: REQ-PUBLIC-001#criterion.entrypoint
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
        criterion: REQ-PUBLIC-001#criterion.entrypoint
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
        criterion: REQ-PUBLIC-001#criterion.entrypoint
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
        criterion: REQ-PUBLIC-001#criterion.entrypoint
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
        criterion: REQ-PUBLIC-001#criterion.entrypoint
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
        criterion: REQ-PUBLIC-001#criterion.entrypoint
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
        criterion: REQ-PUBLIC-001#criterion.entrypoint
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
        criterion: REQ-PUBLIC-001#criterion.entrypoint
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
        criterion: REQ-PUBLIC-001#criterion.entrypoint
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
        criterion: REQ-PUBLIC-001#criterion.entrypoint
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
        criterion: REQ-PUBLIC-001#criterion.entrypoint
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
        criterion: REQ-PUBLIC-001#criterion.entrypoint
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
        criterion: REQ-PUBLIC-001#criterion.entrypoint
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
        criterion: REQ-PUBLIC-001#criterion.entrypoint
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
        criterion: REQ-PUBLIC-001#criterion.entrypoint
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
        criterion: REQ-PUBLIC-001#criterion.entrypoint
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
        criterion: REQ-PUBLIC-001#criterion.entrypoint
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
        criterion: REQ-PUBLIC-001#criterion.entrypoint
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
        criterion: REQ-PUBLIC-001#criterion.entrypoint
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
        criterion: REQ-PUBLIC-001#criterion.entrypoint
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
        criterion: REQ-PUBLIC-001#criterion.entrypoint
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
        criterion: REQ-PUBLIC-001#criterion.entrypoint
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
        criterion: REQ-PUBLIC-001#criterion.entrypoint
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
        criterion: REQ-PUBLIC-001#criterion.entrypoint
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
        criterion: REQ-PUBLIC-001#criterion.entrypoint
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
        criterion: REQ-PUBLIC-001#criterion.entrypoint
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
        criterion: REQ-PUBLIC-001#criterion.entrypoint
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
        criterion: REQ-PUBLIC-001#criterion.entrypoint
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
        criterion: REQ-PUBLIC-001#criterion.entrypoint
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
        criterion: REQ-PUBLIC-001#criterion.entrypoint
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
        criterion: REQ-PUBLIC-001#criterion.entrypoint
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
        criterion: REQ-PUBLIC-001#criterion.entrypoint
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
        criterion: REQ-PUBLIC-001#criterion.entrypoint
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
        criterion: REQ-PUBLIC-001#criterion.entrypoint
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
        criterion: REQ-PUBLIC-001#criterion.entrypoint
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
        criterion: REQ-PUBLIC-001#criterion.entrypoint
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
        criterion: REQ-PUBLIC-001#criterion.entrypoint
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
        criterion: REQ-PUBLIC-001#criterion.entrypoint
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
        criterion: REQ-PUBLIC-001#criterion.entrypoint
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
        criterion: REQ-PUBLIC-001#criterion.entrypoint
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
        criterion: REQ-PUBLIC-001#criterion.entrypoint
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
        criterion: REQ-PUBLIC-001#criterion.entrypoint
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
        criterion: REQ-PUBLIC-001#criterion.entrypoint
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
        criterion: REQ-PUBLIC-001#criterion.entrypoint
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
        criterion: REQ-PUBLIC-001#criterion.entrypoint
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
        criterion: REQ-PUBLIC-001#criterion.entrypoint
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
        criterion: REQ-PUBLIC-001#criterion.entrypoint
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
        criterion: REQ-PUBLIC-001#criterion.entrypoint
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
        criterion: REQ-PUBLIC-001#criterion.entrypoint
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
        criterion: REQ-PUBLIC-001#criterion.entrypoint
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
        criterion: REQ-PUBLIC-001#criterion.entrypoint
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
        criterion: REQ-PUBLIC-001#criterion.entrypoint
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
        criterion: REQ-PUBLIC-001#criterion.entrypoint
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
        criterion: REQ-PUBLIC-001#criterion.entrypoint
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
        criterion: REQ-PUBLIC-001#criterion.entrypoint
  - id: public-api-083
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint addressable by an exact target.
    targets:
    - id: entrypoint-083-rust-crates-syu-validation-src-lib-rs-evaluate-com
      adapter: rust
      path: crates/syu-validation/src/lib.rs
      selector:
        kind: symbol
        name: evaluate_completion
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.entrypoint
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
        criterion: REQ-PUBLIC-001#criterion.entrypoint
```
