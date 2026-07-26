---
title: "Workbench implementation / Workbench Features"
description: "Generated reference for docs/syu/workbench-features.yaml"
---

> Generated from `docs/syu/workbench-features.yaml`.

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
        - **id**: journey-layout
          - **adapter**: declared
          - **path**: crates/syu-app-ui/assets/workbench.css
          - **selector**:
            - **kind**: file
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
                - FEAT-WORKBENCH-GUIDED-JOURNEY-001#binding.journey/target.journey-layout
              - **runner**:
                - **runner**: cargo-test
                - **arguments**:
                  - **package**: syu-workbench-server
                  - **test**: tests::journey_action_exposes_one_friendly_next_step_and_can_cancel
- **id**: FEAT-WORKBENCH-COMPLETION-HISTORY-001
  - **title**: Durable completion history
  - **summary**: Render server-owned attempts and finalization state in Workbench.
  - **status**: implemented
  - **bindings**:
    - **id**: implementation
      - **role**: implementation
      - **facet**: completion-history
      - **responsibility**: Build the completion history projection from the append-only delivery store.
      - **targets**:
        - **id**: completion-history
          - **adapter**: rust
          - **path**: crates/syu-workbench-server/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: completion_history
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-WORKBENCH-009#criterion.completion-history
        - **id**: completion-history-view
          - **adapter**: rust
          - **path**: crates/syu-workbench-server/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: CompletionHistoryView
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-WORKBENCH-009#criterion.completion-history
        - **id**: completion-attempt-view
          - **adapter**: rust
          - **path**: crates/syu-workbench-server/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: CompletionAttemptView
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-WORKBENCH-009#criterion.completion-history
        - **id**: finalize-command
          - **adapter**: rust
          - **path**: crates/syu-workbench-server/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: FinalizeCommand
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-WORKBENCH-009#criterion.completion-history
        - **id**: approve-endpoint
          - **adapter**: rust
          - **path**: crates/syu-workbench-server/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: api_approve
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-WORKBENCH-009#criterion.completion-history
        - **id**: finalize-preview-endpoint
          - **adapter**: rust
          - **path**: crates/syu-workbench-server/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: api_finalize_preview
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-WORKBENCH-009#criterion.completion-history
        - **id**: finalize-apply-endpoint
          - **adapter**: rust
          - **path**: crates/syu-workbench-server/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: api_finalize_apply
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-WORKBENCH-009#criterion.completion-history
    - **id**: verification
      - **role**: verification
      - **facet**: completion-history
      - **responsibility**: Verify durable completion history projection.
      - **targets**:
        - **id**: completion-history-test
          - **adapter**: rust
          - **path**: crates/syu-workbench-server/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: tests::completion_history_projection_is_store_backed
          - **claims**:
            - **kind**: verifies
              - **criterion**: REQ-WORKBENCH-009#criterion.completion-history
              - **covers**:
                - FEAT-WORKBENCH-COMPLETION-HISTORY-001#binding.implementation/target.completion-history
                - FEAT-WORKBENCH-COMPLETION-HISTORY-001#binding.implementation/target.completion-history-view
                - FEAT-WORKBENCH-COMPLETION-HISTORY-001#binding.implementation/target.completion-attempt-view
                - FEAT-WORKBENCH-COMPLETION-HISTORY-001#binding.implementation/target.finalize-command
                - FEAT-WORKBENCH-COMPLETION-HISTORY-001#binding.implementation/target.approve-endpoint
                - FEAT-WORKBENCH-COMPLETION-HISTORY-001#binding.implementation/target.finalize-preview-endpoint
                - FEAT-WORKBENCH-COMPLETION-HISTORY-001#binding.implementation/target.finalize-apply-endpoint
              - **runner**:
                - **runner**: cargo-test
                - **arguments**:
                  - **package**: syu-workbench-server
                  - **test**: tests::completion_history_projection_is_store_backed
- **id**: FEAT-WORKBENCH-SERVER-001
  - **title**: Workbench server
  - **summary**: Expose canonical read, work, validation, edit, and security APIs.
  - **status**: implemented
  - **bindings**:
    - **id**: server
      - **role**: implementation
      - **facet**: server
      - **responsibility**: Serve canonical Workbench HTTP routes and mutation guards.
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
      - **owns**:
        - **id**: workbench-server-module
          - **adapter**: rust
          - **path**: crates/syu-workbench-server/src/lib.rs
          - **selector**:
            - **kind**: module
            - **name**: lib
- **id**: FEAT-WORKBENCH-PROJECTION-001
  - **title**: Workbench projection
  - **summary**: Project canonical readiness, work, scope, specification, and diagnostics state.
  - **status**: implemented
  - **bindings**:
    - **id**: projection
      - **role**: implementation
      - **facet**: projection
      - **responsibility**: Build the canonical Workbench projection from indexed server data.
      - **targets**:
        - **id**: project
          - **adapter**: rust
          - **path**: crates/syu-workbench-server/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: project
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-WORKBENCH-001#criterion.canonical-projection
    - **id**: projection-contract
      - **role**: contract-source
      - **facet**: projection
      - **responsibility**: Define the canonical projection boundary consumed by verification.
      - **targets**:
        - **id**: project-contract
          - **adapter**: rust
          - **path**: crates/syu-workbench-server/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: project
      - **owns**:
        - **id**: workspace-module
          - **adapter**: rust
          - **path**: crates/syu-workspace/src/lib.rs
          - **selector**:
            - **kind**: module
            - **name**: lib
        - **id**: inventory-module
          - **adapter**: rust
          - **path**: crates/syu-inventory/src/lib.rs
          - **selector**:
            - **kind**: module
            - **name**: lib
        - **id**: validation-module
          - **adapter**: rust
          - **path**: crates/syu-validation/src/lib.rs
          - **selector**:
            - **kind**: module
            - **name**: lib
        - **id**: readiness-module
          - **adapter**: rust
          - **path**: crates/syu-validation/src/readiness.rs
          - **selector**:
            - **kind**: module
            - **name**: readiness
        - **id**: readiness-file
          - **adapter**: rust
          - **path**: crates/syu-validation/src/readiness.rs
          - **selector**:
            - **kind**: file
        - **id**: diagnostics-module
          - **adapter**: rust
          - **path**: crates/syu-diagnostics/src/lib.rs
          - **selector**:
            - **kind**: module
            - **name**: lib
        - **id**: code-intel-module
          - **adapter**: rust
          - **path**: crates/syu-code-intel/src/lib.rs
          - **selector**:
            - **kind**: module
            - **name**: lib
        - **id**: branch-scope-module
          - **adapter**: rust
          - **path**: crates/syu-code-intel/src/branch_scope.rs
          - **selector**:
            - **kind**: module
            - **name**: branch_scope
        - **id**: project-model-module
          - **adapter**: rust
          - **path**: crates/syu-project-model/src/lib.rs
          - **selector**:
            - **kind**: module
            - **name**: lib
        - **id**: domain-module
          - **adapter**: rust
          - **path**: crates/syu-domain/src/lib.rs
          - **selector**:
            - **kind**: module
            - **name**: lib
  - **contracts**:
    - **id**: projection-verification
      - **kind**: function
      - **source**: FEAT-WORKBENCH-PROJECTION-001#binding.projection-contract/target.project-contract
      - **participants**:
        - **target**: FEAT-WORKBENCH-PROJECTION-001#binding.projection/target.project
          - **role**: producer
        - **target**: FEAT-WORKBENCH-VERIFICATION-001#binding.verification-harness/target.verification-endpoint
          - **role**: verifier
        - **target**: FEAT-WORKBENCH-VERIFICATION-001#binding.test-exposure/target.server-e2e
          - **role**: verifier
      - **guarantees**:
        - REQ-WORKBENCH-001#criterion.canonical-projection
