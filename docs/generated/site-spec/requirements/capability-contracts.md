---
title: "Capability acceptance contracts / Capability Contracts"
description: "Generated reference for docs/syu/requirements/capability-contracts.yaml"
---

> Generated from `docs/syu/requirements/capability-contracts.yaml`.

## Parsed content

### Schema

- syu/spec/v1

### Kind

- requirements

### Namespace

- capabilities

### Category

- Capability acceptance contracts

### Requirements

- **id**: REQ-CAPABILITY-001
  - **title**: Canonical capability behavior
  - **description**: Each implemented core capability exposes one exact behavior boundary with executable evidence.
  - **priority**: critical
  - **status**: implemented
  - **criteria**:
    - **id**: spec-model
      - **kind**: behavior
      - **statement**: Canonical specification documents accept the v1 shape and reject obsolete or ambiguous identities.
      - **governed_by**:
        - POL-ADOPTION-001#rule.feature-evidence
    - **id**: project-config
      - **kind**: behavior
      - **statement**: Project configuration preserves explicit inventory, readiness, verification, and work limits while rejecting unknown fields.
      - **governed_by**:
        - POL-ADOPTION-001#rule.feature-evidence
    - **id**: inventory-discovery
      - **kind**: behavior
      - **statement**: The active inventory profile discovers and unions configured providers with stable public and support exposure.
      - **governed_by**:
        - POL-ADOPTION-001#rule.feature-evidence
    - **id**: exact-identity
      - **kind**: behavior
      - **statement**: An exact selector resolves one artifact identity and rejects ambiguous headings, markers, or symbols.
      - **governed_by**:
        - POL-ADOPTION-001#rule.feature-evidence
    - **id**: graph-index
      - **kind**: behavior
      - **statement**: The workspace index loads configured specification roots and derives exact graph and target relations.
      - **governed_by**:
        - POL-ADOPTION-001#rule.feature-evidence
    - **id**: ownership-resolution
      - **kind**: behavior
      - **statement**: Current ownership excludes planned Features and resolves each active governed artifact to one exact owner.
      - **governed_by**:
        - POL-ADOPTION-001#rule.active-status
        - POL-ADOPTION-001#rule.feature-evidence
    - **id**: readiness-ladder
      - **kind**: quality
      - **statement**: Readiness advances monotonically through traceable, seedable, work-ready, verifiable, and closed-loop evidence.
      - **governed_by**:
        - POL-ADOPTION-001#rule.bounded-rollout
    - **id**: change-validation
      - **kind**: security
      - **statement**: Changed semantic artifacts are rejected when ownership, lifecycle, or approved plan scope is incomplete.
      - **governed_by**:
        - POL-ADOPTION-001#rule.feature-evidence
    - **id**: context-export
      - **kind**: behavior
      - **statement**: A selected plan slice exports bounded target and support context without exceeding configured budgets.
      - **governed_by**:
        - POL-ADOPTION-001#rule.feature-evidence
    - **id**: contract-closure
      - **kind**: behavior
      - **statement**: Contract sources, participants, and guarantees retain exact typed references.
      - **governed_by**:
        - POL-ADOPTION-001#rule.feature-evidence
    - **id**: verification-execution
      - **kind**: behavior
      - **statement**: Verification executes only a configured exact test and rejects zero-match or unprovable runs.
      - **governed_by**:
        - POL-ADOPTION-001#rule.feature-evidence
    - **id**: plan-validation
      - **kind**: security
      - **statement**: Execution reconstructs the exact plan basis and fails closed when its configuration or inventory cannot be restored.
      - **governed_by**:
        - POL-ADOPTION-001#rule.feature-evidence
    - **id**: receipt-validation
      - **kind**: behavior
      - **statement**: Completion rejects receipts and post-state evidence that do not prove the selected exact slice.
      - **governed_by**:
        - POL-ADOPTION-001#rule.feature-evidence
    - **id**: cli-orchestration
      - **kind**: behavior
      - **statement**: The CLI applies the canonical workspace validator and reports configured readiness through stable commands.
      - **governed_by**:
        - POL-ADOPTION-001#rule.bounded-rollout
    - **id**: lsp-navigation
      - **kind**: behavior
      - **statement**: Language-server hover resolves each specification layer from the canonical workspace.
      - **governed_by**:
        - POL-ADOPTION-001#rule.feature-evidence
    - **id**: docs-generation
      - **kind**: documentation
      - **statement**: Every canonical specification source has a current generated reference page and index entry.
      - **governed_by**:
        - POL-ADOPTION-001#rule.feature-evidence
  - **bindings**:
    - **id**: core-verification
      - **role**: verification
      - **facet**: capability-verification
      - **responsibility**: Verify core specification, configuration, inventory, identity, index, ownership, readiness, change, context, and contract behavior.
      - **targets**:
        - **id**: spec-model-test
          - **adapter**: rust
          - **path**: crates/syu-spec-model/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: tests::old_shape_is_rejected
          - **claims**:
            - **kind**: verifies
              - **criterion**: REQ-CAPABILITY-001#criterion.spec-model
              - **covers**:
                - FEAT-SPEC-MODEL-001#binding.implementation/target.spec-document
              - **runner**:
                - **runner**: cargo-test
                - **arguments**:
                  - **package**: syu-spec-model
                  - **test**: tests::old_shape_is_rejected
        - **id**: project-config-test
          - **adapter**: rust
          - **path**: crates/syu-project-model/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: tests::project_config_preserves_readiness_configuration
          - **claims**:
            - **kind**: verifies
              - **criterion**: REQ-CAPABILITY-001#criterion.project-config
              - **covers**:
                - FEAT-PROJECT-CONFIG-001#binding.implementation/target.project-config
              - **runner**:
                - **runner**: cargo-test
                - **arguments**:
                  - **package**: syu-project-model
                  - **test**: tests::project_config_preserves_readiness_configuration
        - **id**: inventory-test
          - **adapter**: rust
          - **path**: crates/syu-inventory/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: tests::active_profile_unions_rust_and_javascript_providers
          - **claims**:
            - **kind**: verifies
              - **criterion**: REQ-CAPABILITY-001#criterion.inventory-discovery
              - **covers**:
                - FEAT-INVENTORY-001#binding.implementation/target.inventory-registry
              - **runner**:
                - **runner**: cargo-test
                - **arguments**:
                  - **package**: syu-inventory
                  - **test**: tests::active_profile_unions_rust_and_javascript_providers
        - **id**: identity-test
          - **adapter**: rust
          - **path**: crates/syu-workspace/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: tests::heading_selectors_reject_ambiguity
          - **claims**:
            - **kind**: verifies
              - **criterion**: REQ-CAPABILITY-001#criterion.exact-identity
              - **covers**:
                - FEAT-IDENTITY-001#binding.implementation/target.target-resolver
              - **runner**:
                - **runner**: cargo-test
                - **arguments**:
                  - **package**: syu-workspace
                  - **test**: tests::heading_selectors_reject_ambiguity
        - **id**: index-test
          - **adapter**: rust
          - **path**: crates/syu-workspace/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: tests::nested_spec_directories_load_from_noncanonical_workspace_roots
          - **claims**:
            - **kind**: verifies
              - **criterion**: REQ-CAPABILITY-001#criterion.graph-index
              - **covers**:
                - FEAT-INDEX-001#binding.implementation/target.spec-index
              - **runner**:
                - **runner**: cargo-test
                - **arguments**:
                  - **package**: syu-workspace
                  - **test**: tests::nested_spec_directories_load_from_noncanonical_workspace_roots
        - **id**: ownership-test
          - **adapter**: rust
          - **path**: crates/syu-workspace/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: tests::planned_features_do_not_own_active_artifacts
          - **claims**:
            - **kind**: verifies
              - **criterion**: REQ-CAPABILITY-001#criterion.ownership-resolution
              - **covers**:
                - FEAT-OWNERSHIP-001#binding.implementation/target.ownership-ref
              - **runner**:
                - **runner**: cargo-test
                - **arguments**:
                  - **package**: syu-workspace
                  - **test**: tests::planned_features_do_not_own_active_artifacts
        - **id**: readiness-test
          - **adapter**: rust
          - **path**: crates/syu-validation/src/readiness.rs
          - **selector**:
            - **kind**: symbol
            - **name**: tests::readiness_levels_add_axes_monotonically
          - **claims**:
            - **kind**: verifies
              - **criterion**: REQ-CAPABILITY-001#criterion.readiness-ladder
              - **covers**:
                - FEAT-READINESS-001#binding.implementation/target.readiness-evaluate
              - **runner**:
                - **runner**: cargo-test
                - **arguments**:
                  - **package**: syu-validation
                  - **test**: readiness::tests::readiness_levels_add_axes_monotonically
        - **id**: change-validation-test
          - **adapter**: rust
          - **path**: crates/syu-validation/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: tests::hunkless_changes_require_file_scope_for_editable_targets
          - **claims**:
            - **kind**: verifies
              - **criterion**: REQ-CAPABILITY-001#criterion.change-validation
              - **covers**:
                - FEAT-CHANGE-VALIDATION-001#binding.implementation/target.change-validate
              - **runner**:
                - **runner**: cargo-test
                - **arguments**:
                  - **package**: syu-validation
                  - **test**: tests::hunkless_changes_require_file_scope_for_editable_targets
        - **id**: context-test
          - **adapter**: rust
          - **path**: crates/syu-planner/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: tests::context_pack_distinguishes_target_and_support_entries_for_missing_targets
          - **claims**:
            - **kind**: verifies
              - **criterion**: REQ-CAPABILITY-001#criterion.context-export
              - **covers**:
                - FEAT-CONTEXT-001#binding.implementation/target.context-export
              - **runner**:
                - **runner**: cargo-test
                - **arguments**:
                  - **package**: syu-planner
                  - **test**: tests::context_pack_distinguishes_target_and_support_entries_for_missing_targets
        - **id**: contract-test
          - **adapter**: rust
          - **path**: crates/syu-spec-model/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: tests::binding_level_relations_and_non_target_contract_refs_are_rejected
          - **claims**:
            - **kind**: verifies
              - **criterion**: REQ-CAPABILITY-001#criterion.contract-closure
              - **covers**:
                - FEAT-CONTRACT-001#binding.implementation/target.contract-model
              - **runner**:
                - **runner**: cargo-test
                - **arguments**:
                  - **package**: syu-spec-model
                  - **test**: tests::binding_level_relations_and_non_target_contract_refs_are_rejected
    - **id**: delivery-verification
      - **role**: verification
      - **facet**: capability-verification
      - **responsibility**: Verify exact execution, basis reconstruction, receipt closure, CLI orchestration, LSP navigation, and generated documentation.
      - **targets**:
        - **id**: verification-test
          - **adapter**: rust
          - **path**: crates/syu-validation/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: tests::exact_test_execution_requires_match
          - **claims**:
            - **kind**: verifies
              - **criterion**: REQ-CAPABILITY-001#criterion.verification-execution
              - **covers**:
                - FEAT-VERIFICATION-001#binding.implementation/target.execute-verification
              - **runner**:
                - **runner**: cargo-test
                - **arguments**:
                  - **package**: syu-validation
                  - **test**: tests::exact_test_execution_requires_match
        - **id**: plan-validation-test
          - **adapter**: rust
          - **path**: crates/syu-validation/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: tests::canonical_execution_reconstructs_basis_after_editable_change
          - **claims**:
            - **kind**: verifies
              - **criterion**: REQ-CAPABILITY-001#criterion.plan-validation
              - **covers**:
                - FEAT-PLAN-VALIDATION-001#binding.implementation/target.canonical-plan-validation
              - **runner**:
                - **runner**: cargo-test
                - **arguments**:
                  - **package**: syu-validation
                  - **test**: tests::canonical_execution_reconstructs_basis_after_editable_change
        - **id**: receipt-validation-test
          - **adapter**: rust
          - **path**: crates/syu-validation/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: tests::completion_report_rejects_unchanged_modify_target
          - **claims**:
            - **kind**: verifies
              - **criterion**: REQ-CAPABILITY-001#criterion.receipt-validation
              - **covers**:
                - FEAT-RESULT-VALIDATION-001#binding.implementation/target.receipt-validation
              - **runner**:
                - **runner**: cargo-test
                - **arguments**:
                  - **package**: syu-validation
                  - **test**: tests::completion_report_rejects_unchanged_modify_target
        - **id**: cli-test
          - **adapter**: rust
          - **path**: tests/v1_cli.rs
          - **selector**:
            - **kind**: symbol
            - **name**: current_workspace_validates_and_reports_configured_readiness
          - **claims**:
            - **kind**: verifies
              - **criterion**: REQ-CAPABILITY-001#criterion.cli-orchestration
              - **covers**:
                - FEAT-CLI-001#binding.implementation/target.cli-run
              - **runner**:
                - **runner**: cargo-test-integration
                - **arguments**:
                  - **package**: syu
                  - **harness**: v1_cli
                  - **test**: current_workspace_validates_and_reports_configured_readiness
        - **id**: lsp-test
          - **adapter**: rust
          - **path**: src/lsp/handlers.rs
          - **selector**:
            - **kind**: symbol
            - **name**: tests::handle_hover_renders_each_spec_layer
          - **claims**:
            - **kind**: verifies
              - **criterion**: REQ-CAPABILITY-001#criterion.lsp-navigation
              - **covers**:
                - FEAT-LSP-001#binding.implementation/target.lsp-server
              - **runner**:
                - **runner**: cargo-test
                - **arguments**:
                  - **package**: syu
                  - **test**: lsp::handlers::tests::handle_hover_renders_each_spec_layer
        - **id**: docs-test
          - **adapter**: rust
          - **path**: tests/v1_cli.rs
          - **selector**:
            - **kind**: symbol
            - **name**: generated_spec_reference_covers_every_source_document
          - **claims**:
            - **kind**: verifies
              - **criterion**: REQ-CAPABILITY-001#criterion.docs-generation
              - **covers**:
                - FEAT-DOCS-001#binding.implementation/target.generated-index
              - **runner**:
                - **runner**: cargo-test-integration
                - **arguments**:
                  - **package**: syu
                  - **harness**: v1_cli
                  - **test**: generated_spec_reference_covers_every_source_document

