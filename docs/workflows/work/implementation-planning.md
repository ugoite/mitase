# Implementation planning

Current implementation planning uses `syu work` artifacts.

Minimal flow:

```bash
cargo run --quiet -- work plan --workspace . --request request.yaml --out plan.yaml
cargo run --quiet -- validate . --plan plan.yaml
cargo run --quiet -- work export-context --workspace . --plan plan.yaml --slice <slice-id>
```

Guidelines:

- Keep `requested_targets` or `seeds` exact.
- Treat a ready work plan as executable only after `validate --plan` passes.
- `syu/work-plan/v1` uses `execution: isolated-slices`.
- Execute each slice from the plan basis revision in its own worktree or branch.
- Validate post-state with `--slice <slice-id>` against only that isolated slice workspace.
- Sequential same-branch execution for multiple slices from one plan is not supported in v1.
- Export context from the current revision only.
- Use small slices that fit the configured byte and target budgets.

Only the active `work` and `validate` commands described above are part of the current root CLI.
