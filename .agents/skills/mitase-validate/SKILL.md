---
name: mitase-validate
description: Use when selecting, running, or interpreting Mitase validation, tests, or CI checks.
---

# Mitase validation

Root `mise.toml` is the validation source of truth. GitHub Actions only places
the corresponding hosted lanes.

## Root commands

```bash
mise run setup
mise run fmt
mise run fmt:check
mise run lint
mise run check
mise run test
mise run ci
mise run ci:merge
```

`mise run check:repo` invokes the specification-only `mitase check .` command,
the canonical whole-repository validator. Keep `mitase validate workspace` as
the detailed public validation command; it is not a replacement for the root
Mise task contract.

## Validation order

1. Run the narrowest relevant task or test.
2. Run the touched-area format/lint/check task.
3. Expand once to `mise run ci` when the change crosses surfaces or is ready
   for the merge gate.
4. Treat the `ci:lane:*` tasks as hosted packing units; do not invent local
   semantics for them.

For blocking external operations, use a platform-appropriate bounded process
or harness. Do not assume an async timeout interrupts synchronous filesystem,
Docker, browser, or network work.

## Hosted CI

`ci-rust-check`, `ci-rust-test`, and `ci-repo` run in parallel and
`ci-required` aggregates them. The merge-group run validates the synthetic
merge commit. CodeQL and release workflows remain separate from the canonical
product task graph.
