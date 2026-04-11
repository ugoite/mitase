#!/usr/bin/env bash
# FEAT-QUALITY-001

set -euo pipefail

ensure_app_dependencies() {
  npm ci
}

snapshot_dist() {
  if [[ ! -d dist ]]; then
    printf '__missing__\n'
    return 0
  fi

  while IFS= read -r path; do
    sha256sum "$path"
  done < <(find dist -type f | LC_ALL=C sort)
}

check_app_dist_freshness() {
  local repo_root
  local app_dir
  local before_snapshot
  local after_snapshot

  repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
  app_dir="${repo_root}/app"
  before_snapshot="$(mktemp)"
  after_snapshot="$(mktemp)"
  trap 'rm -f "${before_snapshot:-}" "${after_snapshot:-}"' EXIT

  cd "$app_dir"
  snapshot_dist >"$before_snapshot"
  ensure_app_dependencies
  npm run build:wasm
  npm run check
  npm run build
  snapshot_dist >"$after_snapshot"

  cd "$repo_root"
  if ! cmp -s "$before_snapshot" "$after_snapshot"; then
    echo "Checked-in browser bundle under app/dist is stale." >&2
    echo "Commit the regenerated app/dist output after reviewing the build diff." >&2
    git --no-pager status --short -- app/dist >&2
    git --no-pager diff --stat -- app/dist >&2
    exit 1
  fi
}

check_app_dist_freshness "$@"
