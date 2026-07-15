## Summary

Linked issue or specification: FEAT-WORKBENCH-READINESS-UI-001

This PR intentionally makes the v1 readiness and Workbench contracts stricter and is not compatibility-preserving before release.

## Work plan

- Request / plan: canonical artifact-index, readiness, verification, and Workbench session closure.
- Execution slice: inventory → ownership → seedability → workability → verification → post-state validation.

## Validation

- [ ] `cargo fmt --check`
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
- [ ] `cargo test --workspace`
- [ ] `syu validate workspace .`
- [ ] `syu readiness report . --format json`