- **id**: FEAT-WORKBENCH-WORK-UI-001
  - **title**: Workbench work UI
  - **summary**: Drive the WorkRequest to result-validation journey from the Work page.
  - **status**: implemented
  - **bindings**:
    - **id**: work
      - **role**: implementation
      - **facet**: work
      - **responsibility**: Plan and validate a bounded Workbench work session.
      - **targets**:
        - **id**: plan
          - **adapter**: rust
          - **path**: crates/syu-workbench-server/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: api_plan
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-WORKBENCH-002#criterion.work-session
      - **owns**:
        - **id**: planner-module
          - **adapter**: rust
          - **path**: crates/syu-planner/src/lib.rs
          - **selector**:
            - **kind**: module
            - **name**: lib
        - **id**: work-model-module
          - **adapter**: rust
          - **path**: crates/syu-work-model/src/lib.rs
          - **selector**:
            - **kind**: module
            - **name**: lib
        - **id**: cli-module
          - **adapter**: rust
          - **path**: src/lib.rs
          - **selector**:
            - **kind**: module
            - **name**: lib
        - **id**: cli-main-module
          - **adapter**: rust
          - **path**: src/main.rs
          - **selector**:
            - **kind**: module
            - **name**: main
        - **id**: cli-main-file
          - **adapter**: rust
          - **path**: src/main.rs
          - **selector**:
            - **kind**: file
        - **id**: lsp-module
          - **adapter**: rust
          - **path**: src/lsp/mod.rs
          - **selector**:
            - **kind**: module
            - **name**: mod
        - **id**: lsp-file
          - **adapter**: rust
          - **path**: src/lsp/mod.rs
          - **selector**:
            - **kind**: file
        - **id**: lsp-handlers-module
          - **adapter**: rust
          - **path**: src/lsp/handlers.rs
          - **selector**:
            - **kind**: module
            - **name**: handlers
        - **id**: lsp-handlers-file
          - **adapter**: rust
          - **path**: src/lsp/handlers.rs
          - **selector**:
            - **kind**: file
        - **id**: lsp-protocol-module
          - **adapter**: rust
          - **path**: src/lsp/protocol.rs
          - **selector**:
            - **kind**: module
            - **name**: protocol
        - **id**: lsp-protocol-file
          - **adapter**: rust
          - **path**: src/lsp/protocol.rs
          - **selector**:
            - **kind**: file
        - **id**: lsp-server-module
          - **adapter**: rust
          - **path**: src/lsp/server.rs
          - **selector**:
            - **kind**: module
            - **name**: server
        - **id**: lsp-server-file
          - **adapter**: rust
          - **path**: src/lsp/server.rs
          - **selector**:
            - **kind**: file
- **id**: FEAT-WORKBENCH-TARGET-SUGGESTIONS-001
  - **title**: Workbench target suggestions
  - **summary**: Rank exact targets with evidence and convert only reviewed approvals into WorkRequest scope.
  - **status**: implemented
  - **bindings**:
    - **id**: suggestions
      - **role**: implementation
      - **facet**: planning
      - **responsibility**: Derive, review, reject, and approve exact target candidates without silently widening executable scope.
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
- **id**: FEAT-WORKBENCH-SPEC-EDITOR-001
  - **title**: Workbench specification editor
  - **summary**: Preview and apply structured specification and configuration edits transactionally.
  - **status**: implemented
  - **bindings**:
    - **id**: editor
      - **role**: implementation
      - **facet**: editor
      - **responsibility**: Apply validated structured edits through the Workbench transaction boundary.
      - **targets**:
        - **id**: specification-apply
          - **adapter**: rust
          - **path**: crates/syu-workbench-server/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: api_specification_apply
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-WORKBENCH-003#criterion.transactional-spec-edit
        - **id**: config-apply
          - **adapter**: rust
          - **path**: crates/syu-workbench-server/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: api_config_apply
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-WORKBENCH-004#criterion.transactional-config-edit
        - **id**: specification-candidates
          - **adapter**: rust
          - **path**: crates/syu-workbench-server/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: api_specification_candidates
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-WORKBENCH-007#criterion.guided-specification-selection
        - **id**: specification-candidate-preview
          - **adapter**: rust
          - **path**: crates/syu-workbench-server/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: api_specification_candidate_preview
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-WORKBENCH-007#criterion.guided-specification-selection
      - **owns**:
        - **id**: spec-model-module
          - **adapter**: rust
          - **path**: crates/syu-spec-model/src/lib.rs
          - **selector**:
            - **kind**: module
            - **name**: lib
        - **id**: app-ui-components-module
          - **adapter**: rust
          - **path**: crates/syu-app-ui/src/components.rs
          - **selector**:
            - **kind**: module
            - **name**: components
        - **id**: app-ui-components-file
          - **adapter**: rust
          - **path**: crates/syu-app-ui/src/components.rs
          - **selector**:
            - **kind**: file
        - **id**: app-ui-document-module
          - **adapter**: rust
          - **path**: crates/syu-app-ui/src/document.rs
          - **selector**:
            - **kind**: module
            - **name**: document
        - **id**: app-ui-document-file
          - **adapter**: rust
          - **path**: crates/syu-app-ui/src/document.rs
          - **selector**:
            - **kind**: file
        - **id**: app-ui-lib-module
          - **adapter**: rust
          - **path**: crates/syu-app-ui/src/lib.rs
          - **selector**:
            - **kind**: module
            - **name**: lib
        - **id**: app-ui-pages-module
          - **adapter**: rust
          - **path**: crates/syu-app-ui/src/pages.rs
          - **selector**:
            - **kind**: module
            - **name**: pages
        - **id**: app-ui-pages-file
          - **adapter**: rust
          - **path**: crates/syu-app-ui/src/pages.rs
          - **selector**:
            - **kind**: file
        - **id**: app-ui-shell-module
          - **adapter**: rust
          - **path**: crates/syu-app-ui/src/shell.rs
          - **selector**:
            - **kind**: module
            - **name**: shell
        - **id**: app-ui-shell-file
          - **adapter**: rust
          - **path**: crates/syu-app-ui/src/shell.rs
          - **selector**:
            - **kind**: file
