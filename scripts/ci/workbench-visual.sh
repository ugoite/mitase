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
    """  if(String(url).includes('/api/config')) return body({config:JSON.parse(document.querySelector('#syu-projection').textContent).config,hash:'visual-test-hash'});\n"""
    """  if(String(url).includes('/api/validate')) return body(JSON.parse(document.querySelector('#syu-projection').textContent).validation);\n"""
    """  if(String(url).includes('/api/context/')) return body('schema: syu/context/v1\\n');\n"""
    """  return body({error:`unhandled fetch ${url}`},404);\n"""
    """};\n"""
)
PY

state_script="$(cat "$tmp/state.html")<script src=\"file://${tmp}/api-mock.js\"></script><script src=\"file://${repo_root}/crates/syu-app-ui/assets/projection.js\"></script>"

sed \
  -e "s|/assets/workbench.css|file://${repo_root}/crates/syu-app-ui/assets/workbench.css|" \
  -e "s|/assets/catalog.js|file://${tmp}/catalog.js|" \
  -e "s|/assets/i18n.js|file://${repo_root}/crates/syu-app-ui/assets/i18n.js|" \
  -e "s|/assets/app.js|file://${repo_root}/crates/syu-app-ui/assets/app.js|" \
  -e "s|<script src=\"/assets/projection.js\"></script>|${state_script}|" \
  crates/syu-app-ui/assets/workbench.html >"$tmp/workbench.html"

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

  click('[data-route="items"]');
  click('[data-page="items"] [data-tab="policy"]');
  if(!document.querySelector('[data-items-detail]')) failures.push('items detail missing');

  if(window.__SYU_VISUAL_ERRORS__.length) failures.push(`js errors: ${window.__SYU_VISUAL_ERRORS__.join(', ')}`);
  document.body.append(document.createTextNode(failures.length ? `__SYU_TEST_FAIL__ ${failures.join('; ')}` : '__SYU_TEST_PASS__'));
}, 800);
</script>
HTML

render() {
  local fixture="$1" width="$2" height="$3" query="$4"
  local encoded="crates/syu-app-ui/tests/baselines/${fixture}.png.b64"
  base64 --decode <"$encoded" >"$tmp/reference.png" 2>/dev/null || base64 -D <"$encoded" >"$tmp/reference.png"
  "$chrome" --headless --disable-gpu --hide-scrollbars --no-sandbox --allow-file-access-from-files \
    --force-device-scale-factor=1 --virtual-time-budget=1200 --window-size="${width},${height}" \
    --screenshot="$tmp/actual.png" "file://${tmp}/workbench.html${query}" >/dev/null 2>&1
  python3 scripts/ci/png_visual_diff.py "$tmp/reference.png" "$tmp/actual.png"
}

render 01-work-overview-en-light 1440 900 '?page=work&workTab=overview&lang=en&theme=light'
render 02-work-slices-en-light 1440 900 '?page=work&workTab=slices&lang=en&theme=light'
render 03-settings-language-en-light 1440 900 '?page=settings&settingsLayer=application&settingsPage=language&lang=en&theme=light'
render 04-settings-validation-en-light 1440 900 '?page=settings&settingsLayer=workspace&settingsPage=validation&lang=en&theme=light'
render 05-diagnostics-zero-en-light 1440 900 '?page=diagnostics&lang=en&theme=light'
render 06-settings-language-ja-light 1440 900 '?page=settings&settingsLayer=application&settingsPage=language&lang=ja&theme=light'
render 07-settings-appearance-ja-dark 1440 900 '?page=settings&settingsLayer=application&settingsPage=appearance&lang=ja&theme=dark'
render 08-diagnostics-zero-ja-dark 1440 900 '?page=diagnostics&lang=ja&theme=dark'
render 09-mobile-settings-ja-light 390 844 '?page=settings&settingsLayer=application&settingsPage=language&lang=ja&theme=light'
render 10-work-context-en-light 1440 900 '?page=work&workTab=context&lang=en&theme=light'
render 11-scope-en-light 1440 900 '?page=scope&scopeTab=change&lang=en&theme=light'
render 12-items-en-light 1440 900 '?page=items&itemsTab=requirement&lang=en&theme=light'
render 13-command-palette-en-light 1440 900 '?page=work&palette=1&lang=en&theme=light'
render 14-work-overview-ja-light 1440 900 '?page=work&workTab=overview&lang=ja&theme=light'
render 15-scope-ja-light 1440 900 '?page=scope&scopeTab=change&lang=ja&theme=light'
render 16-items-ja-light 1440 900 '?page=items&itemsTab=requirement&lang=ja&theme=light'

behavior="$("$chrome" --headless --disable-gpu --no-sandbox --allow-file-access-from-files --virtual-time-budget=1800 --dump-dom "file://${tmp}/workbench.html?page=work&lang=en&theme=light")"
echo "$behavior" | grep -q '__SYU_TEST_PASS__'
if echo "$behavior" | grep -q '__SYU_TEST_FAIL__'; then
  echo "$behavior" | grep '__SYU_TEST_FAIL__' >&2
  exit 1
fi
