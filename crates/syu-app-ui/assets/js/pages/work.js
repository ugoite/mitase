import { renderTarget } from '../components/target.js';
export function renderWork(work) {
  return (work.plan?.slices || []).flatMap(slice => slice.editable_targets).map(renderTarget);
}
