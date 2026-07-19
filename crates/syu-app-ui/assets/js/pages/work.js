import { translate } from '../i18n.js';

const t = key => translate(key);

function replace(root, content) {
  if (!root) return;
  root.replaceChildren();
  if (content) root.append(content);
}

function element(tag, className, text) {
  const node = document.createElement(tag);
  if (className) node.className = className;
  if (text) node.textContent = text;
  return node;
}

function button(label, onClick, primary = false, icon = '→') {
  const node = element('button', `btn journey-action${primary ? ' primary' : ''}`);
  node.type = 'button';
  const iconNode = element('span', 'journey-action-icon', icon);
  iconNode.setAttribute('aria-hidden', 'true');
  node.append(iconNode, element('span', 'journey-action-label', label));
  node.addEventListener('click', onClick);
  return node;
}

function actionText(action, kind) {
  const key = action?.[`${kind}_key`];
  if (key) return t(key);
  const namespace = kind === 'label' ? 'action' : kind;
  return t(`journey.${namespace}.${action?.action || 'unknown'}`);
}

function matchingCandidates(state) {
  const query = String(state.journeyQuery || '').trim().toLowerCase();
  const items = state.projection.specifications?.specifications || [];
  const candidates = [];
  items.forEach(item => {
    if (item.kind !== 'requirement' || item.status !== 'implemented' || !item.criteria?.length) return;
    item.criteria.forEach(criterion => {
      const searchable = [item.title, item.summary, item.description, criterion.statement]
        .filter(Boolean)
        .join(' ')
        .toLowerCase();
      if (!query || searchable.includes(query)) candidates.push({ item, criterion });
    });
  });
  return candidates.slice(0, 6);
}

function run(state, action) {
  return state.runAction(() => state.api.runJourneyAction(state.projection, action));
}

function renderStart(root, state) {
  root.append(element('p', 'journey-copy', t('journey.intro')));
  const form = element('form', 'journey-intake');
  const label = element('label', null, t('journey.prompt'));
  const input = document.createElement('textarea');
  input.rows = 3;
  input.value = state.journeyQuery || '';
  input.placeholder = t('journey.placeholder');
  label.append(input);
  form.append(label);
  form.append(button(t('journey.find'), event => {
    event.preventDefault();
    state.journeyQuery = input.value;
    state.render();
  }, true, '⌕'));
  form.addEventListener('submit', event => event.preventDefault());
  root.append(form);
  if (!String(state.journeyQuery || '').trim()) return;
  const candidates = matchingCandidates(state);
  root.append(element('h2', 'journey-section-title', t('journey.choose')));
  if (!candidates.length) {
    root.append(element('p', 'empty-state', t('journey.no_match')));
    return;
  }
  candidates.forEach(({ item, criterion }) => {
    const card = element('article', 'journey-card');
    card.append(element('h3', null, item.title));
    card.append(element('p', null, criterion.statement || item.summary || item.description || 'This behavior is available for review.'));
    card.append(button(t('journey.review'), () => run(state, {
      action: 'create',
      anchor: criterion.anchor,
      summary: state.journeyQuery,
    }), true));
    root.append(card);
  });
}

function progressIcon(status) {
  return status === 'complete' ? '✓' : status === 'current' ? '●' : '○';
}

function renderProgress(root, journey) {
  const steps = element('ol', 'journey-steps');
  (journey.steps || []).forEach(step => {
    const label = t(`journey.step.${step.id}`);
    const item = element('li', `journey-step ${step.status}`);
    item.setAttribute('aria-label', label);
    item.title = label;
    const icon = element('span', 'journey-step-icon', progressIcon(step.status));
    icon.setAttribute('aria-hidden', 'true');
    item.append(icon);
    if (step.status === 'current') item.append(element('span', 'journey-step-label', label));
    steps.append(item);
  });
  root.append(steps);
}

function toneFor(status) {
  if (['approved', 'reviewed', 'ready', 'complete', 'completed'].includes(status)) return 'success';
  if (['blocked', 'implementation_blocked', 'error'].includes(status)) return 'danger';
  if (['proposed', 'draft', 'needs_attention'].includes(status)) return 'warning';
  if (['in_progress', 'active'].includes(status)) return 'active';
  return 'muted';
}

