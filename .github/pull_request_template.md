<!-- FEAT-CONTRIB-002 -->

## Summary

Describe the user-visible change and the repository surfaces it touches.

## Linked issue or specification

- Closing keyword issue: `Closes #123` / `Fixes #123` / `Resolves #123`
- Requirement / feature IDs:

Use a GitHub closing keyword (`Closes #123`, `Fixes #123`, or `Resolves #123`) when this PR implements an issue so the issue closes automatically after the merge queue lands the change on `main`.

If you list requirement or feature IDs here, include the same IDs in the PR title so the squash commit headline preserves them in `git log`.

## Validation

- [ ] `scripts/ci/quality-gates.sh`
- [ ] `scripts/ci/coverage.sh summary` (required when Rust logic changes)
- [ ] `cargo run -- validate .`
- [ ] Docs, examples, or self-spec updated when behavior changed

## Release notes

- [ ] This change should appear in the next release notes
- [ ] No release note needed
