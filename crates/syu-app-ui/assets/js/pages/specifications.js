export function renderSpecifications(specifications, root = document.querySelector('[data-specifications-detail]')) {
  const entries = specifications?.specifications || [];
  if (!root) return entries;
  root.replaceChildren();
  const selected = entries[0];
  root.textContent = selected ? `${selected.id}: ${selected.title}` : 'No specifications.';
  return entries;
}
