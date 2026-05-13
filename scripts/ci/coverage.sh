#!/usr/bin/env bash
# FEAT-QUALITY-001

set -euo pipefail

LINE_THRESHOLD=100

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

enforce_lcov_threshold() {
  local lcov_path="$1"

  # Keep the repository contract equivalent to cargo llvm-cov --fail-under-lines 100,
  # but evaluate it from LCOV's executed-line data so the gate reflects true line coverage.
  python3 - "$lcov_path" "$LINE_THRESHOLD" <<'PY'
import sys

lcov_path = sys.argv[1]
threshold = float(sys.argv[2])
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
if coverage + 1e-9 < threshold:
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
    raise SystemExit(1)
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
    summary)
      generate_lcov target/coverage/lcov.info
      generate_spec_coverage_summary target/coverage/lcov.info target/coverage/spec-coverage-summary.md
      enforce_lcov_threshold target/coverage/lcov.info
      ;;
    lcov)
      generate_lcov target/coverage/lcov.info
      generate_spec_coverage_summary target/coverage/lcov.info target/coverage/spec-coverage-summary.md
      enforce_lcov_threshold target/coverage/lcov.info
      ;;
    html)
      generate_lcov target/coverage/lcov.info
      generate_spec_coverage_summary target/coverage/lcov.info target/coverage/spec-coverage-summary.md
      enforce_lcov_threshold target/coverage/lcov.info
      cargo llvm-cov --no-run --html
      ;;
    *)
      echo "usage: scripts/ci/coverage.sh [summary|lcov|html]" >&2
      exit 1
      ;;
  esac
}

run_coverage "$@"
