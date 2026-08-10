---
title: "Public entrypoint contracts / Work Planning"
description: "Generated reference for docs/mitase/features/public-entrypoints/work-planning.yaml"
---

> Generated from `docs/mitase/features/public-entrypoints/work-planning.yaml`.

## Parsed content

### Schema

- mitase/spec/v1

### Kind

- features

### Namespace

- public

### Category

- Public entrypoint contracts

### Features

- **id**: FEAT-PUBLIC-WORK-PLANNING-001
  - **title**: Work planning
  - **summary**: Govern exact Work origins, requested-target identity, canonical plan digests, and split-recovery selection entrypoints.
  - **status**: implemented
  - **bindings**:
    - **id**: public-api-043
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint exactly addressable and linked to its verified capability boundary.
      - **targets**:
        - **id**: entrypoint-048-rust-crates-mitase-work-model-src-lib-rs-requestedtarget-c
          - **adapter**: rust
          - **path**: crates/mitase-work-model/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: RequestedTarget::criterion
          - **claims**:
            - **kind**: exposes
              - **target**: FEAT-PLANNER-001#binding.implementation/target.canonical-plan
    - **id**: public-api-044
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint exactly addressable and linked to its verified capability boundary.
      - **targets**:
        - **id**: entrypoint-049-rust-crates-mitase-work-model-src-lib-rs-requestedtarget-r
          - **adapter**: rust
          - **path**: crates/mitase-work-model/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: RequestedTarget::reference
          - **claims**:
            - **kind**: exposes
              - **target**: FEAT-PLANNER-001#binding.implementation/target.canonical-plan
    - **id**: public-api-045
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint exactly addressable and linked to its verified capability boundary.
      - **targets**:
        - **id**: entrypoint-050-rust-crates-mitase-work-model-src-lib-rs-requestedtarget-t
          - **adapter**: rust
          - **path**: crates/mitase-work-model/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: RequestedTarget::transition
          - **claims**:
            - **kind**: exposes
              - **target**: FEAT-PLANNER-001#binding.implementation/target.canonical-plan
    - **id**: public-api-046
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint exactly addressable and linked to its verified capability boundary.
      - **targets**:
        - **id**: entrypoint-051-rust-crates-mitase-work-model-src-lib-rs-work-plan-digest
          - **adapter**: rust
          - **path**: crates/mitase-work-model/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: work_plan_digest
          - **claims**:
            - **kind**: exposes
              - **target**: FEAT-PLANNER-001#binding.implementation/target.canonical-plan
    - **id**: public-api-split-work-recommendation
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Govern bounded split recommendations.
      - **targets**:
        - **id**: entrypoint-split-work-recommendation
          - **adapter**: rust
          - **path**: crates/mitase-planner/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: split_work_recommendation
          - **claims**:
            - **kind**: exposes
              - **target**: FEAT-PLANNER-001#binding.implementation/target.canonical-plan
    - **id**: public-api-readonly-targets-fingerprint
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Govern readonly target fingerprints.
      - **targets**:
        - **id**: entrypoint-readonly-targets-fingerprint
          - **adapter**: rust
          - **path**: crates/mitase-work-model/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: readonly_targets_fingerprint
          - **claims**:
            - **kind**: exposes
              - **target**: FEAT-PLANNER-001#binding.implementation/target.canonical-plan

## Source YAML

```yaml
schema: mitase/spec/v1
kind: features
namespace: public
category: Public entrypoint contracts
features:
- id: FEAT-PUBLIC-WORK-PLANNING-001
  title: Work planning
  summary: Govern exact Work origins, requested-target identity, canonical plan digests, and split-recovery selection entrypoints.
  status: implemented
  bindings:
  - id: public-api-043
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint exactly addressable and linked to its verified capability boundary.
    targets:
    - id: entrypoint-048-rust-crates-mitase-work-model-src-lib-rs-requestedtarget-c
      adapter: rust
      path: crates/mitase-work-model/src/lib.rs
      selector:
        kind: symbol
        name: RequestedTarget::criterion
      claims:
      - kind: exposes
        target: FEAT-PLANNER-001#binding.implementation/target.canonical-plan
  - id: public-api-044
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint exactly addressable and linked to its verified capability boundary.
    targets:
    - id: entrypoint-049-rust-crates-mitase-work-model-src-lib-rs-requestedtarget-r
      adapter: rust
      path: crates/mitase-work-model/src/lib.rs
      selector:
        kind: symbol
        name: RequestedTarget::reference
      claims:
      - kind: exposes
        target: FEAT-PLANNER-001#binding.implementation/target.canonical-plan
  - id: public-api-045
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint exactly addressable and linked to its verified capability boundary.
    targets:
    - id: entrypoint-050-rust-crates-mitase-work-model-src-lib-rs-requestedtarget-t
      adapter: rust
      path: crates/mitase-work-model/src/lib.rs
      selector:
        kind: symbol
        name: RequestedTarget::transition
      claims:
      - kind: exposes
        target: FEAT-PLANNER-001#binding.implementation/target.canonical-plan
  - id: public-api-046
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint exactly addressable and linked to its verified capability boundary.
    targets:
    - id: entrypoint-051-rust-crates-mitase-work-model-src-lib-rs-work-plan-digest
      adapter: rust
      path: crates/mitase-work-model/src/lib.rs
      selector:
        kind: symbol
        name: work_plan_digest
      claims:
      - kind: exposes
        target: FEAT-PLANNER-001#binding.implementation/target.canonical-plan
  - id: public-api-split-work-recommendation
    role: implementation
    facet: public
    responsibility: Govern bounded split recommendations.
    targets:
    - id: entrypoint-split-work-recommendation
      adapter: rust
      path: crates/mitase-planner/src/lib.rs
      selector:
        kind: symbol
        name: split_work_recommendation
      claims:
      - kind: exposes
        target: FEAT-PLANNER-001#binding.implementation/target.canonical-plan
  - id: public-api-readonly-targets-fingerprint
    role: implementation
    facet: public
    responsibility: Govern readonly target fingerprints.
    targets:
    - id: entrypoint-readonly-targets-fingerprint
      adapter: rust
      path: crates/mitase-work-model/src/lib.rs
      selector:
        kind: symbol
        name: readonly_targets_fingerprint
      claims:
      - kind: exposes
        target: FEAT-PLANNER-001#binding.implementation/target.canonical-plan
```
