---
title: "Public entrypoint contracts / Workspace Loading"
description: "Generated reference for docs/mitase/features/public-entrypoints/workspace-loading.yaml"
---

> Generated from `docs/mitase/features/public-entrypoints/workspace-loading.yaml`.

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

- **id**: FEAT-PUBLIC-WORKSPACE-LOADING-001
  - **title**: Workspace loading
  - **summary**: Govern workspace loading, overlays, index access, and content reads.
  - **status**: implemented
  - **bindings**:
    - **id**: public-api-061
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint exactly addressable and linked to its verified capability boundary.
      - **targets**:
        - **id**: entrypoint-066-rust-crates-mitase-workspace-src-lib-rs-specindex-anchor
          - **adapter**: rust
          - **path**: crates/mitase-workspace/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: SpecIndex::anchor
          - **claims**:
            - **kind**: exposes
              - **target**: FEAT-INDEX-001#binding.implementation/target.spec-index
    - **id**: public-api-062
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint exactly addressable and linked to its verified capability boundary.
      - **targets**:
        - **id**: entrypoint-067-rust-crates-mitase-workspace-src-lib-rs-specindex-build
          - **adapter**: rust
          - **path**: crates/mitase-workspace/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: SpecIndex::build
          - **claims**:
            - **kind**: exposes
              - **target**: FEAT-INDEX-001#binding.implementation/target.spec-index
    - **id**: public-api-063
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint exactly addressable and linked to its verified capability boundary.
      - **targets**:
        - **id**: entrypoint-068-rust-crates-mitase-workspace-src-lib-rs-specindex-target
          - **adapter**: rust
          - **path**: crates/mitase-workspace/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: SpecIndex::target
          - **claims**:
            - **kind**: exposes
              - **target**: FEAT-INDEX-001#binding.implementation/target.spec-index
    - **id**: public-api-064
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint exactly addressable and linked to its verified capability boundary.
      - **targets**:
        - **id**: entrypoint-069-rust-crates-mitase-workspace-src-lib-rs-specworkspace-fing
          - **adapter**: rust
          - **path**: crates/mitase-workspace/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: SpecWorkspace::fingerprint
          - **claims**:
            - **kind**: exposes
              - **target**: FEAT-INDEX-001#binding.implementation/target.spec-index
    - **id**: public-api-065
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint exactly addressable and linked to its verified capability boundary.
      - **targets**:
        - **id**: entrypoint-070-rust-crates-mitase-workspace-src-lib-rs-specworkspace-inde
          - **adapter**: rust
          - **path**: crates/mitase-workspace/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: SpecWorkspace::index
          - **claims**:
            - **kind**: exposes
              - **target**: FEAT-INDEX-001#binding.implementation/target.spec-index
    - **id**: public-api-066
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint exactly addressable and linked to its verified capability boundary.
      - **targets**:
        - **id**: entrypoint-071-rust-crates-mitase-workspace-src-lib-rs-specworkspace-load
          - **adapter**: rust
          - **path**: crates/mitase-workspace/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: SpecWorkspace::load
          - **claims**:
            - **kind**: exposes
              - **target**: FEAT-INDEX-001#binding.implementation/target.spec-index
    - **id**: public-api-067
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint exactly addressable and linked to its verified capability boundary.
      - **targets**:
        - **id**: entrypoint-072-rust-crates-mitase-workspace-src-lib-rs-specworkspace-over
          - **adapter**: rust
          - **path**: crates/mitase-workspace/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: SpecWorkspace::overlay_config
          - **claims**:
            - **kind**: exposes
              - **target**: FEAT-INDEX-001#binding.implementation/target.spec-index
    - **id**: public-api-068
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint exactly addressable and linked to its verified capability boundary.
      - **targets**:
        - **id**: entrypoint-073-rust-crates-mitase-workspace-src-lib-rs-specworkspace-over
          - **adapter**: rust
          - **path**: crates/mitase-workspace/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: SpecWorkspace::overlay_document
          - **claims**:
            - **kind**: exposes
              - **target**: FEAT-INDEX-001#binding.implementation/target.spec-index
    - **id**: public-api-072
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint exactly addressable and linked to its verified capability boundary.
      - **targets**:
        - **id**: entrypoint-077-rust-crates-mitase-workspace-src-lib-rs-specworkspace-read
          - **adapter**: rust
          - **path**: crates/mitase-workspace/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: SpecWorkspace::read_bytes
          - **claims**:
            - **kind**: exposes
              - **target**: FEAT-INDEX-001#binding.implementation/target.spec-index
    - **id**: public-api-073
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint exactly addressable and linked to its verified capability boundary.
      - **targets**:
        - **id**: entrypoint-078-rust-crates-mitase-workspace-src-lib-rs-specworkspace-read
          - **adapter**: rust
          - **path**: crates/mitase-workspace/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: SpecWorkspace::read_to_string
          - **claims**:
            - **kind**: exposes
              - **target**: FEAT-INDEX-001#binding.implementation/target.spec-index

