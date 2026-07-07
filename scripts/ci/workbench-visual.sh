#!/usr/bin/env bash
# REQ-WORKBENCH-001
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$repo_root"
python3 scripts/ci/check-workbench-contract.py

chrome="${CHROME_BIN:-}"
if [[ -z "$chrome" ]]; then
  for candidate in google-chrome google-chrome-stable chromium chromium-browser; do
    if command -v "$candidate" >/dev/null 2>&1; then
      chrome="$(command -v "$candidate")"
      break
    fi
  done
fi
if [[ -z "$chrome" ]]; then
  echo "Chrome/Chromium is required for the Workbench visual gate" >&2
  exit 1
fi

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

{
  printf 'window.SYU_I18N={en:'
  cat crates/syu-app-ui/assets/locales/en.json
  printf ',ja:'
  cat crates/syu-app-ui/assets/locales/ja.json
  printf '};\n'
} >"$tmp/catalog.js"
: >"$tmp/projection.js"

sed \
  -e "s|/assets/workbench.css|file://${repo_root}/crates/syu-app-ui/assets/workbench.css|" \
  -e "s|/assets/catalog.js|file://${tmp}/catalog.js|" \
  -e "s|/assets/i18n.js|file://${repo_root}/crates/syu-app-ui/assets/i18n.js|" \
  -e "s|/assets/app.js|file://${repo_root}/crates/syu-app-ui/assets/app.js|" \
  -e "s|/assets/projection.js|file://${tmp}/projection.js|" \
  crates/syu-app-ui/assets/workbench.html >"$tmp/workbench.html"

render() {
  local fixture="$1" width="$2" height="$3" query="$4"
  local encoded="crates/syu-app-ui/tests/baselines/${fixture}.png.b64"
  base64 --decode <"$encoded" >"$tmp/reference.png" 2>/dev/null || base64 -D <"$encoded" >"$tmp/reference.png"
  "$chrome" --headless --disable-gpu --hide-scrollbars --no-sandbox --allow-file-access-from-files \
    --force-device-scale-factor=1 --virtual-time-budget=1200 --window-size="${width},${height}" \
    --screenshot="$tmp/actual.png" "file://${tmp}/workbench.html${query}" >/dev/null 2>&1
  python3 scripts/ci/png_visual_diff.py "$tmp/reference.png" "$tmp/actual.png"
}

render 01-work-overview-en-light 1440 900 '?page=work&tab=overview&lang=en&theme=light'
render 02-work-slices-en-light 1440 900 '?page=work&tab=slices&lang=en&theme=light'
render 03-settings-language-en-light 1440 900 '?page=settings&settingsLayer=application&settingsPage=language&lang=en&theme=light'
render 04-settings-validation-en-light 1440 900 '?page=settings&settingsLayer=workspace&settingsPage=validation&lang=en&theme=light'
render 05-diagnostics-zero-en-light 1440 900 '?page=diagnostics&lang=en&theme=light'
render 06-settings-language-ja-light 1440 900 '?page=settings&settingsLayer=application&settingsPage=language&lang=ja&theme=light'
render 07-settings-appearance-ja-dark 1440 900 '?page=settings&settingsLayer=application&settingsPage=appearance&lang=ja&theme=dark'
render 08-diagnostics-zero-ja-dark 1440 900 '?page=diagnostics&lang=ja&theme=dark'
render 09-mobile-settings-ja-light 390 844 '?page=settings&settingsLayer=application&settingsPage=language&lang=ja&theme=light'
render 10-work-context-en-light 1440 900 '?page=work&tab=context&lang=en&theme=light'
render 11-scope-en-light 1440 900 '?page=scope&lang=en&theme=light'
render 12-items-en-light 1440 900 '?page=items&lang=en&theme=light'
render 13-command-palette-en-light 1440 900 '?page=work&palette=1&lang=en&theme=light'
render 14-work-overview-ja-light 1440 900 '?page=work&tab=overview&lang=ja&theme=light'
render 15-scope-ja-light 1440 900 '?page=scope&lang=ja&theme=light'
render 16-items-ja-light 1440 900 '?page=items&lang=ja&theme=light'
