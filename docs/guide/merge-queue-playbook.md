# Maintainer playbook: merge queue and `merge_group` triage

<!-- FEAT-CONTRIB-002 -->

Use this guide when pull requests look stuck in the merge queue even though the
branch itself looks clean.

## Know the difference between auto-merge and an actual queue entry

These two states look close in the GitHub UI but mean different things:

- **auto-merge enabled**: GitHub is willing to queue the PR once every
  prerequisite is satisfied
- **in the merge queue**: GitHub has already created a queue entry and will run
  the required `merge_group` checks for that entry

Check both before you assume the queue is doing work:

```bash
gh api graphql -f query='
query($owner:String!,$repo:String!,$num:Int!) {
  repository(owner:$owner,name:$repo) {
    pullRequest(number:$num) {
      state
      reviewDecision
      mergeStateStatus
      autoMergeRequest {
        enabledAt
      }
      isInMergeQueue
      mergeQueueEntry {
        position
        state
        estimatedTimeToMerge
      }
      commits(last:1) {
        nodes {
          commit {
            statusCheckRollup {
              state
            }
          }
        }
      }
    }
  }
}' -F owner=ugoite -F repo=syu -F num=123
```

- `autoMergeRequest != null` means auto-merge is enabled
- `isInMergeQueue` plus `mergeQueueEntry` tell you whether GitHub actually
  created a queue entry yet
- `reviewDecision` tells you whether GitHub still sees a review requirement
- `commits.nodes[0].commit.statusCheckRollup.state` tells you whether the
  current PR-head checks are green enough to enter the queue at all

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

If `autoMergeRequest` is present but `isInMergeQueue` is still `false`, the PR
is only **queue-eligible**. GitHub is still waiting on another prerequisite.

## Review threads can block queue progress

The merge queue only helps once GitHub considers the PR review-complete.
Unresolved review conversations can still block queue entry even when the branch
looks clean and every required job is green.

When a PR looks queue-ready but will not enroll, check:

1. `reviewDecision` from `gh pr view ... --json reviewDecision`
2. the review-thread state in the GitHub UI
3. recent merge errors such as `All comments must be resolved`

If maintainers intentionally want the PR to advance, resolve or explicitly close
the outstanding review conversations first. Do not assume `mergeStateStatus:
CLEAN` is enough on its own.

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

## Use the merge queue watchdog before you start poking at PRs

The repository now ships a watchdog that checks for the exact incident pattern
from issue #328: queue entries that are still `AWAITING_CHECKS` even though the
latest required `merge_group` workflows already succeeded.

- Scheduled automation: `.github/workflows/merge-queue-watchdog.yml`
- Manual local check: `scripts/ci/check-merge-queue-health.sh`

To run the same query locally:

```bash
GH_TOKEN="$(gh auth token)" bash scripts/ci/check-merge-queue-health.sh
```

When the watchdog fails, treat that as proof that GitHub lost track of queue
progress rather than as evidence that contributor code is still broken.

## Useful recovery checks

1. Confirm both required workflows still include `merge_group:`.
2. Confirm the required job names have not drifted from branch protection
   expectations.
3. Check whether a newer queue branch was created after the runs you first
   inspected.
4. Re-read the PR GraphQL queue state to see whether the entry moved, merged, or
   dropped out of the queue.

## When to requeue

Requeue the PR when the branch is still valid but GitHub has dropped the queue
entry or left the PR outside the queue after base churn.

Typical signals:

- `mergeStateStatus: CLEAN`
- `autoMergeRequest: null`
- `isInMergeQueue: false`
- `mergeQueueEntry: null`

That combination means the PR is open and mergeable, but GitHub is no longer
trying to merge it.

To requeue from the CLI:

```bash
gh pr merge 123 --auto --squash
```

Or use the GitHub UI to re-enable auto-merge. After that, re-run the GraphQL
queue query above and confirm that `isInMergeQueue` turned `true` or
`mergeQueueEntry` became non-null before walking away.

The repository also ships a scheduled/manual recovery workflow for this failure
mode:

```bash
bash scripts/ci/requeue-dropped-merge-queue-prs.sh
MERGE_QUEUE_REQUEUE_DRY_RUN=true bash scripts/ci/requeue-dropped-merge-queue-prs.sh
```

Use the dry-run mode first when you want a report without changing GitHub state.
The scheduled workflow re-enables auto-merge only for PRs that are still open,
clean, approved, outside the queue, and already green on the PR head.

## When to update repository docs or tests

If maintainers intentionally rename required queue jobs or add/remove required
merge-group workflows, update:

- `.github/merge-queue-checks.json`
- the affected workflow files
- `scripts/ci/requeue-dropped-merge-queue-prs.sh` if the queue signals change
- repository-quality tests that assert the queue contract
- this playbook so the troubleshooting commands stay aligned with reality
