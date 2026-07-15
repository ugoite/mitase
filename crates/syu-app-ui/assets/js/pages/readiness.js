import { renderReadiness } from '../components/readiness.js';

export function renderReadinessPage(readiness, root = document.querySelector('[data-readiness-content]')) {
  if (!root) return;
  root.replaceChildren();
  root.append(renderReadiness(readiness));
  const list = document.createElement('ul');
  for (const [axis, value] of Object.entries(readiness.axes || {})) {
    const row = document.createElement('li');
    row.textContent = `${axis}: ${value.ready}/${value.required}`;
    const blockers = (value.blockers || []).map(blocker => blocker.message || blocker).join('; ');
    if (blockers) row.title = blockers;
    list.append(row);
  }
  root.append(list);
}
