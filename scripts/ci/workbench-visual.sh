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
    """  if(path.includes('/api/source?target=')) return body({path:'tests/behavior.rs',content:'#[test]\\nfn behavior_stays_valid() {}',hash:'visual-test-hash',line_start:1,line_end:2,is_excerpt:true});\n"""
    """  if(path.includes('/api/source?path=syu.yaml')) return body({content:'schema: syu/config/v1\\nworkspace:\\n  spec_roots: [docs/syu]\\n',hash:'visual-test-hash'});\n"""
    """  if(path.includes('/api/scope/diff')) return body({range:'origin/main...HEAD',state:'ready',additions:2,deletions:1,files:[{path:'src/lib.rs',status:'modified',additions:2,deletions:1,patch:'diff --git a/src/lib.rs b/src/lib.rs\\n--- a/src/lib.rs\\n+++ b/src/lib.rs\\n@@ -1 +1,2 @@\\n-old\\n+new\\n+line'}]});\n"""
    """  if(path.includes('/api/scope/branch')) return body({branch:{range:'origin/main...HEAD',state:'ready',reason:null,changed:[{path:'src/lib.rs',status:'modified',owners:['FEAT-VISUAL'],anchors:['REQ-VISUAL#criterion.behavior'],artifact_identities:['rust:src/lib.rs']}],owned:[],unowned:[],affected_items:[]}});\n"""
    """  if(path.includes('/api/config')) return body({});\n"""
    """  if(path.includes('/target-suggestions/approve')) return body({approved_ids:['target-visual'],split_recommendation:null});\n"""
    """  if(path.includes('/target-suggestions')) return body({criterion:'REQ-VISUAL#criterion.behavior',suggestion_token:'visual-suggestion-token',suggestions:[{id:'target-visual',rank:1,ref:'rust:src/lib.rs#behavior',confidence:'high',role:'implementation',evidence:['visual smoke evidence'],evidence_fingerprint:'visual-evidence'}],split_recommendation:null});\n"""
    """  if(path.includes('/api/work/action')) {\n"""
    """    const action=payload.action; window.__SYU_FLOW__.push(action);\n"""
    """    const journey=(step,primary,status)=>projection.journey={title:payload.summary||projection.work.request?.summary||'Make the behavior clear',current_step:step,steps:[],primary_action:{action:primary,confirmation_required:['approve','start','finalize'].includes(primary)},recovery_action:primary==='cancel'?null:{action:'cancel',confirmation_required:true},approved_scope:step==='review'?null:{editable_target_count:1,slice_count:1},evidence:{status,blockers:[]},related_specification:projection.journey.related_specification||null,advanced:{request_id:'work-visual',plan_id:projection.work.plan?.id||null,selected_slice_id:'slice-visual-flow',attempt_id:projection.work.completion?.current?.attempt_id||null,specification_anchor:projection.journey.advanced?.specification_anchor||null}};\n"""
    """    if(action==='create') { const item=projection.specifications.specifications.find(candidate=>candidate.criteria?.some(criterion=>criterion.anchor===payload.anchor)); const criterion=item?.criteria.find(candidate=>candidate.anchor===payload.anchor); projection.journey.related_specification=item&&criterion?{title:item.title,overview:item.summary||item.description||'',status:item.status,criterion_statement:criterion.statement}:null; projection.journey.advanced={specification_anchor:criterion?.anchor||null}; projection.work.request={summary:payload.summary,operation:'modify',seed_count:1,requested_target_count:0}; journey('review','prepare','draft'); }\n"""
    """    else if(action==='prepare') { projection.work.plan={id:'PLAN-VISUAL-FLOW',digest:'visual-plan',status:'ready',slices:[{id:'slice-visual-flow',editable_targets:[{reference:'FEAT-VISUAL#binding.work/target.code',access:'editable',path:'src/lib.rs'}]}]}; projection.work.selected_slice='slice-visual-flow'; projection.work.validation={state:'passed',context:'work-plan'}; journey('approve','approve','reviewed'); }\n"""
    """    else if(action==='approve') journey('implement','start','approved');\n"""
    """    else if(action==='start') { projection.work.agent={run_id:'agent-visual-flow',status:'active'}; journey('verify','verify','in_progress'); }\n"""
    """    else if(action==='verify') { projection.work.agent.status='completed'; projection.work.completion={current:{attempt_id:'attempt-visual-flow',status:'complete',plan_digest:'visual-plan',slice_id:'slice-visual-flow',demonstrated:['REQ-VISUAL#criterion.behavior'],finalized:false},previous:[]}; journey('complete','finalize','ready'); }\n"""
    """    else if(action==='finalize') { projection.work.completion.current.finalized=true; journey('complete','cancel','complete'); }\n"""
    """    return body(projection);\n"""
    """  }\n"""
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
window.addEventListener('error',event=>window.__SYU_VISUAL_ERRORS__.push(`${event.error?.stack||event.message||'error'} at ${event.filename}:${event.lineno}:${event.colno}`));
window.addEventListener('unhandledrejection',event=>window.__SYU_VISUAL_ERRORS__.push(event.reason?.stack||String(event.reason||'rejection')));
setTimeout(()=>{
  const failures=[];
  const wait=ms=>new Promise(resolve=>setTimeout(resolve,ms));
  const click=async s=>{const node=document.querySelector(s);if(!node){failures.push(`missing ${s}`);return;}node.click();await wait(80);};
  const visible=s=>{const node=document.querySelector(s);return !!node&&!node.hidden;};

  (async()=>{
  const workStart=document.querySelector('[data-page="work"] .work-start');
  if(!workStart) failures.push('Work did not show the specification-first start state');
  if(document.querySelector('[data-page="work"] .journey-intake, [data-page="work"] .journey-card')) failures.push('Work still shows the legacy behavior search');
  const locale=document.documentElement.dataset.locale;
  if(locale==='ja') {
    if(workStart?.querySelector('h2')?.textContent!=='作業を作る仕様を選ぶ') failures.push('Japanese initial Work title is not localized');
    if(workStart?.querySelector('.journey-action-label')?.textContent!=='仕様一覧を開く') failures.push('Japanese initial Work CTA is not localized');
    if(!workStart?.querySelector('p')?.textContent.includes('仕様一覧から対象を選びます')) failures.push('Japanese initial Work explanation is not localized');
  }
  await click('[data-page="work"] .work-start .journey-action');
  if(!visible('[data-page="specifications"]')) failures.push('Work start did not open Specifications');
  const selectedSpecificationTitle=document.querySelector('[data-page="specifications"] [data-specifications-detail] .canvas-head h2')?.textContent;
  const selectedCriterion=document.querySelector('[data-page="specifications"] .specification-criterion');
  const selectedCriterionAnchor=selectedCriterion?.querySelector('strong')?.textContent;
  const selectedCriterionStatement=selectedCriterion?.querySelector('p')?.textContent;
  await click('[data-page="specifications"] [data-review-target-suggestions]');
  if(!document.querySelector('[data-page="specifications"] .target-suggestions')) failures.push('Target Suggestions did not open');
  await click('[data-page="specifications"] [data-approve-target-suggestions]');
  if(!visible('[data-page="specifications"]')) failures.push('Target Suggestions approval left Specifications');
  if(projection.work.request) failures.push('Target Suggestions approval created a WorkRequest');
  await click('[data-page="specifications"] [data-create-work]');
  if(!visible('[data-page="work"]')) failures.push('specification Create Work did not open Work');
  if(document.querySelector('[data-page="work"] .journey-header h2')?.textContent.startsWith('Change ')) failures.push('Create Work title still has the English Change prefix');
  if(document.querySelector('[data-page="work"] [data-work-specification-title]')?.textContent!==selectedSpecificationTitle) failures.push('related specification does not match the selected specification');
  if(document.querySelector('[data-page="work"] [data-work-specification-criterion]')?.textContent!==selectedCriterionStatement) failures.push('related criterion does not match the selected criterion');
  if(document.querySelector('[data-page="work"] [data-work-specification-anchor]')?.dataset.workSpecificationAnchor!==selectedCriterionAnchor) failures.push('Work seed anchor does not match the selected criterion');
  if(document.querySelectorAll('[data-page="work"] [data-work-specification-title]').length!==1) failures.push('related specification title is missing or duplicated');
  if(document.querySelectorAll('[data-page="work"] [data-work-specification-criterion]').length!==1) failures.push('related criterion is missing or duplicated');
  if(document.querySelector('[data-work-overview] [data-work-specification-title], [data-work-overview] [data-work-specification-criterion]')) failures.push('specification content leaked into the Work pane');
  const specificationBody=document.querySelector('[data-work-specification] .journey-specification-body');
  const specificationToggle=document.querySelector('[data-work-specification] .journey-specification-toggle');
  const columns=getComputedStyle(document.querySelector('[data-work-journey-workspace]')).gridTemplateColumns.split(' ').length;
  if(window.innerWidth>=1200 && columns!==2) failures.push(`desktop Work layout did not split: ${columns} columns`);
  if(window.innerWidth<1200 && getComputedStyle(specificationBody).display!=='none') failures.push('narrow related specification started expanded');
  if(window.innerWidth<1200) {
    specificationToggle.click();
    await wait(40);
    if(getComputedStyle(document.querySelector('[data-work-specification] .journey-specification-body')).display==='none') failures.push('narrow related specification did not expand');
  }
  const relatedFeature=document.querySelector('[data-work-specification] .related-row.specification');
  if(!relatedFeature) failures.push('related feature navigation is missing');
  else {
    relatedFeature.click();
    await wait(40);
    const relationType=document.querySelector('[data-work-specification] .related-chooser select');
    if(!relationType) failures.push('related type selector is missing');
    else {
      relationType.value='implementation';
      relationType.dispatchEvent(new Event('change',{bubbles:true}));
      await wait(40);
      const relatedCode=document.querySelector('[data-work-specification] .related-row.implementation');
      if(!relatedCode) failures.push('related implementation target is missing');
      else {
        relatedCode.click();
        await wait(80);
        if(!document.querySelector('[data-work-specification] .source-code')) failures.push('related source excerpt did not render');
      }
    }
  }
  window.confirm=()=>true;
  await click('[data-page="work"] .journey-action.primary');
  await click('[data-page="work"] .journey-count.interactive');
  if(!document.querySelector('[data-work-specification] [data-journey-panel="scope"]')) failures.push('scope count did not open the scope panel');
  const scopeTarget=document.querySelector('[data-work-specification] [data-scope-target]');
  if(!scopeTarget) failures.push('scope panel did not render an exact target');
  else {
    scopeTarget.click();
    await wait(80);
    if(!document.querySelector('[data-work-specification] .source-code')) failures.push('scope target did not render its source excerpt');
  }
  await click('[data-page="work"] .journey-action.primary');
  await click('[data-page="work"] .journey-action.primary');
  if(!document.querySelector('[data-work-specification] [data-journey-panel="diff"] .diff-file')) failures.push('implementation did not open the diff panel');
  await click('[data-page="work"] .journey-action.primary');
  await click('[data-page="work"] .journey-action.primary');
  if(JSON.stringify(window.__SYU_FLOW__)!=='["create","prepare","approve","start","verify","finalize"]') failures.push(`unexpected work flow: ${JSON.stringify(window.__SYU_FLOW__)}`);
  if(!document.querySelector('[data-page="work"] .journey-advanced')) failures.push('advanced completion evidence missing');
  await click('[data-page="work"] [data-tab="slices"]');
  if(!visible('[data-page="work"] [data-panel="slices"]')) failures.push('Work slices tab did not open');
  if(!document.querySelector('[data-work-slice-detail] .journey-scope-target')) failures.push('Work slices tab did not render targets');

  await click('[data-route="scope"]');
  if(document.querySelector('[data-page="scope"] [data-scope-create-work]')) failures.push('Scope still exposes a Work creation entrypoint');
  await click('[data-scope-mode-button="branch"]');
  if(!document.querySelector('[data-scope-detail] .diff-file')) failures.push('Branch scope did not render diff');
  await click('[data-page="scope"] [data-tab="intent"]');
  if(!document.querySelector('[data-scope-detail] .scope-flow')) failures.push('Scope intent tab did not render');

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

for viewport in 1280,900 760,900; do
  for locale in en ja; do
    behavior="$("$chrome" --headless --disable-gpu --no-sandbox --allow-file-access-from-files --window-size="$viewport" --virtual-time-budget=6000 --dump-dom "file://$tmp/workbench.html?page=work&lang=$locale&theme=light")"
    if ! echo "$behavior" | grep -q 'id="syu-visual-behavior-result" data-status="pass"'; then
      echo "$behavior" | grep 'id="syu-visual-behavior-result"' >&2 || true
      exit 1
    fi
  done
done

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
  await devtools.send('Emulation.setDeviceMetricsOverride', {
    width: 1280,
    height: 900,
    deviceScaleFactor: 1,
    mobile: false,
  });
  await devtools.send('Page.addScriptToEvaluateOnNewDocument', {
    source: `
      window.__SYU_BROWSER_ERRORS__ = [];
      window.addEventListener('error', event => window.__SYU_BROWSER_ERRORS__.push(event.message || 'error'));
      window.addEventListener('unhandledrejection', event => window.__SYU_BROWSER_ERRORS__.push(String(event.reason || 'rejection')));
      window.addEventListener('syu-workbench-error', event => window.__SYU_BROWSER_ERRORS__.push(String(event.detail || 'Workbench startup failed')));
    `,
  });
  const load = new Promise(resolve => devtools.on('Page.loadEventFired', resolve));
  const pageUrl = new URL(targetUrl);
  pageUrl.searchParams.set('lang', 'ja');
  await devtools.send('Page.navigate', { url: pageUrl.toString() });
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
            if (Date.now() >= deadline) {
              return reject(new Error(
                'timeout waiting for ' + description
                + '; page=' + (document.querySelector('[data-page]:not([hidden])')?.dataset.page || 'none')
                + '; busy=' + document.body.getAttribute('aria-busy')
                + '; buttons=' + [...document.querySelectorAll('[data-page="specifications"] button')]
                  .map(node => node.textContent.trim() + ':' + node.disabled).join('|'),
              ));
            }
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

        const initialWorkStart = document.querySelector('[data-page="work"] .work-start');
        const initialWorkTitle = initialWorkStart?.querySelector('h2')?.textContent || '';
        const initialWorkCta = initialWorkStart?.querySelector('.journey-action-label')?.textContent || '';
        const initialWorkExplanation = initialWorkStart?.querySelector('p')?.textContent || '';
        await click('specifications', '[data-route="specifications"]');
        const selectedSpecificationTitle = document.querySelector('[data-page="specifications"] [data-specifications-detail] .canvas-head h2')?.textContent || '';
        const selectedCriterion = document.querySelector('[data-page="specifications"] .specification-criterion');
        const selectedCriterionAnchor = selectedCriterion?.querySelector('strong')?.textContent || '';
        const selectedCriterionStatement = selectedCriterion?.querySelector('p')?.textContent || '';
        await click('create', '[data-page="specifications"] [data-create-work]');
        await click('prepare', '[data-page="work"] .journey-action.primary');
        const approvalStep = document.documentElement.lang === 'ja' ? '承認' : 'Approve';
        await wait('approval step', () => document.querySelector('[data-page="work"] .journey-step.current')?.getAttribute('aria-label') === approvalStep);

        return {
          flow,
          currentStep: document.querySelector('[data-page="work"] .journey-step.current')?.getAttribute('aria-label') || '',
          initialWorkTitle,
          initialWorkCta,
          initialWorkExplanation,
          specificationTitleCount: document.querySelectorAll('[data-page="work"] [data-work-specification-title]').length,
          specificationCriterionCount: document.querySelectorAll('[data-page="work"] [data-work-specification-criterion]').length,
          selectedSpecificationTitle,
          selectedCriterionAnchor,
          selectedCriterionStatement,
          workSpecificationTitle: document.querySelector('[data-page="work"] [data-work-specification-title]')?.textContent || '',
          workSpecificationCriterion: document.querySelector('[data-page="work"] [data-work-specification-criterion]')?.textContent || '',
          workSpecificationAnchor: document.querySelector('[data-page="work"] [data-work-specification-anchor]')?.dataset.workSpecificationAnchor || '',
          workTitle: document.querySelector('[data-page="work"] .journey-header h2')?.textContent || '',
          workPaneSpecificationCount: document.querySelectorAll('[data-work-overview] [data-work-specification-title], [data-work-overview] [data-work-specification-criterion]').length,
          layoutColumns: getComputedStyle(document.querySelector('[data-work-journey-workspace]')).gridTemplateColumns.split(' ').length,
          errors: window.__SYU_BROWSER_ERRORS__ || [],
        };
      })()
    `,
  });
  if (evaluation.exceptionDetails) throw new Error(evaluation.exceptionDetails.text || 'browser flow failed');
  const result = { ...evaluation.result.value, mutationResponses };
  const expectedFlow = ['specifications', 'create', 'prepare'];
  if (JSON.stringify(result.flow) !== JSON.stringify(expectedFlow)) {
    throw new Error(`unexpected Workbench browser flow: ${JSON.stringify(result)}`);
  }
  if (
    result.errors.length
    || result.currentStep !== '承認'
    || result.initialWorkTitle !== '作業を作る仕様を選ぶ'
    || result.initialWorkCta !== '仕様一覧を開く'
    || !result.initialWorkExplanation.includes('仕様一覧から対象を選びます')
    || result.specificationTitleCount !== 1
    || result.specificationCriterionCount !== 1
    || result.workSpecificationTitle !== result.selectedSpecificationTitle
    || result.workSpecificationCriterion !== result.selectedCriterionStatement
    || result.workSpecificationAnchor !== result.selectedCriterionAnchor
    || result.workTitle.startsWith('Change ')
    || result.workPaneSpecificationCount !== 0
    || result.layoutColumns !== 2
  ) {
    throw new Error(`Workbench browser errors: ${JSON.stringify(result)}`);
  }
  if (mutationResponses.length < 2 || mutationResponses.some(response => response.status < 200 || response.status >= 300)) {
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
