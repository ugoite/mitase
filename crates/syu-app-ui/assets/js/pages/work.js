import { renderTarget } from '../components/target.js';

function replace(root, content) {
  if (!root) return;
  root.replaceChildren();
  if (typeof content === 'string') root.textContent = content;
  else if (content) root.append(content);
}

export function renderWork(work, state) {
  const plan = work?.plan;
  replace(
    document.querySelector('[data-work-overview-summary]'),
    state.error ? `Error: ${state.error.message}` : plan ? `${plan.id}: ${plan.status}` : work?.request ? 'Work request created. Press Plan to derive the safe slice.' : 'Choose an implemented criterion in Specifications to create a Modify Work request.',
  );
  const selectedPlan = plan?.slices || [];
  const selectedSliceId = work?.selected_slice
    || (selectedPlan.some(slice => slice.id === state.selectedSlice) ? state.selectedSlice : selectedPlan[0]?.id || null);
  if (selectedSliceId) state.selectedSlice = selectedSliceId;

  const planButton = document.querySelector('[data-work-plan]');
  if (planButton) {
    planButton.disabled = !work?.request;
    planButton.onclick = () => state.runAction(
      () => state.api.planWork(state.projection),
      () => { state.selectedSlice = null; state.planApproved = false; },
    );
  }
  const newButton = document.querySelector('[data-work-new]');
  if (newButton) newButton.onclick = () => state.go('specifications');
  const seedButton = document.querySelector('[data-work-seed]');
  if (seedButton) seedButton.disabled = true;

  const selected = selectedPlan.find(slice => slice.id === state.selectedSlice) || selectedPlan[0];
  const contextButton = document.querySelector('[data-work-context]');
  if (contextButton) {
    contextButton.disabled = !selected;
    contextButton.onclick = () => selected && state.runAction(
      () => state.api.exportContext(state.projection, selected.id),
      () => { state.selectedSlice = selected.id; },
    );
  }
  const validateButton = document.querySelector('[data-work-validate]');
  if (validateButton) {
    validateButton.disabled = !plan;
    validateButton.onclick = () => plan && state.runAction(() => state.api.validateWork(state.projection));
  }
  const verifyButton = document.querySelector('[data-work-verify]');
  if (verifyButton) {
    verifyButton.disabled = !selected || work?.validation?.state !== 'passed' || !state.planApproved;
    verifyButton.onclick = () => selected && state.runAction(
      () => state.api.verifyWork(state.projection, selected.id),
      attempt => { state.verificationReceipt = attempt.receipt; state.selectedSlice = selected.id; },
    );
  }
  const approveButton = document.querySelector('[data-work-approve]');
  if (approveButton) {
    approveButton.disabled = !plan || work?.validation?.state !== 'passed';
    approveButton.onclick = () => plan && state.runAction(
      () => state.api.approveWork(state.projection),
      () => { state.planApproved = true; },
    );
  }
  const agentButton = document.querySelector('[data-work-agent-start]');
  if (agentButton) {
    agentButton.disabled = !selected || !state.planApproved || Boolean(work?.agent);
    agentButton.onclick = () => selected && state.runAction(
      () => state.api.startAgent(state.projection, selected.id),
      () => { state.selectedSlice = selected.id; },
    );
  }
  const finalizeButton = document.querySelector('[data-work-finalize]');
  const currentAttempt = work?.completion?.current;
  if (finalizeButton) {
    finalizeButton.disabled = !currentAttempt || currentAttempt.status !== 'complete' || currentAttempt.finalized;
    finalizeButton.onclick = () => currentAttempt && state.runAction(
      async () => {
        const preview = await state.api.finalizePreview(state.projection, currentAttempt.attempt_id);
        return state.api.finalizeApply(state.projection, currentAttempt.attempt_id, preview.preview_token);
      },
    );
  }
  const history = document.querySelector('[data-work-completion-history]');
  if (history) {
    history.replaceChildren();
    const completion = work?.completion;
    const attempts = [completion?.current, ...(completion?.previous || [])].filter(Boolean);
    if (!attempts.length) history.textContent = 'No completion attempts yet.';
    else attempts.forEach(attempt => {
      const item = document.createElement('article');
      const title = document.createElement('h3');
      title.textContent = attempt.status + ': ' + attempt.attempt_id;
      item.append(title);
      const identity = document.createElement('p');
      identity.textContent = attempt.plan_digest + ' / ' + attempt.slice_id;
      item.append(identity);
      (attempt.blockers || []).forEach(blocker => {
        const blockerNode = document.createElement('p');
        blockerNode.textContent = blocker.code + ': ' + blocker.message + ' Next: ' + blocker.next_action;
        item.append(blockerNode);
      });
      const finalized = document.createElement('p');
      finalized.textContent = attempt.finalized ? 'Finalized' : 'Not finalized';
      item.append(finalized);
      history.append(item);
    });
  }
  const agentHistory = document.querySelector('[data-work-agent-history]');
  if (agentHistory) {
    agentHistory.replaceChildren();
    const agent = work?.agent;
    if (!agent) agentHistory.textContent = 'No scoped agent run yet.';
    else {
      const heading = document.createElement('h2');
      heading.textContent = `Agent ${agent.status}: ${agent.run_id}`;
      agentHistory.append(heading);
      const identity = document.createElement('p');
      identity.textContent = `${agent.plan_digest} / ${agent.slice_id}`;
      agentHistory.append(identity);
      (work.agent_events || []).forEach(event => {
        const item = document.createElement('p');
        item.textContent = `${event.kind}: ${event.created_at}`;
        agentHistory.append(item);
      });
    }
  }
  const slices = plan?.slices || [];
  const rail = document.querySelector('[data-work-slices-rail]');
  if (rail) {
    rail.replaceChildren();
    slices.forEach(slice => {
      const button = document.createElement('button');
      button.type = 'button';
      button.className = 'rail-item';
      button.textContent = slice.id;
      button.addEventListener('click', () => {
        state.selectedSlice = slice.id;
        state.render();
      });
      rail.append(button);
    });
  }
  const detail = document.querySelector('[data-work-slice-detail]');
  if (detail) {
    detail.replaceChildren();
    if (!selected) detail.textContent = 'No selected slice.';
    else {
      const heading = document.createElement('h2');
      heading.textContent = selected.id;
      detail.append(heading);
      const list = document.createElement('ul');
      selected.editable_targets.map(renderTarget).forEach(item => list.append(item));
      detail.append(list);
    }
  }
  replace(document.querySelector('[data-work-context-detail]'), work?.context_pack ? `Context pack loaded for ${work.context_pack.slice_id}.` : 'Select a slice and export context.');
  replace(document.querySelector('[data-work-validation-detail]'), work?.validation?.state || 'not_run');
}
