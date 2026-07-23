import { renderDiff } from '../components/diff.js';
import { renderDiagnostics } from './diagnostics.js';
import { translate } from '../i18n.js';

const t = key => translate(key);

function element(tag, className, text) {
  const node = document.createElement(tag);
  if (className) node.className = className;
  if (text !== undefined && text !== null) node.textContent = text;
  return node;
}

function emptyState(icon, title, description, tone = '') {
  const empty = element('div', `context-empty-state${tone ? ` ${tone}` : ''}`);
  const marker = element('span', 'context-empty-icon', icon);
  marker.setAttribute('aria-hidden', 'true');
  empty.append(marker, element('h2', null, title), element('p', null, description));
  return empty;
}

function translatedStatus(status) {
  const key = `status.${String(status || 'unknown').toLowerCase()}`;
  const value = t(key);
  return value === key ? status : value;
}

function renderFlow(root, steps) {
  const flow = element('ol', 'scope-flow');
  steps.forEach((step, index) => {
    const item = element('li', `scope-flow-step ${step.tone || 'muted'}`);
    item.append(
      element('span', 'scope-flow-icon', step.icon),
      element('strong', null, step.label),
    );
    if (index < steps.length - 1) item.append(element('span', 'scope-flow-arrow', '→'));
    flow.append(item);
  });
  root.append(flow);
}

function selectedBranchChange(branch, state) {
  const changed = branch?.changed || [];
  return changed.find(item => item.path === state.selectedScopePath) || changed[0] || null;
}

function renderPlanRail(rail, work, state) {
  const slices = work?.plan?.slices || [];
  slices.forEach((slice, index) => {
    const button = element('button', `rail-item${slice.id === state.selectedSlice ? ' active' : ''}`);
    button.type = 'button';
    const copy = element('div');
    copy.append(
      element('b', null, t('journey.scope.step').replace('{number}', String(index + 1))),
      element('p', null, `${slice.editable_targets?.length || 0} ${t('journey.targets')}`),
    );
    button.append(copy);
    button.addEventListener('click', () => {
      state.selectedSlice = slice.id;
      state.render();
    });
    rail.append(button);
  });
}

function renderBranchChange(root, branch, state) {
  const changed = branch?.changed || [];
  if (!changed.length) {
    root.append(emptyState('↔', t('scope.branch.empty.title'), t('scope.branch.empty.description')));
    return;
  }
  const blocked = (branch.unowned || []).length > 0;
  const head = element('section', `scope-change-summary ${blocked ? 'danger' : 'success'}`);
  const copy = element('div');
  copy.append(element('h2', null, t('scope.branch.summary')));
  const chips = element('div', 'meta-line');
  chips.append(
    element('span', 'chip', `${changed.length} ${t('scope.branch.files')}`),
    element('span', 'chip green-chip', `${branch.owned?.length || 0} ${t('scope.branch.owned')}`),
    element('span', `chip ${blocked ? 'red-chip' : 'green-chip'}`, `${branch.unowned?.length || 0} ${t('scope.branch.unowned')}`),
  );
  copy.append(chips);
  head.append(element('span', `scope-change-icon${blocked ? '' : ' success'}`, blocked ? '!' : '✓'), copy);
  root.append(head);

  if (blocked) {
    const explanation = element('section', 'scope-visual-explanation danger');
    renderFlow(explanation, [
      { icon: '±', label: t('scope.visual.change'), tone: 'active' },
      { icon: '×', label: t('scope.visual.owner_missing'), tone: 'danger' },
      { icon: '○', label: t('scope.visual.work_blocked'), tone: 'muted' },
    ]);
    explanation.append(element('p', null, t('scope.branch.unowned_description')));
    root.append(explanation);
  }
  const annotations = Object.fromEntries(changed.map(change => [
    change.path,
    {
      label: change.owners?.length ? t('scope.branch.owned') : t('scope.branch.unowned'),
      tone: change.owners?.length ? 'green-chip' : 'red-chip',
    },
  ]));
  const diffRoot = element('section', 'scope-diff');
  renderDiff(state.scopeDiff, diffRoot, {
    loading: state.scopeDiffLoading,
    error: state.scopeDiffError,
    openFirst: true,
    annotations,
  });
  root.append(diffRoot);
}

