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

Requested targets use explicit transition objects:

```yaml
requested_targets:
  - ref: FEAT-TEST-001#binding.impl/handler-missing
    transition: add
```

Supported transitions are:

- `add`: the target must not exist yet and is planned as a new editable target.
- `modify`: the target must already exist and is planned as an editable target.
- `remove`: the target must already exist and is planned for removal.
- `run-only`: the target must already exist and is planned as run-only context.
- `readonly`: the target must already exist and is planned as readonly context.

Legacy string targets still default their transition from `operation`, but new requests should use the object form so the transition is explicit.

Typical flow:

```bash
cargo run --quiet -- work plan --workspace . --request request.yaml --out plan.yaml
cargo run --quiet -- validate . --plan plan.yaml
```

Use `syu/work-request/v1` as the current request wire format.
