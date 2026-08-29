---
title: "Mitase Re-Foundation freeze"
description: "The pre-v1 product boundary, acceptance gates, and phased cutover for Mitase."
sidebar_position: 2
---

# Mitase Re-Foundation freeze

**Status:** frozen
**Date:** 2026-08-29
**Scope:** pre-v1 product boundary and implementation direction

This is a foundation freeze, not a claim that every re-foundation phase is
complete. It fixes the product that later code and documentation must preserve.

## One-sentence definition

Mitase is a compiler for executable software specifications. It connects intent
to implementation and proves that a repository still satisfies its own
specification.

The operational boundary is:

> Mitase tells you what must be true. It does not make it true.

Mitase describes, connects, inspects, and verifies a repository specification.
It does not modify, execute, schedule, retry, or deliver repository work.

## Foundation boundary

### Mitase owns

- Philosophy, Policy, Requirement, Criterion, and Feature definitions;
- exact Binding and Artifact target references;
- the forward specification graph and reverse relations derived by `SpecIndex`;
- schema, relation, target, and verification validation;
- repository inventory, code intelligence, artifact resolution, and drift
  detection;
- Verification Claims, coverage, queries, diagnostics, and reporting.

### Mitase does not own

| Concern | Destination |
| --- | --- |
| WorkRequest, WorkPlan, ExecutionSlice | External planning or delivery tool |
| Shell command or test execution | External runner |
| Patch application or workspace mutation | External implementation tool |
| Agent lifecycle, Codex/Claude session, retries | External agent runtime |
| Delivery state, execution persistence, task queue | External service |

The current checkout includes transitional implementation surfaces from the
earlier direction. Their existence is tracked so they can be removed safely;
it is not permission to extend the frozen boundary.

## Canonical domain model

```text
Philosophy
    ↓
Policy
    ↓
Requirement
    ↓
Criterion
    ↓
Feature
    ↓
Binding
    ↓
Artifact

Criterion
    ↓
Verification Claim
    ↓
Verifier / Test / Artifact
```

The saved direction is upstream to downstream. Reverse relations are derived,
not separately authored. A Binding owns the exact Artifact target. Artifact
types are extended through adapters and resolvers rather than by turning every
repository technology into a new top-level specification kind.

The minimum domain rules are:

- every Requirement has at least one Criterion;
- every implemented Feature has a valid Binding;
- every Verification Claim resolves to an existing Criterion and target;
- an implemented or verified status is derived from resolvable repository
  evidence;
- a mismatch between a human declaration and derived evidence is a diagnostic.

## Validation contract

Validation is three layered:

1. **Structural:** YAML/schema shape, ID uniqueness, and references.
2. **Semantic:** domain rules, ownership, required Criteria, and claim
   consistency.
3. **Repository:** files, symbols, tests, structured selectors, and other
   Artifact targets resolve in the current checkout.

The target v1 CLI is correspondingly specification-only:

```text
mitase check
mitase validate
mitase query
mitase show
mitase list
```

`mitase check` is the CI-facing whole-repository gate. `mitase show` exposes a
specification's forward relations and derived reverse relations. The CLI does
not run tests or apply changes.

## Dogfood acceptance

Ugoite is the first production consumer and acceptance suite. Its existing
machine-readable registry is normalized into:

```text
philosophy → Philosophy
policies   → Policy
requirements → Requirement + Criterion
features   → Feature + Binding
```

Source code, tests, OpenAPI, architecture documents, UI documents, security
documents, and CI configuration remain Artifacts. They are not added as new
top-level specification kinds.

The migration is accepted only when Ugoite's specification is parseable by
Mitase, all implemented bindings and required source/test references resolve,
existing semantics are preserved, and duplicated Ugoite-specific validation can
be removed. Mitase must validate both itself and Ugoite before the re-founded
release is considered complete.

## Phased cutover

| Phase | Outcome | Acceptance focus |
| --- | --- | --- |
| 0. Product boundary freeze | This document and ADR establish the product | README, architecture, glossary, and boundary agree |
| 1. Complete Mitase rename | One repository-controlled identity | no old product identity in active surfaces |
| 2. Remove Work runtime | Execution lifecycle leaves Mitase | no WorkRequest/WorkPlan/Agent runtime dependency |
| 3. Consolidate specification core | Thin specification libraries and CLI | index and validation work without execution crates |
| 4. Artifact resolution | Generic exact target resolution | file, directory, symbol, test, and structured selectors |
| 5. Verification model | Criteria are connected to proof claims | claims and targets resolve without running them |
| 6. Ugoite migration | First real consumer is dogfooded | no semantic loss and `mitase check` in Ugoite CI |
| 7. Coverage and query | Humans and agents can consume semantic context | reverse queries and unverified/broken reports |
| 8. Spec browser | Inspection-only visualization | UI consumes core model without mutation or execution |

The implementation should land as focused PRs. Rename precedes Work runtime
removal; Ugoite migration begins only after the specification core and
verification semantics are stable. Optional browser work follows the core, not
the other way around.

## Breaking-change policy

Mitase is pre-v1. Breaking changes are allowed and preferred when they remove a
wrong product boundary. The project will not add compatibility aliases,
deprecated internal formats, migration commands, or a compatibility parser for
the former Work schemas unless a future request explicitly requires one.

`mitase.yaml` and the `mitase/*` specification namespace are canonical. The
former Work schema identifiers are removed as part of the Work runtime cutover;
they are not a second source of truth.

## CI contract at re-foundation completion

Mitase's release gate will require:

```text
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
mitase check .
```

The same engine validates Mitase itself and, after migration, Ugoite. A green
result proves the declared specification agrees with the checked-out
repository; it does not grant Mitase authority to change that repository.

## Non-goals

The re-foundation does not add an AI implementation agent, task planner,
automatic source modification, patch generation, shell runner, test runner,
workflow orchestration, cloud service, collaboration server, Git hosting,
project management, or issue tracking to Mitase.

For the complete vocabulary and the architectural consequences, see the
[domain glossary](../understand/model/domain-glossary.md), [architecture
guide](../understand/model/v1-architecture.md), and [ADR
0002](./adr-0002-remove-work-execution.md).