function toneIcon(tone) {
  return { success: '✓', danger: '!', warning: '◇', active: '●', muted: '○' }[tone] || '○';
}

function stateHeader(label, status) {
  const tone = toneFor(status);
  const header = element('div', `journey-state-head ${tone}`);
  const icon = element('span', 'journey-state-icon', toneIcon(tone));
  icon.setAttribute('aria-hidden', 'true');
  header.append(icon, element('strong', null, label));
  return header;
}

function countChip(value, icon, label) {
  const chip = element('span', 'journey-count');
  chip.setAttribute('aria-label', `${label}: ${value}`);
  chip.title = `${label}: ${value}`;
  const iconNode = element('span', null, icon);
  iconNode.setAttribute('aria-hidden', 'true');
  chip.append(iconNode, element('b', null, String(value)));
  return chip;
}

function renderScope(journey) {
  if (!journey.approved_scope) return null;
  const scope = journey.approved_scope;
  const card = element('section', 'journey-state journey-scope');
  card.append(stateHeader(t(`journey.scope.${scope.status || 'proposed'}`), scope.status));
  const counts = element('div', 'journey-counts');
  counts.append(
    countChip(scope.editable_target_count, '↔', t('journey.targets')),
    countChip(scope.slice_count, '▱', t('journey.steps_count')),
  );
  card.append(counts);
  return card;
}

function renderEvidence(journey) {
  const evidence = journey.evidence || {};
  const status = evidence.status || 'not_started';
  const blockers = evidence.blockers || [];
  const card = element('section', `journey-state journey-evidence ${toneFor(status)}${blockers.length ? ' has-blockers' : ''}`);
  card.append(stateHeader(t(`journey.evidence_label.${status}`), status));
  blockers.forEach(blocker => {
    const item = element('div', 'journey-blocker');
    item.append(element('strong', null, blocker.message));
    item.append(element('p', null, blocker.next_action));
    card.append(item);
  });
  return card;
}

function renderAdvanced(root, journey, work) {
  const advanced = element('details', 'journey-advanced');
  advanced.append(element('summary', null, t('journey.advanced')));
  const technical = journey.advanced || {};
  const attempts = [work?.completion?.current, ...(work?.completion?.previous || [])].filter(attempt => attempt && attempt.plan_digest === work?.plan?.digest);
  const identities = [
    `${t('journey.advanced.request')}: ${technical.request_id || t('journey.advanced.none')}`,
    `${t('journey.advanced.plan')}: ${technical.plan_id || t('journey.advanced.none')}`,
  ];
  if (!attempts.length) {
    identities.push(`${t('journey.advanced.step')}: ${technical.selected_slice_id || t('journey.advanced.none')}`);
  }
  advanced.append(element('p', null, identities.join('\n')));
  if (attempts.length) {
    advanced.append(element('h3', null, t('journey.advanced.completion')));
    advanced.append(element('p', null, `${t('journey.advanced.slice')}: ${technical.selected_slice_id || t('journey.advanced.none')}`));
    attempts.forEach(attempt => {
      advanced.append(element('p', null, `${t(`journey.status.${attempt.status}`)}: ${attempt.attempt_id}\n${t('journey.advanced.demonstrated')}: ${(attempt.demonstrated || []).join(', ') || t('journey.advanced.none')}`));
    });
  }
  const agent = work?.agent;
  if (agent) {
    advanced.append(element('h3', null, t('journey.advanced.agent')));
    advanced.append(element('p', null, `${t(`journey.status.${agent.status}`)}: ${agent.run_id}`));
    (work.agent_events || []).forEach(event => {
      const detail = event.event || {};
      advanced.append(element('p', null, `${detail.kind || 'event'}: ${event.created_at}`));
    });
  }
  root.append(advanced);
}

