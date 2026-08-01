import { localizeEnum, localizeSpecificationTitle, translate } from '../i18n.js';
import { renderDiff } from '../components/diff.js';
import { renderSpecificationDetail } from './specifications.js';
import { renderReadinessPage } from './readiness.js';
import { renderDiagnostics } from './diagnostics.js';

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

async function loadWorkDiff(state, force = false) {
  if (state.scopeDiffLoading || (state.scopeDiff && !force)) return;
  state.scopeDiffLoading = true;
  state.scopeDiffError = null;
  state.render();
  try {
    state.scopeDiff = await state.api.readScopeDiff(state.scopeRange || '');
  } catch (error) {
    state.scopeDiffError = error.message;
  }
  state.scopeDiffLoading = false;
  state.render();
}

function openWorkDiff(state, force = false) {
  state.journeyContextTab = 'diff';
  state.journeySpecificationExpanded = true;
  state.journeyDiffRequested = true;
  loadWorkDiff(state, force);
}

function run(state, action) {
  return state.runAction(
    () => state.api.runJourneyAction(state.projection, action),
    () => {
      if (action.action === 'verify') {
        state.journeyContextTab = 'diagnostics';
        state.journeySpecificationExpanded = true;
      }
      if (action.action === 'start' || action.action === 'retry') openWorkDiff(state, true);
      if (action.action === 'rename') state.workTitleEditing = false;
    },
    action.action === 'rename' ? t('work.title.saving') : actionText(action, 'label'),
  );
}

function dispatchJourneyAction(state, action) {
  if (action.action === 'choose_specification') {
    state.go('specifications');
    return;
  }
  run(state, { action: action.action });
}

