---
title: "Policies / Policies"
description: "Generated reference for docs/spec/policies/policies.yaml"
---

> Generated from `docs/spec/policies/policies.yaml`.

## Parsed content

### Category

- Policies

### Version

- 1

### Language

- en

### Policies

- **id**: POL-001
  - **title**: Keep every layer adjacent, connected, and explainable
  - **summary**: Philosophy, policy, requirement, and feature should form a deliberate chain rather than isolated islands.
  - **description**:
    - |
      Specification-driven development becomes decorative when layers drift
      apart. `syu` should reject broken adjacency, reciprocal-link mismatches,
      and isolated definitions so teams can maintain a continuous story from
      philosophy to implementation.
  - **linked_philosophies**:
    - PHIL-001
    - PHIL-003
  - **linked_requirements**:
    - REQ-CORE-001
    - REQ-CORE-009
    - REQ-CORE-015
- **id**: POL-002
  - **title**: Validation should explain the current state instead of only failing
  - **summary**: Errors, reports, and browsing should make the layered model legible even when the workspace is broken.
  - **description**:
    - |
      A specification tool should not answer every problem with a wall of opaque
      failures. Validation should surface rule codes, summaries, and reasons;
      reports should preserve that explanation; and the interactive browser
      should still show the user what exists today.
  - **linked_philosophies**:
    - PHIL-001
    - PHIL-003
  - **linked_requirements**:
    - REQ-CORE-001
    - REQ-CORE-004
    - REQ-CORE-010
    - REQ-CORE-015
- **id**: POL-003
  - **title**: Traceability should prove ownership from specification to code and tests
  - **summary**: Declared traces should map to real files, real symbols, and optional full-file ownership.
  - **description**:
    - |
      Declared traces are only credible when `syu` can verify the referenced
      file, symbol, owning ID, and optional documentation snippets from the
      repository. When a project opts in, public APIs and tests should also be
      required to belong to some feature or requirement so maintenance work does
      not quietly outrun the specification.
  - **linked_philosophies**:
    - PHIL-001
    - PHIL-002
  - **linked_requirements**:
    - REQ-CORE-002
    - REQ-CORE-003
- **id**: POL-004
  - **title**: Starting, browsing, and configuring syu should stay low ceremony
  - **summary**: Setup and exploration should be quick enough that teams do not resent the tool.
  - **description**:
    - |
      A specification-driven workflow should not begin with directory trivia or
      require users to memorize the model before they can look around. `syu
      init`, the root config, interactive browse mode, and approachable help
      text should make adoption feel incremental rather than invasive.
  - **linked_philosophies**:
    - PHIL-003
  - **linked_requirements**:
    - REQ-CORE-009
    - REQ-CORE-011
    - REQ-CORE-012
    - REQ-CORE-015
- **id**: POL-005
  - **title**: Documentation and examples must lower adoption friction
  - **summary**: Guides, reports, sites, and examples are part of the product surface.
  - **description**:
    - |
      Users should not need tribal knowledge to understand philosophy, policy,
      requirements, features, config, or commands. English guides, a rendered
      docs site, examples, and checked-in generated artifacts should explain not
      just how `syu` works, but why the workflow exists.
  - **linked_philosophies**:
    - PHIL-002
    - PHIL-003
  - **linked_requirements**:
    - REQ-CORE-004
    - REQ-CORE-010
    - REQ-CORE-012
    - REQ-CORE-016
- **id**: POL-006
  - **title**: The self-hosted repository should demonstrate polyglot, contributor-friendly use
  - **summary**: `syu` should prove that its model works in the kind of mixed repositories it claims to support.
  - **description**:
    - |
      `syu` should use its own ideas to support contributors. The repository
      should provide examples, tests for those examples, a devcontainer, and a
      low-friction pre-commit setup that make the documented workflow real.
  - **linked_philosophies**:
    - PHIL-002
    - PHIL-003
  - **linked_requirements**:
    - REQ-CORE-002
    - REQ-CORE-011
    - REQ-CORE-012
    - REQ-CORE-013
    - REQ-CORE-014
    - REQ-CORE-016
- **id**: POL-007
  - **title**: Delivery automation must remain trustworthy, secure, and reviewable
  - **summary**: Quality gates, release flow, packaging, and installation are part of the product contract.
  - **description**:
    - |
      Shipping the tool is part of the tool. Quality gates, coverage,
      release-please automation, track-specific release notes, release
      artifacts, package publication, and installation should be explicit,
      reviewable, and continuously validated.
  - **linked_philosophies**:
    - PHIL-001
    - PHIL-003
  - **linked_requirements**:
    - REQ-CORE-005
    - REQ-CORE-006
    - REQ-CORE-007
    - REQ-CORE-008
    - REQ-CORE-014

