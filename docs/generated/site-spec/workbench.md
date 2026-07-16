---
title: "Workbench behavior / Workbench"
description: "Generated reference for docs/syu/workbench.yaml"
---

> Generated from `docs/syu/workbench.yaml`.

## Parsed content

### Schema

- syu/spec/v1

### Kind

- requirements

### Namespace

- workbench

### Category

- Workbench behavior

### Requirements

- **id**: REQ-WORKBENCH-001
  - **title**: Canonical projection
  - **description**: The Workbench renders server-owned canonical projection state.
  - **priority**: critical
  - **status**: implemented
  - **criteria**:
    - **id**: canonical-projection
      - **kind**: behavior
      - **statement**: Projection data is read from the Workbench server without client-side semantic inference.
      - **governed_by**:
        - POL-DELIVERY-001#rule.exact-ownership
- **id**: REQ-WORKBENCH-002
  - **title**: Work session
  - **description**: A Workbench session carries a WorkRequest, canonical WorkPlan, context, verification, and result state.
  - **priority**: critical
  - **status**: implemented
  - **criteria**:
    - **id**: work-session
      - **kind**: behavior
      - **statement**: A selected work origin can be planned and validated through the canonical session API.
      - **governed_by**:
        - POL-DELIVERY-001#rule.exact-ownership
  - **bindings**:
    - **id**: work-check
      - **role**: verification
      - **facet**: verification
      - **responsibility**: Verify the Workbench session execution path.
      - **targets**:
        - **id**: work-test
          - **adapter**: rust
          - **path**: crates/syu-workbench-server/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: tests::workbench_work_session_flow
          - **claims**:
            - **kind**: verifies
              - **criterion**: REQ-WORKBENCH-002#criterion.work-session
              - **covers**:
                - FEAT-WORKBENCH-WORK-UI-001#binding.work/target.plan
              - **runner**:
                - **runner**: cargo-test
                - **arguments**:
                  - **package**: syu-workbench-server
                  - **test**: tests::workbench_work_session_flow
- **id**: REQ-WORKBENCH-003
  - **title**: Transactional specification edit
  - **description**: Structured specification edits are previewed and atomically applied only after canonical validation.
  - **priority**: high
  - **status**: implemented
  - **criteria**:
    - **id**: transactional-spec-edit
      - **kind**: behavior
      - **statement**: A stale or invalid specification edit is rejected without changing the canonical source.
      - **governed_by**:
        - POL-DELIVERY-001#rule.exact-ownership
  - **bindings**:
    - **id**: spec-edit-check
      - **role**: verification
      - **facet**: verification
      - **responsibility**: Verify transactional specification editing.
      - **targets**:
        - **id**: spec-edit-test
          - **adapter**: rust
          - **path**: crates/syu-workbench-server/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: tests::workbench_spec_edit_transaction
          - **claims**:
            - **kind**: verifies
              - **criterion**: REQ-WORKBENCH-003#criterion.transactional-spec-edit
              - **covers**:
                - FEAT-WORKBENCH-SPEC-EDITOR-001#binding.editor/target.specification-apply
              - **runner**:
                - **runner**: cargo-test
                - **arguments**:
                  - **package**: syu-workbench-server
                  - **test**: tests::workbench_spec_edit_transaction
- **id**: REQ-WORKBENCH-007
  - **title**: Guided specification selection
  - **description**: The Workbench searches typed specification candidates and previews safe human-facing edits with graph and work impact.
  - **priority**: high
  - **status**: implemented
  - **criteria**:
    - **id**: guided-specification-selection
      - **kind**: behavior
      - **statement**: A candidate search and typed specification edit preserves advanced links and requires validated preview evidence before apply.
      - **governed_by**:
        - POL-DELIVERY-001#rule.exact-ownership
  - **bindings**:
    - **id**: guided-specification-check
      - **role**: verification
      - **facet**: verification
      - **responsibility**: Verify guided specification search, preview, and typed creation.
      - **targets**:
        - **id**: guided-specification-test
          - **adapter**: rust
          - **path**: crates/syu-workbench-server/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: tests::workbench_specification_candidates_support_search_edit_and_create
          - **claims**:
            - **kind**: verifies
              - **criterion**: REQ-WORKBENCH-007#criterion.guided-specification-selection
              - **covers**:
                - FEAT-WORKBENCH-SPEC-EDITOR-001#binding.editor/target.specification-candidates
                - FEAT-WORKBENCH-SPEC-EDITOR-001#binding.editor/target.specification-candidate-preview
              - **runner**:
                - **runner**: cargo-test
                - **arguments**:
                  - **package**: syu-workbench-server
                  - **test**: tests::workbench_specification_candidates_support_search_edit_and_create
