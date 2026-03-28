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

