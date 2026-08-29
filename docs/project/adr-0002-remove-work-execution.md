---
title: "ADR 0002: Remove Work execution from Mitase"
description: "Freeze the Mitase boundary around repository specifications and verification."
sidebar_position: 3
---

# ADR 0002: Remove Work execution from Mitase

**Status:** accepted
**Date:** 2026-08-29
**Supersedes:** the execution-oriented boundary in ADR 0001

## Context

The repository contains two products under one name:

1. a specification system that models intent, requirements, exact artifact
   ownership, and verification; and
2. a Work system that plans slices, persists delivery state, coordinates agent
   runs, and applies repository changes.

The first product is the durable value and can serve humans, CI, IDEs, and
external agents. The second product changes and executes a repository. Keeping
both in Mitase makes the product boundary unstable and encourages the core to
grow back into a planner or agent runtime.

## Decision

Mitase is a repository-native executable specification engine:

> Mitase tells you what must be true. It does not make it true.

Mitase owns:

- Philosophy, Policy, Requirement, Criterion, and Feature schema;
- exact Binding and Artifact target references;
- the forward specification graph and derived reverse index;
- inventory, code intelligence, artifact resolution, and drift detection;
- structural, semantic, and repository validation;
- Verification Claims, coverage, diagnostics, reporting, and queries.

The following are outside Mitase and belong to external tools or products:

- WorkRequest, WorkPlan, and ExecutionSlice;
- shell command execution, patch application, and test execution;
- Agent lifecycle, Codex/Claude sessions, retries, and task queues;
- delivery state, execution persistence, and workspace mutation.

The canonical stored direction remains:

```text
Philosophy → Policy → Requirement → Criterion → Feature → Binding → Artifact
Criterion → Verification Claim → Verifier / Test / Artifact
```

Forward references are persisted. Reverse relationships are derived by
`SpecIndex`. The exact target and verification semantics produced by the Work
model remain valuable as specification concepts, but execution lifecycle
semantics do not.

## Consequences

Positive consequences:

- Mitase has one sentence that defines its responsibility.
- The core can be loaded and validated without an agent, planner, delivery
  store, or Workbench execution server.
- Status, coverage, and drift can be recomputed from the current checkout.
- External implementation tools can ask Mitase for semantic context without
  granting Mitase mutation authority.

Trade-offs:

- The current Work/Agent/Workbench runtime must be removed or moved out in
  follow-up changes.
- Some existing documentation and tests will be retired or rewritten as the
  execution surface disappears.
- Users who want planning or mutation must compose Mitase with another tool.

## Breaking-change policy

This is a pre-v1 re-foundation. No compatibility alias, legacy parser, field
alias, migration command, or execution-state migration is required or desired
unless a future change explicitly requests one. The old internal format is not
an input contract to preserve.

In particular, `mitase/work-request/v1` and `mitase/work-plan/v1` are not part
of the target product schema. Their removal is a breaking cutover, not a
deprecation period.

## Implementation sequence

1. Freeze the product boundary in README, architecture, glossary, and this
   ADR.
2. Complete the checked-in Mitase rename and remove remaining old identity
   surfaces.
3. Remove Work schemas, CLI commands, and Work/Planner/Agent/Delivery runtime
   crates; retain Criterion, exact Artifact targets, Binding, provenance, and
   Verification Claim concepts.
4. Consolidate the specification core and make validation independent of
   execution libraries.
5. Strengthen artifact resolution, verification claims, coverage, and query
   surfaces.
6. Migrate Ugoite's real executable specification registry and make the
   `mitase check` command its acceptance gate.

Optional visualization is a later inspection surface. It must consume the core
model and must not reintroduce mutation or execution ownership.
