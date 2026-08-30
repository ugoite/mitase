# Repository instructions

## Product boundary

Mitase is a compiler for executable software specifications. It connects a
repository's intent to its implementation and verifies that the repository
still satisfies its own specification.

> Mitase tells you what must be true. It does not make it true.

The frozen product model is:

```text
Philosophy → Policy → Requirement → Criterion → Feature → Binding → Artifact
                                      │
                                      └→ Verification Claim → Verifier / Test / Artifact
```

Mitase owns the specification graph, exact artifact bindings, repository
inventory, artifact resolution, verification claims, coverage, diagnostics,
and queries. Work requests and plans, execution slices, shell or test
execution, patch application, agents, retries, delivery state, task queues,
and workspace mutation belong to external repository tooling.

The [Mitase Re-Foundation freeze](docs/project/mitase-re-foundation-freeze.md)
is the highest-priority design constraint. The current checkout still contains
transitional Work surfaces; their existence is not permission to add new
product responsibilities or compatibility layers.

## Architecture

- `mitase-spec-model`: canonical specification types and relations.
- `mitase-inventory`, `mitase-code-intel`, and `mitase-workspace`: repository
  inspection and exact artifact resolution.
- `mitase-validation` and `mitase-diagnostics`: structural, semantic, and
  repository validation.
- `mitase`: the CLI and specification-facing entrypoints.
- `mitase-work-model` and `mitase-planner`: transitional execution-oriented
  surfaces scheduled for removal or replacement by the re-foundation phases.

Do not route repository delivery behavior back into Mitase. Repository skills
may create worktrees, implement and review changes, run commands, coordinate
CI, and merge pull requests, but those are tooling behaviors.

## Commands

The root `mise.toml` is the canonical local and hosted command surface. Use
only its root tasks:

```bash
mise run setup
mise run fmt
mise run fmt:check
mise run lint
mise run check
mise run test
mise run ci
```

`mise run check:repo` currently wraps `cargo run --quiet -- validate workspace .`
as a transitional validator. When the re-founded `mitase check .` command is
available, replace that inner command and remove the transitional wording; do
not make the old Work model a new public contract.

`CARGO_TARGET_DIR=target/rust` is the portable repository default. Developers
who need intermediate build sharing across worktrees may set
`CARGO_BUILD_BUILD_DIR` from their own shell/profile without adding a
repository-specific absolute path.

## Validation and hosted CI

The semantic checks are owned by Mise. `.github/workflows/ci.yml` only places
the parallel `ci:lane:rust-check`, `ci:lane:rust-test`, and `ci:lane:repo`
tasks, then aggregates them in `ci-required`. The merge group validates its
synthetic merge commit. CodeQL and release remain separate concerns.

Rust dependency cache and compiler cache are separate. Shared dependency
cache writes are limited to the `main` push lane, and sccache is read-only for
pull requests and merge groups. Do not restore or publish a shared `target`
directory as a compiler cache.

## Delivery lifecycle

Every change is delivered as one focused unit:

```text
repository default branch
  → independent worktree
  → existing Issue discovery or required Issue creation
  → focused implementation
  → focused validation
  → pull request
  → independent reviewer
  → review convergence
  → required CI
  → merge
```

The primary implementer keeps a review ledger. A reviewer records the exact
commit reviewed and returns:

```text
REVIEW_BASE: <commit>
REVIEWED_FROM: <previous head, or none>
REVIEWED_HEAD: <commit>
REVIEW_SCOPE: <scope>
REVIEWED_INVARIANTS: <invariants checked>
REVIEWED_CHECKS: <checks and CI evidence>
VERDICT: APPROVE | CHANGE_REQUEST
CARRIED_BLOCKERS: <none or each prior blocker with RESOLVED | STILL_OPEN>
NEW_BLOCKERS: <none or complete list>
FOLLOW_UPS: <issue links or none>
EVIDENCE: <focused tests/checks>
LIMITATIONS: <environment limitations>
```

Reviewers must examine the delta from the previous reviewed head before
rechecking cumulative invariants. `CHANGE_REQUEST` is reserved for a problem
that makes merging unsafe or impossible. Improvements, debt, or findings in a
different scope become follow-up Issues and do not block this pull request.

## Public GitHub content

Issue and pull request bodies, comments, and review records are public. Include
only the externally safe problem statement, design decision, changed behavior,
related Issue, validation evidence, and limitations. Never include private
conversation, internal prompts, local filesystem paths, credentials, tokens,
usage data, or private implementation history.

## Release boundary

Release workflow and publishing semantics are a separate delivery unit. A
release resolves a selected tag/source revision once and all artifacts use
that immutable revision. Do not publish an incidental transitional `main`
state from this repository-tooling change.
