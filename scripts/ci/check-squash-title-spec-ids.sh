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

python3 - <<'PY' "$pr_payload"
import json
import re
import sys

pr = json.loads(sys.argv[1])
title = pr.get("title", "")
body = pr.get("body") or ""

spec_ids = sorted(set(re.findall(r"\b(?:REQ|FEAT)-[A-Z0-9-]+\b", body)))
if not spec_ids:
    print("No spec IDs found in PR body; skipping squash-title preservation check.")
    raise SystemExit(0)

missing = [spec_id for spec_id in spec_ids if spec_id not in title]
if not missing:
    print("PR title already preserves all spec IDs for squash history.")
    raise SystemExit(0)

print(
    "::error title=PR title is missing spec IDs::The PR body lists spec IDs that "
    "do not appear in the PR title: "
    + ", ".join(missing)
    + ". GitHub squash merges use the PR title as the final commit headline, so "
    "include each listed ID in the title to keep local git history traceable."
)
raise SystemExit(1)
PY
