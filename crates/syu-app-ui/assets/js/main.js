import { createState } from './state.js';
import { bindRouter } from './router.js';
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
  renderSpecifications(projection.specifications);
  renderDiagnostics(projection.diagnostics);
  renderSettings(projection);
}

export async function startWorkbench() {
  const node = document.querySelector('#syu-projection');
  if (!node) throw new Error('canonical Workbench projection is missing');
  const state = createState(JSON.parse(node.textContent));
  state.api = api;
  bindRouter(state, () => render(state));
  render(state);
  return state;
}

if (typeof window !== 'undefined') {
  startWorkbench().catch((error) => {
    window.dispatchEvent(new CustomEvent('syu-workbench-error', { detail: error }));
  });
}