- **id**: FEAT-WORKBENCH-NAVIGATION-001
  - **title**: Workbench navigation
  - **summary**: Keep HTML navigation, module routes, keyboard handling, and accessibility attributes aligned.
  - **status**: implemented
  - **bindings**:
    - **id**: navigation
      - **role**: implementation
      - **facet**: tooling
      - **responsibility**: Implement browser navigation and accessibility behavior as typed server DTO consumers.
      - **targets**:
        - **id**: html-navigation
          - **adapter**: html
          - **path**: crates/syu-app-ui/assets/workbench.html
          - **selector**:
            - **kind**: marker
            - **value**: data-page="work"
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-WORKBENCH-006#criterion.accessible-navigation
        - **id**: workbench-css
          - **adapter**: declared
          - **path**: crates/syu-app-ui/assets/workbench.css
          - **selector**:
            - **kind**: file
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-WORKBENCH-006#criterion.accessible-navigation
        - **id**: javascript-navigation
          - **adapter**: javascript
          - **path**: crates/syu-app-ui/assets/js/router.js
          - **selector**:
            - **kind**: symbol
            - **name**: bindRouter
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-WORKBENCH-006#criterion.accessible-navigation
        - **id**: keyboard-navigation
          - **adapter**: javascript
          - **path**: crates/syu-app-ui/assets/js/router.js
          - **selector**:
            - **kind**: symbol
            - **name**: bindKeyboardTabs
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-WORKBENCH-006#criterion.accessible-navigation
        - **id**: accessibility-attributes
          - **adapter**: html
          - **path**: crates/syu-app-ui/assets/workbench.html
          - **selector**:
            - **kind**: marker
            - **value**: data-i18n-aria="a11y.main_pages"
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-WORKBENCH-006#criterion.accessible-navigation
        - **id**: projection-bootstrap
          - **adapter**: javascript
          - **path**: crates/syu-app-ui/assets/js/main.js
          - **selector**:
            - **kind**: symbol
            - **name**: startWorkbench
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-WORKBENCH-012#criterion.responsive-browser-interactions
        - **id**: active-page-render
          - **adapter**: javascript
          - **path**: crates/syu-app-ui/assets/js/main.js
          - **selector**:
            - **kind**: symbol
            - **name**: render
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-WORKBENCH-012#criterion.responsive-browser-interactions
        - **id**: local-specification-filter
          - **adapter**: javascript
          - **path**: crates/syu-app-ui/assets/js/pages/specifications.js
          - **selector**:
            - **kind**: symbol
            - **name**: candidatesFor
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-WORKBENCH-012#criterion.responsive-browser-interactions
        - **id**: busy-status
          - **adapter**: html
          - **path**: crates/syu-app-ui/assets/workbench.html
          - **selector**:
            - **kind**: marker
            - **value**: data-workbench-status="busy"
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-WORKBENCH-012#criterion.responsive-browser-interactions
      - **owns**:
        - **id**: assets-api-module
          - **adapter**: javascript
          - **path**: crates/syu-app-ui/assets/js/api.js
          - **selector**:
            - **kind**: module
            - **name**: *
        - **id**: assets-action-module
          - **adapter**: javascript
          - **path**: crates/syu-app-ui/assets/js/components/action.js
          - **selector**:
            - **kind**: module
            - **name**: *
        - **id**: assets-diagnostic-module
          - **adapter**: javascript
          - **path**: crates/syu-app-ui/assets/js/components/diagnostic.js
          - **selector**:
            - **kind**: module
            - **name**: *
        - **id**: assets-diff-module
          - **adapter**: javascript
          - **path**: crates/syu-app-ui/assets/js/components/diff.js
          - **selector**:
            - **kind**: module
            - **name**: *
        - **id**: assets-editor-module
          - **adapter**: javascript
          - **path**: crates/syu-app-ui/assets/js/components/editor.js
          - **selector**:
            - **kind**: module
            - **name**: *
        - **id**: assets-readiness-module
          - **adapter**: javascript
          - **path**: crates/syu-app-ui/assets/js/components/readiness.js
          - **selector**:
            - **kind**: module
            - **name**: *
        - **id**: assets-target-module
          - **adapter**: javascript
          - **path**: crates/syu-app-ui/assets/js/components/target.js
          - **selector**:
            - **kind**: module
            - **name**: *
        - **id**: assets-js-i18n-module
          - **adapter**: javascript
          - **path**: crates/syu-app-ui/assets/js/i18n.js
          - **selector**:
            - **kind**: module
            - **name**: *
        - **id**: assets-main-module
          - **adapter**: javascript
          - **path**: crates/syu-app-ui/assets/js/main.js
          - **selector**:
            - **kind**: module
            - **name**: *
        - **id**: assets-router-module
          - **adapter**: javascript
          - **path**: crates/syu-app-ui/assets/js/router.js
          - **selector**:
            - **kind**: module
            - **name**: *
        - **id**: assets-state-module
          - **adapter**: javascript
          - **path**: crates/syu-app-ui/assets/js/state.js
          - **selector**:
            - **kind**: module
            - **name**: *
        - **id**: assets-page-diagnostics-module
          - **adapter**: javascript
          - **path**: crates/syu-app-ui/assets/js/pages/diagnostics.js
          - **selector**:
            - **kind**: module
            - **name**: *
        - **id**: assets-page-readiness-module
          - **adapter**: javascript
          - **path**: crates/syu-app-ui/assets/js/pages/readiness.js
          - **selector**:
            - **kind**: module
            - **name**: *
        - **id**: assets-page-scope-module
          - **adapter**: javascript
          - **path**: crates/syu-app-ui/assets/js/pages/scope.js
          - **selector**:
            - **kind**: module
            - **name**: *
        - **id**: assets-page-settings-module
          - **adapter**: javascript
          - **path**: crates/syu-app-ui/assets/js/pages/settings.js
          - **selector**:
            - **kind**: module
            - **name**: *
        - **id**: assets-page-specifications-module
          - **adapter**: javascript
          - **path**: crates/syu-app-ui/assets/js/pages/specifications.js
          - **selector**:
            - **kind**: module
            - **name**: *
        - **id**: assets-page-work-module
          - **adapter**: javascript
          - **path**: crates/syu-app-ui/assets/js/pages/work.js
          - **selector**:
            - **kind**: module
            - **name**: *
        - **id**: html-file
          - **adapter**: html
          - **path**: crates/syu-app-ui/assets/workbench.html
          - **selector**:
            - **kind**: file
        - **id**: declared-html-file
          - **adapter**: declared
          - **path**: crates/syu-app-ui/assets/workbench.html
          - **selector**:
            - **kind**: file
        - **id**: locale-en-file
          - **adapter**: declared
          - **path**: crates/syu-app-ui/assets/locales/en.json
          - **selector**:
            - **kind**: file
        - **id**: locale-ja-file
          - **adapter**: declared
          - **path**: crates/syu-app-ui/assets/locales/ja.json
          - **selector**:
            - **kind**: file
