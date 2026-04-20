# syu VS Code extension

<!-- FEAT-VSCODE-001 -->

This extension keeps `syu` close to the editor instead of forcing every lookup
through a terminal:

- refresh `syu validate --format json` diagnostics into the Problems panel
- show the current file's linked requirements, features, policies, and
  philosophies in the **syu Context** explorer view
- jump from a spec ID to its YAML document
- open the traced files that belong to a requirement or feature
- use inline CodeLens actions on YAML spec IDs, traced files, and traced symbols
  without opening the command palette first

## Current protocol

The first cut keeps the integration intentionally small:

- diagnostics come from the checked-in `syu` CLI via `syu validate . --format json`
- navigation reads the same `docs/syu` workspace files directly so the extension
  can link source files back to requirements and features without requiring a
  second server process

That keeps the extension usable today while leaving room for a shared LSP server
later.

## Running from source

Switch your shell to the checked-in Node 20 version from
`editors/vscode/.nvmrc`, then use the pinned npm release from
`editors/vscode/package.json` to install dependencies from the repository root.
If you are hopping between the extension, docs site, and browser app, use the
repository Node workflow guide at `docs/guide/node-workflow.md` as the one-place
runtime map first:

```bash
nvm use "$(cat editors/vscode/.nvmrc)"
scripts/ci/pinned-npm.sh install editors/vscode
npm --prefix editors/vscode ci
```

1. Open `editors/vscode/` in VS Code.
2. Press `F5` to start an Extension Development Host.
3. Open a repository that contains `syu.yaml` or `docs/syu/features/features.yaml`.
4. If the `syu` binary is not on your `PATH`, set **syu › Binary Path**.

## Commands

- `syu: Refresh diagnostics`
- `syu: Trace active file`
- `syu: Open spec item by ID`
- `syu: Show related files for spec ID`

## Settings

- `syu.binaryPath`: path to the `syu` CLI binary
- `syu.autoRefreshDiagnostics`: rerun diagnostics after saves
