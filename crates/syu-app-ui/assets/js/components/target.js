export function renderTarget(target) {
  const item = document.createElement('li');
  item.textContent = `${target.reference} · ${target.access} · ${target.path || target.resolved_path || ''}`;
  return item;
}
