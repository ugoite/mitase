# mitase VS Code extension

<!-- FEAT-VSCODE-001 -->

This extension keeps `mitase` close to the editor instead of forcing every lookup
through a terminal:

- refresh `mitase validate --format json` diagnostics into the Problems panel
- show the current file's linked requirements, features, policies, and
  philosophies in the **mitase Context** explorer view
- jump from a spec ID to its YAML document
- open the traced files that belong to a requirement or feature
- use inline CodeLens actions on YAML spec IDs, traced files, and traced symbols
  without opening the command palette first

## Current protocol

The first cut keeps the integration intentionally small:

- diagnostics come from the checked-in `mitase` CLI via `mitase validate workspace . --format json`
- navigation reads the same `docs/mitase` workspace files directly so the extension
  can link source files back to requirements and features without requiring a
  second server process

That keeps the extension usable today while leaving room for a shared LSP server
later.

## Running from source

Switch your shell to the checked-in Node 20 version from
`editors/vscode/.nvmrc`, then use the pinned npm release from
`editors/vscode/package.json` to install dependencies from the repository root.
If you are hopping between the extension and the docs site, use the repository
Node workflow guide at `docs/contribute/maintaining/node-workflow.md` as the one-place runtime
map first:

```bash
nvm use "$(cat editors/vscode/.nvmrc)"
scripts/ci/pinned-npm.sh install editors/vscode
npm --prefix editors/vscode ci
```

1. Open `editors/vscode/` in VS Code.
2. Press `F5` to start an Extension Development Host.
3. Open a repository that contains `mitase.yaml` or `docs/mitase/features/features.yaml`.
4. If the `mitase` binary is not on your `PATH`, set **mitase › Binary Path**.

## Commands

- `mitase: Refresh diagnostics`
- `mitase: Trace active file`
- `mitase: Open spec item by ID`
- `mitase: Show related files for spec ID`

## Settings

- `mitase.binaryPath`: path to the `mitase` CLI binary
- `mitase.autoRefreshDiagnostics`: rerun diagnostics after saves
