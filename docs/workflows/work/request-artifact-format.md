# Work request format

The active planning input artifact is `syu/work-request/v1`. It represents one
typed Work origin; it is not a generic goal, seed, file, or behavior picker.

Required fields:

- `schema`
- `id`
- `title`
- `operation`
- `origin`
- `constraints`
- `requested_targets`

Add requests also require explicit per-target budgets:

- `constraints.max_added_bytes_per_target`
- `constraints.max_added_lines_per_target`

These budgets are enforced per target during planning and validation.

Requested targets use explicit transition objects:

```yaml
origin:
  kind: requirement-criterion
  criterion: REQ-TEST-001#criterion.behavior
requested_targets:
  - reference: FEAT-TEST-001#binding.impl/target.handler-missing
    transition: add
```

Supported transitions are:

- `add`: the target must not exist yet and is planned as a new editable target.
- `modify`: the target must already exist and is planned as an editable target.
- `remove`: the target must already exist and is planned for removal.
- `run-only`: the target must already exist and is planned as run-only context.
- `readonly`: the target must already exist and is planned as readonly context.

Typical flow:

```bash
cargo run --quiet -- work plan --workspace . --request request.yaml --out plan.yaml
cargo run --quiet -- validate plan . --plan plan.yaml --plan-digest <digest> --slice-id <slice-id>
```

`Requirement criterion` is the human-facing semantic meaning of “behavior” in
this flow. Feature implementation bindings and exact implementation targets
are also valid Work origins when the server projects them as enabled origin
capabilities. The browser copies only the projected `origin` and never invents
targets, criteria, or contracts.

Use `syu/work-request/v1` as the current request wire format. Old `summary`,
`seeds`, and generic identity payloads are intentionally rejected before v1.

Workbench split recovery may add the internal `exact_scope` closure fields to
the same v1 request when a user selects one proposed slice. Those fields are
server-owned evidence of the selected Generated targets and contracts; they
are revalidated against the origin closure before planning and are not a
second user-editable scope mechanism.
