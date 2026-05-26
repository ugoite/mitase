#!/usr/bin/env bash
# FEAT-QUALITY-001

set -euo pipefail

ensure_cargo_bin_in_path() {
  local cargo_bin_dir

  cargo_bin_dir="${HOME}/.cargo/bin"

  if [[ -d "$cargo_bin_dir" && ":${PATH}:" != *":${cargo_bin_dir}:"* ]]; then
    PATH="${cargo_bin_dir}:${PATH}"
    export PATH
  fi
}

configure_llvm_tools() {
  if [[ -n "${LLVM_COV:-}" && -n "${LLVM_PROFDATA:-}" ]]; then
    return 0
  fi

  if command -v xcrun >/dev/null 2>&1; then
    LLVM_COV="${LLVM_COV:-$(xcrun --find llvm-cov 2>/dev/null || true)}"
    LLVM_PROFDATA="${LLVM_PROFDATA:-$(xcrun --find llvm-profdata 2>/dev/null || true)}"
    export LLVM_COV
    export LLVM_PROFDATA
  fi
}

generate_lcov() {
  local output_path="$1"

  mkdir -p "$(dirname "$output_path")"
  if cargo nextest --help >/dev/null 2>&1; then
    cargo llvm-cov nextest --lcov --output-path "$output_path"
  else
    cargo llvm-cov test --lcov --output-path "$output_path"
  fi
}

generate_spec_coverage_summary() {
  local lcov_path="$1"
  local output_path="$2"

  python3 scripts/ci/write-spec-coverage-summary.py "$lcov_path" "$output_path"

  if [[ -n "${GITHUB_STEP_SUMMARY:-}" ]]; then
    {
      echo
      cat "$output_path"
    } >>"$GITHUB_STEP_SUMMARY"
  fi
}

report_lcov_coverage() {
  local lcov_path="$1"

  python3 - "$lcov_path" <<'PY'
import sys

lcov_path = sys.argv[1]
covered = 0
total = 0
current_file = None
misses = []

with open(lcov_path, encoding="utf-8") as handle:
    for line in handle:
        if line.startswith("SF:"):
            current_file = line[3:].strip()
            continue
        if not line.startswith("DA:"):
            continue
        _, payload = line.split(":", 1)
        line_number_text, count = payload.strip().split(",", 1)
        line_number = int(line_number_text)
        total += 1
        if int(count) > 0:
            covered += 1
        else:
            misses.append((current_file, line_number))

coverage = 100.0 if total == 0 else covered * 100.0 / total
print(f"line coverage: {coverage:.2f}% ({covered}/{total})")

if misses:
    print("\nuncovered executable lines:")
    by_file = {}
    for filename, line_number in misses:
        by_file.setdefault(filename or "<unknown>", []).append(line_number)
    for filename, line_numbers in by_file.items():
        print(f"\n{filename} ({len(line_numbers)} missed)")
        try:
            with open(filename, encoding="utf-8") as source:
                source_lines = source.read().splitlines()
        except OSError:
            source_lines = []
        for line_number in line_numbers:
            snippet = ""
            if 1 <= line_number <= len(source_lines):
                snippet = source_lines[line_number - 1].strip()
            print(f"  {line_number}: {snippet}")
PY
}

