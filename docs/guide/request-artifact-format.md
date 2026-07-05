# Work request format

The active planning input artifact is `syu/work-request/v1`.

Required fields:

- `schema`
- `id`
- `summary`
- `operation`
- `seeds` or `requested_targets`

Add requests also require explicit per-target budgets:

- `constraints.max_added_bytes_per_target`
- `constraints.max_added_lines_per_target`

These budgets are enforced per target during planning and validation.

Typical flow:

```bash
cargo run --quiet -- work plan --workspace . --request request.yaml --out plan.yaml
cargo run --quiet -- validate . --plan plan.yaml
```

Use `syu/work-request/v1` as the current request wire format.
