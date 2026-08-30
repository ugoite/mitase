---
name: mitase-orient
description: Use when starting work in Mitase, routing a change to the owning repository surface, or choosing the correct validation lane.
---

# Mitase orientation

Use this skill before implementation when a task crosses repository surfaces,
mentions CI or delivery, or the owning code path is not already known.

## Read first

- `AGENTS.md`
- `README.md`
- `docs/project/mitase-re-foundation-freeze.md`
- `docs/project/adr-0002-remove-work-execution.md`
- `mise.toml`
- the relevant workflow under `.github/workflows/`
- the relevant source-of-truth document or specification

## Routing questions

Determine:

1. Which crate, document, script, or workflow owns the behavior.
2. Whether the change is authoritative, derived, adapter-only, user-facing,
   specification-facing, or operational.
3. Which source of truth and invariant must remain valid.
4. Which root `mise.toml` task is the smallest useful validation.
5. Whether the change affects `ci-rust-check`, `ci-rust-test`, `ci-repo`, or a
   separate CodeQL or release workflow.

Only root tasks from `mise.toml` are valid. Do not use GitHub Actions command
snippets as a local command source of truth.

## Boundary check

Mitase owns specification description, resolution, and verification. Do not
route WorkRequest, WorkPlan, agent lifecycle, shell/test execution, retries,
patch application, workspace mutation, or delivery state back into product
code. The former Work and Workbench runtime surfaces have been removed; do not
reintroduce them into the product.

## Worktree preflight

Before editing, record the current branch, worktree status, exact base commit,
and pre-existing changes. Start from the repository default branch in an
independent worktree or branch. Never overwrite unrelated changes.

## Output

Give a short routing result:

- owning surface and files to inspect;
- source-of-truth/invariant;
- focused validation command;
- broader CI lane, only if the change requires it.
