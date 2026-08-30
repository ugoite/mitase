# Reviewer workflow

For the active v1 surface, review starts with validation and exact work scope.

Suggested sequence:

```bash
cargo run --quiet -- validate workspace .
cargo run --quiet -- validate change . --range origin/main...HEAD
```

What to check:

- changed artifacts still have explicit owners
- changed requirements and features still validate cleanly
- any transitional plan artifact is externally produced and its exact digest
  and slice are reviewed by repository tooling
- execution context and delivery evidence are owned by repository tooling

Historical `trace`, `relate`, `log`, and `browse` commands are not part of the active root CLI.
