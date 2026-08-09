---
title: "Workbench implementation / Scoped Agent"
description: "Generated reference for docs/syu/features/workbench/scoped-agent.yaml"
---

> Generated from `docs/syu/features/workbench/scoped-agent.yaml`.

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

- **id**: FEAT-WORKBENCH-SCOPED-AGENT-001
  - **title**: Scoped agent evidence
  - **summary**: Expose the approved lifecycle-aware agent boundary and append-only execution evidence under an exact plan-and-slice identity.
  - **status**: implemented
  - **bindings**:
    - **id**: agent-api
      - **role**: implementation
      - **facet**: scoped-agent
      - **responsibility**: Connect Workbench actions to the provider-neutral scoped agent API for exact Modify, Add, and Remove transitions.
      - **targets**:
        - **id**: agent-start
          - **adapter**: rust
          - **path**: crates/syu-workbench-server/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: api_agent_start
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-WORKBENCH-011#criterion.scoped-agent
        - **id**: agent-patch
          - **adapter**: rust
          - **path**: crates/syu-workbench-server/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: api_agent_patch
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-WORKBENCH-011#criterion.scoped-agent
        - **id**: agent-projection
          - **adapter**: rust
          - **path**: crates/syu-workbench-server/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: project_session
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-WORKBENCH-011#criterion.scoped-agent
        - **id**: agent-ui
          - **adapter**: javascript
          - **path**: crates/syu-app-ui/assets/js/pages/work.js
          - **selector**:
            - **kind**: symbol
            - **name**: renderWork
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-WORKBENCH-011#criterion.scoped-agent

## Source YAML

```yaml
schema: syu/spec/v1
kind: features
namespace: workbench
category: Workbench implementation
features:
- id: FEAT-WORKBENCH-SCOPED-AGENT-001
  title: Scoped agent evidence
  summary: Expose the approved lifecycle-aware agent boundary and append-only execution evidence under an exact plan-and-slice identity.
  status: implemented
  bindings:
  - id: agent-api
    role: implementation
    facet: scoped-agent
    responsibility: Connect Workbench actions to the provider-neutral scoped agent API for exact Modify, Add, and Remove transitions.
    targets:
    - id: agent-start
      adapter: rust
      path: crates/syu-workbench-server/src/lib.rs
      selector: { kind: symbol, name: api_agent_start }
      claims:
      - kind: satisfies
        criterion: REQ-WORKBENCH-011#criterion.scoped-agent
    - id: agent-patch
      adapter: rust
      path: crates/syu-workbench-server/src/lib.rs
      selector: { kind: symbol, name: api_agent_patch }
      claims:
      - kind: satisfies
        criterion: REQ-WORKBENCH-011#criterion.scoped-agent
    - id: agent-projection
      adapter: rust
      path: crates/syu-workbench-server/src/lib.rs
      selector: { kind: symbol, name: project_session }
      claims:
      - kind: satisfies
        criterion: REQ-WORKBENCH-011#criterion.scoped-agent
    - id: agent-ui
      adapter: javascript
      path: crates/syu-app-ui/assets/js/pages/work.js
      selector: { kind: symbol, name: renderWork }
      claims:
      - kind: satisfies
        criterion: REQ-WORKBENCH-011#criterion.scoped-agent
```
