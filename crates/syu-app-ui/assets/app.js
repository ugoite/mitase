(() => {
  const $ = (selector, root = document) => root.querySelector(selector);
  const $$ = (selector, root = document) => [...root.querySelectorAll(selector)];

  const DEFAULT_TAB = { work: 'overview', scope: 'change', items: 'all' };
  const TAB_PARAM = { work: 'workTab', scope: 'scopeTab', items: 'itemsTab' };
  const VALID_PAGES = new Set(['work', 'scope', 'items', 'diagnostics', 'settings']);
  const VALID_TABS = {
    work: new Set(['overview', 'slices', 'context', 'validation']),
    scope: new Set(['change', 'verify', 'reference', 'intent']),
    items: new Set(['all', 'philosophy', 'policy', 'requirement', 'feature']),
  };

  const normalizedPage = value => VALID_PAGES.has(value) ? value : 'work';
  const normalizedTab = (group, value) => VALID_TABS[group]?.has(value) ? value : DEFAULT_TAB[group] || null;
  const readTab = page => normalizedTab(page, new URL(location).searchParams.get(TAB_PARAM[page] || `${page}Tab`) || DEFAULT_TAB[page] || null);

  function updateUrl(mutator, push = true) {
    const url = new URL(location.href);
    mutator(url);
    if (push) history.pushState({}, '', url);
  }

  function showPage(page, push = true) {
    page = normalizedPage(page);
    $$('[data-page]').forEach(node => { node.hidden = node.dataset.page !== page; });
    $$('[data-route]').forEach(node => node.classList.toggle('active', node.dataset.route === page));
    window.SyuWorkbench?.onRoute?.(page);
    const current = readTab(page);
    if (current) selectTab(page, current, false);
    if (push) updateUrl(url => { url.searchParams.set('page', page); });
  }

  function selectTab(group, tab, push = true) {
    tab = normalizedTab(group, tab);
    if (!tab) return;
    $$(`[data-tab-group="${group}"]`).forEach(node => {
      const selected = node.dataset.tab === tab;
      node.classList.toggle('active', selected);
      node.setAttribute('aria-selected', String(selected));
      node.tabIndex = selected ? 0 : -1;
    });
    $$(`[data-panel-group="${group}"]`).forEach(node => { node.hidden = node.dataset.panel !== tab; });
    if (push) updateUrl(url => { url.searchParams.set(TAB_PARAM[group] || `${group}Tab`, tab); });
  }

  $$('[data-route]').forEach(node => node.addEventListener('click', () => showPage(node.dataset.route)));
  $$('[data-tab-group]').forEach(node => node.addEventListener('click', () => selectTab(node.dataset.tabGroup, node.dataset.tab)));

  function setupTabs(group) {
    const tabs = $$(`[data-tab-group="${group}"]`);
    const panels = $$(`[data-panel-group="${group}"]`);
    if (!tabs.length || !panels.length) return;
    const tablist = tabs[0].closest('.tabs');
    tablist?.setAttribute('role', 'tablist');
    tabs.forEach(tab => {
      tab.id = `${group}-tab-${tab.dataset.tab}`;
      tab.setAttribute('role', 'tab');
      tab.setAttribute('aria-controls', `${group}-panel-${tab.dataset.tab}`);
      tab.tabIndex = tab.classList.contains('active') ? 0 : -1;
    });
    panels.forEach(panel => {
      panel.id = `${group}-panel-${panel.dataset.panel}`;
      panel.setAttribute('role', 'tabpanel');
      panel.setAttribute('aria-labelledby', `${group}-tab-${panel.dataset.panel}`);
      panel.tabIndex = 0;
    });
    tablist?.addEventListener('keydown', event => {
      const current = tabs.indexOf(document.activeElement);
      if (current < 0) return;
      const nextIndex = {
        ArrowRight: (current + 1) % tabs.length,
        ArrowLeft: (current + tabs.length - 1) % tabs.length,
        Home: 0,
        End: tabs.length - 1,
      }[event.key];
      if (nextIndex === undefined) return;
      event.preventDefault();
      tabs[nextIndex].focus();
      selectTab(group, tabs[nextIndex].dataset.tab);
    });
  }

  Object.keys(DEFAULT_TAB).forEach(setupTabs);

  const overlay = $('.palette-overlay');
  const paletteInput = $('[data-palette-input]');
  $('[data-open-palette]')?.addEventListener('click', () => {
    overlay?.classList.add('open');
    paletteInput?.focus();
  });
  overlay?.addEventListener('click', event => {
    if (event.target === overlay) overlay.classList.remove('open');
  });
  paletteInput?.addEventListener('input', () => {
    const query = paletteInput.value.trim().toLowerCase();
    $$('.palette-result').forEach(node => {
      node.hidden = query && !node.textContent.toLowerCase().includes(query);
    });
  });
  $$('[data-command-route]').forEach(node => node.addEventListener('click', () => {
    const page = node.dataset.commandRoute;
    showPage(page);
    if (node.dataset.commandTab) selectTab(page, node.dataset.commandTab);
    overlay?.classList.remove('open');
    const target = node.dataset.commandFocus && $(`[data-focus-id="${node.dataset.commandFocus}"]`);
    target?.focus();
    target?.classList.add('focus-ring');
    setTimeout(() => target?.classList.remove('focus-ring'), 1800);
  }));

  addEventListener('keydown', event => {
    if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 'k') {
      event.preventDefault();
      overlay?.classList.toggle('open');
      if (overlay?.classList.contains('open')) paletteInput?.focus();
    }
    if (event.key === 'Escape') overlay?.classList.remove('open');
  });

  onpopstate = () => location.reload();

  const url = new URL(location.href);
  const page = url.searchParams.get('page') || 'work';
  showPage(page, false);
  const tab = readTab(page);
  if (tab) selectTab(page, tab, false);
  if (url.searchParams.get('palette') === '1') overlay?.classList.add('open');
})();
