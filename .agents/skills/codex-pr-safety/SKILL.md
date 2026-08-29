---
name: codex-pr-safety
description: Use when creating or updating Mitase pull requests so public content is validated before submission.
---

# Codex PR safety

Use this skill before opening a pull request.

## Read first

- `.github/pull_request_template.md`
- `.github/workflows/pr-require-close-issue.yml`
- the relevant Issue

## Workflow

1. Write the PR body to a Markdown file.
2. Keep the body aligned with the repository template and explicit closing
   Issue syntax.
3. Validate that the body contains no private conversation, internal prompt,
   local path, credential, token, usage data, or private history.
4. Create the PR with a body-file command.
5. Use the PR link only after creation succeeds.

The public body should contain only the safe problem statement, design/boundary
decision, changed behavior, related Issue, and validation evidence.
