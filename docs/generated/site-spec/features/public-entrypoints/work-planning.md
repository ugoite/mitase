---
title: "Public entrypoint contracts / Work Planning"
description: "Generated reference for docs/syu/features/public-entrypoints/work-planning.yaml"
---

> Generated from `docs/syu/features/public-entrypoints/work-planning.yaml`.

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

## Source YAML

```yaml
schema: syu/spec/v1
kind: features
namespace: public
category: Public entrypoint contracts
features:
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
```
