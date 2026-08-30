# Mitase

Mitase is a compiler for executable software specifications. It connects a
repository's intent to its implementation and verifies that the repository
still satisfies its own specification.

> Mitase tells you what must be true. It does not make it true.

Mitase owns the specification graph, exact artifact bindings, repository
inventory, artifact resolution, validation, verification claims, coverage,
diagnostics, and queries that explain how those pieces connect. It is
repository-native: the specification and its references live with the code
they describe.

The durable foundation separates repository-owned meaning from Mitase's role as
interpreter, index, resolver, and validator. Mitase can determine whether
declared relationships and evidence are structurally satisfied, but it does
not own planning, implementation, execution, testing, review, retries, or
delivery.

## Frozen product boundary

The canonical model is:

```text
Philosophy → Policy → Requirement → Criterion → Feature → Binding → Artifact
                                      │
                                      └→ Verification Claim → Verifier / Test / Artifact
```

Forward relations are persisted in the specification. Reverse relations are
derived by the index. A Binding owns the exact Artifact target that a Feature
is responsible for; an Artifact is an external repository object, not another
specification kind.

Work requests and plans, execution slices, shell or test execution, patch
application, agents, retries, delivery state, task queues, and workspace
mutation are outside Mitase. The former implementation surfaces for that
earlier direction have been removed from the current checkout and must not be
reintroduced into the frozen product boundary.

Read the [Mitase Re-Foundation freeze](docs/project/mitase-re-foundation-freeze.md)
for the decision, acceptance gates, and follow-up sequence.

```bash
mise run check:repo
```

The target v1 CLI is intentionally limited to specification operations:
`mitase check`, `mitase validate`, `mitase query`, `mitase show`, and
`mitase list`. The re-foundation removes execution commands rather than
replacing them with compatibility aliases.

See [the v1 architecture](docs/understand/model/v1-architecture.md).
