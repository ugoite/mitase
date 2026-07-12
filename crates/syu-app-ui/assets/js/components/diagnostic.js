export function renderDiagnostic(diagnostic) {
  const item = document.createElement('li');
  item.textContent = `${diagnostic.phase}: ${diagnostic.diagnostic.message}`;
  return item;
}
