import { createState } from './state.js';
import { bindRouter, navigate } from './router.js';
import * as api from './api.js';
import { renderWork } from './pages/work.js';
import { renderReadinessPage } from './pages/readiness.js';
import { renderScope } from './pages/scope.js';
import { renderSpecifications } from './pages/specifications.js';
import { renderDiagnostics } from './pages/diagnostics.js';
import { renderSettings } from './pages/settings.js';

function render(state) {
  const projection = state.projection;
  renderWork(projection.work, state);
  renderReadinessPage(projection.readiness);
  renderScope(projection.scope);
  renderSpecifications(projection.specifications, state);
  renderDiagnostics(projection.diagnostics);
  renderSettings(projection);
}

async function refreshAfterAction(state, action, onResult) {
  state.error = null;
  try {
    const result = await action();
    onResult?.(result);
    state.projection = await api.readProjection();
  } catch (error) {
    state.error = error;
  }
  state.render();
}

export async function startWorkbench() {
  // The initial GET establishes the server-owned CSRF token. The inline
  // projection remains useful for the first paint, but must not be the only
  // source of browser state because mutations are guarded by the server.
  const projection = await api.readProjection();
  const state = createState(projection);
  state.api = api;
  state.render = () => render(state);
  state.go = (page) => {
    state.selectedPage = navigate(page, false);
    state.render();
  };
  state.runAction = (action, onResult) => refreshAfterAction(state, action, onResult);
  bindRouter(state, () => render(state));
  render(state);
  return state;
}

if (typeof window !== 'undefined') {
  startWorkbench().catch((error) => {
    window.dispatchEvent(new CustomEvent('syu-workbench-error', { detail: error }));
  });
}
