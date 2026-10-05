---
title: "Start here"
description: "Choose the shortest path from discovering mitase to a validated workspace."
sidebar_position: 1
---

# Start here

`mitase` is for repositories that want declared intent, exact implementation
targets, and verification evidence to remain connected in the repository
instead of relying on people to reconstruct those relationships during every
change.

Every path starts the same way: `mitase init .`, then `mitase check .`.
`init` creates the mechanical workspace files; what differs is what you do
next.

## Choose your next step

- [First run](./first-run/index.md). A new workspace: install, run `init`,
  confirm the first check, then write your first Requirement in the tutorial.
- [Adopt an existing repository](./adopt/index.md). A repository that already
  has code and tests: run `init`, inspect the discovered conventions with
  `mitase config effective .`, then connect one bounded capability.

A new workspace needs a small, connected specification; an existing repository
needs inventory and a bounded first capability before it claims ownership.
Neither path starts by hand-writing configuration files.
