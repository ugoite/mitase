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
  - **summary**: Enforces ownership for public Rust, Python, Go, Java, C#, and TypeScript/JavaScript symbols and tests.
  - **description**:
    - |
      When enabled, `syu` requires every public Rust, Python, Go, Java, C#,
      and TypeScript/JavaScript symbol to belong to some feature and every
      test in those inventoried languages to belong to some requirement, in
      addition to verifying declared traces. The
      validate command can enable or disable this for one run with
      `--require-symbol-trace-coverage` or
      `--require-symbol-trace-coverage=false`.
- **key**: validate.trace_ownership_mode
  - **type**: enum(mapping|inline|sidecar)
  - **default**: mapping
  - **summary**: Chooses whether trace YAML alone is sufficient or ownership must also be declared inline or in a sidecar manifest.
  - **description**:
    - |
      `mapping` keeps the current default where checked-in requirement and
      feature trace mappings are enough to audit ownership. `inline` adds an
      extra breadcrumb by requiring traced source and test files to mention
      their owning requirement or feature ID directly. `sidecar` keeps the YAML
      trace mapping as the canonical file/symbol link but requires each traced
      file to have an adjacent `&lt;file&gt;.syu-ownership.yaml` manifest with
      matching owner IDs and symbols. Autofix updates the sidecar manifest
      instead of inserting IDs into source when `sidecar` is enabled.
      Repository-relative generated paths listed in
      `validate.symbol_trace_coverage_ignored_paths` stay opted out of the
      extra `SYU-trace-id-001` ownership requirement in `inline` and `sidecar`
      mode so generated outputs do not need inline IDs or sidecar manifests.
- **key**: validate.symbol_trace_coverage_ignored_paths
  - **type**: array&lt;path&gt;
  - **default**:
    - build
    - coverage
    - dist
    - target
    - app/build
    - app/coverage
    - app/dist
    - app/target
    - tests/fixtures/workspaces
  - **summary**: Exact repository-relative directories that strict symbol coverage skips.
  - **description**:
    - |
      These paths are matched exactly against repository-relative directories
      while `syu` inventories public symbols and tests for strict ownership
      checks. The defaults skip common build outputs such as `app/dist`,
      repository-root `build/`, `coverage/`, `dist/`, and `target/`, plus the
      checked-in `tests/fixtures/workspaces/` sample repositories, without
      hiding authored nested paths like `src/build/`. The same list also opts
      those generated paths out of the extra inline or sidecar ownership
      breadcrumb enforced by `validate.trace_ownership_mode`, so generated
      artifacts do not fail `SYU-trace-id-001` just because they lack checked-in
      IDs or adjacent ownership manifests. Set this list to `[]` when you
      intentionally want generated artifacts to count toward strict trace
      coverage and ownership enforcement too.

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
    summary: Enforces ownership for public Rust, Python, Go, Java, C#, and TypeScript/JavaScript symbols and tests.
    description: |
      When enabled, `syu` requires every public Rust, Python, Go, Java, C#,
      and TypeScript/JavaScript symbol to belong to some feature and every
      test in those inventoried languages to belong to some requirement, in
      addition to verifying declared traces. The
      validate command can enable or disable this for one run with
      `--require-symbol-trace-coverage` or
      `--require-symbol-trace-coverage=false`.
  - key: validate.trace_ownership_mode
    type: enum(mapping|inline|sidecar)
    default: mapping
    summary: Chooses whether trace YAML alone is sufficient or ownership must also be declared inline or in a sidecar manifest.
    description: |
      `mapping` keeps the current default where checked-in requirement and
      feature trace mappings are enough to audit ownership. `inline` adds an
      extra breadcrumb by requiring traced source and test files to mention
      their owning requirement or feature ID directly. `sidecar` keeps the YAML
      trace mapping as the canonical file/symbol link but requires each traced
      file to have an adjacent `<file>.syu-ownership.yaml` manifest with
      matching owner IDs and symbols. Autofix updates the sidecar manifest
      instead of inserting IDs into source when `sidecar` is enabled.
      Repository-relative generated paths listed in
      `validate.symbol_trace_coverage_ignored_paths` stay opted out of the
      extra `SYU-trace-id-001` ownership requirement in `inline` and `sidecar`
      mode so generated outputs do not need inline IDs or sidecar manifests.
  - key: validate.symbol_trace_coverage_ignored_paths
    type: array<path>
    default:
      - build
      - coverage
      - dist
      - target
      - app/build
      - app/coverage
      - app/dist
      - app/target
      - tests/fixtures/workspaces
    summary: Exact repository-relative directories that strict symbol coverage skips.
    description: |
      These paths are matched exactly against repository-relative directories
      while `syu` inventories public symbols and tests for strict ownership
      checks. The defaults skip common build outputs such as `app/dist`,
      repository-root `build/`, `coverage/`, `dist/`, and `target/`, plus the
      checked-in `tests/fixtures/workspaces/` sample repositories, without
      hiding authored nested paths like `src/build/`. The same list also opts
      those generated paths out of the extra inline or sidecar ownership
      breadcrumb enforced by `validate.trace_ownership_mode`, so generated
      artifacts do not fail `SYU-trace-id-001` just because they lack checked-in
      IDs or adjacent ownership manifests. Set this list to `[]` when you
      intentionally want generated artifacts to count toward strict trace
      coverage and ownership enforcement too.
```
