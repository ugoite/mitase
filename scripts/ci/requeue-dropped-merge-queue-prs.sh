#!/usr/bin/env bash
# FEAT-QUALITY-001

set -euo pipefail

MERGE_QUEUE_REQUEUE_TMPFILES=()

cleanup_merge_queue_requeue_tempfiles() {
  if ((${#MERGE_QUEUE_REQUEUE_TMPFILES[@]} == 0)); then
    return 0
  fi

  rm -f "${MERGE_QUEUE_REQUEUE_TMPFILES[@]}"
}

fetch_pull_request_queue_state() {
  local owner="$1"
  local repo="$2"
  local query

  query="$(cat <<'GRAPHQL'
query($owner:String!,$repo:String!) {
  repository(owner:$owner,name:$repo) {
    pullRequests(states: OPEN, baseRefName: "main", first: 100, orderBy: {field: UPDATED_AT, direction: DESC}) {
      nodes {
        number
        title
        state
        baseRefName
        mergeStateStatus
        reviewDecision
        isInMergeQueue
        autoMergeRequest {
          enabledAt
        }
        mergeQueueEntry {
          state
        }
        commits(last: 1) {
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
  }
}
GRAPHQL
)"

  env GH_PAGER=cat gh api graphql -f query="$query" -F owner="$owner" -F repo="$repo"
}

select_requeue_candidates() {
  local queue_path="$1"

  python3 - "$queue_path" <<'PY'
import json
import sys

queue_payload = json.load(open(sys.argv[1], encoding="utf-8"))
prs = (
    queue_payload.get("data", {})
    .get("repository", {})
    .get("pullRequests", {})
    .get("nodes", [])
)

candidates = []
for pr in prs:
    rollup = (
        pr.get("commits", {})
        .get("nodes", [{}])[-1]
        .get("commit", {})
        .get("statusCheckRollup") or {}
    ).get("state")
    if pr.get("state") != "OPEN":
        continue
    if pr.get("baseRefName") != "main":
        continue
    if pr.get("mergeStateStatus") != "CLEAN":
        continue
    if pr.get("reviewDecision") not in {"APPROVED", None}:
        continue
    if pr.get("autoMergeRequest") is not None:
        continue
    if pr.get("isInMergeQueue"):
        continue
    if pr.get("mergeQueueEntry") is not None:
        continue
    if rollup != "SUCCESS":
        continue

    candidates.append(
        {
            "number": pr["number"],
            "title": pr.get("title") or "",
            "merge_state_status": pr.get("mergeStateStatus") or "",
            "review_decision": pr.get("reviewDecision") or "",
            "status_rollup": rollup or "",
        }
    )

json.dump(candidates, sys.stdout)
PY
}

render_requeue_report() {
  local candidates_path="$1"
  local dry_run="$2"

  python3 - "$candidates_path" "$dry_run" <<'PY'
import json
import os
import sys

candidates = json.load(open(sys.argv[1], encoding="utf-8"))
dry_run = sys.argv[2] == "true"
action_phrase = "would re-enable" if dry_run else "re-enabled"

if not candidates:
    message = "merge queue re-enrollment: no dropped clean PRs found."
    print(message)
    summary_path = os.environ.get("GITHUB_STEP_SUMMARY")
    if summary_path:
        with open(summary_path, "a", encoding="utf-8") as handle:
            handle.write("\n## Merge queue re-enrollment\n\n")
            handle.write(message + "\n")
    raise SystemExit(0)

lines = [
    f"merge queue re-enrollment {action_phrase} auto-merge for the following clean PRs:",
    "",
]
summary_rows = [
    "| PR | Title | mergeStateStatus | reviewDecision | status rollup |",
    "| --- | --- | --- | --- | --- |",
]
for entry in candidates:
    lines.append(
        f"- PR #{entry['number']} `{entry['title']}` "
        f"(mergeStateStatus={entry['merge_state_status']}, "
        f"reviewDecision={entry['review_decision'] or 'unknown'}, "
        f"statusCheckRollup={entry['status_rollup'] or 'unknown'})"
    )
    summary_rows.append(
        f"| #{entry['number']} | {entry['title']} | {entry['merge_state_status']} | "
        f"{entry['review_decision'] or 'unknown'} | {entry['status_rollup'] or 'unknown'} |"
    )

lines.extend(
    [
        "",
        "Next steps:",
        "1. Re-run the queue GraphQL query from the merge-queue playbook.",
        "2. Confirm `autoMergeRequest` is no longer null.",
        "3. Confirm `isInMergeQueue` or `mergeQueueEntry` becomes visible for the PR.",
    ]
)

print("\n".join(lines))
summary_path = os.environ.get("GITHUB_STEP_SUMMARY")
if summary_path:
    with open(summary_path, "a", encoding="utf-8") as handle:
        handle.write("\n## Merge queue re-enrollment\n\n")
        handle.write("\n".join(summary_rows) + "\n")
PY
}

reenroll_candidates() {
  local repo_slug="$1"
  local candidates_path="$2"

  python3 - "$candidates_path" <<'PY' | while IFS= read -r pr_number; do
import json
import sys

for entry in json.load(open(sys.argv[1], encoding="utf-8")):
    print(entry["number"])
PY
    env GH_PAGER=cat gh pr merge "$pr_number" --repo "$repo_slug" --auto --squash
  done
}

requeue_dropped_merge_queue_prs() {
  local repo_slug owner repo dry_run queue_json candidates_json

  repo_slug="${1:-${GITHUB_REPOSITORY:-ugoite/syu}}"
  dry_run="${MERGE_QUEUE_REQUEUE_DRY_RUN:-false}"
  owner="${repo_slug%%/*}"
  repo="${repo_slug#*/}"

  if [[ "$owner" == "$repo" ]]; then
    printf '%s\n' "usage: scripts/ci/requeue-dropped-merge-queue-prs.sh [owner/repo]" >&2
    exit 1
  fi

  queue_json="$(mktemp)"
  candidates_json="$(mktemp)"
  MERGE_QUEUE_REQUEUE_TMPFILES=("$queue_json" "$candidates_json")
  trap cleanup_merge_queue_requeue_tempfiles EXIT

  fetch_pull_request_queue_state "$owner" "$repo" >"$queue_json"
  select_requeue_candidates "$queue_json" >"$candidates_json"

  if [[ "$dry_run" != "true" ]]; then
    reenroll_candidates "$repo_slug" "$candidates_json"
  fi

  render_requeue_report "$candidates_json" "$dry_run"
}

main() {
  requeue_dropped_merge_queue_prs "$@"
}

main "$@"
