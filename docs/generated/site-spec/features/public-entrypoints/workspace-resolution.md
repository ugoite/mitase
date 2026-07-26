---
title: "Public entrypoint contracts / Workspace Resolution"
description: "Generated reference for docs/syu/features/public-entrypoints/workspace-resolution.yaml"
---

> Generated from `docs/syu/features/public-entrypoints/workspace-resolution.yaml`.

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

- **id**: FEAT-PUBLIC-WORKSPACE-RESOLUTION-001
  - **title**: Workspace resolution
  - **summary**: Govern selector resolution and stable workspace fingerprints.
  - **status**: implemented
  - **bindings**:
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
              - **criterion**: REQ-PUBLIC-001#criterion.workspace-resolution
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
              - **criterion**: REQ-PUBLIC-001#criterion.workspace-resolution
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
              - **criterion**: REQ-PUBLIC-001#criterion.workspace-resolution
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
              - **criterion**: REQ-PUBLIC-001#criterion.workspace-resolution
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
              - **criterion**: REQ-PUBLIC-001#criterion.workspace-resolution
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
              - **criterion**: REQ-PUBLIC-001#criterion.workspace-resolution
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
              - **criterion**: REQ-PUBLIC-001#criterion.workspace-resolution
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
              - **criterion**: REQ-PUBLIC-001#criterion.workspace-resolution
    - **id**: public-api-ownership-fingerprint
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Govern ownership graph fingerprints.
      - **targets**:
        - **id**: entrypoint-ownership-fingerprint
          - **adapter**: rust
          - **path**: crates/syu-workspace/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: ownership_fingerprint
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.workspace-resolution
    - **id**: public-api-spec-fingerprint
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Govern specification graph fingerprints.
      - **targets**:
        - **id**: entrypoint-spec-fingerprint
          - **adapter**: rust
          - **path**: crates/syu-workspace/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: spec_fingerprint
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.workspace-resolution
    - **id**: public-readiness
      - **role**: verification
      - **facet**: public
      - **responsibility**: Prove workspace resolution entrypoints have bounded canonical plans.
      - **targets**:
        - **id**: canonical-plans
          - **adapter**: rust
          - **path**: crates/syu-validation/src/readiness.rs
          - **selector**:
            - **kind**: symbol
            - **name**: tests::workspace_resolution_public_entrypoints_have_canonical_plans
          - **claims**:
            - **kind**: verifies
              - **criterion**: REQ-PUBLIC-001#criterion.workspace-resolution
              - **covers**:
                - FEAT-PUBLIC-WORKSPACE-RESOLUTION-001#binding.public-api-069/target.entrypoint-074-rust-crates-syu-workspace-src-lib-rs-specworkspace-path
                - FEAT-PUBLIC-WORKSPACE-RESOLUTION-001#binding.public-api-070/target.entrypoint-075-rust-crates-syu-workspace-src-lib-rs-specworkspace-path
                - FEAT-PUBLIC-WORKSPACE-RESOLUTION-001#binding.public-api-071/target.entrypoint-076-rust-crates-syu-workspace-src-lib-rs-specworkspace-path
                - FEAT-PUBLIC-WORKSPACE-RESOLUTION-001#binding.public-api-074/target.entrypoint-079-rust-crates-syu-workspace-src-lib-rs-specworkspace-try
                - FEAT-PUBLIC-WORKSPACE-RESOLUTION-001#binding.public-api-075/target.entrypoint-080-rust-crates-syu-workspace-src-lib-rs-resolve-target
                - FEAT-PUBLIC-WORKSPACE-RESOLUTION-001#binding.public-api-076/target.entrypoint-082-rust-crates-syu-workspace-src-lib-rs-resolve-target-wit
                - FEAT-PUBLIC-WORKSPACE-RESOLUTION-001#binding.public-api-077/target.entrypoint-083-rust-crates-syu-workspace-src-lib-rs-selector-supports
                - FEAT-PUBLIC-WORKSPACE-RESOLUTION-001#binding.public-api-resolve-indexed-target/target.entrypoint-resolve-indexed-target
                - FEAT-PUBLIC-WORKSPACE-RESOLUTION-001#binding.public-api-ownership-fingerprint/target.entrypoint-ownership-fingerprint
                - FEAT-PUBLIC-WORKSPACE-RESOLUTION-001#binding.public-api-spec-fingerprint/target.entrypoint-spec-fingerprint
              - **runner**:
                - **runner**: cargo-test
                - **arguments**:
                  - **package**: syu-validation
                  - **test**: tests::workspace_resolution_public_entrypoints_have_canonical_plans

