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
  echo "Chrome/Chromium is required for the Workbench browser smoke test" >&2
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

cargo run --quiet -- workbench project \
  --workspace fixtures/v1/valid-web-app \
  --format json >"$tmp/projection.json"

python3 - <<'PY' "$tmp/projection.json" "$tmp/state.html" "$tmp/api-mock.js"
import pathlib, sys

projection = pathlib.Path(sys.argv[1]).read_text().replace('<', '\\u003c')
pathlib.Path(sys.argv[2]).write_text(
    f'<script type="application/json" id="syu-projection">{projection}</script>'
)
pathlib.Path(sys.argv[3]).write_text(
    """window.fetch=async(url,options={})=>{\n"""
    """  const body=(value,status=200)=>Promise.resolve({ok:status>=200&&status<300,status,text:async()=>typeof value==='string'?value:JSON.stringify(value)});\n"""
    """  if(String(url).includes('/api/source?path=syu.yaml')) return body({content:'schema: syu/config/v1\\nworkspace:\\n  spec_roots: [docs/syu]\\n',hash:'visual-test-hash'});\n"""
    """  if(String(url).includes('/api/config')) return body({});\n"""
    """  if(String(url).includes('/api/validate')) return body(JSON.parse(document.querySelector('#syu-projection').textContent).validation);\n"""
    """  if(String(url).includes('/api/context/')) return body('schema: syu/context/v1\\n');\n"""
    """  return body({error:`unhandled fetch ${url}`},404);\n"""
    """};\n"""
)
PY

python3 - <<'PY' "$repo_root" "$tmp"
import pathlib
import sys

repo_root = pathlib.Path(sys.argv[1])
tmp = pathlib.Path(sys.argv[2])

html = (repo_root / "crates/syu-app-ui/assets/workbench.html").read_text()

html = html.replace(
    "/assets/workbench.css",
    f"file://{repo_root}/crates/syu-app-ui/assets/workbench.css",
)
html = html.replace(
    "/assets/catalog.js",
    f"file://{tmp}/catalog.js",
)
html = html.replace(
    "/assets/i18n.js",
    f"file://{repo_root}/crates/syu-app-ui/assets/i18n.js",
)
html = html.replace(
    "/assets/js/main.js",
    f"file://{repo_root}/crates/syu-app-ui/assets/js/main.js",
)
html = html.replace(
    '<script type="application/json" id="syu-projection"></script>',
    (tmp / "state.html").read_text()
    + f'<script src="file://{tmp}/api-mock.js"></script>',
)

(tmp / "workbench.html").write_text(html)
PY

cat >>"$tmp/workbench.html" <<'HTML'
<script>
window.__SYU_VISUAL_ERRORS__=[];
window.addEventListener('error',event=>window.__SYU_VISUAL_ERRORS__.push(event.message||'error'));
window.addEventListener('unhandledrejection',event=>window.__SYU_VISUAL_ERRORS__.push(String(event.reason||'rejection')));
setTimeout(()=>{
  const failures=[];
  const click=s=>document.querySelector(s)?.click();
  const visible=s=>{const node=document.querySelector(s);return !!node&&!node.hidden;};

  click('[data-page="work"] [data-tab="slices"]');
  if(!visible('[data-page="work"] [data-panel="slices"]')) failures.push('work slices panel did not open');
  click('[data-page="work"] [data-tab="context"]');
  if(!visible('[data-page="work"] [data-panel="context"]')) failures.push('work context panel did not open');

  click('[data-route="settings"]');
  click('[data-settings-layer="workspace"]');
  if(!visible('[data-settings-layer-panel="workspace"]')) failures.push('workspace settings layer hidden');
  if(visible('[data-settings-layer-panel="application"]')) failures.push('application settings layer still visible');
  click('[data-settings-page="validation"]');
  if(!visible('[data-settings-page-panel="validation"]')) failures.push('settings validation page did not open');
  if(visible('[data-settings-page-panel="general"]')) failures.push('settings general page still visible');

  click('[data-route="specifications"]');
  click('[data-page="specifications"] [data-tab="policy"]');
  if(!document.querySelector('[data-specifications-detail]')) failures.push('specifications detail missing');

  if(window.__SYU_VISUAL_ERRORS__.length) failures.push(`js errors: ${window.__SYU_VISUAL_ERRORS__.join(', ')}`);
  const result = document.createElement('div');
  result.id = 'syu-visual-behavior-result';
  result.dataset.status = failures.length ? 'fail' : 'pass';
  result.textContent = failures.join('; ');
  document.body.append(result);
}, 800);
</script>
HTML

behavior="$("$chrome" --headless --disable-gpu --no-sandbox --allow-file-access-from-files --virtual-time-budget=1800 --dump-dom "file://${tmp}/workbench.html?page=work&lang=en&theme=light")"
echo "$behavior" | grep -q 'id="syu-visual-behavior-result" data-status="pass"'
if echo "$behavior" | grep -q 'id="syu-visual-behavior-result" data-status="fail"'; then
  echo "$behavior" | grep 'id="syu-visual-behavior-result"' >&2
  exit 1
fi
