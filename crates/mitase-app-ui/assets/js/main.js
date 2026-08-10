import { createState, replaceProjection } from './state.js';
import { bindRouter, navigate, syncPageLocation } from './router.js';
import * as api from './api.js';
import { renderWork } from './pages/work.js';
import { initReadiness, renderReadinessPage } from './pages/readiness.js';
import { initScope, renderScope } from './pages/scope.js';
import { initSpecifications, renderSpecifications } from './pages/specifications.js';
import { initDiagnostics, renderDiagnostics } from './pages/diagnostics.js';
import { renderSettings } from './pages/settings.js';
import { translate } from './i18n.js';

function render(state) {
  const projection = state.projection;
  const renderPage = {
    work: () => renderWork(projection.work, state),
    readiness: () => renderReadinessPage(projection.readiness, undefined, { state }),
    scope: () => renderScope(projection.scope, state),
    specifications: () => renderSpecifications(projection.specifications, state),
    diagnostics: () => renderDiagnostics(projection.diagnostics, undefined, state),
    settings: () => renderSettings(projection),
  }[state.selectedPage] || (() => renderWork(projection.work, state));
  renderPage();
  renderWorkspaceIdentity(projection);
  renderBusyState(state);
}

async function refreshAfterAction(state, action, onResult, busyLabel) {
  if (state.busy) return;
  state.error = null;
  state.busy = true;
  state.busyLabel = busyLabel || translate('common.loading');
  state.render();
  try {
    const result = await action();
    const returnedProjection = result?.schema === 'mitase/work-select-slice-response/v1'
      ? result.projection
      : result;
    const nextProjection = returnedProjection?.snapshot && returnedProjection?.work
      ? returnedProjection
      : await api.readProjection();
    Object.assign(state, replaceProjection(state, nextProjection));
    onResult?.(result);
  } catch (error) {
    state.error = error;
  }
  state.busy = false;
  state.busyLabel = '';
  state.render();
}

function readInlineProjection() {
  const node = document.querySelector('#mitase-projection');
  if (!node?.textContent?.trim()) return null;
  try { return JSON.parse(node.textContent); } catch { return null; }
}

function renderWorkspaceIdentity(projection) {
  const snapshot = projection?.snapshot || {};
  const parts = String(snapshot.root || '').split('/').filter(Boolean);
  const workspace = parts.at(-1) || 'Mitase';
  const revision = String(snapshot.revision || '').slice(0, 8) || 'unknown';
  document.querySelectorAll('[data-workspace-name]').forEach(node => { node.textContent = workspace; });
  document.querySelectorAll('[data-workspace-revision]').forEach(node => {
    node.textContent = translate('workspace.revision').replace('{revision}', revision);
  });
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
    status.setAttribute('aria-label', state.busyLabel || translate('common.loading'));
    const label = status.querySelector('[data-progress-label]');
    if (label) label.textContent = state.busyLabel || translate('common.loading');
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
      startupStatus.setAttribute('aria-label', translate('common.starting'));
      const label = startupStatus.querySelector('[data-progress-label]');
      if (label) label.textContent = translate('common.starting');
    }
  }
  const projection = inlineProjection || await api.readProjection();
  const state = createState(projection);
  state.api = api;
  state.render = () => {
    if (!state.busy) restoreBusyButtons();
    render(state);
  };
  document.addEventListener('mitase:locale', () => state.render());
  initSpecifications(state);
  initReadiness(state);
  initDiagnostics(state);
  initScope(state);
  state.go = (page, routeState = {}) => {
    if (page === 'work' && Object.prototype.hasOwnProperty.call(routeState, 'workItem')) {
      state.journeyRouteItemId = routeState.workItem;
      state.journeyContextItemId = routeState.workItem;
      state.journeySpecificationAnchor = null;
    }
    if (page === 'specifications' && Object.prototype.hasOwnProperty.call(routeState, 'item')) {
      state.selectedSpecification = routeState.item;
    }
    state.selectedPage = navigate(page, false);
    syncPageLocation(state, state.selectedPage, routeState);
    state.render();
  };
  state.runAction = (action, onResult, busyLabel) => refreshAfterAction(state, action, onResult, busyLabel);
  bindRouter(state, page => {
    if (page === 'specifications') {
      state.specificationSourceTarget = null;
      state.specificationSource = null;
      state.specificationSourceFull = false;
    }
    state.render();
  });
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
    window.dispatchEvent(new CustomEvent('mitase-workbench-error', { detail: error }));
  });
}
