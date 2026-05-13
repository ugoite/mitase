# PR Repair, Merge, or Queue Workflow

## Task

Safely repair merge blockers for the requested PR, then merge or queue it only after required CI is green.

## Success criteria

- Final state is one of: `merged`, `queued`, `pending_ci`, `skipped`, or `blocked`.
- If unresolved review comments or failed required CI are fixable, make the smallest safe fix on the PR branch, validate it, push it, and re-check.
- Do not enable auto-merge or queue after a repair until required checks on the pushed head commit are green.
- Never self-approve. If the viewer is the author, skip approval.
- Never merge or queue while unresolved substantive review threads, failed required checks, conflicts, or explicit reviewer blockers remain.

## Eligibility

Proceed only if the PR already has an overview comment or is from Dependabot. Otherwise stop with `result: skipped`.

## Minimum evidence

1. `git status --short`
2. `gh pr view <PR> --json number,title,author,comments,reviews,state,isDraft,mergeStateStatus,headRefName,baseRefName,headRefOid,mergedAt,mergeCommit,files`
3. `gh pr checks <PR>`
4. Before any approval, merge, or queue request, inspect unresolved review threads with GraphQL:
   `repository(owner:$owner,name:$name){pullRequest(number:$number){reviewThreads(first:100){nodes{isResolved comments(first:20){nodes{author{login} body path line}}}}}}}`
5. `gh api user --jq '{login: .login, id: .id}'` only if approval might be attempted.

## Blocker classification

Fixable blockers:

- Unresolved substantive review comments about the PR diff.
- Failed required CI caused by PR code, tests, docs, formatting, or hooks.
- Straightforward merge conflicts.

Non-fixable blockers:

- Permission limits.
- Unclear product decisions.
- Broad conflicts.
- Reviewer comments requiring human judgment.
- External or flaky CI after one focused retry.

Ignore empty pending self-review objects. Do not ignore unresolved review threads or overview text stating a material concern.

## Repair loop

- Maximum two repair cycles total.
- Each cycle must target one named blocker.
- Check out the PR branch only when a fixable blocker exists: `gh pr checkout <PR>`.
- For unresolved comments, read only the commented file/line and directly related code. Implement the requested fix; do not re-review the PR.
- For failed CI, inspect only the first failing required job/log. Fix the smallest PR-caused issue; do not chase unrelated failures.
- Run targeted validation for the fix, then touched-area formatting/checks.
- Widen validation once only if the fix risk requires it.
- Stage only intended files, commit with a concise message, and push once clean.

## Remote CI gate after repair push

- After a push, get the new `headRefOid` and run `gh pr checks <PR>` against that PR state.
- If checks are pending, poll up to four times with short waits.
- If a required check fails, inspect only the first failing required job/log and perform the next repair cycle if available.
- If all required checks pass, proceed to merge or queue.
- If checks are still pending after polling, stop with `result: pending_ci`; do not enable auto-merge yet.

## Merge command rules

- If the PR is already in `mergeQueueEntry`, report `queued` unless it is already merged or required checks are failing.
- If branch protection or merge queue controls strategy, use `gh pr merge <PR> --auto` with no method, `--delete-branch`, or `--yes` flags; then verify queue state once.
- If no merge queue is involved, use the repository-supported method from `gh repo view --json squashMergeAllowed,mergeCommitAllowed,rebaseMergeAllowed`.
- If a merge command returns a queue-managed strategy message, do not retry with another strategy flag; check GraphQL queue state once.

## Final response

Return exactly:

```text
result: merged / queued / pending_ci / skipped / blocked
pr_number: #<PR>
actions_taken: <short summary including fixes/commits and whether remote CI became green>
validation: <commands/checks actually run>
ci_status: green / failed / pending / queued / blocked / not_checked
blockers: <none or precise blocker>
```