enforce_diff_coverage() {
  local lcov_path="$1"
  local base_ref="${2:-origin/main}"
  local goal_plan_path="${3:-}"
  local output_format="${4:-text}"

  python3 - "$lcov_path" "$base_ref" "$goal_plan_path" "$output_format" <<'PY'
import collections
import json
import subprocess
import sys
from pathlib import Path

lcov_path = Path(sys.argv[1])
base_ref = sys.argv[2]
goal_plan_path = sys.argv[3] if len(sys.argv) > 3 else ""
output_format = sys.argv[4] if len(sys.argv) > 4 else "text"

repo_root = Path(
    subprocess.check_output(
        ["git", "rev-parse", "--show-toplevel"], text=True
    ).strip()
)

def normalize_repo_path(raw_path: str) -> Path:
    path = Path(raw_path)
    if path.is_absolute():
        try:
            return path.resolve().relative_to(repo_root)
        except ValueError:
            return path.resolve()
    return path

def ensure_base_ref() -> None:
    try:
        subprocess.run(
            ["git", "rev-parse", "--verify", base_ref],
            check=True,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            text=True,
        )
    except subprocess.CalledProcessError:
        subprocess.run(["git", "fetch", "--depth=1", "origin", "main"], check=True)

def changed_lines() -> dict[Path, set[int]]:
    diff = subprocess.check_output(
        ["git", "diff", "--unified=0", f"{base_ref}...HEAD", "--", "*.rs"],
        text=True,
    )
    current_path = None
    new_line = None
    lines: dict[Path, set[int]] = collections.defaultdict(set)

    for raw_line in diff.splitlines():
        if raw_line.startswith("+++ b/"):
            current_path = normalize_repo_path(raw_line[6:].strip())
            continue
        if raw_line.startswith("@@ ") and current_path is not None:
            _, plus, _ = raw_line.split(" ", 2)
            start_text, count_text = plus[1:].split(",") if "," in plus[1:] else (plus[1:], "1")
            new_line = int(start_text)
            remaining = int(count_text)
            continue
        if current_path is None or new_line is None:
            continue
        if raw_line.startswith("+") and not raw_line.startswith("+++ "):
            lines[current_path].add(new_line)
            new_line += 1
            continue
        if raw_line.startswith("-") and not raw_line.startswith("--- "):
            continue
        if raw_line.startswith(" "):
            new_line += 1
            continue
        # Ignore non-hunk context.
    return lines

def is_test_path(path: Path) -> bool:
    return "tests" in path.parts or path.as_posix().startswith("test/")

def is_generated_path(path: Path) -> bool:
    raw = path.as_posix()
    return raw.startswith("docs/generated/") or raw.startswith("target/") or "/generated/" in raw

def ignored_task_coverage_lines(path: Path) -> set[int]:
    if path.as_posix() != "src/command/task.rs":
        return set()

    try:
        source_lines = path.read_text(encoding="utf-8").splitlines()
    except OSError:
        return set()

    ignored_lines: set[int] = set()
    ignore = False
    for line_number, line in enumerate(source_lines, start=1):
        stripped = line.strip()
        if stripped == "// coverage:ignore-start":
            ignore = True
            ignored_lines.add(line_number)
            continue
        if stripped == "// coverage:ignore-end":
            ignored_lines.add(line_number)
            ignore = False
            continue
        if ignore:
            ignored_lines.add(line_number)
    return ignored_lines

def path_matches(pattern: str, path: Path) -> bool:
    return path.as_posix() == pattern or path.match(pattern)

def path_in_scope(path: Path, include_patterns: list[str], exclude_patterns: list[str]) -> bool:
    if not include_patterns:
        return False
    included = any(path_matches(pattern, path) for pattern in include_patterns)
    excluded = any(path_matches(pattern, path) for pattern in exclude_patterns)
    return included and not excluded

def lcov_counts() -> dict[Path, dict[int, int]]:
    counts: dict[Path, dict[int, int]] = collections.defaultdict(dict)
    current_path = None
    for raw_line in lcov_path.read_text(encoding="utf-8").splitlines():
        if raw_line.startswith("SF:"):
            current_path = normalize_repo_path(raw_line[3:].strip())
            continue
        if raw_line == "end_of_record":
            current_path = None
            continue
        if current_path is None or not raw_line.startswith("DA:"):
            continue
        payload = raw_line[3:]
        line_number_text, count_text = payload.split(",", 1)
        counts[current_path][int(line_number_text)] = int(count_text)
    return counts

def load_goal_plan() -> dict | None:
    if not goal_plan_path:
        return None

    try:
        import yaml
    except ImportError as error:
        raise RuntimeError("PyYAML is required to read a Goal Plan artifact") from error

    goal_plan = yaml.safe_load(Path(goal_plan_path).read_text(encoding="utf-8"))
    if not isinstance(goal_plan, dict):
        raise RuntimeError(f"goal plan `{goal_plan_path}` must parse to a mapping")
    return goal_plan

def normalize_scope_patterns(raw_patterns: list) -> list[str]:
    patterns: list[str] = []
    for entry in raw_patterns:
        if isinstance(entry, str):
            patterns.append(entry)
            continue
        if isinstance(entry, dict):
            file = entry.get("file")
            if isinstance(file, str) and file.strip():
                patterns.append(file)
                continue
            raise RuntimeError(
                "goal plan implementation scope entries must declare a file path"
            )
        patterns.append(str(entry))
    return patterns

def render_text_report(
    *,
    goal_id: str | None,
    goal_title: str | None,
    plan_steps: list[str],
    uncovered: list[tuple[Path, list[int]]],
    outside_scope: list[tuple[Path, list[int]]],
) -> str:
    lines: list[str] = []
    if goal_id:
        lines.append(f"goal-scoped coverage failed: {goal_id}")
    else:
        lines.append("changed-line coverage failed:")
    if goal_title:
        lines.append(f"Goal: {goal_title}")
    if plan_steps:
        lines.append("")
        lines.append("Plan steps:")
        for index, step in enumerate(plan_steps, start=1):
            lines.append(f"  {index}. {step}")
    if uncovered:
        lines.append("")
        lines.append("Missing changed-line coverage:")
        for path, line_numbers in uncovered:
            lines.append(f"  {path.as_posix()}")
            for line_number in line_numbers:
                lines.append(f"    {line_number}")
    if outside_scope:
        lines.append("")
        lines.append("Changed production files outside goal scope:")
        for path, line_numbers in outside_scope:
            lines.append(f"  {path.as_posix()}")
            for line_number in line_numbers:
                lines.append(f"    {line_number}")
    if uncovered or outside_scope:
        lines.append("")
        lines.append(
            "Suggested action: narrow the Goal Plan scope, update the plan, or add tests that execute the missing lines."
        )
    return "\n".join(lines)

def render_json_report(
    *,
    goal_id: str | None,
    goal_title: str | None,
    plan_steps: list[str],
    uncovered: list[tuple[Path, list[int]]],
    outside_scope: list[tuple[Path, list[int]]],
) -> str:
    return json.dumps(
        {
            "status": "failed" if uncovered or outside_scope else "passed",
            "mode": "goal_scoped" if goal_id else "changed_lines",
            "goal": {"id": goal_id, "title": goal_title} if goal_id or goal_title else None,
            "plan_steps": plan_steps,
            "missing_changed_line_coverage": [
                {"file": path.as_posix(), "lines": line_numbers}
                for path, line_numbers in uncovered
            ],
            "changed_files_outside_goal_scope": [
                {"file": path.as_posix(), "lines": line_numbers}
                for path, line_numbers in outside_scope
            ],
        },
        indent=2,
        sort_keys=True,
    )

ensure_base_ref()
diff_lines = changed_lines()
coverage = lcov_counts()
goal_plan = load_goal_plan()
goal = goal_plan.get("goal", {}) if goal_plan else {}
implementation_plan = goal_plan.get("implementation_plan", {}) if goal_plan else {}
scope = implementation_plan.get("scope", {}) if isinstance(implementation_plan, dict) else {}
raw_include_patterns = list(scope.get("include", [])) if isinstance(scope, dict) else []
include_patterns = normalize_scope_patterns(raw_include_patterns)
exclude_patterns = list(scope.get("exclude", [])) if isinstance(scope, dict) else []
plan_steps = []
if goal_plan:
    if not include_patterns:
        raise RuntimeError("goal plan coverage requires implementation_plan.scope.include")
    if isinstance(implementation_plan.get("steps"), list):
        plan_steps = [str(step) for step in implementation_plan["steps"] if str(step).strip()]

goal_id = goal.get("id") if isinstance(goal, dict) else None
goal_title = goal.get("title") if isinstance(goal, dict) else None

misses = []
outside_scope = []
for path, line_numbers in diff_lines.items():
    if is_test_path(path) or is_generated_path(path):
        continue
    if path.as_posix() == "src/command/task.rs":
        ignored_lines = ignored_task_coverage_lines(path)
        line_numbers = [line for line in line_numbers if line not in ignored_lines]
        if not line_numbers:
            continue
    if goal_plan and not path_in_scope(path, include_patterns, exclude_patterns):
        outside_scope.append((path, sorted(line_numbers)))
        continue
    covered_lines = coverage.get(path, {})
    missing_line_numbers = []
    for line_number in sorted(line_numbers):
        if covered_lines.get(line_number, 0) <= 0:
            missing_line_numbers.append(line_number)
    if missing_line_numbers:
        misses.append((path, missing_line_numbers))

failed = bool(misses or outside_scope)
if output_format == "json":
    print(
        render_json_report(
            goal_id=goal_id,
            goal_title=goal_title,
            plan_steps=plan_steps,
            uncovered=misses,
            outside_scope=outside_scope,
        )
    )
else:
    if failed:
        print(
            render_text_report(
                goal_id=goal_id,
                goal_title=goal_title,
                plan_steps=plan_steps,
                uncovered=misses,
                outside_scope=outside_scope,
            )
        )
    else:
        if goal_id:
            print(f"goal-scoped coverage passed: {goal_id}")
        else:
            print("changed-line coverage passed")

if failed:
    sys.exit(1)
PY
}

