import { translate } from '../i18n.js';

const t = key => translate(key);

function message(key, values = {}) {
  return Object.entries(values).reduce((text, [name, value]) => text.replaceAll(`{${name}}`, value), t(key));
}

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

function button(label, onClick, primary = false) {
  const node = element('button', `btn journey-action${primary ? ' primary' : ''}`, label);
  node.type = 'button';
  node.addEventListener('click', onClick);
  return node;
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
  }, true));
  form.addEventListener('submit', event => event.preventDefault());
  root.append(form);
  if (!String(state.journeyQuery || '').trim()) return;
  const candidates = matchingCandidates(state);
  const heading = element('h2', 'journey-section-title', t('journey.choose'));
  root.append(heading);
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

function renderJourney(root, journey, state, work) {
  const header = element('header', 'journey-header');
  header.append(element('p', 'eyebrow', t('journey.eyebrow')));
  header.append(element('h2', null, journey.title));
  root.append(header);
  const steps = element('ol', 'journey-steps');
  (journey.steps || []).forEach(step => steps.append(element('li', `journey-step ${step.status}`, t(`journey.step.${step.id}`))));
  root.append(steps);
  root.append(element('p', 'journey-copy', t(`journey.explanation.${journey.primary_action.action}`)));
  if (journey.approved_scope) {
    const scope = element('section', 'journey-card');
    scope.append(element('h3', null, t('journey.scope')));
    scope.append(element('p', null, message('journey.scope_count', {
      targets: journey.approved_scope.editable_target_count,
      slices: journey.approved_scope.slice_count,
    })));
    root.append(scope);
  }
  const evidence = journey.evidence || {};
  const evidenceCard = element('section', 'journey-card');
  evidenceCard.append(element('h3', null, t('journey.evidence')));
  evidenceCard.append(element('p', null, t(`journey.evidence.${evidence.status || 'not_started'}`)));
  (evidence.blockers || []).forEach(blocker => {
    const item = element('div', 'journey-blocker');
    item.append(element('strong', null, blocker.message));
    item.append(element('p', null, blocker.next_action));
    evidenceCard.append(item);
  });
  root.append(evidenceCard);
  const action = journey.primary_action;
  root.append(button(t(`journey.action.${action.action}`), () => {
    if (action.confirmation_required && !window.confirm(`${t(`journey.explanation.${action.action}`)}\n\n${t('journey.confirm')}`)) return;
    run(state, { action: action.action });
  }, true));
  if (journey.recovery_action) {
    const recovery = journey.recovery_action;
    root.append(button(t(`journey.action.${recovery.action}`), () => {
      if (recovery.confirmation_required && !window.confirm(`${t(`journey.explanation.${recovery.action}`)}\n\n${t('journey.confirm')}`)) return;
      run(state, { action: recovery.action });
    }));
  }
  const advanced = element('details', 'journey-advanced');
  advanced.append(element('summary', null, t('journey.advanced')));
  const technical = journey.advanced || {};
  advanced.append(element('p', null, `${t('journey.advanced.request')}: ${technical.request_id || t('journey.advanced.none')}\n${t('journey.advanced.plan')}: ${technical.plan_id || t('journey.advanced.none')}\n${t('journey.advanced.step')}: ${technical.selected_slice_id || t('journey.advanced.none')}\n${t('journey.advanced.attempt')}: ${technical.attempt_id || t('journey.advanced.none')}`));
  const attempts = [work?.completion?.current, ...(work?.completion?.previous || [])].filter(attempt => attempt?.plan_digest === work?.plan?.digest);
  if (attempts.length) {
    advanced.append(element('h3', null, t('journey.advanced.completion')));
    attempts.forEach(attempt => {
      const item = element('p', null, `${t(`journey.status.${attempt.status}`)}: ${attempt.attempt_id}\n${t('journey.advanced.slice')}: ${attempt.slice_id}\n${t('journey.advanced.demonstrated')}: ${(attempt.demonstrated || []).join(', ') || t('journey.advanced.none')}`);
      advanced.append(item);
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

export function renderWork(work, state) {
  const root = document.querySelector('[data-work-overview-summary]');
  const journey = state.projection.journey;
  if (!root) return;
  const content = element('div', 'journey');
  if (state.error) content.append(element('p', 'status-message status-error', state.error.message));
  if (!journey || journey.current_step === 'describe') renderStart(content, state);
  else renderJourney(content, journey, state, work);
  replace(root, content);
}
