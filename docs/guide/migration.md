# Migration guide

Start here when you are upgrading an existing `syu` workspace between alpha
releases. This page documents breaking changes and release-specific upgrade
steps. When `syu validate .` starts failing after an upgrade, check the section
for the version you just installed.

> **Note:** `syu` is in alpha. The config schema, YAML spec format, and CLI
> flags may change in any alpha release. This guide is updated with every
> release that introduces user-visible breaks.

---

## Upgrading to `v0.0.1-alpha.7`

### New `syu.yaml` fields

| Field | Default | Notes |
|---|---|---|
| `validate.require_reciprocal_links` | `true` | New. Adjacent-layer links must be reciprocal. See [Understanding validation output](./getting-started.md#understanding-validation-output). |
| `report.output` | `stdout` | New. Optional. When unset, `syu report` prints to stdout as before. Set this to a file path to use a default output destination when `--output` is not provided. |

### Action required

**`validate.require_reciprocal_links: true` is on by default.**

If your spec has one-directional links (e.g. a requirement lists a feature but
the feature does not list the requirement back), `syu validate` will now fail
with `SYU-graph-reciprocal-001`.

To fix: add the missing back-references. Or, if your project needs a phased
migration, temporarily opt out:

```yaml
# syu.yaml
validate:
  require_reciprocal_links: false
```

### New validation rules enabled by default

| Code | Severity | Description |
|---|---|---|
| `SYU-graph-reciprocal-001` | error | Adjacent-layer links must be reciprocal |

### Spec root default

The `spec.root` default remains `docs/syu` (unchanged from alpha.6).

---

## Upgrading to `v0.0.1-alpha.6`

### `spec.root` default changed

| | alpha.5 | alpha.6 |
|---|---|---|
| Default `spec.root` | `docs/spec` | `docs/syu` |

If your `syu.yaml` relied on the implicit default (`docs/spec`) without
explicitly declaring `spec.root`, you must either:

1. Move your spec directory: `mv docs/spec docs/syu`, or
2. Add the following to `syu.yaml` to keep the old path:

   ```yaml
   spec:
     root: docs/spec
   ```

### New structured validation rule IDs

alpha.6 introduced the `SYU-*` rule code taxonomy. Error output format changed:

| Before (alpha.5) | After (alpha.6) |
|---|---|
| `error: orphaned item REQ-001` | `error[SYU-graph-orphaned-001] REQ-001` |

CI scripts that grep for the old error format may need updating.

---

## Upgrading to `v0.0.1-alpha.5`

### Hierarchical folder support

alpha.5 added support for nested `features/` and `requirements/` directories.
Flat single-file layouts (`features.yaml`, `requirements.yaml`) continue to
work; no migration is required.

### Runtime auto-detection

The `runtimes.python.command` and `runtimes.node.command` fields now accept
`auto` (default) in addition to explicit paths. If you previously hard-coded
the interpreter path, `auto` will use whatever is on `$PATH`, which may pick
up a different version in CI.

---

## Contributing migration notes

Every PR that introduces a **breaking change** — to `syu.yaml` fields, spec
YAML schema, CLI flag names, or default validation behaviour — **must** add an
entry to this file before merge. The entry should include:

1. The version being changed (use the next planned alpha tag)
2. A table showing old → new for any config/schema changes
3. The exact action required to upgrade an existing repository
4. Any new default-on validation rules

See the repository's
[`CONTRIBUTING.md`](https://github.com/ugoite/syu/blob/main/CONTRIBUTING.md)
for the full contribution workflow.

---

## Version compatibility summary

This guide only has release-by-release notes starting at `alpha.5`. The earlier
`alpha.1`-`alpha.4` builds shipped before the current migration notes and docs
layout stabilized, so this repository does **not** maintain step-by-step upgrade
instructions for those versions. If you are upgrading from one of those early
alphas, treat `alpha.5` as the first supported landing point: compare your
workspace against a freshly generated scaffold from the version you are
upgrading to, make the required `spec.root` and validation-config updates, then
run `syu validate .` until the workspace is green.

| syu version | `spec.root` default | `require_reciprocal_links` | `report.output` |
|---|---|---|---|
| alpha.1–alpha.4 | pre-`alpha.5`; migrate manually to the `alpha.5+` layout first | not yet documented | not yet documented |
| alpha.5 | `docs/spec` | not present | not present |
| alpha.6 | `docs/syu` | not present | not present |
| alpha.7 | `docs/syu` | `true` | stdout when unset; otherwise configured path |