run_coverage() {
  local mode="summary"
  local goal_plan_path=""
  local output_format="text"
  local repo_root

  if [[ $# -gt 0 ]]; then
    mode="$1"
    shift
  fi

  while [[ $# -gt 0 ]]; do
    case "$1" in
      --goal)
        goal_plan_path="${2:-}"
        if [[ -z "$goal_plan_path" ]]; then
          echo "usage: scripts/ci/coverage.sh pr [--goal <goal-plan.yaml>] [--format text|json]" >&2
          exit 1
        fi
        shift 2
        ;;
      --format)
        output_format="${2:-}"
        if [[ -z "$output_format" ]]; then
          echo "usage: scripts/ci/coverage.sh pr [--goal <goal-plan.yaml>] [--format text|json]" >&2
          exit 1
        fi
        shift 2
        ;;
      *)
        echo "usage: scripts/ci/coverage.sh [pr|summary|lcov|html]" >&2
        exit 1
        ;;
    esac
  done

  if [[ "$mode" != "pr" && -n "$goal_plan_path" ]]; then
    echo "usage: scripts/ci/coverage.sh pr [--goal <goal-plan.yaml>] [--format text|json]" >&2
    exit 1
  fi

  if [[ "$output_format" != "text" && "$output_format" != "json" ]]; then
    echo "usage: scripts/ci/coverage.sh pr [--goal <goal-plan.yaml>] [--format text|json]" >&2
    exit 1
  fi

  repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
  cd "$repo_root"

  ensure_cargo_bin_in_path

  if ! command -v cargo-llvm-cov >/dev/null 2>&1; then
    echo "cargo-llvm-cov is required. Install it with: cargo install cargo-llvm-cov --locked" >&2
    exit 1
  fi

  configure_llvm_tools

  case "$mode" in
    pr)
      generate_lcov target/coverage/lcov.info
      if [[ "$output_format" == "json" ]]; then
        generate_spec_coverage_summary target/coverage/lcov.info target/coverage/spec-coverage-summary.md >/dev/null
      else
        generate_spec_coverage_summary target/coverage/lcov.info target/coverage/spec-coverage-summary.md
      fi
      enforce_diff_coverage target/coverage/lcov.info origin/main "$goal_plan_path" "$output_format"
      ;;
    summary)
      generate_lcov target/coverage/lcov.info
      generate_spec_coverage_summary target/coverage/lcov.info target/coverage/spec-coverage-summary.md
      report_lcov_coverage target/coverage/lcov.info
      ;;
    lcov)
      generate_lcov target/coverage/lcov.info
      generate_spec_coverage_summary target/coverage/lcov.info target/coverage/spec-coverage-summary.md
      report_lcov_coverage target/coverage/lcov.info
      ;;
    html)
      generate_lcov target/coverage/lcov.info
      generate_spec_coverage_summary target/coverage/lcov.info target/coverage/spec-coverage-summary.md
      report_lcov_coverage target/coverage/lcov.info
      cargo llvm-cov --no-run --html
      ;;
    *)
      echo "usage: scripts/ci/coverage.sh [pr|summary|lcov|html]" >&2
      exit 1
      ;;
  esac
}

run_coverage "$@"
