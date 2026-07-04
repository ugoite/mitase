# Work plan format

The active planner output is `syu/work-plan/v1`.

A work plan contains:

- plan basis and workspace fingerprint
- ready or blocked status
- execution slices
- slice budgets
- completion checks

Validation entry points:

```bash
cargo run --quiet -- validate . --plan plan.yaml
cargo run --quiet -- work export-context --workspace . --plan plan.yaml --slice <slice-id>
```

Use `syu/work-plan/v1` as the current planner output format.