- **id**: FEAT-WORKBENCH-VERIFICATION-001
  - **title**: Workbench verification tests
  - **summary**: Exercise the real Workbench HTTP server and exact verification targets.
  - **status**: implemented
  - **bindings**:
    - **id**: verification-harness
      - **role**: implementation
      - **facet**: verification
      - **responsibility**: Maintain the executable Workbench HTTP verification harness.
      - **targets**:
        - **id**: verification-endpoint
          - **adapter**: rust
          - **path**: crates/syu-workbench-server/src/lib.rs
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
          - **path**: crates/syu-workbench-server/src/lib.rs
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
                  - **package**: syu-workbench-server
                  - **test**: tests::workbench_http_closed_loop_flow
        - **id**: server-transport
          - **adapter**: rust
          - **path**: crates/syu-workbench-server/src/lib.rs
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
                  - **package**: syu-workbench-server
                  - **test**: tests::workbench_http_server_transport_flow
        - **id**: command-help
          - **adapter**: rust
          - **path**: tests/workbench_command.rs
          - **selector**:
            - **kind**: symbol
            - **name**: workbench_help_lists_browser_launch_options
          - **claims**:
            - **kind**: verifies
              - **criterion**: REQ-WORKBENCH-002#criterion.work-session
              - **covers**:
                - FEAT-WORKBENCH-WORK-UI-001#binding.work/target.plan
              - **runner**:
                - **runner**: cargo-test-integration
                - **arguments**:
                  - **package**: syu
                  - **harness**: workbench_command
                  - **test**: workbench_help_lists_browser_launch_options
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
                  - **package**: syu
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
                  - **package**: syu
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
                  - **package**: syu
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
                  - **package**: syu
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
                  - **package**: syu
                  - **harness**: workbench_visual
                  - **test**: workbench_rendered_dom_uses_projection_driven_placeholders
- **id**: FEAT-WORKBENCH-QUALITY-GATES-001
  - **title**: Layered quality gates
  - **summary**: Separate staged pre-commit checks, push-diff fast checks, and complete CI validation.
  - **status**: implemented
  - **bindings**:
    - **id**: quality-gates
      - **role**: implementation
      - **facet**: repository-tooling
      - **responsibility**: Run only the quality checks appropriate to each Git lifecycle stage.
      - **targets**:
        - **id**: quality-gate-script
          - **adapter**: declared
          - **path**: scripts/ci/quality-gates.sh
          - **selector**:
            - **kind**: file
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-WORKBENCH-010#criterion.lifecycle-separation
        - **id**: release-artifacts-configuration
          - **adapter**: declared
          - **path**: .github/workflows/release-artifacts.yml
          - **selector**:
            - **kind**: file
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-WORKBENCH-010#criterion.lifecycle-separation
        - **id**: pre-commit-configuration
          - **adapter**: declared
          - **path**: .pre-commit-config.yaml
          - **selector**:
            - **kind**: file
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-WORKBENCH-010#criterion.lifecycle-separation
        - **id**: pre-commit-ci-configuration
          - **adapter**: declared
          - **path**: .pre-commit-ci.yaml
          - **selector**:
            - **kind**: file
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-WORKBENCH-010#criterion.lifecycle-separation
      - **owns**:
        - **id**: ci-workflow
          - **adapter**: declared
          - **path**: .github/workflows/ci.yml
          - **selector**:
            - **kind**: file
        - **id**: quality-workflow
          - **adapter**: declared
          - **path**: .github/workflows/quality.yml
          - **selector**:
            - **kind**: file
        - **id**: branch-push-workflow
          - **adapter**: declared
          - **path**: .github/workflows/branch-push.yml
          - **selector**:
            - **kind**: file
        - **id**: release-artifacts-workflow
          - **adapter**: declared
          - **path**: .github/workflows/release-artifacts.yml
          - **selector**:
            - **kind**: file
        - **id**: pre-commit-config
          - **adapter**: declared
          - **path**: .pre-commit-config.yaml
          - **selector**:
            - **kind**: file
        - **id**: pre-commit-ci-config
          - **adapter**: declared
          - **path**: .pre-commit-ci.yaml
          - **selector**:
            - **kind**: file
        - **id**: contributing-guide
          - **adapter**: declared
          - **path**: CONTRIBUTING.md
          - **selector**:
            - **kind**: file
        - **id**: cargo-manifest
          - **adapter**: declared
          - **path**: Cargo.toml
          - **selector**:
            - **kind**: file
        - **id**: root-build-script
          - **adapter**: declared
          - **path**: build.rs
          - **selector**:
            - **kind**: file
        - **id**: root-build-module
          - **adapter**: rust
          - **path**: build.rs
          - **selector**:
            - **kind**: module
            - **name**: *
        - **id**: changed-validation-hook
          - **adapter**: declared
          - **path**: scripts/dev/validate-changed.sh
          - **selector**:
            - **kind**: file
        - **id**: cli-validation
          - **adapter**: declared
          - **path**: src/lib.rs
          - **selector**:
            - **kind**: file
        - **id**: cli-validation-tests
          - **adapter**: declared
          - **path**: tests/v1_cli.rs
          - **selector**:
            - **kind**: file
        - **id**: repository-configuration
          - **adapter**: declared
          - **path**: syu.yaml
          - **selector**:
            - **kind**: file
        - **id**: workbench-contract-check
          - **adapter**: declared
          - **path**: scripts/ci/check-workbench-contract.py
          - **selector**:
            - **kind**: file
        - **id**: workbench-visual-check
          - **adapter**: declared
          - **path**: scripts/ci/workbench-visual.sh
          - **selector**:
            - **kind**: file
    - **id**: quality-gates-verification
      - **role**: verification
      - **facet**: repository-tooling
      - **responsibility**: Verify that staged validation uses the Git index snapshot.
      - **targets**:
        - **id**: staged-validation-test
          - **adapter**: rust
          - **path**: tests/v1_cli.rs
          - **selector**:
            - **kind**: symbol
            - **name**: staged_change_validation_uses_the_index_snapshot
          - **claims**:
            - **kind**: verifies
              - **criterion**: REQ-WORKBENCH-010#criterion.lifecycle-separation
              - **covers**:
                - FEAT-WORKBENCH-QUALITY-GATES-001#binding.quality-gates/target.quality-gate-script
                - FEAT-WORKBENCH-QUALITY-GATES-001#binding.quality-gates/target.release-artifacts-configuration
                - FEAT-WORKBENCH-QUALITY-GATES-001#binding.quality-gates/target.pre-commit-configuration
                - FEAT-WORKBENCH-QUALITY-GATES-001#binding.quality-gates/target.pre-commit-ci-configuration
              - **runner**:
                - **runner**: cargo-test-integration
                - **arguments**:
                  - **package**: syu
                  - **harness**: v1_cli
                  - **test**: staged_change_validation_uses_the_index_snapshot
