# syu configuration

<!-- FEAT-DOCS-001 -->

`syu` reads `syu.yaml` from the workspace root.

The self-hosted repository also keeps a structured configuration reference
under `docs/syu/config/`:

- `docs/syu/config/overview.yaml`
- `docs/syu/config/spec.yaml`
- `docs/syu/config/validate.yaml`
- `docs/syu/config/workbench.yaml`
- `docs/syu/config/report.yaml`
- `docs/syu/config/runtimes.yaml`

Add new supported config items there first, then update this guide when the
change also needs narrative explanation or new examples.

## Key concepts

Before reading the field reference below, it helps to know what the validation
flags are actually controlling:

**Criterion closure**
A requirement criterion is complete only when governance, implementation, and
verification all connect. In v1 that means the criterion names governing policy
rules, at least one implementation binding satisfies it, and at least one
verification binding verifies it.

**Binding target**
A binding target is the concrete file or selector a v1 binding owns. Examples
include a Rust symbol, an OpenAPI operation, a Markdown heading, or a whole
file selector. The planner and validator use those targets to prove that the
declared implementation or verification surface exists.

**Contract counterpart**
Contracts connect multiple bindings around one shared interface or interaction.
For example, an HTTP contract can name one provider binding and one consumer
binding. v1 validation checks that all required counterparts remain present and
consistent when work plans touch a contracted area.

**Changed impact**
When changed-file validation is enabled, syu compares the changed artifacts and
spec anchors against the current v1 graph. That lets it detect missing
implementation, verification, or contract follow-through instead of only
checking for legacy reciprocal links.

## Minimal configuration

```yaml
schema: syu/config/v1
workspace:
  spec_roots: [docs/syu]
  artifact_roots: [src, tests]
  excludes: []
profiles: { active: [], custom: {} }
validation:
  preset: standard
  deny_warnings: false
  rules: {}
  changed:
    require_owned_changes: false
work:
  slicing:
    max_editable_files: 4
    max_editable_symbols: 8
    max_verification_targets: 4
    max_readonly_targets: 8
    max_total_bytes: 16384
  context:
    include_parent_principles: false
    include_parent_rules: false
adapters: { enabled: [rust, markdown, yaml, json] }
```

## Fields

### `version`

The `syu` CLI version that generated the config. `syu init` keeps this aligned
with the running binary. For backwards compatibility, legacy numeric values are
still accepted when reading existing configs.

### `workspace.spec_roots`

Controls where `syu` reads v1 spec documents.

Use a relative path for normal workspaces:

```yaml
workspace:
  spec_roots: [docs/syu]
  artifact_roots: [src, tests]
  excludes: []
```

New workspaces default to `docs/syu`. Existing repositories can keep another
layout, including `docs/spec`, by setting `workspace.spec_roots` explicitly.

When you are starting a brand-new workspace, `syu init --spec-root docs/spec`
scaffolds the starter files into that repository-relative path immediately and
writes the matching `workspace.spec_roots` entry for you.

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

When `true`, `syu` scans Rust, Python, Go, Java, C#, Kotlin, and TypeScript/JavaScript source and test files, plus Ruby source and test files, to confirm that every public symbol belongs to some feature and every test belongs to some requirement.

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
or `syu init . --template go-only` as a reminder that Go now supports symbol
checks, coverage ownership, and `doc_contains`, and that Ruby, Java, C#, and
Kotlin do too.

### `validate.historical_ids.enabled`

Controls whether `syu` rejects IDs that were already deleted somewhere in Git
history.

- `true`: a philosophy, policy, requirement, or feature cannot reuse an ID
  that was previously deleted in the repository history
- `false`: the historical ID rule is skipped, which is useful for temporary
  migrations that are renaming old data

Keep this enabled in steady state so a deleted ID stays retired. If you are
intentionally migrating legacy content, turn it off for the migration window
and restore it after the old identifiers have been replaced.

### `validate.historical_ids.start_ref`

Optional Git ref that limits the historical-ID scan to commits after a chosen
baseline.

Use this when a migration wants to compare against a known branch point instead
of the full repository history. The default is to scan the entire history that
Git exposes from `HEAD`.

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
`coverage/`, `dist/`, `target/`, and the checked-in
`tests/fixtures/workspaces/` repositories without hiding authored nested paths
like `src/build/`.

This same list also opts those generated paths out of the extra ownership
breadcrumb enforced by `validate.trace_ownership_mode: inline` or `sidecar`.
That keeps generated assets from failing `SYU-trace-id-001` just because they
do not carry inline IDs or adjacent ownership manifests. Set the list to `[]`
when you intentionally want generated outputs to participate in both strict
coverage inventory and ownership enforcement.

### `workbench.bind`

Sets the default listener address for `syu workbench`.

- `127.0.0.1`: the Workbench binds only to the local machine by default
- any other IP address: requires an explicit `--allow-remote-bind` launch flag

Keep the default loopback value unless you deliberately want the server to be
reachable from another host.

### `workbench.port`

Sets the default TCP port for `syu workbench`.

Use one repository-native port so browser and desktop clients can point to the
same typed API surface. The command-line `--port` flag still overrides the
config value for ad hoc runs.

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
- treat runtime overrides as environment-specific, not project-specific, unless
  your team truly needs a pinned executable name
