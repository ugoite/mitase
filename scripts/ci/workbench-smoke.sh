#!/usr/bin/env bash
# FEAT-QUALITY-001

set -euo pipefail

run_workbench_smoke() {
  local repo_root

  repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
  cd "$repo_root"

  cargo test --test workbench_smoke
  cargo test -p syu-workbench-server workbench
}

run_workbench_smoke "$@"