- **id**: REQ-WORKBENCH-004
  - **title**: Transactional configuration edit
  - **description**: Structured configuration edits use the same preview, basis, and atomic apply flow.
  - **priority**: high
  - **status**: implemented
  - **criteria**:
    - **id**: transactional-config-edit
      - **kind**: behavior
      - **statement**: A configuration candidate is applied only after overlay validation succeeds.
      - **governed_by**:
        - POL-DELIVERY-001#rule.exact-ownership
  - **bindings**:
    - **id**: config-edit-check
      - **role**: verification
      - **facet**: verification
      - **responsibility**: Verify transactional configuration editing.
      - **targets**:
        - **id**: config-edit-test
          - **adapter**: rust
          - **path**: crates/syu-workbench-server/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: tests::workbench_config_edit_transaction
          - **claims**:
            - **kind**: verifies
              - **criterion**: REQ-WORKBENCH-004#criterion.transactional-config-edit
              - **covers**:
                - FEAT-WORKBENCH-SPEC-EDITOR-001#binding.editor/target.config-apply
              - **runner**:
                - **runner**: cargo-test
                - **arguments**:
                  - **package**: syu-workbench-server
                  - **test**: tests::workbench_config_edit_transaction
- **id**: REQ-WORKBENCH-008
  - **title**: Reviewed target suggestions
  - **description**: The Workbench ranks exact implementation and verification candidates with human-readable evidence before any executable scope is created.
  - **priority**: high
  - **status**: implemented
  - **criteria**:
    - **id**: reviewed-target-suggestions
      - **kind**: behavior
      - **statement**: Suggestions remain advisory until explicit approval, rejected evidence stays suppressed, approved candidates become exact WorkRequest targets, and budget overflow recommends split work.
      - **governed_by**:
        - POL-DELIVERY-001#rule.exact-ownership
  - **bindings**:
    - **id**: target-suggestion-check
      - **role**: verification
      - **facet**: verification
      - **responsibility**: Verify advisory suggestions, evidence-bound rejection, and exact approval.
      - **targets**:
        - **id**: target-suggestion-test
          - **adapter**: rust
          - **path**: crates/syu-workbench-server/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: tests::target_suggestions_require_review_before_exact_work_request
          - **claims**:
            - **kind**: verifies
              - **criterion**: REQ-WORKBENCH-008#criterion.reviewed-target-suggestions
              - **covers**:
                - FEAT-WORKBENCH-TARGET-SUGGESTIONS-001#binding.suggestions/target.rank-candidates
                - FEAT-WORKBENCH-TARGET-SUGGESTIONS-001#binding.suggestions/target.approve-candidates
                - FEAT-WORKBENCH-TARGET-SUGGESTIONS-001#binding.suggestions/target.suggestion-review-ui
              - **runner**:
                - **runner**: cargo-test
                - **arguments**:
                  - **package**: syu-workbench-server
                  - **test**: tests::target_suggestions_require_review_before_exact_work_request
- **id**: REQ-WORKBENCH-005
  - **title**: Secure local server
  - **description**: The Workbench server is loopback-first and protects mutation endpoints.
  - **priority**: critical
  - **status**: implemented
  - **criteria**:
    - **id**: secure-local-server
      - **kind**: security
      - **statement**: Remote bind requires a session token and mutations require CSRF and origin checks.
      - **governed_by**:
        - POL-DELIVERY-001#rule.exact-ownership
  - **bindings**:
    - **id**: security-check
      - **role**: verification
      - **facet**: verification
      - **responsibility**: Verify secure Workbench server launch and mutation guards.
      - **targets**:
        - **id**: security-test
          - **adapter**: rust
          - **path**: crates/syu-workbench-server/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: tests::workbench_security_flow
          - **claims**:
            - **kind**: verifies
              - **criterion**: REQ-WORKBENCH-005#criterion.secure-local-server
              - **covers**:
                - FEAT-WORKBENCH-SERVER-001#binding.server/target.mutation-guard
              - **runner**:
                - **runner**: cargo-test
                - **arguments**:
                  - **package**: syu-workbench-server
                  - **test**: tests::workbench_security_flow
