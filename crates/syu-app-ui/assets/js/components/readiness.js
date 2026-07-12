export function renderReadiness(readiness) {
  const item = document.createElement('output');
  item.textContent = `${readiness.target}: ${readiness.status}`;
  return item;
}
