# Getting started

Install a release build or run the binary from source, then create a workspace
with the current v2 authoring source.

## Quick start commands

```bash
RELEASE="$(gh release view --json tagName -q .tagName --repo ugoite/mitase)"
curl -fsSL "https://github.com/ugoite/mitase/releases/download/${RELEASE}/install-mitase.sh" | env MITASE_VERSION=alpha bash
cargo run --quiet -- validate workspace .
```

The normal v1 CLI surface is specification-only:

- `validate`
- `check`
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

A new workspace needs:

- `mitase.yaml`
- a `docs/mitase` tree with philosophy, policy, requirement, and feature documents
- artifact roots that point at real implementation and verification files

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
