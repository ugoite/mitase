# Polyglot example

This example is maintained against the active `mitase` v1 model.

It demonstrates one requirement and one feature connected through explicit bindings.

Current commands:

- `cargo run --quiet -- validate workspace examples/polyglot`
- `cargo run --quiet -- workbench project --workspace examples/polyglot`

Primary implementation targets:
- `frontend/feature.ts` (typescript)
- `python/app.py` (python)

Primary verification targets:
- `frontend/traceability.test.ts` (Node test `typescriptRequirementTest`)
- `python/test_traceability.py` (pytest `test_python_requirement`)

Exact verification commands:
- `(cd examples/polyglot && npm install && node --import tsx --test --test-reporter=tap --test-name-pattern=^typescriptRequirementTest$ frontend/traceability.test.ts)`
- `(cd examples/polyglot && pytest python/test_traceability.py::test_python_requirement -v)`

The example is written for the active v1 CLI and spec model.
