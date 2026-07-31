export const PAGES = ['work', 'readiness', 'scope', 'specifications', 'diagnostics', 'settings'];
export const TAB_GROUPS = ['work', 'scope', 'specifications'];
export const SPECIFICATION_DETAIL_TABS = ['information', 'trace', 'evidence'];

export function navigate(page, push = true) {
  const selected = PAGES.includes(page) ? page : 'work';
  document.querySelectorAll('[data-page]').forEach(node => { node.hidden = node.dataset.page !== selected; });
  document.querySelectorAll('[data-route]').forEach(node => node.classList.toggle('active', node.dataset.route === selected));
  if (push) history.pushState({}, '', `?page=${encodeURIComponent(selected)}`);
  return selected;
}

function selectTab(group, tab, push = true) {
  const tabs = [...document.querySelectorAll(`[data-tab-group="${group}"]`)];
  if (!tabs.some(node => node.dataset.tab === tab)) return;
  tabs.forEach(node => {
    const selected = node.dataset.tab === tab;
    node.classList.toggle('active', selected);
    node.setAttribute('aria-selected', String(selected));
    node.tabIndex = selected ? 0 : -1;
  });
  document.querySelectorAll(`[data-panel-group="${group}"]`).forEach(node => {
    node.hidden = node.dataset.panel !== tab;
  });
  if (push) {
    const parameters = new URLSearchParams(location.search);
    parameters.set('page', group === 'specifications' ? 'specifications' : group);
    parameters.set(`${group}Tab`, tab);
    if (group === 'specifications') {
      parameters.delete('item');
      parameters.delete('detailTab');
      parameters.delete('node');
      parameters.delete('traceMode');
      parameters.delete('depth');
    }
    history.pushState({}, '', `?${parameters.toString()}`);
  }
}

export function syncSpecificationLocation(state, push = true) {
  const parameters = new URLSearchParams(location.search);
  parameters.set('page', 'specifications');
  parameters.set('specificationsTab', state.specificationKind || 'all');
  if (state.selectedSpecification) parameters.set('item', state.selectedSpecification);
  else parameters.delete('item');
  if (SPECIFICATION_DETAIL_TABS.includes(state.specificationDetailTab)) {
    parameters.set('detailTab', state.specificationDetailTab);
  } else {
    parameters.delete('detailTab');
  }
  if (state.specificationTraceNode) parameters.set('node', state.specificationTraceNode);
  else parameters.delete('node');
  if (state.specificationDetailTab === 'trace' && state.specificationTraceMode === 'exact') parameters.set('traceMode', 'exact');
  else parameters.delete('traceMode');
  if (state.specificationDetailTab === 'trace' && state.specificationTraceDepth > 1) parameters.set('depth', String(state.specificationTraceDepth));
  else parameters.delete('depth');
  const url = `?${parameters.toString()}`;
  if (push) history.pushState({}, '', url); else history.replaceState({}, '', url);
}

function bindKeyboardTabs() {
  for (const group of TAB_GROUPS) {
    const tabs = [...document.querySelectorAll(`[data-tab-group="${group}"]`)];
    const list = tabs[0]?.closest('.tabs');
    if (!list || !tabs.length) continue;
    list.setAttribute('role', 'tablist');
    tabs.forEach((tab, index) => {
      tab.id = `${group}-tab-${tab.dataset.tab}`;
      tab.setAttribute('role', 'tab');
      tab.setAttribute('aria-controls', `${group}-panel-${tab.dataset.tab}`);
      tab.tabIndex = index === 0 ? 0 : -1;
      tab.addEventListener('click', () => selectTab(group, tab.dataset.tab));
    });
    list.addEventListener('keydown', event => {
      const current = tabs.indexOf(document.activeElement);
      if (current < 0) return;
      const delta = { ArrowRight: 1, ArrowDown: 1, ArrowLeft: -1, ArrowUp: -1 }[event.key];
      const next = event.key === 'Home' ? 0 : event.key === 'End' ? tabs.length - 1 : delta === undefined ? -1 : (current + delta + tabs.length) % tabs.length;
      if (next < 0) return;
      event.preventDefault();
      tabs[next].focus();
      selectTab(group, tabs[next].dataset.tab);
    });
  }
}

export function bindRouter(state, onRoute) {
  document.querySelectorAll('[data-route]').forEach(node => node.addEventListener('click', () => {
    state.selectedPage = navigate(node.dataset.route);
    onRoute?.(state.selectedPage);
  }));
  bindKeyboardTabs();
  const applyLocation = () => {
    const parameters = new URL(location.href).searchParams;
    const page = parameters.get('page') || state.selectedPage || 'work';
    state.selectedPage = navigate(page, false);
    for (const group of TAB_GROUPS) {
      const requested = parameters.get(`${group}Tab`);
      const fallback = document.querySelector(`[data-tab-group="${group}"].active`)?.dataset.tab;
      const selected = requested || fallback;
      selectTab(group, selected, false);
      if (group === 'scope' && selected) state.selectedScopeTab = selected;
      if (group === 'specifications' && selected) state.specificationKind = selected;
    }
    if (page === 'specifications') {
      const item = parameters.get('item');
      if (item) state.selectedSpecification = item;
      const detailTab = parameters.get('detailTab');
      if (SPECIFICATION_DETAIL_TABS.includes(detailTab)) state.specificationDetailTab = detailTab;
      const traceMode = parameters.get('traceMode');
      if (traceMode === 'exact' || traceMode === 'readable') state.specificationTraceMode = traceMode;
      const depth = Number.parseInt(parameters.get('depth') || '3', 10);
      state.specificationTraceDepth = Number.isFinite(depth) ? Math.max(1, Math.min(depth, 8)) : 3;
      state.specificationTraceNode = parameters.get('node');
    }
    onRoute?.(state.selectedPage);
  };
  applyLocation();
  addEventListener('popstate', () => {
    applyLocation();
    state.render();
  });
}