- **id**: FEAT-WORKBENCH-SCOPED-AGENT-001
  - **title**: Scoped agent evidence
  - **summary**: Expose the approved agent boundary and append-only execution evidence in Workbench.
  - **status**: implemented
  - **bindings**:
    - **id**: agent-api
      - **role**: implementation
      - **facet**: scoped-agent
      - **responsibility**: Connect Workbench actions to the provider-neutral scoped agent API.
      - **targets**:
        - **id**: agent-start
          - **adapter**: rust
          - **path**: crates/syu-workbench-server/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: api_agent_start
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-WORKBENCH-011#criterion.scoped-agent
        - **id**: agent-patch
          - **adapter**: rust
          - **path**: crates/syu-workbench-server/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: api_agent_patch
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-WORKBENCH-011#criterion.scoped-agent
        - **id**: agent-projection
          - **adapter**: rust
          - **path**: crates/syu-workbench-server/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: project_session
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-WORKBENCH-011#criterion.scoped-agent
        - **id**: agent-ui
          - **adapter**: javascript
          - **path**: crates/syu-app-ui/assets/js/pages/work.js
          - **selector**:
            - **kind**: symbol
            - **name**: renderWork
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-WORKBENCH-011#criterion.scoped-agent
    - **id**: verification
      - **role**: verification
      - **facet**: scoped-agent
      - **responsibility**: Verify agent rejection and evidence rendering through the Workbench server.
      - **targets**:
        - **id**: agent-http-test
          - **adapter**: rust
          - **path**: crates/syu-workbench-server/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: tests::workbench_agent_rejects_unrelated_write_before_application
          - **claims**:
            - **kind**: verifies
              - **criterion**: REQ-WORKBENCH-011#criterion.scoped-agent
              - **covers**:
                - FEAT-WORKBENCH-SCOPED-AGENT-001#binding.agent-api/target.agent-start
                - FEAT-WORKBENCH-SCOPED-AGENT-001#binding.agent-api/target.agent-patch
                - FEAT-WORKBENCH-SCOPED-AGENT-001#binding.agent-api/target.agent-projection
                - FEAT-WORKBENCH-SCOPED-AGENT-001#binding.agent-api/target.agent-ui
              - **runner**:
                - **runner**: cargo-test
                - **arguments**:
                  - **package**: syu-workbench-server
                  - **test**: tests::workbench_agent_rejects_unrelated_write_before_application

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
    - id: journey-layout
      adapter: declared
      path: crates/syu-app-ui/assets/workbench.css
      selector: { kind: file }
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
        - FEAT-WORKBENCH-GUIDED-JOURNEY-001#binding.journey/target.journey-layout
        runner: { runner: cargo-test, arguments: { package: syu-workbench-server, test: tests::journey_action_exposes_one_friendly_next_step_and_can_cancel } }
- id: FEAT-WORKBENCH-COMPLETION-HISTORY-001
  title: Durable completion history
  summary: Render server-owned attempts and finalization state in Workbench.
  status: implemented
  bindings:
  - id: implementation
    role: implementation
    facet: completion-history
    responsibility: Build the completion history projection from the append-only delivery store.
    targets:
    - id: completion-history
      adapter: rust
      path: crates/syu-workbench-server/src/lib.rs
      selector:
        kind: symbol
        name: completion_history
      claims:
      - kind: satisfies
        criterion: REQ-WORKBENCH-009#criterion.completion-history
    - id: completion-history-view
      adapter: rust
      path: crates/syu-workbench-server/src/lib.rs
      selector:
        kind: symbol
        name: CompletionHistoryView
      claims:
      - kind: satisfies
        criterion: REQ-WORKBENCH-009#criterion.completion-history
    - id: completion-attempt-view
      adapter: rust
      path: crates/syu-workbench-server/src/lib.rs
      selector:
        kind: symbol
        name: CompletionAttemptView
      claims:
      - kind: satisfies
        criterion: REQ-WORKBENCH-009#criterion.completion-history
    - id: finalize-command
      adapter: rust
      path: crates/syu-workbench-server/src/lib.rs
      selector:
        kind: symbol
        name: FinalizeCommand
      claims:
      - kind: satisfies
        criterion: REQ-WORKBENCH-009#criterion.completion-history
    - id: approve-endpoint
      adapter: rust
      path: crates/syu-workbench-server/src/lib.rs
      selector:
        kind: symbol
        name: api_approve
      claims:
      - kind: satisfies
        criterion: REQ-WORKBENCH-009#criterion.completion-history
    - id: finalize-preview-endpoint
      adapter: rust
      path: crates/syu-workbench-server/src/lib.rs
      selector:
        kind: symbol
        name: api_finalize_preview
      claims:
      - kind: satisfies
        criterion: REQ-WORKBENCH-009#criterion.completion-history
    - id: finalize-apply-endpoint
      adapter: rust
      path: crates/syu-workbench-server/src/lib.rs
      selector:
        kind: symbol
        name: api_finalize_apply
      claims:
      - kind: satisfies
        criterion: REQ-WORKBENCH-009#criterion.completion-history
  - id: verification
    role: verification
    facet: completion-history
    responsibility: Verify durable completion history projection.
    targets:
    - id: completion-history-test
      adapter: rust
      path: crates/syu-workbench-server/src/lib.rs
      selector:
        kind: symbol
        name: tests::completion_history_projection_is_store_backed
      claims:
      - kind: verifies
        criterion: REQ-WORKBENCH-009#criterion.completion-history
        covers:
        - FEAT-WORKBENCH-COMPLETION-HISTORY-001#binding.implementation/target.completion-history
        - FEAT-WORKBENCH-COMPLETION-HISTORY-001#binding.implementation/target.completion-history-view
        - FEAT-WORKBENCH-COMPLETION-HISTORY-001#binding.implementation/target.completion-attempt-view
        - FEAT-WORKBENCH-COMPLETION-HISTORY-001#binding.implementation/target.finalize-command
        - FEAT-WORKBENCH-COMPLETION-HISTORY-001#binding.implementation/target.approve-endpoint
        - FEAT-WORKBENCH-COMPLETION-HISTORY-001#binding.implementation/target.finalize-preview-endpoint
        - FEAT-WORKBENCH-COMPLETION-HISTORY-001#binding.implementation/target.finalize-apply-endpoint
        runner:
          runner: cargo-test
          arguments:
            package: syu-workbench-server
            test: tests::completion_history_projection_is_store_backed

- id: FEAT-WORKBENCH-SERVER-001
  title: Workbench server
  summary: Expose canonical read, work, validation, edit, and security APIs.
  status: implemented
  bindings:
  - id: server
    role: implementation
    facet: server
    responsibility: Serve canonical Workbench HTTP routes and mutation guards.
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
    owns:
    - id: workbench-server-module
      adapter: rust
      path: crates/syu-workbench-server/src/lib.rs
      selector:
        kind: module
        name: lib
