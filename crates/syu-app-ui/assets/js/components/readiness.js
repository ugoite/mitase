export function renderReadiness(readiness) {
  const item = document.createElement('output');
  item.textContent = `${readiness.target}: ${readiness.status} (${readiness.execution_state || 'not-run'})`;
  return item;
}
