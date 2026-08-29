# Implementation planning

Implementation planning is external repository tooling. The Mitase CLI does
not create or execute Work plans.

The transitional validation entrypoint can inspect a plan artifact produced by
that tooling:

```bash
cargo run --quiet -- validate plan . --plan plan.yaml --plan-digest <digest> --slice-id <slice-id>
```

Guidelines:

- Keep the typed `origin` and `requested_targets` exact. A Requirement
  criterion is the semantic “behavior” identity; Feature implementation
  origins are server-resolved exact target sets.
- Treat a ready work plan as external tooling state, not as a Mitase product
  responsibility. If transitional validation is needed, use the exact digest
  and slice.
- `mitase/work-plan/v1` uses `execution: isolated-slices`.
- Execute each slice from the plan basis revision in its own worktree or branch.
- Validate post-state with the exact `--plan-digest <digest> --slice-id <slice-id>` pair against only that isolated slice workspace.
- Sequential same-branch execution for multiple slices from one plan is not supported in v1.
- Repository tooling owns context export and execution using the exact
  `{ plan_digest, slice_id }` execution identity.
- Use small slices that fit the configured byte and target budgets.

The former `work` and `task` command groups are no longer part of the root
CLI. Work request and plan artifacts remain documented here only as
transitional inputs until the Work runtime removal phase.