function renderJourney(root, journey, state, work) {
  const header = element('header', 'journey-header');
  header.append(element('h2', null, journey.title));
  root.append(header);
  renderProgress(root, journey);

  const next = element('section', 'journey-next');
  next.append(element('p', 'journey-copy', actionText(journey.primary_action, 'explanation')));
  const actions = element('div', 'journey-actions');
  const action = journey.primary_action;
  actions.append(button(actionText(action, 'label'), () => {
    if (action.confirmation_required && !window.confirm(`${actionText(action, 'explanation')}\n\n${t('journey.confirm')}`)) return;
    run(state, { action: action.action });
  }, true));
  if (journey.recovery_action) {
    const recovery = journey.recovery_action;
    actions.append(button(actionText(recovery, 'label'), () => {
      if (recovery.confirmation_required && !window.confirm(`${actionText(recovery, 'explanation')}\n\n${t('journey.confirm')}`)) return;
      run(state, { action: recovery.action });
    }, false, '×'));
  }
  next.append(actions);
  root.append(next);

  const states = element('div', 'journey-state-grid');
  const scope = renderScope(journey);
  if (scope) states.append(scope);
  states.append(renderEvidence(journey));
  root.append(states);
  renderAdvanced(root, journey, work);
}

function specificationStatus(value) {
  if (!value) return null;
  const normalized = String(value).toLowerCase().replace(/\s+/g, '-');
  const chip = element('span', `chip status-component status-${normalized}`);
  const dot = element('span', 'status-dot');
  dot.setAttribute('aria-hidden', 'true');
  chip.append(dot, element('span', null, t(`items.status.${normalized}`)));
  return chip;
}

function renderSpecification(root, workspace, journey, state) {
  const specification = journey?.related_specification;
  const specificationAnchor = journey?.advanced?.specification_anchor || null;
  if (state.journeySpecificationAnchor !== specificationAnchor) {
    state.journeySpecificationAnchor = specificationAnchor;
    state.journeySpecificationExpanded = false;
  }
  root.hidden = !specification;
  workspace?.classList.toggle('has-specification', Boolean(specification));
  root.replaceChildren();
  if (!specification) return;

  const header = element('header', 'journey-specification-head');
  const title = element('div');
  title.append(element('p', 'eyebrow', t('journey.specification')));
  const heading = element('h2', null, specification.title);
  heading.dataset.workSpecificationTitle = '';
  title.append(heading);
  header.append(title);
  const toggle = element('button', 'journey-specification-toggle');
  toggle.type = 'button';
  toggle.setAttribute('aria-expanded', String(state.journeySpecificationExpanded));
  toggle.setAttribute('aria-label', state.journeySpecificationExpanded
    ? t('journey.specification.collapse')
    : t('journey.specification.expand'));
  toggle.title = toggle.getAttribute('aria-label');
  toggle.textContent = state.journeySpecificationExpanded ? '−' : '+';
  toggle.addEventListener('click', () => {
    state.journeySpecificationExpanded = !state.journeySpecificationExpanded;
    state.render();
  });
  header.append(toggle);
  root.append(header);

  const body = element('div', `journey-specification-body${state.journeySpecificationExpanded ? ' expanded' : ''}`);
  const status = specificationStatus(specification.status);
  if (status) body.append(status);
  if (specification.overview) {
    const overview = element('p', 'journey-specification-overview', specification.overview);
    overview.dataset.workSpecificationOverview = '';
    body.append(overview);
  }
  const criterion = element('section', 'journey-specification-criterion');
  criterion.append(element('h3', null, t('journey.specification.criterion')));
  const statement = element('p', null, specification.criterion_statement);
  statement.dataset.workSpecificationCriterion = '';
  criterion.append(statement);
  body.append(criterion);

  const technical = journey.advanced || {};
  if (technical.specification_anchor) {
    const advanced = element('details', 'journey-specification-advanced');
    advanced.append(element('summary', null, t('common.advanced')));
    advanced.append(element('p', 'path', `${t('items.field.anchor')}: ${technical.specification_anchor}`));
    body.append(advanced);
  }
  root.append(body);
}

export function renderWork(work, state) {
  const root = document.querySelector('[data-work-overview-summary]');
  const specificationRoot = document.querySelector('[data-work-specification]');
  const workspace = document.querySelector('[data-work-journey-workspace]');
  const journey = state.projection.journey;
  if (!root) return;
  const content = element('div', 'journey');
  if (state.error) content.append(element('p', 'status-message status-error', state.error.message));
  if (!journey || journey.current_step === 'describe') renderStart(content, state);
  else renderJourney(content, journey, state, work);
  replace(root, content);
  if (specificationRoot) renderSpecification(specificationRoot, workspace, journey, state);
}
