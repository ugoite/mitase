import { renderDiagnostic } from '../components/diagnostic.js';

export function renderDiagnostics(diagnostics, root = document.querySelector('[data-diagnostic-result]')) {
  if (!root) return;
  root.replaceChildren();
  const diagnosticsList = diagnostics?.validation?.diagnostics || [];
  if (!diagnosticsList.length) {
    root.textContent = diagnostics?.validation?.state === 'not_run' ? 'Validation has not run.' : 'No diagnostics.';
    return;
  }
  const list = document.createElement('ul');
  diagnosticsList.forEach(diagnostic => list.append(renderDiagnostic(diagnostic)));
  root.append(list);
}