- id: FEAT-WORKBENCH-PROJECTION-001
  title: Workbench projection
  summary: Project canonical readiness, work, scope, specification, and diagnostics state.
  status: implemented
  bindings:
  - id: projection
    role: implementation
    facet: projection
    responsibility: Build the canonical Workbench projection from indexed server data.
    targets:
    - id: project
      adapter: rust
      path: crates/syu-workbench-server/src/lib.rs
      selector:
        kind: symbol
        name: project
      claims:
      - kind: satisfies
        criterion: REQ-WORKBENCH-001#criterion.canonical-projection
  - id: projection-contract
    role: contract-source
    facet: projection
    responsibility: Define the canonical projection boundary consumed by verification.
    targets:
    - id: project-contract
      adapter: rust
      path: crates/syu-workbench-server/src/lib.rs
      selector:
        kind: symbol
        name: project
    owns:
    - id: workspace-module
      adapter: rust
      path: crates/syu-workspace/src/lib.rs
      selector:
        kind: module
        name: lib
    - id: inventory-module
      adapter: rust
      path: crates/syu-inventory/src/lib.rs
      selector:
        kind: module
        name: lib
    - id: validation-module
      adapter: rust
      path: crates/syu-validation/src/lib.rs
      selector:
        kind: module
        name: lib
    - id: readiness-module
      adapter: rust
      path: crates/syu-validation/src/readiness.rs
      selector:
        kind: module
        name: readiness
    - id: readiness-file
      adapter: rust
      path: crates/syu-validation/src/readiness.rs
      selector:
        kind: file
    - id: diagnostics-module
      adapter: rust
      path: crates/syu-diagnostics/src/lib.rs
      selector:
        kind: module
        name: lib
    - id: code-intel-module
      adapter: rust
      path: crates/syu-code-intel/src/lib.rs
      selector:
        kind: module
        name: lib
    - id: branch-scope-module
      adapter: rust
      path: crates/syu-code-intel/src/branch_scope.rs
      selector:
        kind: module
        name: branch_scope
    - id: project-model-module
      adapter: rust
      path: crates/syu-project-model/src/lib.rs
      selector:
        kind: module
        name: lib
    - id: domain-module
      adapter: rust
      path: crates/syu-domain/src/lib.rs
      selector:
        kind: module
        name: lib
  contracts:
  - id: projection-verification
    kind: function
    source: FEAT-WORKBENCH-PROJECTION-001#binding.projection-contract/target.project-contract
    participants:
    - target: FEAT-WORKBENCH-PROJECTION-001#binding.projection/target.project
      role: producer
    - target: FEAT-WORKBENCH-VERIFICATION-001#binding.verification-harness/target.verification-endpoint
      role: verifier
    - target: FEAT-WORKBENCH-VERIFICATION-001#binding.test-exposure/target.server-e2e
      role: verifier
    guarantees:
    - REQ-WORKBENCH-001#criterion.canonical-projection
- id: FEAT-WORKBENCH-WORK-UI-001
  title: Workbench work UI
  summary: Drive the WorkRequest to result-validation journey from the Work page.
  status: implemented
  bindings:
  - id: work
    role: implementation
    facet: work
    responsibility: Plan and validate a bounded Workbench work session.
    targets:
    - id: plan
      adapter: rust
      path: crates/syu-workbench-server/src/lib.rs
      selector:
        kind: symbol
        name: api_plan
      claims:
      - kind: satisfies
        criterion: REQ-WORKBENCH-002#criterion.work-session
    owns:
    - id: planner-module
      adapter: rust
      path: crates/syu-planner/src/lib.rs
      selector:
        kind: module
        name: lib
    - id: work-model-module
      adapter: rust
      path: crates/syu-work-model/src/lib.rs
      selector:
        kind: module
        name: lib
    - id: cli-module
      adapter: rust
      path: src/lib.rs
      selector:
        kind: module
        name: lib
    - id: cli-main-module
      adapter: rust
      path: src/main.rs
      selector:
        kind: module
        name: main
    - id: cli-main-file
      adapter: rust
      path: src/main.rs
      selector:
        kind: file
    - id: lsp-module
      adapter: rust
      path: src/lsp/mod.rs
      selector:
        kind: module
        name: mod
    - id: lsp-file
      adapter: rust
      path: src/lsp/mod.rs
      selector:
        kind: file
    - id: lsp-handlers-module
      adapter: rust
      path: src/lsp/handlers.rs
      selector:
        kind: module
        name: handlers
    - id: lsp-handlers-file
      adapter: rust
      path: src/lsp/handlers.rs
      selector:
        kind: file
    - id: lsp-protocol-module
      adapter: rust
      path: src/lsp/protocol.rs
      selector:
        kind: module
        name: protocol
    - id: lsp-protocol-file
      adapter: rust
      path: src/lsp/protocol.rs
      selector:
        kind: file
    - id: lsp-server-module
      adapter: rust
      path: src/lsp/server.rs
      selector:
        kind: module
        name: server
    - id: lsp-server-file
      adapter: rust
      path: src/lsp/server.rs
      selector:
        kind: file
- id: FEAT-WORKBENCH-TARGET-SUGGESTIONS-001
  title: Workbench target suggestions
  summary: Rank exact targets with evidence and convert only reviewed approvals into WorkRequest scope.
  status: implemented
  bindings:
  - id: suggestions
    role: implementation
    facet: planning
    responsibility: Derive, review, reject, and approve exact target candidates without silently widening executable scope.
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
- id: FEAT-WORKBENCH-SPEC-EDITOR-001
  title: Workbench specification editor
  summary: Preview and apply structured specification and configuration edits transactionally.
  status: implemented
  bindings:
  - id: editor
    role: implementation
    facet: editor
    responsibility: Apply validated structured edits through the Workbench transaction boundary.
    targets:
    - id: specification-apply
      adapter: rust
      path: crates/syu-workbench-server/src/lib.rs
      selector:
        kind: symbol
        name: api_specification_apply
      claims:
      - kind: satisfies
        criterion: REQ-WORKBENCH-003#criterion.transactional-spec-edit
    - id: config-apply
      adapter: rust
      path: crates/syu-workbench-server/src/lib.rs
      selector:
        kind: symbol
        name: api_config_apply
      claims:
      - kind: satisfies
        criterion: REQ-WORKBENCH-004#criterion.transactional-config-edit
    - id: specification-candidates
      adapter: rust
      path: crates/syu-workbench-server/src/lib.rs
      selector:
        kind: symbol
        name: api_specification_candidates
      claims:
      - kind: satisfies
        criterion: REQ-WORKBENCH-007#criterion.guided-specification-selection
    - id: specification-candidate-preview
      adapter: rust
      path: crates/syu-workbench-server/src/lib.rs
      selector:
        kind: symbol
        name: api_specification_candidate_preview
      claims:
      - kind: satisfies
        criterion: REQ-WORKBENCH-007#criterion.guided-specification-selection
    owns:
    - id: spec-model-module
      adapter: rust
      path: crates/syu-spec-model/src/lib.rs
      selector:
        kind: module
        name: lib
    - id: app-ui-components-module
      adapter: rust
      path: crates/syu-app-ui/src/components.rs
      selector:
        kind: module
        name: components
    - id: app-ui-components-file
      adapter: rust
      path: crates/syu-app-ui/src/components.rs
      selector:
        kind: file
    - id: app-ui-document-module
      adapter: rust
      path: crates/syu-app-ui/src/document.rs
      selector:
        kind: module
        name: document
    - id: app-ui-document-file
      adapter: rust
      path: crates/syu-app-ui/src/document.rs
      selector:
        kind: file
    - id: app-ui-lib-module
      adapter: rust
      path: crates/syu-app-ui/src/lib.rs
      selector:
        kind: module
        name: lib
    - id: app-ui-pages-module
      adapter: rust
      path: crates/syu-app-ui/src/pages.rs
      selector:
        kind: module
        name: pages
    - id: app-ui-pages-file
      adapter: rust
      path: crates/syu-app-ui/src/pages.rs
      selector:
        kind: file
    - id: app-ui-shell-module
      adapter: rust
      path: crates/syu-app-ui/src/shell.rs
      selector:
        kind: module
        name: shell
    - id: app-ui-shell-file
      adapter: rust
      path: crates/syu-app-ui/src/shell.rs
      selector:
        kind: file
