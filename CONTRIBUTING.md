# Contributing to syu

<!-- FEAT-CONTRIB-002 -->

`syu` uses GitHub Flow. `main` is the only long-lived branch and it should stay
releaseable at all times.

## Start with the model

Before you change behavior, specs, contributor guidance, or release workflow,
read [`docs/guide/concepts.md`](docs/guide/concepts.md). For small typo-only or
mechanical CI fixes, you can start with the workflow below and come back to the
guide when you need the fuller model. `syu` is built around the four-layer
model: philosophy -> policies -> requirements -> features.

The contribution workflow exists to keep those layers connected to real code,
tests, docs, and maintenance work. In practice that means spec edits under
`docs/syu/` are part of normal feature and bug-fix work here, not optional
after-the-fact documentation.

## Working model

1. Branch from `main`.
2. Make a focused change on a short-lived branch.
3. Run the local gates.
4. Open a pull request back to `main`.
5. Merge with squash once CI is green and review conversations are resolved.
6. Delete the branch after merge.

When the PR body lists requirement or feature IDs, include
the same IDs in the PR title. GitHub uses the PR title as the squash commit headline,
so this keeps spec traceability visible in local `git log` after merge.
If the pull request changes `docs/syu/`, fill in the PR template's
**Linked issue or specification** section with either an issue reference (`#123`)
or one or more spec IDs such as `REQ-CORE-001` / `FEAT-CHECK-001`. CI checks
that self-spec changes stay anchored to explicit intent.

Use a GitHub closing keyword (`Closes #123`, `Fixes #123`, or `Resolves #123`) when this PR implements an issue so the issue closes automatically after the merge queue lands the change on `main`.

Local helper worktrees under `.worktrees/` are treated as contributor-local
state and ignored by the repository so `git status` stays focused on the main
checkout.

## Local checks

Run branch 1 for every change. Then add any later branches below that also
match your change:

<!-- FEAT-DOCTOR-001 -->

Before you choose a branch below, run `syu doctor .` when you want one quick
summary of the current Rust toolchain, Node/npm expectations, optional
dependency installs, and Playwright browser readiness for this checkout.

1. **Every change**

   ```bash
   scripts/ci/quality-gates.sh
   cargo run -- validate .
   ```

   `scripts/ci/quality-gates.sh` also checks that the committed
   `docs/generated/` artifacts are fresh. To refresh those files directly, run:

   ```bash
   scripts/ci/check-generated-docs-freshness.sh
   ```

2. **Rust logic, CLI behavior, or validation rules** (`src/`, `crates/`, tests,
   CI scripts that affect Rust flows)

   ```bash
   scripts/ci/coverage.sh summary
   ```

   That command also emits a requirement/feature coverage summary so reviewers
   can inspect the current Rust line coverage in spec terms.

3. **Browser app, WASM, or checked-in `app/dist` bundle** (`app/src`,
   `app/wasm`, browser build config, or generated browser assets)

   For the CI-aligned happy path, run:

   ```bash
   scripts/ci/validate-app.sh
   ```

   `scripts/ci/validate-app.sh` starts with the shared repository gates and then
   runs the browser-specific checks below. In the devcontainer or Codespaces,
   install the browser tooling first with:

   ```bash
   bash .devcontainer/setup-browser-tooling.sh
   ```

   Outside the devcontainer, or when you only need the raw follow-up steps,
   install the browser app dependencies with:

   ```bash
   scripts/ci/pinned-npm.sh install app
   npm --prefix app ci
   ```

