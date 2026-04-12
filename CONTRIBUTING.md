# Contributing to syu

<!-- FEAT-CONTRIB-002 -->

`syu` uses GitHub Flow. `main` is the only long-lived branch and it should stay
releaseable at all times.

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

Run the shared repository gates before opening or updating a pull request:

```bash
scripts/ci/quality-gates.sh
scripts/ci/coverage.sh summary
cargo run -- validate .
```

`scripts/ci/quality-gates.sh` also checks that the committed `docs/generated/`
artifacts are fresh. To refresh those files directly, run:

```bash
scripts/ci/check-generated-docs-freshness.sh
```

If a change touches `app/src`, `app/wasm`, or the browser build configuration,
also run:

```bash
scripts/ci/check-app-dist-freshness.sh
```

That rebuilds the browser app and fails when the checked-in `app/dist` bundle is
stale, leaving the regenerated files in your worktree so you can review and
commit them.

### Browser app workflow

For changes under `app/`, install the browser app dependencies first:

```bash
npm --prefix app ci
```

When the change affects the built bundle, run the same freshness flow CI uses:

```bash
scripts/ci/check-app-dist-freshness.sh
```

That script runs `npm run build:wasm`, `npm run check`, and `npm run build`,
then compares the regenerated output against the checked-in `app/dist` bundle.

When the change affects browser behavior, routing, or Playwright coverage, also
install the local browser once and run the end-to-end suite:

```bash
npx --prefix app playwright install --with-deps chromium
npm --prefix app run test:e2e
```

`npm run test:e2e` uses `app/playwright.config.ts` to launch
`cargo run -- app .` automatically, so run it after the shared Rust gates pass.

### Documentation site workflow

For `website/` changes, install the docs-site dependencies and use the local dev
server while iterating:

```bash
npm --prefix website ci
npm --prefix website run start
```

Before opening a pull request, run the same docs-site build CI uses:

```bash
npm --prefix website run build
```

`npm run build` regenerates the checked-in site docs first, matching
`.github/actions/build-docs-site`.

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

The devcontainer/Codespaces post-create step runs this script automatically and
also preinstalls `wasm-pack`, the `app/` dependencies, and Playwright Chromium
so browser-app validation is ready as soon as the environment finishes
provisioning.

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
