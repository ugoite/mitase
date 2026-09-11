# Getting started

Install a release build or run the binary from source, then create the smallest
workspace with the current v2 authoring source. The default human renderer is
the Balanced Hybrid layout: a concise outcome first, followed by the evidence
needed to understand an unresolved result.

## Quick start commands

```bash
RELEASE="$(gh release view --json tagName -q .tagName --repo ugoite/mitase)"
curl -fsSL "https://github.com/ugoite/mitase/releases/download/${RELEASE}/install-mitase.sh" | env MITASE_VERSION=alpha bash
mitase check .
```

The normal v1 CLI surface is specification-only:

- `validate`
- `check`
- `readiness report`
- `config effective`
- `normalize`
- `query`
- `show`
- `list`

Execution and delivery tooling is external to the Mitase CLI. See the
[Re-Foundation freeze](../../project/mitase-re-foundation-freeze.md) before
building new integrations.

The current authoring source is `schema: mitase/authoring/v2`; the workspace
frontend normalizes it into Mitase's canonical graph before validation. During
the 0.1.x dogfood period, normal loading also accepts existing canonical v1
sources. The explicit `migrate <source> --stdout` helper is the read-only path
for those existing sources, and the 0.2.x release line closes the v1 entrance;
see the [authoring contract](../../workflows/repository/authoring-v2.md) and
[migration guide](../../workflows/repository/migration.md).

Use the read-only discovery commands to inspect the canonical graph:

```bash
mitase list .
mitase list . --kind requirement --status implemented --format json
mitase list . --namespace capabilities --category "Mitase functional units" --format json
mitase list . --unverified-criteria --format json
mitase show REQ-CAPABILITY-001 . --format json
mitase query REQ-CAPABILITY-001 . --relation implementation-targets --format json
```

`list` returns stable ID, kind, namespace, category, title, status, and source
fields. Use exact `--namespace` and `--category` filters to narrow that view;
`--unverified-criteria` adds criteria without current exact verification
coverage. `show` includes criterion statements, exact implementation and
verification targets, authored relations, derived reverse relations, and
execution-free verification assessments. It does not run tests or other
configured runners. `query` inspects explicit relations for a specification ID,
local anchor, or exact bound target reference; it does not infer dependencies.

## Choosing an output format

Human-readable `text` is the default and is intended for a person reading a
terminal or a review log. Use `compact` when a CI log or annotation needs one
record per line, or `json` when an integration needs stable fields:

```bash
mitase check . --format text
mitase check . --format compact
mitase check . --format json
```

Compact validation starts with a one-line outcome and counts, then prints each
diagnostic as `path:line:column: severity[rule]: message`. Relation and
read-only next-read hints follow on the same line when available. Compact
output never contains ANSI control sequences, including in CI or redirected
output. JSON responses carry the additive top-level
`schema_version: "mitase/cli/v1"`; the command-specific fields remain at the
same level. See the [complete CLI contract](../../workflows/repository/cli-machine-contract.md)
for all supported commands.

All normal commands use these exit-code meanings:

| Code | Meaning |
| --- | --- |
| `0` | The command completed with a valid or successfully inspected result. |
| `1` | The workspace loaded, but validation/readiness failed, or a structured frontend diagnostic was reported. |
| `2` | A top-level command or workspace error occurred. |

Use the exit code for automation; do not infer failure from the severity of an
individual diagnostic.

A first-run workspace needs only:

- `mitase.yaml`
- one `docs/mitase` v2 requirement document
- real implementation and verification files referenced by that document

The minimal config is one line:

```yaml
schema: mitase/config/v1
```

The requirement short form connects one criterion to one implementation target
and one verification target. Copy the checked-in
[`fixtures/first-run-short`](../../../fixtures/first-run-short) tree to try the
same path locally, or continue to the [tutorial](./tutorial.md) for the full
four-layer story.

Use `examples/` as the reference bootstrap path for concrete layouts.

## Is mitase right for this repository?

`mitase` fits a repository when the team wants implementation work to stay
explainable from durable intent through exact code and verification evidence.
It is especially useful when requirements, tests, and ownership boundaries need
to remain visible during change. If the repository already has working code,
follow the [adoption path](../adopt/index.md) rather than forcing a greenfield
layout.

## Understanding validation output

Validation checks the configured specification graph, artifact ownership, and
the readiness rules in `mitase.yaml`. A successful result means the current
workspace satisfies those configured rules; it does not mean every future
capability is already specified. When validation is blocked, use the
[troubleshooting workflow](../../workflows/repository/troubleshooting.md) to
repair one connected issue at a time.
