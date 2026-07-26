---
title: "Public entrypoint contracts / Workbench Server"
description: "Generated reference for docs/syu/features/public-entrypoints/workbench-server.yaml"
---

> Generated from `docs/syu/features/public-entrypoints/workbench-server.yaml`.

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

- **id**: FEAT-PUBLIC-WORKBENCH-SERVER-001
  - **title**: Workbench server lifecycle
  - **summary**: Govern Workbench server construction, projection, routing, and execution.
  - **status**: implemented
  - **bindings**:
    - **id**: public-api-047
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint addressable by an exact target.
      - **targets**:
        - **id**: entrypoint-052-rust-crates-syu-workbench-server-src-lib-rs-branchscope
          - **adapter**: rust
          - **path**: crates/syu-workbench-server/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: BranchScopeView::not_applicable
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.workbench-server-lifecycle
    - **id**: public-api-048
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint addressable by an exact target.
      - **targets**:
        - **id**: entrypoint-053-rust-crates-syu-workbench-server-src-lib-rs-validationr
          - **adapter**: rust
          - **path**: crates/syu-workbench-server/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: ValidationRunView::completed
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.workbench-server-lifecycle
    - **id**: public-api-049
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint addressable by an exact target.
      - **targets**:
        - **id**: entrypoint-054-rust-crates-syu-workbench-server-src-lib-rs-validationr
          - **adapter**: rust
          - **path**: crates/syu-workbench-server/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: ValidationRunView::failed
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.workbench-server-lifecycle
    - **id**: public-api-050
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint addressable by an exact target.
      - **targets**:
        - **id**: entrypoint-055-rust-crates-syu-workbench-server-src-lib-rs-validationr
          - **adapter**: rust
          - **path**: crates/syu-workbench-server/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: ValidationRunView::not_applicable
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.workbench-server-lifecycle
    - **id**: public-api-051
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint addressable by an exact target.
      - **targets**:
        - **id**: entrypoint-056-rust-crates-syu-workbench-server-src-lib-rs-validationr
          - **adapter**: rust
          - **path**: crates/syu-workbench-server/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: ValidationRunView::not_run
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.workbench-server-lifecycle
    - **id**: public-api-052
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint addressable by an exact target.
      - **targets**:
        - **id**: entrypoint-057-rust-crates-syu-workbench-server-src-lib-rs-workbenchse
          - **adapter**: rust
          - **path**: crates/syu-workbench-server/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: WorkbenchServer::new
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.workbench-server-lifecycle
    - **id**: public-api-053
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint addressable by an exact target.
      - **targets**:
        - **id**: entrypoint-058-rust-crates-syu-workbench-server-src-lib-rs-workbenchse
          - **adapter**: rust
          - **path**: crates/syu-workbench-server/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: WorkbenchServer::projection
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.workbench-server-lifecycle
    - **id**: public-api-054
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint addressable by an exact target.
      - **targets**:
        - **id**: entrypoint-059-rust-crates-syu-workbench-server-src-lib-rs-workbenchse
          - **adapter**: rust
          - **path**: crates/syu-workbench-server/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: WorkbenchServer::router
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.workbench-server-lifecycle
    - **id**: public-api-055
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint addressable by an exact target.
      - **targets**:
        - **id**: entrypoint-060-rust-crates-syu-workbench-server-src-lib-rs-workbenchse
          - **adapter**: rust
          - **path**: crates/syu-workbench-server/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: WorkbenchServer::run
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.workbench-server-lifecycle
    - **id**: public-api-056
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint addressable by an exact target.
      - **targets**:
        - **id**: entrypoint-061-rust-crates-syu-workbench-server-src-lib-rs-workbenchse
          - **adapter**: rust
          - **path**: crates/syu-workbench-server/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: WorkbenchServer::with_launch
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.workbench-server-lifecycle
    - **id**: public-api-057
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint addressable by an exact target.
      - **targets**:
        - **id**: entrypoint-062-rust-crates-syu-workbench-server-src-lib-rs-workbenchse
          - **adapter**: rust
          - **path**: crates/syu-workbench-server/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: WorkbenchServer::with_request
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.workbench-server-lifecycle
    - **id**: public-readiness
      - **role**: verification
      - **facet**: public
      - **responsibility**: Prove workbench server lifecycle entrypoints have bounded canonical plans.
      - **targets**:
        - **id**: canonical-plans
          - **adapter**: rust
          - **path**: crates/syu-validation/src/readiness.rs
          - **selector**:
            - **kind**: symbol
            - **name**: tests::workbench_server_lifecycle_public_entrypoints_have_canonical_plans
          - **claims**:
            - **kind**: verifies
              - **criterion**: REQ-PUBLIC-001#criterion.workbench-server-lifecycle
              - **covers**:
                - FEAT-PUBLIC-WORKBENCH-SERVER-001#binding.public-api-047/target.entrypoint-052-rust-crates-syu-workbench-server-src-lib-rs-branchscope
                - FEAT-PUBLIC-WORKBENCH-SERVER-001#binding.public-api-048/target.entrypoint-053-rust-crates-syu-workbench-server-src-lib-rs-validationr
                - FEAT-PUBLIC-WORKBENCH-SERVER-001#binding.public-api-049/target.entrypoint-054-rust-crates-syu-workbench-server-src-lib-rs-validationr
                - FEAT-PUBLIC-WORKBENCH-SERVER-001#binding.public-api-050/target.entrypoint-055-rust-crates-syu-workbench-server-src-lib-rs-validationr
                - FEAT-PUBLIC-WORKBENCH-SERVER-001#binding.public-api-051/target.entrypoint-056-rust-crates-syu-workbench-server-src-lib-rs-validationr
                - FEAT-PUBLIC-WORKBENCH-SERVER-001#binding.public-api-052/target.entrypoint-057-rust-crates-syu-workbench-server-src-lib-rs-workbenchse
                - FEAT-PUBLIC-WORKBENCH-SERVER-001#binding.public-api-053/target.entrypoint-058-rust-crates-syu-workbench-server-src-lib-rs-workbenchse
                - FEAT-PUBLIC-WORKBENCH-SERVER-001#binding.public-api-054/target.entrypoint-059-rust-crates-syu-workbench-server-src-lib-rs-workbenchse
                - FEAT-PUBLIC-WORKBENCH-SERVER-001#binding.public-api-055/target.entrypoint-060-rust-crates-syu-workbench-server-src-lib-rs-workbenchse
                - FEAT-PUBLIC-WORKBENCH-SERVER-001#binding.public-api-056/target.entrypoint-061-rust-crates-syu-workbench-server-src-lib-rs-workbenchse
                - FEAT-PUBLIC-WORKBENCH-SERVER-001#binding.public-api-057/target.entrypoint-062-rust-crates-syu-workbench-server-src-lib-rs-workbenchse
              - **runner**:
                - **runner**: cargo-test
                - **arguments**:
                  - **package**: syu-validation
                  - **test**: tests::workbench_server_lifecycle_public_entrypoints_have_canonical_plans

