---
title: "Public entrypoint contracts / Workspace Loading"
description: "Generated reference for docs/syu/features/public-entrypoints/workspace-loading.yaml"
---

> Generated from `docs/syu/features/public-entrypoints/workspace-loading.yaml`.

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

## Source YAML

```yaml
schema: syu/spec/v1
kind: features
namespace: public
category: Public entrypoint contracts
features:
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
```