- **id**: REQ-WORKBENCH-006
  - **title**: Accessible navigation
  - **description**: Workbench primary pages and actions are keyboard navigable and localized.
  - **priority**: medium
  - **status**: implemented
  - **criteria**:
    - **id**: accessible-navigation
      - **kind**: quality
      - **statement**: The primary Workbench pages expose semantic controls and matching Japanese and English catalogs.
      - **governed_by**:
        - POL-DELIVERY-001#rule.exact-ownership
  - **bindings**:
    - **id**: navigation-check
      - **role**: verification
      - **facet**: verification
      - **responsibility**: Verify accessible Workbench navigation and localization.
      - **targets**:
        - **id**: navigation-test
          - **adapter**: rust
          - **path**: crates/syu-workbench-server/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: tests::workbench_accessible_navigation
          - **claims**:
            - **kind**: verifies
              - **criterion**: REQ-WORKBENCH-006#criterion.accessible-navigation
              - **covers**:
                - FEAT-WORKBENCH-NAVIGATION-001#binding.navigation/target.html-navigation
                - FEAT-WORKBENCH-NAVIGATION-001#binding.navigation/target.javascript-navigation
                - FEAT-WORKBENCH-NAVIGATION-001#binding.navigation/target.keyboard-navigation
                - FEAT-WORKBENCH-NAVIGATION-001#binding.navigation/target.accessibility-attributes
                - FEAT-WORKBENCH-NAVIGATION-001#binding.navigation/target.workbench-css
              - **runner**:
                - **runner**: cargo-test
                - **arguments**:
                  - **package**: syu-workbench-server
                  - **test**: tests::workbench_accessible_navigation
- **id**: REQ-WORKBENCH-009
  - **title**: Durable completion history
  - **description**: Workbench renders the shared completion attempt history and finalization state from the server-owned store.
  - **priority**: critical
  - **status**: implemented
  - **criteria**:
    - **id**: completion-history
      - **kind**: behavior
      - **statement**: Current and previous attempts expose plan and slice identity, status, blockers, next action, demonstrated criteria, and finalized state.
      - **governed_by**:
        - POL-DELIVERY-001#rule.exact-ownership
- **id**: REQ-WORKBENCH-010
  - **title**: Layered quality gates
  - **description**: Local hooks and CI apply proportional quality checks without weakening the complete release gate.
  - **priority**: high
  - **status**: implemented
  - **criteria**:
    - **id**: lifecycle-separation
      - **kind**: quality
      - **statement**: Pre-commit validates the staged snapshot, pre-push scopes fast Rust and specification checks to the push diff, and CI runs the complete workspace gate.
      - **governed_by**:
        - POL-DELIVERY-001#rule.exact-ownership

## Source YAML

