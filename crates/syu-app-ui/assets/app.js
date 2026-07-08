(() => {
  const $ = (selector, root = document) => root.querySelector(selector);
  const $$ = (selector, root = document) => [...root.querySelectorAll(selector)];

  const DEFAULT_TAB = { work: 'overview', scope: 'change', items: 'all' };
  const TAB_PARAM = { work: 'workTab', scope: 'scopeTab', items: 'itemsTab' };

  const readTab = page => new URL(location).searchParams.get(TAB_PARAM[page] || `${page}Tab`) || DEFAULT_TAB[page] || null;

  function updateUrl(mutator, push = true) {
    const url = new URL(location.href);
    mutator(url);
    if (push) history.pushState({}, '', url);
  }

  function showPage(page, push = true) {
    $$('[data-page]').forEach(node => { node.hidden = node.dataset.page !== page; });
    $$('[data-route]').forEach(node => node.classList.toggle('active', node.dataset.route === page));
    window.SyuWorkbench?.onRoute?.(page);
    const current = readTab(page);
    if (current) selectTab(page, current, false);
    if (push) updateUrl(url => { url.searchParams.set('page', page); });
  }

  function selectTab(group, tab, push = true) {
    $$(`[data-tab-group="${group}"]`).forEach(node => {
      const selected = node.dataset.tab === tab;
      node.classList.toggle('active', selected);
      node.setAttribute('aria-selected', String(selected));
    });
    $$(`[data-panel-group="${group}"]`).forEach(node => { node.hidden = node.dataset.panel !== tab; });
    if (push) updateUrl(url => { url.searchParams.set(TAB_PARAM[group] || `${group}Tab`, tab); });
  }

  $$('[data-route]').forEach(node => node.addEventListener('click', () => showPage(node.dataset.route)));
  $$('[data-tab-group]').forEach(node => node.addEventListener('click', () => selectTab(node.dataset.tabGroup, node.dataset.tab)));

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
