---
title: "Plan and execute work"
description: "Turn an exact request into bounded slices, review it, and carry it through verification."
sidebar_position: 1
---

# Transitional Work runtime

> This page documents the pre-Re-Foundation execution runtime. It is not part
> of the frozen Mitase product boundary and must not be extended with new
> planning, agent, delivery, or mutation responsibilities.

External repository tooling owns the execution side of the former model. It
may derive small, reviewable slices from one exact Work origin, select a
canonical slice when the origin expands into independent candidates, export
context, and keep completion evidence tied to the exact plan-and-slice
identity. Mitase does not inspect execution artifacts while this runtime is
removed.

- [Workbench](./workbench.md) — the current product flow and projection.
- [Implementation planning](./implementation-planning.md) — the canonical
  plan-first loop.
- [Work request format](./request-artifact-format.md) — the input artifact.
- [Work plan format](./goal-plan-format.md) — the validated output artifact.
- [Command card](./command-card.md) — a compact list of the current commands.
