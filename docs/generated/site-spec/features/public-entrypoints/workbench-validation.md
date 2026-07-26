---
title: "Public entrypoint contracts / Workbench Validation"
description: "Generated reference for docs/syu/features/public-entrypoints/workbench-validation.yaml"
---

> Generated from `docs/syu/features/public-entrypoints/workbench-validation.yaml`.

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

- **id**: FEAT-PUBLIC-WORKBENCH-VALIDATION-001
  - **title**: Workbench server validation
  - **summary**: Govern branch-scope and verification-receipt entrypoints.
  - **status**: implemented
  - **bindings**:
    - **id**: public-api-058
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint addressable by an exact target.
      - **targets**:
        - **id**: entrypoint-063-rust-crates-syu-workbench-server-src-lib-rs-branch-scop
          - **adapter**: rust
          - **path**: crates/syu-workbench-server/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: branch_scope_view
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.workbench-server-validation
    - **id**: public-api-059
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint addressable by an exact target.
      - **targets**:
        - **id**: entrypoint-064-rust-crates-syu-workbench-server-src-lib-rs-execute-ver
          - **adapter**: rust
          - **path**: crates/syu-workbench-server/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: execute_verification
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.workbench-server-validation
    - **id**: public-api-060
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint addressable by an exact target.
      - **targets**:
        - **id**: entrypoint-065-rust-crates-syu-workbench-server-src-lib-rs-validate-ve
          - **adapter**: rust
          - **path**: crates/syu-workbench-server/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: validate_verification_receipt
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.workbench-server-validation
    - **id**: public-readiness
      - **role**: verification
      - **facet**: public
      - **responsibility**: Prove workbench server validation entrypoints have bounded canonical plans.
      - **targets**:
        - **id**: canonical-plans
          - **adapter**: rust
          - **path**: crates/syu-validation/src/readiness.rs
          - **selector**:
            - **kind**: symbol
            - **name**: tests::workbench_server_validation_public_entrypoints_have_canonical_plans
          - **claims**:
            - **kind**: verifies
              - **criterion**: REQ-PUBLIC-001#criterion.workbench-server-validation
              - **covers**:
                - FEAT-PUBLIC-WORKBENCH-VALIDATION-001#binding.public-api-058/target.entrypoint-063-rust-crates-syu-workbench-server-src-lib-rs-branch-scop
                - FEAT-PUBLIC-WORKBENCH-VALIDATION-001#binding.public-api-059/target.entrypoint-064-rust-crates-syu-workbench-server-src-lib-rs-execute-ver
                - FEAT-PUBLIC-WORKBENCH-VALIDATION-001#binding.public-api-060/target.entrypoint-065-rust-crates-syu-workbench-server-src-lib-rs-validate-ve
              - **runner**:
                - **runner**: cargo-test
                - **arguments**:
                  - **package**: syu-validation
                  - **test**: tests::workbench_server_validation_public_entrypoints_have_canonical_plans

## Source YAML

```yaml
schema: syu/spec/v1
kind: features
namespace: public
category: Public entrypoint contracts
features:
- id: FEAT-PUBLIC-WORKBENCH-VALIDATION-001
  title: Workbench server validation
  summary: Govern branch-scope and verification-receipt entrypoints.
  status: implemented
  bindings:
  - id: public-api-058
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint addressable by an exact target.
    targets:
    - id: entrypoint-063-rust-crates-syu-workbench-server-src-lib-rs-branch-scop
      adapter: rust
      path: crates/syu-workbench-server/src/lib.rs
      selector:
        kind: symbol
        name: branch_scope_view
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.workbench-server-validation
  - id: public-api-059
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint addressable by an exact target.
    targets:
    - id: entrypoint-064-rust-crates-syu-workbench-server-src-lib-rs-execute-ver
      adapter: rust
      path: crates/syu-workbench-server/src/lib.rs
      selector:
        kind: symbol
        name: execute_verification
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.workbench-server-validation
  - id: public-api-060
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint addressable by an exact target.
    targets:
    - id: entrypoint-065-rust-crates-syu-workbench-server-src-lib-rs-validate-ve
      adapter: rust
      path: crates/syu-workbench-server/src/lib.rs
      selector:
        kind: symbol
        name: validate_verification_receipt
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.workbench-server-validation
  - id: public-readiness
    role: verification
    facet: public
    responsibility: Prove workbench server validation entrypoints have bounded canonical
      plans.
    targets:
    - id: canonical-plans
      adapter: rust
      path: crates/syu-validation/src/readiness.rs
      selector:
        kind: symbol
        name: tests::workbench_server_validation_public_entrypoints_have_canonical_plans
      claims:
      - kind: verifies
        criterion: REQ-PUBLIC-001#criterion.workbench-server-validation
        covers:
        - FEAT-PUBLIC-WORKBENCH-VALIDATION-001#binding.public-api-058/target.entrypoint-063-rust-crates-syu-workbench-server-src-lib-rs-branch-scop
        - FEAT-PUBLIC-WORKBENCH-VALIDATION-001#binding.public-api-059/target.entrypoint-064-rust-crates-syu-workbench-server-src-lib-rs-execute-ver
        - FEAT-PUBLIC-WORKBENCH-VALIDATION-001#binding.public-api-060/target.entrypoint-065-rust-crates-syu-workbench-server-src-lib-rs-validate-ve
        runner:
          runner: cargo-test
          arguments:
            package: syu-validation
            test: tests::workbench_server_validation_public_entrypoints_have_canonical_plans
```
