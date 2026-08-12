# TypeScript example

This example is maintained against the active `mitase` v1 model.

It demonstrates one requirement and one feature connected through explicit bindings.

Current commands:

- `cargo run --quiet -- validate workspace examples/typescript-only`
- `cargo run --quiet -- workbench project --workspace examples/typescript-only`
- `(cd examples/typescript-only && npm install && node --import tsx --test --test-reporter=tap --test-name-pattern=^typescriptRequirementTest$ src/app.test.ts)`

Primary implementation targets:
- `src/app.ts` (typescript)

Primary verification targets:
- `src/app.test.ts` (typescript; exact claim `typescriptRequirementTest`)

The example is written for the active v1 CLI and spec model.
