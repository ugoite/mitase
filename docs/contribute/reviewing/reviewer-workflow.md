# Reviewer workflow

For the active v1 surface, review starts with validation and exact work scope.

Suggested sequence:

```bash
cargo run --quiet -- validate .
cargo run --quiet -- validate . --range origin/main...HEAD
cargo run --quiet -- validate plan . --plan plan.yaml --plan-digest <digest> --slice-id <slice-id>
cargo run --quiet -- workbench project --workspace . --format json
```

What to check:

- changed artifacts still have explicit owners
- changed requirements and features still validate cleanly
- any transitional plan artifact is externally produced and has the exact
  digest and slice under review
- execution context and delivery evidence are owned by repository tooling

Historical `trace`, `relate`, `log`, and `browse` commands are not part of the active root CLI.
