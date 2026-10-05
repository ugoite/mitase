---
title: "ADR 0004: Authoring authority and derived semantic IR"
description: "Fix mitase/authoring/v2 as the sole repository source authority and the internal semantic representation as derived."
sidebar_position: 5
---

# ADR 0004: Authoring authority and derived semantic IR

**Status:** accepted
**Date:** 2026-10-05
**Amends:** [ADR 0002](./adr-0002-remove-work-execution.md) (narrowly; clarifies the specification source boundary)

## Context

The v0.2.2 workspace accepts `mitase/authoring/v2` at its external boundary,
but every authoring document is still normalized into the internal
`mitase/spec/v1` canonical serialization before indexing, validation, queries,
reports, and editor integration.

That leaves two authorities in practice:

- the repository source the user wrote, and
- the internal serialization the implementation treats as canonical.

The v0.2.3 re-foundation removes the second authority.

## Decision

The authority boundary is:

> Authoring is authoritative. Semantic IR is derived.

- `mitase/authoring/v2` is the sole normal repository authoring source.
- The internal representation is a schema-less semantic representation derived
  by parsing and normalizing that source.
- It is an inspection and index input, not another source format. It has no
  schema field and cannot be deserialized as workspace input.
- `mitase/spec/v1` is not an internal canonical representation. It remains
  accepted only as a legacy migration input through the explicit read-only
  `mitase migrate <source> --stdout` transition.

The domain model itself does not change in this step. Philosophy, Policy,
Requirement, Criterion, Feature, ArtifactBinding, ArtifactTarget, TargetClaim,
Contract, SpecAnchor, and BoundTargetRef remain the semantic model.

## Consequences

- Existing `mitase/authoring/v2` repositories require no source migration.
- The v0.2.2 semantic graph is frozen first as a schema-less projection
  regression before the old canonical representation is removed.
- Later v0.2.3 changes isolate legacy v1 migration, replace the canonical
  document with the semantic document, delete the old representation, lock the
  public normalize contract, and harden release acceptance.
- `mitase normalize` remains read-only. Its future machine contract reports a
  semantic payload plus provenance and is not itself an authoring source.

## Breaking-change policy

This is a pre-v1 cleanup. No compatibility layer, deprecated internal format,
or migration path is added for the removed internal serialization unless a
future request explicitly requires one.
