# Maintainer playbook: merge queue and `merge_group` triage

<!-- FEAT-CONTRIB-002 -->

Use this guide when pull requests look stuck in the merge queue even though the
branch itself looks clean.

## Which workflows must react to `merge_group`

The repository currently relies on these workflows for merge-queue progress:

- `.github/workflows/ci.yml`
- `.github/workflows/codeql.yml`

Both must declare `merge_group:` in their trigger set so GitHub can run the
required checks against the queue branch, not just against the pull request
head.

## First checks when a PR looks stuck

Start with the PR itself:

```bash
gh pr view 123 --json state,mergeStateStatus,autoMergeRequest,statusCheckRollup
```

Useful quick reads:

- `state: OPEN` + `mergeStateStatus: CLEAN` usually means the branch itself is
  mergeable
- `autoMergeRequest` tells you whether the PR is waiting to enter the queue when
  ready
- `statusCheckRollup` shows whether the pull-request checks are still failing on
  the PR head before the queue branch even matters

Then inspect the queue entry directly:

```bash
gh api graphql -f query='
query($owner:String!,$repo:String!,$num:Int!) {
  repository(owner:$owner,name:$repo) {
    pullRequest(number:$num) {
      state
      isInMergeQueue
      mergeStateStatus
      mergeQueueEntry {
        position
        state
        estimatedTimeToMerge
      }
    }
  }
}' -F owner=ugoite -F repo=syu -F num=123
```

That is the fastest way to distinguish three different states that look similar
in the web UI:

- **not in queue**: `isInMergeQueue: false`
- **queued and waiting**: `isInMergeQueue: true`
- **already merged**: `state: MERGED` or a non-null `mergedAt`

## How to read common queue states

| Signal | What it usually means | What to check next |
| --- | --- | --- |
| `isInMergeQueue: true` + `mergeQueueEntry.state: AWAITING_CHECKS` | GitHub has created the queue branch and is waiting for required `merge_group` checks | inspect the latest `merge_group` workflow runs |
| `mergeStateStatus: BEHIND` | the PR head is behind `main` and may need the queue to rebuild against a newer base | check whether the queue entry exists yet and whether newer queue runs were spawned |
| `mergeStateStatus: BLOCKED` + `autoMergeRequest` present | auto-merge is enabled, but GitHub still sees an unmet requirement before queue entry | inspect failing PR-head checks, required reviews, or queue prerequisites |
| `state: MERGED` | the work is done | clean up local branches if needed |

## Inspect the `merge_group` runs

When a PR stays in `AWAITING_CHECKS`, verify the queue-branch runs rather than
the pull-request runs:

```bash
gh run list --workflow ci --event merge_group --limit 10
gh run list --workflow codeql --event merge_group --limit 10
```

For a specific run:

```bash
gh run view 123456789 --log-failed
```

The queue branch names look like:

```text
gh-readonly-queue/main/pr-123-<sha>
```

If `ci` and `codeql` both completed successfully for that queue branch but the
PR still shows `mergeQueueEntry.state = AWAITING_CHECKS`, treat that as a merge
queue incident rather than a contributor error.

## Useful recovery checks

1. Confirm both required workflows still include `merge_group:`.
2. Confirm the required job names have not drifted from branch protection
   expectations.
3. Check whether a newer queue branch was created after the runs you first
   inspected.
4. Re-read the PR GraphQL queue state to see whether the entry moved, merged, or
   dropped out of the queue.

## When to update repository docs or tests

If maintainers intentionally rename required queue jobs or add/remove required
merge-group workflows, update:

- `.github/merge-queue-checks.json`
- the affected workflow files
- repository-quality tests that assert the queue contract
- this playbook so the troubleshooting commands stay aligned with reality