Normal Cargo-driven builds no longer run that install step for you. If you are
in a fresh clone or fresh worktree and the embedded browser app dependencies are
missing or stale, `build.rs` stops and points back to the commands above so
Rust-only or docs-only work does not silently mutate your `app/node_modules`
tree.

   Then run the same freshness flow CI uses:

   ```bash
   scripts/ci/check-browser-app-freshness.sh
   ```

   That script clears any locally generated browser outputs, reruns
   `npm run build:wasm`, then runs `npm run check` and `npm run build`. It
   leaves a fresh local `app/src/wasm` bridge plus an `app/dist/` artifact
   behind for inspection without checking either generated output into git.

   When the change affects browser behavior, routing, or Playwright coverage,
   also install the local browser once and run the end-to-end suite:

   ```bash
   scripts/ci/validate-app.sh --e2e
   ```

   The wrapper expands to:

   ```bash
   npx --prefix app playwright install --with-deps chromium
   npm --prefix app run test:e2e
   ```

   `scripts/ci/validate-app.sh --e2e` also installs Playwright Chromium and runs
   `npm --prefix app run test:e2e`, which uses `app/playwright.config.ts` to
   launch `cargo run -- app .` automatically. The devcontainer/Codespaces
   post-create step keeps this browser setup opt-in so docs-only or Rust-only
   contributors do not pay for it by default.

4. **Documentation site** (`website/`)

   For the CI-aligned happy path, run:

   ```bash
   scripts/ci/validate-website.sh
   ```

   `scripts/ci/validate-website.sh` starts with the shared repository gates and
   then runs the docs-site install/build sequence below. Install the docs-site
   dependencies first when you need the raw follow-up steps only:

   ```bash
   bash scripts/ci/install-docs-site-deps.sh
   ```

   The script removes `website/node_modules` before reinstalling so repeated
   runs stay deterministic across branch switches and reused worktrees. The raw
   install step remains:

   ```bash
   scripts/ci/pinned-npm.sh install website
   npm --prefix website ci
   ```

   Use the local dev server while iterating:

   ```bash
   npm --prefix website run start
   ```

   Before opening a pull request, run the same docs-site build CI uses:

   ```bash
   npm --prefix website run build
   ```

   After the shared gates, `scripts/ci/validate-website.sh` runs the same
   install and build sequence as `.github/actions/build-docs-site`.

5. **Docs-only edits outside `website/`, `app/`, or Rust logic**

   Stop after the shared gates only when your change does not feed the docs
   site. If you touched `README.md`, files under `docs/guide/` or
   `docs/generated/site-spec/`, or docs-site build inputs such as
   `scripts/generate-site-docs.py` or `.github/actions/build-docs-site`, also
   run branch 4's docs-site build.

### Node.js version strategy

For the contributor-facing quick matrix, switching rules, and app/docs/editor
commands in one place, start with [`docs/guide/node-workflow.md`](docs/guide/node-workflow.md).

The repository intentionally uses different Node.js majors for different
surfaces:

- **Browser app and browser-adjacent CI jobs use Node 25.** That is the runtime
  used in `ci/npm-audit` for `app/` and in the `check-msrv` job that still
  bootstraps the browser toolchain.
- **Docs-site automation uses Node 20.** The shared
  `.github/actions/build-docs-site` action pins the Docusaurus build to Node 20,
  and `ci/npm-audit` uses the same major for `website/`.

When you work locally, match the Node major to the surface you are changing:

- use **Node 25** for `app/`, browser-app freshness checks, Playwright, and
  other browser-tooling work
- use **Node 20** for `website/` and docs-site builds

The checked-in source of truth now lives in each package directory:
`app/.nvmrc` plus `app/package.json#engines`, and `website/.nvmrc` plus
`website/package.json#engines`.

If you switch between both in one shell session, use a version manager such as
`nvm`, `fnm`, or `Volta` so the browser app and docs site each run on the same
major that CI expects. Both checked-in Node surfaces also record the expected
npm release in `package.json` via the `packageManager` field.

### Rust version

The minimum supported Rust version (MSRV) is **1.88**. CI verifies this with a
dedicated `check-msrv` job. You can test locally against the MSRV by running:

```bash
rustup toolchain install 1.88
cargo +1.88 check --all-targets
```

If you use the hooks, install them once:

```bash
scripts/install-precommit.sh
```

