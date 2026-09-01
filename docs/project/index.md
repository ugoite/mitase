---
title: "Project decisions"
description: "Read the durable architectural decisions that shape mitase before v1."
sidebar_position: 1
---

# Project decisions

This section records decisions that should stay visible to maintainers without
making architecture jargon the first thing a new user sees.

- [Mitase Re-Foundation freeze](./mitase-re-foundation-freeze.md) — the frozen
  product boundary, phased cutover, and pre-v1 acceptance gates.
- [The v1 canonical model](./v1-canonical-model.md) — the decision behind the
  original strict, single-version model; its execution-oriented boundary is
  superseded by ADR 0002.
- [ADR 0002: Remove Work execution](./adr-0002-remove-work-execution.md) — the
  decision to keep execution lifecycle concerns outside Mitase.
- [Release candidate identity](./release-candidate-contract.md) — the
  reproducible source and artifact identity contract for release promotion.
