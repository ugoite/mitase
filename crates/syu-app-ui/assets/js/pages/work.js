import { translate } from '../i18n.js';
import { renderSourceDetail, renderSpecificationDetail } from './specifications.js';

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
    state.journeyCandidateAnchor = null;
    root.append(element('p', 'empty-state', t('journey.no_match')));
    return;
  }
  if (!candidates.some(({ criterion }) => criterion.anchor === state.journeyCandidateAnchor)) {
    state.journeyCandidateAnchor = candidates[0].criterion.anchor;
  }
  candidates.forEach(({ item, criterion }) => {
    const selected = criterion.anchor === state.journeyCandidateAnchor;
    const card = element('article', `journey-card${selected ? ' selected' : ''}`);
    card.append(element('h3', null, item.title));
    const meta = element('div', 'meta-line');
    meta.append(element('span', `chip status-component status-${item.status || 'planned'}`, item.status || item.kind));
    card.append(meta);
    card.append(button(t('journey.preview'), () => {
      state.journeyCandidateAnchor = criterion.anchor;
      state.journeyContextTab = 'specification';
      state.render();
    }, selected, '◈'));
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

function countChip(value, icon, label, onClick) {
  const chip = element(onClick ? 'button' : 'span', `journey-count${onClick ? ' interactive' : ''}`);
  if (onClick) {
    chip.type = 'button';
    chip.addEventListener('click', onClick);
  }
  chip.setAttribute('aria-label', `${label}: ${value}`);
  chip.title = `${label}: ${value}`;
  const iconNode = element('span', null, icon);
  iconNode.setAttribute('aria-hidden', 'true');
  chip.append(iconNode, element('b', null, String(value)));
  return chip;
}

function renderScope(journey, state) {
  if (!journey.approved_scope) return null;
  const scope = journey.approved_scope;
  const card = element('section', 'journey-state journey-scope');
  card.append(stateHeader(t(`journey.scope.${scope.status || 'proposed'}`), scope.status));
  const counts = element('div', 'journey-counts');
  counts.append(
    countChip(scope.editable_target_count, '↔', t('journey.targets'), () => {
      state.journeyContextTab = 'scope';
      state.journeyScopeFocus = 'targets';
      state.journeyContextTarget = null;
      state.render();
    }),
    countChip(scope.slice_count, '▱', t('journey.steps_count'), () => {
      state.journeyContextTab = 'scope';
      state.journeyScopeFocus = 'steps';
      state.journeyContextTarget = null;
      state.render();
    }),
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
  const scope = renderScope(journey, state);
  if (scope) states.append(scope);
  states.append(renderEvidence(journey));
  root.append(states);
  renderAdvanced(root, journey, work);
}

function resetContextSource(state) {
  state.journeyContextTarget = null;
  state.specificationSourceTarget = null;
  state.specificationSource = null;
  state.specificationSourceFull = false;
}

function renderContextTabs(root, state, hasScope) {
  const tabs = element('div', 'journey-context-tabs');
  const addTab = (id, label, icon) => {
    const tab = element('button', `journey-context-tab${state.journeyContextTab === id ? ' active' : ''}`);
    tab.type = 'button';
    tab.setAttribute('aria-pressed', String(state.journeyContextTab === id));
    tab.append(element('span', null, icon), element('span', null, label));
    tab.addEventListener('click', () => {
      state.journeyContextTab = id;
      resetContextSource(state);
      state.render();
    });
    tabs.append(tab);
  };
  addTab('specification', t('journey.panel.specification'), '◈');
  if (hasScope) addTab('scope', t('journey.panel.scope'), '↔');
  root.append(tabs);
}

function renderScopeDetail(root, journey, state, work) {
  const plan = work?.plan;
  const slices = plan?.slices || [];
  if (!slices.length) {
    root.append(element('p', 'context-empty', t('journey.scope.empty')));
    return;
  }
  const selectedSlice = slices.find(slice => slice.id === state.selectedSlice) || slices[0];
  state.selectedSlice = selectedSlice.id;

  const steps = element('section', `journey-scope-section${state.journeyScopeFocus === 'steps' ? ' focused' : ''}`);
  steps.append(element('h3', null, t('journey.scope.steps')));
  const stepList = element('div', 'journey-scope-steps');
  slices.forEach((slice, index) => {
    const step = element('button', `journey-scope-step${slice.id === selectedSlice.id ? ' active' : ''}`);
    step.type = 'button';
    step.title = slice.id;
    step.append(
      element('span', 'journey-scope-index', String(index + 1)),
      element('span', null, t('journey.scope.step').replace('{number}', String(index + 1))),
    );
    step.addEventListener('click', () => {
      state.selectedSlice = slice.id;
      state.journeyScopeFocus = 'targets';
      state.render();
    });
    stepList.append(step);
  });
  steps.append(stepList);
  root.append(steps);

  const targets = element('section', `journey-scope-section${state.journeyScopeFocus === 'targets' ? ' focused' : ''}`);
  targets.append(element('h3', null, t('journey.scope.targets')));
  const targetList = element('div', 'journey-scope-targets');
  (selectedSlice.editable_targets || []).forEach(target => {
    const targetButton = element('button', 'journey-scope-target');
    targetButton.type = 'button';
    targetButton.setAttribute('data-scope-target', target.reference);
    targetButton.append(
      element('span', 'journey-scope-target-icon', '↔'),
      element('strong', null, target.path),
      element('span', 'chip blue-chip', t('journey.scope.editable')),
    );
    targetButton.addEventListener('click', () => {
      state.journeyContextTarget = target;
      state.specificationSourceTarget = target;
      state.specificationSource = null;
      state.specificationSourceFull = false;
      state.render();
    });
    targetList.append(targetButton);
  });
  if (!targetList.childElementCount) targetList.append(element('p', 'context-empty', t('journey.scope.no_targets')));
  targets.append(targetList);
  root.append(targets);
}

function renderSpecification(root, workspace, journey, state, work) {
  const specification = journey?.related_specification;
  const specificationAnchor = journey?.advanced?.specification_anchor || null;
  const candidates = journey?.current_step === 'describe' && String(state.journeyQuery || '').trim()
    ? matchingCandidates(state)
    : [];
  const candidate = candidates.find(({ criterion }) => criterion.anchor === state.journeyCandidateAnchor)
    || candidates[0]
    || null;
  const contextAnchor = specificationAnchor || candidate?.criterion.anchor || null;
  if (state.journeySpecificationAnchor !== specificationAnchor) {
    state.journeySpecificationAnchor = specificationAnchor;
    state.journeySpecificationExpanded = false;
    state.journeyContextTab = 'specification';
    state.journeyContextItemId = contextAnchor?.split('#')[0] || null;
    state.journeyContextHistory = [];
    resetContextSource(state);
  }
  if (!specification && candidate && state.journeyContextItemId !== candidate.item.id) {
    state.journeyContextItemId = candidate.item.id;
    state.journeyContextHistory = [];
    resetContextSource(state);
  }
  const hasContext = Boolean(specification || candidate);
  const hasScope = Boolean(journey?.approved_scope && work?.plan?.slices?.length);
  if (!hasScope && state.journeyContextTab === 'scope') state.journeyContextTab = 'specification';
  root.hidden = !hasContext;
  workspace?.classList.toggle('has-specification', hasContext);
  root.replaceChildren();
  if (!hasContext) return;

  const header = element('header', 'journey-specification-head');
  const heading = element('div');
  heading.append(element('p', 'eyebrow', t('journey.context')));
  if (candidate && !specification) heading.append(element('h2', null, t('journey.specification.preview')));
  header.append(heading);
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
  if (specification) renderContextTabs(root, state, hasScope);

  const body = element('div', `journey-specification-body${state.journeySpecificationExpanded ? ' expanded' : ''}`);
  body.setAttribute('data-journey-panel', state.journeyContextTab);
  if (state.journeyContextTarget) {
    state.specificationSourceTarget = state.journeyContextTarget;
    renderSourceDetail(body, state, state.journeyContextTarget, () => {
      resetContextSource(state);
      state.render();
    });
    root.append(body);
    return;
  }
  if (state.journeyContextTab === 'scope') {
    renderScopeDetail(body, journey, state, work);
    root.append(body);
    return;
  }
  const items = state.projection.specifications?.specifications || [];
  const selected = items.find(item => item.id === state.journeyContextItemId)
    || items.find(item => item.id === contextAnchor?.split('#')[0]);
  if (!selected) return;
  if (state.journeyContextHistory.length && !state.journeyContextTarget) {
    body.append(button(t('items.back'), () => {
      state.journeyContextItemId = state.journeyContextHistory.pop();
      state.journeyContextTarget = null;
      state.specificationSourceTarget = null;
      state.specificationSource = null;
      state.render();
    }, false, '←'));
  }
  renderSpecificationDetail(body, state, selected, {
    readOnly: true,
    hideHeading: Boolean(candidate && !specification),
    highlightedAnchor: contextAnchor,
    action: candidate && !specification ? {
      label: t('journey.select'),
      icon: '✓',
      onClick: () => run(state, {
        action: 'create',
        anchor: candidate.criterion.anchor,
        summary: state.journeyQuery,
      }),
    } : null,
    onItem: itemId => {
      if (itemId === state.journeyContextItemId) return;
      state.journeyContextHistory.push(state.journeyContextItemId);
      state.journeyContextItemId = itemId;
      state.render();
    },
    onTarget: target => {
      state.journeyContextTarget = target;
      state.specificationSourceTarget = target;
      state.specificationSource = null;
      state.specificationSourceFull = false;
      state.render();
    },
  });
  if (candidate && !specification) {
    body.setAttribute('data-journey-preview', candidate.criterion.anchor);
  } else {
    body.querySelector('.canvas-head h2')?.setAttribute('data-work-specification-title', '');
    body.querySelector('.specification-criterion.is-highlighted p')?.setAttribute('data-work-specification-criterion', '');
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
  if (specificationRoot) renderSpecification(specificationRoot, workspace, journey, state, work);
}
