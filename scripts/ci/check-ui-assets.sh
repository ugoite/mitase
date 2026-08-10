#!/usr/bin/env bash
# FEAT-QUALITY-001

set -euo pipefail

check_ui_assets() {
  local repo_root ui_root source_css built_css

  repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
  ui_root="${repo_root}/crates/mitase-app-ui"
  source_css="${ui_root}/tailwind.css"
  built_css="${ui_root}/assets/tailwind.css"

  test -f "$source_css"
  test -f "$built_css"

  grep -F '@import "tailwindcss";' "$source_css" >/dev/null
  grep -F '@source "./src/**/*.{rs,html,css}";' "$source_css" >/dev/null
  grep -F "@layer theme" "$built_css" >/dev/null
  grep -F -- "--color-command-active" "$built_css" >/dev/null
  grep -F -- "--color-evidence-pending" "$built_css" >/dev/null
  python3 "$repo_root/scripts/ci/check-workbench-contract.py"

  if find "$repo_root" \
    -path "$repo_root/target" -prune -o \
    -path "$repo_root/.git" -prune -o \
    -path "$repo_root/website" -prune -o \
    -path "$repo_root/editors/vscode" -prune -o \
    -path "$repo_root/examples/browser-ui" -prune -o \
    -path "$repo_root/examples/typescript-only" -prune -o \
    -type f \( \
      -name "vite.config.*" -o \
      -name "playwright.config.*" -o \
      -name "package.json" -o \
      -name "package-lock.json" \
    \) -print -quit | grep -q .; then
    echo "unexpected frontend package or browser-app build config outside allowed docs/editor/example surfaces" >&2
    exit 1
  fi

  if [[ -n "${TAILWINDCSS_BIN:-}" ]]; then
    "$TAILWINDCSS_BIN" -i "$source_css" -o "${built_css}.check"
    test -s "${built_css}.check"
    grep -F "@layer theme" "${built_css}.check" >/dev/null
    rm -f "${built_css}.check"
  else
    echo "Tailwind CLI not configured; validated checked-in mitase-app-ui Tailwind source and asset."
  fi
}

check_ui_assets "$@"
