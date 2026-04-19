#!/usr/bin/env bash
# FEAT-QUALITY-001

set -euo pipefail

run_optional_e2e() {
  npx --prefix app playwright install --with-deps chromium
  npm --prefix app run test:e2e
}

validate_app() {
  local repo_root
  local run_e2e=0

  repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
  cd "$repo_root"

  if [[ "${1:-}" == "--e2e" ]]; then
    run_e2e=1
    shift
  fi

  if [[ $# -ne 0 ]]; then
    echo "usage: scripts/ci/validate-app.sh [--e2e]" >&2
    exit 1
  fi

  bash scripts/ci/quality-gates.sh
  npm --prefix app ci
  bash scripts/ci/check-browser-app-freshness.sh

  if [[ "$run_e2e" -eq 1 ]]; then
    run_optional_e2e
  fi
}

validate_app "$@"
