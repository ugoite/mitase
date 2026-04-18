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
stricter mode for mature repositories.

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

When `true`, `syu` scans Rust, Python, and TypeScript/JavaScript source and test
files to confirm that every public symbol belongs to some feature and every
test belongs to some requirement. Declared Go traces are still validated, but
Go is not yet part of this strict undeclared-symbol inventory.

- `false`: only declared traces are verified
- `true`: undeclared public APIs and tests become validation errors

This is useful once the repository wants maintenance work to stay fully owned by
the specification across the supported implementation languages.
For an experimental strict run, use `syu validate . --require-symbol-trace-coverage`.
Before you turn it on in a mixed-language repository, review the [trace adapter
capability matrix](./trace-adapter-support.md) so you only depend on adapters
that currently participate in the strict inventory.

For polyglot repositories, treat this as a staged switch instead of an all-or-
nothing starting point:

1. Keep `require_symbol_trace_coverage: false` while the repository is still
   mixing supported and unsupported implementation languages or while older
   areas have not been traced yet.
2. Keep declaring requirement and feature traces for every area, but use the
   trace shape the current validator can actually check. For supported
   lightweight adapters such as `shell`, `yaml`, `json`, `markdown`, and
   `gitignore`, that usually means file-level, explicit-symbol, or wildcard
   ownership without `doc_contains`. For unsupported implementation languages
   such as Go, Java, or C#, keep the spec layers linked and defer code-level
   mappings until adapter support lands.
3. Turn `require_symbol_trace_coverage: true` on once the supported-language
   parts of the repository are ready to keep every public API and test owned by
   the spec, while unsupported-language areas stay at the spec-layer guidance
   stage or on supported lightweight adapters until richer adapters exist.

The important rule is honesty: only promise symbol-level strictness where `syu`
can verify it today, and use the looser trace forms to keep the rest of a
polyglot repository connected to the spec without pretending the validator can
inspect more than it really can.
See [getting started](./getting-started.md#unsupported-implementation-languages-can-still-adopt-the-spec-layers-first)
for the current adoption path and roadmap links.

### `validate.symbol_trace_coverage_ignored_paths`

Controls which repository-relative directories strict symbol coverage skips.

The defaults are exact paths, not basename wildcards:

```yaml
validate:
  symbol_trace_coverage_ignored_paths:
    - build
    - coverage
    - dist
    - target
    - app/build
    - app/coverage
    - app/dist
    - app/target
```

That keeps common generated build output out of strict ownership checks without
hiding authored nested directories such as `src/build/` or `tests/coverage/`.
Set the list to `[]` if you intentionally want generated artifacts to count
toward symbol-trace coverage, or replace it with your own exact paths when your
repository uses a different layout.

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
  wants public APIs and tests to remain fully owned by the spec across the
  currently supported languages
- for mixed-language repositories, keep `validate.require_symbol_trace_coverage:
  false` during adoption and use explicit file or wildcard ownership for areas
  that the current language adapters cannot inspect yet
- set `report.output` when your repository checks in one stable report artifact
  path
- set `app.bind` and `app.port` only when your team really has a stable local
  browser-app convention worth checking in
- set `report.output` when your repository checks in one stable report artifact
  path
- treat runtime overrides as environment-specific, not project-specific, unless
  your team truly needs a pinned executable name
