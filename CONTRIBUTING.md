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

Local helper worktrees under `.worktrees/` are treated as contributor-local
state and ignored by the repository so `git status` stays focused on the main
checkout.

## Local checks

Run branch 1 for every change. Then add any later branches below that also
match your change:

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

3. **Browser app, WASM, or browser build inputs** (`app/src`, `app/src/wasm`,
   `app/wasm`, browser build config, or browser build scripts)

   Install the browser app dependencies first:

   ```bash
   npm --prefix app ci
   ```

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
   npx --prefix app playwright install --with-deps chromium
   npm --prefix app run test:e2e
   ```

   `npm run test:e2e` uses `app/playwright.config.ts` to launch
   `cargo run -- app .` automatically, so run it after the shared Rust gates
   pass.

4. **Documentation site** (`website/`)

   Install the docs-site dependencies first:

   ```bash
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

   `npm run build` regenerates the checked-in site docs first, matching
   `.github/actions/build-docs-site`.

5. **Docs-only edits outside `website/`, `app/`, or Rust logic**

   Stop after the shared gates only when your change does not feed the docs
   site. If you touched `README.md`, files under `docs/guide/` or
   `docs/generated/site-spec/`, or docs-site build inputs such as
   `scripts/generate-site-docs.py` or `.github/actions/build-docs-site`, also
   run branch 4's docs-site build.

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
so run `npm --prefix website ci` yourself when you are working on the docs site.

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
