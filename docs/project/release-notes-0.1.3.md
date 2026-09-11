---
title: "Mitase 0.1.3 release notes"
description: "The 0.1.3 patch-line improvements to CLI diagnostics and traceability."
sidebar_position: 6
---

# Mitase 0.1.3 release notes

Mitase 0.1.3 makes the existing specification compiler easier to use in a
terminal, CI log, and read-only integration. The semantic graph, artifact
identities, validation meaning, and exit-code behavior remain the authority.

## What changed

- The default human output uses the Balanced Hybrid layout: an outcome and
  counts first, followed by structured diagnostics with exact locations,
  relation context, evidence traceability, and a next read-only action when
  available.
- `--format compact` provides a stable one-line-per-record stream for
  validation and graph inspection. It does not emit ANSI control sequences.
- Normal JSON responses carry the additive top-level
  `schema_version: "mitase/cli/v1"`. Existing command-specific payload fields
  stay at the same level, so integrations can select the contract without
  changing their field paths.
- `show`, `list`, `query`, readiness reporting, and change validation expose
  the same read-only specification evidence through focused human and machine
  surfaces.

## Boundary

Mitase still reports what must be true; it does not make it true. These
surfaces do not execute tests or runners, edit source files, apply fixes, or
own planning and delivery state. Use the documented exit code rather than
diagnostic wording to drive external repository tooling.

The release-line acceptance gate exercises the self-hosted Mitase corpus, the
versioned Ugoite acceptance corpus, canonical graph preservation, exact
artifact resolution, and the frozen architecture boundary. Release promotion
continues to use an immutable candidate source revision and exact artifact
bytes.
