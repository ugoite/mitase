# C# fallback example

This example is maintained against the active `mitase` v1 model.

It demonstrates one requirement and one feature connected through explicit bindings.

Current commands:

- `cargo run --quiet -- validate examples/csharp-fallback`

Primary implementation targets:
- `src/OrderSummary.cs` (csharp)
- `scripts/check-workspace.sh` (shell)

Primary verification targets:
- `tests/OrderSummaryTests.cs` (csharp)

The example is written for the active v1 CLI and spec model.
