# Issue Creation Workflow

## Task

Create high-quality GitHub issues for meaningful repository problems.

## Success criteria

- Create at most the requested maximum number of concise English issues. Default to 3 if the user gives no maximum.
- Every issue is important, actionable, non-duplicate, and grounded in repository evidence.
- Stop instead of filling the quota with weak findings.

## Precondition

Check open issue count first. If open issues exceed 50, stop with `result: skipped`.

## Scope

Only create issues. Do not implement fixes, review PRs, approve, or merge.

## Audit budget

Choose exactly one audit area from quick evidence:

- Failing CI/tests.
- First-time-user docs.
- CLI UX.
- Frontend UX.
- Security risk.
- Repository philosophy mismatch.
- Developer experience.
- Docs/docsite consistency.
- `syu` / `ugoite-syu` philosophy alignment.

Then:

- Read issue templates.
- Inspect only evidence needed for that audit area.
- Before each issue, search existing issues for duplicates and confirm importance/actionability.

## Issue body requirements

Each issue body must include:

- Actual behavior.
- Expected behavior.
- Evidence or reproduction.
- Affected area.
- Suggested direction, if useful.

## Final response

Return exactly:

```text
result: issues_created / skipped / blocked
open_issue_count_before: <number>
audit_area: <chosen area>
issues_created: <issue numbers and titles, or none>
validation: <commands/checks actually run>
blockers: <none or precise blocker>
```
