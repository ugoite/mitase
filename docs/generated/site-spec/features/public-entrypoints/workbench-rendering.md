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
      - **responsibility**: Keep this public entrypoint exactly addressable and linked to its verified capability boundary.
      - **targets**:
        - **id**: entrypoint-026-rust-crates-syu-app-ui-src-lib-rs-workbenchview-new
          - **adapter**: rust
          - **path**: crates/syu-app-ui/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: WorkbenchView::new
          - **claims**:
            - **kind**: exposes
              - **target**: FEAT-WORKBENCH-NAVIGATION-001#binding.navigation/target.html-navigation
    - **id**: public-api-027
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint exactly addressable and linked to its verified capability boundary.
      - **targets**:
        - **id**: entrypoint-027-rust-crates-syu-app-ui-src-lib-rs-workbenchview-render
          - **adapter**: rust
          - **path**: crates/syu-app-ui/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: WorkbenchView::render_html
          - **claims**:
            - **kind**: exposes
              - **target**: FEAT-WORKBENCH-NAVIGATION-001#binding.navigation/target.html-navigation
    - **id**: public-api-028
      - **role**: implementation
      - **facet**: public
      - **responsibility**: Keep this public entrypoint exactly addressable and linked to its verified capability boundary.
      - **targets**:
        - **id**: entrypoint-028-rust-crates-syu-app-ui-src-lib-rs-locale-catalog-script
          - **adapter**: rust
          - **path**: crates/syu-app-ui/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: locale_catalog_script
          - **claims**:
            - **kind**: exposes
              - **target**: FEAT-WORKBENCH-NAVIGATION-001#binding.navigation/target.html-navigation

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
    responsibility: Keep this public entrypoint exactly addressable and linked to its verified capability boundary.
    targets:
    - id: entrypoint-026-rust-crates-syu-app-ui-src-lib-rs-workbenchview-new
      adapter: rust
      path: crates/syu-app-ui/src/lib.rs
      selector:
        kind: symbol
        name: WorkbenchView::new
      claims:
      - kind: exposes
        target: FEAT-WORKBENCH-NAVIGATION-001#binding.navigation/target.html-navigation
  - id: public-api-027
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint exactly addressable and linked to its verified capability boundary.
    targets:
    - id: entrypoint-027-rust-crates-syu-app-ui-src-lib-rs-workbenchview-render
      adapter: rust
      path: crates/syu-app-ui/src/lib.rs
      selector:
        kind: symbol
        name: WorkbenchView::render_html
      claims:
      - kind: exposes
        target: FEAT-WORKBENCH-NAVIGATION-001#binding.navigation/target.html-navigation
  - id: public-api-028
    role: implementation
    facet: public
    responsibility: Keep this public entrypoint exactly addressable and linked to its verified capability boundary.
    targets:
    - id: entrypoint-028-rust-crates-syu-app-ui-src-lib-rs-locale-catalog-script
      adapter: rust
      path: crates/syu-app-ui/src/lib.rs
      selector:
        kind: symbol
        name: locale_catalog_script
      claims:
      - kind: exposes
        target: FEAT-WORKBENCH-NAVIGATION-001#binding.navigation/target.html-navigation
```
