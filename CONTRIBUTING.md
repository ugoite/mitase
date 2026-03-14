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

## Local checks

Run the shared repository gates before opening or updating a pull request:

```bash
scripts/ci/quality-gates.sh
scripts/ci/coverage.sh summary
cargo run -- validate .
```

If you use the hooks, install them once:

```bash
python -m pip install pre-commit
pre-commit install --hook-type pre-commit --hook-type pre-push
```

## Expectations for changes

- update the self-hosted specification in `docs/spec/` when behavior changes
- update docs and examples when user-facing workflows change
- add or update tests for new behavior
- keep `main` ready for the next release

## Releases

Stable releases are prepared from `main` with release-please.
Prereleases are cut from `main` as needed after the same quality gates and user
story validation pass.