## Source YAML

```yaml
schema: syu/spec/v1
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
        criterion: REQ-PUBLIC-001#criterion.workspace-resolution
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
        criterion: REQ-PUBLIC-001#criterion.workspace-resolution
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
        criterion: REQ-PUBLIC-001#criterion.workspace-resolution
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
        criterion: REQ-PUBLIC-001#criterion.workspace-resolution
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
        criterion: REQ-PUBLIC-001#criterion.workspace-resolution
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
        criterion: REQ-PUBLIC-001#criterion.workspace-resolution
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
        criterion: REQ-PUBLIC-001#criterion.workspace-resolution
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
        criterion: REQ-PUBLIC-001#criterion.workspace-resolution
  - id: public-api-ownership-fingerprint
    role: implementation
    facet: public
    responsibility: Govern ownership graph fingerprints.
    targets:
    - id: entrypoint-ownership-fingerprint
      adapter: rust
      path: crates/syu-workspace/src/lib.rs
      selector:
        kind: symbol
        name: ownership_fingerprint
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.workspace-resolution
  - id: public-api-spec-fingerprint
    role: implementation
    facet: public
    responsibility: Govern specification graph fingerprints.
    targets:
    - id: entrypoint-spec-fingerprint
      adapter: rust
      path: crates/syu-workspace/src/lib.rs
      selector:
        kind: symbol
        name: spec_fingerprint
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.workspace-resolution
  - id: public-readiness
    role: verification
    facet: public
    responsibility: Prove workspace resolution entrypoints have bounded canonical
      plans.
    targets:
    - id: canonical-plans
      adapter: rust
      path: crates/syu-validation/src/readiness.rs
      selector:
        kind: symbol
        name: tests::workspace_resolution_public_entrypoints_have_canonical_plans
      claims:
      - kind: verifies
        criterion: REQ-PUBLIC-001#criterion.workspace-resolution
        covers:
        - FEAT-PUBLIC-WORKSPACE-RESOLUTION-001#binding.public-api-069/target.entrypoint-074-rust-crates-syu-workspace-src-lib-rs-specworkspace-path
        - FEAT-PUBLIC-WORKSPACE-RESOLUTION-001#binding.public-api-070/target.entrypoint-075-rust-crates-syu-workspace-src-lib-rs-specworkspace-path
        - FEAT-PUBLIC-WORKSPACE-RESOLUTION-001#binding.public-api-071/target.entrypoint-076-rust-crates-syu-workspace-src-lib-rs-specworkspace-path
        - FEAT-PUBLIC-WORKSPACE-RESOLUTION-001#binding.public-api-074/target.entrypoint-079-rust-crates-syu-workspace-src-lib-rs-specworkspace-try
        - FEAT-PUBLIC-WORKSPACE-RESOLUTION-001#binding.public-api-075/target.entrypoint-080-rust-crates-syu-workspace-src-lib-rs-resolve-target
        - FEAT-PUBLIC-WORKSPACE-RESOLUTION-001#binding.public-api-076/target.entrypoint-082-rust-crates-syu-workspace-src-lib-rs-resolve-target-wit
        - FEAT-PUBLIC-WORKSPACE-RESOLUTION-001#binding.public-api-077/target.entrypoint-083-rust-crates-syu-workspace-src-lib-rs-selector-supports
        - FEAT-PUBLIC-WORKSPACE-RESOLUTION-001#binding.public-api-resolve-indexed-target/target.entrypoint-resolve-indexed-target
        - FEAT-PUBLIC-WORKSPACE-RESOLUTION-001#binding.public-api-ownership-fingerprint/target.entrypoint-ownership-fingerprint
        - FEAT-PUBLIC-WORKSPACE-RESOLUTION-001#binding.public-api-spec-fingerprint/target.entrypoint-spec-fingerprint
        runner:
          runner: cargo-test
          arguments:
            package: syu-validation
            test: tests::workspace_resolution_public_entrypoints_have_canonical_plans
```
