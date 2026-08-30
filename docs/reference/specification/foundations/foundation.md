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

- **id**: PHIL-003
  - **title**: Build Rules, Not Disposable Artifacts
  - **summary**: Encode product behavior as durable, reusable rules that can be continuously re-verified.
  - **principles**:
    - **id**: durable-rules
      - **statement**: Product behavior should be encoded as durable, reusable rules rather than manual one-off artifacts. Specifications, implementation, and validation are coupled so behavior can be continuously re-verified.
      - **applies_to**:
        - product
        - specification
        - implementation
        - verification
    - **id**: reusable-validation
      - **statement**: Tie behavior changes to specs and tests in the same change set. Prefer reusable helpers, schemas, and automation over manual procedural steps, and avoid introducing logic that cannot be continuously validated in CI.
      - **applies_to**:
        - implementation
        - verification
  - **bindings**:
    - **id**: architecture-guide
      - **role**: documentation
      - **facet**: architecture
      - **responsibility**: Explain the durable rule foundation and its relationship to specification, implementation, and verification.
      - **targets**:
        - **id**: architecture
          - **adapter**: markdown
          - **path**: docs/understand/model/v1-architecture.md
          - **selector**:
            - **kind**: heading
            - **value**: Foundation hierarchy
          - **claims**:
            - **kind**: documents
              - **anchor**: PHIL-003#principle.durable-rules
            - **kind**: documents
              - **anchor**: PHIL-003#principle.reusable-validation
- **id**: PHIL-005
  - **title**: Authority Without Workflow Ownership
  - **summary**: Mitase interprets and validates repository-owned meaning without owning repository workflow.
  - **principles**:
    - **id**: authority-without-workflow-ownership
      - **statement**: Mitase may be authoritative about the meaning of repository-owned specifications, their exact relationships, and whether declared repository evidence is structurally satisfied without becoming authoritative over how work is planned, implemented, executed, tested, reviewed, retried, or delivered.
      - **applies_to**:
        - specification
        - interpretation
        - indexing
        - resolution
        - validation
  - **bindings**:
    - **id**: authority-boundary-guide
      - **role**: documentation
      - **facet**: authority
      - **responsibility**: Explain the boundary between repository-owned specification authority and Mitase interpretation, indexing, resolution, and validation.
      - **targets**:
        - **id**: architecture
          - **adapter**: markdown
          - **path**: docs/understand/model/v1-architecture.md
          - **selector**:
            - **kind**: heading
            - **value**: Authority boundary
          - **claims**:
            - **kind**: documents
              - **anchor**: PHIL-005#principle.authority-without-workflow-ownership
- **id**: PHIL-006
  - **title**: Evidence Before Authority
  - **summary**: Treat declared relationships as authoritative only when their exact repository evidence resolves.
  - **principles**:
    - **id**: resolved-evidence
      - **statement**: A declared implementation or verification relationship becomes authoritative only after its exact repository targets resolve; unresolved declarations are diagnostics, not evidence.
      - **applies_to**:
        - specification
        - resolution
        - validation
  - **bindings**:
    - **id**: evidence-boundary-guide
      - **role**: documentation
      - **facet**: evidence
      - **responsibility**: Explain why exact resolution and current repository evidence are prerequisites for derived state.
      - **targets**:
        - **id**: architecture
          - **adapter**: markdown
          - **path**: docs/understand/model/v1-architecture.md
          - **selector**:
            - **kind**: heading
            - **value**: Evidence and derived state
          - **claims**:
            - **kind**: documents
              - **anchor**: PHIL-006#principle.resolved-evidence

## Source YAML

```yaml
schema: mitase/spec/v1
kind: philosophies
namespace: foundation
category: Foundation
philosophies:
  - id: PHIL-003
    title: Build Rules, Not Disposable Artifacts
    summary: Encode product behavior as durable, reusable rules that can be continuously re-verified.
    principles:
      - id: durable-rules
        statement: Product behavior should be encoded as durable, reusable rules rather than manual one-off artifacts. Specifications, implementation, and validation are coupled so behavior can be continuously re-verified.
        applies_to: [product, specification, implementation, verification]
      - id: reusable-validation
        statement: Tie behavior changes to specs and tests in the same change set. Prefer reusable helpers, schemas, and automation over manual procedural steps, and avoid introducing logic that cannot be continuously validated in CI.
        applies_to: [implementation, verification]
    bindings:
      - id: architecture-guide
        role: documentation
        facet: architecture
        responsibility: Explain the durable rule foundation and its relationship to specification, implementation, and verification.
        targets:
          - id: architecture
            adapter: markdown
            path: docs/understand/model/v1-architecture.md
            selector: { kind: heading, value: Foundation hierarchy }
            claims:
              - kind: documents
                anchor: PHIL-003#principle.durable-rules
              - kind: documents
                anchor: PHIL-003#principle.reusable-validation

  - id: PHIL-005
    title: Authority Without Workflow Ownership
    summary: Mitase interprets and validates repository-owned meaning without owning repository workflow.
    principles:
      - id: authority-without-workflow-ownership
        statement: Mitase may be authoritative about the meaning of repository-owned specifications, their exact relationships, and whether declared repository evidence is structurally satisfied without becoming authoritative over how work is planned, implemented, executed, tested, reviewed, retried, or delivered.
        applies_to: [specification, interpretation, indexing, resolution, validation]
    bindings:
      - id: authority-boundary-guide
        role: documentation
        facet: authority
        responsibility: Explain the boundary between repository-owned specification authority and Mitase interpretation, indexing, resolution, and validation.
        targets:
          - id: architecture
            adapter: markdown
            path: docs/understand/model/v1-architecture.md
            selector: { kind: heading, value: Authority boundary }
            claims:
              - kind: documents
                anchor: PHIL-005#principle.authority-without-workflow-ownership

  - id: PHIL-006
    title: Evidence Before Authority
    summary: Treat declared relationships as authoritative only when their exact repository evidence resolves.
    principles:
      - id: resolved-evidence
        statement: A declared implementation or verification relationship becomes authoritative only after its exact repository targets resolve; unresolved declarations are diagnostics, not evidence.
        applies_to: [specification, resolution, validation]
    bindings:
      - id: evidence-boundary-guide
        role: documentation
        facet: evidence
        responsibility: Explain why exact resolution and current repository evidence are prerequisites for derived state.
        targets:
          - id: architecture
            adapter: markdown
            path: docs/understand/model/v1-architecture.md
            selector: { kind: heading, value: Evidence and derived state }
            claims:
              - kind: documents
                anchor: PHIL-006#principle.resolved-evidence
```
