---
title: "Public entrypoint contracts / Workbench Rendering"
description: "Generated reference for docs/syu/features/public-entrypoints/workbench-rendering.yaml"
---

> Generated from `docs/syu/features/public-entrypoints/workbench-rendering.yaml`.

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

- **id**: FEAT-PUBLIC-WORKBENCH-RENDERING-001
  - **title**: Workbench shell rendering
  - **summary**: Govern localized server-rendered Workbench shell construction.
  - **status**: implemented
  - **bindings**:
    - **id**: public-api-026
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint addressable by an exact target.
      - **targets**:
        - **id**: entrypoint-026-rust-crates-syu-app-ui-src-lib-rs-workbenchview-new
          - **adapter**: rust
          - **path**: crates/syu-app-ui/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: WorkbenchView::new
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.workbench-rendering
    - **id**: public-api-027
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint addressable by an exact target.
      - **targets**:
        - **id**: entrypoint-027-rust-crates-syu-app-ui-src-lib-rs-workbenchview-render
          - **adapter**: rust
          - **path**: crates/syu-app-ui/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: WorkbenchView::render_html
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.workbench-rendering
    - **id**: public-api-028
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint addressable by an exact target.
      - **targets**:
        - **id**: entrypoint-028-rust-crates-syu-app-ui-src-lib-rs-locale-catalog-script
          - **adapter**: rust
          - **path**: crates/syu-app-ui/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: locale_catalog_script
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-PUBLIC-001#criterion.workbench-rendering
    - **id**: public-readiness
      - **role**: verification
      - **facet**: public
      - **responsibility**: Prove workbench shell rendering entrypoints have bounded canonical plans.
      - **targets**:
        - **id**: canonical-plans
          - **adapter**: rust
          - **path**: crates/syu-validation/src/readiness.rs
          - **selector**:
            - **kind**: symbol
            - **name**: tests::workbench_rendering_public_entrypoints_have_canonical_plans
          - **claims**:
            - **kind**: verifies
              - **criterion**: REQ-PUBLIC-001#criterion.workbench-rendering
              - **covers**:
                - FEAT-PUBLIC-WORKBENCH-RENDERING-001#binding.public-api-026/target.entrypoint-026-rust-crates-syu-app-ui-src-lib-rs-workbenchview-new
                - FEAT-PUBLIC-WORKBENCH-RENDERING-001#binding.public-api-027/target.entrypoint-027-rust-crates-syu-app-ui-src-lib-rs-workbenchview-render
                - FEAT-PUBLIC-WORKBENCH-RENDERING-001#binding.public-api-028/target.entrypoint-028-rust-crates-syu-app-ui-src-lib-rs-locale-catalog-script
              - **runner**:
                - **runner**: cargo-test
                - **arguments**:
                  - **package**: syu-validation
                  - **test**: tests::workbench_rendering_public_entrypoints_have_canonical_plans

## Source YAML

```yaml
schema: syu/spec/v1
kind: features
namespace: public
category: Public entrypoint contracts
features:
- id: FEAT-PUBLIC-WORKBENCH-RENDERING-001
  title: Workbench shell rendering
  summary: Govern localized server-rendered Workbench shell construction.
  status: implemented
  bindings:
  - id: public-api-026
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint addressable by an exact target.
    targets:
    - id: entrypoint-026-rust-crates-syu-app-ui-src-lib-rs-workbenchview-new
      adapter: rust
      path: crates/syu-app-ui/src/lib.rs
      selector:
        kind: symbol
        name: WorkbenchView::new
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.workbench-rendering
  - id: public-api-027
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint addressable by an exact target.
    targets:
    - id: entrypoint-027-rust-crates-syu-app-ui-src-lib-rs-workbenchview-render
      adapter: rust
      path: crates/syu-app-ui/src/lib.rs
      selector:
        kind: symbol
        name: WorkbenchView::render_html
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.workbench-rendering
  - id: public-api-028
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint addressable by an exact target.
    targets:
    - id: entrypoint-028-rust-crates-syu-app-ui-src-lib-rs-locale-catalog-script
      adapter: rust
      path: crates/syu-app-ui/src/lib.rs
      selector:
        kind: symbol
        name: locale_catalog_script
      claims:
      - kind: satisfies
        criterion: REQ-PUBLIC-001#criterion.workbench-rendering
  - id: public-readiness
    role: verification
    facet: public
    responsibility: Prove workbench shell rendering entrypoints have bounded canonical
      plans.
    targets:
    - id: canonical-plans
      adapter: rust
      path: crates/syu-validation/src/readiness.rs
      selector:
        kind: symbol
        name: tests::workbench_rendering_public_entrypoints_have_canonical_plans
      claims:
      - kind: verifies
        criterion: REQ-PUBLIC-001#criterion.workbench-rendering
        covers:
        - FEAT-PUBLIC-WORKBENCH-RENDERING-001#binding.public-api-026/target.entrypoint-026-rust-crates-syu-app-ui-src-lib-rs-workbenchview-new
        - FEAT-PUBLIC-WORKBENCH-RENDERING-001#binding.public-api-027/target.entrypoint-027-rust-crates-syu-app-ui-src-lib-rs-workbenchview-render
        - FEAT-PUBLIC-WORKBENCH-RENDERING-001#binding.public-api-028/target.entrypoint-028-rust-crates-syu-app-ui-src-lib-rs-locale-catalog-script
        runner:
          runner: cargo-test
          arguments:
            package: syu-validation
            test: tests::workbench_rendering_public_entrypoints_have_canonical_plans
```
