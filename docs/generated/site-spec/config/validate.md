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
- **allow_planned**:
  - --allow-planned[=true|false]
  - validate.allow_planned
- **require_non_orphaned_items**:
  - --require-non-orphaned-items[=true|false]
  - validate.require_non_orphaned_items
- **require_reciprocal_links**:
  - --require-reciprocal-links[=true|false]
  - validate.require_reciprocal_links
- **require_symbol_trace_coverage**:
  - --require-symbol-trace-coverage[=true|false]
  - validate.require_symbol_trace_coverage

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
      rejected entirely. The validate command can also override this value for a
      single run with `--allow-planned` or `--allow-planned=false`.
- **key**: validate.require_non_orphaned_items
  - **type**: boolean
  - **default**: True
  - **summary**: Rejects isolated philosophy, policy, requirement, and feature nodes.
  - **description**:
    - |
      This keeps the adjacent-layer graph deliberate by requiring every item to
      stay connected to at least one neighboring layer. Use
      `--require-non-orphaned-items=false` when a migration needs a temporary
      exception without changing the checked-in config.
- **key**: validate.require_reciprocal_links
  - **type**: boolean
  - **default**: True
  - **summary**: Requires adjacent-layer links to be confirmed from both sides.
  - **description**:
    - |
      This keeps navigation explainable by making philosophy, policy,
      requirement, and feature relationships agree in both directions.
      Repositories doing a gradual migration can temporarily set it to `false`
      while still keeping broken-reference validation enabled. For one-off runs,
      `syu validate . --require-reciprocal-links=false` provides the same
      temporary relaxation without editing `syu.yaml`.
- **key**: validate.require_symbol_trace_coverage
  - **type**: boolean
  - **default**: False
  - **summary**: Enforces ownership for public Rust symbols and Rust tests.
  - **description**:
    - |
      When enabled, `syu` requires every public Rust symbol to belong to some
      feature and every Rust test to belong to some requirement, in addition to
      verifying declared traces. The validate command can enable or disable this
      for one run with `--require-symbol-trace-coverage` or
      `--require-symbol-trace-coverage=false`.

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
  allow_planned:
    - --allow-planned[=true|false]
    - validate.allow_planned
  require_non_orphaned_items:
    - --require-non-orphaned-items[=true|false]
    - validate.require_non_orphaned_items
  require_reciprocal_links:
    - --require-reciprocal-links[=true|false]
    - validate.require_reciprocal_links
  require_symbol_trace_coverage:
    - --require-symbol-trace-coverage[=true|false]
    - validate.require_symbol_trace_coverage
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
      rejected entirely. The validate command can also override this value for a
      single run with `--allow-planned` or `--allow-planned=false`.
  - key: validate.require_non_orphaned_items
    type: boolean
    default: true
    summary: Rejects isolated philosophy, policy, requirement, and feature nodes.
    description: |
      This keeps the adjacent-layer graph deliberate by requiring every item to
      stay connected to at least one neighboring layer. Use
      `--require-non-orphaned-items=false` when a migration needs a temporary
      exception without changing the checked-in config.
  - key: validate.require_reciprocal_links
    type: boolean
    default: true
    summary: Requires adjacent-layer links to be confirmed from both sides.
    description: |
      This keeps navigation explainable by making philosophy, policy,
      requirement, and feature relationships agree in both directions.
      Repositories doing a gradual migration can temporarily set it to `false`
      while still keeping broken-reference validation enabled. For one-off runs,
      `syu validate . --require-reciprocal-links=false` provides the same
      temporary relaxation without editing `syu.yaml`.
  - key: validate.require_symbol_trace_coverage
    type: boolean
    default: false
    summary: Enforces ownership for public Rust symbols and Rust tests.
    description: |
      When enabled, `syu` requires every public Rust symbol to belong to some
      feature and every Rust test to belong to some requirement, in addition to
      verifying declared traces. The validate command can enable or disable this
      for one run with `--require-symbol-trace-coverage` or
      `--require-symbol-trace-coverage=false`.
```
