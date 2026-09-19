---
title: "Mitase 0.2.0 release notes"
description: "The first stable v2-only release of Mitase."
sidebar_position: 6
---

# Mitase 0.2.0 release notes

Mitase 0.2.0 is the first stable release on the v2-only source line. The
specification graph, exact artifact bindings, repository inventory, and
read-only verification surfaces remain the product authority.

## Source policy

- Normal `check` and `validate` accept v2 authoring documents.
- The former v1 authoring entrance is rejected with `MITASE-SOURCE-001` and an
  explicit migration action.
- `mitase migrate <source> --stdout` is read-only and emits
  `schema: mitase/authoring/v2`; it never overwrites the source or mutates the
  workspace.
- Stable selectors use plain `v<major>.<minor>.<patch>` tags. Alpha, beta, and
  RC selectors are not supported release channels.

## Upgrade guidance

Review the [migration guide](../workflows/repository/migration.md) and convert
v1 documents explicitly before adopting the v0.2.0 source line. The migration
is a source-authoring conversion, not a compatibility alias or an automatic
workspace rewrite.

## Boundary

Mitase reports what must be true; it does not make it true. Release tooling,
planning, execution, test running, patch application, and delivery state
remain outside the product boundary.
