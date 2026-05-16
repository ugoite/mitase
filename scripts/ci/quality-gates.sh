#!/usr/bin/env bash
# FEAT-QUALITY-001

set -euo pipefail

run_quality_gates() {
  local mode="${1:-full}"
  local repo_root

  repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
  cd "$repo_root"

  cargo fmt --all --check
  cargo clippy --all-targets --all-features -- -D warnings
  cargo run -- validate .

  case "$mode" in
    fast)
      cargo test --lib --bins
      ;;
    full)
      cargo test
      bash scripts/ci/check-generated-docs-freshness.sh

      mkdir -p target/quality
      cargo run -- report . --output target/quality/syu-report.md >/dev/null
      ;;
    *)
      echo "usage: scripts/ci/quality-gates.sh [fast|full]" >&2
      exit 1
      ;;
  esac
}

run_quality_gates "$@"
