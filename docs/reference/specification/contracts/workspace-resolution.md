---
title: "Public entrypoint contracts / Workspace Resolution"
description: "Generated reference for docs/mitase/features/public-entrypoints/workspace-resolution.yaml"
---

> Generated from `docs/mitase/features/public-entrypoints/workspace-resolution.yaml`.

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

- **id**: FEAT-PUBLIC-WORKSPACE-RESOLUTION-001
  - **title**: Workspace resolution
  - **summary**: Govern selector resolution and stable workspace fingerprints.
  - **status**: implemented
  - **bindings**:
    - **id**: public-api-069
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint exactly addressable and linked to its verified capability boundary.
      - **targets**:
        - **id**: entrypoint-074-rust-crates-mitase-workspace-src-lib-rs-specworkspace-path
          - **adapter**: rust
          - **path**: crates/mitase-workspace/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: SpecWorkspace::path_is_artifact
          - **claims**:
            - **kind**: exposes
              - **target**: FEAT-IDENTITY-001#binding.implementation/target.target-resolver
    - **id**: public-api-070
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint exactly addressable and linked to its verified capability boundary.
      - **targets**:
        - **id**: entrypoint-075-rust-crates-mitase-workspace-src-lib-rs-specworkspace-path
          - **adapter**: rust
          - **path**: crates/mitase-workspace/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: SpecWorkspace::path_is_excluded
          - **claims**:
            - **kind**: exposes
              - **target**: FEAT-IDENTITY-001#binding.implementation/target.target-resolver
    - **id**: public-api-071
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint exactly addressable and linked to its verified capability boundary.
      - **targets**:
        - **id**: entrypoint-076-rust-crates-mitase-workspace-src-lib-rs-specworkspace-path
          - **adapter**: rust
          - **path**: crates/mitase-workspace/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: SpecWorkspace::path_is_spec
          - **claims**:
            - **kind**: exposes
              - **target**: FEAT-IDENTITY-001#binding.implementation/target.target-resolver
    - **id**: public-api-074
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint exactly addressable and linked to its verified capability boundary.
      - **targets**:
        - **id**: entrypoint-079-rust-crates-mitase-workspace-src-lib-rs-specworkspace-try
          - **adapter**: rust
          - **path**: crates/mitase-workspace/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: SpecWorkspace::try_fingerprint
          - **claims**:
            - **kind**: exposes
              - **target**: FEAT-IDENTITY-001#binding.implementation/target.target-resolver
    - **id**: public-api-075
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint exactly addressable and linked to its verified capability boundary.
      - **targets**:
        - **id**: entrypoint-080-rust-crates-mitase-workspace-src-lib-rs-resolve-target
          - **adapter**: rust
          - **path**: crates/mitase-workspace/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: resolve_target
          - **claims**:
            - **kind**: exposes
              - **target**: FEAT-IDENTITY-001#binding.implementation/target.target-resolver
    - **id**: public-api-076
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint exactly addressable and linked to its verified capability boundary.
      - **targets**:
        - **id**: entrypoint-082-rust-crates-mitase-workspace-src-lib-rs-resolve-target-wit
          - **adapter**: rust
          - **path**: crates/mitase-workspace/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: resolve_target_with_adapters
          - **claims**:
            - **kind**: exposes
              - **target**: FEAT-IDENTITY-001#binding.implementation/target.target-resolver
    - **id**: public-api-077
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint exactly addressable and linked to its verified capability boundary.
      - **targets**:
        - **id**: entrypoint-083-rust-crates-mitase-workspace-src-lib-rs-selector-supports
          - **adapter**: rust
          - **path**: crates/mitase-workspace/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: selector_supports_editable
          - **claims**:
            - **kind**: exposes
              - **target**: FEAT-IDENTITY-001#binding.implementation/target.target-resolver
    - **id**: public-api-resolve-indexed-target
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint exactly addressable and linked to its verified capability boundary.
      - **targets**:
        - **id**: entrypoint-resolve-indexed-target
          - **adapter**: rust
          - **path**: crates/mitase-workspace/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: resolve_indexed_target
          - **claims**:
            - **kind**: exposes
              - **target**: FEAT-IDENTITY-001#binding.implementation/target.target-resolver
    - **id**: public-api-ownership-fingerprint
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Govern ownership graph fingerprints.
      - **targets**:
        - **id**: entrypoint-ownership-fingerprint
          - **adapter**: rust
          - **path**: crates/mitase-workspace/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: ownership_fingerprint
          - **claims**:
            - **kind**: exposes
              - **target**: FEAT-IDENTITY-001#binding.implementation/target.target-resolver
    - **id**: public-api-spec-fingerprint
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Govern specification graph fingerprints.
      - **targets**:
        - **id**: entrypoint-spec-fingerprint
          - **adapter**: rust
          - **path**: crates/mitase-workspace/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: spec_fingerprint
          - **claims**:
            - **kind**: exposes
              - **target**: FEAT-IDENTITY-001#binding.implementation/target.target-resolver

