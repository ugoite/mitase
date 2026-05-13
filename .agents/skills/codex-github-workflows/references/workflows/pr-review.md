# PR Review Workflow

## Task

Review the requested PR.

## Success criteria

- Post inline comments only for real, actionable issues on changed lines.
- Post exactly one English overview comment.
- Create separate issues only for important, actionable, non-duplicate problems outside the diff.
- Stop when review confidence is useful; do not keep searching after no concrete risk remains.

## Eligibility

Proceed only if the PR has no existing overview comment and is not from Dependabot. Otherwise stop with `result: skipped`.

## Minimum evidence

1. `git status --short`
2. `gh pr view <PR> --json id,number,title,body,author,headRefName,baseRefName,files,comments,reviews,state,isDraft,mergeStateStatus,closingIssuesReferences,headRefOid`
3. `gh pr checks <PR>`
4. A patch snapshot using one known-good form:
   - `gh pr diff <PR> --patch --color=never > /tmp/pr-<PR>.patch`
   - or `git diff --unified=40 origin/main...<headRefOid> -- <paths-from-metadata>`

## Review budget

- Start from PR claims and changed files.
- Inspect adjacent code only to prove or refute a named risk.
- Prefer searching the saved patch before wider `rg`.
- Do not run tests only for thoroughness; run them only for a suspected behavior or confidence-changing risk.
- If a likely issue is found, get exact PR-head line numbers once with:
  `git show <headRefOid>:<path> | nl -ba | sed -n '<range>p'`

## Shell and diff safety

- Do not use `gh pr diff --stat` or append file paths after `gh pr diff`; these forms are not reliable across GitHub CLI versions.
- Avoid backticks inside double-quoted shell patterns. Use single-quoted patterns or simpler terms.
- If one diff/search command shape fails, switch once to the known-good form above; do not explore more CLI variants.

## Inline comments

- Use GraphQL `addPullRequestReviewThread` directly.
- Comment only on changed lines.
- Keep each comment English, specific, actionable, and non-speculative.

## Overview comment

Write the overview to a temp Markdown file and post exactly once. Include:

- Overall assessment.
- Strengths.
- Main concerns, if any.
- Validation performed.
- Follow-up issues, if any.

## Forbidden actions

Do not approve, merge, implement fixes, create speculative issues, or perform broad repo audits.

## Final response

Return exactly:

```text
result: reviewed / skipped / blocked
pr_number: #<PR>
inline_comments: <count and short summary>
issues_created: <issue numbers or none>
validation: <commands/checks actually run>
overview_comment: posted / not_posted
blockers: <none or precise blocker>
```