## Source YAML

```yaml
category: Policies
version: 1
language: en

policies:
  - id: POL-001
    title: Keep every layer adjacent, connected, and explainable
    summary: "Philosophy, policy, requirement, and feature should form a deliberate chain rather than isolated islands."
    description: |
      Specification-driven development becomes decorative when layers drift
      apart. `syu` should reject broken adjacency, reciprocal-link mismatches,
      and isolated definitions so teams can maintain a continuous story from
      philosophy to implementation.
    linked_philosophies:
      - PHIL-001
      - PHIL-003
    linked_requirements:
      - REQ-CORE-001
      - REQ-CORE-009
      - REQ-CORE-015

  - id: POL-002
    title: Validation should explain the current state instead of only failing
    summary: "Errors, reports, and browsing should make the layered model legible even when the workspace is broken."
    description: |
      A specification tool should not answer every problem with a wall of opaque
      failures. Validation should surface rule codes, summaries, and reasons;
      reports should preserve that explanation; and the interactive browser
      should still show the user what exists today.
    linked_philosophies:
      - PHIL-001
      - PHIL-003
    linked_requirements:
      - REQ-CORE-001
      - REQ-CORE-004
      - REQ-CORE-010
      - REQ-CORE-015

  - id: POL-003
    title: Traceability should prove ownership from specification to code and tests
    summary: "Declared traces should map to real files, real symbols, and optional full-file ownership."
    description: |
      Declared traces are only credible when `syu` can verify the referenced
      file, symbol, owning ID, and optional documentation snippets from the
      repository. When a project opts in, public APIs and tests should also be
      required to belong to some feature or requirement so maintenance work does
      not quietly outrun the specification.
    linked_philosophies:
      - PHIL-001
      - PHIL-002
    linked_requirements:
      - REQ-CORE-002
      - REQ-CORE-003

  - id: POL-004
    title: Starting, browsing, and configuring syu should stay low ceremony
    summary: "Setup and exploration should be quick enough that teams do not resent the tool."
    description: |
      A specification-driven workflow should not begin with directory trivia or
      require users to memorize the model before they can look around. `syu
      init`, the root config, interactive browse mode, and approachable help
      text should make adoption feel incremental rather than invasive.
    linked_philosophies:
      - PHIL-003
    linked_requirements:
      - REQ-CORE-009
      - REQ-CORE-011
      - REQ-CORE-012
      - REQ-CORE-015

  - id: POL-005
    title: Documentation and examples must lower adoption friction
    summary: "Guides, reports, sites, and examples are part of the product surface."
    description: |
      Users should not need tribal knowledge to understand philosophy, policy,
      requirements, features, config, or commands. English guides, a rendered
      docs site, examples, and checked-in generated artifacts should explain not
      just how `syu` works, but why the workflow exists.
    linked_philosophies:
      - PHIL-002
      - PHIL-003
    linked_requirements:
      - REQ-CORE-004
      - REQ-CORE-010
      - REQ-CORE-012
      - REQ-CORE-016

  - id: POL-006
    title: The self-hosted repository should demonstrate polyglot, contributor-friendly use
    summary: "`syu` should prove that its model works in the kind of mixed repositories it claims to support."
    description: |
      `syu` should use its own ideas to support contributors. The repository
      should provide examples, tests for those examples, a devcontainer, and a
      low-friction pre-commit setup that make the documented workflow real.
    linked_philosophies:
      - PHIL-002
      - PHIL-003
    linked_requirements:
      - REQ-CORE-002
      - REQ-CORE-011
      - REQ-CORE-012
      - REQ-CORE-013
      - REQ-CORE-014
      - REQ-CORE-016

  - id: POL-007
    title: Delivery automation must remain trustworthy, secure, and reviewable
    summary: "Quality gates, release flow, packaging, and installation are part of the product contract."
    description: |
      Shipping the tool is part of the tool. Quality gates, coverage,
      release-please automation, track-specific release notes, release
      artifacts, package publication, and installation should be explicit,
      reviewable, and continuously validated.
    linked_philosophies:
      - PHIL-001
      - PHIL-003
    linked_requirements:
      - REQ-CORE-005
      - REQ-CORE-006
      - REQ-CORE-007
      - REQ-CORE-008
      - REQ-CORE-014
```
