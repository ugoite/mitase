#!/usr/bin/env bash
# FEAT-QUALITY-001

set -euo pipefail

validate_website() {
  local repo_root

  repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
  cd "$repo_root"

  bash scripts/ci/quality-gates.sh
  bash scripts/ci/install-docs-site-deps.sh
  npm --prefix website run build
}

validate_website "$@"
