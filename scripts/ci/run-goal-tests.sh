#!/usr/bin/env bash
# FEAT-QUALITY-001

set -euo pipefail

run_goal_tests() {
  local repo_root
  local range="${1:-origin/main...HEAD}"
  local goal_plan_path="${2:-target/syu/goal.yaml}"
  local selected_tests_path="target/syu/selected-tests.json"

  repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
  cd "$repo_root"

  mkdir -p "$(dirname "$goal_plan_path")"

  cargo run --quiet -- task infer --range "$range" --output "$goal_plan_path"
  cargo run --quiet -- task test-select "$goal_plan_path" --format json >"$selected_tests_path"

  python3 - "$selected_tests_path" <<'PY'
import json
import subprocess
import sys
from pathlib import Path

selected_tests_path = Path(sys.argv[1])
plan = json.loads(selected_tests_path.read_text(encoding="utf-8"))
goal_id = plan.get("goal_id", "<unknown>")
goal_title = plan.get("goal_title", "<unknown>")
commands = plan.get("commands", [])
unique_commands = []
seen_commands = set()

for item in commands:
    command = item["command"]
    if command in seen_commands:
        continue
    seen_commands.add(command)
    unique_commands.append(item)

if not unique_commands:
    raise SystemExit(f"no goal-selected test commands were produced for {goal_id}")

print(f"goal: {goal_id} - {goal_title}")
print(f"selection mode: {plan.get('selection_mode', '<unknown>')}")
print(f"escalation: {plan.get('escalation', {}).get('level', '<unknown>')}")

for item in unique_commands:
    command = item["command"]
    reason = item.get("reason", "")
    language = item.get("language", "")
    print(f"running [{language}] {command}")
    if reason:
        print(f"  reason: {reason}")
    subprocess.run(command, shell=True, check=True)
PY
}

run_goal_tests "$@"
