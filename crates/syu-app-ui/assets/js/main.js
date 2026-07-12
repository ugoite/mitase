import { readProjection } from './api.js';
import { createState } from './state.js';
import { navigate } from './router.js';

export async function startWorkbench() {
  const state = createState(await readProjection());
  navigate(state.selectedPage, false);
  return state;
}

if (typeof window !== 'undefined') {
  startWorkbench().catch((error) => {
    window.dispatchEvent(new CustomEvent('syu-workbench-error', { detail: error }));
  });
}
