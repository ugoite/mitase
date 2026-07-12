import { createState } from './state.js';
import { navigate } from './router.js';

export async function startWorkbench() {
  const node = document.querySelector('#syu-projection');
  if (!node) throw new Error('canonical Workbench projection is missing');
  const state = createState(JSON.parse(node.textContent));
  navigate(state.selectedPage, false);
  return state;
}

if (typeof window !== 'undefined') {
  startWorkbench().catch((error) => {
    window.dispatchEvent(new CustomEvent('syu-workbench-error', { detail: error }));
  });
}
