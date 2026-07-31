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
      - **responsibility**: Keep this public entrypoint exactly addressable and linked to its verified capability boundary.
      - **targets**:
        - **id**: entrypoint-029-rust-crates-syu-code-intel-src-lib-rs-resolve-symbol
          - **adapter**: rust
          - **path**: crates/syu-code-intel/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: resolve_symbol
          - **claims**:
            - **kind**: exposes
              - **target**: FEAT-IDENTITY-001#binding.implementation/target.target-resolver
    - **id**: public-api-030
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint exactly addressable and linked to its verified capability boundary.
      - **targets**:
        - **id**: entrypoint-030-rust-crates-syu-diagnostics-src-lib-rs-diagnostic-error
          - **adapter**: rust
          - **path**: crates/syu-diagnostics/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: Diagnostic::error
          - **claims**:
            - **kind**: exposes
              - **target**: FEAT-IDENTITY-001#binding.implementation/target.target-resolver
    - **id**: public-api-031
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint exactly addressable and linked to its verified capability boundary.
      - **targets**:
        - **id**: entrypoint-031-rust-crates-syu-diagnostics-src-lib-rs-validationresult
          - **adapter**: rust
          - **path**: crates/syu-diagnostics/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: ValidationResult::is_valid
          - **claims**:
            - **kind**: exposes
              - **target**: FEAT-IDENTITY-001#binding.implementation/target.target-resolver

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
    responsibility: Keep this public entrypoint exactly addressable and linked to its verified capability boundary.
    targets:
    - id: entrypoint-029-rust-crates-syu-code-intel-src-lib-rs-resolve-symbol
      adapter: rust
      path: crates/syu-code-intel/src/lib.rs
      selector:
        kind: symbol
        name: resolve_symbol
      claims:
      - kind: exposes
        target: FEAT-IDENTITY-001#binding.implementation/target.target-resolver
  - id: public-api-030
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint exactly addressable and linked to its verified capability boundary.
    targets:
    - id: entrypoint-030-rust-crates-syu-diagnostics-src-lib-rs-diagnostic-error
      adapter: rust
      path: crates/syu-diagnostics/src/lib.rs
      selector:
        kind: symbol
        name: Diagnostic::error
      claims:
      - kind: exposes
        target: FEAT-IDENTITY-001#binding.implementation/target.target-resolver
  - id: public-api-031
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint exactly addressable and linked to its verified capability boundary.
    targets:
    - id: entrypoint-031-rust-crates-syu-diagnostics-src-lib-rs-validationresult
      adapter: rust
      path: crates/syu-diagnostics/src/lib.rs
      selector:
        kind: symbol
        name: ValidationResult::is_valid
      claims:
      - kind: exposes
        target: FEAT-IDENTITY-001#binding.implementation/target.target-resolver
```
