import { renderDiagnostic } from '../components/diagnostic.js';
import { translate } from '../i18n.js';

const t = key => translate(key);

export function renderDiagnostics(diagnostics, root = document.querySelector('[data-diagnostic-result]')) {
  if (!root) return;
  root.replaceChildren();
  const diagnosticsList = diagnostics?.validation?.diagnostics || [];
  if (!diagnosticsList.length) {
    const notRun = diagnostics?.validation?.state === 'not_run';
    const empty = document.createElement('div');
    empty.className = `context-empty-state ${notRun ? 'muted' : 'success'}`;
    const icon = document.createElement('span');
    icon.className = 'context-empty-icon';
    icon.textContent = notRun ? '○' : '✓';
    const title = document.createElement('h2');
    title.textContent = notRun ? t('diagnostics.not_run.title') : t('diagnostics.zero.title');
    const description = document.createElement('p');
    description.textContent = notRun ? t('diagnostics.not_run.description') : t('diagnostics.zero.description');
    empty.append(icon, title, description);
    root.append(empty);
    return;
  }
  const list = document.createElement('ul');
  diagnosticsList.forEach(diagnostic => list.append(renderDiagnostic(diagnostic)));
  root.append(list);
}
