---
title: "Mitase 0.2.3 release notes"
description: "Authoring Authority / Semantic IR Re-Foundation on the stable v2-only release line."
sidebar_position: 8
---

# Mitase 0.2.3 release notes

Mitase 0.2.3 is the Authoring Authority / Semantic IR Re-Foundation.
`mitase/authoring/v2` is now the sole repository authoring authority. The
former `mitase/spec/v1` canonical serialization has been removed from the
runtime model.

## Main change

Authoring is authoritative. Semantic IR is derived.

- `mitase/authoring/v2` is the only normal repository authoring source.
- Mitase compiles it into a schema-less semantic representation used for
  indexing, validation, diagnostics, queries, reports, and editor
  integration.
- `mitase/spec/v1` remains accepted only as a legacy migration input through
  the explicit read-only `mitase migrate <source> --stdout` command, which
  emits deterministic v0.2 source without writing to the workspace.

## Compatibility

Existing `mitase/authoring/v2` repositories require no source migration.

Breaking machine-contract change: `mitase normalize` no longer returns
`document.schema = mitase/spec/v1`. Consumers parsing
`normalize --format json|yaml` output must read the versioned contract:

- `contract_version = mitase/normalization-result/v1`
- `semantic`, the schema-less semantic payload
- `provenance.source_schema = mitase/authoring/v2`

See [Upgrading to `v0.2.3`](../workflows/repository/migration.md#upgrading-to-v023).

## Boundary

No planner, agent, test runner, implementation, or delivery responsibility is
added. Mitase reports what must be true; it does not make it true. Release
tooling, planning, execution, test running, patch application, and delivery
state remain outside the product boundary.
