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
editor protocol later. The repository now also ships `syu lsp` as an
editor-agnostic stdio server foundation; use the dedicated
[LSP guide](./lsp.md) for direct transport details, current capabilities, and
non-VS Code client setup.

## How this relates to `syu lsp`

The repository now also ships `syu lsp`, but the VS Code extension is still
**CLI-backed first** today rather than LSP-backed first.

Current split:

- **Still uses the CLI directly**: validation diagnostics come from
  `syu validate . --format json`
- **Still reads checked-in YAML directly**: the tree view, spec-ID lookups, and
  related-file navigation read the workspace files without a language server in
  the middle
- **LSP server exists as a shared editor foundation**: `syu lsp` already gives
  editor clients a stdio transport plus hover for spec IDs, but it does not yet
  replace the extension's current diagnostics and navigation pipeline

If you want the server contract itself, start from the dedicated
[LSP guide](./lsp.md). Use `syu lsp --help` for the current CLI surface and
transport flags.

## Short-term contributor roadmap

For now, extension contributors should treat the architecture like this:

1. keep the current CLI / YAML integration as the production path for
   diagnostics and navigation
2. treat `syu lsp` as the place to grow editor-agnostic capabilities that
   multiple clients could share
3. move features from extension-only logic to LSP only when the server can
   preserve the current UX instead of regressing it

In practical terms, that means:

- diagnostics still belong to the CLI-backed flow today
- workspace graph reads and file-opening flows still belong to the extension's
  checked-in YAML model today
- hover is the first capability that can already be shared through `syu lsp`

That keeps the current extension dependable while making room for a cleaner
cross-editor story later.

## Run it from source

Switch your shell to the checked-in Node 20 version from
`editors/vscode/.nvmrc`, then use the pinned npm release from
`editors/vscode/package.json` to install dependencies from the repository root.
If you are switching between the extension, the docs site, and the browser app,
use the [repository Node workflow guide](./node-workflow.md) as the one-place
runtime map first:

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
