# Polyglot example

This example is maintained against the active `mitase` v1 model.

It demonstrates one requirement and one feature connected through explicit bindings.

Current commands:

- `cargo run --quiet -- validate examples/polyglot`

Primary implementation targets:
- `frontend/feature.ts` (typescript)
- `python/app.py` (python)
- `src/trace.rs` (rust)

Primary verification targets:
- `frontend/traceability.test.ts` (typescript)
- `python/test_traceability.py` (python)

The example is written for the active v1 CLI and spec model.
