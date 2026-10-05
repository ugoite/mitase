---
title: "ADR 0003: Bounded bootstrap initialization"
description: "Permit Mitase to create its own missing bootstrap metadata without reopening workspace mutation."
sidebar_position: 4
---

# ADR 0003: Bounded bootstrap initialization

**Status:** accepted
**Date:** 2026-10-05
**Amends:** [ADR 0002](./adr-0002-remove-work-execution.md) (narrowly; does not
remove or reverse it)

## Context

[ADR 0002](./adr-0002-remove-work-execution.md) and the
[Re-Foundation freeze](./mitase-re-foundation-freeze.md) forbid workspace
mutation by Mitase:

> Mitase tells you what must be true. It does not make it true.

Patch application and workspace mutation belong to external implementation
tools. That ban is load-bearing: it keeps the specification core free of
planner, agent-runtime, and delivery-state responsibilities.

At the same time, starting with Mitase is heavier than it should be. A new
repository must hand-write `mitase.yaml`, create the spec root directory, and
understand configuration conventions before the first `mitase check .` can
run. That mechanical bootstrap work currently leaks into documentation and
makes adoption look harder than the product is.

A `mitase init` command would fix the onboarding cost, but it writes to the
workspace, so it needs an explicit product boundary before any code exists.

## Decision

Permit one narrow exception to the workspace-mutation ban:

> Mitase may create missing Mitase-owned bootstrap metadata. It does not
> create or modify implementation artifacts, verification artifacts, or
> normative specification meaning.

Concretely, initialization may create only:

- `mitase.yaml` with the minimal canonical schema declaration;
- the configured spec-root directory marker (for example
  `docs/mitase/.gitkeep`) so the spec root exists after a fresh clone.

Initialization must not:

- author Philosophy, Policy, Requirement, Criterion, or Feature meaning;
- infer Criterion statements, responsibilities, claims, or coverage;
- create or modify source files, tests, or any verification evidence;
- overwrite an existing valid configuration, repair a broken one, or act as
  a configuration editor;
- run tests, initialize version control, or commit.

The general rule from ADR 0002 stands: implementation and work mutation stay
outside Mitase. This ADR adds a bounded bootstrap exception for
Mitase-owned metadata only.

## Consequences

Positive consequences:

- First-run setup becomes `mitase init .` followed by `mitase check .`,
  instead of hand-written configuration before any value is visible.
- The exception is enumerable: two metadata paths, fixed minimal content,
  deterministic output. Reviewers can verify the boundary by inspection.
- Normative meaning stays repository-owned. `mitase-authoring` keeps its
  rule that normative text and explicit relations are never inferred.

Trade-offs:

- The CLI is no longer purely read-only. Documentation, help contracts, and
  the architecture guide must describe the bootstrap exception precisely
  instead of claiming a blanket read-only posture.
- `mitase init` must carry its own safety contract: conflict checks before
  writes, idempotent re-runs, refusal to leave the target workspace, and a
  read-only dry run. Those guarantees belong to the implementing change, not
  to this ADR.

## Implementation sequence

1. Record the boundary in this ADR, the Re-Foundation freeze, the
   architecture guide, and the repository README.
2. Constrain the self-hosted specification first: a bootstrap rule on
   `POL-AUTHORITY-001`, a `workspace-bootstrap` criterion on
   `REQ-CAPABILITY-001`, and a planned `FEAT-INIT-001` with no implementation
   binding.
3. Implement `mitase init` as a separate focused change that satisfies that
   criterion and closes the planned feature with exact bindings and a
   verification claim.
4. Reorganize onboarding documentation around `mitase init` only after the
   command exists.
