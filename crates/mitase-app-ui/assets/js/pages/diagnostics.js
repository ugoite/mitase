import { renderDiagnostic } from '../components/diagnostic.js';
import { translate } from '../i18n.js';

const t = key => translate(key);
const severityRank = { error: 0, warning: 1, info: 2 };

function element(tag, className, text) {
  const node = document.createElement(tag);
  if (className) node.className = className;
  if (text !== undefined && text !== null) node.textContent = text;
  return node;
}

function stateTone(state) {
  if (state === 'passed') return ['success', '✓', t('diagnostics.passed')];
  if (state === 'issues') return ['danger', '!', t('diagnostics.issues_found')];
  if (state === 'failed') return ['danger', '×', t('diagnostics.failed')];
  if (state === 'not_applicable') return ['muted', '—', t('diagnostics.not_applicable')];
  return ['muted', '○', t('diagnostics.not_run.title')];
}

function formatDuration(duration) {
  if (duration === null || duration === undefined) return null;
  return duration < 1000 ? `${duration} ms` : `${(duration / 1000).toFixed(1)} s`;
}

function contextLabel(context) {
  const normalized = String(context || 'workspace').replaceAll('-', '_');
  const key = `diagnostics.context.${normalized}`;
  const translated = t(key);
  return translated === key ? String(context || 'workspace').replaceAll('_', ' ') : translated;
}

function blockerTitle(blocker) {
  if (document.documentElement.lang !== 'ja') return blocker.message;
  return {
    'MITASE-COMPLETION-CHECK': t('completion.blocker.check'),
    'MITASE-WORK-011': t('completion.blocker.unchanged'),
  }[blocker.code] || t('completion.blocker.default');
}

function filterButton(label, value, current, count, onClick, tone = '') {
  const button = element('button', `insight-filter${current === value ? ' active' : ''}${tone ? ` ${tone}` : ''}`);
  button.type = 'button';
  button.setAttribute('aria-pressed', String(current === value));
  button.append(element('span', null, label));
  if (count !== null) button.append(element('b', null, String(count)));
  button.addEventListener('click', onClick);
  return button;
}

function renderFilters(validation, state, compact) {
  const host = compact
    ? element('div', 'insight-filterbar compact')
    : document.querySelector('[data-diagnostics-filters]');
  if (!host) return null;
  host.replaceChildren();
  const diagnostics = validation.diagnostics || [];
  const phases = [{ id: 'all', issue_count: diagnostics.length }, ...(validation.phases || [])];
  const phaseGroup = element('div', 'insight-filter-group');
  phases.forEach(phase => {
    phaseGroup.append(filterButton(
      t(`diagnostics.phase.${phase.id}`),
      phase.id,
      state.selectedDiagnosticPhase,
      phase.issue_count,
      () => {
        state.selectedDiagnosticPhase = phase.id;
        state.render();
      },
      phase.state === 'passed' ? 'success' : phase.issue_count ? 'danger' : '',
    ));
  });

  const severityGroup = element('div', 'insight-filter-group');
  ['all', 'error', 'warning', 'info'].forEach(severity => {
    const count = severity === 'all'
      ? diagnostics.length
      : diagnostics.filter(item => item.severity === severity).length;
    severityGroup.append(filterButton(
      severity === 'all' ? t('diagnostics.severity.all') : t(`diagnostics.severity.${severity}`),
      severity,
      state.selectedDiagnosticSeverity,
      count,
      () => {
        state.selectedDiagnosticSeverity = severity;
        state.render();
      },
      severity === 'error' ? 'danger' : severity === 'warning' ? 'warning' : '',
    ));
  });

  const sort = element('select', 'native-select insight-sort');
  sort.setAttribute('aria-label', t('common.sort'));
  [
    ['severity', t('diagnostics.sort.severity')],
    ['location', t('diagnostics.sort.location')],
    ['rule', t('diagnostics.sort.rule')],
  ].forEach(([value, label]) => {
    const option = element('option', null, label);
    option.value = value;
    sort.append(option);
  });
  sort.value = state.diagnosticSort;
  sort.addEventListener('change', () => {
    state.diagnosticSort = sort.value;
    state.render();
  });
  host.append(phaseGroup, severityGroup, sort);
  return host;
}

function renderCompletion(root, completion, planDigest) {
  const attempts = [completion?.current, ...(completion?.previous || [])]
    .filter(attempt => attempt && (!planDigest || attempt.plan_digest === planDigest));
  if (!attempts.length) return;
  const section = element('section', 'completion-evidence');
  section.append(element('h2', 'insight-section-title', t('diagnostics.completion')));
  attempts.forEach(attempt => {
    const card = element('details', `completion-card ${attempt.status}`);
    const summary = element('summary');
    summary.append(
      element('span', 'status-marker', attempt.status === 'complete' ? '✓' : '!'),
      element('strong', null, t(`journey.status.${attempt.status}`)),
      element('code', 'chip', attempt.slice_id),
    );
    card.append(summary);
    const body = element('div', 'completion-card-body');
    if (attempt.demonstrated?.length) {
      const list = element('ul', 'diagnostic-evidence');
      attempt.demonstrated.forEach(anchor => list.append(element('li', null, anchor)));
      body.append(element('h3', null, t('journey.advanced.demonstrated')), list);
    }
    (attempt.blockers || []).forEach(blocker => {
      const item = element('details', 'journey-blocker-detail');
      const summary = element('summary');
      summary.append(
        element('span', 'status-marker', '!'),
        element('strong', null, blockerTitle(blocker)),
        element('code', 'chip diagnostic-rule', blocker.code),
      );
      const detail = element('div', 'journey-blocker');
      if (document.documentElement.lang === 'ja') {
        const technical = element('details', 'diagnostic-technical');
        technical.append(
          element('summary', null, t('diagnostics.technical')),
          element('p', null, blocker.message),
        );
        detail.append(technical);
      } else {
        detail.append(element('p', null, blocker.message));
      }
      detail.append(element(
        'p',
        null,
        document.documentElement.lang === 'ja' ? t('completion.blocker.next') : blocker.next_action,
      ));
      item.append(summary, detail);
      body.append(item);
    });
    if (attempt.next_action && !attempt.blockers?.length) {
      body.append(element('p', null, attempt.next_action));
    }
    card.append(body);
    section.append(card);
  });
  root.append(section);
}

