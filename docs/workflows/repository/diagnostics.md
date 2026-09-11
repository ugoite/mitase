---
title: "Structured diagnostics"
description: "The shared diagnostic contract used by validation and CLI consumers."
sidebar_position: 3
---

# Structured diagnostics

`mitase-diagnostics` is the shared diagnostic API for validation consumers.
The CLI's JSON output and the LSP `textDocument/publishDiagnostics` output
consume the same structured value; they do not reinterpret rule messages
independently.

Run a validation command with `--format json` to inspect the contract:

```bash
mitase validate workspace . --format json
```

Each item in `diagnostics` contains these fields:

- `code`: stable validation rule code.
- `phase`: validation phase (`config`, `graph`, `targets`, `scope`, or
  `readiness`).
- `severity`: `error`, `warning`, or `info`.
- `reason`: the human-readable explanation.
- `primary`: the primary repository span. `path`, `line`, `column`,
  `end_line`, and `end_column` make the location usable by editor clients;
  values that are not known are `null`.
- `related_spans`: additional locations and their explanations.
- `subject` and `reference`: structured specification or repository subjects
  when a diagnostic relates two exact entities.
- `candidates`: deterministic resolution candidates when a declaration is
  unresolved, ambiguous, or unsupported.
- `evidence`: structured validation evidence.
- `suggested_action`: an optional next action.
- `fix`: an optional safe-fix description. Mitase reports this information;
  it does not apply repository changes.

Candidates are evidence for explaining a resolution failure, not permission
to choose on the user's behalf. Ambiguous resolution remains a validation
error until the author makes the declaration exact.

The text format is a compact human-readable view of the same diagnostic. Use
JSON when an integration needs stable fields or exact locations.

For the complete 0.1.x JSON result, stream, and exit-code contract, see the
[CLI machine contract](./cli-machine-contract.md).