function renderStart(root, journey, state) {
  const action = journey?.primary_action;
  if (!action) return;
  const empty = element('section', 'context-empty-state work-start');
  empty.append(
    element('span', 'context-empty-icon', '◈'),
    element('h2', null, journey.title_key ? t(journey.title_key) : journey.title),
    element('p', null, actionText(action, 'explanation')),
    button(actionText(action, 'label'), () => dispatchJourneyAction(state, action), true, '→'),
  );
  root.append(empty);
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

function renderEvidence(journey, state) {
  const evidence = journey.evidence || {};
  const status = evidence.status || 'not_started';
  const blockers = evidence.blockers || [];
  const card = element('section', `journey-state journey-evidence ${toneFor(status)}${blockers.length ? ' has-blockers' : ''}`);
  card.append(stateHeader(t(`journey.evidence_label.${status}`), status));
  if (blockers.length) {
    card.append(countChip(blockers.length, '!', t('journey.blockers'), () => {
      state.journeyContextTab = 'diagnostics';
      state.journeySpecificationExpanded = true;
      state.render();
    }));
    const flow = element('div', 'journey-blocked-flow');
    [
      ['✓', t('journey.visual.intent'), 'success'],
      ['!', t('journey.visual.split'), 'danger'],
      ['○', t('journey.visual.retry'), 'muted'],
    ].forEach(([icon, label, tone]) => {
      const step = element('span', `journey-blocked-step ${tone}`);
      step.append(element('b', null, icon), element('span', null, label));
      flow.append(step);
    });
    card.append(flow);
    const details = element('div', 'journey-blocker-list');
    blockers.forEach((blocker, index) => {
      const item = element('details', 'journey-blocker-detail');
      const summary = element('summary');
      summary.append(
        element('span', 'status-marker', '!'),
        element('strong', null, blocker.message || `${t('journey.blockers')} ${index + 1}`),
      );
      item.append(summary, element('p', null, blocker.next_action));
      details.append(item);
    });
    card.append(details);
  } else if (['in_progress', 'ready', 'complete'].includes(status)) {
    card.append(countChip(
      state.scopeDiff?.files?.length ?? '…',
      '±',
      t('diff.action'),
      () => openWorkDiff(state),
    ));
  }
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
  if (state.workTitleEditing) {
    const form = element('form', 'work-title-editor');
    const input = document.createElement('input');
    input.className = 'work-title-input';
    input.value = journey.title;
    input.maxLength = 120;
    input.required = true;
    input.setAttribute('aria-label', t('work.title.input'));
    const save = element('button', 'icon-button success', '✓');
    save.type = 'submit';
    save.setAttribute('aria-label', t('common.save'));
    save.title = t('common.save');
    const cancel = element('button', 'icon-button', '×');
    cancel.type = 'button';
    cancel.setAttribute('aria-label', t('common.cancel'));
    cancel.title = t('common.cancel');
    cancel.addEventListener('click', () => {
      state.workTitleEditing = false;
      state.render();
    });
    form.append(input, save, cancel);
    form.addEventListener('submit', event => {
      event.preventDefault();
      if (!input.reportValidity()) return;
      run(state, { action: 'rename', title: input.value });
    });
    form.addEventListener('keydown', event => {
      if (event.key !== 'Escape') return;
      event.preventDefault();
      state.workTitleEditing = false;
      state.render();
    });
    header.append(form);
    queueMicrotask(() => {
      input.focus();
      input.select();
    });
  } else {
    const title = element('h2', null, journey.title);
    const edit = element('button', 'work-title-edit', '✎');
    edit.type = 'button';
    edit.setAttribute('aria-label', t('work.title.edit'));
    edit.title = t('work.title.edit');
    edit.addEventListener('click', () => {
      state.workTitleEditing = true;
      state.render();
    });
    header.append(title, edit);
  }
  root.append(header);
  renderProgress(root, journey);

  if (work?.request) {
    const requestMeta = element('div', 'meta-line work-request-meta');
    requestMeta.append(
      element('span', 'chip', `${t('work.request.operation')}: ${localizeEnum('operation', work.request.operation)}`),
      element('span', 'chip', `${t('work.request.seed')}: ${work.request.seed_count}`),
    );
    root.append(requestMeta);
  }

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
  states.append(renderEvidence(journey, state));
  root.append(states);
  renderAdvanced(root, journey, work);
}

function resetContextSource(state) {
  state.journeyContextTarget = null;
  state.specificationSourceTarget = null;
  state.specificationSource = null;
  state.specificationSourceFull = false;
}

function renderContextTabs(root, state, hasScope, hasWorkInsights) {
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
  if (hasWorkInsights) {
    addTab('readiness', t('nav.readiness'), '◇');
    addTab('diagnostics', t('nav.diagnostics'), '✓');
    addTab('diff', t('diff.action'), '±');
  }
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
      element('span', 'chip blue-chip', localizeEnum('target.access', target.access || 'editable')),
      ...(target.transition ? [element('span', 'chip', localizeEnum('target.transition', target.transition))] : []),
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
  const contextAnchor = specificationAnchor;
  if (state.journeySpecificationAnchor !== specificationAnchor) {
    state.journeySpecificationAnchor = specificationAnchor;
    state.journeySpecificationExpanded = false;
    state.journeyContextTab = 'specification';
    state.journeyContextItemId = contextAnchor?.split('#')[0] || null;
    state.journeyContextHistory = [];
    resetContextSource(state);
  }
  const hasContext = Boolean(specification);
  const hasScope = Boolean(journey?.approved_scope && work?.plan?.slices?.length);
  const hasWorkInsights = Boolean(work?.request);
  if (!hasScope && state.journeyContextTab === 'scope') state.journeyContextTab = 'specification';
  const hasPanel = hasContext || hasWorkInsights;
  root.hidden = !hasPanel;
  workspace?.classList.toggle('has-specification', hasPanel);
  root.replaceChildren();
  if (!hasPanel) return;

  const header = element('header', 'journey-specification-head');
  const heading = element('div');
  heading.append(element('p', 'eyebrow', t('journey.context')));
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
  if (hasWorkInsights) renderContextTabs(root, state, hasScope, hasWorkInsights);

  const body = element('div', `journey-specification-body${state.journeySpecificationExpanded ? ' expanded' : ''}`);
  body.setAttribute('data-journey-panel', state.journeyContextTab);
  if (contextAnchor) body.setAttribute('data-work-specification-anchor', contextAnchor);
  if (state.journeyContextTab === 'readiness') {
    renderReadinessPage(state.projection.readiness, body, { state, compact: true });
    root.append(body);
    return;
  }
  if (state.journeyContextTab === 'diagnostics') {
    renderDiagnostics(
      { validation: work?.validation },
      body,
      state,
      { compact: true, completion: work?.completion, planDigest: work?.plan?.digest },
    );
    root.append(body);
    return;
  }
  if (state.journeyContextTab === 'diff') {
    renderDiff(state.scopeDiff, body, {
      loading: state.scopeDiffLoading,
      error: state.scopeDiffError,
      compact: true,
      openFirst: true,
    });
    if (!state.scopeDiff && !state.scopeDiffLoading && !state.scopeDiffError) {
      queueMicrotask(() => loadWorkDiff(state));
    }
    root.append(body);
    return;
  }
  if (state.journeyContextTarget) state.specificationSourceTarget = state.journeyContextTarget;
  if (state.journeyContextTab === 'scope') {
    renderScopeDetail(body, journey, state, work);
    root.append(body);
    return;
  }
  const items = state.projection.specifications?.specifications || [];
  const selected = items.find(item => item.id === state.journeyContextItemId)
    || items.find(item => item.id === contextAnchor?.split('#')[0]);
  if (!selected) {
    body.append(element('p', 'context-empty', t('journey.specification.empty')));
    root.append(body);
    return;
  }
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
    hideHeading: false,
    highlightedAnchor: contextAnchor,
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
    onSourceClose: () => {
      resetContextSource(state);
    },
  });
  body.querySelector('.specification-detail-head h2')?.setAttribute('data-work-specification-title', '');
  body.querySelector('.specification-criterion.is-highlighted p')?.setAttribute('data-work-specification-criterion', '');
  root.append(body);
}