export function renderDiagnostics(
  diagnostics,
  root = document.querySelector('[data-diagnostic-result]'),
  state = null,
  options = {},
) {
  if (!root) return;
  root.replaceChildren();
  const validation = diagnostics?.validation || diagnostics || {};
  const uiState = state || {
    selectedDiagnosticPhase: 'all',
    selectedDiagnosticSeverity: 'all',
    diagnosticSort: 'severity',
    render: () => {},
  };
  const [tone, icon, label] = stateTone(validation.state);
  const summary = element('section', `insight-summary ${tone}`);
  const status = element('span', 'insight-status-icon', icon);
  status.setAttribute('aria-hidden', 'true');
  const copy = element('div', 'insight-summary-copy');
  copy.append(element('h2', null, label));
  const facts = element('div', 'insight-facts');
  facts.append(element('span', 'chip', contextLabel(validation.context)));
  if (validation.evaluated_rule_count) {
    facts.append(element('span', 'chip', `${validation.evaluated_rule_count} ${t('diagnostics.rules')}`));
  }
  const duration = formatDuration(validation.duration_ms);
  if (duration) facts.append(element('span', 'chip', duration));
  copy.append(facts);
  summary.append(status, copy);
  root.append(summary);

  if (options.completion) renderCompletion(root, options.completion, options.planDigest);
  if (validation.state === 'not_run') return;
  const filters = renderFilters(validation, uiState, Boolean(options.compact));
  if (filters && options.compact) root.append(filters);

  const allDiagnostics = validation.diagnostics || [];
  const filtered = allDiagnostics.filter(diagnostic => {
    const phaseMatches = uiState.selectedDiagnosticPhase === 'all'
      || diagnostic.phase === uiState.selectedDiagnosticPhase;
    const severityMatches = uiState.selectedDiagnosticSeverity === 'all'
      || diagnostic.severity === uiState.selectedDiagnosticSeverity;
    return phaseMatches && severityMatches;
  });
  filtered.sort((left, right) => {
    if (uiState.diagnosticSort === 'location') {
      return String(left.primary?.path || '').localeCompare(String(right.primary?.path || ''));
    }
    if (uiState.diagnosticSort === 'rule') return String(left.rule_id).localeCompare(String(right.rule_id));
    return (severityRank[left.severity] ?? 9) - (severityRank[right.severity] ?? 9)
      || String(left.primary?.path || '').localeCompare(String(right.primary?.path || ''));
  });

  if (!filtered.length) {
    const empty = element('div', 'context-empty-state success');
    const emptyIcon = element('span', 'context-empty-icon', '✓');
    emptyIcon.setAttribute('aria-hidden', 'true');
    empty.append(
      emptyIcon,
      element('h2', null, t('diagnostics.zero.title')),
      element('p', null, t('diagnostics.zero.description')),
    );
    root.append(empty);
    return;
  }
  const list = element('div', 'diagnostic-list');
  filtered.forEach(diagnostic => list.append(renderDiagnostic(diagnostic)));
  root.append(list);
}

export function initDiagnostics(state) {
  const context = document.querySelector('[data-diagnostics-context]');
  const rangeWrap = document.querySelector('[data-diagnostics-range-wrap]');
  const range = document.querySelector('[data-validation-range]');
  const run = document.querySelector('[data-diagnostics-run]');
  const syncContext = () => {
    state.diagnosticContext = context?.value || 'workspace';
    if (rangeWrap) rangeWrap.hidden = state.diagnosticContext !== 'git_range';
  };
  context?.addEventListener('change', () => {
    syncContext();
    state.render();
  });
  range?.addEventListener('input', () => { state.diagnosticRange = range.value; });
  run?.addEventListener('click', () => {
    syncContext();
    state.runAction(async () => {
      const validation = await state.api.runDiagnostics(
        state.projection,
        state.diagnosticContext,
        state.diagnosticRange,
      );
      state.projection.diagnostics.validation = validation;
      if (['work-plan', 'work_plan', 'slice'].includes(state.diagnosticContext)) {
        state.projection.work.validation = validation;
      }
      return state.projection;
    }, null, t('filter.validate'));
  });
  if (context) {
    context.value = ['workspace', 'git_range', 'work-plan', 'slice'].includes(state.diagnosticContext)
      ? state.diagnosticContext
      : 'workspace';
  }
  syncContext();
}
