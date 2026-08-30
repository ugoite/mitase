#!/usr/bin/env bash
# REQ-WORKBENCH-001
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$repo_root"
python3 scripts/ci/check-workbench-contract.py
node scripts/ci/check-workbench-i18n.mjs

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
  printf 'window.MITASE_I18N={en:'
  cat crates/mitase-app-ui/assets/locales/en.json
  printf ',ja:'
  cat crates/mitase-app-ui/assets/locales/ja.json
  printf '};\n'
} >"$tmp/catalog.js"

cargo run --quiet -p mitase-workbench-server --bin mitase-workbench -- project \
  --workspace fixtures/v1/valid-web-app \
  --format json >"$tmp/projection.json"

python3 - <<'PY' "$tmp/projection.json" "$tmp/state.html" "$tmp/api-mock.js"
import pathlib, sys
import json

projection_data = json.loads(pathlib.Path(sys.argv[1]).read_text())
def requirement_capability(anchor):
    return {
        'schema': 'mitase/work-origin-capability/v1',
        'origin': {
            'kind': 'requirement-criterion',
            'criterion': anchor,
        },
        'label': 'Requirement criterion',
        'enabled': True,
        'disabled_code': None,
        'disabled_message': None,
        'nearest': [],
    }
capability = requirement_capability('REQ-CAPABILITY-001#criterion.behavior')
synthetic = {
    'id': 'REQ-CAPABILITY-001',
    'kind': 'requirement',
    'path': 'spec/requirement.yaml',
    'source_hash': 'visual-capability-hash',
    'title': 'Canonical capability behavior',
    'presentation_title_key': 'specification.title.REQ-CAPABILITY-001',
    'summary': 'A built-in capability contract used by the Workbench smoke.',
    'description': 'A built-in capability contract used by the Workbench smoke.',
    'status': 'implemented',
    'priority': 'critical',
    'principles': [],
    'rules': [],
    'criteria': [{
        'anchor': 'REQ-CAPABILITY-001#criterion.behavior',
        'kind': 'behavior',
        'statement': 'The capability has an exact behavior boundary.',
        'governed_by': [],
    }],
    'bindings': [{
        'anchor': 'REQ-CAPABILITY-001#binding.visual',
        'role': 'implementation',
        'facet': 'visual',
        'responsibility': 'Keeps the visual capability contract explicit.',
        'owns': [],
        'targets': [],
    }],
    'contracts': [],
    'anchors': ['REQ-CAPABILITY-001#criterion.behavior'],
    'origin_capabilities': [capability],
}
projection_data['specifications']['specifications'].insert(0, synthetic)
requirement = next(
    item for item in projection_data['specifications']['specifications']
    if item['kind'] == 'requirement' and item.get('criteria')
)
fallback_anchor = requirement['criteria'][0]['anchor']
fallback_capability = requirement_capability(fallback_anchor)
def capability_anchor(value):
    origin = value.get('origin') or (value.get('nearest') or [{}])[0]
    return origin.get('criterion') if origin.get('kind') == 'requirement-criterion' else None
projection_data['specifications']['origin_capabilities'] = [
    value for value in projection_data['specifications'].get('origin_capabilities', [])
    if capability_anchor(value) != fallback_anchor
] + [fallback_capability, capability]
requirement['origin_capabilities'] = [
    value for value in requirement.get('origin_capabilities', [])
    if capability_anchor(value) != fallback_anchor
] + [fallback_capability]
feature = next(item for item in projection_data['specifications']['specifications'] if item['kind'] == 'feature')
feature_target = feature['bindings'][0]['targets'][0]
for anchor in (capability['origin']['criterion'], fallback_anchor):
    if not any(claim.get('criterion') == anchor for claim in feature_target.setdefault('claims', [])):
        feature_target['claims'].append({'kind': 'satisfies', 'criterion': anchor})
for candidate in projection_data['specifications']['specifications']:
    for binding in candidate.get('bindings', []):
        for ownership in binding.get('owns', []):
            ownership['selector'] = {'kind': 'module', 'name': '*'}
