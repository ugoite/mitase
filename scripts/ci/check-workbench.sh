#!/usr/bin/env bash
# FEAT-QUALITY-001

set -euo pipefail

run_workbench_checks() {
  local repo_root

  repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
  cd "$repo_root"

  cargo test -p syu-code-intel
  cargo test -p syu-workbench-server
  cargo test -p syu-app-ui

  bash scripts/ci/check-ui-assets.sh
  bash scripts/ci/workbench-smoke.sh
}

run_workbench_checks "$@"
