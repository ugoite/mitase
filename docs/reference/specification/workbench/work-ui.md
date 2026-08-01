---
title: "Workbench implementation / Work Ui"
description: "Generated reference for docs/syu/features/workbench/work-ui.yaml"
---

> Generated from `docs/syu/features/workbench/work-ui.yaml`.

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

- **id**: FEAT-WORKBENCH-WORK-UI-001
  - **title**: Workbench work UI
  - **summary**: Drive the WorkRequest to result-validation journey from the Work page.
  - **status**: implemented
  - **bindings**:
    - **id**: work
      - **role**: implementation
      - **facet**: work
      - **responsibility**: Plan and validate a bounded Workbench work session.
      - **targets**:
        - **id**: plan
          - **adapter**: rust
          - **path**: crates/syu-workbench-server/src/lib.rs
          - **selector**:
            - **kind**: symbol
            - **name**: api_plan
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-WORKBENCH-002#criterion.work-session

## Source YAML

```yaml
schema: syu/spec/v1
kind: features
namespace: workbench
category: Workbench implementation
features:
- id: FEAT-WORKBENCH-WORK-UI-001
  title: Workbench work UI
  summary: Drive the WorkRequest to result-validation journey from the Work page.
  status: implemented
  bindings:
  - id: work
    role: implementation
    facet: work
    responsibility: Plan and validate a bounded Workbench work session.
    targets:
    - id: plan
      adapter: rust
      path: crates/syu-workbench-server/src/lib.rs
      selector:
        kind: symbol
        name: api_plan
      claims:
      - kind: satisfies
        criterion: REQ-WORKBENCH-002#criterion.work-session
```
