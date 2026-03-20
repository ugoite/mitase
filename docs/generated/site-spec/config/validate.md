---
title: "Configuration / Validate"
description: "Generated reference for docs/syu/config/validate.yaml"
---

> Generated from `docs/syu/config/validate.yaml`.

## Parsed content

### Category

- Configuration

### Version

- 1

### Section

- validate

### Precedence

- **default_fix**:
  - --fix
  - --no-fix
  - validate.default_fix

### Items

- **key**: validate.default_fix
  - **type**: boolean
  - **default**: False
  - **summary**: Enables conservative autofix by default.
  - **description**:
    - |
      When `true`, `syu validate` behaves as if `--fix` was passed unless the
      user explicitly disables fixes with `--no-fix`.
- **key**: validate.allow_planned
  - **type**: boolean
  - **default**: True
  - **summary**: Controls whether `planned` requirements and features are accepted.
  - **description**:
    - |
      `planned` items remain valid only while they avoid declaring real traces.
      Setting this to `false` tightens a workspace so backlog-style entries are
      rejected entirely.
- **key**: validate.require_non_orphaned_items
  - **type**: boolean
  - **default**: True
  - **summary**: Rejects isolated philosophy, policy, requirement, and feature nodes.
  - **description**:
    - |
      This keeps the adjacent-layer graph deliberate by requiring every item to
      stay connected to at least one neighboring layer.
- **key**: validate.require_symbol_trace_coverage
  - **type**: boolean
  - **default**: False
  - **summary**: Enforces ownership for public Rust symbols and Rust tests.
  - **description**:
    - |
      When enabled, `syu` requires every public Rust symbol to belong to some
      feature and every Rust test to belong to some requirement, in addition to
      verifying declared traces.

## Source YAML

```yaml
category: Configuration
version: 1
section: validate
precedence:
  default_fix:
    - --fix
    - --no-fix
    - validate.default_fix
items:
  - key: validate.default_fix
    type: boolean
    default: false
    summary: Enables conservative autofix by default.
    description: |
      When `true`, `syu validate` behaves as if `--fix` was passed unless the
      user explicitly disables fixes with `--no-fix`.
  - key: validate.allow_planned
    type: boolean
    default: true
    summary: Controls whether `planned` requirements and features are accepted.
    description: |
      `planned` items remain valid only while they avoid declaring real traces.
      Setting this to `false` tightens a workspace so backlog-style entries are
      rejected entirely.
  - key: validate.require_non_orphaned_items
    type: boolean
    default: true
    summary: Rejects isolated philosophy, policy, requirement, and feature nodes.
    description: |
      This keeps the adjacent-layer graph deliberate by requiring every item to
      stay connected to at least one neighboring layer.
  - key: validate.require_symbol_trace_coverage
    type: boolean
    default: false
    summary: Enforces ownership for public Rust symbols and Rust tests.
    description: |
      When enabled, `syu` requires every public Rust symbol to belong to some
      feature and every Rust test to belong to some requirement, in addition to
      verifying declared traces.
```
