#!/usr/bin/env bash
# FEAT-CONTRIB-001

set -euo pipefail

repo_root() {
  cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd
}

log_step() {
  printf "\n[devcontainer] %s\n" "$1"
}

main() {
  local root
  root="$(repo_root)"

  cd "$root"

  log_step "Installing browser-app dependencies for local app builds, scripts/ci/check-browser-app-freshness.sh, and npm --prefix app run test:e2e."
  npm --prefix app ci

  log_step "Installing Playwright Chromium for npm --prefix app run test:e2e."
  npx --prefix app playwright install --with-deps chromium

  log_step "Browser tooling setup complete."
}

main "$@"
