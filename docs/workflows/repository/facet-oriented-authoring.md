---
title: "Facet-oriented authoring"
description: "An optional pattern for projecting one capability across opaque Binding facets."
---

# Facet-oriented authoring

This is an **optional authoring pattern** over the unchanged
`mitase/authoring/v2` contract. It introduces no new schema, no new
specification kind, and no `authoring/v3`. The canonical model already carries
everything the pattern needs: `Binding.facet`, `role: implementation` exact
targets, direct `TargetClaim::Satisfies`, and exact `Verifies` claims with
`covers`.

## The pattern

Write one semantic capability `Feature` whose `role: implementation` bindings
each carry a project-defined opaque `facet` string and whose exact targets
hold direct `kind: satisfies` claims against the same surface-independent
`Criterion`:

```yaml
features:
  - id: FEAT-ENTRY-001
    status: implemented
    bindings:
      - id: core
        role: implementation
        facet: core
        responsibility: Own the canonical durable behavior.
        targets:
          - id: create-entry
            adapter: rust
            path: crates/example/src/entry.rs
            selector: { kind: symbol, name: create_entry }
            claims:
              - kind: satisfies
                criterion: REQ-ENTRY-001#criterion.creation
      - id: frontend
        role: implementation
        facet: frontend
        responsibility: Expose the same outcome through the browser client.
        targets:
          - id: entry-api
            adapter: typescript
            path: frontend/src/lib/entry-api.ts
            selector: { kind: symbol, name: entryApi }
            claims:
              - kind: satisfies
                criterion: REQ-ENTRY-001#criterion.creation
```

Mitase never interprets facet names. `core`, `frontend`, `mcp`,
`weird-project-specific-name` are all opaque strings that sort lexically in
reports. A project owns its facet vocabulary in its own docs; Mitase must not
ship a fixed vocabulary.

## Rules

1. `Criterion` statements stay surface-independent wherever possible. A
   surface-specific criterion is allowed only when the boundary itself carries
   meaning (for example, an HTTP status code).
2. Only `role: implementation` targets with **direct** `kind: satisfies`
   claims participate in the facet matrix. `role: operation` targets,
   `role: contract-source` targets, and implementation targets that only
   `expose` another target are catalogued separately and never counted as
   facet coverage.
3. Do not infer coverage through `Exposes`. An exposes-only target is listed
   with its relation, not projected as if it satisfied the criterion.
4. Every new implementation target ships its exact `Verifies` claim in the
   same change. A facet target covered only by another facet's test is not
   covered.
5. Never invent a target or a claim to fill a blank cell. A missing facet is
   a product decision, not a specification error, and the projection shows
   the blank honestly.
6. One exact artifact has one implementation owner. Claiming the same exact
   symbol from two `role: implementation` bindings (for example `cli-core`
   and `cli-remote`) is an ownership ambiguity. Extract transport-specific
   exact helper symbols first, or keep a shared transitional facet until the
   split is real.
7. Cross-facet implementations of one criterion stay connected by one
   `Contract` (`MITASE-CONTRACT-006`). When two or more `role: implementation`
   bindings with different facets satisfy the same criterion, the owning
   feature declares a contract whose participants cover each binding and whose
   guarantees name the shared criterion. The contract source lives in a
   `role: contract-source` binding (for example the `api` facet). Without
   that contract, current validation rejects the workspace.

## What Mitase does not do

- Missing facets are **not** validation failures. `mitase check` and
  `mitase validate` never require any facet to exist.
- `declared_verification` is **declared structural verification**: an exact
  `Verifies` claim covers the target and its runner metadata is structurally
  complete. Mitase never runs the runner, so the report never says a test
  passed.
- The projection is read-only. It cannot change the graph, add coverage, or
  decide which surfaces a product requires.

## Old style stays valid

A `Feature` that keeps surface targets at `role: operation`, or an
implementation target without a direct `satisfies` claim, is still a valid
specification. The facet report lists those targets under
`non_semantic_targets` so the old style and the facet-oriented style can
coexist in one workspace.
