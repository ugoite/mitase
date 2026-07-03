#!/usr/bin/env bash
# FEAT-QUALITY-001

set -euo pipefail

write_sha256() {
  local path="$1"

  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$path"
    return 0
  fi

  if command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$path"
    return 0
  fi

  echo "sha256sum or shasum is required to snapshot docs/generated" >&2
  exit 1
}

snapshot_generated_docs() {
  if [[ ! -d docs/generated ]]; then
    printf '__missing__\n'
    return 0
  fi

  while IFS= read -r path; do
    write_sha256 "$path"
  done < <(find docs/generated -type f | LC_ALL=C sort)
}

check_generated_docs_freshness() {
  local repo_root
  local before_snapshot
  local after_snapshot

  repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
  before_snapshot="$(mktemp)"
  after_snapshot="$(mktemp)"
  trap 'rm -f "${before_snapshot:-}" "${after_snapshot:-}"' EXIT

  cd "$repo_root"
  snapshot_generated_docs >"$before_snapshot"
  python3 scripts/generate-site-docs.py
  cargo run --quiet -- report . --output docs/generated/syu-report.md >/dev/null
  snapshot_generated_docs >"$after_snapshot"

  if ! cmp -s "$before_snapshot" "$after_snapshot"; then
    echo "Checked-in generated documentation under docs/generated is stale." >&2
    echo "Commit the regenerated docs/generated output after reviewing the diff." >&2
    git --no-pager status --short -- docs/generated >&2
    git --no-pager diff --stat -- docs/generated >&2
    git --no-pager diff -- docs/generated >&2
    exit 1
  fi
}

check_generated_docs_freshness "$@"
