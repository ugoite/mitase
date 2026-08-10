# Contributing

Quality gates are split by lifecycle:

- pre-commit runs file hygiene, changed-shell shellcheck, Rust formatting, and staged mitase specification validation.
- pre-push runs formatting and clippy only when Rust files are in the push, plus change validation for `mitase.yaml` or `docs/mitase/**` changes.
- CI runs the complete release gate.

Run the full release gate before opening a change:

```bash
scripts/ci/quality-gates.sh full
```

Executable changes should be driven by `mitase work plan` from exact specification anchors. Keep edits inside the selected slice and use `mitase validate --plan` to verify scope.
