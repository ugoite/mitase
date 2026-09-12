# Repository Node workflow

<!-- FEAT-DOCS-001 -->

Use this guide when you are contributing to `mitase` itself and need the
repository's Node workflow. The root `mise.toml` is the single source of truth
for the toolchain and task entrypoints.

## Quick matrix

| Surface | Maintained by | Root task entrypoints |
| --- | --- | --- |
| Docs site (`website/`) | `mise.toml` | `mise run setup:website`, `mise run build:website` |
| VS Code extension (`editors/vscode/`) | `mise.toml` | `mise run setup:vscode`, `mise run check:vscode`, `mise run test:vscode` |

The exact Node and npm versions are the `node` and `npm` entries in the root
`mise.toml`. The package `engines` fields describe compatibility for each
surface; they are not separate installation instructions.

## Provision the maintained Node surfaces

Install the root tools and locked dependencies once:

```bash
mise install
mise run setup
```

The two surface-specific setup tasks are useful when only one package needs to
be refreshed. They still use the Node and npm versions selected by root Mise.

## Docs-site work

```bash
mise run setup:website
npm --prefix website run start
```

Before opening a PR, run the same build CI uses:

```bash
mise run build:website
```

## VS Code extension work

```bash
mise run setup:vscode
mise run check:vscode
mise run test:vscode
```

If you are editing the extension and the docs site in the same session, the
same root-managed Node/npm environment serves both.

## Fast switching rules

1. Changing `website/` or docs-site build inputs? Run `mise run build:website`.
2. Changing `editors/vscode/`? Run `mise run check:vscode` and
   `mise run test:vscode`.
3. Unsure which task applies? Start with the root `mise run check` and
   `mise run test` entrypoints.

If you want the full contributor gate matrix after switching runtimes, return to
[`CONTRIBUTING.md`](https://github.com/ugoite/mitase/blob/main/CONTRIBUTING.md).
If you only need the extension setup, jump to the
[VS Code extension guide](../../workflows/integrations/vscode-extension.md).
