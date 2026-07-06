---
title: "Foundation / Foundation"
description: "Generated reference for docs/syu/foundation.yaml"
---

> Generated from `docs/syu/foundation.yaml`.

## Parsed content

### Schema

- syu/spec/v1

### Kind

- philosophies

### Namespace

- foundation

### Category

- Foundation

### Philosophies

- **id**: PHIL-001
  - **title**: Traceable delivery
  - **summary**: Durable behavior remains explainable from intent to evidence.
  - **principles**:
    - **id**: exact-intent
      - **statement**: Durable behavior must connect exact intent to exact implementation and evidence.
      - **applies_to**:
        - product
        - code
        - work
  - **bindings**:
    - **id**: architecture-guide
      - **role**: documentation
      - **facet**: architecture
      - **responsibility**: Explain the canonical v1 traceability architecture.
      - **targets**:
        - **id**: architecture
          - **adapter**: markdown
          - **path**: docs/guide/v1-architecture.md
          - **selector**:
            - **kind**: heading
            - **value**: Syu v1 architecture
      - **documents**:
        - PHIL-001#principle.exact-intent

## Source YAML

```yaml
schema: syu/spec/v1
kind: philosophies
namespace: foundation
category: Foundation
philosophies:
  - id: PHIL-001
    title: Traceable delivery
    summary: Durable behavior remains explainable from intent to evidence.
    principles:
      - id: exact-intent
        statement: Durable behavior must connect exact intent to exact implementation and evidence.
        applies_to: [product, code, work]
    bindings:
      - id: architecture-guide
        role: documentation
        facet: architecture
        responsibility: Explain the canonical v1 traceability architecture.
        targets:
          - id: architecture
            adapter: markdown
            path: docs/guide/v1-architecture.md
            selector: { kind: heading, value: Syu v1 architecture }
        documents: [PHIL-001#principle.exact-intent]
```
