# Python example

This example is maintained against the active `mitase` v1 model.

It demonstrates one requirement and one feature connected through explicit bindings.

Current commands:

- `cargo run --quiet -- validate workspace examples/python-only`
- `cargo run --quiet -- workbench project --workspace examples/python-only`
- `(cd examples/python-only && pytest python/test_traceability.py::test_python_requirement -v)`

Primary implementation targets:
- `python/app.py` (python)

Primary verification targets:
- `python/test_traceability.py` (python; exact claim `test_python_requirement`)

The example is written for the active v1 CLI and spec model.
