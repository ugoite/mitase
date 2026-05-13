# Issue Implementation and PR Creation Workflow

## Task

Implement the requested issue and create a PR.

## Success criteria

- The issue is implemented with the smallest coherent change.
- A PR is created in English with `Closes #<ISSUE>`.
- Validation is targeted first, then broadened only if risk requires it.
- CI is reported as `green`, `pending`, `failed`, or `blocked`.

## Pre-check

- Read `gh issue view <ISSUE> --json number,title,body,labels,comments,state`.
- Search linked/open PRs. If an open PR already solves the issue, stop with `result: skipped`.

## Scope guard

Implement only the requested issue. Do not review or merge PRs, create unrelated issues, or add adjacent improvements.

## Implementation budget

- Start from updated `origin/main` and create a focused branch unless the issue explicitly points to an existing branch.
- Read only code needed to understand and change the issue behavior.
- Fix root cause with existing patterns.
- If adding or changing fields, types, serialization, rendering, or CLI output, use targeted search for all same-pattern call sites before running broad tests.

## Validation sequence

1. Targeted tests or smoke test for changed behavior.
2. Type, lint, or build check for the affected package when applicable.
3. Formatting/check commands for touched areas.
4. Broader tests once only if changed behavior risk justifies it.
5. Pre-commit once before commit or final push if available.

## PR creation

Write body to `/tmp/pr-<ISSUE>.md` and create with:

```bash
gh pr create --title "<English title>" --body-file /tmp/pr-<ISSUE>.md
```

Body format:

```markdown
Summary:
- ...

Validation:
- ...

Closes #<ISSUE>
```

## Final response

Return exactly:

```text
result: pr_created / skipped / blocked
issue_number: #<ISSUE>
pr_number: <PR number or none>
branch: <branch name or none>
changes: <short summary>
validation: <commands/checks actually run>
ci_status: green / failed / pending / blocked / not_checked
blockers: <none or precise blocker>
```
