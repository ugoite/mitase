---
title: "Mitase 0.2.2 release notes"
description: "Facet projection reporting on the stable v2-only release line."
sidebar_position: 7
---

# Mitase 0.2.2 release notes

Mitase 0.2.2 adds the read-only Facet Projection report on the stable v2-only
source line. The `mitase/authoring/v2` schema, the canonical specification
model, and the existing `show` / `query` / `list` machine contracts are
unchanged.

## New capability

- `mitase report facets <source> <workspace> --format json|markdown` projects
  current `role: implementation` exact targets with direct `satisfies` claims
  by their opaque binding facet, under the independent
  `mitase/facet-projection-report/v1` contract.
- Accepted sources are feature IDs (`FEAT-*`) and requirement criterion
  anchors (`REQ-*#criterion.*`); anything else is a top-level error.
- Facet names stay project-defined opaque strings. Missing facets are not
  validation failures, and `declared_verification` describes declaration
  structure only; Mitase never runs a verifier.
- The optional [facet-oriented authoring](../workflows/repository/facet-oriented-authoring.md)
  pattern documents how one capability feature can carry the same outcome
  across facets. Old-style specifications stay valid.

## Upgrade guidance

No specification migration is required. Existing v2 documents validate
unchanged, and the new report reads the existing graph without rewriting it.

## Boundary

Mitase reports what must be true; it does not make it true. Release tooling,
planning, execution, test running, patch application, and delivery state
remain outside the product boundary.
