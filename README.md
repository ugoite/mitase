# Mitase

Keep what a repository promises connected to the code that implements it and
the evidence designated to verify it.

Mitase is a repository-native compiler and validator for executable software
specifications. It turns declared intent, requirements, implementation
ownership, and verification claims into a semantic graph, resolves their exact
repository targets, and checks that those relationships still hold.

> Mitase tells you what must be true. It does not make it true.

Requirements, code, and tests drift when the relationships between them live
only in documents, conventions, or reviewer memory. Mitase makes those
relationships explicit and machine-checkable. Once declared, exact bindings
can be checked again as the repository changes instead of being rediscovered
by hand.

Mitase does not plan work, write code, run tests, review changes, retry agents,
or deliver software. External tools do those jobs. Mitase checks whether the
resulting repository still agrees with what it says must be true.

Mitase owns the specification graph, exact artifact bindings, repository
inventory, artifact resolution, validation, verification claims, coverage,
diagnostics, and queries that explain how those pieces connect. It is
repository-native: the specification and its references live with the code
they describe.

## Product boundary

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
reintroduced into the frozen product boundary. The narrow exception is bounded
bootstrap initialization of missing Mitase-owned metadata, which never authors
normative specification meaning or implementation evidence; see
[ADR 0003](docs/project/adr-0003-bounded-bootstrap-initialization.md).

Read the [Mitase Re-Foundation freeze](docs/project/mitase-re-foundation-freeze.md)
for the decision, acceptance gates, and follow-up sequence.

```bash
mise run check:repo
```

The normal v1 CLI is intentionally limited to specification operations:
`mitase check`, `mitase validate`, `mitase readiness report`,
`mitase config effective`, `mitase normalize`, `mitase query`, `mitase show`,
`mitase list`, `mitase report pr`, and `mitase report facets`. The re-foundation removes execution
commands rather than replacing them with compatibility aliases. Bounded
bootstrap initialization of Mitase-owned metadata is the single
workspace-writing exception to this specification-only posture
([ADR 0003](docs/project/adr-0003-bounded-bootstrap-initialization.md)). Use
`--format text` for the default
Balanced Hybrid human view, `--format compact` for one-line CI records, and
`--format json` for the stable machine contract.

The explicit `mitase migrate <source> --stdout` helper is a read-only v0.1 to
v0.2 authoring transition. It is not part of normal `check` or `validate`
loading and never writes to a workspace; see the
[migration guide](docs/workflows/repository/migration.md).

See [the v1 architecture](docs/understand/model/v1-architecture.md).

## Repository development

The root [`mise.toml`](mise.toml) is the source of truth for the maintained
development surface: Rust, Node/npm, the Docusaurus docs site, and the VS Code
extension. From a fresh checkout:

```bash
mise install
mise run setup
```

Use the same root entrypoints locally and in CI:

```bash
mise run fmt
mise run lint
mise run check
mise run test
mise run build
```

Use `mise run build:website` for a docs-site build and
`mise run check:vscode` / `mise run test:vscode` for focused extension work.