## Source YAML

```yaml
schema: mitase/spec/v1
kind: features
namespace: public
category: Public entrypoint contracts
features:
- id: FEAT-PUBLIC-WORKSPACE-RESOLUTION-001
  title: Workspace resolution
  summary: Govern selector resolution and stable workspace fingerprints.
  status: implemented
  bindings:
  - id: public-api-069
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint exactly addressable and linked to its verified capability boundary.
    targets:
    - id: entrypoint-074-rust-crates-mitase-workspace-src-lib-rs-specworkspace-path
      adapter: rust
      path: crates/mitase-workspace/src/lib.rs
      selector:
        kind: symbol
        name: SpecWorkspace::path_is_artifact
      claims:
      - kind: exposes
        target: FEAT-IDENTITY-001#binding.implementation/target.target-resolver
  - id: public-api-070
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint exactly addressable and linked to its verified capability boundary.
    targets:
    - id: entrypoint-075-rust-crates-mitase-workspace-src-lib-rs-specworkspace-path
      adapter: rust
      path: crates/mitase-workspace/src/lib.rs
      selector:
        kind: symbol
        name: SpecWorkspace::path_is_excluded
      claims:
      - kind: exposes
        target: FEAT-IDENTITY-001#binding.implementation/target.target-resolver
  - id: public-api-071
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint exactly addressable and linked to its verified capability boundary.
    targets:
    - id: entrypoint-076-rust-crates-mitase-workspace-src-lib-rs-specworkspace-path
      adapter: rust
      path: crates/mitase-workspace/src/lib.rs
      selector:
        kind: symbol
        name: SpecWorkspace::path_is_spec
      claims:
      - kind: exposes
        target: FEAT-IDENTITY-001#binding.implementation/target.target-resolver
  - id: public-api-074
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint exactly addressable and linked to its verified capability boundary.
    targets:
    - id: entrypoint-079-rust-crates-mitase-workspace-src-lib-rs-specworkspace-try
      adapter: rust
      path: crates/mitase-workspace/src/lib.rs
      selector:
        kind: symbol
        name: SpecWorkspace::try_fingerprint
      claims:
      - kind: exposes
        target: FEAT-IDENTITY-001#binding.implementation/target.target-resolver
  - id: public-api-075
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint exactly addressable and linked to its verified capability boundary.
    targets:
    - id: entrypoint-080-rust-crates-mitase-workspace-src-lib-rs-resolve-target
      adapter: rust
      path: crates/mitase-workspace/src/lib.rs
      selector:
        kind: symbol
        name: resolve_target
      claims:
      - kind: exposes
        target: FEAT-IDENTITY-001#binding.implementation/target.target-resolver
  - id: public-api-076
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint exactly addressable and linked to its verified capability boundary.
    targets:
    - id: entrypoint-082-rust-crates-mitase-workspace-src-lib-rs-resolve-target-wit
      adapter: rust
      path: crates/mitase-workspace/src/lib.rs
      selector:
        kind: symbol
        name: resolve_target_with_adapters
      claims:
      - kind: exposes
        target: FEAT-IDENTITY-001#binding.implementation/target.target-resolver
  - id: public-api-077
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint exactly addressable and linked to its verified capability boundary.
    targets:
    - id: entrypoint-083-rust-crates-mitase-workspace-src-lib-rs-selector-supports
      adapter: rust
      path: crates/mitase-workspace/src/lib.rs
      selector:
        kind: symbol
        name: selector_supports_editable
      claims:
      - kind: exposes
        target: FEAT-IDENTITY-001#binding.implementation/target.target-resolver
  - id: public-api-resolve-indexed-target
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint exactly addressable and linked to its verified capability boundary.
    targets:
    - id: entrypoint-resolve-indexed-target
      adapter: rust
      path: crates/mitase-workspace/src/lib.rs
      selector:
        kind: symbol
        name: resolve_indexed_target
      claims:
      - kind: exposes
        target: FEAT-IDENTITY-001#binding.implementation/target.target-resolver
  - id: public-api-ownership-fingerprint
    role: implementation
    facet: public
    responsibility: Govern ownership graph fingerprints.
    targets:
    - id: entrypoint-ownership-fingerprint
      adapter: rust
      path: crates/mitase-workspace/src/lib.rs
      selector:
        kind: symbol
        name: ownership_fingerprint
      claims:
      - kind: exposes
        target: FEAT-IDENTITY-001#binding.implementation/target.target-resolver
  - id: public-api-spec-fingerprint
    role: implementation
    facet: public
    responsibility: Govern specification graph fingerprints.
    targets:
    - id: entrypoint-spec-fingerprint
      adapter: rust
      path: crates/mitase-workspace/src/lib.rs
      selector:
        kind: symbol
        name: spec_fingerprint
      claims:
      - kind: exposes
        target: FEAT-IDENTITY-001#binding.implementation/target.target-resolver
```