```yaml
schema: syu/spec/v1
kind: requirements
namespace: workbench
category: Workbench behavior
requirements:
  - id: REQ-WORKBENCH-001
    title: Canonical projection
    description: The Workbench renders server-owned canonical projection state.
    priority: critical
    status: implemented
    criteria:
      - id: canonical-projection
        kind: behavior
        statement: Projection data is read from the Workbench server without client-side semantic inference.
        governed_by: [POL-DELIVERY-001#rule.exact-ownership]
  - id: REQ-WORKBENCH-002
    title: Work session
    description: A Workbench session carries a WorkRequest, canonical WorkPlan, context, verification, and result state.
    priority: critical
    status: implemented
    criteria:
      - id: work-session
        kind: behavior
        statement: A selected work origin can be planned and validated through the canonical session API.
        governed_by: [POL-DELIVERY-001#rule.exact-ownership]
    bindings:
      - id: work-check
        role: verification
        facet: verification
        responsibility: Verify the Workbench session execution path.
        targets:
          - id: work-test
            adapter: rust
            path: crates/syu-workbench-server/src/lib.rs
            selector: { kind: symbol, name: tests::workbench_work_session_flow }
            claims:
              - kind: verifies
                criterion: REQ-WORKBENCH-002#criterion.work-session
                covers: [FEAT-WORKBENCH-WORK-UI-001#binding.work/target.plan]
                runner: { runner: cargo-test, arguments: { package: syu-workbench-server, test: tests::workbench_work_session_flow } }
  - id: REQ-WORKBENCH-003
    title: Transactional specification edit
    description: Structured specification edits are previewed and atomically applied only after canonical validation.
    priority: high
    status: implemented
    criteria:
      - id: transactional-spec-edit
        kind: behavior
        statement: A stale or invalid specification edit is rejected without changing the canonical source.
        governed_by: [POL-DELIVERY-001#rule.exact-ownership]
    bindings:
      - id: spec-edit-check
        role: verification
        facet: verification
        responsibility: Verify transactional specification editing.
        targets:
          - id: spec-edit-test
            adapter: rust
            path: crates/syu-workbench-server/src/lib.rs
            selector: { kind: symbol, name: tests::workbench_spec_edit_transaction }
            claims:
              - kind: verifies
                criterion: REQ-WORKBENCH-003#criterion.transactional-spec-edit
                covers: [FEAT-WORKBENCH-SPEC-EDITOR-001#binding.editor/target.specification-apply]
                runner: { runner: cargo-test, arguments: { package: syu-workbench-server, test: tests::workbench_spec_edit_transaction } }
  - id: REQ-WORKBENCH-007
    title: Guided specification selection
    description: The Workbench searches typed specification candidates and previews safe human-facing edits with graph and work impact.
    priority: high
    status: implemented
    criteria:
      - id: guided-specification-selection
        kind: behavior
        statement: A candidate search and typed specification edit preserves advanced links and requires validated preview evidence before apply.
        governed_by: [POL-DELIVERY-001#rule.exact-ownership]
    bindings:
      - id: guided-specification-check
        role: verification
        facet: verification
        responsibility: Verify guided specification search, preview, and typed creation.
        targets:
          - id: guided-specification-test
            adapter: rust
            path: crates/syu-workbench-server/src/lib.rs
            selector: { kind: symbol, name: tests::workbench_specification_candidates_support_search_edit_and_create }
            claims:
              - kind: verifies
                criterion: REQ-WORKBENCH-007#criterion.guided-specification-selection
                covers:
                  - FEAT-WORKBENCH-SPEC-EDITOR-001#binding.editor/target.specification-candidates
                  - FEAT-WORKBENCH-SPEC-EDITOR-001#binding.editor/target.specification-candidate-preview
                runner: { runner: cargo-test, arguments: { package: syu-workbench-server, test: tests::workbench_specification_candidates_support_search_edit_and_create } }
  - id: REQ-WORKBENCH-004
    title: Transactional configuration edit
    description: Structured configuration edits use the same preview, basis, and atomic apply flow.
    priority: high
    status: implemented
    criteria:
      - id: transactional-config-edit
        kind: behavior
        statement: A configuration candidate is applied only after overlay validation succeeds.
        governed_by: [POL-DELIVERY-001#rule.exact-ownership]
    bindings:
      - id: config-edit-check
        role: verification
        facet: verification
        responsibility: Verify transactional configuration editing.
        targets:
          - id: config-edit-test
            adapter: rust
            path: crates/syu-workbench-server/src/lib.rs
            selector: { kind: symbol, name: tests::workbench_config_edit_transaction }
            claims:
              - kind: verifies
                criterion: REQ-WORKBENCH-004#criterion.transactional-config-edit
                covers: [FEAT-WORKBENCH-SPEC-EDITOR-001#binding.editor/target.config-apply]
                runner: { runner: cargo-test, arguments: { package: syu-workbench-server, test: tests::workbench_config_edit_transaction } }
  - id: REQ-WORKBENCH-008
    title: Reviewed target suggestions
    description: The Workbench ranks exact implementation and verification candidates with human-readable evidence before any executable scope is created.
    priority: high
    status: implemented
    criteria:
      - id: reviewed-target-suggestions
        kind: behavior
        statement: Suggestions remain advisory until explicit approval, rejected evidence stays suppressed, approved candidates become exact WorkRequest targets, and budget overflow recommends split work.
        governed_by: [POL-DELIVERY-001#rule.exact-ownership]
    bindings:
      - id: target-suggestion-check
        role: verification
        facet: verification
        responsibility: Verify advisory suggestions, evidence-bound rejection, and exact approval.
        targets:
          - id: target-suggestion-test
            adapter: rust
            path: crates/syu-workbench-server/src/lib.rs
            selector: { kind: symbol, name: tests::target_suggestions_require_review_before_exact_work_request }
            claims:
              - kind: verifies
                criterion: REQ-WORKBENCH-008#criterion.reviewed-target-suggestions
                covers:
                  - FEAT-WORKBENCH-TARGET-SUGGESTIONS-001#binding.suggestions/target.rank-candidates
                  - FEAT-WORKBENCH-TARGET-SUGGESTIONS-001#binding.suggestions/target.approve-candidates
                  - FEAT-WORKBENCH-TARGET-SUGGESTIONS-001#binding.suggestions/target.suggestion-review-ui
                runner: { runner: cargo-test, arguments: { package: syu-workbench-server, test: tests::target_suggestions_require_review_before_exact_work_request } }
  - id: REQ-WORKBENCH-005
    title: Secure local server
    description: The Workbench server is loopback-first and protects mutation endpoints.
    priority: critical
    status: implemented
    criteria:
      - id: secure-local-server
        kind: security
        statement: Remote bind requires a session token and mutations require CSRF and origin checks.
        governed_by: [POL-DELIVERY-001#rule.exact-ownership]
    bindings:
      - id: security-check
        role: verification
        facet: verification
        responsibility: Verify secure Workbench server launch and mutation guards.
        targets:
          - id: security-test
            adapter: rust
            path: crates/syu-workbench-server/src/lib.rs
            selector: { kind: symbol, name: tests::workbench_security_flow }
            claims:
              - kind: verifies
                criterion: REQ-WORKBENCH-005#criterion.secure-local-server
                covers: [FEAT-WORKBENCH-SERVER-001#binding.server/target.mutation-guard]
                runner: { runner: cargo-test, arguments: { package: syu-workbench-server, test: tests::workbench_security_flow } }
  - id: REQ-WORKBENCH-006
    title: Accessible navigation
    description: Workbench primary pages and actions are keyboard navigable and localized.
    priority: medium
    status: implemented
    criteria:
      - id: accessible-navigation
        kind: quality
        statement: The primary Workbench pages expose semantic controls and matching Japanese and English catalogs.
        governed_by: [POL-DELIVERY-001#rule.exact-ownership]
    bindings:
      - id: navigation-check
        role: verification
        facet: verification
        responsibility: Verify accessible Workbench navigation and localization.
        targets:
          - id: navigation-test
            adapter: rust
            path: crates/syu-workbench-server/src/lib.rs
            selector: { kind: symbol, name: tests::workbench_accessible_navigation }
            claims:
              - kind: verifies
                criterion: REQ-WORKBENCH-006#criterion.accessible-navigation
                covers:
                  - FEAT-WORKBENCH-NAVIGATION-001#binding.navigation/target.html-navigation
                  - FEAT-WORKBENCH-NAVIGATION-001#binding.navigation/target.javascript-navigation
                  - FEAT-WORKBENCH-NAVIGATION-001#binding.navigation/target.keyboard-navigation
                  - FEAT-WORKBENCH-NAVIGATION-001#binding.navigation/target.accessibility-attributes
                  - FEAT-WORKBENCH-NAVIGATION-001#binding.navigation/target.workbench-css
                runner: { runner: cargo-test, arguments: { package: syu-workbench-server, test: tests::workbench_accessible_navigation } }
  - id: REQ-WORKBENCH-009
    title: Durable completion history
    description: Workbench renders the shared completion attempt history and finalization state from the server-owned store.
    priority: critical
    status: implemented
    criteria:
      - id: completion-history
        kind: behavior
        statement: Current and previous attempts expose plan and slice identity, status, blockers, next action, demonstrated criteria, and finalized state.
        governed_by: [POL-DELIVERY-001#rule.exact-ownership]
  - id: REQ-WORKBENCH-010
    title: Layered quality gates
    description: Local hooks and CI apply proportional quality checks without weakening the complete release gate.
    priority: high
    status: implemented
    criteria:
      - id: lifecycle-separation
        kind: quality
        statement: Pre-commit validates the staged snapshot, pre-push scopes fast Rust and specification checks to the push diff, and CI runs the complete workspace gate.
        governed_by: [POL-DELIVERY-001#rule.exact-ownership]
```
