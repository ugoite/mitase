(() => {
  'use strict';

  const stateNode = document.querySelector('#syu-projection');
  if (!stateNode) return;

  const rawProjection = JSON.parse(stateNode.textContent);
  const projection = {
    ...rawProjection,
    items: rawProjection.items || rawProjection.specifications?.items || [],
    workspace: rawProjection.workspace || rawProjection.snapshot || {},
    requested_work: rawProjection.requested_work || rawProjection.work?.request || null,
    plan: rawProjection.plan || rawProjection.work?.plan || null,
    validation: rawProjection.validation || rawProjection.diagnostics?.validation,
  };
  const one = (selector, root = document) => root.querySelector(selector);
  const all = (selector, root = document) => [...root.querySelectorAll(selector)];
  const t = key => window.SyuPreferences.t(key);
  const text = (node, value) => { if (node) node.textContent = value ?? ''; };
  const clear = node => { if (node) node.replaceChildren(); };
  const clone = value => JSON.parse(JSON.stringify(value));
  const buttonByKey = key => one(`[data-i18n-aria="${key}"]`);
  const EMPTY_HASH = 'sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855';
  let csrfToken = '';

  const mutationBasis = () => ({
    expected_revision: projection.workspace.revision || projection.snapshot?.revision || '',
    expected_workspace_fingerprint: projection.workspace.fingerprint || projection.snapshot?.fingerprint || '',
    expected_source_hash: projection.workspace.source_hash || EMPTY_HASH,
  });

  const ACTION_ICONS = {
    edit: '<svg aria-hidden="true" viewBox="0 0 24 24"><path d="M4 20h4L19 9l-4-4L4 16v4Z"></path><path d="m13.5 6.5 4 4"></path></svg>',
    plan: '<svg aria-hidden="true" viewBox="0 0 24 24"><path d="M9 4h6l1 2h3v15H5V6h3l1-2Z"></path><path d="M9 12h6M9 16h5"></path></svg>',
    save: '<svg aria-hidden="true" viewBox="0 0 24 24"><path d="m5 12 4 4L19 6"></path></svg>',
    reset: '<svg aria-hidden="true" viewBox="0 0 24 24"><path d="M4 4v6h6"></path><path d="M5.5 15a7 7 0 1 0 .7-7.8L4 10"></path></svg>',
    preview: '<svg aria-hidden="true" viewBox="0 0 24 24"><path d="M3 12s3-6 9-6 9 6 9 6-3 6-9 6-9-6-9-6Z"></path><circle cx="12" cy="12" r="2.5"></circle></svg>',
    export: '<svg aria-hidden="true" viewBox="0 0 24 24"><path d="M12 3v12"></path><path d="m7 10 5 5 5-5"></path><path d="M5 21h14"></path></svg>',
    copy: '<svg aria-hidden="true" viewBox="0 0 24 24"><rect x="8" y="8" width="11" height="11" rx="2"></rect><path d="M5 15H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h8a2 2 0 0 1 2 2v1"></path></svg>',
    open: '<svg aria-hidden="true" viewBox="0 0 24 24"><path d="M14 4h6v6"></path><path d="M10 14 20 4"></path><path d="M20 14v6H4V4h6"></path></svg>',
    validate: '<svg aria-hidden="true" viewBox="0 0 24 24"><circle cx="12" cy="12" r="9"></circle><path d="m8 12 3 3 5-6"></path></svg>',
  };

  const requestedWork = projection.requested_work || null;
  const plan = projection.plan || null;
  let selectedAnchor = null;
  let draftWorkRequest = requestedWork ? clone(requestedWork) : defaultWorkRequest();
  let lastRun = projection.validation;
  let selectedSliceId = plan?.slices[0]?.id || null;
  let selectedScopeGroup = 'change';
  let selectedScopeTarget = null;
  let selectedScopeMode = 'plan';
  let selectedBranchEntry = null;
  let selectedBranchAnchor = null;
  let branchScope = null;
  let branchScopeLoaded = false;
  let selectedItemKind = 'all';
  let selectedItemId = null;
  let selectedContextGroup = 'editable';
  let selectedContextEntry = null;
  let selectedDiagnosticPhase = 'all';
  let selectedDiagnosticKey = null;
  let itemSearchQuery = '';
  let settingsBound = false;

  const statusLabel = status => ({
    ready: t('work.status.ready'),
    needs_review: t('work.status.needs_review'),
    blocked: t('work.status.blocked'),
  })[status] || status;

  const itemStatusLabel = value => value ? t(`items.status.${value}`) : '';
  const itemPriorityLabel = value => value ? t(`items.priority.${value}`) : '';
  const scopeGroupLabel = value => ({
    change: t('scope.change'),
    verify: t('scope.verify'),
    reference: t('work.context.reference'),
    intent: t('work.context.specification'),
  })[value] || titleCase(String(value || ''));
  const transitionLabel = value => ({
    add: t('operation.add'),
    modify: t('operation.modify'),
    remove: t('operation.remove'),
    readonly: t('work.context.reference'),
    run_only: t('work.context.verification'),
    'run-only': t('work.context.verification'),
  })[value] || titleCase(String(value || '').replaceAll('-', '_'));
  const accessLabel = value => ({
    change: t('work.context.editable'),
    editable: t('work.context.editable'),
    verify: t('work.context.verification'),
    verification: t('work.context.verification'),
    reference: t('work.context.reference'),
    readonly: t('work.context.reference'),
    intent: t('work.context.specification'),
  })[value] || titleCase(String(value || '').replaceAll('-', '_'));
  const runContextLabel = value => ({
    workspace: t('diagnostics.context.workspace'),
    git_range: t('diagnostics.context.git_range'),
    work_plan: t('diagnostics.context.work_plan'),
    slice: t('diagnostics.context.slice'),
  })[String(value || '').replaceAll('-', '_')] || titleCase(String(value || '').replaceAll('-', '_'));

  const selectorLabel = selector => {
    if (!selector) return '';
    if (typeof selector === 'string') return selector;
    if (selector.kind === 'file') return 'file';
    if (selector.kind === 'symbol') return `symbol: ${(selector.names || []).join(', ')}`;
    if (selector.kind === 'operation') return `operation: ${selector.method || ''} ${selector.path || ''}`.trim();
    if (selector.kind === 'json-pointer') return `json-pointer: ${selector.value || ''}`;
    return `${selector.kind || 'selector'}${selector.value ? `: ${selector.value}` : ''}`;
  };

  const phaseStateClass = state => ({
    passed: 'green',
    issues: 'orange',
    failed: 'red',
    running: 'blue running',
    not_applicable: 'gray',
    not_run: 'gray',
  })[state] || 'gray';

  const phaseStateA11y = state => ({
    passed: 'a11y.passed',
    issues: 'a11y.issues',
    failed: 'diagnostics.failed',
    running: 'a11y.running',
    not_applicable: 'a11y.not_applicable',
    not_run: 'a11y.not_run',
  })[state];

  async function api(url, options = {}) {
    let requestUrl = url;
    let requestOptions = { ...options };
    let payload = null;
    if (requestOptions.body) {
      try { payload = JSON.parse(requestOptions.body); } catch {}
    }
    if (requestUrl === '/api/work/request') {
      requestOptions = { ...requestOptions, method: 'POST', body: JSON.stringify({ basis: mutationBasis(), request: payload?.request || payload }) };
    } else if (requestUrl.startsWith('/api/specifications/')) {
      const anchor = requestUrl.split('/').slice(3).join('/');
      requestUrl = `/api/specifications/${anchor}`;
      const fields = payload?.fields || Object.fromEntries(Object.entries(payload || {}).filter(([key]) => !['id', 'kind', 'path', 'source_hash', 'title', 'summary', 'description', 'status', 'priority', 'principles', 'rules', 'criteria', 'bindings', 'contracts', 'anchors', 'preview_token'].includes(key)));
      for (const key of ['title', 'summary', 'description', 'status', 'priority']) {
        if (payload && Object.prototype.hasOwnProperty.call(payload, key)) fields[key] = payload[key];
      }
      requestOptions = { ...requestOptions, body: JSON.stringify({ basis: mutationBasis(), patch: { kind: 'specification', item_id: payload?.id || anchor.split('#')[0], fields }, preview_token: payload?.preview_token || null }) };
    } else if (requestUrl.startsWith('/api/config/')) {
      requestOptions = { ...requestOptions, body: JSON.stringify({ basis: mutationBasis(), patch: { kind: 'config', config: payload?.config || payload }, preview_token: payload?.preview_token || null }) };
    }
    const mutating = ['POST', 'PUT', 'DELETE'].includes((requestOptions.method || 'GET').toUpperCase());
    if (mutating && !csrfToken) {
      const csrfResponse = await fetch('/api/projection');
      csrfToken = csrfResponse.headers.get('x-syu-csrf-token') || '';
    }
    const response = await fetch(requestUrl, {
      headers: { 'content-type': 'application/json', ...(mutating && csrfToken ? { 'x-syu-csrf-token': csrfToken } : {}), ...(requestOptions.headers || {}) },
      ...requestOptions,
    });
    const body = await response.text();
    if (!response.ok) {
      let message = body;
      try {
        message = JSON.parse(body).error || body;
      } catch {}
      throw new Error(message);
    }
    try {
      return JSON.parse(body);
    } catch {
      return body;
    }
  }

  function toast(message) {
    const host = one('.toast');
    text(host, message);
    host?.classList.add('show');
    setTimeout(() => host?.classList.remove('show'), 2600);
  }

  function el(tag, className, textValue) {
    const node = document.createElement(tag);
    if (className) node.className = className;
    if (textValue !== undefined) node.textContent = textValue;
    return node;
  }

  function fragment(...nodes) {
    const out = document.createDocumentFragment();
    nodes.filter(Boolean).forEach(node => out.append(node));
    return out;
  }

  function chip(label, className = '') {
    return el('span', `chip${className ? ` ${className}` : ''}`, label);
  }

  function actionButton(label, ariaKey, onClick, className = 'btn compact', icon = 'edit') {
    const button = el('button', className);
    button.setAttribute('aria-label', t(ariaKey));
    button.dataset.i18nAria = ariaKey;
    button.dataset.i18nTitle = ariaKey;
    button.title = t(ariaKey);
    const iconWrap = el('span', 'btn-icon');
    iconWrap.innerHTML = ACTION_ICONS[icon] || ACTION_ICONS.edit;
    button.append(iconWrap, el('span', 'btn-label', label));
    if (onClick) button.addEventListener('click', onClick);
    return button;
  }

  function emptyState(titleKey, descriptionKey) {
    const wrap = el('div', 'empty-state');
    wrap.append(el('h2', '', t(titleKey)), el('p', '', t(descriptionKey)));
    return wrap;
  }

  function emptyStateWithActions(titleKey, descriptionKey, actions = []) {
    const wrap = emptyState(titleKey, descriptionKey);
    const actionWrap = el('div', 'empty-actions');
    actions.filter(Boolean).forEach(action => actionWrap.append(action));
    if (actionWrap.childElementCount) wrap.append(actionWrap);
    return wrap;
  }

  function advancedDetails(...nodes) {
    const details = el('details', 'advanced-editor');
    const summary = el('summary', '', t('common.advanced'));
    const body = el('div', 'advanced-editor-body');
    nodes.filter(Boolean).forEach(node => body.append(node));
    details.append(summary, body);
    return details;
  }

  function metaLine(values) {
    const line = el('div', 'meta-line');
    values.filter(Boolean).forEach(value => line.append(value));
    return line;
  }

  function linesList(values, bulletClass = '') {
    const list = el('ul', 'list');
    values.forEach(value => {
      const row = document.createElement('li');
      row.append(el('span', `bullet${bulletClass ? ` ${bulletClass}` : ''}`), el('span', '', value));
      list.append(row);
    });
    return list;
  }

  function summaryCard(titleKey, content) {
    const card = el('div', 'card');
    card.append(el('h3', '', t(titleKey)), content);
    return card;
  }

  function canvasHead(title, description, chips = [], actions = []) {
    const head = el('div', 'canvas-head');
    const copy = document.createElement('div');
    copy.append(el('h2', '', title), el('p', '', description));
    if (chips.length) copy.append(metaLine(chips));
    const actionWrap = el('div', 'actions');
    actions.forEach(action => actionWrap.append(action));
    head.append(copy, actionWrap);
    return head;
  }

  function currentSlice() {
    return plan?.slices.find(slice => slice.id === selectedSliceId) || plan?.slices[0] || null;
  }

  function workTargets() {
    return plan ? plan.slices.flatMap(slice => [
      ...slice.editable_targets.map(target => ({ group: 'change', slice, target })),
      ...slice.verification_targets.map(target => ({ group: 'verify', slice, target })),
      ...slice.readonly_context.map(target => ({ group: 'reference', slice, target })),
      ...slice.anchors.map(anchor => ({
        group: 'intent',
        slice,
        target: {
          reference: String(anchor),
          resolved_path: String(anchor),
          resolved_selector: { description: String(anchor) },
          reason: slice.goal,
          transition: 'readonly',
          access: 'readonly',
          adapter: 'anchor',
        },
      })),
    ]) : [];
  }

  function titleCase(value) {
    return value ? value.replaceAll('_', ' ').replace(/\b\w/g, match => match.toUpperCase()) : '';
  }

  function itemMatchesQuery(item, query) {
    if (!query) return true;
    return [
      item.id,
      item.kind,
      item.title,
      item.summary,
      item.description,
      item.path,
      ...(item.anchors || []),
      ...(item.principles || []).map(row => row.statement),
      ...(item.rules || []).map(row => row.statement),
      ...(item.criteria || []).map(row => row.statement),
    ].some(value => String(value || '').toLowerCase().includes(query));
  }

  function currentItems() {
    const query = itemSearchQuery.trim().toLowerCase();
    const order = { philosophy: 0, policy: 1, requirement: 2, feature: 3 };
    return projection.items
      .filter(item => selectedItemKind === 'all' || item.kind === selectedItemKind)
      .filter(item => itemMatchesQuery(item, query))
      .sort((left, right) => (order[left.kind] - order[right.kind]) || left.id.localeCompare(right.id));
  }

  function anchorDescription(item, anchor) {
    const row = [
      ...(item.principles || []),
      ...(item.rules || []),
      ...(item.criteria || []),
      ...(item.bindings || []),
      ...(item.contracts || []),
    ].find(candidate => candidate.anchor === anchor);
    return row?.statement || row?.responsibility || item.summary || item.description || item.title;
  }

  function renderWork() {
    const page = one('[data-page="work"]');
    text(one('[data-work-plan-label]', page), plan?.id || draftWorkRequest.id || t('common.request'));
    text(one('[data-tab="slices"] .mini-count', page), plan?.slices.length || '');
    text(one('[data-tab="validation"] .mini-count', page), plan?.diagnostics.length || '');
    renderWorkOverview();
    renderWorkSlices();
    renderWorkContext();
    renderWorkValidation();
  }

  function defaultWorkRequest() {
    // Canonical seed creation must stay exact: seeds: [selectedAnchor]
    return {
      schema: 'syu/work-request/v1',
      id: `WORK-${new Date().toISOString().slice(0, 10).replaceAll('-', '')}`,
      summary: '',
      operation: 'modify',
      seeds: selectedAnchor ? [selectedAnchor] : [],
      constraints: {
        include_facets: [],
        exclude_paths: [],
        max_slices: null,
        max_added_bytes_per_target: null,
        max_added_lines_per_target: null,
      },
      requested_targets: [],
    };
  }

  function renderWorkOverview() {
    const host = one('[data-work-overview]');
    clear(host);
    if (!host) return;
    if (!plan && host.dataset.mode !== 'editor') {
      renderWorkStart(host);
      return;
    }
    if (host.dataset.mode === 'editor') {
      renderWorkRequestEditor(host, !plan);
      return;
    }
    const body = document.createDocumentFragment();
    body.append(canvasHead(
      plan.request.summary,
      t('work.overview.description').replace('{root}', projection.workspace.root),
      [
        chip(statusLabel(plan.status), plan.status === 'blocked' ? 'red-chip' : plan.status === 'ready' ? 'green-chip' : 'orange-chip'),
        chip(titleCase(String(plan.request.operation))),
        chip(`${t('work.basis')} ${plan.basis.revision.slice(0, 9)}`),
      ],
      [actionButton(t('common.edit'), 'a11y.edit_request', () => renderWorkRequestEditor(host, false), 'btn compact', 'edit')],
    ));
    const grid = el('div', 'grid2');
    grid.append(
      summaryCard('work.card.reason', el('p', '', t('work.card.reason.body').replace('{revision}', plan.basis.revision.slice(0, 9)))),
    );
    body.append(grid);
    const constraints = [];
    if (plan.request.seeds.length) constraints.push(...plan.request.seeds.map(seed => `${t('work.seed')}: ${String(seed)}`));
    if (plan.request.constraints.include_facets?.length) constraints.push(`${t('work.facets')}: ${plan.request.constraints.include_facets.join(', ')}`);
    if (plan.request.constraints.exclude_paths?.length) constraints.push(`${t('work.exclude_paths')}: ${plan.request.constraints.exclude_paths.join(', ')}`);
    if (plan.request.constraints.max_slices) constraints.push(`${t('work.max_slices')}: ${plan.request.constraints.max_slices}`);
    body.append(summaryCard('work.card.seed', linesList(constraints.length ? constraints : [t('work.constraints.none')])));
    host.append(body);
  }

  function renderWorkStart(host) {
    const actions = el('div', 'work-start-grid');
    const choice = (icon, titleKey, descriptionKey, onClick) => {
      const button = el('button', 'work-start-card');
      button.type = 'button';
      button.append(el('span', 'work-start-icon', icon), el('b', '', t(titleKey)), el('p', '', t(descriptionKey)));
      button.addEventListener('click', onClick);
      return button;
    };
    actions.append(
      choice('⑂', 'work.start.branch', 'work.start.branch_description', () => {
        one('[data-route="scope"]')?.click();
        one('[data-scope-mode-button="branch"]')?.click();
      }),
      choice('◇', 'work.start.specification', 'work.start.specification_description', () => one('[data-route="specifications"]')?.click()),
      choice('+', 'work.start.describe', 'work.start.describe_description', () => {
        host.dataset.mode = 'editor';
        renderWorkRequestEditor(host, true);
      }),
    );
    host.append(
      canvasHead(t('work.empty.title'), t('work.empty.description')),
      actions,
    );
  }

  function renderWorkRequestEditor(host, isEmpty) {
    host.dataset.mode = 'editor';
    clear(host);
    const initial = clone(plan?.request || requestedWork || defaultWorkRequest());
    const request = clone(draftWorkRequest || initial);
    const commitDraft = () => { draftWorkRequest = clone(request); };
    const exactAnchors = projection.items.flatMap(item => item.anchors.map(anchor => ({ item, anchor })));
    const form = el('form', 'form');
    form.append(canvasHead(
      isEmpty ? t('work.intake.title') : t('work.request.editor_title'),
      isEmpty ? t('work.intake.description') : t('work.request.editor_description'),
      [chip(t('work.request.steps'), 'blue-chip')],
      [actionButton(t('common.reset'), 'common.reset', () => {
        draftWorkRequest = clone(initial);
        host.dataset.mode = '';
        renderWorkOverview();
      }, 'btn ghost compact', 'reset')],
    ));
    const steps = el('div', 'meta-line');
    steps.append(chip(t('work.request.step.seed')), chip(t('work.request.step.plan')), chip(t('work.request.step.validate')));
    form.append(steps);
    form.append(field('work.request.summary', textareaControl(request.summary || '', value => { request.summary = value; }, 'work-request-summary')));
    form.append(field('work.request.operation', selectControl(['add', 'modify', 'remove', 'refactor', 'document', 'investigate'], request.operation || 'modify', value => { request.operation = value; }, value => t(`operation.${value}`), 'work-request-operation')));
    const seedCard = el('div', 'card');
    seedCard.append(el('h3', '', t('work.request.seed')));
    const selectedSeeds = el('div', 'meta-line');
    (request.seeds || []).forEach(seed => {
      const remove = actionButton(String(seed), 'common.reset', () => {
        request.seeds = request.seeds.filter(value => String(value) !== String(seed));
        commitDraft();
        renderWorkRequestEditor(host, isEmpty);
      }, 'chip chip-button active', 'reset');
      selectedSeeds.append(remove);
    });
    seedCard.append(selectedSeeds);
    const seedSearch = inputControl('', () => {
      [...seedList.children].forEach(row => {
        row.hidden = seedSearch.value.trim() && !row.textContent.toLowerCase().includes(seedSearch.value.trim().toLowerCase());
      });
    });
    seedSearch.placeholder = t('items.search.placeholder');
    const seedList = el('div', 'settings-builder');
    exactAnchors.forEach(({ item, anchor }) => {
      const row = el('button', 'rail-subitem');
      row.type = 'button';
      row.append(el('b', '', item.title), el('span', '', `${t(`items.${item.kind}`)} · ${anchorDescription(item, anchor)}`), el('small', 'path', anchor));
      row.addEventListener('click', () => {
        request.requested_targets = [];
        request.seeds = [...new Set([...(request.seeds || []).map(String), anchor])];
        commitDraft();
        renderWorkRequestEditor(host, isEmpty);
      });
      seedList.append(row);
    });
    seedCard.append(field('items.search.label', seedSearch), seedList);
    form.append(seedCard);

    const constraints = request.constraints || {};
    request.constraints = constraints;
    const advanced = el('div', 'form');
    advanced.append(field('work.facets', inputControl((constraints.include_facets || []).join(', '), value => { constraints.include_facets = value.split(',').map(part => part.trim()).filter(Boolean); })));
    advanced.append(field('work.exclude_paths', inputControl((constraints.exclude_paths || []).join(', '), value => { constraints.exclude_paths = value.split(',').map(part => part.trim()).filter(Boolean); })));
    [
      ['work.max_slices', 'max_slices'],
      ['work.max_added_bytes_per_target', 'max_added_bytes_per_target'],
      ['work.max_added_lines_per_target', 'max_added_lines_per_target'],
    ].forEach(([labelKey, key]) => {
      const input = inputControl(constraints[key] ?? '', value => { constraints[key] = value.trim() ? Number(value) : null; });
      input.type = 'number';
      input.min = '0';
      input.step = '1';
      advanced.append(field(labelKey, input));
    });
    const targetCard = el('div', 'card');
    targetCard.append(el('h3', '', t('work.request.targets')));
    (request.requested_targets || []).forEach((target, index) => {
      const row = el('div', 'form two-column');
      row.append(
        field('work.request.target_ref', inputControl(target.ref || '', value => { target.ref = value; request.seeds = []; })),
        field('work.request.target_criterion', inputControl(target.criterion || '', value => { target.criterion = value.trim() || null; })),
        field('work.request.target_transition', selectControl(['add', 'modify', 'remove', 'run-only', 'readonly'], target.transition || 'modify', value => { target.transition = value; })),
        actionButton(t('common.reset'), 'common.reset', () => {
          request.requested_targets.splice(index, 1);
          commitDraft();
          renderWorkRequestEditor(host, isEmpty);
        }, 'btn ghost compact', 'reset'),
      );
      targetCard.append(row);
    });
    targetCard.append(actionButton(t('common.new'), 'a11y.new_item_row', () => {
      request.seeds = [];
      request.requested_targets = [...(request.requested_targets || []), { ref: '', criterion: null, transition: 'modify' }];
      commitDraft();
      renderWorkRequestEditor(host, isEmpty);
    }, 'btn compact', 'plan'));
    advanced.append(targetCard);
    form.append(advancedDetails(advanced));
    const actions = el('div', 'actions');
    const planButton = actionButton(t('common.plan'), 'a11y.work_plan', null, 'btn primary compact', 'plan');
    planButton.type = 'submit';
    actions.append(planButton);
    form.append(actions);
    form.addEventListener('submit', async event => {
      event.preventDefault();
      if (!request.summary?.trim()) return toast(t('work.request.summary_required'));
      if ((request.seeds || []).length && (request.requested_targets || []).length) return toast(t('work.request.seed_target_exclusive'));
      for (const key of ['max_slices', 'max_added_bytes_per_target', 'max_added_lines_per_target']) {
        const value = request.constraints?.[key];
        if (value !== null && value !== undefined && (!Number.isInteger(value) || value < 0)) return toast(t('settings.number_invalid'));
      }
      draftWorkRequest = clone(request);
      await api('/api/work/request', { method: 'POST', body: JSON.stringify({ basis: mutationBasis(), request }) });
      location.assign('/?page=work&workTab=overview');
    });
    host.append(form);
  }

  function renderWorkSlices() {
    const rail = one('[data-work-slices-rail]');
    const detail = one('[data-work-slice-detail]');
    clear(rail);
    clear(detail);
    if (!plan) {
      detail?.append(emptyState('work.slices.empty.title', 'work.slices.empty.description'));
      return;
    }
    rail?.append(el('div', 'rail-title', t('work.slices.title')));
    plan.slices.forEach(slice => {
      const button = el('button', `rail-item${(selectedSliceId || plan.slices[0].id) === slice.id ? ' active' : ''}`);
      const label = document.createElement('span');
      label.append(el('b', '', slice.id), el('p', '', slice.goal));
      button.append(el('span', 'status-circle green'), label, el('span', 'n', String(slice.editable_targets.length)));
      button.addEventListener('click', () => {
        selectedSliceId = slice.id;
        selectedContextEntry = null;
        renderWorkSlices();
        renderWorkContext();
        renderScope();
      });
      rail?.append(button);
    });
    const slice = currentSlice();
    if (!slice) return;
    const targetsCard = el('div', 'card');
    targetsCard.style.padding = '8px 12px';
    [...slice.editable_targets, ...slice.verification_targets, ...slice.readonly_context].forEach(target => {
      const row = el('div', 'path-row');
      row.append(el('span', 'path', target.resolved_path), chip(target.transition), chip(target.access), el('span', '', target.reason));
      targetsCard.append(row);
    });
    detail?.append(
      canvasHead(
        `${slice.id} · ${slice.goal}`,
        slice.acceptance[0]?.statement || slice.goal,
        [
          chip(titleCase(String(slice.confidence || 'exact')), 'green-chip'),
          chip(`${slice.editable_targets.length} ${t('work.context.editable').toLowerCase()}`),
          chip(`${slice.verification_targets.length} ${t('work.context.verification').toLowerCase()}`),
          chip(`${slice.readonly_context.length} ${t('work.context.reference').toLowerCase()}`),
        ],
        [
          actionButton(t('common.copy'), 'a11y.copy_slice', async () => {
            await navigator.clipboard.writeText([slice.id, slice.goal, ...slice.anchors.map(String)].join('\n'));
            toast(t('toast.locator_copied'));
          }, 'btn compact', 'copy'),
          actionButton(t('common.replan'), 'a11y.work_plan', () => renderWorkRequestEditor(one('[data-work-slice-detail]'), false), 'btn compact', 'plan'),
          actionButton(t('common.export'), 'a11y.export_slice', async () => {
            const yaml = await api('/api/work/context', { method: 'POST', body: JSON.stringify({ basis: mutationBasis(), slice_id: slice.id }) });
            const link = document.createElement('a');
            link.href = URL.createObjectURL(new Blob([yaml], { type: 'application/yaml' }));
            link.download = `${slice.id}-context.yaml`;
            link.click();
          }, 'btn primary compact', 'export'),
        ],
      ),
      (() => {
        const grid = el('div', 'grid2');
        grid.append(
          summaryCard('work.goal', el('p', '', slice.goal)),
          summaryCard('work.acceptance', linesList(slice.acceptance.map(entry => entry.statement), 'green')),
        );
        return grid;
      })(),
      el('div', 'section-label', t('scope.exact_targets')),
      targetsCard,
    );
  }

  function contextGroupsForSlice(slice) {
    return [
      { key: 'editable', label: t('work.context.editable'), items: slice.editable_targets, color: 'red' },
      { key: 'verification', label: t('work.context.verification'), items: slice.verification_targets, color: 'blue' },
      { key: 'reference', label: t('work.context.reference'), items: slice.readonly_context, color: 'gray' },
      {
        key: 'specification',
        label: t('work.context.specification'),
        items: slice.anchors.map(anchor => ({ reference: String(anchor), resolved_path: String(anchor), resolved_selector: { description: String(anchor) }, reason: slice.goal })),
        color: 'purple',
      },
    ];
  }

  function renderWorkContext() {
    const rail = one('[data-work-context-rail]');
    const detail = one('[data-work-context-detail]');
    clear(rail);
    clear(detail);
    if (!plan) {
      detail?.append(emptyState('work.context.empty.title', 'work.context.empty.description'));
      return;
    }
    const slice = currentSlice();
    if (!slice) return;
    const groups = contextGroupsForSlice(slice);
    const selected = groups.find(group => group.key === selectedContextGroup) || groups[0];
    selectedContextGroup = selected.key;
    if (!selectedContextEntry || !selected.items.some(item => item.reference === selectedContextEntry.reference)) {
      selectedContextEntry = selected.items[0] || null;
    }
    rail?.append(el('div', 'rail-title', t('work.context.groups')));
    groups.forEach(group => {
      const header = el('button', `rail-item${group.key === selectedContextGroup ? ' active' : ''}`);
      const label = document.createElement('span');
      label.append(el('b', '', group.label), el('p', '', t('common.items_count').replace('{count}', group.items.length)));
      header.append(el('span', `status-circle ${group.color}`), label, el('span', 'n', String(group.items.length)));
      header.addEventListener('click', () => {
        selectedContextGroup = group.key;
        selectedContextEntry = group.items[0] || null;
        renderWorkContext();
      });
      rail?.append(header);
      if (group.key === selectedContextGroup) {
        group.items.forEach(item => {
          const row = el('button', `rail-subitem${selectedContextEntry?.reference === item.reference ? ' active' : ''}`);
          row.append(el('span', 'path', item.resolved_path), el('small', '', item.resolved_selector.description));
          row.addEventListener('click', () => {
            selectedContextEntry = item;
            renderWorkContext();
          });
          rail?.append(row);
        });
      }
    });
    renderWorkContextDetail(detail, slice, selected, selectedContextEntry);
  }

  function renderWorkContextDetail(host, slice, selectedGroup, item) {
    if (!item) {
      host?.append(emptyState('work.context.empty.title', 'work.context.empty.description'));
      return;
    }
    host?.append(
      canvasHead(
        `${t('work.context.title')} · ${slice.id}`,
        t('work.context.description').replace('{slice}', slice.id),
        [chip(selectedGroup.label), chip(t('work.context.ready'), 'green-chip')],
        [actionButton(t('common.download'), 'a11y.download_context', async () => {
            const yaml = await api('/api/work/context', { method: 'POST', body: JSON.stringify({ basis: mutationBasis(), slice_id: slice.id }) });
          const link = document.createElement('a');
          link.href = URL.createObjectURL(new Blob([yaml], { type: 'application/yaml' }));
          link.download = `${slice.id}-context.yaml`;
          link.click();
        }, 'btn primary compact', 'export')],
      ),
      (() => {
        const locator = el('div', 'target-locator');
        locator.append(el('div', 'path', item.resolved_path), el('div', 'selector', item.reference || item.resolved_selector.description));
        return locator;
      })(),
      summaryCard('work.context.instruction', el('p', '', item.reason || slice.goal)),
      summaryCard('work.context.source', (() => {
        const code = el('div', 'code');
        code.textContent = `${item.resolved_path}\n${item.resolved_selector.description}`;
        return code;
      })()),
    );
  }

  function renderWorkValidation() {
    const rail = one('[data-work-validation-rail]');
    const detail = one('[data-work-validation-detail]');
    clear(rail);
    clear(detail);
    if (!plan) {
      detail?.append(emptyState('work.validation.empty.title', 'work.validation.empty.description'));
      return;
    }
    rail?.append(el('div', 'rail-title', t('work.validation.plan_diagnostics')));
    if (!plan.diagnostics.length) {
      detail?.append(canvasHead(
        t('diagnostics.not_run.title'),
        t('diagnostics.not_run.description'),
        [],
        [actionButton(t('filter.validate'), 'a11y.validate_plan', async event => {
          const button = event.currentTarget;
          button.disabled = true;
          try {
            const next = await api('/api/work/validate', { method: 'POST', body: JSON.stringify(mutationBasis()) });
            lastRun = next;
            openWorkPage('validation');
            toast(t('filter.validate'));
          } catch (error) {
            lastRun = failedValidationRun('work_plan', error.message);
            toast(error.message);
          } finally {
            button.disabled = false;
          }
        }, 'btn primary compact', 'validate')],
      ));
      return;
    }
    const selected = plan.diagnostics.find(diagnostic => diagnostic.rule_id === selectedDiagnosticKey) || plan.diagnostics[0];
    selectedDiagnosticKey = selected.rule_id;
    plan.diagnostics.forEach((diagnostic, index) => {
      const button = el('button', `rail-item${diagnostic.rule_id === selectedDiagnosticKey ? ' active' : ''}`);
      const label = document.createElement('span');
      label.append(el('b', '', diagnostic.rule_id), el('p', '', diagnostic.message));
      button.append(el('span', `status-circle ${diagnostic.severity === 'error' ? 'red' : diagnostic.severity === 'warning' ? 'orange' : 'blue'}`), label, el('span', 'n', String(index + 1)));
      button.addEventListener('click', () => {
        selectedDiagnosticKey = diagnostic.rule_id;
        renderWorkValidation();
      });
      rail?.append(button);
    });
    renderPlanDiagnostic(selected);
  }

  function renderPlanDiagnostic(diagnostic) {
    const detail = one('[data-work-validation-detail]');
    clear(detail);
    detail?.append(
      canvasHead(
        diagnostic.rule_id,
        diagnostic.message,
        [chip(diagnostic.severity, diagnostic.severity === 'error' ? 'red-chip' : diagnostic.severity === 'warning' ? 'orange-chip' : 'blue-chip')],
        [actionButton(t('filter.validate'), 'a11y.validate_plan', () => location.assign('/?page=diagnostics'), 'btn primary compact', 'validate')],
      ),
      summaryCard('work.validation.selected', el('p', '', diagnostic.help || diagnostic.message)),
    );
  }

  async function ensureBranchScope(force = false) {
    if (branchScopeLoaded && !force) return;
    branchScopeLoaded = true;
    const range = one('[data-scope-range]')?.value?.trim();
    const suffix = range ? `?range=${encodeURIComponent(range)}` : '';
    branchScope = await api(`/api/scope/branch${suffix}`);
  }

  function renderScope() {
    const page = one('[data-page="scope"]');
    const planControl = one('[data-scope-plan-control]', page);
    const rangeControl = one('[data-scope-range-control]', page);
    const createButton = one('[data-scope-create-work]', page);
    all('[data-scope-mode-button]', page).forEach(node => node.classList.toggle('active', node.dataset.scopeModeButton === selectedScopeMode));
    planControl.hidden = selectedScopeMode !== 'plan';
    rangeControl.hidden = selectedScopeMode !== 'branch';
    text(one('[data-scope-plan-label]', page), plan?.id || draftWorkRequest.id || t('common.request'));
    text(one('[data-scope-slice-label]', page), currentSlice()?.id || '');
    if (selectedScopeMode === 'branch') {
      renderBranchScope();
      return;
    }
    const groups = {
      change: workTargets().filter(entry => entry.group === 'change'),
      verify: workTargets().filter(entry => entry.group === 'verify'),
      reference: workTargets().filter(entry => entry.group === 'reference'),
      intent: workTargets().filter(entry => entry.group === 'intent'),
    };
    ['change', 'verify', 'reference', 'intent'].forEach(group => {
      text(one(`[data-tab="${group}"] .mini-count`, page), groups[group].length || '');
    });
    const rail = one('[data-scope-rail]');
    const detail = one('[data-scope-detail]');
    clear(rail);
    clear(detail);
    if (!plan) {
      if (createButton) createButton.disabled = true;
      detail?.append(emptyStateWithActions('scope.empty.title', 'scope.empty.description', [
        actionButton(t('scope.mode.branch'), 'scope.mode.branch', () => one('[data-scope-mode-button="branch"]')?.click(), 'btn primary compact', 'open'),
        actionButton(t('work.start.specification'), 'a11y.open_items', () => one('[data-route="specifications"]')?.click(), 'btn compact', 'open'),
      ]));
      return;
    }
    rail?.append(el('div', 'rail-title', t('scope.exact_targets')));
    const visible = groups[selectedScopeGroup] || groups.change;
    if (!selectedScopeTarget || !visible.some(entry => entry.target.reference === selectedScopeTarget.reference)) {
      selectedScopeTarget = visible[0]?.target || null;
    }
    visible.forEach(entry => {
      const button = el('button', `rail-item${selectedScopeTarget?.reference === entry.target.reference ? ' active' : ''}`);
      const label = document.createElement('span');
      label.append(el('b', '', entry.target.resolved_selector.description), el('p', '', entry.target.resolved_path));
      button.append(el('span', `status-circle ${entry.group === 'change' ? 'red' : entry.group === 'verify' ? 'blue' : entry.group === 'reference' ? 'gray' : 'purple'}`), label);
      button.addEventListener('click', () => {
        selectedScopeTarget = entry.target;
        renderScope();
      });
      rail?.append(button);
    });
    const selectedEntry = visible.find(entry => entry.target.reference === selectedScopeTarget?.reference) || visible[0] || null;
    if (createButton) createButton.disabled = !selectedEntry;
    renderScopeDetail(selectedEntry);
  }

  function renderScopeDetail(entry) {
    const detail = one('[data-scope-detail]');
    clear(detail);
    if (!entry) {
      detail?.append(emptyState('scope.empty.title', 'scope.empty.description'));
      return;
    }
    const target = entry.target;
    const specification = entry.slice.anchors.map(String);
    detail?.append(
      canvasHead(
        target.resolved_selector.description,
        '',
        [chip(scopeGroupLabel(entry.group)), chip(entry.slice.id)],
        [
          actionButton(t('common.copy'), 'a11y.copy_locator', async () => {
            await navigator.clipboard.writeText(target.reference);
            toast(t('toast.locator_copied'));
          }, 'btn compact', 'copy'),
          actionButton(t('common.open'), 'a11y.open_source', () => toast(target.resolved_path), 'btn compact', 'open'),
        ],
      ),
      summaryCard('scope.why', el('p', '', target.reason)),
      summaryCard('scope.specification', linesList(specification.length ? specification : [target.reference], 'purple')),
      summaryCard('scope.code', (() => {
        const locator = el('div', 'target-locator flat');
        locator.append(el('div', 'path', target.resolved_path), el('div', 'selector', target.reference));
        return locator;
      })()),
      advancedDetails(summaryCard('scope.lifecycle', linesList([
        `${t('scope.transition')}: ${transitionLabel(target.transition || entry.group)}`,
        `${t('scope.access')}: ${accessLabel(target.access || entry.group)}`,
        `${t('scope.adapter')}: ${target.adapter || 'anchor'}`,
      ]))),
    );
  }

  function renderBranchScope() {
    const rail = one('[data-scope-rail]');
    const detail = one('[data-scope-detail]');
    const createButton = one('[data-scope-create-work]', one('[data-page="scope"]'));
    clear(rail);
    clear(detail);
    ['change', 'verify', 'reference', 'intent'].forEach(group => {
      text(one(`[data-tab="${group}"] .mini-count`, one('[data-page="scope"]')), group === 'change' ? (branchScope?.changed?.length || '') : '');
    });
    if (!branchScope) {
      if (createButton) createButton.disabled = true;
      detail?.append(emptyState('scope.branch.loading.title', 'scope.branch.loading.description'));
      return;
    }
    if (branchScope.state !== 'ready') {
      if (createButton) createButton.disabled = true;
      detail?.append(
        canvasHead(t('scope.mode.branch'), branchScope.reason || t('scope.branch.not_applicable.description'), [chip(t('diagnostics.not_applicable'))], []),
        el('div', 'notice warn', branchScope.reason || t('scope.branch.not_applicable.description')),
      );
      return;
    }
    rail?.append(el('div', 'rail-title', `${t('scope.mode.branch')} · ${branchScope.range}`));
    if (!selectedBranchEntry || !branchScope.changed.some(entry => entry.path === selectedBranchEntry.path)) {
      selectedBranchEntry = branchScope.changed[0] || null;
      selectedBranchAnchor = selectedBranchEntry?.anchors?.[0] || null;
    }
    if (selectedBranchEntry && !selectedBranchEntry.anchors.includes(selectedBranchAnchor)) {
      selectedBranchAnchor = selectedBranchEntry.anchors[0] || null;
    }
    branchScope.changed.forEach(entry => {
      const button = el('button', `rail-item${selectedBranchEntry?.path === entry.path ? ' active' : ''}`);
      const label = document.createElement('span');
      label.append(el('b', '', entry.path), el('p', '', entry.owners.length ? entry.owners.join(', ') : t('scope.branch.unowned')));
      button.append(el('span', `status-circle ${entry.owners.length ? 'orange' : 'gray'}`), label);
      button.addEventListener('click', () => {
        selectedBranchEntry = entry;
        selectedBranchAnchor = entry.anchors[0] || null;
        renderBranchScope();
      });
      rail?.append(button);
    });
    if (!selectedBranchEntry) {
      if (createButton) createButton.disabled = true;
      detail?.append(emptyStateWithActions('scope.branch.empty.title', 'scope.branch.empty.description', [
        actionButton(t('work.start.specification'), 'a11y.open_items', () => one('[data-route="specifications"]')?.click(), 'btn primary compact', 'open'),
      ]));
      return;
    }
    if (createButton) createButton.disabled = !selectedBranchAnchor;
    detail?.append(
      canvasHead(
        selectedBranchEntry.path,
        '',
        [chip(selectedBranchEntry.status), chip(selectedBranchEntry.owners.length ? t('scope.branch.owned') : t('scope.branch.unowned'))],
        [],
      ),
      summaryCard('scope.why', el('p', '', selectedBranchEntry.owners.length ? selectedBranchEntry.owners.join(', ') : t('scope.branch.unowned_description'))),
      (() => {
        if (!selectedBranchEntry.anchors.length) {
          return summaryCard('items.bindings', linesList([t('scope.branch.no_anchor')]));
        }
        const card = el('div', 'card');
        card.append(el('h3', '', t('scope.specification')));
        selectedBranchEntry.anchors.forEach(anchor => {
          const button = el('button', `rail-subitem${selectedBranchAnchor === anchor ? ' active' : ''}`);
          button.append(el('span', 'path', anchor));
          button.addEventListener('click', () => {
            selectedBranchAnchor = anchor;
            renderBranchScope();
          });
          card.append(button);
        });
        return card;
      })()
    );
  }

  function renderItemRailRow(rail, item) {
    const button = el('button', `rail-item${item.id === selectedItemId ? ' active' : ''}`);
    const label = document.createElement('span');
    label.append(el('b', '', item.id), el('p', '', item.title));
    button.append(label, el('span', 'n', String(item.anchors.length)));
    button.addEventListener('click', () => {
      selectedItemId = item.id;
      renderItems();
    });
    rail?.append(button);
  }

  function appendItemGroup(rail, kind, items) {
    if (selectedItemKind === 'all') rail?.append(el('div', 'rail-title rail-section', t(`items.${kind}`)));
    items.forEach(item => renderItemRailRow(rail, item));
  }

  function renderItems() {
    const page = one('[data-page="specifications"]');
    text(one('[data-tab="all"] .mini-count', page), String(projection.items.length));
    ['philosophy', 'policy', 'requirement', 'feature'].forEach(kind => {
      text(one(`[data-tab="${kind}"] .mini-count`, page), projection.items.filter(item => item.kind === kind).length || '');
    });
    const rail = one('[data-items-rail]');
    const detail = one('[data-items-detail]');
    clear(rail);
    clear(detail);
    const visible = currentItems();
    if (!visible.some(item => item.id === selectedItemId)) selectedItemId = visible[0]?.id || null;
    if (selectedItemKind === 'all') {
      ['philosophy', 'policy', 'requirement', 'feature'].forEach(kind => appendItemGroup(rail, kind, visible.filter(item => item.kind === kind)));
    } else {
      rail?.append(el('div', 'rail-title', t(`items.${selectedItemKind}`)));
      visible.forEach(item => renderItemRailRow(rail, item));
    }
    if (!visible.length) {
      detail?.append(emptyState('items.empty.title', 'items.empty.description'));
      return;
    }
    const item = visible.find(candidate => candidate.id === selectedItemId) || visible[0];
    if (selectedAnchor && !item.anchors.includes(selectedAnchor)) selectedAnchor = null;
    renderItemDetail(detail, item);
  }

  function renderReadiness() {
    const host = one('[data-readiness-content]');
    if (!host) return;
    clear(host);
    const readiness = projection.readiness || { status: 'Not run', target: 'unknown', axes: {}, blockers: [] };
    const rows = Object.entries(readiness.axes || {}).map(([name, axis]) => {
      const row = el('div', 'card');
      row.append(el('h3', '', titleCase(name.replaceAll('_', ' '))));
      row.append(el('p', '', `${axis.ready}/${axis.required}`));
      if (axis.blockers?.length) row.append(linesList(axis.blockers, 'red'));
      return row;
    });
    host.append(canvasHead(`${readiness.target} · ${readiness.status}`, 'Readiness is evaluated explicitly from the canonical server state.', [], []));
    rows.forEach(row => host.append(row));
    if (readiness.blockers?.length) host.append(summaryCard('diagnostics.issues', linesList(readiness.blockers, 'red')));
  }

  function renderItemDetail(detail, item) {
    const summary = item.description || item.summary || item.path;
    detail?.append(
      canvasHead(
        item.title,
        summary,
        [chip(item.id), chip(t(`items.${item.kind}`)), item.status ? chip(itemStatusLabel(item.status), 'green-chip') : null, item.priority ? chip(itemPriorityLabel(item.priority)) : null],
        [
          actionButton(t('common.edit'), 'a11y.edit_item', () => openItemEditor(item), 'btn compact', 'edit'),
          (() => {
            const button = actionButton(t('items.create_work'), 'a11y.create_work', async () => {
            if (!selectedAnchor) return toast(t('toast.select_anchor'));
            const request = defaultWorkRequest();
            request.id = `WORK-${Date.now()}`;
            request.summary = t('work.request.summary_from_anchor').replace('{anchor}', selectedAnchor);
            request.seeds = [selectedAnchor];
            draftWorkRequest = request;
            await api('/api/work/request', { method: 'POST', body: JSON.stringify({ basis: mutationBasis(), request }) });
            location.assign('/?page=work&workTab=overview');
            }, 'btn primary compact', 'plan');
            button.disabled = !selectedAnchor;
            return button;
          })(),
        ],
      ),
      summaryCard('items.planner', renderAnchorPicker(item)),
      ...renderItemSections(item),
    );
  }

  function renderAnchorPicker(item) {
    const wrap = document.createElement('div');
    if (!item.anchors.length) {
      wrap.append(el('p', '', t('items.no_exact_seed')));
      return wrap;
    }
    item.anchors.forEach(anchor => {
      const label = el('label', 'radio-row');
      const radio = document.createElement('input');
      radio.type = 'radio';
      radio.name = `anchor-${item.id}`;
      radio.checked = selectedAnchor === anchor;
      radio.addEventListener('change', () => {
        selectedAnchor = anchor;
        renderItems();
      });
      const copy = el('span', 'anchor-choice');
      copy.append(el('b', '', anchorDescription(item, anchor)), el('small', 'path', anchor));
      label.append(radio, copy);
      wrap.append(label);
    });
    return wrap;
  }

  function renderItemSections(item) {
    const sections = [];
    if (item.principles.length) sections.push(renderStatementSection(t('items.principles'), item.principles.map(row => ({ title: row.statement, anchor: row.anchor, meta: [['items.field.applies_to', row.applies_to]] }))));
    if (item.rules.length) sections.push(renderStatementSection(t('items.rules'), item.rules.map(row => ({ title: row.statement, anchor: row.anchor, meta: [['items.field.level', [row.level]], ['items.field.governed_by', row.governed_by], ['items.field.applies_to_roles', row.applies_to_roles], ['items.field.enforcement', row.enforcement ? [row.enforcement] : []]] }))));
    if (item.criteria.length) sections.push(renderStatementSection(t('items.criteria'), item.criteria.map(row => ({ title: row.statement, anchor: row.anchor, meta: [['items.field.kind', [row.kind]], ['items.field.governed_by', row.governed_by]] }))));
    if (item.bindings.length) sections.push(renderBindingSection(item.bindings));
    if (item.contracts.length) sections.push(renderContractSection(item.contracts));
    return sections;
  }

  function renderStatementSection(label, values) {
    const title = el('div', 'section-label', label);
    const card = el('div', 'card');
    values.forEach(value => {
      const block = el('div', 'target-locator');
      block.append(el('div', 'selector', value.title), el('div', 'path', value.anchor));
      (value.meta || []).forEach(([labelKey, entries]) => {
        if (!entries?.length) return;
        block.append(el('p', '', `${t(labelKey)}: ${entries.join(', ')}`));
      });
      card.append(block);
    });
    return fragment(title, card);
  }

  function renderBindingSection(bindings) {
    const title = el('div', 'section-label', t('items.bindings'));
    const card = el('div', 'card');
    bindings.forEach(binding => {
      const block = el('div', 'target-locator');
      block.append(el('div', 'selector', binding.responsibility || binding.facet), el('div', 'path', binding.anchor), metaLine([chip(binding.role), chip(binding.facet)]));
      const targets = binding.targets?.length ? binding.targets : [{ reference: t('items.no_targets'), path: '', selector: null, adapter: '' }];
      targets.forEach(target => {
        const row = el('div', 'path-row');
        row.append(el('span', 'path', target.reference), chip(target.adapter || t('items.no_targets')), el('span', '', [target.path, selectorLabel(target.selector)].filter(Boolean).join(' · ')));
        block.append(row);
        (target.claims || []).forEach(claim => block.append(el('p', '', `${claim.kind}: ${claim.criterion || claim.anchor || claim.rule || ''}`)));
      });
      card.append(block);
    });
    return fragment(title, card);
  }

  function renderContractSection(contracts) {
    const title = el('div', 'section-label', t('items.contracts'));
    const card = el('div', 'card');
    contracts.forEach(contract => {
      const block = el('div', 'target-locator');
      block.append(el('div', 'selector', `${contract.kind} · ${contract.source}`), el('div', 'path', contract.anchor));
      if (contract.participants?.length) block.append(el('p', '', `${t('items.field.participants')}: ${contract.participants.map(row => `${row.role}: ${row.binding}`).join(', ')}`));
      if (contract.guarantees?.length) block.append(el('p', '', `${t('items.field.guarantees')}: ${contract.guarantees.join(', ')}`));
      card.append(block);
    });
    return fragment(title, card);
  }

  function field(labelKey, control) {
    const wrap = el('div', 'field');
    const label = el('label', '', t(labelKey));
    if (control.id) label.htmlFor = control.id;
    wrap.append(label, control);
    return wrap;
  }

  function inputControl(value, onInput, id = '') {
    const input = el('input', 'input');
    input.value = value || '';
    if (id) input.id = id;
    input.addEventListener('input', event => onInput(event.target.value));
    return input;
  }

  function textareaControl(value, onInput, id = '') {
    const input = el('textarea', 'textarea');
    input.value = value || '';
    if (id) input.id = id;
    input.addEventListener('input', event => onInput(event.target.value));
    return input;
  }

  function selectControl(values, current, onInput, renderLabel = value => value, id = '') {
    const select = el('select', 'native-select');
    if (id) select.id = id;
    values.forEach(value => {
      const option = document.createElement('option');
      option.value = value;
      option.textContent = renderLabel(value);
      option.selected = value === current;
      select.append(option);
    });
    select.addEventListener('change', event => onInput(event.target.value));
    return select;
  }

  function statementEditor(labelKey, rows, columns, createRow, onAdd) {
    const wrap = el('div', 'card');
    wrap.append(el('h3', '', t(labelKey)));
    rows.forEach((row, index) => {
      const rowWrap = el('div', 'form two-column');
      columns.forEach(column => {
        rowWrap.append(field(column.labelKey, column.control(row, index)));
      });
      const actions = el('div', 'actions');
      const up = actionButton('↑', 'a11y.move_up', () => {
        if (index <= 0) return;
        [rows[index - 1], rows[index]] = [rows[index], rows[index - 1]];
        onAdd?.();
      }, 'btn compact', 'open');
      const down = actionButton('↓', 'a11y.move_down', () => {
        if (index >= rows.length - 1) return;
        [rows[index + 1], rows[index]] = [rows[index], rows[index + 1]];
        onAdd?.();
      }, 'btn compact', 'open');
      const del = actionButton(t('common.delete'), 'common.delete', () => {
        if (!confirm(t('items.delete_confirm'))) return;
        rows.splice(index, 1);
        onAdd?.();
      }, 'btn ghost compact', 'reset');
      actions.append(up, down, del);
      rowWrap.append(actions);
      wrap.append(rowWrap);
    });
    const addButton = actionButton(t('common.new'), 'a11y.new_item_row', () => {
      rows.push(createRow(rows.length));
      onAdd?.();
    }, 'btn compact', 'plan');
    wrap.append(addButton);
    return wrap;
  }

  function csvEditor(labelKey, value, onInput) {
    return field(labelKey, inputControl((value || []).join(', '), next => onInput(next.split(',').map(part => part.trim()).filter(Boolean))));
  }

  function openItemEditor(item) {
    const detail = one('[data-items-detail]');
    clear(detail);
    const draft = clone(item);
    draft.expected_hash = item.source_hash || EMPTY_HASH;
    let previewToken = null;
    const apply = actionButton(t('common.apply'), 'common.apply', async () => {
      try {
        const result = await api(`/api/specifications/${encodeURIComponent(draft.id)}/apply`, { method: 'PUT', body: JSON.stringify({ ...draft, preview_token: previewToken }) });
        toast(`${t('common.apply')} · ${result.changed_lines}`);
        location.reload();
      } catch (error) {
        toast(error.message);
      }
    }, 'btn primary compact', 'save');
    apply.disabled = true;
    detail?.append(
      canvasHead(
        draft.id,
        t('items.edit.description'),
        [chip(t(`items.${draft.kind}`))],
        [
          actionButton(t('common.reset'), 'common.reset', () => renderItems(), 'btn ghost compact', 'reset'),
          actionButton(t('common.preview'), 'common.preview', async () => {
            try {
              const result = await api(`/api/specifications/${encodeURIComponent(draft.id)}/preview`, { method: 'POST', body: JSON.stringify({ ...draft }) });
              previewToken = result.preview_token;
              apply.disabled = !previewToken;
              toast(`${result.changed_lines} ${t('settings.changed_lines')}`);
            } catch (error) {
              toast(error.message);
            }
          }, 'btn compact', 'preview'),
          apply,
        ],
      ),
      renderItemEditForm(draft),
    );
  }

  function renderItemEditForm(draft) {
    const form = el('div', 'form');
    const rerender = () => openItemEditor(draft);
    form.append(field('items.field.title', inputControl(draft.title, value => { draft.title = value; })));
    if (draft.kind === 'requirement') {
      form.append(field('items.field.description', textareaControl(draft.description || '', value => { draft.description = value; })));
    } else if (draft.kind === 'feature') {
      form.append(field('items.field.expected_behavior', textareaControl(draft.summary || '', value => { draft.summary = value; })));
    } else {
      form.append(field('items.field.summary', textareaControl(draft.summary || '', value => { draft.summary = value; })));
      if (draft.kind === 'policy') form.append(field('items.field.description', textareaControl(draft.description || '', value => { draft.description = value; })));
    }
    if (draft.status !== null && draft.status !== undefined) {
      form.append(field('items.field.status', selectControl(['planned', 'implemented', 'deprecated'], draft.status, value => { draft.status = value; }, value => t(`items.status.${value}`))));
    }
    if (draft.priority !== null && draft.priority !== undefined) {
      form.append(field('items.field.priority', selectControl(['low', 'medium', 'high', 'critical'], draft.priority, value => { draft.priority = value; }, value => t(`items.priority.${value}`))));
    }
    if (draft.kind === 'philosophy') {
      draft.principles ||= [];
      form.append(statementEditor('items.principles', draft.principles, [
        { labelKey: 'items.field.statement', control: row => textareaControl(row.statement, value => { row.statement = value; }) },
      ], index => ({ anchor: `${draft.id}#principle.row-${index + 1}`, statement: '', applies_to: [] }), rerender));
    }
    if (draft.kind === 'policy') {
      draft.rules ||= [];
      form.append(statementEditor('items.rules', draft.rules, [
        { labelKey: 'items.field.level', control: row => selectControl(['must', 'should', 'may'], row.level, value => { row.level = value; }) },
        { labelKey: 'items.field.statement', control: row => textareaControl(row.statement, value => { row.statement = value; }) },
      ], index => ({ anchor: `${draft.id}#rule.row-${index + 1}`, level: 'must', statement: '', governed_by: [], applies_to_roles: [], enforcement: null }), rerender));
    }
    if (draft.kind === 'requirement') {
      draft.criteria ||= [];
      form.append(statementEditor('items.criteria', draft.criteria, [
        { labelKey: 'items.field.statement', control: row => textareaControl(row.statement, value => { row.statement = value; }) },
      ], index => ({ anchor: `${draft.id}#criterion.row-${index + 1}`, kind: 'behavior', statement: '', governed_by: [] }), rerender));
    }

    const metadata = el('div', 'form');
    if (draft.kind === 'philosophy') {
      metadata.append(statementMetadataEditor('items.principles', draft.principles, [
        { labelKey: 'items.field.anchor', control: row => inputControl(row.anchor, value => { row.anchor = value; }) },
        { labelKey: 'items.field.applies_to', control: row => inputControl((row.applies_to || []).join(', '), value => { row.applies_to = value.split(',').map(part => part.trim()).filter(Boolean); }) },
      ]));
    }
    if (draft.kind === 'policy') {
      metadata.append(statementMetadataEditor('items.rules', draft.rules, [
        { labelKey: 'items.field.anchor', control: row => inputControl(row.anchor, value => { row.anchor = value; }) },
        { labelKey: 'items.field.governed_by', control: row => inputControl((row.governed_by || []).join(', '), value => { row.governed_by = value.split(',').map(part => part.trim()).filter(Boolean); }) },
        { labelKey: 'items.field.applies_to_roles', control: row => inputControl((row.applies_to_roles || []).join(', '), value => { row.applies_to_roles = value.split(',').map(part => part.trim()).filter(Boolean); }) },
        { labelKey: 'items.field.enforcement', control: row => inputControl(row.enforcement || '', value => { row.enforcement = value.trim() || null; }) },
      ]));
    }
    if (draft.kind === 'requirement') {
      metadata.append(statementMetadataEditor('items.criteria', draft.criteria, [
        { labelKey: 'items.field.anchor', control: row => inputControl(row.anchor, value => { row.anchor = value; }) },
        { labelKey: 'items.field.kind', control: row => selectControl(['behavior', 'quality', 'security', 'operational', 'documentation', 'compatibility', 'custom'], row.kind, value => { row.kind = value; }) },
        { labelKey: 'items.field.governed_by', control: row => inputControl((row.governed_by || []).join(', '), value => { row.governed_by = value.split(',').map(part => part.trim()).filter(Boolean); }) },
      ]));
    }
    draft.bindings ||= [];
    metadata.append(bindingEditor(draft, rerender));
    if (draft.kind === 'feature') {
      draft.contracts ||= [];
      metadata.append(contractEditor(draft, rerender));
    }
    form.append(advancedDetails(metadata));
    return form;
  }

  function statementMetadataEditor(labelKey, rows, columns) {
    const wrap = el('div', 'card');
    wrap.append(el('h3', '', t(labelKey)));
    rows.forEach(row => {
      const rowWrap = el('div', 'form two-column metadata-row');
      columns.forEach(column => rowWrap.append(field(column.labelKey, column.control(row))));
      wrap.append(rowWrap);
    });
    return wrap;
  }

  function bindingEditor(draft, rerender) {
    const roles = ['implementation', 'verification', 'documentation', 'enforcement', 'contract_source', 'configuration', 'generated', 'migration', 'operation', 'evidence'];
    const card = el('div', 'card');
    card.append(el('h3', '', t('items.bindings')));
    draft.bindings.forEach((binding, index) => {
      binding.targets ||= [];
      const block = el('div', 'form two-column');
      block.append(
        field('items.field.anchor', inputControl(binding.anchor, value => { binding.anchor = value; })),
        field('items.field.role', selectControl(roles, binding.role, value => { binding.role = value; })),
        field('items.field.facet', inputControl(binding.facet || '', value => { binding.facet = value; })),
        field('items.field.responsibility', textareaControl(binding.responsibility || '', value => { binding.responsibility = value; })),
      );
      binding.targets.forEach((target, targetIndex) => {
        const targetBlock = el('div', 'form two-column full');
        const selector = target.selector || { kind: 'symbol', name: '' };
        target.selector = selector;
        targetBlock.append(
          field('items.field.target_ref', inputControl(target.reference || '', value => { target.reference = value; })),
          field('items.field.path', inputControl(target.path || '', value => { target.path = value; })),
          field('items.field.adapter', inputControl(target.adapter || '', value => { target.adapter = value; })),
          field('items.field.selector_kind', selectControl(['symbol', 'operation', 'heading', 'json-pointer', 'marker'], selector.kind || 'symbol', value => { target.selector = { kind: value }; rerender(); })),
        );
        if (selector.kind === 'symbol') targetBlock.append(field('items.field.selector_value', inputControl(selector.name || '', value => { selector.name = value.trim(); })));
        if (selector.kind === 'operation') targetBlock.append(field('items.field.selector_value', inputControl(`${selector.method || ''} ${selector.path || ''}`.trim(), value => { const [method, ...path] = value.split(/\s+/); selector.method = method || ''; selector.path = path.join(' '); })));
        if (!['symbol', 'operation'].includes(selector.kind)) targetBlock.append(field('items.field.selector_value', inputControl(selector.value || '', value => { selector.value = value; })));
        targetBlock.append(actionButton(t('common.delete'), 'common.delete', () => {
          if (!confirm(t('items.delete_confirm'))) return;
          binding.targets.splice(targetIndex, 1);
          rerender();
        }, 'btn ghost compact', 'reset'));
        block.append(targetBlock);
      });
      block.append(
        actionButton(t('items.target_new'), 'a11y.new_item_row', () => {
          binding.targets.push({ reference: `${binding.anchor}/target.target-${binding.targets.length + 1}`, path: '', selector: { kind: 'file' }, adapter: '' });
          rerender();
        }, 'btn compact', 'plan'),
        actionButton(t('common.delete'), 'common.delete', () => {
          if (!confirm(t('items.delete_confirm'))) return;
          draft.bindings.splice(index, 1);
          rerender();
        }, 'btn ghost compact', 'reset'),
      );
      card.append(block);
    });
    card.append(actionButton(t('common.new'), 'a11y.new_item_row', () => {
      draft.bindings.push({ anchor: `${draft.id}#binding.row-${draft.bindings.length + 1}`, role: 'implementation', facet: 'implementation', responsibility: '', targets: [], satisfies: [], verifies: [], documents: [], enforces: [], generated_from: [], evidences: [] });
      rerender();
    }, 'btn compact', 'plan'));
    return card;
  }

  function contractEditor(draft, rerender) {
    const card = el('div', 'card');
    card.append(el('h3', '', t('items.contracts')));
    draft.contracts.forEach((contract, index) => {
      contract.participants ||= [];
      const block = el('div', 'form two-column');
      block.append(
        field('items.field.anchor', inputControl(contract.anchor, value => { contract.anchor = value; })),
        field('items.field.kind', selectControl(['http', 'event', 'function', 'schema', 'cli', 'file', 'custom'], contract.kind, value => { contract.kind = value; })),
        field('items.field.source', inputControl(contract.source || '', value => { contract.source = value; })),
        csvEditor('items.field.guarantees', contract.guarantees, value => { contract.guarantees = value; }),
      );
      contract.participants.forEach((participant, participantIndex) => {
        block.append(
          field('items.field.participant_binding', inputControl(participant.binding || '', value => { participant.binding = value; })),
          field('items.field.participant_role', inputControl(participant.role || '', value => { participant.role = value; })),
          actionButton(t('common.delete'), 'common.delete', () => {
            if (!confirm(t('items.delete_confirm'))) return;
            contract.participants.splice(participantIndex, 1);
            rerender();
          }, 'btn ghost compact', 'reset'),
        );
      });
      block.append(
        actionButton(t('items.participant_new'), 'a11y.new_item_row', () => {
          contract.participants.push({ binding: '', role: '' });
          rerender();
        }, 'btn compact', 'plan'),
        actionButton(t('common.delete'), 'common.delete', () => {
          if (!confirm(t('items.delete_confirm'))) return;
          draft.contracts.splice(index, 1);
          rerender();
        }, 'btn ghost compact', 'reset'),
      );
      card.append(block);
    });
    card.append(actionButton(t('common.new'), 'a11y.new_item_row', () => {
      draft.contracts.push({ anchor: `${draft.id}#contract.row-${draft.contracts.length + 1}`, kind: 'http', source: '', participants: [], guarantees: [] });
      rerender();
    }, 'btn compact', 'plan'));
    return card;
  }

  function firstSpecRoot() {
    const root = projection.config?.workspace?.spec_roots?.[0] || 'spec';
    return String(root).replace(/\/+$/, '');
  }

  function newItemDraft(kind) {
    const idPrefix = { philosophy: 'PHI', policy: 'POL', requirement: 'REQ', feature: 'FEAT' }[kind] || 'REQ';
    const id = `${idPrefix}-NEW-${Date.now().toString().slice(-6)}`;
    const folder = { philosophy: 'philosophies', policy: 'policies', requirement: 'requirements', feature: 'features' }[kind] || 'requirements';
    return {
      id,
      kind,
      path: `${firstSpecRoot()}/${folder}/${id.toLowerCase()}.yaml`,
      source_hash: EMPTY_HASH,
      title: '',
      summary: kind === 'requirement' ? undefined : '',
      description: kind === 'requirement' || kind === 'policy' ? '' : undefined,
      status: kind === 'philosophy' || kind === 'policy' ? null : 'planned',
      priority: kind === 'requirement' ? 'medium' : null,
      principles: kind === 'philosophy' ? [{ anchor: `${id}#principle.intent`, statement: '', applies_to: [] }] : [],
      rules: kind === 'policy' ? [{ anchor: `${id}#rule.default`, level: 'must', statement: '', governed_by: [], applies_to_roles: [], enforcement: null }] : [],
      criteria: kind === 'requirement' ? [{ anchor: `${id}#criterion.acceptance`, kind: 'behavior', statement: '', governed_by: [] }] : [],
      bindings: [],
      contracts: [],
      anchors: [],
    };
  }

  function updatePhaseStatus(node, state) {
    const dot = one('.status-circle', node);
    if (!dot) return;
    dot.className = `status-circle tab-status ${phaseStateClass(state)}`;
    const key = phaseStateA11y(state);
    dot.setAttribute('aria-label', key ? t(key) : state);
  }

  function renderDiagnosticTabs(run) {
    const page = one('[data-page="diagnostics"]');
    all('[data-diagnostic-phase]', page).forEach(tab => {
      const selected = tab.dataset.diagnosticPhase === selectedDiagnosticPhase;
      tab.classList.toggle('active', selected);
      tab.setAttribute('aria-selected', String(selected));
      const phase = run.phases.find(item => item.id === tab.dataset.diagnosticPhase);
      const state = tab.dataset.diagnosticPhase === 'all' ? run.state : (phase?.state || 'not_run');
      updatePhaseStatus(tab, state);
    });
  }

  function diagnosticSummaryTitle(run) {
    if (run.state === 'not_run') return t('diagnostics.not_run.title');
    if (run.state === 'failed') return t('diagnostics.failed');
    if (run.state === 'not_applicable') return t('diagnostics.not_applicable');
    if (run.diagnostics.length === 0) return t('diagnostics.zero.title');
    return `${run.diagnostics.length} ${t('diagnostics.issues_found')}`;
  }

  function diagnosticSummaryDescription(run) {
    if (run.state === 'not_run') return t('diagnostics.not_run.description');
    if (run.state === 'failed' || run.state === 'not_applicable') return run.reason || '';
    if (run.diagnostics.length === 0) return t('diagnostics.zero.description');
    return t('diagnostics.inspect_phases');
  }

  function describePhase(phase, run) {
    if (!phase) return diagnosticSummaryDescription(run);
    const label = t(`diagnostics.phase.${phase.id}`);
    if (phase.state === 'not_applicable') return phase.not_applicable_reason || `${label} ${t('diagnostics.not_applicable').toLowerCase()}`;
    if (phase.state === 'not_run') return `${label} ${t('diagnostics.not_run.description').toLowerCase()}`;
    if (phase.state === 'failed') return run.reason || label;
    if (phase.issue_count === 0) return `${label} ${t('diagnostics.zero.description').toLowerCase()}`;
    return `${phase.issue_count} ${t('diagnostics.issues_found')}`;
  }

  function renderPhaseCard(run, phase) {
    if (!phase) return null;
    return summaryCard('diagnostics.title', (() => {
      const body = document.createElement('div');
      body.append(
        el('h4', '', t(`diagnostics.phase.${phase.id}`)),
        el('p', '', describePhase(phase, run)),
        metaLine([
          chip(phase.state === 'passed' ? t('diagnostics.passed') : titleCase(phase.state.replaceAll('_', ' ')), phase.state === 'passed' ? 'green-chip' : phase.state === 'issues' ? 'orange-chip' : phase.state === 'failed' ? 'red-chip' : 'blue-chip'),
          chip(`${phase.evaluated_rules} ${t('diagnostics.rules_summary').toLowerCase()}`),
          chip(`${phase.issue_count} ${t('diagnostics.issues_found')}`),
        ]),
      );
      return body;
    })());
  }

  function renderDiagnosticSummary(run) {
    const host = one('[data-diagnostic-result]');
    clear(host);
    const chips = [];
    if (run.state === 'passed') chips.push(chip(t('diagnostics.passed'), 'green-chip'));
    if (run.context) chips.push(chip(runContextLabel(run.context)));
    if (run.basis) chips.push(chip(run.basis));
    if (run.completed_at) chips.push(chip(window.SyuPreferences.formatDate(run.completed_at), 'blue-chip'));
    host?.append(
      canvasHead(
        diagnosticSummaryTitle(run),
        diagnosticSummaryDescription(run),
        chips,
        [],
      ),
    );
  }

  function renderDiagnosticIssues(run, phaseId) {
    const page = one('[data-page="diagnostics"]');
    const workspace = one('.workspace', page);
    let rail = one('.diagnostic-rail', workspace);
    const phase = phaseId === 'all' ? null : run.phases.find(item => item.id === phaseId) || null;
    const visible = run.diagnostics.filter(diagnostic => phaseId === 'all' || diagnostic.phase === phaseId);
    renderDiagnosticSummary(run);
    if (!visible.length) {
      rail?.remove();
      workspace?.classList.add('no-rail');
      const host = one('[data-diagnostic-result]');
      const phaseCard = renderPhaseCard(run, phase);
      if (phaseCard) host?.append(phaseCard);
      return;
    }
    workspace?.classList.remove('no-rail');
    if (!rail) {
      rail = el('aside', 'rail diagnostic-rail');
      workspace?.prepend(rail);
    }
    clear(rail);
    rail.append(el('div', 'rail-title', t('diagnostics.title')));
    const selected = visible.find(diagnostic => diagnostic.rule_id === selectedDiagnosticKey) || visible[0];
    selectedDiagnosticKey = selected.rule_id;
    visible.forEach(diagnostic => {
      const button = el('button', `rail-item${diagnostic.rule_id === selectedDiagnosticKey ? ' active' : ''}`);
      const label = document.createElement('span');
      label.append(el('b', '', diagnostic.message), el('p', '', diagnostic.rule_id));
      button.append(el('span', `status-circle ${diagnostic.severity === 'error' ? 'red' : diagnostic.severity === 'warning' ? 'orange' : 'blue'}`), label);
      button.addEventListener('click', () => {
        selectedDiagnosticKey = diagnostic.rule_id;
        renderDiagnosticIssues(run, phaseId);
      });
      rail.append(button);
    });
    renderDiagnosticDetail(run, selected, phase);
  }

  function renderDiagnosticDetail(run, diagnostic, phase) {
    renderDiagnosticSummary(run);
    const host = one('[data-diagnostic-result]');
    const phaseCard = renderPhaseCard(run, phase);
    if (phaseCard) host?.append(phaseCard);
    host?.append(
      (() => {
        const card = el('div', 'card');
        const location = diagnostic.primary ? `${diagnostic.primary.path}:${diagnostic.primary.line ?? '-'}` : diagnostic.rule_id;
        card.append(el('h4', '', diagnostic.message), el('div', 'notice', diagnostic.help || diagnostic.message));
        const technical = advancedDetails(
          metaLine([chip(diagnostic.severity), chip(diagnostic.phase), chip(diagnostic.rule_id)]),
          el('p', 'path', location),
        );
        card.append(technical);
        return card;
      })(),
    );
  }

  function failedValidationRun(context, message) {
    return {
      state: 'failed',
      context,
      basis: null,
      started_at: null,
      completed_at: Date.now(),
      duration_ms: null,
      evaluated_rule_count: 0,
      issue_counts: { error: 0, warning: 0, info: 0 },
      applicable_phase_count: 0,
      skipped_phase_count: 5,
      phases: ['config', 'graph', 'targets', 'scope', 'plan'].map(id => ({
        id,
        state: 'failed',
        issue_count: 0,
        evaluated_rules: 0,
        issue_counts: { error: 0, warning: 0, info: 0 },
        not_applicable_reason: message,
      })),
      diagnostics: [],
      reason: message,
    };
  }

  async function runValidationFromCurrentControl() {
    const page = one('[data-page="diagnostics"]');
    const context = one('[data-diagnostics-context]', page);
    const range = one('[data-validation-range]', page);
    const requested = context?.value || 'workspace';
    try {
      const next = await api('/api/work/validate', {
        method: 'POST',
        body: JSON.stringify(mutationBasis()),
      });
      lastRun = next;
      renderRun(next);
    } catch (error) {
      lastRun = failedValidationRun(requested, error.message);
      renderRun(lastRun);
    }
  }

  function renderRun(run) {
    lastRun = run;
    renderDiagnosticTabs(run);
    renderDiagnosticIssues(run, selectedDiagnosticPhase);
  }

  function bindDiagnostics() {
    const page = one('[data-page="diagnostics"]');
    const context = one('[data-diagnostics-context]', page);
    const rangeWrap = one('[data-diagnostics-range-wrap]', page);
    const validateButton = one('[data-focus-id="validate-all"]', page);
    context?.addEventListener('change', () => { rangeWrap.hidden = context.value !== 'git_range'; });
    validateButton?.addEventListener('click', async () => {
      validateButton.disabled = true;
      validateButton.classList.add('running');
      try {
        await runValidationFromCurrentControl();
      } finally {
        validateButton.disabled = false;
        validateButton.classList.remove('running');
      }
    });
    all('[data-diagnostic-phase]', page).forEach(tab => {
      tab.addEventListener('click', () => {
        selectedDiagnosticPhase = tab.dataset.diagnosticPhase;
        renderDiagnosticTabs(lastRun);
        renderDiagnosticIssues(lastRun, selectedDiagnosticPhase);
      });
    });
    renderRun(lastRun);
  }

  function bindItemsTabs() {
    all('[data-tab-group="items"]').forEach(button => {
      button.addEventListener('click', () => {
        selectedItemKind = button.dataset.tab;
        selectedItemId = null;
        selectedAnchor = null;
        renderItems();
      });
    });
    one('[data-items-search]')?.addEventListener('input', event => {
      itemSearchQuery = event.target.value;
      selectedItemId = null;
      renderItems();
    });
  }

  function bindScopeTabs() {
    all('[data-tab-group="scope"]').forEach(button => {
      button.addEventListener('click', () => {
        selectedScopeGroup = button.dataset.tab;
        renderScope();
      });
    });
    all('[data-scope-mode-button]').forEach(button => button.addEventListener('click', async () => {
      selectedScopeMode = button.dataset.scopeModeButton;
      if (selectedScopeMode === 'branch') {
        try {
          await ensureBranchScope();
        } catch (error) {
          branchScope = { state: 'not_applicable', reason: error.message, changed: [] };
        }
      }
      renderScope();
    }));
  }

  function openWorkPage(tab = 'overview') {
    one('[data-route="work"]')?.click();
    one(`[data-tab-group="work"][data-tab="${tab}"]`)?.click();
  }

  function transitionForStatus(status) {
    return ({
      added: 'add',
      modified: 'modify',
      removed: 'remove',
      deleted: 'remove',
      renamed: 'modify',
      copied: 'modify',
      binary: 'modify',
    })[status] || 'modify';
  }

  function bindActions() {
    one('[data-items-new]')?.addEventListener('click', () => openItemEditor(newItemDraft(selectedItemKind === 'all' ? 'requirement' : selectedItemKind)));
    one('[data-work-new]')?.addEventListener('click', () => {
      draftWorkRequest = defaultWorkRequest();
      renderWorkRequestEditor(one('[data-work-overview]'), true);
    });
    one('[data-work-seed]')?.addEventListener('click', () => {
      draftWorkRequest = defaultWorkRequest();
      openWorkPage('overview');
      const host = one('[data-work-overview]');
      if (host) renderWorkRequestEditor(host, !plan);
    });
    one('[data-work-plan]')?.addEventListener('click', () => renderWorkRequestEditor(one('[data-work-overview]'), !plan));
    one('[data-scope-refresh]')?.addEventListener('click', async () => {
      if (selectedScopeMode !== 'branch') return renderScope();
      try {
        await ensureBranchScope(true);
        renderScope();
      } catch (error) {
        toast(error.message);
      }
    });
    one('[data-scope-create-work]')?.addEventListener('click', async () => {
      if (selectedScopeMode === 'branch') {
        const reference = selectedBranchAnchor;
        if (!reference) return toast(t('scope.branch.no_anchor'));
        const request = defaultWorkRequest();
        request.summary = t('scope.branch.work_summary').replace('{path}', selectedBranchEntry.path);
        request.seeds = [];
        request.requested_targets = [{ ref: reference, transition: transitionForStatus(selectedBranchEntry.status) }];
        await api('/api/work/request', { method: 'POST', body: JSON.stringify({ basis: mutationBasis(), request }) });
        location.assign('/?page=work&workTab=overview');
        return;
      }
      const entry = workTargets().find(candidate => candidate.target.reference === selectedScopeTarget?.reference);
      if (!entry?.slice?.id) return toast(t('scope.empty.description'));
      selectedSliceId = entry.slice.id;
      renderWork();
      openWorkPage('slices');
    });
    one('[data-scope-range]')?.addEventListener('change', async () => {
      if (selectedScopeMode !== 'branch') return;
      try {
        await ensureBranchScope(true);
        renderScope();
      } catch (error) {
        toast(error.message);
      }
    });
  }

  function ensureSettingsBound() {
    if (settingsBound) return;
    settingsBound = true;
    bindSettings();
  }

  function bindSettings() {
    const page = one('[data-page="settings"]');
    if (!page) return;
    let config = clone(projection.config);
    let configHash = EMPTY_HASH;
    let previewToken = null;
    let customFacetRows = [];
    let ruleOverrideRows = [];
    let enabledAdapters = [];
    const previewOutput = one('[data-settings-preview-output]', page);
    const noticeHost = el('div', 'notice');
    noticeHost.hidden = true;
    one('[data-settings-layer-panel="workspace"] .settings-panel', page)?.prepend(noticeHost);
    const applyButton = one('[data-settings-apply]', page);
    if (applyButton) applyButton.disabled = true;

    const setNotice = (message, kind = '') => {
      noticeHost.hidden = !message;
      noticeHost.className = `notice${kind ? ` ${kind}` : ''}`;
      noticeHost.textContent = message || '';
    };

    const readToggle = node => !node.classList.contains('off');
    const setToggle = (node, value) => node.classList.toggle('off', !value);
    const splitCsv = value => value.split(',').map(part => part.trim()).filter(Boolean);
    const uniq = values => [...new Set(values.filter(Boolean))];
    const knownAdapters = ['rust', 'typescript', 'markdown', 'openapi', 'yaml', 'json'];

    const controls = {
      specRoots: one('[data-config-spec-roots]', page),
      excludes: one('[data-config-excludes]', page),
      activeProfiles: one('[data-config-active-profiles]', page),
      customFacets: one('[data-config-custom-facets]', page),
      preset: one('[data-config-preset]', page),
      baselineStrategy: one('[data-config-baseline-strategy]', page),
      baselineRef: one('[data-config-baseline-ref]', page),
      baselineRefWrap: one('[data-config-baseline-ref-wrap]', page),
      ruleOverrides: one('[data-config-rule-overrides]', page),
      denyWarnings: one('[data-config-deny-warnings]', page),
      requireOwned: one('[data-config-require-owned]', page),
      editableFiles: one('[data-config-editable-files]', page),
      editableSymbols: one('[data-config-editable-symbols]', page),
      verificationTargets: one('[data-config-verification-targets]', page),
      readonlyTargets: one('[data-config-readonly-targets]', page),
      totalBytes: one('[data-config-total-bytes]', page),
      includePrinciples: one('[data-config-include-principles]', page),
      includeRules: one('[data-config-include-rules]', page),
      adapters: one('[data-config-adapters]', page),
    };

    const namedField = (label, control) => {
      const wrap = el('div', 'field');
      wrap.append(el('label', '', label), control);
      return wrap;
    };

    function syncBaselineControl() {
      const needsRef = ['merge-base', 'revision'].includes(controls.baselineStrategy.value);
      controls.baselineRefWrap.hidden = !needsRef;
      controls.baselineRef.required = needsRef;
    }

    function readNonNegativeInteger(control, labelKey) {
      const value = Number(control.value);
      if (!Number.isInteger(value) || value < 0) {
        throw new Error(t('settings.number_invalid').replace('{field}', t(labelKey)));
      }
      return value;
    }

    function renderCustomFacetRows() {
      clear(controls.customFacets);
      if (!customFacetRows.length) {
        controls.customFacets.append(el('div', 'settings-builder-empty', t('settings.builder.no_custom_facets')));
      }
      customFacetRows.forEach((row, index) => {
        const wrap = el('div', 'settings-row');
        wrap.append(
          namedField(t('settings.builder.profile'), inputControl(row.profile, value => { row.profile = value; })),
          namedField(t('settings.builder.facet'), inputControl(row.facet, value => { row.facet = value; })),
          namedField(t('settings.builder.include_paths'), inputControl(row.include, value => { row.include = value; })),
          (() => {
            const actions = el('div', 'settings-row-actions');
            actions.append(actionButton(t('common.reset'), 'common.reset', () => {
              customFacetRows.splice(index, 1);
              renderCustomFacetRows();
            }, 'btn ghost compact', 'reset'));
            return actions;
          })(),
        );
        controls.customFacets.append(wrap);
      });
      const actions = el('div', 'settings-builder-actions');
      actions.append(actionButton(t('common.new'), 'a11y.new_item_row', () => {
        customFacetRows.push({ profile: '', facet: '', include: '' });
        renderCustomFacetRows();
      }, 'btn compact', 'plan'));
      controls.customFacets.append(actions);
    }

    function renderRuleOverrideRows() {
      clear(controls.ruleOverrides);
      if (!ruleOverrideRows.length) {
        controls.ruleOverrides.append(el('div', 'settings-builder-empty', t('settings.builder.no_rule_overrides')));
      }
      ruleOverrideRows.forEach((row, index) => {
        const wrap = el('div', 'settings-row');
        wrap.append(
          namedField(t('settings.builder.rule_id'), inputControl(row.rule, value => { row.rule = value; })),
          namedField(t('settings.builder.severity'), selectControl(['error', 'warning', 'info', 'off'], row.level, value => { row.level = value; }, titleCase)),
          (() => {
            const actions = el('div', 'settings-row-actions');
            actions.append(actionButton(t('common.reset'), 'common.reset', () => {
              ruleOverrideRows.splice(index, 1);
              renderRuleOverrideRows();
            }, 'btn ghost compact', 'reset'));
            return actions;
          })(),
        );
        controls.ruleOverrides.append(wrap);
      });
      const actions = el('div', 'settings-builder-actions');
      actions.append(actionButton(t('common.new'), 'a11y.new_item_row', () => {
        ruleOverrideRows.push({ rule: '', level: 'warning' });
        renderRuleOverrideRows();
      }, 'btn compact', 'plan'));
      controls.ruleOverrides.append(actions);
    }

    function renderAdapterChips() {
      clear(controls.adapters);
      uniq([...knownAdapters, ...enabledAdapters]).forEach(adapter => {
        const button = el('button', `chip chip-button${enabledAdapters.includes(adapter) ? ' active' : ''}`, adapter);
        button.type = 'button';
        button.addEventListener('click', () => {
          enabledAdapters = enabledAdapters.includes(adapter)
            ? enabledAdapters.filter(value => value !== adapter)
            : [...enabledAdapters, adapter];
          renderAdapterChips();
        });
        controls.adapters.append(button);
      });
    }

    function populate(source) {
      config = source.config;
      configHash = source.hash;
      text(one('[data-settings-hash]', page), `${t('settings.source_hash')} ${configHash.slice(0, 16)}…`);
      controls.specRoots.value = config.workspace.spec_roots.join(', ');
      controls.excludes.value = config.workspace.excludes.join(', ');
      controls.activeProfiles.value = config.inventory.active_profile || '';
      customFacetRows = [];
      renderCustomFacetRows();
      controls.preset.value = config.validation.preset;
      const baseline = config.validation.changed.baseline;
      controls.baselineStrategy.value = baseline?.strategy || 'none';
      controls.baselineRef.value = baseline?.against || baseline?.revision || '';
      syncBaselineControl();
      ruleOverrideRows = [];
      renderRuleOverrideRows();
      setToggle(controls.denyWarnings, false);
      setToggle(controls.requireOwned, config.validation.changed.require_owned_changes);
      controls.editableFiles.value = config.work.slicing.max_editable_files;
      controls.editableSymbols.value = config.work.slicing.max_editable_symbols;
      controls.verificationTargets.value = config.work.slicing.max_verification_targets;
      controls.readonlyTargets.value = config.work.slicing.max_readonly_targets;
      controls.totalBytes.value = config.work.slicing.max_total_bytes;
      setToggle(controls.includePrinciples, false);
      setToggle(controls.includeRules, false);
      enabledAdapters = Object.keys(config.inventory.profiles?.find(profile => profile.id === config.inventory.active_profile)?.providers || {});
      renderAdapterChips();
    }

    function collect() {
      config.workspace.spec_roots = splitCsv(controls.specRoots.value);
      config.workspace.excludes = splitCsv(controls.excludes.value);
      config.inventory.active_profile = controls.activeProfiles.value.trim();
      config.validation.preset = controls.preset.value;
      const strategy = controls.baselineStrategy.value;
      const reference = controls.baselineRef.value.trim();
      if (strategy === 'merge-base') {
        if (!reference) throw new Error(t('settings.baseline_ref_required'));
        config.validation.changed.baseline = { strategy, against: reference };
      } else if (strategy === 'revision') {
        if (!reference) throw new Error(t('settings.baseline_ref_required'));
        config.validation.changed.baseline = { strategy, revision: reference };
      } else if (strategy === 'parent') {
        config.validation.changed.baseline = { strategy: 'parent' };
      } else {
        config.validation.changed.baseline = null;
      }
      config.validation.changed.require_owned_changes = readToggle(controls.requireOwned);
      config.work.slicing.max_editable_files = readNonNegativeInteger(controls.editableFiles, 'settings.editable_files');
      config.work.slicing.max_editable_symbols = readNonNegativeInteger(controls.editableSymbols, 'settings.editable_symbols');
      config.work.slicing.max_verification_targets = readNonNegativeInteger(controls.verificationTargets, 'settings.verification_targets');
      config.work.slicing.max_readonly_targets = readNonNegativeInteger(controls.readonlyTargets, 'settings.readonly_targets');
      config.work.slicing.max_total_bytes = readNonNegativeInteger(controls.totalBytes, 'settings.total_bytes');
      const active = config.inventory.profiles.find(profile => profile.id === config.inventory.active_profile);
      if (active) active.providers = Object.fromEntries(enabledAdapters.map(adapter => [adapter, {}]));
      return config;
    }

    all('[data-settings-layer-panel="workspace"] .toggle', page).forEach(toggle => {
      toggle.setAttribute('role', 'switch');
      toggle.tabIndex = 0;
      toggle.addEventListener('click', () => toggle.classList.toggle('off'));
      toggle.addEventListener('keydown', event => {
        if (event.key === 'Enter' || event.key === ' ') {
          event.preventDefault();
          toggle.classList.toggle('off');
        }
      });
    });
    controls.baselineStrategy?.addEventListener('change', syncBaselineControl);

    one('[data-settings-preview]', page)?.addEventListener('click', async () => {
      try {
        const result = await api('/api/config/preview', { method: 'POST', body: JSON.stringify({ config: collect(), expected_hash: configHash }) });
        previewToken = result.preview_token;
        if (applyButton) applyButton.disabled = !previewToken;
        previewOutput.textContent = `${t('settings.changed_lines')}: ${result.changed_lines}\n${result.new_hash}`;
        setNotice(`${result.changed_lines} ${t('settings.changed_lines')}`, '');
      } catch (error) {
        setNotice(error.message, 'error');
      }
    });

    one('[data-settings-apply]', page)?.addEventListener('click', async () => {
      if (!previewToken) return;
      try {
        const result = await api('/api/config/apply', { method: 'PUT', body: JSON.stringify({ config: collect(), preview_token: previewToken }) });
        configHash = result.new_hash;
        previewToken = null;
        if (applyButton) applyButton.disabled = true;
        previewOutput.textContent = `${t('common.apply')} · ${result.new_hash}`;
        setNotice(t('common.apply'), 'success');
      } catch (error) {
        setNotice(error.message, 'error');
      }
    });

    one('[data-reset-preferences]', page)?.addEventListener('click', () => location.assign('/?page=settings'));

    (async () => {
      try {
        populate(await api('/api/config'));
      } catch (error) {
        populate({ config, hash: EMPTY_HASH });
        setNotice(error.message, 'warn');
      }
    })();
  }

  function bindPalette() {
    const dialog = one('.palette-dialog');
    if (!dialog) return;
    all('[data-dynamic-palette]', dialog).forEach(node => node.remove());
    projection.items.slice(0, 8).forEach(item => {
      const button = el('button', 'palette-result');
      button.dataset.dynamicPalette = 'true';
      button.append(el('span', 'r-ico', '→'), el('span', '', item.id), el('span', 'route', `${t('nav.items')} › ${t(`items.${item.kind}`)}`));
      button.addEventListener('click', () => {
        one('[data-route="specifications"]')?.click();
        one(`[data-tab-group="items"][data-tab="${item.kind}"]`)?.click();
        selectedItemId = item.id;
        selectedAnchor = item.anchors[0] || null;
        renderItems();
        one('.palette-overlay')?.classList.remove('open');
      });
      dialog.append(button);
    });
  }

  function renderAll() {
    renderWork();
    renderScope();
    renderItems();
    renderReadiness();
    renderRun(lastRun);
    bindPalette();
  }

  window.SyuWorkbench = {
    async onRoute(page) {
      if (page === 'settings') ensureSettingsBound();
      if (page === 'scope' && selectedScopeMode === 'branch' && !branchScopeLoaded) {
        try {
          await ensureBranchScope();
          renderScope();
        } catch (error) {
          branchScope = { state: 'not_applicable', reason: error.message, changed: [] };
          renderScope();
        }
      }
    },
    renderAll,
  };

  bindItemsTabs();
  bindScopeTabs();
  renderAll();
  bindDiagnostics();
  bindActions();
  one('[data-readiness-run]')?.addEventListener('click', async () => {
    const button = one('[data-readiness-run]');
    button.disabled = true;
    try {
      projection.readiness = await api('/api/readiness/run', { method: 'POST', body: JSON.stringify({}) });
      renderReadiness();
    } catch (error) {
      toast(error.message);
    } finally {
      button.disabled = false;
    }
  });
  bindPalette();
  if (document.querySelector('[data-page="settings"]')?.hidden === false) ensureSettingsBound();
  document.addEventListener('syu:locale', () => {
    settingsBound = false;
    renderAll();
    if (document.querySelector('[data-page="settings"]')?.hidden === false) ensureSettingsBound();
  });
})();
