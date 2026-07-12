export function renderTarget(target) {
  const item = document.createElement('li');
  item.textContent = `${target.reference} · ${target.access}`;
  return item;
}
