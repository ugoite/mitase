# Contributing

Quality gates are split by lifecycle:

- pre-commit runs file hygiene, changed-shell shellcheck, Rust formatting, and staged mitase specification validation.
- pre-push runs formatting and clippy only when Rust files are in the push, plus change validation for `mitase.yaml` or `docs/mitase/**` changes.
- CI runs the complete release gate.

Run the full release gate before opening a change:

```bash
scripts/ci/quality-gates.sh full
```

Specification changes should preserve the forward Philosophy → Policy →
Requirement → Criterion → Feature → Binding graph and exact Artifact targets.
Do not add new planning, agent, delivery, retry, or workspace-mutation
responsibilities to Mitase. Those are external-tool concerns under the
[Re-Foundation freeze](docs/project/mitase-re-foundation-freeze.md).

The current checkout still contains transitional execution-oriented code while
the follow-up removal phases land. Tests for that code may remain until their
removal PR, but new product documentation and new APIs must follow the frozen
boundary.
