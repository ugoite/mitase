# Mitase

Mitase v1 connects Philosophy principles, Policy rules, Requirement criteria, Feature bindings, exact artifact targets, validation, and executable work slices.

```bash
cargo run -- validate workspace .
cargo run -- work plan --request fixtures/v1/valid-web-app/work.yaml --out plan.yaml --workspace fixtures/v1/valid-web-app
cargo run -- validate plan fixtures/v1/valid-web-app --plan plan.yaml --plan-digest <digest> --slice-id <slice-id>
cargo run -- work export-context --plan plan.yaml --plan-digest <digest> --slice-id invalid-credentials-backend --workspace fixtures/v1/valid-web-app
```

Only `mitase/spec/v1`, `mitase/config/v1`, `mitase/work-request/v1`, and `mitase/work-plan/v1` are accepted. YAML parsing is strict and unknown fields are errors.

See [the v1 architecture](docs/understand/model/v1-architecture.md).
