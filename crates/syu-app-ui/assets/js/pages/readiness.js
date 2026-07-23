import { translate } from '../i18n.js';

const t = key => translate(key);

function element(tag, className, text) {
  const node = document.createElement(tag);
  if (className) node.className = className;
  if (text !== undefined && text !== null) node.textContent = text;
  return node;
}

function readinessTone(readiness) {
  if (readiness.execution_state === 'not-run') return ['muted', '○', t('readiness.status.not_run')];
  if (readiness.blocking_subjects > 0 || String(readiness.status).toLowerCase().includes('block')) {
    return ['danger', '!', t('readiness.status.blocked')];
  }
  return ['success', '✓', t('readiness.status.ready')];
}

function axisLabel(axis) {
  const translated = t(`readiness.axis.${axis}`);
  return translated === `readiness.axis.${axis}`
    ? axis.replaceAll('_', ' ')
    : translated;
}

function controlButton(label, value, current, count, onClick, tone = '') {
  const button = element('button', `insight-filter${current === value ? ' active' : ''}${tone ? ` ${tone}` : ''}`);
  button.type = 'button';
  button.setAttribute('aria-pressed', String(current === value));
  button.append(element('span', null, label), element('b', null, String(count)));
  button.addEventListener('click', onClick);
  return button;
}

export function renderReadinessPage(
  readiness,
  root = document.querySelector('[data-readiness-content]'),
  options = {},
) {
  if (!root) return;
  root.replaceChildren();
  const state = options.state || {
    readinessFilter: 'all',
    readinessSort: 'attention',
    render: () => {},
  };
  const [tone, icon, label] = readinessTone(readiness);
  const summary = element('section', `insight-summary ${tone}`);
  const iconNode = element('span', 'insight-status-icon', icon);
  iconNode.setAttribute('aria-hidden', 'true');
  const copy = element('div', 'insight-summary-copy');
  copy.append(
    element('h2', null, label),
    element(
      'p',
      null,
      readiness.execution_state === 'not-run'
        ? t('readiness.description.not_run')
        : t('readiness.description.complete'),
    ),
  );
  const target = element('span', 'chip insight-target', readiness.target);
  target.setAttribute('aria-label', `${t('readiness.target')}: ${readiness.target}`);
  summary.append(iconNode, copy, target);
  root.append(summary);

  const entries = Object.entries(readiness.axes || {}).map(([axis, value]) => ({
    axis,
    value,
    ready: value.ready >= value.required,
  }));
  if (!entries.length) return;

  const controls = element('div', `insight-filterbar${options.compact ? ' compact' : ''}`);
  const group = element('div', 'insight-filter-group');
  group.append(
    controlButton(t('filter.all'), 'all', state.readinessFilter, entries.length, () => {
      state.readinessFilter = 'all';
      state.render();
    }),
    controlButton(t('readiness.filter.attention'), 'attention', state.readinessFilter, entries.filter(entry => !entry.ready).length, () => {
      state.readinessFilter = 'attention';
      state.render();
    }, 'danger'),
    controlButton(t('readiness.filter.ready'), 'ready', state.readinessFilter, entries.filter(entry => entry.ready).length, () => {
      state.readinessFilter = 'ready';
      state.render();
    }, 'success'),
  );
  const sort = element('select', 'native-select insight-sort');
  sort.setAttribute('aria-label', t('common.sort'));
  [
    ['attention', t('readiness.sort.attention')],
    ['name', t('readiness.sort.name')],
    ['coverage', t('readiness.sort.coverage')],
  ].forEach(([value, text]) => {
    const option = element('option', null, text);
    option.value = value;
    sort.append(option);
  });
  sort.value = state.readinessSort;
  sort.addEventListener('change', () => {
    state.readinessSort = sort.value;
    state.render();
  });
  controls.append(group, sort);
  root.append(controls);

  const visible = entries
    .filter(entry => state.readinessFilter === 'all'
      || (state.readinessFilter === 'ready' ? entry.ready : !entry.ready))
    .sort((left, right) => {
      if (state.readinessSort === 'name') return axisLabel(left.axis).localeCompare(axisLabel(right.axis));
      if (state.readinessSort === 'coverage') {
        const leftCoverage = left.value.required ? left.value.ready / left.value.required : 1;
        const rightCoverage = right.value.required ? right.value.ready / right.value.required : 1;
        return leftCoverage - rightCoverage;
      }
      return Number(left.ready) - Number(right.ready)
        || axisLabel(left.axis).localeCompare(axisLabel(right.axis));
    });
  const list = element('div', 'readiness-list');
  visible.forEach(({ axis, value, ready }) => {
    const row = element('details', `readiness-row ${ready ? 'success' : 'danger'}`);
    const rowSummary = element('summary');
    const marker = element('span', 'status-marker', ready ? '✓' : '!');
    marker.setAttribute('aria-hidden', 'true');
    rowSummary.append(
      marker,
      element('strong', null, axisLabel(axis)),
      element('span', `chip ${ready ? 'green-chip' : 'red-chip'}`, `${value.ready}/${value.required}`),
    );
    row.append(rowSummary);
    const blockers = value.blockers || [];
    const detail = element('div', 'readiness-row-detail');
    if (blockers.length) {
      const listNode = element('ul', 'readiness-blockers');
      blockers.forEach(blocker => {
        const valueNode = blocker.message || blocker;
        listNode.append(element('li', null, valueNode));
      });
      detail.append(listNode);
    } else {
      detail.append(element('p', null, t('readiness.axis.ready')));
    }
    row.append(detail);
    list.append(row);
  });
  root.append(list);
}

export function initReadiness(state) {
  const run = document.querySelector('[data-readiness-run]');
  run?.addEventListener('click', () => state.runAction(async () => {
    const readiness = await state.api.runReadiness();
    state.projection.readiness = readiness;
    return state.projection;
  }, null, t('readiness.run')));
}
