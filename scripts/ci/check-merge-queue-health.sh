#!/usr/bin/env bash
# FEAT-QUALITY-001

set -euo pipefail

MERGE_QUEUE_WATCHDOG_TMPFILES=()

cleanup_merge_queue_tempfiles() {
  if ((${#MERGE_QUEUE_WATCHDOG_TMPFILES[@]} == 0)); then
    return 0
  fi

  rm -f "${MERGE_QUEUE_WATCHDOG_TMPFILES[@]}"
}

load_required_merge_queue_workflows() {
  local manifest_path="$1"

  python3 - "$manifest_path" <<'PY'
import json
import sys

with open(sys.argv[1], encoding="utf-8") as handle:
    manifest = json.load(handle)

workflows = sorted(
    {
        entry["workflow"]
        for entry in manifest.get("required_checks", [])
        if entry.get("workflow")
    }
)
print("\n".join(workflows))
PY
}

fetch_merge_queue_state() {
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
        mergeStateStatus
        isInMergeQueue
        autoMergeRequest {
          enabledAt
        }
        mergeQueueEntry {
          state
        }
      }
    }
  }
}
GRAPHQL
)"

  env GH_PAGER=cat gh api graphql -f query="$query" -F owner="$owner" -F repo="$repo"
}

fetch_merge_group_runs() {
  local owner="$1"
  local repo="$2"
  local run_limit="$3"

  env GH_PAGER=cat gh run list --repo "$owner/$repo" --event merge_group --limit "$run_limit" \
    --json databaseId,workflowName,headBranch,status,conclusion
}

render_watchdog_report() {
  local required_path="$1"
  local queue_path="$2"
  local runs_path="$3"

  python3 - "$required_path" "$queue_path" "$runs_path" <<'PY'
import json
import os
import re
import sys

required_path, queue_path, runs_path = sys.argv[1:4]
required_workflows = [
    line.strip()
    for line in open(required_path, encoding="utf-8").read().splitlines()
    if line.strip()
]
queue_payload = json.load(open(queue_path, encoding="utf-8"))
runs_payload = json.load(open(runs_path, encoding="utf-8"))

queue_prs = (
    queue_payload.get("data", {})
    .get("repository", {})
    .get("pullRequests", {})
    .get("nodes", [])
)

branch_pattern = re.compile(r"^gh-readonly-queue/main/pr-(?P<number>\d+)-")
latest_runs = {}
for run in runs_payload:
    branch = run.get("headBranch") or ""
    match = branch_pattern.match(branch)
    if not match:
        continue
    workflow = run.get("workflowName")
    if workflow not in required_workflows:
        continue
    pr_number = int(match.group("number"))
    latest_runs.setdefault(pr_number, {})
    latest_runs[pr_number].setdefault(workflow, run)

stuck_entries = []
for pr in queue_prs:
    queue_entry = pr.get("mergeQueueEntry") or {}
    if queue_entry.get("state") != "AWAITING_CHECKS":
        continue

    pr_number = pr["number"]
    observed_runs = latest_runs.get(pr_number, {})
    if not required_workflows:
        continue
    if all(
        observed_runs.get(workflow, {}).get("conclusion") == "success"
        for workflow in required_workflows
    ):
        stuck_entries.append(
            {
                "number": pr_number,
                "title": pr.get("title") or "",
                "merge_state_status": pr.get("mergeStateStatus") or "",
                "auto_merge_enabled": bool(pr.get("autoMergeRequest")),
                "runs": observed_runs,
            }
        )

if not stuck_entries:
    message = (
        "merge queue watchdog: no stuck AWAITING_CHECKS entries found after "
        f"successful {', '.join(required_workflows)} merge_group runs."
    )
    print(message)
    summary_path = os.environ.get("GITHUB_STEP_SUMMARY")
    if summary_path:
        with open(summary_path, "a", encoding="utf-8") as handle:
            handle.write("\n## Merge queue watchdog\n\n")
            handle.write(message + "\n")
    raise SystemExit(0)

lines = [
    "merge queue watchdog found PRs that are still `AWAITING_CHECKS` even though "
    "their latest required merge_group workflows already succeeded:",
    "",
]
summary_rows = [
    "| PR | Title | mergeStateStatus | auto-merge | Successful merge_group runs |",
    "| --- | --- | --- | --- | --- |",
]
for entry in stuck_entries:
    run_summary = ", ".join(
        f"{workflow}#{entry['runs'][workflow]['databaseId']}"
        for workflow in required_workflows
    )
    lines.append(
        f"- PR #{entry['number']} `{entry['title']}` "
        f"(mergeStateStatus={entry['merge_state_status']}, "
        f"auto_merge_enabled={entry['auto_merge_enabled']}) -> {run_summary}"
    )
    summary_rows.append(
        f"| #{entry['number']} | {entry['title']} | {entry['merge_state_status']} | "
        f"{'enabled' if entry['auto_merge_enabled'] else 'disabled'} | {run_summary} |"
    )

lines.extend(
    [
        "",
        "Next steps:",
        "1. Confirm the queue state again with the merge-queue playbook GraphQL query.",
        "2. Requeue the PR with `gh pr merge <number> --auto --squash` if GitHub no longer advances it.",
        "3. Treat repeated hits as a merge queue incident, not a contributor error.",
    ]
)

print("\n".join(lines))
summary_path = os.environ.get("GITHUB_STEP_SUMMARY")
if summary_path:
    with open(summary_path, "a", encoding="utf-8") as handle:
        handle.write("\n## Merge queue watchdog\n\n")
        handle.write("\n".join(summary_rows) + "\n\n")
        handle.write(
            "These PRs are still `AWAITING_CHECKS` even though their latest required "
            "merge_group workflows completed successfully.\n"
        )

raise SystemExit(1)
PY
}

check_merge_queue_health() {
  local repo_root repo_slug owner repo run_limit
  local queue_json runs_json workflows_txt

  repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
  repo_slug="${1:-${GITHUB_REPOSITORY:-ugoite/syu}}"
  run_limit="${MERGE_QUEUE_RUN_LIMIT:-100}"
  owner="${repo_slug%%/*}"
  repo="${repo_slug#*/}"

  if [[ "$owner" == "$repo" ]]; then
    printf '%s\n' "usage: scripts/ci/check-merge-queue-health.sh [owner/repo]" >&2
    exit 1
  fi

  workflows_txt="$(mktemp)"
  queue_json="$(mktemp)"
  runs_json="$(mktemp)"
  MERGE_QUEUE_WATCHDOG_TMPFILES=("$workflows_txt" "$queue_json" "$runs_json")
  trap cleanup_merge_queue_tempfiles EXIT

  load_required_merge_queue_workflows "$repo_root/.github/merge-queue-checks.json" >"$workflows_txt"
  fetch_merge_queue_state "$owner" "$repo" >"$queue_json"
  fetch_merge_group_runs "$owner" "$repo" "$run_limit" >"$runs_json"
  render_watchdog_report "$workflows_txt" "$queue_json" "$runs_json"
}

main() {
  check_merge_queue_health "$@"
}

main "$@"
