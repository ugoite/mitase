---
title: "Foundation / Foundation"
description: "Generated reference for docs/mitase/philosophies/foundation.yaml"
---

> Generated from `docs/mitase/philosophies/foundation.yaml`.

## Parsed content

### Schema

- mitase/spec/v1

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
          - **path**: docs/understand/model/v1-architecture.md
          - **selector**:
            - **kind**: heading
            - **value**: Mitase v1 architecture
          - **claims**:
            - **kind**: documents
              - **anchor**: PHIL-001#principle.exact-intent

## Source YAML

```yaml
schema: mitase/spec/v1
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
            path: docs/understand/model/v1-architecture.md
            selector: { kind: heading, value: Mitase v1 architecture }
            claims:
              - kind: documents
                anchor: PHIL-001#principle.exact-intent
```
