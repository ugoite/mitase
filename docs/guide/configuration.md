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

## Key concepts

Before reading the field reference below, it helps to know what the validation
flags are actually controlling:

**Orphaned item**
A spec item (philosophy, policy, requirement, or feature) that has no links to
any adjacent layer. For example, a philosophy with no linked policies, or a
feature with no linked requirements. Orphans usually mean the specification has
drifted — you defined something but never connected it to the rest of the graph.
`require_non_orphaned_items` enforces that every item is reachable.

**Reciprocal link**
syu's spec graph is bidirectional: if a requirement links to a feature, the
feature must also list that requirement in its `linked_requirements`. A
reciprocal link is this two-way confirmation. `require_reciprocal_links`
enforces both directions so the graph stays consistent even when files are
edited independently.

**Symbol trace**
A *symbol* is a named function, method, or class in your source code.
A symbol trace is a declaration in a requirement or feature YAML that names the
specific symbols (and optionally a required doc-comment string) that implement
or test that spec item. Symbol traces let `syu` verify that the code actually
exists at the claimed location.

**Symbol trace coverage**
When `require_symbol_trace_coverage: true`, syu additionally checks that every
*public* symbol in the relevant source files is claimed by at least one spec
item, and that every test function is claimed by at least one requirement. 100%
coverage means no public API or test is left undeclared. This is an optional
stricter mode for mature repositories. Use
[`trace-adapter-support.md`](./trace-adapter-support.md) when you need the
current per-language matrix for symbol discovery, `doc_contains`, and strict
coverage behavior.

## Minimal configuration

```yaml
version: 0.0.1-alpha.8
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

When you are starting a brand-new workspace, `syu init --spec-root docs/spec`
scaffolds the starter files into that repository-relative path immediately and
writes the matching `spec.root` value for you.

### `validate.default_fix`

When `true`, `syu validate` behaves as if `--fix` was passed unless the user
explicitly provides `--no-fix`.

### `validate.allow_planned`

Controls whether `planned` requirements and features are allowed.

- `true`: `planned` items are valid, but they must not declare traces yet
- `false`: any `planned` or legacy `planed` status is rejected

Use `syu validate . --allow-planned` or `syu validate . --allow-planned=false`
when you want to trial a looser or stricter run without editing `syu.yaml`.

### `validate.require_non_orphaned_items`

When `true`, philosophy, policy, requirement, and feature entries must each
connect to at least one adjacent layer. This is on by default because isolated
definitions usually mean the specification has drifted away from the repository.

Use `syu validate . --require-non-orphaned-items=false` for a one-off migration
run when you do not want to commit a config change.

### `validate.require_reciprocal_links`

When `true`, adjacent-layer relationships must be confirmed from both sides.

- `true`: `SYU-graph-reciprocal-001` remains an error
- `false`: missing backlinks stop failing validation, but broken references still do

Keep this enabled for steady-state self-hosting. Turning it off is mainly useful
when a repository is migrating an existing spec graph and wants to phase in
backlinks after the forward links are already trustworthy.
For one-off runs, use `syu validate . --require-reciprocal-links=false` instead
of editing `syu.yaml`.

### `validate.require_symbol_trace_coverage`

When `true`, `syu` scans Rust, Python, Go, Java, C#, and TypeScript/JavaScript
source and test files to confirm that every public symbol belongs to some
feature and every test belongs to some requirement.

- `false`: only declared traces are verified
- `true`: undeclared public APIs and tests become validation errors

This is useful once the repository wants maintenance work to stay fully owned by
the specification across the supported implementation languages. Strict
coverage still skips configured repository-relative generated paths such as
`build/`, `coverage/`, `dist/`, and `target/` so authored files nested under
`src/` or `tests/` keep counting.
For an experimental strict run, use `syu validate . --require-symbol-trace-coverage`.
If you want a gradual C# rollout, borrow the lighter starter shape from the
[`examples/csharp-fallback` workspace on GitHub](https://github.com/ugoite/syu/tree/main/examples/csharp-fallback).
That example keeps the higher-layer spec and surrounding automation explicit
without requiring every checked-in C# file to be traced immediately. Use
[`examples/go-only` workspace on GitHub](https://github.com/ugoite/syu/tree/main/examples/go-only)
or `syu init . --template go-only` as a reminder that Go now supports
symbol checks, coverage ownership, and `doc_contains`.

### `validate.trace_ownership_mode`

Controls whether traced files need an extra ownership breadcrumb beyond the
checked-in requirement or feature trace mapping.

- `mapping`: the YAML trace entries are enough on their own
- `inline`: traced files must also mention their owning requirement or feature ID
- `sidecar`: traced files must carry ownership in adjacent `<file>.syu-ownership.yaml` manifests

Keep `mapping` when you want the lightest workflow. Use `inline` or `sidecar`
when repositories want a second ownership signal close to the code as well.
Generated paths listed in `validate.symbol_trace_coverage_ignored_paths` stay
opted out of the extra `SYU-trace-id-001` ownership check in `inline` and
`sidecar` mode so build outputs do not need checked-in IDs or sidecar manifests.
Declared traces in those files are still validated for file readability and
symbol existence.

### `validate.symbol_trace_coverage_ignored_paths`

Controls which repository-relative generated directories `syu` skips while
building the strict symbol-coverage inventory.

By default this list excludes common build outputs such as `build/`,
`coverage/`, `dist/`, `target/`, `app/dist`, and the checked-in
`tests/fixtures/workspaces/` repositories without hiding authored nested paths
like `src/build/`.

This same list also opts those generated paths out of the extra ownership
breadcrumb enforced by `validate.trace_ownership_mode: inline` or `sidecar`.
That keeps generated assets from failing `SYU-trace-id-001` just because they
do not carry inline IDs or adjacent ownership manifests. Set the list to `[]`
when you intentionally want generated outputs to participate in both strict
coverage inventory and ownership enforcement.

### `app.bind`

Controls the default address that `syu app` binds to.

Use `127.0.0.1` for a localhost-only browser app or `0.0.0.0` when a demo or
container workflow needs the server to be reachable from outside the process.
When the configured address is non-loopback, `syu app` still requires the
explicit `--allow-remote` flag before it starts serving workspace data.

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

For delivery and validation strictness, CLI flags override config for a single
invocation:

1. `--allow-planned[=true|false]`
2. `validate.allow_planned`

1. `--require-non-orphaned-items[=true|false]`
2. `validate.require_non_orphaned_items`

1. `--require-reciprocal-links[=true|false]`
2. `validate.require_reciprocal_links`

1. `--require-symbol-trace-coverage[=true|false]`
2. `validate.require_symbol_trace_coverage`

Passing the flag with no value means `true`. Use `=false` when you want a
temporary relaxed run without changing the checked-in config.

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
