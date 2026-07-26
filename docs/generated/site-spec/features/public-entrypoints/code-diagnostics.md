---
title: "Public entrypoint contracts / Code Diagnostics"
description: "Generated reference for docs/syu/features/public-entrypoints/code-diagnostics.yaml"
---

> Generated from `docs/syu/features/public-entrypoints/code-diagnostics.yaml`.

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

- **id**: FEAT-PUBLIC-CODE-DIAGNOSTICS-001
  - **title**: Code diagnostics
  - **summary**: Govern symbol resolution and diagnostic result entrypoints.
  - **status**: implemented
  - **bindings**:
    - **id**: public-api-029
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint addressable by an exact target.
      - **targets**:
        - **id**: entrypoint-029-rust-crates-syu-code-intel-src-lib-rs-resolve-symbol
          - **adapter**: rust
          - **path**: crates/syu-code-intel/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: resolve_symbol
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.code-diagnostics
    - **id**: public-api-030
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint addressable by an exact target.
      - **targets**:
        - **id**: entrypoint-030-rust-crates-syu-diagnostics-src-lib-rs-diagnostic-error
          - **adapter**: rust
          - **path**: crates/syu-diagnostics/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: Diagnostic::error
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.code-diagnostics
    - **id**: public-api-031
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint addressable by an exact target.
      - **targets**:
        - **id**: entrypoint-031-rust-crates-syu-diagnostics-src-lib-rs-validationresult
          - **adapter**: rust
          - **path**: crates/syu-diagnostics/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: ValidationResult::is_valid
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.code-diagnostics
    - **id**: public-readiness
      - **role**: verification
      - **facet**: public
      - **responsibility**: Prove code diagnostics entrypoints have bounded canonical plans.
      - **targets**:
        - **id**: canonical-plans
          - **adapter**: rust
          - **path**: crates/syu-validation/src/readiness.rs
          - **selector**:
            - **kind**: symbol
            - **name**: tests::code_diagnostics_public_entrypoints_have_canonical_plans
          - **claims**:
            - **kind**: verifies
              - **criterion**: REQ-PUBLIC-001#criterion.code-diagnostics
              - **covers**:
                - FEAT-PUBLIC-CODE-DIAGNOSTICS-001#binding.public-api-029/target.entrypoint-029-rust-crates-syu-code-intel-src-lib-rs-resolve-symbol
                - FEAT-PUBLIC-CODE-DIAGNOSTICS-001#binding.public-api-030/target.entrypoint-030-rust-crates-syu-diagnostics-src-lib-rs-diagnostic-error
                - FEAT-PUBLIC-CODE-DIAGNOSTICS-001#binding.public-api-031/target.entrypoint-031-rust-crates-syu-diagnostics-src-lib-rs-validationresult
              - **runner**:
                - **runner**: cargo-test
                - **arguments**:
                  - **package**: syu-validation
                  - **test**: tests::code_diagnostics_public_entrypoints_have_canonical_plans

## Source YAML

```yaml
schema: syu/spec/v1
kind: features
namespace: public
category: Public entrypoint contracts
features:
- id: FEAT-PUBLIC-CODE-DIAGNOSTICS-001
  title: Code diagnostics
  summary: Govern symbol resolution and diagnostic result entrypoints.
  status: implemented
  bindings:
  - id: public-api-029
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint addressable by an exact target.
    targets:
    - id: entrypoint-029-rust-crates-syu-code-intel-src-lib-rs-resolve-symbol
      adapter: rust
      path: crates/syu-code-intel/src/lib.rs
      selector:
        kind: symbol
        name: resolve_symbol
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.code-diagnostics
  - id: public-api-030
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint addressable by an exact target.
    targets:
    - id: entrypoint-030-rust-crates-syu-diagnostics-src-lib-rs-diagnostic-error
      adapter: rust
      path: crates/syu-diagnostics/src/lib.rs
      selector:
        kind: symbol
        name: Diagnostic::error
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.code-diagnostics
  - id: public-api-031
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint addressable by an exact target.
    targets:
    - id: entrypoint-031-rust-crates-syu-diagnostics-src-lib-rs-validationresult
      adapter: rust
      path: crates/syu-diagnostics/src/lib.rs
      selector:
        kind: symbol
        name: ValidationResult::is_valid
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.code-diagnostics
  - id: public-readiness
    role: verification
    facet: public
    responsibility: Prove code diagnostics entrypoints have bounded canonical plans.
    targets:
    - id: canonical-plans
      adapter: rust
      path: crates/syu-validation/src/readiness.rs
      selector:
        kind: symbol
        name: tests::code_diagnostics_public_entrypoints_have_canonical_plans
      claims:
      - kind: verifies
        criterion: REQ-PUBLIC-001#criterion.code-diagnostics
        covers:
        - FEAT-PUBLIC-CODE-DIAGNOSTICS-001#binding.public-api-029/target.entrypoint-029-rust-crates-syu-code-intel-src-lib-rs-resolve-symbol
        - FEAT-PUBLIC-CODE-DIAGNOSTICS-001#binding.public-api-030/target.entrypoint-030-rust-crates-syu-diagnostics-src-lib-rs-diagnostic-error
        - FEAT-PUBLIC-CODE-DIAGNOSTICS-001#binding.public-api-031/target.entrypoint-031-rust-crates-syu-diagnostics-src-lib-rs-validationresult
        runner:
          runner: cargo-test
          arguments:
            package: syu-validation
            test: tests::code_diagnostics_public_entrypoints_have_canonical_plans
```