function renderWorkSlices(work, state) {
  const rail = document.querySelector('[data-work-slices-rail]');
  const root = document.querySelector('[data-work-slice-detail]');
  if (!rail || !root) return;
  rail.replaceChildren();
  root.replaceChildren();
  const slices = work?.plan?.slices || [];
  if (!slices.length) {
    root.append(element('p', 'context-empty', t('work.slices.empty.description')));
    return;
  }
  const selected = slices.find(slice => slice.id === state.selectedSlice) || slices[0];
  state.selectedSlice = selected.id;
  slices.forEach((slice, index) => {
    const item = element('button', `rail-item${slice.id === selected.id ? ' active' : ''}`);
    item.type = 'button';
    const copy = element('div');
    copy.append(
      element('b', null, t('journey.scope.step').replace('{number}', String(index + 1))),
      element('p', null, `${slice.editable_targets?.length || 0} ${t('journey.targets')}`),
    );
    item.append(copy);
    item.addEventListener('click', () => {
      state.selectedSlice = slice.id;
      state.render();
    });
    rail.append(item);
  });
  root.append(element('h2', null, t('work.exact_targets')));
  (selected.editable_targets || []).forEach(target => {
    const row = element('button', 'journey-scope-target');
    row.type = 'button';
    row.append(
      element('span', 'journey-scope-target-icon', '↔'),
      element('strong', null, target.path),
      element('span', 'chip blue-chip', localizeEnum('target.access', target.access || 'editable')),
      ...(target.transition ? [element('span', 'chip', localizeEnum('target.transition', target.transition))] : []),
    );
    row.addEventListener('click', () => {
      state.journeyContextTarget = target;
      state.journeyContextTab = 'scope';
      state.journeySpecificationExpanded = true;
      state.render();
    });
    root.append(row);
  });
}

function renderWorkContext(work) {
  const rail = document.querySelector('[data-work-context-rail]');
  const root = document.querySelector('[data-work-context-detail]');
  if (!rail || !root) return;
  rail.replaceChildren();
  root.replaceChildren();
  if (!work?.context_pack) {
    root.append(element('p', 'context-empty', t('work.context.empty.description')));
    return;
  }
  const card = element('section', 'scope-detail-card');
  card.append(
    element('span', 'chip green-chip', t('work.context.ready')),
    element('h2', null, t('work.context.title')),
    element('p', null, `${work.context_pack.entry_count} ${t('common.item')}`),
  );
  root.append(card);
}

function renderWorkValidation(work, state) {
  const rail = document.querySelector('[data-work-validation-rail]');
  const root = document.querySelector('[data-work-validation-detail]');
  if (!rail || !root) return;
  rail.replaceChildren();
  renderDiagnostics(
    { validation: work?.validation },
    root,
    state,
    { completion: work?.completion, planDigest: work?.plan?.digest },
  );
}

export function renderWork(work, state) {
  const root = document.querySelector('[data-work-overview-summary]');
  const specificationRoot = document.querySelector('[data-work-specification]');
  const workspace = document.querySelector('[data-work-journey-workspace]');
  const journey = state.projection.journey;
  if (!root) return;
  const content = element('div', 'journey');
  if (state.error) content.append(element('p', 'status-message status-error', state.error.message));
  if (!journey || journey.current_step === 'select_specification') renderStart(content, journey, state);
  else renderJourney(content, journey, state, work);
  replace(root, content);
  if (specificationRoot) renderSpecification(specificationRoot, workspace, journey, state, work);
  renderWorkSlices(work, state);
  renderWorkContext(work);
  renderWorkValidation(work, state);
}
