# VS Code extension

<!-- FEAT-VSCODE-001 -->

The checked-in VS Code extension under `editors/vscode/` keeps `syu` close to the
editor:

- validation diagnostics flow into the Problems panel
- the **syu Context** explorer view shows the active file's linked
  requirements, features, policies, and philosophies
- spec IDs open their YAML documents without manual terminal lookups
- requirement and feature IDs can jump straight to their traced files

## What the extension uses today

This first cut stays deliberately small:

- diagnostics come from `syu validate . --format json`
- navigation reads the checked-in spec YAML directly from the workspace

That means the extension works with today's CLI while leaving room for a shared
editor protocol later.

## Run it from source

Switch your shell to the checked-in Node 20 version from
`editors/vscode/.nvmrc`, then use the pinned npm release from
`editors/vscode/package.json` to install dependencies from the repository root:

```bash
nvm use "$(cat editors/vscode/.nvmrc)"
scripts/ci/pinned-npm.sh install editors/vscode
npm --prefix editors/vscode ci
```

1. Open `editors/vscode/` in VS Code.
2. Press `F5` to start an Extension Development Host.
3. Open a repository that contains `syu.yaml` or `docs/syu/features/features.yaml`.

If the `syu` binary is not already on your `PATH`, open VS Code settings and set
`syu.binaryPath` to the installed CLI.

## Commands

- `syu: Refresh diagnostics`
- `syu: Trace active file`
- `syu: Open spec item by ID`
- `syu: Show related files for spec ID`

`syu: Trace active file` uses the current text selection as a symbol when the
selection looks like an identifier. Without a selection, it shows file-level
ownership for the active file.

## Settings

- `syu.binaryPath`: CLI path used for diagnostics
- `syu.autoRefreshDiagnostics`: rerun diagnostics after saves
