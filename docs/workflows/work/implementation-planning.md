# Implementation planning

Current implementation planning uses `mitase work` artifacts.

Minimal flow:

```bash
cargo run --quiet -- work plan --workspace . --request request.yaml --out plan.yaml
cargo run --quiet -- validate plan . --plan plan.yaml --plan-digest <digest> --slice-id <slice-id>
cargo run --quiet -- work export-context --workspace . --plan plan.yaml --plan-digest <digest> --slice-id <slice-id>
```

Guidelines:

- Keep the typed `origin` and `requested_targets` exact. A Requirement
  criterion is the semantic “behavior” identity; Feature implementation
  origins are server-resolved exact target sets.
- Treat a ready work plan as executable only after `validate plan` passes for the exact digest and slice.
- `mitase/work-plan/v1` uses `execution: isolated-slices`.
- Execute each slice from the plan basis revision in its own worktree or branch.
- Validate post-state with the exact `--plan-digest <digest> --slice-id <slice-id>` pair against only that isolated slice workspace.
- Sequential same-branch execution for multiple slices from one plan is not supported in v1.
- Export context from the current revision only, using the exact
  `{ plan_digest, slice_id }` execution identity.
- Use small slices that fit the configured byte and target budgets.

Only the active `work` and `validate` commands described above are part of the current root CLI.
