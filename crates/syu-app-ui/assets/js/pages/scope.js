export function renderScope(scope, root = document.querySelector('[data-scope-detail]')) {
  if (!root) return;
  root.replaceChildren();
  const branch = scope?.branch;
  if (!branch) {
    root.textContent = 'Select a scope mode to inspect the server projection.';
    return;
  }
  const summary = document.createElement('p');
  summary.textContent = `${branch.state}: ${(branch.changed || []).length} changed artifact(s)`;
  root.append(summary);
}