If bootstrap fails or the script cannot find the final `pre-commit` binary, run
the same interpreter check that the script selected:
`python3 -m site --user-base` (or `python -m site --user-base` when the script
detected `python` instead of `python3`). If you installed `pre-commit` with
`pipx`, also run `pipx environment --value PIPX_BIN_DIR` to see where the
binary should live. Compare the reported paths with your `PATH`, then rerun
`scripts/install-precommit.sh`. The script prints the same checks when setup
fails.

The devcontainer/Codespaces post-create step runs
`.devcontainer/post-create.sh` automatically so the setup explains itself while
it provisions. That script:

- installs `cargo-llvm-cov` for `scripts/ci/coverage.sh summary`
- installs `wasm-pack` plus the `app/` dependencies for local browser-app work,
  `scripts/ci/check-browser-app-freshness.sh`, and
  `npm --prefix app run test:e2e`
- installs Playwright Chromium for `npm --prefix app run test:e2e`
- runs `scripts/install-precommit.sh` so local hooks match the contributor path

Read the script output or this section when you want to map setup time to the
checks it unlocks. It does **not** install `website/` docs-site dependencies,
so run `bash scripts/ci/install-docs-site-deps.sh` yourself when you are working
on the docs site.

## Dependency security

Dependency advisories are checked automatically on a weekly schedule (every Monday
at 06:00 UTC) via the CI workflow. The scheduled run only executes the
`dependency-audit` job (`cargo audit`) and the `npm-audit` job (`npm audit`
against both `app/` and `website/`), so the rest of CI is not rerun weekly.
Contributors do **not** need to run manual audits — failed scheduled runs are
reported via the default GitHub Actions failure notification for maintainers.

## Expectations for changes

- update the self-hosted specification in `docs/syu/` when behavior changes
- update docs and examples when user-facing workflows change
- add or update tests for new behavior
- keep the GitHub Pages docs deployment working when `website/` or docs-site workflow files change
- keep `main` ready for the next release

## Releases

Stable releases are prepared from `main` with release-please.
Prereleases are cut from `main` as needed after the same quality gates and user
story validation pass.

Maintainers triaging stuck merge-queue entries should use the
[merge queue playbook](docs/guide/merge-queue-playbook.md) to inspect
`merge_group` runs, queue state, and required workflow coverage.

When queue enrollment disappears for a clean PR, prefer the checked-in
`scripts/ci/requeue-dropped-merge-queue-prs.sh` workflow path before manually
toggling auto-merge in the UI so the recovery flow stays auditable.

When maintainers intentionally rename merge-queue check contexts or add/remove
`merge_group` workflows, update `.github/merge-queue-checks.json` and the
repository-quality assertions in the same change.

GitHub release notes are generated per release track so alpha, beta, and stable
releases each compare against the previous tag in the same track.

### Changelog

`CHANGELOG.md` is generated automatically by release-please from conventional
commit messages. The changelog is updated on every release PR; do not edit it
by hand.

Commit messages that appear in the changelog follow the
[Conventional Commits](https://www.conventionalcommits.org/) format:

| Prefix | Section in changelog |
|--------|---------------------|
| `feat:` | Features |
| `fix:` | Bug Fixes |
| `docs:` | Documentation |
| `ci:` | CI/CD |
| `chore:` | Miscellaneous |
| `BREAKING CHANGE:` footer | Breaking Changes |

Write the subject line in the imperative mood (e.g. `feat: add syu list command`)
so the generated changelog reads naturally.

### Migration notes

Every PR that introduces a **breaking change** must add a corresponding entry to
`docs/guide/migration.md` before the PR is merged. A breaking change is any of:

- A `syu.yaml` field added, removed, or with a changed default
- A spec YAML schema change that requires user edits to existing `docs/syu/` files
- A new default-on validation rule (one that was previously off or did not exist)
- A CLI flag that is renamed, removed, or has a changed default

The migration entry must include the target version, a table of old → new
values, and the exact steps needed to upgrade an existing repository without
breakage. See `docs/guide/migration.md` for the expected format.
