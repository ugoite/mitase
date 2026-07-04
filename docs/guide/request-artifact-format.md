# Work request format

The active planning input artifact is `syu/work-request/v1`.

Required fields:

- `schema`
- `id`
- `summary`
- `operation`
- `seeds` or `requested_targets`

Typical flow:

```bash
cargo run --quiet -- work plan --workspace . --request request.yaml --out plan.yaml
cargo run --quiet -- validate . --plan plan.yaml
```

Use `syu/work-request/v1` as the current request wire format.
