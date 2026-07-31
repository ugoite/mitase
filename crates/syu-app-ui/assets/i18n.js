function initializePreferences() {
  'use strict';
  const one = (selector, root = document) => root.querySelector(selector);
  const all = (selector, root = document) => [...root.querySelectorAll(selector)];
  const supported = ['en', 'ja'];
  const params = () => new URL(location.href).searchParams;
  const osLanguage = (navigator.language || 'en').toLowerCase().startsWith('ja') ? 'ja' : 'en';
  let locale = 'en';
  let themePreference = 'system';
  let formatPreference = localStorage.getItem('syu.formatLocale') || 'interface';
  let numberFormatter = new Intl.NumberFormat('en');
  let dateFormatter = new Intl.DateTimeFormat('en');
  let preferences = (() => { try { return JSON.parse(localStorage.getItem('syu.applicationPreferences') || '{}'); } catch { return {}; } })();

  function lookup(key) {
    return window.SYU_I18N[locale]?.[key];
  }

  function required(key) {
    const value = lookup(key);
    if (value === undefined) throw new Error(`Missing ${locale} translation: ${key}`);
    return value;
  }

  function translate(language) {
    locale = supported.includes(language) ? language : osLanguage;
    document.documentElement.lang = locale;
    document.documentElement.dataset.locale = locale;
    document.title = locale === 'ja' ? 'Syu ワークベンチ' : 'Syu Workbench';
    const formatLocale = formatPreference === 'interface' ? locale : formatPreference;
    numberFormatter = new Intl.NumberFormat(formatLocale);
    dateFormatter = new Intl.DateTimeFormat(formatLocale);
    all('[data-i18n]').forEach(element => { element.textContent = required(element.dataset.i18n); });
    all('[data-i18n-placeholder]').forEach(element => { element.placeholder = required(element.dataset.i18nPlaceholder); });
    all('[data-i18n-title]').forEach(element => { element.title = required(element.dataset.i18nTitle); });
    all('[data-i18n-aria]').forEach(element => { element.setAttribute('aria-label', required(element.dataset.i18nAria)); });
    all('[data-language-select]').forEach(element => { element.value = locale; });
    localStorage.setItem('syu.locale', locale);
    document.dispatchEvent(new CustomEvent('syu:locale', { detail: { locale } }));
  }

  function applyPreferences() {
    document.documentElement.dataset.density = preferences.compact ? 'compact' : 'comfortable';
    document.documentElement.dataset.reduceMotion = preferences['reduce-motion'] ? 'true' : 'false';
    document.documentElement.dataset.focusVisibility = preferences['focus-visibility'] ? 'high' : 'normal';
    document.documentElement.dataset.statusText = preferences['status-text'] ? 'true' : 'false';
    all('[data-preference]').forEach(element => {
      const active = element.dataset.preference === 'follow-os' ? !!preferences.followOs : !!preferences[element.dataset.preference];
      element.classList.toggle('off', !active); element.setAttribute('aria-checked', String(active));
    });
    localStorage.setItem('syu.applicationPreferences', JSON.stringify(preferences));
  }

  function applyTheme(preference) {
    themePreference = ['system', 'light', 'dark'].includes(preference) ? preference : 'system';
    const resolved = themePreference === 'system'
      ? (matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light')
      : themePreference;
    document.documentElement.dataset.theme = resolved;
    document.documentElement.dataset.themePreference = themePreference;
    all('[data-theme-choice]').forEach(element => element.classList.toggle('active', element.dataset.themeChoice === themePreference));
    localStorage.setItem('syu.theme', themePreference);
  }

  function setLayer(layer, updateUrl = true) {
    all('[data-settings-layer]').forEach(element => {
      const selected = element.dataset.settingsLayer === layer;
      element.classList.toggle('active', selected);
      element.setAttribute('aria-selected', String(selected));
    });
    all('[data-settings-layer-panel]').forEach(element => { element.hidden = element.dataset.settingsLayerPanel !== layer; });
    all('[data-settings-toolbar]').forEach(element => { element.hidden = element.dataset.settingsToolbar !== layer; });
    if (updateUrl) {
      const url = new URL(location.href); url.searchParams.set('settingsLayer', layer); history.replaceState({}, '', url);
    }
    const requested = params().get('settingsPage');
    const first = one(`[data-settings-layer-panel="${layer}"] [data-settings-page]`);
    const fallback = first?.dataset.settingsPage;
    const host = one(`[data-settings-layer-panel="${layer}"]`);
    const exists = requested && one(`[data-settings-page="${requested}"]`, host);
    setPage(layer, exists ? requested : fallback, updateUrl);
  }

  function setPage(layer, page, updateUrl = true) {
    const host = one(`[data-settings-layer-panel="${layer}"]`);
    if (!host || !page) return;
    const target = one(`[data-settings-page="${page}"]`, host);
    if (!target) {
      const fallback = one('[data-settings-page]', host);
      if (!fallback) return;
      page = fallback.dataset.settingsPage;
    }
    all('[data-settings-page]', host).forEach(element => element.classList.toggle('active', element.dataset.settingsPage === page));
    all('[data-settings-page-panel]', host).forEach(element => { element.hidden = element.dataset.settingsPagePanel !== page; });
    if (updateUrl) {
      const url = new URL(location.href); url.searchParams.set('settingsPage', page); history.replaceState({}, '', url);
    }
  }

  window.SyuPreferences = { translate, theme: applyTheme, settingsLayer: setLayer, settingsPage: setPage, t: required, lookup, formatNumber: value => numberFormatter.format(value), formatDate: value => dateFormatter.format(value) };
  translate(params().get('lang') || localStorage.getItem('syu.locale') || osLanguage);
  applyTheme(params().get('theme') || localStorage.getItem('syu.theme') || 'system');
  matchMedia('(prefers-color-scheme: dark)').addEventListener('change', () => { if (themePreference === 'system') applyTheme('system'); });
  all('[data-language-select]').forEach(element => element.addEventListener('change', () => { preferences.followOs = false; applyPreferences(); translate(element.value); }));
  const formatSelect = one('#format-locale');
  if (formatSelect) {
    const values = ['interface', 'en-US', 'ja-JP']; [...formatSelect.options].forEach((option, index) => { option.value = values[index]; }); formatSelect.value = formatPreference;
    formatSelect.addEventListener('change', () => { formatPreference = formatSelect.value; localStorage.setItem('syu.formatLocale', formatPreference); translate(locale); });
  }
  all('[data-theme-choice]').forEach(element => element.addEventListener('click', () => applyTheme(element.dataset.themeChoice)));
  all('[data-settings-layer]').forEach(element => element.addEventListener('click', () => setLayer(element.dataset.settingsLayer)));
  all('[data-settings-page]').forEach(element => element.addEventListener('click', () => setPage(element.closest('[data-settings-layer-panel]').dataset.settingsLayerPanel, element.dataset.settingsPage)));
  all('[data-preference]').forEach(element => {
    element.setAttribute('role', 'switch'); element.tabIndex = 0;
    const toggle = () => { const key = element.dataset.preference; if (key === 'follow-os') { preferences.followOs = !preferences.followOs; if (preferences.followOs) translate(osLanguage); } else { preferences[key] = !preferences[key]; } applyPreferences(); };
    element.addEventListener('click', toggle); element.addEventListener('keydown', event => { if (event.key === 'Enter' || event.key === ' ') { event.preventDefault(); toggle(); } });
  });
  one('[data-reset-preferences]')?.addEventListener('click', () => { preferences = {}; applyPreferences(); translate(osLanguage); applyTheme('system'); });
  addEventListener('languagechange', () => { if (preferences.followOs) translate((navigator.language || 'en').toLowerCase().startsWith('ja') ? 'ja' : 'en'); });
  applyPreferences();
  setLayer(params().get('settingsLayer') || 'application', false);
}

initializePreferences();
