# Historical migration notes

> **Re-Foundation policy:** These notes describe earlier alpha cutovers only.
> They are not a compatibility contract for the Mitase Re-Foundation. New
> pre-v1 work must use the canonical current design; no legacy internal-format
> parser, alias, or migration layer is added unless explicitly requested.

Start here when you are upgrading an existing `mitase` workspace between alpha
releases. This page documents breaking changes and release-specific upgrade
steps. When `mitase validate workspace .` starts failing after an upgrade, check the section
for the version you just installed.

> **Note:** This page is historical guidance for pre-v1 alpha workspaces. The
> current repository surface is the v1 `schema: mitase/config/v1` model; older
> fields and commands mentioned below are kept only as migration notes.

---

## Current pre-v1 cutover

The current v1 surface intentionally breaks the previous Work/verification
wire and CLI shapes. There are no compatibility aliases:

| Previous surface | Current surface | Required action |
|---|---|---|
| Generic Work summaries/seeds and implicit behavior selection | `mitase/work-request/v1` with an exact `origin` and `requested_targets` | Rewrite request artifacts. “Behavior” means the exact Requirement criterion; Feature implementation bindings and exact implementation targets are separate origin choices. |
| `mitase work` / `mitase task` execution lifecycle | External repository tooling; the former Mitase command groups are removed | Keep planning, execution, retry, and finalization outside Mitase. Transitional plan/result inspection remains until the Work runtime removal phase. |
| `max_slices_per_seed` | `max_slices_per_origin` | Rename the config key. |
| Legacy `ReadinessLevel` public-entrypoint combinations | v1 public-entrypoint probes support only `off`, `seedable`, or `work-ready` | Change unsupported probe levels; they are rejected by configuration validation. |

Recreate stale plan, approval, receipt, and context artifacts after migration;
their canonical digests and execution identities are intentionally not
backward-compatible.

---

## Upgrading to `v0.0.1-alpha.7`

### New `mitase.yaml` fields

| Field | Default | Notes |
|---|---|---|
| `validate.require_reciprocal_links` | `true` | New. Adjacent-layer links must be reciprocal. See [Understanding validation output](../../start-here/first-run/getting-started.md#understanding-validation-output). |

### Action required

**`validate.require_reciprocal_links: true` is on by default.**

If your spec has one-directional links (e.g. a requirement lists a feature but
the feature does not list the requirement back), `mitase validate` will now fail
with `MITASE-graph-reciprocal-001`.

To fix: add the missing back-references. Or, if your project needs a phased
migration, temporarily opt out:

```yaml
# mitase.yaml
validate:
  require_reciprocal_links: false
```

### New validation rules enabled by default

| Code | Severity | Description |
|---|---|---|
| `MITASE-graph-reciprocal-001` | error | Adjacent-layer links must be reciprocal |

### Spec root default

The `spec.root` default remains `docs/mitase` (unchanged from alpha.6).

---

## Upgrading to `v0.0.1-alpha.6`

### `spec.root` default changed

| | alpha.5 | alpha.6 |
|---|---|---|
| Default `spec.root` | `docs/spec` | `docs/mitase` |

If your `mitase.yaml` relied on the implicit default (`docs/spec`) without
explicitly declaring `spec.root`, you must either:

1. Move your spec directory: `mv docs/spec docs/mitase`, or
2. Add the following to `mitase.yaml` to keep the old path:

   ```yaml
   spec:
     root: docs/spec
   ```

### New structured validation rule IDs

alpha.6 introduced the `MITASE-*` rule code taxonomy. Error output format changed:

| Before (alpha.5) | After (alpha.6) |
|---|---|
| `error: orphaned item REQ-001` | `error[MITASE-graph-orphaned-001] REQ-001` |

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

Every PR that introduces a **breaking change** — to `mitase.yaml` fields, spec
YAML schema, CLI flag names, or default validation behaviour — **must** add an
entry to this file before merge. The entry should include:

1. The version being changed (use the next planned alpha tag)
2. A table showing old → new for any config/schema changes
3. The exact action required to upgrade an existing repository
4. Any new default-on validation rules

See the repository's
[`CONTRIBUTING.md`](https://github.com/ugoite/mitase/blob/main/CONTRIBUTING.md)
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
run `mitase validate workspace .` until the workspace is green.

| mitase version | `spec.root` default | `require_reciprocal_links` |
|---|---|---|
| alpha.1–alpha.4 | pre-`alpha.5`; migrate manually to the `alpha.5+` layout first | not yet documented |
| alpha.5 | `docs/spec` | not present |
| alpha.6 | `docs/mitase` | not present |
| alpha.7 | `docs/mitase` | `true` |
