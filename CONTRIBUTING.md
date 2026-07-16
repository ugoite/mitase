# Contributing

Quality gates are split by lifecycle:

- pre-commit runs file hygiene, changed-shell shellcheck, Rust formatting, and staged syu specification validation.
- pre-push runs formatting and clippy only when Rust files are in the push, plus change validation for `syu.yaml` or `docs/syu/**` changes.
- CI runs the complete release gate.

Run the full release gate before opening a change:

```bash
scripts/ci/quality-gates.sh full
```

Executable changes should be driven by `syu work plan` from exact specification anchors. Keep edits inside the selected slice and use `syu validate --plan` to verify scope.
