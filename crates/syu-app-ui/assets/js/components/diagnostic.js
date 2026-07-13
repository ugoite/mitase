export function renderDiagnostic(diagnostic) {
  const item = document.createElement('li');
  item.textContent = `${diagnostic.phase}: ${diagnostic.message || diagnostic.diagnostic?.message || ''}`;
  return item;
}
