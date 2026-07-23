import { translate } from '../i18n.js';

const t = key => translate(key);

function readinessTone(readiness) {
  if (readiness.execution_state === 'not-run') return ['muted', '○', t('readiness.status.not_run')];
  if (readiness.blocking_subjects > 0 || String(readiness.status).toLowerCase().includes('block')) {
    return ['danger', '!', t('readiness.status.blocked')];
  }
  return ['success', '✓', t('readiness.status.ready')];
}

export function renderReadinessPage(readiness, root = document.querySelector('[data-readiness-content]')) {
  if (!root) return;
  root.replaceChildren();
  const [tone, icon, label] = readinessTone(readiness);
  const summary = document.createElement('section');
  summary.className = `readiness-summary ${tone}`;
  const iconNode = document.createElement('span');
  iconNode.className = 'readiness-icon';
  iconNode.setAttribute('aria-hidden', 'true');
  iconNode.textContent = icon;
  const copy = document.createElement('div');
  const heading = document.createElement('h2');
  heading.textContent = label;
  const description = document.createElement('p');
  description.textContent = readiness.execution_state === 'not-run'
    ? t('readiness.description.not_run')
    : t('readiness.description.complete');
  copy.append(heading, description);
  summary.append(iconNode, copy);
  root.append(summary);

  const axes = document.createElement('div');
  axes.className = 'readiness-axes';
  for (const [axis, value] of Object.entries(readiness.axes || {})) {
    const row = document.createElement('section');
    row.className = `readiness-axis ${value.ready >= value.required ? 'success' : 'warning'}`;
    const name = document.createElement('strong');
    name.textContent = axis;
    const count = document.createElement('span');
    count.className = 'chip';
    count.textContent = `${value.ready}/${value.required}`;
    row.append(name, count);
    const blockers = (value.blockers || []).map(blocker => blocker.message || blocker).join('; ');
    if (blockers) row.title = blockers;
    axes.append(row);
  }
  if (axes.childElementCount) root.append(axes);
}

export function initReadiness(state) {
  const run = document.querySelector('[data-readiness-run]');
  run?.addEventListener('click', () => state.runAction(async () => {
    const readiness = await state.api.runReadiness();
    state.projection.readiness = readiness;
    return state.projection;
  }));
}