projection = json.dumps(projection_data).replace('<', '\\u003c')
pathlib.Path(sys.argv[2]).write_text(
    f'<script type="application/json" id="mitase-projection">{projection}</script>'
)
pathlib.Path(sys.argv[3]).write_text(
    """let projection=JSON.parse(document.querySelector('#mitase-projection').textContent);\n"""
    """const visualRequirement=projection.specifications.specifications.find(item=>item.kind==='requirement'&&item.criteria?.length);\n"""
    """if(visualRequirement) { const visualAnchor=visualRequirement.criteria[0].anchor; const visualCapability={schema:'mitase/work-origin-capability/v1',origin:{kind:'requirement-criterion',criterion:visualAnchor},label:'Requirement criterion',enabled:true,disabled_code:null,disabled_message:null,nearest:[]}; const same=value=>value.origin?.criterion===visualAnchor||value.nearest?.some(origin=>origin.kind==='requirement-criterion'&&origin.criterion===visualAnchor); visualRequirement.origin_capabilities=[visualCapability,...(visualRequirement.origin_capabilities||[]).filter(value=>!same(value))]; projection.specifications.origin_capabilities=[visualCapability,...(projection.specifications.origin_capabilities||[]).filter(value=>!same(value))]; const visualFeature=projection.specifications.specifications.find(item=>item.kind==='feature'); const visualTarget=visualFeature?.bindings?.[0]?.targets?.[0]; if(visualTarget) { visualTarget.claims=visualTarget.claims||[]; if(!visualTarget.claims.some(claim=>claim.criterion===visualAnchor)) visualTarget.claims.push({kind:'satisfies',criterion:visualAnchor}); } document.querySelector('#mitase-projection').textContent=JSON.stringify(projection); }\n"""
    """let receipt=null;\n"""
    """let approvedTargetSuggestions=[];\n"""
    """let nestedPatches=[]; window.__MITASE_NESTED_PATCHES__=nestedPatches;\n"""
    """const csrfToken='visual-csrf-token';\n"""
    """window.__MITASE_FLOW__=[];\n"""
    """const body=(value,status=200,headers={})=>Promise.resolve({ok:status>=200&&status<300,status,headers:{get:name=>headers[name.toLowerCase()]||null},text:async()=>value==null?'':typeof value==='string'?value:JSON.stringify(value)});\n"""
    """window.fetch=async(url,options={})=>{\n"""
    """  const path=String(url);\n"""
    """  const payload=options.body?JSON.parse(options.body):{};\n"""
    """  const method=(options.method||'GET').toUpperCase();\n"""
    """  const suppliedCsrf=Object.entries(options.headers||{}).find(([key])=>key.toLowerCase()==='x-mitase-csrf-token')?.[1];\n"""
    """  if(method!=='GET' && suppliedCsrf!==csrfToken) return body({error:'missing csrf token'},403);\n"""
    """  if(path.includes('/api/projection')) return body(projection,200,{'x-mitase-csrf-token':csrfToken});\n"""
    """  if(path.includes('/api/work/session')) return body({ready:true},200,{'x-mitase-csrf-token':csrfToken});\n"""
    """  if(path.includes('/api/specifications/candidates/preview')) { nestedPatches.push(payload.patch); return body({preview_token:'visual-preview-token',old_hash:'visual-old-hash',new_hash:'visual-new-hash',workspace_fingerprint:'visual-fingerprint',impact:{readiness_before:{status:'ready'},readiness_after:{status:'ready'},changed_anchors:[],affected_ownership:[],implementation_targets:[],verification_targets:[],target_suggestions:[]}},200,{'x-mitase-csrf-token':csrfToken}); }\n"""
    """  if(path.includes('/api/specifications/') && path.includes('/trace')) { const itemId=decodeURIComponent(path.split('/api/specifications/')[1].split('/')[0]); const criterion=projection.specifications.specifications.find(item=>item.id===itemId)?.criteria?.[0]?.anchor||`${itemId}#criterion.behavior`; const runtimeTarget='FEAT-AUTH-001#binding.backend/target.handler'; const related={specification:itemId==='REQ-AUTH-001'?[{item_id:'FEAT-AUTH-001',kind:'feature',title:'Authentication feature',presentation_title_key:null}]:[],implementation:[{item_id:'FEAT-AUTH-001',target:{reference:runtimeTarget,path:'src/handlers.rs',selector:{kind:'symbol',name:'handler'},adapter:'rust',lifecycle:'present',claims:[]}}],verification:[]}; return body({root_item_id:itemId,revision:'visual-revision',workspace_fingerprint:'visual-fingerprint',source_hash:'visual-source-hash',mode:path.includes('mode=exact')?'exact':'readable',nodes:[],edges:[],related,closures:[{criterion,implementation_targets:[runtimeTarget],verification_targets:[runtimeTarget,'FEAT-AUTH-001#binding.backend/target.test'],state:'declaration-only',reasons:[],runtime_status:'partial',runtime_timestamp:'2026-08-02T00:00:00Z',runtime_revision:'visual-revision',runtime_receipt:'slice-visual@visual-revision@2026-08-02T00:00:00Z',runtime_executions:[{identity:'slice-visual#execution-0',target:runtimeTarget,claim:{target:runtimeTarget,criterion},status:'passed'}],readiness_blockers:[],diagnostics:[],hidden_target_count:0,hidden_reason_count:0,hidden_readiness_count:0,hidden_diagnostic_count:0}],truncated:false,hidden_node_count:0,hidden_edge_count:0}); }\n"""
    """  if(path.includes('/api/source?target=')) return body({path:'tests/behavior.rs',content:'#[test]\\nfn behavior_stays_valid() {}',hash:'visual-test-hash',line_start:1,line_end:2,is_excerpt:true});\n"""
    """  if(path.includes('/api/source?path=mitase.yaml')) return body({content:'schema: mitase/config/v1\\nworkspace:\\n  spec_roots: [docs/mitase]\\n',hash:'visual-test-hash'});\n"""
    """  if(path.includes('/api/scope/diff')) return body({range:'origin/main...HEAD',state:'ready',additions:2,deletions:1,files:[{path:'src/lib.rs',status:'modified',additions:2,deletions:1,patch:'diff --git a/src/lib.rs b/src/lib.rs\\n--- a/src/lib.rs\\n+++ b/src/lib.rs\\n@@ -1 +1,2 @@\\n-old\\n+new\\n+line'}]});\n"""
    """  if(path.includes('/api/scope/branch')) return body({branch:{range:'origin/main...HEAD',state:'ready',reason:null,changed:[{path:'src/lib.rs',status:'modified',owners:['FEAT-VISUAL'],anchors:['REQ-VISUAL#criterion.behavior'],artifact_identities:['rust:src/lib.rs']}],owned:[],unowned:[],affected_items:[]}});\n"""
    """  if(path.includes('/api/config')) return body({});\n"""
    """  if(path.includes('/target-suggestions/approve')) { approvedTargetSuggestions.push('target-visual'); return body({approved_ids:['target-visual'],split_recommendation:null}); }\n"""
    """  if(path.includes('/target-suggestions')) return body({criterion:'REQ-VISUAL#criterion.behavior',suggestion_token:'visual-suggestion-token',suggestions:[{id:'target-visual',rank:1,ref:'rust:src/lib.rs#behavior',confidence:'high',role:'implementation',evidence:['visual smoke evidence'],evidence_fingerprint:'visual-evidence'}],approved_ids:approvedTargetSuggestions,split_recommendation:null});\n"""
    """  if(path.includes('/api/work/action')) {\n"""
    """    const action=payload.action; window.__MITASE_FLOW__.push(action);\n"""
    """    const journey=(step,primary,status)=>projection.journey={title:payload.title||projection.work.request?.title||'Make the behavior clear',current_step:step,steps:[],primary_action:{action:primary,confirmation_required:['approve','start','finalize'].includes(primary)},recovery_action:primary==='cancel'?null:{action:'cancel',confirmation_required:true},approved_scope:step==='review'?null:{editable_target_count:1,slice_count:1},evidence:{status,blockers:[]},related_specification:projection.journey.related_specification||null,advanced:{request_id:'work-visual',plan_id:projection.work.plan?.id||null,selected_slice_id:'slice-visual-flow',attempt_id:projection.work.completion?.current?.attempt_id||null,specification_anchor:projection.journey.advanced?.specification_anchor||null}};\n"""
    """    if(action==='create') { const criterionAnchor=payload.anchor||payload.origin?.criterion; const item=projection.specifications.specifications.find(candidate=>candidate.criteria?.some(criterion=>criterion.anchor===criterionAnchor)); const criterion=item?.criteria.find(candidate=>candidate.anchor===criterionAnchor); projection.journey.related_specification=item&&criterion?{title:item.title,overview:item.summary||item.description||'',status:item.status,criterion_statement:criterion.statement}:null; projection.journey.advanced={specification_anchor:criterion?.anchor||null}; projection.work.request={title:payload.title,operation:'modify',origin:payload.origin,constraints:{},requested_targets:[]}; journey('review','prepare','draft'); }\n"""
    """    else if(action==='prepare') { projection.work.plan={id:'PLAN-VISUAL-FLOW',digest:'visual-plan',status:'ready',slices:[{id:'slice-visual-flow',editable_targets:[{reference:'FEAT-VISUAL#binding.work/target.code',access:'run-only',transition:'run-only',path:'src/lib.rs'}]}]}; projection.work.selected_slice='slice-visual-flow'; projection.work.validation={state:'passed',context:'work-plan'}; journey('approve','approve','reviewed'); }\n"""
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

html = (repo_root / "crates/mitase-app-ui/assets/workbench.html").read_text()

html = html.replace(
    "/assets/workbench.css",
    f"file://{repo_root}/crates/mitase-app-ui/assets/workbench.css",
)
html = html.replace(
    "/assets/catalog.js",
    f"file://{tmp}/catalog.js",
)
html = html.replace(
    "/assets/i18n.js",
    f"file://{repo_root}/crates/mitase-app-ui/assets/i18n.js",
)
html = html.replace(
    "/assets/js/main.js",
    f"file://{repo_root}/crates/mitase-app-ui/assets/js/main.js",
)
html = html.replace(
    '<script type="application/json" id="mitase-projection"></script>',
    (tmp / "state.html").read_text()
    + f'<script src="file://{tmp}/api-mock.js"></script>',
)

(tmp / "workbench.html").write_text(html)
PY

cat >>"$tmp/workbench.html" <<'HTML'
<script>
window.__MITASE_VISUAL_ERRORS__=[];
window.addEventListener('error',event=>window.__MITASE_VISUAL_ERRORS__.push(`${event.error?.stack||event.message||'error'} at ${event.filename}:${event.lineno}:${event.colno}`));
window.addEventListener('unhandledrejection',event=>window.__MITASE_VISUAL_ERRORS__.push(event.reason?.stack||String(event.reason||'rejection')));
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
  window.MitasePreferences.translate('ja');
  await wait(40);
  if(document.documentElement.lang!=='ja') failures.push('Japanese locale did not apply');
  if(document.querySelector('[data-page="work"] h1')?.textContent.trim()!=='作業') failures.push('already-rendered Work page did not rerender in Japanese');
  window.MitasePreferences.translate('en');
  await wait(40);
  if(document.querySelector('[data-page="work"] h1')?.textContent.trim()!=='Work') failures.push('already-rendered Work page did not rerender in English');
  await click('[data-page="work"] .work-start .journey-action');
  if(!visible('[data-page="specifications"]')) failures.push('Work start did not open Specifications');
  if(new URL(location.href).searchParams.get('page')!=='specifications') failures.push('programmatic Work start did not synchronize the Specifications URL');
  history.back();
  await wait(100);
  if(new URL(location.href).searchParams.get('page')!=='work' || !visible('[data-page="work"]')) failures.push('back navigation did not restore the Work route after programmatic transition');
  history.forward();
  await wait(100);
  if(new URL(location.href).searchParams.get('page')!=='specifications' || !visible('[data-page="specifications"]')) failures.push('forward navigation did not restore the Specifications route after programmatic transition');
  const detailTabs=document.querySelector('[data-page="specifications"] .specification-detail-tabs');
  const detailTabButtons=[...(detailTabs?.querySelectorAll('[role="tab"]')||[])];
  if(detailTabButtons.length!==3) failures.push('Specification detail tabs are incomplete');
  else {
    detailTabButtons[0].focus();
    detailTabs.dispatchEvent(new KeyboardEvent('keydown',{key:'ArrowRight',bubbles:true}));
    await wait(40);
    if(!document.querySelector('[data-page="specifications"] [data-detail-tab="trace"][aria-selected="true"]')) failures.push('detail ArrowRight did not select Trace');
    if(document.activeElement?.dataset.detailTab!=='trace') failures.push('detail ArrowRight did not move focus');
    document.querySelector('[data-page="specifications"] .specification-detail-tabs')?.dispatchEvent(new KeyboardEvent('keydown',{key:'End',bubbles:true}));
    await wait(40);
    if(!document.querySelector('[data-page="specifications"] [data-detail-tab="evidence"][aria-selected="true"]')) failures.push('detail End did not select Evidence');
    document.querySelector('[data-page="specifications"] .specification-detail-tabs')?.dispatchEvent(new KeyboardEvent('keydown',{key:'Home',bubbles:true}));
    await wait(40);
    document.querySelector('[data-page="specifications"] [data-detail-tab="evidence"]')?.click();
    await wait(80);
    if(!document.querySelector('[data-page="specifications"] .closure-card')?.textContent.includes('slice-visual#execution-0')) failures.push('runtime execution identity was not rendered separately');
    if(!document.querySelector('[data-page="specifications"] .closure-card')?.textContent.includes('partial')) failures.push('partial runtime evidence was not surfaced');
    document.querySelector('[data-page="specifications"] [data-detail-tab="information"]')?.click();
    await wait(40);
    const bindingEdit=document.querySelector('[data-page="specifications"] .specification-detail-binding .spec-detail-anchor-head .btn');
    if(!bindingEdit) failures.push('typed binding editor is missing');
    else {
      bindingEdit.click();
      await wait(40);
      const facet=document.querySelector('[data-page="specifications"] .specification-editor input[name="facet"]');
      if(!facet) failures.push('typed binding facet field is missing');
      else {
        facet.value='visual-edited';
        facet.dispatchEvent(new Event('input',{bubbles:true}));
        document.querySelector('[data-page="specifications"] .specification-editor').requestSubmit();
        await wait(100);
        const patch=window.__MITASE_NESTED_PATCHES__?.[0];
        if(!patch || patch.edit?.entity!=='binding' || patch.edit?.binding?.anchor) failures.push('browser did not send the typed binding payload');
      }
      document.querySelector('[data-page="specifications"] .specification-editor .canvas-head button')?.click();
      await wait(40);
    }
    const featureRail=[...document.querySelectorAll('[data-page="specifications"] .rail-item')]
      .find(node=>node.textContent.includes('FEAT-AUTH-001'));
    if(featureRail) {
      featureRail.querySelector('.rail-item-select')?.click();
      await wait(60);
      const ownershipEdit=document.querySelector('[data-page="specifications"] .specification-detail-ownership .btn');
      if(!ownershipEdit) failures.push('module ownership editor is missing');
      else {
        ownershipEdit.click();
        await wait(40);
        const editor=document.querySelector('[data-page="specifications"] .specification-editor');
        const selectorKind=editor?.querySelector('[name="selector_kind"]');
        const selectorName=editor?.querySelector('[name="selector_name"]');
        if(!selectorKind || !selectorName) failures.push('module ownership selector fields are missing');
        else {
          selectorKind.value='module';
          selectorName.value='*';
          editor.requestSubmit();
          await wait(100);
          const patches=window.__MITASE_NESTED_PATCHES__||[];
          const patch=patches[patches.length-1];
          if(patch?.edit?.entity!=='ownership' || patch.edit.ownership.selector?.kind!=='module' || patch.edit.ownership.selector?.name!=='*') failures.push('module ownership selector did not round-trip losslessly');
        }
        document.querySelector('[data-page="specifications"] .specification-editor .canvas-head button')?.click();
        await wait(40);
      }
      const requirementRail=[...document.querySelectorAll('[data-page="specifications"] .rail-item')]
        .find(node=>node.textContent.includes('REQ-AUTH-001'));
      requirementRail?.querySelector('.rail-item-select')?.click();
      await wait(60);
    } else failures.push('feature fixture for module ownership is missing');
  }
  const selectedSpecificationTitle=document.querySelector('[data-page="specifications"] [data-specifications-detail] .canvas-head h2')?.textContent;
  const selectedCriterion=document.querySelector('[data-page="specifications"] .specification-criterion');
  const selectedCriterionAnchor=selectedCriterion?.querySelector('strong')?.textContent;
  const selectedCriterionStatement=selectedCriterion?.querySelector('p')?.textContent;
  await click('[data-page="specifications"] [data-review-target-suggestions]');
  if(!document.querySelector('[data-page="specifications"] .target-suggestions')) failures.push('Target Suggestions did not open');
  await click('[data-page="specifications"] [data-approve-target-suggestions]');
  if(!visible('[data-page="specifications"]')) failures.push('Target Suggestions approval left Specifications');
  if(projection.work.request) failures.push('Target Suggestions approval created a WorkRequest');
  await click('[data-page="specifications"] .target-suggestions .btn.ghost');
  await click('[data-page="specifications"] [data-review-target-suggestions]');
  if(!document.querySelector('[data-page="specifications"] [data-target-suggestion-approved]')) failures.push('accepted target suggestion state was not restored');
  await click('[data-page="specifications"] .target-suggestions .btn.ghost');
  await click('[data-page="specifications"] [data-create-work]');
  if(!visible('[data-page="work"]')) failures.push('specification Create Work did not open Work');
  if(new URL(location.href).searchParams.get('page')!=='work' || !new URL(location.href).searchParams.get('workItem')) failures.push('Create Work did not synchronize page=work and workItem');
  if(document.querySelector('[data-page="work"] .journey-header h2')?.textContent.startsWith('Change ')) failures.push('Create Work title still has the English Change prefix');
  if(document.querySelector('[data-page="work"] [data-work-specification-title]')?.textContent!==selectedSpecificationTitle) failures.push('related specification does not match the selected specification');
  if(document.querySelector('[data-page="work"] [data-work-specification-criterion]')?.textContent!==selectedCriterionStatement) failures.push('related criterion does not match the selected criterion');
  if(document.querySelector('[data-page="work"] [data-work-specification-anchor]')?.dataset.workSpecificationAnchor!==selectedCriterionAnchor) failures.push('Work seed anchor does not match the selected criterion');
  const workTraceTab=document.querySelector('[data-page="work"] [data-detail-tab="trace"]');
  if(workTraceTab) {
    workTraceTab.click();
    await wait(80);
    if(new URL(location.href).searchParams.get('page')!=='work') failures.push('Work trace tab rewrote the route to Specifications');
    history.back();
    await wait(100);
    if(new URL(location.href).searchParams.get('page')!=='work') failures.push('Work back navigation left the Work route');
    history.forward();
    await wait(100);
    if(new URL(location.href).searchParams.get('workDetailTab')!=='trace') failures.push('Work forward navigation did not restore Trace context');
    await click('[data-page="work"] [data-detail-tab="information"]');
  } else failures.push('Work trace tab is missing');
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
        const sourceReference=relatedCode.dataset.sourceTarget;
        const sourceClose=document.querySelector('[data-work-specification] .specification-source-inspector .source-inspector-head button');
        if(!sourceClose) failures.push('source inspector close control is missing');
        else {
          sourceClose.click();
          await wait(80);
          if(document.activeElement?.dataset.sourceTarget!==sourceReference) failures.push('source inspector did not restore trigger focus');
        }
      }
    }
  }
  window.confirm=()=>true;
  await click('[data-page="work"] .journey-action.primary');
  window.MitasePreferences.translate('ja');
  await wait(40);
  if(!document.querySelector('[data-page="work"]')?.textContent.includes('実行のみ')) failures.push('run-only target metadata did not localize');
  window.MitasePreferences.translate('en');
  await wait(40);
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
  if(JSON.stringify(window.__MITASE_FLOW__)!=='["create","prepare","approve","start","verify","finalize"]') failures.push(`unexpected work flow: ${JSON.stringify(window.__MITASE_FLOW__)}`);
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

  if(window.__MITASE_VISUAL_ERRORS__.length) failures.push(`js errors: ${window.__MITASE_VISUAL_ERRORS__.join(', ')}`);
  const result = document.createElement('div');
  result.id = 'mitase-visual-behavior-result';
  result.dataset.status = failures.length ? 'fail' : 'pass';
  result.textContent = failures.join('; ');
  document.body.append(result);
  })();
}, 800);
</script>
HTML

