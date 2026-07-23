import { translate } from '../i18n.js';

const t = key => translate(key);

export function renderScope(scope, root = document.querySelector('[data-scope-detail]')) {
  if (!root) return;
  root.replaceChildren();
  const branch = scope?.branch;
  if (!branch) {
    const empty = document.createElement('div');
    empty.className = 'context-empty-state';
    const icon = document.createElement('span');
    icon.className = 'context-empty-icon';
    icon.textContent = '↔';
    const title = document.createElement('h2');
    title.textContent = t('scope.empty.title');
    const description = document.createElement('p');
    description.textContent = t('scope.empty.description');
    empty.append(icon, title, description);
    root.append(empty);
    return;
  }
  const summary = document.createElement('p');
  summary.textContent = `${branch.state}: ${(branch.changed || []).length} changed artifact(s)`;
  root.append(summary);
}