## Source YAML

```yaml
schema: syu/spec/v1
kind: requirements
namespace: capabilities
category: Capability acceptance contracts
requirements:
  - id: REQ-CAPABILITY-001
    title: Canonical capability behavior
    description: Each implemented core capability exposes one exact behavior boundary with executable evidence.
    priority: critical
    status: implemented
    criteria:
      - id: spec-model
        kind: behavior
        statement: Canonical specification documents accept the v1 shape and reject obsolete or ambiguous identities.
        governed_by: [POL-ADOPTION-001#rule.feature-evidence]
      - id: project-config
        kind: behavior
        statement: Project configuration preserves explicit inventory, readiness, verification, and work limits while rejecting unknown fields.
        governed_by: [POL-ADOPTION-001#rule.feature-evidence]
      - id: inventory-discovery
        kind: behavior
        statement: The active inventory profile discovers and unions configured providers with stable public and support exposure.
        governed_by: [POL-ADOPTION-001#rule.feature-evidence]
      - id: exact-identity
        kind: behavior
        statement: An exact selector resolves one artifact identity and rejects ambiguous headings, markers, or symbols.
        governed_by: [POL-ADOPTION-001#rule.feature-evidence]
      - id: graph-index
        kind: behavior
        statement: The workspace index loads configured specification roots and derives exact graph and target relations.
        governed_by: [POL-ADOPTION-001#rule.feature-evidence]
      - id: ownership-resolution
        kind: behavior
        statement: Current ownership excludes planned Features and resolves each active governed artifact to one exact owner.
        governed_by: [POL-ADOPTION-001#rule.active-status, POL-ADOPTION-001#rule.feature-evidence]
      - id: readiness-ladder
        kind: quality
        statement: Readiness advances monotonically through traceable, seedable, work-ready, verifiable, and closed-loop evidence.
        governed_by: [POL-ADOPTION-001#rule.bounded-rollout]
      - id: change-validation
        kind: security
        statement: Changed semantic artifacts are rejected when ownership, lifecycle, or approved plan scope is incomplete.
        governed_by: [POL-ADOPTION-001#rule.feature-evidence]
      - id: context-export
        kind: behavior
        statement: A selected plan slice exports bounded target and support context without exceeding configured budgets.
        governed_by: [POL-ADOPTION-001#rule.feature-evidence]
      - id: contract-closure
        kind: behavior
        statement: Contract sources, participants, and guarantees retain exact typed references.
        governed_by: [POL-ADOPTION-001#rule.feature-evidence]
      - id: verification-execution
        kind: behavior
        statement: Verification executes only a configured exact test and rejects zero-match or unprovable runs.
        governed_by: [POL-ADOPTION-001#rule.feature-evidence]
      - id: plan-validation
        kind: security
        statement: Execution reconstructs the exact plan basis and fails closed when its configuration or inventory cannot be restored.
        governed_by: [POL-ADOPTION-001#rule.feature-evidence]
      - id: receipt-validation
        kind: behavior
        statement: Completion rejects receipts and post-state evidence that do not prove the selected exact slice.
        governed_by: [POL-ADOPTION-001#rule.feature-evidence]
      - id: cli-orchestration
        kind: behavior
        statement: The CLI applies the canonical workspace validator and reports configured readiness through stable commands.
        governed_by: [POL-ADOPTION-001#rule.bounded-rollout]
      - id: lsp-navigation
        kind: behavior
        statement: Language-server hover resolves each specification layer from the canonical workspace.
        governed_by: [POL-ADOPTION-001#rule.feature-evidence]
      - id: docs-generation
        kind: documentation
        statement: Every canonical specification source has a current generated reference page and index entry.
        governed_by: [POL-ADOPTION-001#rule.feature-evidence]
    bindings:
      - id: core-verification
        role: verification
        facet: capability-verification
        responsibility: Verify core specification, configuration, inventory, identity, index, ownership, readiness, change, context, and contract behavior.
        targets:
          - id: spec-model-test
            adapter: rust
            path: crates/syu-spec-model/src/lib.rs
            selector: { kind: symbol, name: tests::old_shape_is_rejected }
            claims:
              - kind: verifies
                criterion: REQ-CAPABILITY-001#criterion.spec-model
                covers: [FEAT-SPEC-MODEL-001#binding.implementation/target.spec-document]
                runner: { runner: cargo-test, arguments: { package: syu-spec-model, test: tests::old_shape_is_rejected } }
          - id: project-config-test
            adapter: rust
            path: crates/syu-project-model/src/lib.rs
            selector: { kind: symbol, name: tests::project_config_preserves_readiness_configuration }
            claims:
              - kind: verifies
                criterion: REQ-CAPABILITY-001#criterion.project-config
                covers: [FEAT-PROJECT-CONFIG-001#binding.implementation/target.project-config]
                runner: { runner: cargo-test, arguments: { package: syu-project-model, test: tests::project_config_preserves_readiness_configuration } }
          - id: inventory-test
            adapter: rust
            path: crates/syu-inventory/src/lib.rs
            selector: { kind: symbol, name: tests::active_profile_unions_rust_and_javascript_providers }
            claims:
              - kind: verifies
                criterion: REQ-CAPABILITY-001#criterion.inventory-discovery
                covers: [FEAT-INVENTORY-001#binding.implementation/target.inventory-registry]
                runner: { runner: cargo-test, arguments: { package: syu-inventory, test: tests::active_profile_unions_rust_and_javascript_providers } }
          - id: identity-test
            adapter: rust
            path: crates/syu-workspace/src/lib.rs
            selector: { kind: symbol, name: tests::heading_selectors_reject_ambiguity }
            claims:
              - kind: verifies
                criterion: REQ-CAPABILITY-001#criterion.exact-identity
                covers: [FEAT-IDENTITY-001#binding.implementation/target.target-resolver]
                runner: { runner: cargo-test, arguments: { package: syu-workspace, test: tests::heading_selectors_reject_ambiguity } }
          - id: index-test
            adapter: rust
            path: crates/syu-workspace/src/lib.rs
            selector: { kind: symbol, name: tests::nested_spec_directories_load_from_noncanonical_workspace_roots }
            claims:
              - kind: verifies
                criterion: REQ-CAPABILITY-001#criterion.graph-index
                covers: [FEAT-INDEX-001#binding.implementation/target.spec-index]
                runner: { runner: cargo-test, arguments: { package: syu-workspace, test: tests::nested_spec_directories_load_from_noncanonical_workspace_roots } }
          - id: ownership-test
            adapter: rust
            path: crates/syu-workspace/src/lib.rs
            selector: { kind: symbol, name: tests::planned_features_do_not_own_active_artifacts }
            claims:
              - kind: verifies
                criterion: REQ-CAPABILITY-001#criterion.ownership-resolution
                covers: [FEAT-OWNERSHIP-001#binding.implementation/target.ownership-ref]
                runner: { runner: cargo-test, arguments: { package: syu-workspace, test: tests::planned_features_do_not_own_active_artifacts } }
          - id: readiness-test
            adapter: rust
            path: crates/syu-validation/src/readiness.rs
            selector: { kind: symbol, name: tests::readiness_levels_add_axes_monotonically }
            claims:
              - kind: verifies
                criterion: REQ-CAPABILITY-001#criterion.readiness-ladder
                covers: [FEAT-READINESS-001#binding.implementation/target.readiness-evaluate]
                runner: { runner: cargo-test, arguments: { package: syu-validation, test: readiness::tests::readiness_levels_add_axes_monotonically } }
          - id: change-validation-test
            adapter: rust
            path: crates/syu-validation/src/lib.rs
            selector: { kind: symbol, name: tests::hunkless_changes_require_file_scope_for_editable_targets }
            claims:
              - kind: verifies
                criterion: REQ-CAPABILITY-001#criterion.change-validation
                covers: [FEAT-CHANGE-VALIDATION-001#binding.implementation/target.change-validate]
                runner: { runner: cargo-test, arguments: { package: syu-validation, test: tests::hunkless_changes_require_file_scope_for_editable_targets } }
          - id: context-test
            adapter: rust
            path: crates/syu-planner/src/lib.rs
            selector: { kind: symbol, name: tests::context_pack_distinguishes_target_and_support_entries_for_missing_targets }
            claims:
              - kind: verifies
                criterion: REQ-CAPABILITY-001#criterion.context-export
                covers: [FEAT-CONTEXT-001#binding.implementation/target.context-export]
                runner: { runner: cargo-test, arguments: { package: syu-planner, test: tests::context_pack_distinguishes_target_and_support_entries_for_missing_targets } }
          - id: contract-test
            adapter: rust
            path: crates/syu-spec-model/src/lib.rs
            selector: { kind: symbol, name: tests::binding_level_relations_and_non_target_contract_refs_are_rejected }
            claims:
              - kind: verifies
                criterion: REQ-CAPABILITY-001#criterion.contract-closure
                covers: [FEAT-CONTRACT-001#binding.implementation/target.contract-model]
                runner: { runner: cargo-test, arguments: { package: syu-spec-model, test: tests::binding_level_relations_and_non_target_contract_refs_are_rejected } }
      - id: delivery-verification
        role: verification
        facet: capability-verification
        responsibility: Verify exact execution, basis reconstruction, receipt closure, CLI orchestration, LSP navigation, and generated documentation.
        targets:
          - id: verification-test
            adapter: rust
            path: crates/syu-validation/src/lib.rs
            selector: { kind: symbol, name: tests::exact_test_execution_requires_match }
            claims:
              - kind: verifies
                criterion: REQ-CAPABILITY-001#criterion.verification-execution
                covers: [FEAT-VERIFICATION-001#binding.implementation/target.execute-verification]
                runner: { runner: cargo-test, arguments: { package: syu-validation, test: tests::exact_test_execution_requires_match } }
          - id: plan-validation-test
            adapter: rust
            path: crates/syu-validation/src/lib.rs
            selector: { kind: symbol, name: tests::canonical_execution_reconstructs_basis_after_editable_change }
            claims:
              - kind: verifies
                criterion: REQ-CAPABILITY-001#criterion.plan-validation
                covers: [FEAT-PLAN-VALIDATION-001#binding.implementation/target.canonical-plan-validation]
                runner: { runner: cargo-test, arguments: { package: syu-validation, test: tests::canonical_execution_reconstructs_basis_after_editable_change } }
          - id: receipt-validation-test
            adapter: rust
            path: crates/syu-validation/src/lib.rs
            selector: { kind: symbol, name: tests::completion_report_rejects_unchanged_modify_target }
            claims:
              - kind: verifies
                criterion: REQ-CAPABILITY-001#criterion.receipt-validation
                covers: [FEAT-RESULT-VALIDATION-001#binding.implementation/target.receipt-validation]
                runner: { runner: cargo-test, arguments: { package: syu-validation, test: tests::completion_report_rejects_unchanged_modify_target } }
          - id: cli-test
            adapter: rust
            path: tests/v1_cli.rs
            selector: { kind: symbol, name: current_workspace_validates_and_reports_configured_readiness }
            claims:
              - kind: verifies
                criterion: REQ-CAPABILITY-001#criterion.cli-orchestration
                covers: [FEAT-CLI-001#binding.implementation/target.cli-run]
                runner: { runner: cargo-test-integration, arguments: { package: syu, harness: v1_cli, test: current_workspace_validates_and_reports_configured_readiness } }
          - id: lsp-test
            adapter: rust
            path: src/lsp/handlers.rs
            selector: { kind: symbol, name: tests::handle_hover_renders_each_spec_layer }
            claims:
              - kind: verifies
                criterion: REQ-CAPABILITY-001#criterion.lsp-navigation
                covers: [FEAT-LSP-001#binding.implementation/target.lsp-server]
                runner: { runner: cargo-test, arguments: { package: syu, test: lsp::handlers::tests::handle_hover_renders_each_spec_layer } }
          - id: docs-test
            adapter: rust
            path: tests/v1_cli.rs
            selector: { kind: symbol, name: generated_spec_reference_covers_every_source_document }
            claims:
              - kind: verifies
                criterion: REQ-CAPABILITY-001#criterion.docs-generation
                covers: [FEAT-DOCS-001#binding.implementation/target.generated-index]
                runner: { runner: cargo-test-integration, arguments: { package: syu, harness: v1_cli, test: generated_spec_reference_covers_every_source_document } }
```
