---
title: "Work planning / Planner"
description: "Generated reference for docs/syu/planner.yaml"
---

> Generated from `docs/syu/planner.yaml`.

## Parsed content

### Schema

- syu/spec/v1

### Kind

- features

### Namespace

- work

### Category

- Work planning

### Features

- **id**: FEAT-WORK-001
  - **title**: Contract-aware work planner
  - **summary**: Build deterministic execution slices and context packs from canonical graph anchors.
  - **status**: implemented
  - **bindings**:
    - **id**: planner-engine
      - **role**: implementation
      - **facet**: tooling
      - **responsibility**: Expand exact seeds and classify bound targets into deterministic slices.
      - **targets**:
        - **id**: plan
          - **adapter**: rust
          - **path**: crates/syu-planner/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **names**:
              - plan
              - build_implementation_slice
              - build_verification_slice
              - export_context
      - **satisfies**:
        - REQ-WORK-001#criterion.exact-slice
- **id**: FEAT-WORK-002
  - **title**: Qualitative item coverage validator
  - **summary**: Classify item-to-evidence traceability by the Syu benefit it unlocks and enforce a repository-wide target.
  - **status**: implemented
  - **bindings**:
    - **id**: coverage-engine
      - **role**: implementation
      - **facet**: tooling
      - **responsibility**: Compute connected, owned, agent-ready, verified, and evidence-ready item coverage from the canonical graph.
      - **targets**:
        - **id**: coverage-validation
          - **adapter**: rust
          - **path**: crates/syu-validation/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **names**:
              - validate_item_coverage
              - validate_artifact_axis
      - **satisfies**:
        - REQ-WORK-002#criterion.qualitative-coverage

## Source YAML

```yaml
schema: syu/spec/v1
kind: features
namespace: work
category: Work planning
features:
  - id: FEAT-WORK-001
    title: Contract-aware work planner
    summary: Build deterministic execution slices and context packs from canonical graph anchors.
    status: implemented
    bindings:
      - id: planner-engine
        role: implementation
        facet: tooling
        responsibility: Expand exact seeds and classify bound targets into deterministic slices.
        targets:
          - id: plan
            adapter: rust
            path: crates/syu-planner/src/lib.rs
            selector: { kind: symbol, names: [plan, build_implementation_slice, build_verification_slice, export_context] }
        satisfies: [REQ-WORK-001#criterion.exact-slice]
  - id: FEAT-WORK-002
    title: Qualitative item coverage validator
    summary: Classify item-to-evidence traceability by the Syu benefit it unlocks and enforce a repository-wide target.
    status: implemented
    bindings:
      - id: coverage-engine
        role: implementation
        facet: tooling
        responsibility: Compute connected, owned, agent-ready, verified, and evidence-ready item coverage from the canonical graph.
        targets:
          - id: coverage-validation
            adapter: rust
            path: crates/syu-validation/src/lib.rs
            selector: { kind: symbol, names: [validate_item_coverage, validate_artifact_axis] }
        satisfies: [REQ-WORK-002#criterion.qualitative-coverage]
```