- id: FEAT-WORKBENCH-NAVIGATION-001
  title: Workbench navigation
  summary: Keep HTML navigation, module routes, keyboard handling, and accessibility attributes aligned.
  status: implemented
  bindings:
  - id: navigation
    role: implementation
    facet: tooling
    responsibility: Implement browser navigation and accessibility behavior as typed server DTO consumers.
    targets:
    - id: html-navigation
      adapter: html
      path: crates/syu-app-ui/assets/workbench.html
      selector:
        kind: marker
        value: data-page="work"
      claims:
      - kind: satisfies
        criterion: REQ-WORKBENCH-006#criterion.accessible-navigation
    - id: workbench-css
      adapter: declared
      path: crates/syu-app-ui/assets/workbench.css
      selector:
        kind: file
      claims:
      - kind: satisfies
        criterion: REQ-WORKBENCH-006#criterion.accessible-navigation
    - id: javascript-navigation
      adapter: javascript
      path: crates/syu-app-ui/assets/js/router.js
      selector:
        kind: symbol
        name: bindRouter
      claims:
      - kind: satisfies
        criterion: REQ-WORKBENCH-006#criterion.accessible-navigation
    - id: keyboard-navigation
      adapter: javascript
      path: crates/syu-app-ui/assets/js/router.js
      selector:
        kind: symbol
        name: bindKeyboardTabs
      claims:
      - kind: satisfies
        criterion: REQ-WORKBENCH-006#criterion.accessible-navigation
    - id: accessibility-attributes
      adapter: html
      path: crates/syu-app-ui/assets/workbench.html
      selector:
        kind: marker
        value: data-i18n-aria="a11y.main_pages"
      claims:
      - kind: satisfies
        criterion: REQ-WORKBENCH-006#criterion.accessible-navigation
    - id: projection-bootstrap
      adapter: javascript
      path: crates/syu-app-ui/assets/js/main.js
      selector:
        kind: symbol
        name: startWorkbench
      claims:
      - kind: satisfies
        criterion: REQ-WORKBENCH-012#criterion.responsive-browser-interactions
    - id: active-page-render
      adapter: javascript
      path: crates/syu-app-ui/assets/js/main.js
      selector:
        kind: symbol
        name: render
      claims:
      - kind: satisfies
        criterion: REQ-WORKBENCH-012#criterion.responsive-browser-interactions
    - id: local-specification-filter
      adapter: javascript
      path: crates/syu-app-ui/assets/js/pages/specifications.js
      selector:
        kind: symbol
        name: candidatesFor
      claims:
      - kind: satisfies
        criterion: REQ-WORKBENCH-012#criterion.responsive-browser-interactions
    - id: busy-status
      adapter: html
      path: crates/syu-app-ui/assets/workbench.html
      selector:
        kind: marker
        value: data-workbench-status="busy"
      claims:
      - kind: satisfies
        criterion: REQ-WORKBENCH-012#criterion.responsive-browser-interactions
    owns:
    - id: assets-api-module
      adapter: javascript
      path: crates/syu-app-ui/assets/js/api.js
      selector:
        kind: module
        name: '*'
    - id: assets-action-module
      adapter: javascript
      path: crates/syu-app-ui/assets/js/components/action.js
      selector:
        kind: module
        name: '*'
    - id: assets-diagnostic-module
      adapter: javascript
      path: crates/syu-app-ui/assets/js/components/diagnostic.js
      selector:
        kind: module
        name: '*'
    - id: assets-diff-module
      adapter: javascript
      path: crates/syu-app-ui/assets/js/components/diff.js
      selector:
        kind: module
        name: '*'
    - id: assets-editor-module
      adapter: javascript
      path: crates/syu-app-ui/assets/js/components/editor.js
      selector:
        kind: module
        name: '*'
    - id: assets-readiness-module
      adapter: javascript
      path: crates/syu-app-ui/assets/js/components/readiness.js
      selector:
        kind: module
        name: '*'
    - id: assets-target-module
      adapter: javascript
      path: crates/syu-app-ui/assets/js/components/target.js
      selector:
        kind: module
        name: '*'
    - id: assets-js-i18n-module
      adapter: javascript
      path: crates/syu-app-ui/assets/js/i18n.js
      selector:
        kind: module
        name: '*'
    - id: assets-main-module
      adapter: javascript
      path: crates/syu-app-ui/assets/js/main.js
      selector:
        kind: module
        name: '*'
    - id: assets-router-module
      adapter: javascript
      path: crates/syu-app-ui/assets/js/router.js
      selector:
        kind: module
        name: '*'
    - id: assets-state-module
      adapter: javascript
      path: crates/syu-app-ui/assets/js/state.js
      selector:
        kind: module
        name: '*'
    - id: assets-page-diagnostics-module
      adapter: javascript
      path: crates/syu-app-ui/assets/js/pages/diagnostics.js
      selector:
        kind: module
        name: '*'
    - id: assets-page-readiness-module
      adapter: javascript
      path: crates/syu-app-ui/assets/js/pages/readiness.js
      selector:
        kind: module
        name: '*'
    - id: assets-page-scope-module
      adapter: javascript
      path: crates/syu-app-ui/assets/js/pages/scope.js
      selector:
        kind: module
        name: '*'
    - id: assets-page-settings-module
      adapter: javascript
      path: crates/syu-app-ui/assets/js/pages/settings.js
      selector:
        kind: module
        name: '*'
    - id: assets-page-specifications-module
      adapter: javascript
      path: crates/syu-app-ui/assets/js/pages/specifications.js
      selector:
        kind: module
        name: '*'
    - id: assets-page-work-module
      adapter: javascript
      path: crates/syu-app-ui/assets/js/pages/work.js
      selector:
        kind: module
        name: '*'
    - id: html-file
      adapter: html
      path: crates/syu-app-ui/assets/workbench.html
      selector:
        kind: file
    - id: declared-html-file
      adapter: declared
      path: crates/syu-app-ui/assets/workbench.html
      selector:
        kind: file
    - id: locale-en-file
      adapter: declared
      path: crates/syu-app-ui/assets/locales/en.json
      selector:
        kind: file
    - id: locale-ja-file
      adapter: declared
      path: crates/syu-app-ui/assets/locales/ja.json
      selector:
        kind: file
- id: FEAT-WORKBENCH-VERIFICATION-001
  title: Workbench verification tests
  summary: Exercise the real Workbench HTTP server and exact verification targets.
  status: implemented
  bindings:
  - id: verification-harness
    role: implementation
    facet: verification
    responsibility: Maintain the executable Workbench HTTP verification harness.
    targets:
    - id: verification-endpoint
      adapter: rust
      path: crates/syu-workbench-server/src/lib.rs
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
      path: crates/syu-workbench-server/src/lib.rs
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
            package: syu-workbench-server
            test: tests::workbench_http_closed_loop_flow
    - id: server-transport
      adapter: rust
      path: crates/syu-workbench-server/src/lib.rs
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
            package: syu-workbench-server
            test: tests::workbench_http_server_transport_flow
    - id: command-help
      adapter: rust
      path: tests/workbench_command.rs
      selector:
        kind: symbol
        name: workbench_help_lists_browser_launch_options
      claims:
      - kind: verifies
        criterion: REQ-WORKBENCH-002#criterion.work-session
        covers:
        - FEAT-WORKBENCH-WORK-UI-001#binding.work/target.plan
        runner:
          runner: cargo-test-integration
          arguments:
            package: syu
            harness: workbench_command
            test: workbench_help_lists_browser_launch_options
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
            package: syu
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
            package: syu
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
            package: syu
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
            package: syu
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
            package: syu
            harness: workbench_visual
            test: workbench_rendered_dom_uses_projection_driven_placeholders
