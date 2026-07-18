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

function button(label, onClick, primary = false) {
  const node = element('button', `btn journey-action${primary ? ' primary' : ''}`, label);
  node.type = 'button';
  node.addEventListener('click', onClick);
  return node;
}

function matchingCandidates(state) {
  const query = String(state.journeyQuery || '').trim().toLowerCase();
  const items = state.projection.specifications?.specifications || [];
  return items.filter(item => item.anchors?.length && (!query || `${item.title} ${item.summary} ${item.description || ''}`.toLowerCase().includes(query))).slice(0, 6);
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
  candidates.forEach(item => {
    const card = element('article', 'journey-card');
    card.append(element('h3', null, item.title));
    card.append(element('p', null, item.summary || item.description || 'This behavior is available for review.'));
    card.append(button(t('journey.review'), () => run(state, {
      action: 'create',
      anchor: item.anchors[0],
      summary: state.journeyQuery,
    }), true));
    root.append(card);
  });
}

function renderJourney(root, journey, state) {
  const header = element('header', 'journey-header');
  header.append(element('p', 'eyebrow', t('journey.eyebrow')));
  header.append(element('h2', null, journey.title));
  root.append(header);
  const steps = element('ol', 'journey-steps');
  (journey.steps || []).forEach(step => steps.append(element('li', `journey-step ${step.status}`, t(`journey.step.${step.id}`))));
  root.append(steps);
  root.append(element('p', 'journey-copy', journey.primary_action.explanation));
  if (journey.approved_scope) {
    const scope = element('section', 'journey-card');
    scope.append(element('h3', null, t('journey.scope')));
    scope.append(element('p', null, journey.approved_scope.summary));
    scope.append(element('p', 'journey-meta', `${journey.approved_scope.editable_target_count} change area${journey.approved_scope.editable_target_count === 1 ? '' : 's'} in ${journey.approved_scope.slice_count} focused step${journey.approved_scope.slice_count === 1 ? '' : 's'}.`));
    root.append(scope);
  }
  const evidence = journey.evidence || {};
  const evidenceCard = element('section', 'journey-card');
  evidenceCard.append(element('h3', null, t('journey.evidence')));
  evidenceCard.append(element('p', null, evidence.summary || 'No evidence yet.'));
  (evidence.blockers || []).forEach(blocker => {
    const item = element('div', 'journey-blocker');
    item.append(element('strong', null, blocker.message));
    item.append(element('p', null, blocker.next_action));
    evidenceCard.append(item);
  });
  root.append(evidenceCard);
  const action = journey.primary_action;
  root.append(button(t(`journey.action.${action.action}`), () => {
    if (action.confirmation_required && !window.confirm(`${action.explanation}\n\n${t('journey.confirm')}`)) return;
    run(state, { action: action.action });
  }, true));
  if (journey.recovery_action) {
    const recovery = journey.recovery_action;
    root.append(button(t(`journey.action.${recovery.action}`), () => {
      if (recovery.confirmation_required && !window.confirm(recovery.explanation)) return;
      run(state, { action: recovery.action });
    }));
  }
  const advanced = element('details', 'journey-advanced');
  advanced.append(element('summary', null, t('journey.advanced')));
  const technical = journey.advanced || {};
  advanced.append(element('p', null, `Request: ${technical.request_id || 'not created'}\nPlan: ${technical.plan_id || 'not prepared'}\nStep: ${technical.selected_slice_id || 'not selected'}\nAttempt: ${technical.attempt_id || 'not started'}`));
  root.append(advanced);
}

export function renderWork(work, state) {
  const root = document.querySelector('[data-work-overview-summary]');
  const journey = state.projection.journey;
  if (!root) return;
  const content = element('div', 'journey');
  if (state.error) content.append(element('p', 'status-message status-error', state.error.message));
  if (!journey || journey.current_step === 'describe') renderStart(content, state);
  else renderJourney(content, journey, state);
  replace(root, content);
  document.querySelectorAll('[data-work-agent-history], [data-work-completion-history]').forEach(node => { node.hidden = true; });
}
