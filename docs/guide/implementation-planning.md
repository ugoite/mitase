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
- Export context from the current revision only.
- Use small slices that fit the configured byte and target budgets.

Only the active `work` and `validate` commands described above are part of the current root CLI.
