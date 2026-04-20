# Repository Node workflow

<!-- FEAT-DOCS-001 -->

Use this guide when you are contributing to `syu` itself and need one place that
answers a practical question fast: **which Node major should I use for this
task right now?**

The repository intentionally does **not** use one shared Node major for every
surface. Different parts of the product move at different speeds, and the
checked-in source of truth lives next to each package rather than in tribal
knowledge.

## Quick matrix

| Surface | Checked-in source of truth | Use this Node major | Typical commands |
| --- | --- | --- | --- |
| Browser app (`app/`) | `app/.nvmrc`, `app/package.json#engines` | **Node 25** | `scripts/ci/pinned-npm.sh install app`, `npm --prefix app ci`, `scripts/ci/check-browser-app-freshness.sh`, `npm --prefix app run test:e2e` |
| Docs site (`website/`) | `website/.nvmrc`, `website/package.json#engines` | **Node 20** | `bash scripts/ci/install-docs-site-deps.sh`, `npm --prefix website run start`, `npm --prefix website run build` |
| VS Code extension (`editors/vscode/`) | `editors/vscode/.nvmrc`, `editors/vscode/package.json#engines` | **Node 20** | `scripts/ci/pinned-npm.sh install editors/vscode`, `npm --prefix editors/vscode ci`, `npm --prefix editors/vscode test` |

All checked-in Node package surfaces also pin the expected npm release through
`package.json#packageManager`, and the repository helpers read that field so the
CLI output stays aligned with CI.

## Do not trust the current shell by default

The devcontainer installs `node:lts`, and a reused local shell may already be
pointing at some other major. Treat the checked-in `.nvmrc` files as the source
of truth for contributor tasks instead of assuming the current shell is already
correct.

When you switch tasks, switch Node first. Pick the one command that matches the
surface you are about to touch:

- Browser app: `nvm use "$(cat app/.nvmrc)"`
- Docs site: `nvm use "$(cat website/.nvmrc)"`
- VS Code extension: `nvm use "$(cat editors/vscode/.nvmrc)"`

`fnm`, `Volta`, or another version manager are equally fine; the important part
is matching the checked-in major for the surface you are about to touch.

## Browser app work

Use **Node 25** for anything under `app/`, browser-app freshness checks,
Playwright, or Cargo commands that embed the browser app.

From the repository root:

```bash
nvm use "$(cat app/.nvmrc)"
scripts/ci/pinned-npm.sh install app
npm --prefix app ci
```

Use the local browser flow when you change app behavior:

```bash
scripts/ci/check-browser-app-freshness.sh
scripts/ci/validate-app.sh --e2e
```

If `app/node_modules` is missing or stale, `build.rs` fails on purpose instead
of silently running a networked install inside a normal Cargo build.

## Docs-site work

Use **Node 20** for anything under `website/` and for docs-site validation.

```bash
nvm use "$(cat website/.nvmrc)"
bash scripts/ci/install-docs-site-deps.sh
npm --prefix website run start
```

Before opening a PR, run the same build CI uses:

```bash
npm --prefix website run build
```

The install helper removes `website/node_modules` first so repeated runs stay
deterministic across branch switches and reused worktrees.

## VS Code extension work

Use **Node 20** for `editors/vscode/`.

```bash
nvm use "$(cat editors/vscode/.nvmrc)"
scripts/ci/pinned-npm.sh install editors/vscode
npm --prefix editors/vscode ci
npm --prefix editors/vscode test
```

If you are editing the extension and the docs site in the same session, you can
stay on Node 20 for both. Switch back to Node 25 before returning to browser-app
tasks.

## Fast switching rules

1. Changing `app/` or running Playwright? Use **Node 25**.
2. Changing `website/` or docs-site build inputs? Use **Node 20**.
3. Changing `editors/vscode/`? Use **Node 20**.
4. Unsure which major wins? Follow the package directory you are executing from,
   then confirm with that surface's `.nvmrc` and `package.json#engines`.

If you want the full contributor gate matrix after switching runtimes, return to
[`CONTRIBUTING.md`](https://github.com/ugoite/syu/blob/main/CONTRIBUTING.md).
If you only need the extension setup, jump to the
[VS Code extension guide](./vscode-extension.md).