function renderPlanChange(root, work, state) {
  const slices = work?.plan?.slices || [];
  const selected = slices.find(slice => slice.id === state.selectedSlice) || slices[0];
  if (!selected) {
    root.append(emptyState('↔', t('scope.empty.title'), t('scope.empty.description')));
    return;
  }
  const card = element('section', 'scope-detail-card');
  card.append(element('h2', null, t('scope.exact_targets')));
  (selected.editable_targets || []).forEach(target => {
    const row = element('div', 'scope-target-row');
    row.append(
      element('span', 'scope-change-icon success', '↔'),
      element('strong', null, target.path),
      element('span', 'chip blue-chip', t('work.context.editable')),
    );
    card.append(row);
  });
  root.append(card);
  const diffRoot = element('section', 'scope-diff');
  renderDiff(state.scopeDiff, diffRoot, {
    loading: state.scopeDiffLoading,
    error: state.scopeDiffError,
    openFirst: true,
  });
  root.append(diffRoot);
}

function renderVerification(root, work, state) {
  renderDiagnostics(
    { validation: work?.validation },
    root,
    state,
    { compact: false, completion: work?.completion, planDigest: work?.plan?.digest },
  );
}

function renderReference(root, branch, work, state) {
  const items = state.selectedScopeMode === 'branch' ? branch?.affected_items || [] : [];
  if (!items.length && !work?.context_pack) {
    root.append(emptyState('◇', t('scope.reference.empty.title'), t('scope.reference.empty.description')));
    return;
  }
  if (work?.context_pack) {
    const context = element('section', 'scope-detail-card');
    context.append(
      element('h2', null, t('work.context.title')),
      element('span', 'chip green-chip', t('work.context.ready')),
      element('p', null, `${work.context_pack.entry_count} ${t('common.items_count').replace('{count}', '')}`.trim()),
    );
    root.append(context);
  }
  items.forEach(item => {
    const card = element('section', 'scope-detail-card');
    card.append(
      element('span', `chip status-${item.status || 'planned'}`, translatedStatus(item.status)),
      element('h2', null, item.title),
      element('p', null, item.summary || item.description || ''),
    );
    root.append(card);
  });
}

function renderIntent(root, branch, work, state) {
  const request = work?.request;
  const blocked = state.selectedScopeMode === 'branch' && branch?.state === 'blocked';
  const card = element('section', `scope-visual-explanation ${blocked ? 'danger' : 'success'}`);
  renderFlow(card, [
    { icon: '◎', label: t('scope.visual.intent'), tone: 'active' },
    { icon: '↔', label: t('scope.visual.scope'), tone: blocked ? 'danger' : 'success' },
    { icon: blocked ? '×' : '✓', label: blocked ? t('scope.visual.review') : t('scope.visual.implement'), tone: blocked ? 'muted' : 'success' },
  ]);
  card.append(element(
    'p',
    null,
    request?.summary || branch?.reason || t('scope.intent.empty'),
  ));
  root.append(card);
}

export function renderScope(
  scope,
  stateOrRoot = document.querySelector('[data-scope-detail]'),
) {
  const state = stateOrRoot?.api ? stateOrRoot : null;
  const root = state ? document.querySelector('[data-scope-detail]') : stateOrRoot;
  const rail = document.querySelector('[data-scope-rail]');
  if (!root) return;
  root.replaceChildren();
  rail?.replaceChildren();
  if (!state) {
    root.append(emptyState('↔', t('scope.empty.title'), t('scope.empty.description')));
    return;
  }
  if (!state.scopeDiff && !state.scopeDiffLoading && !state.scopeDiffError) {
    queueMicrotask(() => loadDiff(state));
  }
  const branch = scope?.branch;
  const work = state.projection.work;
  if (state.scopeLoading) {
    root.append(emptyState('↻', t('scope.branch.loading.title'), t('scope.branch.loading.description')));
    return;
  }
  if (state.scopeError) {
    const error = element('details', 'diagnostic-card error');
    const summary = element('summary', 'diagnostic-card-summary');
    summary.append(
      element('span', 'diagnostic-marker', '!'),
      element('strong', null, t('scope.error.title')),
      element('span', 'chip severity-error', t('diagnostics.severity.error')),
    );
    error.append(summary, element('div', 'diagnostic-card-detail', state.scopeError));
    root.append(error);
    return;
  }
  if (rail) {
    rail.hidden = state.selectedScopeMode === 'branch';
    rail.parentElement?.classList.toggle('no-rail', state.selectedScopeMode === 'branch');
  }
  if (state.selectedScopeMode !== 'branch') renderPlanRail(rail, work, state);

  if (state.selectedScopeTab === 'verify') renderVerification(root, work, state);
  else if (state.selectedScopeTab === 'reference') renderReference(root, branch, work, state);
  else if (state.selectedScopeTab === 'intent') renderIntent(root, branch, work, state);
  else if (state.selectedScopeMode === 'branch') renderBranchChange(root, branch, state);
  else renderPlanChange(root, work, state);
}

