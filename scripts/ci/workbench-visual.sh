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
cleanup_tmp() {
  local attempt
  for ((attempt = 0; attempt < 5; attempt++)); do
    rm -rf "$tmp" 2>/dev/null || true
    [[ ! -e "$tmp" ]] && return 0
    sleep 0.2
  done
}
trap cleanup_tmp EXIT

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
    """let projection=JSON.parse(document.querySelector('#syu-projection').textContent);\n"""
    """let receipt=null;\n"""
    """const csrfToken='visual-csrf-token';\n"""
    """window.__SYU_FLOW__=[];\n"""
    """const body=(value,status=200,headers={})=>Promise.resolve({ok:status>=200&&status<300,status,headers:{get:name=>headers[name.toLowerCase()]||null},text:async()=>value==null?'':typeof value==='string'?value:JSON.stringify(value)});\n"""
    """window.fetch=async(url,options={})=>{\n"""
    """  const path=String(url);\n"""
    """  const payload=options.body?JSON.parse(options.body):{};\n"""
    """  const method=(options.method||'GET').toUpperCase();\n"""
    """  const suppliedCsrf=Object.entries(options.headers||{}).find(([key])=>key.toLowerCase()==='x-syu-csrf-token')?.[1];\n"""
    """  if(method!=='GET' && suppliedCsrf!==csrfToken) return body({error:'missing csrf token'},403);\n"""
    """  if(path.includes('/api/projection')) return body(projection,200,{'x-syu-csrf-token':csrfToken});\n"""
    """  if(path.includes('/api/work/session')) return body({ready:true},200,{'x-syu-csrf-token':csrfToken});\n"""
    """  if(path.includes('/api/source?path=syu.yaml')) return body({content:'schema: syu/config/v1\\nworkspace:\\n  spec_roots: [docs/syu]\\n',hash:'visual-test-hash'});\n"""
    """  if(path.includes('/api/config')) return body({});\n"""
    """  if(path.includes('/api/work/request')) { window.__SYU_FLOW__.push('request'); projection.work.request={summary:payload.request.summary,operation:'modify',seed_count:1,requested_target_count:0}; projection.work.plan=null; return body(projection); }\n"""
    """  if(path.includes('/api/work/plan')) { window.__SYU_FLOW__.push('plan'); projection.work.plan={id:'PLAN-VISUAL-FLOW',status:'ready',slices:[{id:'slice-visual-flow',editable_targets:[{reference:'FEAT-VISUAL#binding.work/target.code',access:'editable',path:'src/lib.rs'}]}]}; return body(projection.work.plan); }\n"""
    """  if(path.includes('/api/work/context')) { window.__SYU_FLOW__.push('context'); projection.work.selected_slice='slice-visual-flow'; projection.work.context_pack={slice_id:'slice-visual-flow',entry_count:2}; return body({schema:'syu/context-pack/v1',slice:'slice-visual-flow',artifact_context:[]}); }\n"""
    """  if(path.includes('/api/work/validate')) { window.__SYU_FLOW__.push('validate'); projection.work.validation={state:'passed',context:'work-plan'}; return body(projection.work.validation); }\n"""
    """  if(path.includes('/api/work/approve')) { window.__SYU_FLOW__.push('approve'); return body({schema:'syu/plan-approval/v1',approval_id:'approval-visual-flow',plan_digest:'visual-plan'}); }\n"""
    """  if(path.includes('/api/work/verify')) { window.__SYU_FLOW__.push('verify'); receipt={schema:'syu/verification-receipt/v1',plan_digest:'visual-plan',slice_id:'slice-visual-flow',revision:'visual-revision',workspace_fingerprint:'visual-fingerprint',started_at:'1',completed_at:'2',executions:[{target:'REQ-VISUAL#binding.test/target.exact',runner:'mock',command:['mock'],exit_code:0,stdout_digest:'stdout',stderr_digest:'stderr',implementation_digests:{},verification_digest:'verification'}]}; projection.work.verification_receipt={slice_id:'slice-visual-flow'}; projection.work.completion={current:{attempt_id:'attempt-visual-flow',status:'complete',plan_digest:'visual-plan',slice_id:'slice-visual-flow',finalized:false},previous:[]}; return body({receipt}); }\n"""
    """  if(path.includes('/api/work/finalize/preview')) { window.__SYU_FLOW__.push('finalize-preview'); return body({preview_token:'visual-preview-token'}); }\n"""
    """  if(path.includes('/api/work/finalize/apply')) { window.__SYU_FLOW__.push('finalize-apply'); projection.work.completion.current.finalized=true; return body({schema:'syu/finalization-receipt/v1'}); }\n"""
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
  const wait=ms=>new Promise(resolve=>setTimeout(resolve,ms));
  const click=async s=>{const node=document.querySelector(s);if(!node){failures.push(`missing ${s}`);return;}node.click();await wait(80);};
  const visible=s=>{const node=document.querySelector(s);return !!node&&!node.hidden;};

  (async()=>{
  await click('[data-route="specifications"]');
  await click('[data-page="specifications"] .specification-criterion button');
  if(!visible('[data-page="work"]')) failures.push('criterion did not open Work page');
  await click('[data-work-plan]');
  if(!document.querySelector('[data-work-slices-rail] .rail-item')) failures.push('Plan did not create a slice');
  await click('[data-work-context]');
  await click('[data-work-validate]');
  await click('[data-work-approve]');
  await click('[data-work-verify]');
  await click('[data-work-finalize]');
  if(JSON.stringify(window.__SYU_FLOW__)!=='["request","plan","context","validate","approve","verify","finalize-preview","finalize-apply"]') failures.push(`unexpected work flow: ${JSON.stringify(window.__SYU_FLOW__)}`);

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
  })();
}, 800);
</script>
HTML