- id: FEAT-WORKBENCH-QUALITY-GATES-001
  title: Layered quality gates
  summary: Separate staged pre-commit checks, push-diff fast checks, and complete CI validation.
  status: implemented
  bindings:
  - id: quality-gates
    role: implementation
    facet: repository-tooling
    responsibility: Run only the quality checks appropriate to each Git lifecycle stage.
    targets:
    - id: quality-gate-script
      adapter: declared
      path: scripts/ci/quality-gates.sh
      selector:
        kind: file
      claims:
      - kind: satisfies
        criterion: REQ-WORKBENCH-010#criterion.lifecycle-separation
    - id: release-artifacts-configuration
      adapter: declared
      path: .github/workflows/release-artifacts.yml
      selector: { kind: file }
      claims:
      - kind: satisfies
        criterion: REQ-WORKBENCH-010#criterion.lifecycle-separation
    - id: pre-commit-configuration
      adapter: declared
      path: .pre-commit-config.yaml
      selector: { kind: file }
      claims:
      - kind: satisfies
        criterion: REQ-WORKBENCH-010#criterion.lifecycle-separation
    - id: pre-commit-ci-configuration
      adapter: declared
      path: .pre-commit-ci.yaml
      selector: { kind: file }
      claims:
      - kind: satisfies
        criterion: REQ-WORKBENCH-010#criterion.lifecycle-separation
    owns:
    - id: ci-workflow
      adapter: declared
      path: .github/workflows/ci.yml
      selector: { kind: file }
    - id: quality-workflow
      adapter: declared
      path: .github/workflows/quality.yml
      selector: { kind: file }
    - id: branch-push-workflow
      adapter: declared
      path: .github/workflows/branch-push.yml
      selector: { kind: file }
    - id: release-artifacts-workflow
      adapter: declared
      path: .github/workflows/release-artifacts.yml
      selector: { kind: file }
    - id: pre-commit-config
      adapter: declared
      path: .pre-commit-config.yaml
      selector: { kind: file }
    - id: pre-commit-ci-config
      adapter: declared
      path: .pre-commit-ci.yaml
      selector: { kind: file }
    - id: contributing-guide
      adapter: declared
      path: CONTRIBUTING.md
      selector: { kind: file }
    - id: cargo-manifest
      adapter: declared
      path: Cargo.toml
      selector: { kind: file }
    - id: root-build-script
      adapter: declared
      path: build.rs
      selector: { kind: file }
    - id: root-build-module
      adapter: rust
      path: build.rs
      selector: { kind: module, name: "*" }
    - id: changed-validation-hook
      adapter: declared
      path: scripts/dev/validate-changed.sh
      selector: { kind: file }
    - id: cli-validation
      adapter: declared
      path: src/lib.rs
      selector: { kind: file }
    - id: cli-validation-tests
      adapter: declared
      path: tests/v1_cli.rs
      selector: { kind: file }
    - id: repository-configuration
      adapter: declared
      path: syu.yaml
      selector: { kind: file }
    - id: workbench-contract-check
      adapter: declared
      path: scripts/ci/check-workbench-contract.py
      selector: { kind: file }
    - id: workbench-visual-check
      adapter: declared
      path: scripts/ci/workbench-visual.sh
      selector: { kind: file }
  - id: quality-gates-verification
    role: verification
    facet: repository-tooling
    responsibility: Verify that staged validation uses the Git index snapshot.
    targets:
    - id: staged-validation-test
      adapter: rust
      path: tests/v1_cli.rs
      selector:
        kind: symbol
        name: staged_change_validation_uses_the_index_snapshot
      claims:
      - kind: verifies
        criterion: REQ-WORKBENCH-010#criterion.lifecycle-separation
        covers:
        - FEAT-WORKBENCH-QUALITY-GATES-001#binding.quality-gates/target.quality-gate-script
        - FEAT-WORKBENCH-QUALITY-GATES-001#binding.quality-gates/target.release-artifacts-configuration
        - FEAT-WORKBENCH-QUALITY-GATES-001#binding.quality-gates/target.pre-commit-configuration
        - FEAT-WORKBENCH-QUALITY-GATES-001#binding.quality-gates/target.pre-commit-ci-configuration
        runner:
          runner: cargo-test-integration
          arguments:
            package: syu
            harness: v1_cli
            test: staged_change_validation_uses_the_index_snapshot

- id: FEAT-WORKBENCH-SCOPED-AGENT-001
  title: Scoped agent evidence
  summary: Expose the approved agent boundary and append-only execution evidence in Workbench.
  status: implemented
  bindings:
  - id: agent-api
    role: implementation
    facet: scoped-agent
    responsibility: Connect Workbench actions to the provider-neutral scoped agent API.
    targets:
    - id: agent-start
      adapter: rust
      path: crates/syu-workbench-server/src/lib.rs
      selector: { kind: symbol, name: api_agent_start }
      claims:
      - kind: satisfies
        criterion: REQ-WORKBENCH-011#criterion.scoped-agent
    - id: agent-patch
      adapter: rust
      path: crates/syu-workbench-server/src/lib.rs
      selector: { kind: symbol, name: api_agent_patch }
      claims:
      - kind: satisfies
        criterion: REQ-WORKBENCH-011#criterion.scoped-agent
    - id: agent-projection
      adapter: rust
      path: crates/syu-workbench-server/src/lib.rs
      selector: { kind: symbol, name: project_session }
      claims:
      - kind: satisfies
        criterion: REQ-WORKBENCH-011#criterion.scoped-agent
    - id: agent-ui
      adapter: javascript
      path: crates/syu-app-ui/assets/js/pages/work.js
      selector: { kind: symbol, name: renderWork }
      claims:
      - kind: satisfies
        criterion: REQ-WORKBENCH-011#criterion.scoped-agent
  - id: verification
    role: verification
    facet: scoped-agent
    responsibility: Verify agent rejection and evidence rendering through the Workbench server.
    targets:
    - id: agent-http-test
      adapter: rust
      path: crates/syu-workbench-server/src/lib.rs
      selector: { kind: symbol, name: tests::workbench_agent_rejects_unrelated_write_before_application }
      claims:
      - kind: verifies
        criterion: REQ-WORKBENCH-011#criterion.scoped-agent
        covers:
        - FEAT-WORKBENCH-SCOPED-AGENT-001#binding.agent-api/target.agent-start
        - FEAT-WORKBENCH-SCOPED-AGENT-001#binding.agent-api/target.agent-patch
        - FEAT-WORKBENCH-SCOPED-AGENT-001#binding.agent-api/target.agent-projection
        - FEAT-WORKBENCH-SCOPED-AGENT-001#binding.agent-api/target.agent-ui
        runner: { runner: cargo-test, arguments: { package: syu-workbench-server, test: tests::workbench_agent_rejects_unrelated_write_before_application } }
```
