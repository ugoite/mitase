import { createState } from './state.js';
import { bindRouter, navigate } from './router.js';
import * as api from './api.js';
import { renderWork } from './pages/work.js';
import { renderReadinessPage } from './pages/readiness.js';
import { renderScope } from './pages/scope.js';
import { initSpecifications, renderSpecifications } from './pages/specifications.js';
import { renderDiagnostics } from './pages/diagnostics.js';
import { renderSettings } from './pages/settings.js';
import { translate } from './i18n.js';

function render(state) {
  const projection = state.projection;
  const renderPage = {
    work: () => renderWork(projection.work, state),
    readiness: () => renderReadinessPage(projection.readiness),
    scope: () => renderScope(projection.scope),
    specifications: () => renderSpecifications(projection.specifications, state),
    diagnostics: () => renderDiagnostics(projection.diagnostics),
    settings: () => renderSettings(projection),
  }[state.selectedPage] || (() => renderWork(projection.work, state));
  renderPage();
  renderWorkspaceIdentity(projection);
  renderBusyState(state);
}

async function refreshAfterAction(state, action, onResult) {
  if (state.busy) return;
  state.error = null;
  state.busy = true;
  state.busyLabel = translate('common.loading');
  state.render();
  try {
    const result = await action();
    state.projection = result?.snapshot && result?.work
      ? result
      : await api.readProjection();
    onResult?.(result);
  } catch (error) {
    state.error = error;
  }
  state.busy = false;
  state.busyLabel = '';
  state.render();
}

function readInlineProjection() {
  const node = document.querySelector('#syu-projection');
  if (!node?.textContent?.trim()) return null;
  try { return JSON.parse(node.textContent); } catch { return null; }
}

function renderWorkspaceIdentity(projection) {
  const snapshot = projection?.snapshot || {};
  const parts = String(snapshot.root || '').split('/').filter(Boolean);
  const workspace = parts.at(-1) || 'Syu';
  const revision = String(snapshot.revision || '').slice(0, 8) || 'unknown';
  document.querySelectorAll('[data-workspace-name]').forEach(node => { node.textContent = workspace; });
  document.querySelectorAll('[data-workspace-revision]').forEach(node => { node.textContent = `revision ${revision}`; });
  document.querySelectorAll('[data-workspace-branch]').forEach(node => { node.textContent = `@ ${revision}`; });
}

function disableBusyButtons() {
  document.querySelectorAll('button').forEach(button => {
    if (!Object.prototype.hasOwnProperty.call(button.dataset, 'busyDisabled')) {
      button.dataset.busyDisabled = String(button.disabled);
    }
    button.disabled = true;
  });
}

function renderBusyState(state) {
  document.body.setAttribute('aria-busy', String(state.busy));
  const status = document.querySelector('[data-workbench-status]');
  if (status) {
    status.hidden = !state.busy;
    status.textContent = state.busyLabel || translate('common.loading');
  }
  if (state.busy) {
    disableBusyButtons();
  } else {
    document.querySelectorAll('[data-busy-disabled]').forEach(button => {
      delete button.dataset.busyDisabled;
    });
  }
}

function restoreBusyButtons() {
  document.querySelectorAll('[data-busy-disabled]').forEach(button => {
    button.disabled = button.dataset.busyDisabled === 'true';
    delete button.dataset.busyDisabled;
  });
}

export async function startWorkbench() {
  const inlineProjection = readInlineProjection();
  if (!inlineProjection) {
    document.body.setAttribute('aria-busy', 'true');
    disableBusyButtons();
    const startupStatus = document.querySelector('[data-workbench-status]');
    if (startupStatus) {
      startupStatus.hidden = false;
      startupStatus.textContent = translate('common.starting');
    }
  }
  const projection = inlineProjection || await api.readProjection();
  const state = createState(projection);
  state.api = api;
  state.render = () => {
    if (!state.busy) restoreBusyButtons();
    render(state);
  };
  initSpecifications(state);
  state.go = (page) => {
    state.selectedPage = navigate(page, false);
    state.render();
  };
  state.runAction = (action, onResult) => refreshAfterAction(state, action, onResult);
  bindRouter(state, () => state.render());
  state.busy = true;
  state.busyLabel = translate('common.starting');
  state.render();
  if (inlineProjection) await api.establishSession();
  state.busy = false;
  state.busyLabel = '';
  state.render();
  return state;
}

if (typeof window !== 'undefined') {
  startWorkbench().catch((error) => {
    window.dispatchEvent(new CustomEvent('syu-workbench-error', { detail: error }));
  });
}
