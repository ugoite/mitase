# Getting started

Install a release build or run the binary from source, then create a v1 workspace.

## Quick start commands

```bash
# x-release-please-start-version
RELEASE="$(gh release view --json tagName -q .tagName --repo ugoite/mitase)"
# x-release-please-end
curl -fsSL "https://github.com/ugoite/mitase/releases/download/${RELEASE}/install-mitase.sh" | env MITASE_VERSION=alpha bash
cargo run --quiet -- validate .
```

The target v1 CLI surface is specification-only:

- `validate`
- `check`
- `query`
- `show`
- `list`

The current alpha checkout also exposes transitional `work`, `task`, and
`workbench` commands. They are retained only until the Re-Foundation removal
phases land; they are not part of the frozen product boundary. See the
[Re-Foundation freeze](../../project/mitase-re-foundation-freeze.md) before
building new integrations.

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
