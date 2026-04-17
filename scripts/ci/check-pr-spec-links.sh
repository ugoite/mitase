#!/usr/bin/env bash
# FEAT-CONTRIB-002

set -euo pipefail

if [[ -z "${GITHUB_REPOSITORY:-}" ]]; then
  echo "GITHUB_REPOSITORY is required." >&2
  exit 1
fi

if [[ -z "${PR_NUMBER:-}" ]]; then
  echo "PR_NUMBER is required." >&2
  exit 1
fi

if [[ -z "${GH_TOKEN:-}" ]]; then
  echo "GH_TOKEN is required." >&2
  exit 1
fi

pr_payload="$(gh api "repos/${GITHUB_REPOSITORY}/pulls/${PR_NUMBER}")"
files_payload="$(gh api --paginate "repos/${GITHUB_REPOSITORY}/pulls/${PR_NUMBER}/files")"

python3 - <<'PY' "$pr_payload" "$files_payload"
import json
import re
import sys

pr = json.loads(sys.argv[1])
files = json.loads(sys.argv[2])

touches_self_spec = any(file["filename"].startswith("docs/syu/") for file in files)
if not touches_self_spec:
    print("No docs/syu/ files changed; skipping spec linkage check.")
    raise SystemExit(0)

content = "\n".join(
    part for part in [pr.get("title", ""), pr.get("body") or ""] if part.strip()
)

has_issue_ref = re.search(r"(^|\s)#\d+\b", content)
has_spec_id = re.search(r"\b(?:REQ|FEAT|POL|PHIL)-[A-Z0-9-]+\b", content)

if has_issue_ref or has_spec_id:
    print("Found linked issue/spec reference for docs/syu/ changes.")
    raise SystemExit(0)

print(
    "::error title=Missing issue or spec linkage::This pull request changes docs/syu/ "
    "but the PR title/body does not mention an issue reference like #123 or a "
    "specification ID like REQ-CORE-001 / FEAT-CHECK-001. Update the "
    "'Linked issue or specification' section in the PR template."
)
raise SystemExit(1)
PY
