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
  cargo llvm-cov --lcov --output-path "$output_path"
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

  python3 - "$lcov_path" "$base_ref" <<'PY'
import collections
import subprocess
import sys
from pathlib import Path

lcov_path = Path(sys.argv[1])
base_ref = sys.argv[2]

repo_root = Path(
    subprocess.check_output(
        ["git", "rev-parse", "--show-toplevel"], text=True
    ).strip()
)

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
            current_path = (repo_root / raw_line[6:]).resolve()
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
    return "tests" in path.parts

def lcov_counts() -> dict[Path, dict[int, int]]:
    counts: dict[Path, dict[int, int]] = collections.defaultdict(dict)
    current_path = None
    for raw_line in lcov_path.read_text(encoding="utf-8").splitlines():
        if raw_line.startswith("SF:"):
            current_path = Path(raw_line[3:].strip()).resolve()
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

ensure_base_ref()
diff_lines = changed_lines()
coverage = lcov_counts()

misses = []
for path, line_numbers in diff_lines.items():
    if is_test_path(path):
        continue
    covered_lines = coverage.get(path, {})
    for line_number in sorted(line_numbers):
        if line_number not in covered_lines:
            continue
        if covered_lines[line_number] <= 0:
            misses.append((path, line_number))

if misses:
    print("changed lines without coverage:")
    for path, line_number in misses:
        print(f"- {path.relative_to(repo_root)}:{line_number}")
else:
    print("coverage measure script")
PY
}

run_coverage() {
  local mode="${1:-summary}"
  local repo_root

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
      generate_spec_coverage_summary target/coverage/lcov.info target/coverage/spec-coverage-summary.md
      enforce_diff_coverage target/coverage/lcov.info origin/main
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
