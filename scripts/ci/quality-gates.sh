#!/usr/bin/env bash
# FEAT-QUALITY-001

set -euo pipefail

run_quality_gates() {
  local repo_root
  repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

  cd "$repo_root"

  cargo fmt --all --check
  cargo clippy --all-targets --all-features -- -D warnings
  cargo test
  cargo run -- validate .
  bash scripts/ci/check-generated-docs-freshness.sh

  mkdir -p target/quality
  cargo run -- report . --output target/quality/syu-report.md >/dev/null
}

run_quality_gates "$@"
