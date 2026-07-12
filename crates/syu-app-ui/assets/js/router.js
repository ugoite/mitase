export const PAGES = ['work', 'readiness', 'scope', 'specifications', 'diagnostics', 'settings'];

export function navigate(page, push = true) {
  const selected = PAGES.includes(page) ? page : 'work';
  document.querySelectorAll('[data-page]').forEach(node => { node.hidden = node.dataset.page !== selected; });
  document.querySelectorAll('[data-route]').forEach(node => node.classList.toggle('active', node.dataset.route === selected));
  if (push) history.pushState({}, '', `?page=${encodeURIComponent(selected)}`);
  return selected;
}
