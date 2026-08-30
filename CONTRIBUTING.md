# Contributing

Quality gates are defined once in the root `mise.toml` and placed into hosted
CI lanes by `.github/workflows/ci.yml`:

- local work uses `mise run fmt`, `mise run lint`, `mise run check`, and
  `mise run test`;
- hosted `ci-rust-check`, `ci-rust-test`, and `ci-repo` run in parallel;
- `ci-required` is the required aggregation status for pull requests and the
  merge queue.

Run the complete repository gate before opening a change:

```bash
mise run ci
```

Specification changes should preserve the forward Philosophy → Policy →
Requirement → Criterion → Feature → Binding graph and exact Artifact targets.
Do not add new planning, agent, delivery, retry, or workspace-mutation
responsibilities to Mitase. Those are external-tool concerns under the
[Re-Foundation freeze](docs/project/mitase-re-foundation-freeze.md).

The execution-oriented removal phases have landed in the current checkout.
Historical documentation may explain that transition, but new product
documentation and new APIs must follow the frozen boundary.

Start each delivery unit from the repository default branch in an independent
worktree, discover or create its Issue first, then implement, validate, open a
PR, obtain an independent review, converge any merge blockers, wait for
required CI, and merge. Public Issue/PR content must not contain private
conversation, internal prompts, local paths, credentials, usage data, or
private implementation history.
