#!/usr/bin/env bash
# FEAT-QUALITY-001

set -euo pipefail

check_pr_coverage() {
  local repo_root

  repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
  cd "$repo_root"

  export SYU_SKIP_BROWSER_APP_BUILD=1

  mkdir -p target/syu
  cargo run --quiet -- task infer --range origin/main...HEAD --output target/syu/goal.yaml
  cargo run --quiet -- task test-select target/syu/goal.yaml --format json >target/syu/selected-tests.json
  scripts/ci/coverage.sh pr --goal target/syu/goal.yaml
}

check_pr_coverage
