# Work plan format

The active planner output is `syu/work-plan/v1`.

A work plan contains:

- the typed Work origin and its linked origin closure
- plan basis and workspace fingerprint
- execution mode (`isolated-slices` in v1)
- ready, blocked, or split-recovery candidate status
- execution slices
- slice budgets
- completion checks

Validation entry points:

```bash
cargo run --quiet -- validate plan . --plan plan.yaml --plan-digest <digest> --slice-id <slice-id>
cargo run --quiet -- work export-context --workspace . --plan plan.yaml --plan-digest <digest> --slice-id <slice-id>
```

When more than one ready slice is produced, Workbench exposes the canonical
slice ids as accessible choices. Selecting one creates the only executable
plan boundary; approval and delivery always use the exact `{ plan_digest,
slice_id }` pair. Use `syu/work-plan/v1` as the current planner output format.
