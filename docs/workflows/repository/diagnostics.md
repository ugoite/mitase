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

Authoring documents also have configurable per-file safety ceilings:

- `MITASE-AUTHORING-005` reports a document whose nonblank source line count
  exceeds `validation.authoring.limits.max_nonblank_lines` (default `1000`).
  Blank lines are ignored; comments count.
- `MITASE-AUTHORING-006` reports a document whose normalized top-level
  Philosophy, Policy, Requirement, or Feature count exceeds
  `validation.authoring.limits.max_top_level_items` (default `12`).

Both are fixed errors in Standard and Strict presets, run in the Graph phase,
and include `actual` and `configured-limit` evidence. Split the document or
raise the corresponding positive limit in `mitase.yaml`.

## Specification quality diagnostics

Quality diagnostics (`MITASE-QUALITY-*`) report structurally unnatural but
formally valid specifications: repeated normative statements across layers
and incoherent responsibility bundles. They run in the Graph phase in both
Standard and Strict presets, and they are Warning or Info findings only. A
workspace that reports only quality findings still passes `mitase check`;
existing errors are unchanged and never weakened.

Filter the quality-only subset by code prefix in text, JSON, and LSP output:

```bash
mitase check . --format json | jq '[.diagnostics[] | select(.code | startswith("MITASE-QUALITY-"))]'
```

Each finding carries stable string key/value `evidence` (matched statements,
signs, reference counts, shared tokens or targets, component sizes), the
subject and counterpart anchors, and a `suggested_action` describing the
authoring choice. Mitase offers no automatic fix: keep the local condition,
generalize the reusable decision, split by responsibility, consolidate the
obligation, or record why the shape is intentional.

For the rule table, comparison rules, negative controls, and what is
deliberately not decided mechanically, see [Spec quality
diagnostics](../../understand/quality/spec-quality-diagnostics.md).

The text format is a compact human-readable view of the same diagnostic. Use
JSON when an integration needs stable fields or exact locations.

For the complete 0.1.x JSON result, stream, and exit-code contract, see the
[CLI machine contract](./cli-machine-contract.md).