## Source YAML

```yaml
schema: syu/spec/v1
kind: features
namespace: public
category: Public entrypoint contracts
features:
- id: FEAT-PUBLIC-WORKBENCH-SERVER-001
  title: Workbench server lifecycle
  summary: Govern Workbench server construction, projection, routing, and execution.
  status: implemented
  bindings:
  - id: public-api-047
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint addressable by an exact target.
    targets:
    - id: entrypoint-052-rust-crates-syu-workbench-server-src-lib-rs-branchscope
      adapter: rust
      path: crates/syu-workbench-server/src/lib.rs
      selector:
        kind: symbol
        name: BranchScopeView::not_applicable
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.workbench-server-lifecycle
  - id: public-api-048
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint addressable by an exact target.
    targets:
    - id: entrypoint-053-rust-crates-syu-workbench-server-src-lib-rs-validationr
      adapter: rust
      path: crates/syu-workbench-server/src/lib.rs
      selector:
        kind: symbol
        name: ValidationRunView::completed
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.workbench-server-lifecycle
  - id: public-api-049
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint addressable by an exact target.
    targets:
    - id: entrypoint-054-rust-crates-syu-workbench-server-src-lib-rs-validationr
      adapter: rust
      path: crates/syu-workbench-server/src/lib.rs
      selector:
        kind: symbol
        name: ValidationRunView::failed
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.workbench-server-lifecycle
  - id: public-api-050
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint addressable by an exact target.
    targets:
    - id: entrypoint-055-rust-crates-syu-workbench-server-src-lib-rs-validationr
      adapter: rust
      path: crates/syu-workbench-server/src/lib.rs
      selector:
        kind: symbol
        name: ValidationRunView::not_applicable
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.workbench-server-lifecycle
  - id: public-api-051
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint addressable by an exact target.
    targets:
    - id: entrypoint-056-rust-crates-syu-workbench-server-src-lib-rs-validationr
      adapter: rust
      path: crates/syu-workbench-server/src/lib.rs
      selector:
        kind: symbol
        name: ValidationRunView::not_run
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.workbench-server-lifecycle
  - id: public-api-052
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint addressable by an exact target.
    targets:
    - id: entrypoint-057-rust-crates-syu-workbench-server-src-lib-rs-workbenchse
      adapter: rust
      path: crates/syu-workbench-server/src/lib.rs
      selector:
        kind: symbol
        name: WorkbenchServer::new
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.workbench-server-lifecycle
  - id: public-api-053
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint addressable by an exact target.
    targets:
    - id: entrypoint-058-rust-crates-syu-workbench-server-src-lib-rs-workbenchse
      adapter: rust
      path: crates/syu-workbench-server/src/lib.rs
      selector:
        kind: symbol
        name: WorkbenchServer::projection
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.workbench-server-lifecycle
  - id: public-api-054
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint addressable by an exact target.
    targets:
    - id: entrypoint-059-rust-crates-syu-workbench-server-src-lib-rs-workbenchse
      adapter: rust
      path: crates/syu-workbench-server/src/lib.rs
      selector:
        kind: symbol
        name: WorkbenchServer::router
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.workbench-server-lifecycle
  - id: public-api-055
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint addressable by an exact target.
    targets:
    - id: entrypoint-060-rust-crates-syu-workbench-server-src-lib-rs-workbenchse
      adapter: rust
      path: crates/syu-workbench-server/src/lib.rs
      selector:
        kind: symbol
        name: WorkbenchServer::run
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.workbench-server-lifecycle
  - id: public-api-056
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint addressable by an exact target.
    targets:
    - id: entrypoint-061-rust-crates-syu-workbench-server-src-lib-rs-workbenchse
      adapter: rust
      path: crates/syu-workbench-server/src/lib.rs
      selector:
        kind: symbol
        name: WorkbenchServer::with_launch
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.workbench-server-lifecycle
  - id: public-api-057
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint addressable by an exact target.
    targets:
    - id: entrypoint-062-rust-crates-syu-workbench-server-src-lib-rs-workbenchse
      adapter: rust
      path: crates/syu-workbench-server/src/lib.rs
      selector:
        kind: symbol
        name: WorkbenchServer::with_request
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.workbench-server-lifecycle
  - id: public-readiness
    role: verification
    facet: public
    responsibility: Prove workbench server lifecycle entrypoints have bounded canonical
      plans.
    targets:
    - id: canonical-plans
      adapter: rust
      path: crates/syu-validation/src/readiness.rs
      selector:
        kind: symbol
        name: tests::workbench_server_lifecycle_public_entrypoints_have_canonical_plans
      claims:
      - kind: verifies
        criterion: REQ-PUBLIC-001#criterion.workbench-server-lifecycle
        covers:
        - FEAT-PUBLIC-WORKBENCH-SERVER-001#binding.public-api-047/target.entrypoint-052-rust-crates-syu-workbench-server-src-lib-rs-branchscope
        - FEAT-PUBLIC-WORKBENCH-SERVER-001#binding.public-api-048/target.entrypoint-053-rust-crates-syu-workbench-server-src-lib-rs-validationr
        - FEAT-PUBLIC-WORKBENCH-SERVER-001#binding.public-api-049/target.entrypoint-054-rust-crates-syu-workbench-server-src-lib-rs-validationr
        - FEAT-PUBLIC-WORKBENCH-SERVER-001#binding.public-api-050/target.entrypoint-055-rust-crates-syu-workbench-server-src-lib-rs-validationr
        - FEAT-PUBLIC-WORKBENCH-SERVER-001#binding.public-api-051/target.entrypoint-056-rust-crates-syu-workbench-server-src-lib-rs-validationr
        - FEAT-PUBLIC-WORKBENCH-SERVER-001#binding.public-api-052/target.entrypoint-057-rust-crates-syu-workbench-server-src-lib-rs-workbenchse
        - FEAT-PUBLIC-WORKBENCH-SERVER-001#binding.public-api-053/target.entrypoint-058-rust-crates-syu-workbench-server-src-lib-rs-workbenchse
        - FEAT-PUBLIC-WORKBENCH-SERVER-001#binding.public-api-054/target.entrypoint-059-rust-crates-syu-workbench-server-src-lib-rs-workbenchse
        - FEAT-PUBLIC-WORKBENCH-SERVER-001#binding.public-api-055/target.entrypoint-060-rust-crates-syu-workbench-server-src-lib-rs-workbenchse
        - FEAT-PUBLIC-WORKBENCH-SERVER-001#binding.public-api-056/target.entrypoint-061-rust-crates-syu-workbench-server-src-lib-rs-workbenchse
        - FEAT-PUBLIC-WORKBENCH-SERVER-001#binding.public-api-057/target.entrypoint-062-rust-crates-syu-workbench-server-src-lib-rs-workbenchse
        runner:
          runner: cargo-test
          arguments:
            package: syu-validation
            test: tests::workbench_server_lifecycle_public_entrypoints_have_canonical_plans
```