behavior="$("$chrome" --headless --disable-gpu --no-sandbox --allow-file-access-from-files --virtual-time-budget=1800 --dump-dom "file://${tmp}/workbench.html?page=work&lang=en&theme=light")"
if ! echo "$behavior" | grep -q 'id="syu-visual-behavior-result" data-status="pass"'; then
  echo "$behavior" | grep 'id="syu-visual-behavior-result"' >&2 || true
  exit 1
fi

read -r server_port debug_port < <(python3 - <<'PY'
import socket

ports = []
for _ in range(2):
    sock = socket.socket()
    sock.bind(("127.0.0.1", 0))
    ports.append(str(sock.getsockname()[1]))
    sock.close()
print(*ports)
PY
)

server_pid=""
chrome_pid=""
cleanup_browser() {
  if [[ -n "$chrome_pid" ]]; then kill "$chrome_pid" 2>/dev/null || true; fi
  if [[ -n "$server_pid" ]]; then kill "$server_pid" 2>/dev/null || true; fi
  wait "$chrome_pid" 2>/dev/null || true
  wait "$server_pid" 2>/dev/null || true
  cleanup_tmp
}
trap cleanup_browser EXIT

cargo run --quiet -- workbench \
  --workspace fixtures/v1/valid-workbench-flow \
  --bind 127.0.0.1 \
  --port "$server_port" \
  --no-open >"$tmp/server.log" 2>&1 &
server_pid=$!

server_ready=0
for _ in $(seq 1 120); do
  if curl --fail --silent "http://127.0.0.1:$server_port/api/projection" >/dev/null; then
    server_ready=1
    break
  fi
  sleep 0.1
done
if [[ "$server_ready" != 1 ]]; then
  cat "$tmp/server.log" >&2
  exit 1
fi

"$chrome" \
  --headless=new \
  --disable-gpu \
  --no-sandbox \
  --remote-allow-origins='*' \
  --remote-debugging-port="$debug_port" \
  --user-data-dir="$tmp/chrome" \
  about:blank >"$tmp/chrome.log" 2>&1 &
chrome_pid=$!

node - "http://127.0.0.1:$server_port/" "$debug_port" <<'NODE'
const [, , targetUrl, debugPort] = process.argv;
const debugOrigin = `http://127.0.0.1:${debugPort}`;

async function waitForTarget() {
  for (let attempt = 0; attempt < 120; attempt += 1) {
    try {
      const response = await fetch(`${debugOrigin}/json/list`);
      if (response.ok) {
        const targets = await response.json();
        const page = targets.find(target => target.type === 'page');
        if (page?.webSocketDebuggerUrl) return page.webSocketDebuggerUrl;
      }
    } catch {}
    await new Promise(resolve => setTimeout(resolve, 100));
  }
  throw new Error('Chrome DevTools page target did not become available');
}

class DevTools {
  constructor(url) {
    this.socket = new WebSocket(url);
    this.nextId = 1;
    this.pending = new Map();
    this.handlers = new Map();
  }

