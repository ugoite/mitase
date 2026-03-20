---
title: "Graph"
description: "Generated reference for docs/spec/errors/graph.yaml"
---

> Generated from `docs/spec/errors/graph.yaml`.

## Parsed content

### Genre

- Graph Integrity

### Version

- 1

### Rules

- **code**: missing-reference
  - **severity**: error
  - **title**: Linked definitions must exist
  - **summary**: A declared relationship is only meaningful when the target actually exists.
  - **description**:
    - |
      The layered model only works when every forward link resolves to a real
      definition. Missing references silently break the chain from philosophy to
      implementation and make it impossible to explain why a change exists.
      This rule forces contributors to either declare the missing item or remove
      the stale relationship before the graph drifts further.
- **code**: reciprocal-link-missing
  - **severity**: error
  - **title**: Adjacent-layer links must be reciprocal
  - **summary**: Both sides of a relationship should agree that the relationship exists.
  - **description**:
    - |
      `syu` treats traceability as a graph that should be understandable in both
      directions. When only one side records a link, reviewers cannot reliably
      navigate from cause to effect or from implementation back to intent. This
      rule preserves explainability by requiring each adjacent layer to confirm
      the relationship from its own point of view.
- **code**: missing-links
  - **severity**: warning
  - **title**: Adjacent-layer expectations should be explicit
  - **summary**: Definitions should usually point to the layers they influence or satisfy.
  - **description**:
    - |
      Not every incomplete link is automatically fatal, but missing adjacent
      links are a strong smell that the spec is becoming harder to navigate.
      This warning nudges authors to keep the graph richly connected so the
      repository remains easy to browse, audit, and evolve over time.
- **code**: orphaned-definition
  - **severity**: error
  - **title**: Definitions must not be isolated from the layered graph
  - **summary**: A philosophy, policy, requirement, or feature should connect to at least one adjacent layer.
  - **description**:
    - |
      An isolated definition may contain prose, but it does not influence the
      rest of the repository. That means it cannot guide implementation,
      constrain behavior, or be justified by higher-level intent. This rule
      exists to keep every layer part of one connected explanation instead of
      allowing decorative or abandoned nodes to accumulate.

## Source YAML

```yaml
genre: Graph Integrity
version: 1

rules:
  - code: missing-reference
    severity: error
    title: Linked definitions must exist
    summary: A declared relationship is only meaningful when the target actually exists.
    description: |
      The layered model only works when every forward link resolves to a real
      definition. Missing references silently break the chain from philosophy to
      implementation and make it impossible to explain why a change exists.
      This rule forces contributors to either declare the missing item or remove
      the stale relationship before the graph drifts further.

  - code: reciprocal-link-missing
    severity: error
    title: Adjacent-layer links must be reciprocal
    summary: Both sides of a relationship should agree that the relationship exists.
    description: |
      `syu` treats traceability as a graph that should be understandable in both
      directions. When only one side records a link, reviewers cannot reliably
      navigate from cause to effect or from implementation back to intent. This
      rule preserves explainability by requiring each adjacent layer to confirm
      the relationship from its own point of view.

  - code: missing-links
    severity: warning
    title: Adjacent-layer expectations should be explicit
    summary: Definitions should usually point to the layers they influence or satisfy.
    description: |
      Not every incomplete link is automatically fatal, but missing adjacent
      links are a strong smell that the spec is becoming harder to navigate.
      This warning nudges authors to keep the graph richly connected so the
      repository remains easy to browse, audit, and evolve over time.

  - code: orphaned-definition
    severity: error
    title: Definitions must not be isolated from the layered graph
    summary: A philosophy, policy, requirement, or feature should connect to at least one adjacent layer.
    description: |
      An isolated definition may contain prose, but it does not influence the
      rest of the repository. That means it cannot guide implementation,
      constrain behavior, or be justified by higher-level intent. This rule
      exists to keep every layer part of one connected explanation instead of
      allowing decorative or abandoned nodes to accumulate.
```
