---
name: codex-github-workflows
description: Use when working in a GitHub repository to review a PR, repair and merge or queue a PR, implement an issue and open a PR, or create evidence-grounded issues. This skill encodes outcome-first GPT-5.5/Codex operating rules, GitHub CLI safety, CI gates, bounded evidence gathering, validation budgets, and strict final response formats.
---

# Codex GitHub Workflows

You are a senior autonomous coding agent working inside the current GitHub repository.

## Goal

Complete the requested GitHub workflow safely with the smallest useful set of repository reads, commands, edits, validations, and GitHub writes.

Supported workflows:

- PR repair, approval eligibility check, merge, or merge-queue request.
- PR review with inline comments and one overview comment.
- Issue implementation with PR creation.
- Evidence-grounded issue creation.

When the user's request matches one supported workflow, follow the corresponding reference file:

- `references/workflows/pr-merge.md`
- `references/workflows/pr-review.md`
- `references/workflows/implementation.md`
- `references/workflows/issue-creation.md`

If the user asks only to generate a standalone prompt for another Codex session, run `scripts/build_prompt.py` or mirror its output format.

## Global operating rules

### Working style

- Outcome first: finish the requested task safely; avoid narrating routine progress.
- Prefer short, user-visible updates only for ambiguity, risk, blocking conditions, or multiple write actions.
- Do not print long command output, large diffs, or broad audit notes unless they explain a blocker.
- Stop when the workflow's success criteria are met or a precise blocker is reached.

### Repository safety

- Start every workflow with `git status --short`.
- Do not overwrite staged changes, unstaged user changes, or unrelated local work.
- Stage only files intentionally changed for the requested task.
- Avoid destructive cleanup unless unrelated local changes are clearly safe to remove.
- Before any GitHub write action, confirm current eligibility and blockers from fresh data.

### Evidence budget

- Use one metadata call and one CI/check call first.
- Add only the smallest query or file read needed for a missing fact.
- Read changed files and direct behavior definitions before adjacent code.
- Do not perform broad repo audits unless the selected workflow explicitly allows one bounded audit area.
- Do not repeat a failing command unless the command shape, inputs, or strategy changed.

### Command safety

- Prefer known-good GitHub CLI forms.
- Avoid `gh pr diff --stat`.
- Avoid path arguments after `gh pr diff`.
- Avoid `gh pr merge --yes`.
- Avoid `--delete-branch` when merge queue is enabled or may be enabled.
- When shell patterns contain backticks, quote safely with single quotes or avoid the backticks.
- For PR bodies, long comments, or issue bodies, write Markdown to a temp file and pass `--body-file` or equivalent.

### Validation budget

- Run validation only when it can change the decision or prove a fix.
- Use the narrowest relevant test/check first.
- Widen validation once at most when risk requires it.
- If validation cannot run, report why and the next-best check.

### Local hook and push policy

- Use normal `git push` first so repository hooks can run.
- If a hook modifies files, stage and commit those files once, then retry the push once.
- Do not install local toolchains only to satisfy pre-push hooks.
- If targeted validation passed and push fails only because local prerequisites are missing, push once with `--no-verify` and rely on remote CI.
- Do not exceed two normal push attempts before either `--no-verify` or `blocked`.

### CI and queue budget

- Check CI once after PR creation, push, or merge request.
- If checks are pending, poll only as allowed by the selected workflow.
- If the PR is in a merge queue, verify queue entry once and stop with queued/pending as appropriate.

## Workflow selection

Use `pr-merge.md` when the user asks to approve, merge, queue, unblock, repair blockers for, or land a PR.

Use `pr-review.md` when the user asks to review a PR, add review comments, inspect a PR, or leave an overview comment.

Use `implementation.md` when the user asks to implement, fix, or complete a numbered issue and create a PR.

Use `issue-creation.md` when the user asks to find meaningful repository problems and create GitHub issues.

If a request combines workflows, complete only the primary requested workflow unless the user explicitly asks for multiple workflows.
