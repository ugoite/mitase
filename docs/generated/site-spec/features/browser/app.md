---
title: "Browser App / App"
description: "Generated reference for docs/syu/features/browser/app.yaml"
---

> Generated from `docs/syu/features/browser/app.yaml`.

## Parsed content

### Category

- Browser App

### Version

- 1

### Features

- **id**: FEAT-APP-001
  - **title**: Local browser workspace app powered by Rust and WebAssembly
  - **summary**: Start `syu app` to inspect the current workspace in a browser with clear startup guidance, a minimal header, layered navigation, section-aware drilldown, discoverable keyboard search shortcuts, linked definitions, and the current validation state.
  - **status**: implemented
  - **linked_requirements**:
    - REQ-CORE-017
  - **implementations**:
    - **rust**:
      - **file**: src/cli.rs
        - **symbols**:
          - AppArgs
      - **file**: src/command/app.rs
        - **symbols**:
          - *
      - **file**: build.rs
        - **symbols**:
          - build_browser_bundle
          - ensure_app_dependencies
          - main
      - **file**: src/config.rs
        - **symbols**:
          - AppConfig
      - **file**: crates/syu-core/src/lib.rs
        - **symbols**:
          - *
      - **file**: app/wasm/src/lib.rs
        - **symbols**:
          - *
    - **yaml**:
      - **file**: docs/syu/config/app.yaml
        - **symbols**:
          - app.bind
          - app.port
    - **typescript**:
      - **file**: app/src/App.tsx
        - **symbols**:
          - *
      - **file**: app/src/main.tsx
        - **symbols**:
          - *
      - **file**: app/tests/browser-app.spec.ts
        - **symbols**:
          - *

## Source YAML

```yaml
category: Browser App
version: 1

features:
  - id: FEAT-APP-001
    title: Local browser workspace app powered by Rust and WebAssembly
    summary: Start `syu app` to inspect the current workspace in a browser with clear startup guidance, a minimal header, layered navigation, section-aware drilldown, discoverable keyboard search shortcuts, linked definitions, and the current validation state.
    status: implemented
    linked_requirements:
      - REQ-CORE-017
    implementations:
      rust:
        - file: src/cli.rs
          symbols:
            - AppArgs
        - file: src/command/app.rs
          symbols:
            - "*"
        - file: build.rs
          symbols:
            - build_browser_bundle
            - ensure_app_dependencies
            - main
        - file: src/config.rs
          symbols:
            - AppConfig
        - file: crates/syu-core/src/lib.rs
          symbols:
            - "*"
        - file: app/wasm/src/lib.rs
          symbols:
            - "*"
      yaml:
        - file: docs/syu/config/app.yaml
          symbols:
            - app.bind
            - app.port
      typescript:
        - file: app/src/App.tsx
          symbols:
            - "*"
        - file: app/src/main.tsx
          symbols:
            - "*"
        - file: app/tests/browser-app.spec.ts
          symbols:
            - "*"
```