  async connect() {
    await new Promise((resolve, reject) => {
      this.socket.addEventListener('open', resolve, { once: true });
      this.socket.addEventListener('error', reject, { once: true });
    });
    this.socket.addEventListener('message', event => {
      const message = JSON.parse(String(event.data));
      if (message.id) {
        const pending = this.pending.get(message.id);
        if (!pending) return;
        this.pending.delete(message.id);
        if (message.error) pending.reject(new Error(message.error.message));
        else pending.resolve(message.result);
        return;
      }
      for (const handler of this.handlers.get(message.method) || []) handler(message.params);
    });
  }

  on(method, handler) {
    const handlers = this.handlers.get(method) || [];
    handlers.push(handler);
    this.handlers.set(method, handlers);
  }

  send(method, params = {}) {
    const id = this.nextId++;
    return new Promise((resolve, reject) => {
      this.pending.set(id, { resolve, reject });
      this.socket.send(JSON.stringify({ id, method, params }));
    });
  }

  close() {
    this.socket.close();
  }
}

async function main() {
  const devtools = new DevTools(await waitForTarget());
  await devtools.connect();
  const mutationResponses = [];
  devtools.on('Network.responseReceived', event => {
    if (event.response.url.includes('/api/work/')) {
      mutationResponses.push({ url: event.response.url, status: event.response.status });
    }
  });
  await devtools.send('Page.enable');
  await devtools.send('Runtime.enable');
  await devtools.send('Network.enable');
  await devtools.send('Page.addScriptToEvaluateOnNewDocument', {
    source: `
      window.__SYU_BROWSER_ERRORS__ = [];
      window.addEventListener('error', event => window.__SYU_BROWSER_ERRORS__.push(event.message || 'error'));
      window.addEventListener('unhandledrejection', event => window.__SYU_BROWSER_ERRORS__.push(String(event.reason || 'rejection')));
      window.addEventListener('syu-workbench-error', event => window.__SYU_BROWSER_ERRORS__.push(String(event.detail || 'Workbench startup failed')));
    `,
  });
  const load = new Promise(resolve => devtools.on('Page.loadEventFired', resolve));
  await devtools.send('Page.navigate', { url: targetUrl });
  await load;

  const evaluation = await devtools.send('Runtime.evaluate', {
    awaitPromise: true,
    returnByValue: true,
    expression: `
      (async () => {
        const flow = [];
        const wait = (description, predicate) => new Promise((resolve, reject) => {
          const deadline = Date.now() + 15000;
          const check = () => {
            let value;
            try { value = predicate(); } catch {}
            if (value) return resolve(value);
            if (Date.now() >= deadline) return reject(new Error('timeout waiting for ' + description));
            setTimeout(check, 50);
          };
          check();
        });
        const click = async (name, selector, predicate = node => !node.disabled) => {
          const node = await wait(name, () => {
            const candidate = document.querySelector(selector);
            return candidate && predicate(candidate) ? candidate : null;
          });
          node.click();
          flow.push(name);
          await new Promise(resolve => setTimeout(resolve, 100));
        };

        await click('specifications', '[data-route="specifications"]');
        await click('request', '[data-page="specifications"] .specification-criterion button');
        await click('plan', '[data-work-plan]');
        await wait('planned slice', () => document.querySelector('[data-work-slices-rail] .rail-item'));
        await click('context', '[data-work-context]');
        await wait('context pack', () => (document.querySelector('[data-work-context-detail]')?.textContent || '').includes('Context pack loaded'));
        await click('validate', '[data-work-validate]');
        await wait('passed validation', () => document.querySelector('[data-work-validation-detail]')?.textContent === 'passed');

        return {
          flow,
          validation: document.querySelector('[data-work-validation-detail]')?.textContent || '',
          errors: window.__SYU_BROWSER_ERRORS__ || [],
        };
      })()
    `,
  });
  if (evaluation.exceptionDetails) throw new Error(evaluation.exceptionDetails.text || 'browser flow failed');
  const result = { ...evaluation.result.value, mutationResponses };
  const expectedFlow = ['specifications', 'request', 'plan', 'context', 'validate'];
  if (JSON.stringify(result.flow) !== JSON.stringify(expectedFlow)) {
    throw new Error(`unexpected Workbench browser flow: ${JSON.stringify(result)}`);
  }
  if (result.errors.length || result.validation !== 'passed') {
    throw new Error(`Workbench browser errors: ${JSON.stringify(result)}`);
  }
  if (mutationResponses.length < 4 || mutationResponses.some(response => response.status < 200 || response.status >= 300)) {
    throw new Error(`Workbench mutation responses were not successful: ${JSON.stringify(result)}`);
  }
  console.log(JSON.stringify(result));
  devtools.close();
}

main().catch(error => {
  console.error(error.stack || error.message || String(error));
  process.exit(1);
});
NODE