## Source YAML

```yaml
schema: mitase/spec/v1
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
    responsibility: Keep this public entrypoint exactly addressable and linked to its verified capability boundary.
    targets:
    - id: entrypoint-066-rust-crates-mitase-workspace-src-lib-rs-specindex-anchor
      adapter: rust
      path: crates/mitase-workspace/src/lib.rs
      selector:
        kind: symbol
        name: SpecIndex::anchor
      claims:
      - kind: exposes
        target: FEAT-INDEX-001#binding.implementation/target.spec-index
  - id: public-api-062
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint exactly addressable and linked to its verified capability boundary.
    targets:
    - id: entrypoint-067-rust-crates-mitase-workspace-src-lib-rs-specindex-build
      adapter: rust
      path: crates/mitase-workspace/src/lib.rs
      selector:
        kind: symbol
        name: SpecIndex::build
      claims:
      - kind: exposes
        target: FEAT-INDEX-001#binding.implementation/target.spec-index
  - id: public-api-063
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint exactly addressable and linked to its verified capability boundary.
    targets:
    - id: entrypoint-068-rust-crates-mitase-workspace-src-lib-rs-specindex-target
      adapter: rust
      path: crates/mitase-workspace/src/lib.rs
      selector:
        kind: symbol
        name: SpecIndex::target
      claims:
      - kind: exposes
        target: FEAT-INDEX-001#binding.implementation/target.spec-index
  - id: public-api-064
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint exactly addressable and linked to its verified capability boundary.
    targets:
    - id: entrypoint-069-rust-crates-mitase-workspace-src-lib-rs-specworkspace-fing
      adapter: rust
      path: crates/mitase-workspace/src/lib.rs
      selector:
        kind: symbol
        name: SpecWorkspace::fingerprint
      claims:
      - kind: exposes
        target: FEAT-INDEX-001#binding.implementation/target.spec-index
  - id: public-api-065
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint exactly addressable and linked to its verified capability boundary.
    targets:
    - id: entrypoint-070-rust-crates-mitase-workspace-src-lib-rs-specworkspace-inde
      adapter: rust
      path: crates/mitase-workspace/src/lib.rs
      selector:
        kind: symbol
        name: SpecWorkspace::index
      claims:
      - kind: exposes
        target: FEAT-INDEX-001#binding.implementation/target.spec-index
  - id: public-api-066
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint exactly addressable and linked to its verified capability boundary.
    targets:
    - id: entrypoint-071-rust-crates-mitase-workspace-src-lib-rs-specworkspace-load
      adapter: rust
      path: crates/mitase-workspace/src/lib.rs
      selector:
        kind: symbol
        name: SpecWorkspace::load
      claims:
      - kind: exposes
        target: FEAT-INDEX-001#binding.implementation/target.spec-index
  - id: public-api-067
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint exactly addressable and linked to its verified capability boundary.
    targets:
    - id: entrypoint-072-rust-crates-mitase-workspace-src-lib-rs-specworkspace-over
      adapter: rust
      path: crates/mitase-workspace/src/lib.rs
      selector:
        kind: symbol
        name: SpecWorkspace::overlay_config
      claims:
      - kind: exposes
        target: FEAT-INDEX-001#binding.implementation/target.spec-index
  - id: public-api-068
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint exactly addressable and linked to its verified capability boundary.
    targets:
    - id: entrypoint-073-rust-crates-mitase-workspace-src-lib-rs-specworkspace-over
      adapter: rust
      path: crates/mitase-workspace/src/lib.rs
      selector:
        kind: symbol
        name: SpecWorkspace::overlay_document
      claims:
      - kind: exposes
        target: FEAT-INDEX-001#binding.implementation/target.spec-index
  - id: public-api-072
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint exactly addressable and linked to its verified capability boundary.
    targets:
    - id: entrypoint-077-rust-crates-mitase-workspace-src-lib-rs-specworkspace-read
      adapter: rust
      path: crates/mitase-workspace/src/lib.rs
      selector:
        kind: symbol
        name: SpecWorkspace::read_bytes
      claims:
      - kind: exposes
        target: FEAT-INDEX-001#binding.implementation/target.spec-index
  - id: public-api-073
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint exactly addressable and linked to its verified capability boundary.
    targets:
    - id: entrypoint-078-rust-crates-mitase-workspace-src-lib-rs-specworkspace-read
      adapter: rust
      path: crates/mitase-workspace/src/lib.rs
      selector:
        kind: symbol
        name: SpecWorkspace::read_to_string
      claims:
      - kind: exposes
        target: FEAT-INDEX-001#binding.implementation/target.spec-index
```
