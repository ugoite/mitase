---
title: "Plan and execute work"
description: "Turn an exact request into bounded slices, review it, and carry it through verification."
sidebar_position: 1
---

# Plan and execute work

`mitase work` is the execution side of the model. It derives small, reviewable
slices from one exact Work origin, lets Workbench select a canonical slice
when the origin expands into independent candidates, exports the context
needed for one slice, and keeps completion evidence tied to the exact
plan-and-slice identity.

- [Workbench](./workbench.md) — the current product flow and projection.
- [Implementation planning](./implementation-planning.md) — the canonical
  plan-first loop.
- [Work request format](./request-artifact-format.md) — the input artifact.
- [Work plan format](./goal-plan-format.md) — the validated output artifact.
- [Command card](./command-card.md) — a compact list of the current commands.
