---
title: "Workbench implementation / Server"
description: "Generated reference for docs/syu/features/workbench/server.yaml"
---

> Generated from `docs/syu/features/workbench/server.yaml`.

## Parsed content

### Schema

- syu/spec/v1

### Kind

- features

### Namespace

- workbench

### Category

- Workbench implementation

### Features

- **id**: FEAT-WORKBENCH-SERVER-001
  - **title**: Workbench server
  - **summary**: Expose canonical read, work, validation, edit, and security APIs.
  - **status**: implemented
  - **bindings**:
    - **id**: server
      - **role**: implementation
      - **facet**: server
      - **responsibility**: Serve canonical Workbench HTTP routes and mutation guards.
      - **owns**:
        - **id**: server-module
          - **adapter**: rust
          - **path**: crates/syu-workbench-server/src/lib.rs
          - **selector**:
            - **kind**: module
            - **name**: lib
      - **targets**:
        - **id**: mutation-guard
          - **adapter**: rust
          - **path**: crates/syu-workbench-server/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: mutation_guard
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-WORKBENCH-005#criterion.secure-local-server
        - **id**: workspace-snapshot
          - **adapter**: rust
          - **path**: crates/syu-workbench-server/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: snapshot
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-WORKBENCH-012#criterion.exact-snapshot-reuse

## Source YAML

```yaml
schema: syu/spec/v1
kind: features
namespace: workbench
category: Workbench implementation
features:
- id: FEAT-WORKBENCH-SERVER-001
  title: Workbench server
  summary: Expose canonical read, work, validation, edit, and security APIs.
  status: implemented
  bindings:
  - id: server
    role: implementation
    facet: server
    responsibility: Serve canonical Workbench HTTP routes and mutation guards.
    owns:
    - id: server-module
      adapter: rust
      path: crates/syu-workbench-server/src/lib.rs
      selector: { kind: module, name: lib }
    targets:
    - id: mutation-guard
      adapter: rust
      path: crates/syu-workbench-server/src/lib.rs
      selector:
        kind: symbol
        name: mutation_guard
      claims:
      - kind: satisfies
        criterion: REQ-WORKBENCH-005#criterion.secure-local-server
    - id: workspace-snapshot
      adapter: rust
      path: crates/syu-workbench-server/src/lib.rs
      selector:
        kind: symbol
        name: snapshot
      claims:
      - kind: satisfies
        criterion: REQ-WORKBENCH-012#criterion.exact-snapshot-reuse
```
