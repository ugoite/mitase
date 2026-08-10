# Team-scale example

This example is maintained against the active `mitase` v1 model.

It demonstrates one requirement and one feature connected through explicit bindings.

Current commands:

- `cargo run --quiet -- validate examples/team-scale`
- `cargo run --quiet -- workbench project --workspace examples/team-scale`

Primary implementation targets:
- `src/auth/login.rs` (rust)
- `src/auth/session.rs` (rust)
- `src/operations/audit.rs` (rust)
- `src/operations/reporting.rs` (rust)

Primary verification targets:
- `tests/auth_login.rs` (rust)
- `tests/audit_visibility.rs` (rust)
- `tests/report_exports.rs` (rust)

The example is written for the active v1 CLI and spec model.
