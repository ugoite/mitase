import { renderTarget } from '../components/target.js';

function replace(root, content) {
  if (!root) return;
  root.replaceChildren();
  if (typeof content === 'string') root.textContent = content;
  else if (content) root.append(content);
}

export function renderWork(work, state) {
  const plan = work?.plan;
  replace(document.querySelector('[data-work-overview]'), plan ? `${plan.id}: ${plan.status}` : 'Choose a server-provided work origin to create a plan.');
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
        renderWork(state.projection.work, state);
      });
      rail.append(button);
    });
  }
  const selected = slices.find(slice => slice.id === state.selectedSlice) || slices[0];
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
  replace(document.querySelector('[data-work-context-detail]'), work?.context_pack ? 'Context pack loaded from the server.' : 'Select a slice to export context.');
  replace(document.querySelector('[data-work-validation-detail]'), work?.validation?.state || 'not_run');
}