async function loadBranch(state, range = '') {
  if (state.scopeLoading) return;
  state.scopeLoading = true;
  state.scopeError = null;
  state.scopeDiffLoading = true;
  state.scopeDiffError = null;
  state.render();
  const [scopeResult, diffResult] = await Promise.allSettled([
    state.api.readBranchScope(range),
    state.api.readScopeDiff(range),
  ]);
  if (scopeResult.status === 'fulfilled') {
    state.projection.scope = scopeResult.value;
    state.scopeRange = scopeResult.value?.branch?.range || range;
    state.selectedScopePath = scopeResult.value?.branch?.changed?.[0]?.path || null;
  } else {
    state.scopeError = scopeResult.reason?.message || String(scopeResult.reason);
  }
  if (diffResult.status === 'fulfilled') state.scopeDiff = diffResult.value;
  else state.scopeDiffError = diffResult.reason?.message || String(diffResult.reason);
  state.scopeLoading = false;
  state.scopeDiffLoading = false;
  state.render();
}

async function loadDiff(state) {
  if (state.scopeDiffLoading || state.scopeDiff) return;
  state.scopeDiffLoading = true;
  state.scopeDiffError = null;
  state.render();
  try {
    state.scopeDiff = await state.api.readScopeDiff(state.scopeRange);
  } catch (error) {
    state.scopeDiffError = error.message;
  }
  state.scopeDiffLoading = false;
  state.render();
}

export function initScope(state) {
  const range = document.querySelector('[data-scope-range]');
  const planControl = document.querySelector('[data-scope-plan-control]');
  const rangeControl = document.querySelector('[data-scope-range-control]');
  const syncMode = mode => {
    state.selectedScopeMode = mode;
    document.querySelectorAll('[data-scope-mode-button]').forEach(button => {
      button.classList.toggle('active', button.dataset.scopeModeButton === mode);
      button.setAttribute('aria-pressed', String(button.dataset.scopeModeButton === mode));
    });
    if (planControl) planControl.hidden = mode !== 'plan';
    if (rangeControl) rangeControl.hidden = mode !== 'branch';
  };
  document.querySelectorAll('[data-scope-mode-button]').forEach(button => {
    button.addEventListener('click', () => {
      syncMode(button.dataset.scopeModeButton);
      if (state.selectedScopeMode === 'branch') loadBranch(state, range?.value || state.scopeRange);
      else {
        loadDiff(state);
        state.render();
      }
    });
  });
  document.querySelectorAll('[data-tab-group="scope"]').forEach(tab => {
    tab.addEventListener('click', () => {
      state.selectedScopeTab = tab.dataset.tab || 'change';
      state.render();
    });
  });
  range?.addEventListener('input', () => { state.scopeRange = range.value; });
  range?.addEventListener('keydown', event => {
    if (event.key !== 'Enter') return;
    event.preventDefault();
    state.scopeDiff = null;
    loadBranch(state, range.value);
  });
  document.querySelector('[data-scope-refresh]')?.addEventListener('click', () => {
    state.scopeDiff = null;
    if (state.selectedScopeMode === 'branch') loadBranch(state, range?.value || state.scopeRange);
    else loadDiff(state);
  });
  document.querySelector('[data-scope-create-work]')?.addEventListener('click', () => {
    const branch = state.projection.scope?.branch;
    const selected = selectedBranchChange(branch, state);
    const anchor = selected?.anchors?.[0];
    if (!anchor) {
      state.scopeError = t('scope.branch.no_anchor');
      state.render();
      return;
    }
    state.runAction(
      () => state.api.runJourneyAction(state.projection, {
        action: 'create',
        anchor,
        summary: t('scope.branch.work_summary').replace('{path}', selected.path),
      }),
      () => state.go('work'),
      t('common.plan'),
    );
  });
  syncMode(state.selectedScopeMode);
}
