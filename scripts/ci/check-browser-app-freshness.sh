#!/usr/bin/env bash
# FEAT-QUALITY-001

set -euo pipefail

ensure_app_dependencies() {
  "${repo_root}/scripts/ci/pinned-npm.sh" check app
  npm ci
}

clear_generated_browser_outputs() {
  rm -rf src/wasm dist
}

check_browser_app_freshness() {
  local repo_root
  local app_dir

  repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
  app_dir="${repo_root}/app"

  cd "$app_dir"
  ensure_app_dependencies
  clear_generated_browser_outputs
  npm run build:wasm

  if [[ ! -f src/wasm/syu_app_wasm.js || ! -f src/wasm/syu_app_wasm_bg.wasm ]]; then
    echo "Browser app Wasm bridge was not regenerated under app/src/wasm." >&2
    exit 1
  fi

  npm run check
  npm run build

  if [[ ! -f dist/index.html ]]; then
    echo "Browser app build did not produce app/dist/index.html." >&2
    exit 1
  fi
}

check_browser_app_freshness "$@"