for viewport in 1280,900 760,900; do
  for locale in en ja; do
    behavior="$("$chrome" --headless --disable-gpu --no-sandbox --allow-file-access-from-files --window-size="$viewport" --virtual-time-budget=12000 --dump-dom "file://$tmp/workbench.html?page=work&lang=$locale&theme=light")"
    if ! echo "$behavior" | grep -q 'id="mitase-visual-behavior-result" data-status="pass"'; then
      echo "$behavior" | grep 'id="mitase-visual-behavior-result"' >&2 || true
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

cargo run --quiet -p mitase-workbench-server --bin mitase-workbench -- serve \
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
      window.__MITASE_BROWSER_ERRORS__ = [];
      window.addEventListener('error', event => window.__MITASE_BROWSER_ERRORS__.push(event.message || 'error'));
      window.addEventListener('unhandledrejection', event => window.__MITASE_BROWSER_ERRORS__.push(String(event.reason || 'rejection')));
      window.addEventListener('mitase-workbench-error', event => window.__MITASE_BROWSER_ERRORS__.push(String(event.detail || 'Workbench startup failed')));
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

        const initialWorkStart = await wait('initial Work start', () => document.querySelector('[data-page="work"] .work-start'));
        const initialWorkTitle = initialWorkStart?.querySelector('h2')?.textContent || '';
        const initialWorkCta = initialWorkStart?.querySelector('.journey-action-label')?.textContent || '';
        const initialWorkExplanation = initialWorkStart?.querySelector('p')?.textContent || '';
        await click('specifications', '[data-page="work"] .work-start .journey-action');
        const specificationUrl = location.href;
        if (new URL(specificationUrl).searchParams.get('page') !== 'specifications') throw new Error('programmatic Specifications transition did not update URL: ' + specificationUrl);
        const requirementRail = [...document.querySelectorAll('[data-page="specifications"] [data-specifications-rail] .rail-item')]
          .find(node => node.textContent.includes('REQ-CAPABILITY-001') || node.textContent.includes('REQ-FIXTURE-001'));
        if (!requirementRail) throw new Error('Requirement origin fixture is missing from the specifications rail');
        requirementRail.querySelector('.rail-item-select')?.click();
        await new Promise(resolve => setTimeout(resolve, 100));
        const selectedSpecificationTitle = document.querySelector('[data-page="specifications"] [data-specifications-detail] .canvas-head h2')?.textContent || '';
        const selectedCriterion = document.querySelector('[data-page="specifications"] .specification-criterion');
        const selectedCriterionAnchor = selectedCriterion?.querySelector('strong')?.textContent || '';
        const selectedCriterionStatement = selectedCriterion?.querySelector('p')?.textContent || '';
        const createWork = await wait('create', () => document.querySelector('[data-page="specifications"] [data-create-work]')
          || [...requirementRail.querySelectorAll('button')]
            .find(node => node.textContent.includes('Requirement criterion') && !node.disabled));
        createWork.click();
        flow.push('create');
        const workUrl = await wait('Work route after create', () => {
          const candidate = new URL(location.href);
          return candidate.searchParams.get('page') === 'work' && candidate.searchParams.get('workItem')
            ? candidate.href
            : null;
        });
        if (new URL(workUrl).searchParams.get('page') !== 'work' || !new URL(workUrl).searchParams.get('workItem')) throw new Error('Create Work did not update canonical URL: ' + workUrl);
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
          specificationUrl,
          workUrl,
          workSpecificationTitle: document.querySelector('[data-page="work"] [data-work-specification-title]')?.textContent || '',
          workSpecificationCriterion: document.querySelector('[data-page="work"] [data-work-specification-criterion]')?.textContent || '',
          workSpecificationAnchor: document.querySelector('[data-page="work"] [data-work-specification-anchor]')?.dataset.workSpecificationAnchor || '',
          workTitle: document.querySelector('[data-page="work"] .journey-header h2')?.textContent || '',
          workPaneSpecificationCount: document.querySelectorAll('[data-work-overview] [data-work-specification-title], [data-work-overview] [data-work-specification-criterion]').length,
          layoutColumns: getComputedStyle(document.querySelector('[data-work-journey-workspace]')).gridTemplateColumns.split(' ').length,
          errors: window.__MITASE_BROWSER_ERRORS__ || [],
        };
      })()
    `,
  });
  if (evaluation.exceptionDetails) throw new Error(evaluation.exceptionDetails.text || 'browser flow failed');
  const result = { ...evaluation.result.value, mutationResponses };
  const navigateAndWait = async url => {
    const loaded = new Promise(resolve => {
      const handler = () => resolve();
      devtools.on('Page.loadEventFired', handler);
    });
    await devtools.send('Page.navigate', { url });
    await loaded;
    await new Promise(resolve => setTimeout(resolve, 250));
  };
  await navigateAndWait(result.specificationUrl);
  const specificationRestore = await devtools.send('Runtime.evaluate', {
    awaitPromise: true,
    returnByValue: true,
    expression: `(async () => {
      for (let attempt = 0; attempt < 100; attempt += 1) {
        const page = document.querySelector('[data-page]:not([hidden])')?.dataset.page;
        if (page === 'specifications') return { page, url: location.href };
        await new Promise(resolve => setTimeout(resolve, 50));
      }
      return { page: document.querySelector('[data-page]:not([hidden])')?.dataset.page || '', url: location.href };
    })()`,
  });
  await navigateAndWait(result.workUrl);
  const workRestore = await devtools.send('Runtime.evaluate', {
    awaitPromise: true,
    returnByValue: true,
    expression: `(async () => {
      for (let attempt = 0; attempt < 100; attempt += 1) {
        const page = document.querySelector('[data-page]:not([hidden])')?.dataset.page;
        const title = document.querySelector('[data-page="work"] [data-work-specification-title]')?.textContent || '';
        const criterion = document.querySelector('[data-page="work"] [data-work-specification-criterion]')?.textContent || '';
        if (page === 'work' && title && criterion) return { page, title, criterion, url: location.href };
        await new Promise(resolve => setTimeout(resolve, 50));
      }
      return { page: document.querySelector('[data-page]:not([hidden])')?.dataset.page || '', title: '', criterion: '', url: location.href };
    })()`,
  });
  const restoredSpecifications = specificationRestore.result?.value || {};
  const restoredWork = workRestore.result?.value || {};
  if (restoredSpecifications.page !== 'specifications' || new URL(restoredSpecifications.url).searchParams.get('page') !== 'specifications') {
    throw new Error(`Specifications route did not survive reload: ${JSON.stringify(restoredSpecifications)}`);
  }
  if (
    restoredWork.page !== 'work'
    || new URL(restoredWork.url).searchParams.get('page') !== 'work'
    || !new URL(restoredWork.url).searchParams.get('workItem')
    || restoredWork.title !== result.workSpecificationTitle
    || restoredWork.criterion !== result.workSpecificationCriterion
  ) {
    throw new Error(`Work context did not survive reload: ${JSON.stringify({ restoredWork, expectedTitle: result.workSpecificationTitle, expectedCriterion: result.workSpecificationCriterion })}`);
  }
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
    || (result.selectedCriterionStatement && result.workSpecificationCriterion !== result.selectedCriterionStatement)
    || (result.selectedCriterionAnchor && result.workSpecificationAnchor !== result.selectedCriterionAnchor)
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
