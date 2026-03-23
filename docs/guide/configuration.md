# syu configuration

<!-- FEAT-DOCS-001 -->

`syu` reads `syu.yaml` from the workspace root.

The self-hosted repository also keeps a structured configuration reference
under `docs/syu/config/`:

- `docs/syu/config/overview.yaml`
- `docs/syu/config/spec.yaml`
- `docs/syu/config/validate.yaml`
- `docs/syu/config/app.yaml`
- `docs/syu/config/report.yaml`
- `docs/syu/config/runtimes.yaml`

Add new supported config items there first, then update this guide when the
change also needs narrative explanation or new examples.

## Minimal configuration

```yaml
version: 0.0.1-alpha.7
spec:
  root: docs/syu
validate:
  default_fix: false
  allow_planned: true
  require_non_orphaned_items: true
  require_reciprocal_links: true
  require_symbol_trace_coverage: false
app:
  bind: 127.0.0.1
  port: 3000
runtimes:
  python:
    command: auto
  node:
    command: auto
```

## Fields

### `version`

The `syu` CLI version that generated the config. `syu init` keeps this aligned
with the running binary. For backwards compatibility, legacy numeric values are
still accepted when reading existing configs.

### `spec.root`

Controls where `syu` reads philosophy, policy, requirements, and features.

Use a relative path for normal workspaces:

```yaml
spec:
  root: docs/syu
```

New workspaces default to `docs/syu`. Existing repositories can keep another
layout, including `docs/spec`, by setting `spec.root` explicitly.

### `validate.default_fix`

When `true`, `syu validate` behaves as if `--fix` was passed unless the user
explicitly provides `--no-fix`.

### `validate.allow_planned`

Controls whether `planned` requirements and features are allowed.

- `true`: `planned` items are valid, but they must not declare traces yet
- `false`: any `planned` or legacy `planed` status is rejected

### `validate.require_non_orphaned_items`

When `true`, philosophy, policy, requirement, and feature entries must each
connect to at least one adjacent layer. This is on by default because isolated
definitions usually mean the specification has drifted away from the repository.

### `validate.require_reciprocal_links`

When `true`, adjacent-layer relationships must be confirmed from both sides.

- `true`: `SYU-graph-reciprocal-001` remains an error
- `false`: missing backlinks stop failing validation, but broken references still do

Keep this enabled for steady-state self-hosting. Turning it off is mainly useful
when a repository is migrating an existing spec graph and wants to phase in
backlinks after the forward links are already trustworthy.

### `validate.require_symbol_trace_coverage`

When `true`, `syu` scans Rust source and test files to confirm that every public
symbol belongs to some feature and every test belongs to some requirement.

- `false`: only declared traces are verified
- `true`: undeclared public APIs and tests become validation errors

This is useful once the repository wants maintenance work to stay fully owned by
the specification.

### `app.bind`

Controls the default address that `syu app` binds to.

Use `127.0.0.1` for a localhost-only browser app or `0.0.0.0` when a demo or
container workflow needs the server to be reachable from outside the process.

### `app.port`

Controls the default port that `syu app` binds to.

CLI flags still override the config so temporary port conflicts can be resolved
without editing the repository.

### `report.output`

Sets the default Markdown destination for `syu report`.

Use a repository-relative path such as:

```yaml
report:
  output: docs/generated/syu-report.md
```

When set in `syu.yaml`, the path is resolved from the workspace root. `--output`
still overrides the config, and relative config paths must stay inside the
workspace root so checked-in report destinations cannot escape the repository.

### `runtimes.python.command`

Controls which Python executable `syu` uses for Python inspection.

Use `auto` to let `syu` search `python3` and then `python`.

### `runtimes.node.command`

Reserved for runtime-backed Node.js workflows. Today the TypeScript inspector is
bundled, but keeping the runtime configurable now makes future integrations more
predictable.

## CLI precedence

For autofix behavior, CLI flags override config:

1. `--fix`
2. `--no-fix`
3. `validate.default_fix`

`validate.allow_planned` is configuration-only. There is no CLI flag to
override it.

`validate.require_non_orphaned_items`,
`validate.require_reciprocal_links`, and
`validate.require_symbol_trace_coverage` are also configuration-only.

For report output paths, CLI flags override config:

1. `--output`
2. `report.output`
3. stdout

For the browser app, CLI flags override config:

1. `--bind`
2. `app.bind`
3. `127.0.0.1`

1. `--port`
2. `app.port`
3. `3000`

## Wildcard file ownership

Traces may use `symbols: ['*']` when one requirement or feature intentionally
owns every relevant symbol in a file:

```yaml
implementations:
  rust:
    - file: src/report.rs
      symbols:
        - "*"
```

This is especially useful for focused modules and self-hosted repositories that
want strict ownership checks without enumerating every public symbol by hand.

## Recommended practice

- keep `syu.yaml` in the workspace root
- check it into version control
- set `validate.allow_planned: false` once your branch or release line should
  forbid backlog items
- leave `validate.require_non_orphaned_items: true` unless you are doing a
  deliberate migration
- leave `validate.require_reciprocal_links: true` unless you are phasing in
  backlinks after stabilizing the forward graph
- turn on `validate.require_symbol_trace_coverage: true` once the repository
  wants public APIs and tests to remain fully owned by the spec
- set `report.output` when your repository checks in one stable report artifact
  path
- set `app.bind` and `app.port` only when your team really has a stable local
  browser-app convention worth checking in
- set `report.output` when your repository checks in one stable report artifact
  path
- treat runtime overrides as environment-specific, not project-specific, unless
  your team truly needs a pinned executable name
