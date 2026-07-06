# Syu

Syu v1 connects Philosophy principles, Policy rules, Requirement criteria, Feature bindings, exact artifact targets, validation, and executable work slices.

```bash
cargo run -- validate .
cargo run -- work plan --request fixtures/v1/valid-web-app/work.yaml --out plan.yaml --workspace fixtures/v1/valid-web-app
cargo run -- validate fixtures/v1/valid-web-app --plan plan.yaml
cargo run -- work export-context --plan plan.yaml --slice invalid-credentials-backend --workspace fixtures/v1/valid-web-app
```

Only `syu/spec/v1`, `syu/config/v1`, `syu/work-request/v1`, and `syu/work-plan/v1` are accepted. YAML parsing is strict and unknown fields are errors.

See [the v1 architecture](docs/guide/v1-architecture.md).
