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
    document.querySelector('[data-work-overview]'),
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
      () => { state.selectedSlice = null; },
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
    verifyButton.disabled = !selected || work?.validation?.state !== 'passed';
    verifyButton.onclick = () => selected && state.runAction(
      () => state.api.verifyWork(state.projection, selected.id),
      receipt => { state.verificationReceipt = receipt; state.selectedSlice = selected.id; },
    );
  }
  const resultButton = document.querySelector('[data-work-result]');
  if (resultButton) {
    resultButton.disabled = !state.verificationReceipt;
    resultButton.onclick = () => state.verificationReceipt && state.runAction(
      () => state.api.validateResult(state.projection, state.verificationReceipt),
    );
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
