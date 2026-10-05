# Getting started

Install a release build or run the binary from source, then start every
workspace the same way: `mitase init .`, followed by `mitase check .`. The
default human renderer is the Balanced Hybrid layout: a concise outcome first,
followed by the evidence needed to understand an unresolved result.

## Quick start commands

```bash
RELEASE="$(gh release view --json tagName -q .tagName --repo ugoite/mitase)"
curl -fsSL "https://github.com/ugoite/mitase/releases/download/${RELEASE}/install-mitase.sh" | bash
mitase init .
mitase check .
```

`mitase init` creates the mechanical workspace files so the first thing you
think about is your Requirement, not configuration:

- `mitase.yaml` with one line: `schema: mitase/config/v1`. Repository
  conventions resolve the standard spec root, excludes, inventory discovery,
  validation defaults, and runner presets; run
  `mitase config effective .` to see exactly what was resolved.
- `docs/mitase/.gitkeep`, so the default spec root exists even after a fresh
  clone.

`init` never authors specification meaning or implementation files. No
Requirement, Criterion, source file, or test is generated. The normal v1 CLI
surface stays specification-only apart from this bootstrap step:

- `init`
- `validate`
- `check`
- `readiness report`
- `config effective`
- `normalize`
- `query`
- `show`
- `list`
- `report pr`
- `report facets`

Execution and delivery tooling is external to the Mitase CLI. See the
[Re-Foundation freeze](../../project/mitase-re-foundation-freeze.md) before
building new integrations.

## What a green check means

A green `mitase check .` on a fresh workspace means the bootstrap is valid:
the configuration parses and the spec root loads. It does not mean any
capability is specified yet. Add your first Requirement under `docs/mitase/`
with the [tutorial](./tutorial.md), then run `mitase check .` again to see
validation report on real intent, ownership, and evidence.

For output formats, exit codes, and the stable machine contract, see the
[complete CLI contract](../../workflows/repository/cli-machine-contract.md):
`--format text` is the default human view, `--format compact` keeps one
record per line for CI, and `--format json` carries the top-level
`schema_version: "mitase/cli/v1"`.

## Is mitase right for this repository?

`mitase` fits a repository when the team wants implementation work to stay
explainable from durable intent through exact code and verification evidence.
It is especially useful when requirements, tests, and ownership boundaries need
to remain visible during change. If the repository already has working code,
follow the [adoption path](../adopt/existing-repository.md) rather than
forcing a greenfield layout.
