# Mitase v1 architecture

Mitase is a repository-native executable specification engine. It reads a
repository's declared intent and implementation references, builds a semantic
index, and reports whether the specification is structurally, semantically,
and repository-valid.

The product boundary is deliberately smaller than a development agent:

> Mitase tells you what must be true. It does not make it true.

## Boundary

| Mitase owns | External tools own |
| --- | --- |
| Philosophy, Policy, Requirement, Criterion, and Feature schema | Work requests and work plans |
| Binding and exact Artifact targets | Shell and test execution |
| Forward specification graph and derived reverse index | Patch application and workspace mutation |
| Inventory and artifact resolution | Agent lifecycle, retries, and task queues |
| Structural, semantic, and repository validation | Delivery state and execution persistence |
| Verification Claims, coverage, drift, diagnostics, and queries | Codex, Claude, or other implementation sessions |

The table is a product boundary, not a claim that every row is already
implemented in the current checkout. Transitional execution surfaces are
follow-up removal work, not a reason to expand the frozen core.

## Foundation hierarchy

The durable foundation is intentionally split by level. `PHIL-003 — Build
Rules, Not Disposable Artifacts` preserves the general ideal that product
behavior is encoded as reusable rules and continuously verifiable evidence.
`PHIL-005 — Authority Without Workflow Ownership` defines Mitase's specific
boundary, and `PHIL-006 — Evidence Before Authority` makes exact resolution a
condition for trusting a declared relationship.

The matching policies decompose that foundation into repository-owned rules:
authority, exact resolution, evidence-derived state, explicit implementation
and verification relations, and one canonical semantic graph. `POL-013 —
Spec-Driven Product Iteration` and `POL-009 — Quality Gates Before Integration`
retain their generic shared meanings. Their actual iteration, test, lint, and
integration gates remain the responsibility of repository tooling; Mitase
describes and validates the declared evidence but does not execute those gates.

This hierarchy deliberately does not adopt storage/provider portability,
serverless topology, or AI workflow philosophies from another product. A
shared identifier is used only when its meaning remains independent of that
product's architecture and operational assumptions.

## Authority boundary

The repository-owned specification is the authority for the repository's
declared meaning. Mitase is authoritative about interpreting that meaning,
indexing its exact relationships, resolving its Artifact targets, and
validating whether its declared evidence is structurally satisfied. That
interpretive authority does not extend to planning, implementing, executing,
testing, reviewing, retrying, or delivering repository work.

The distinction is intentional: an external tool may change the repository,
while Mitase reports whether the resulting repository still agrees with its
specification. Neither side silently becomes the other's source of truth.

## Canonical model

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

The persisted direction is from upstream intent toward implementation. The
index derives reverse relationships such as “what implements this
Requirement?” or “which specifications reference this file?”. This keeps one
source of authority while still making impact and coverage queries cheap.

### Domain rules

- A Requirement has one or more individually verifiable Criteria.
- A Feature is an implementation responsibility; it is not limited to a
  user-facing feature.
- A Binding connects that responsibility to a concrete Artifact target and
  owns the exact selector.
- An Artifact may be a source file, symbol, test, configuration, API operation,
  schema, workflow, directory, or other repository object resolved by an
  adapter.
- Artifact types are extended through adapters and resolvers rather than by
  adding a new top-level specification kind.
- Verification Claims identify what proves a Criterion and must resolve to an
  existing Criterion and target.

## Validation layers

Validation is separated so a readable YAML document is not mistaken for a
valid repository contract.

1. Structural validation checks schema shape, IDs, and references.
2. Semantic validation checks domain rules such as Criterion coverage,
   ownership, and Verification Claim resolution.
3. Repository validation resolves paths, symbols, tests, structured-document
   selectors, and other Artifact targets against the checkout.

## Derived status

Status is calculated from repository state whenever possible. A Feature with a
Binding, a resolvable Artifact, and valid Verification Claims for its required
Criteria can be reported as `verified`; missing or conflicting evidence is a
diagnostic rather than a status that a user can hide with a declaration.

## Evidence and derived state

Exact selectors and typed Verification Claims are declarations until their
referenced repository targets resolve. Once resolved, they provide evidence
for derived ownership, implementation, verification, and coverage state. An
unresolved or conflicting declaration remains visible as a diagnostic; it is
never promoted to authoritative evidence by a human status alone.

## Library direction

The intended dependency direction is:

```text
spec-model → project-model → workspace / inventory → validation → CLI
                                  ↘ code-intel
```

The CLI is a thin shell over these libraries. No core library may require a
planner, agent runtime, delivery store, or execution lifecycle.

See the [domain glossary](./domain-glossary.md), the [product-boundary
freeze](../../project/mitase-re-foundation-freeze.md), and [ADR
0002](../../project/adr-0002-remove-work-execution.md) for the terms and
decisions that govern the re-foundation.
