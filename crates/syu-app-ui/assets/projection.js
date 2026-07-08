(() => {
  'use strict';

  const stateNode = document.querySelector('#syu-projection');
  if (!stateNode) return;

  const projection = JSON.parse(stateNode.textContent);
  const one = (selector, root = document) => root.querySelector(selector);
  const all = (selector, root = document) => [...root.querySelectorAll(selector)];
  const t = key => window.SyuPreferences.t(key);
  const text = (node, value) => { if (node) node.textContent = value ?? ''; };
  const clear = node => { if (node) node.replaceChildren(); };
  const clone = value => JSON.parse(JSON.stringify(value));
  const buttonByKey = key => one(`[data-i18n-aria="${key}"]`);
  const EMPTY_HASH = 'sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855';

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
  let draftWorkRequest = requestedWork ? clone(requestedWork) : defaultWorkRequest();
  let lastRun = projection.validation;
  let selectedSliceId = plan?.slices[0]?.id || null;
  let selectedScopeGroup = 'change';
  let selectedScopeTarget = null;
  let selectedScopeMode = 'plan';
  let selectedBranchEntry = null;
  let branchScope = null;
  let branchScopeLoaded = false;
  let selectedItemKind = 'all';
  let selectedItemId = null;
  let selectedAnchor = null;
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
    const response = await fetch(url, {
      headers: { 'content-type': 'application/json' },
      ...options,
    });
    const body = await response.text();
    if (!response.ok) {
      try {
        throw new Error(JSON.parse(body).error || body);
      } catch {
        throw new Error(body);
      }
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
      constraints: { include_facets: [], exclude_paths: [], max_slices: null },
      requested_targets: [],
    };
  }

  function renderWorkOverview() {
    const host = one('[data-work-overview]');
    clear(host);
    if (!host) return;
    if (!plan || host.dataset.mode === 'editor') {
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
      summaryCard('work.card.intent', el('p', '', plan.request.summary)),
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

  function renderWorkRequestEditor(host, isEmpty) {
    host.dataset.mode = 'editor';
    clear(host);
    const request = draftWorkRequest || defaultWorkRequest();
    const form = el('form', 'form');
    form.append(canvasHead(
      isEmpty ? t('work.intake.title') : t('work.request.editor_title'),
      isEmpty ? t('work.intake.description') : t('work.request.editor_description'),
      [chip(t('work.request.steps'), 'blue-chip')],
      plan ? [actionButton(t('common.reset'), 'common.reset', () => { host.dataset.mode = ''; renderWorkOverview(); }, 'btn ghost compact', 'reset')] : [],
    ));
    const steps = el('div', 'meta-line');
    steps.append(chip(t('work.request.step.seed')), chip(t('work.request.step.plan')), chip(t('work.request.step.validate')));
    form.append(steps);
    form.append(field('work.request.summary', textareaControl(request.summary || '', value => { request.summary = value; }, 'work-request-summary')));
    form.append(field('work.request.operation', selectControl(['add', 'modify', 'remove', 'refactor', 'document', 'investigate'], request.operation || 'modify', value => { request.operation = value; }, value => t(`operation.${value}`), 'work-request-operation')));
    form.append(field('work.request.seed', inputControl((request.seeds || []).map(String).join(', '), value => {
      request.seeds = value.trim() ? value.split(',').map(entry => entry.trim()).filter(Boolean) : [];
    })));
    const actions = el('div', 'actions');
    const planButton = actionButton(t('common.plan'), 'a11y.work_plan', null, 'btn primary compact', 'plan');
    planButton.type = 'submit';
    actions.append(planButton);
    form.append(actions);
    form.addEventListener('submit', async event => {
      event.preventDefault();
      if (!request.summary?.trim()) return toast(t('work.request.summary_required'));
      draftWorkRequest = request;
      await api('/api/work/request', { method: 'PUT', body: JSON.stringify(request) });
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
          actionButton(t('common.export'), 'a11y.export_slice', async () => {
            const yaml = await api(`/api/context/${encodeURIComponent(slice.id)}`, { method: 'POST' });
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
          const yaml = await api(`/api/context/${encodeURIComponent(slice.id)}`, { method: 'POST' });
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
      detail?.append(emptyState('diagnostics.not_run.title', 'diagnostics.not_run.description'));
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
      detail?.append(emptyState('scope.empty.title', 'scope.empty.description'));
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
    renderScopeDetail(visible.find(entry => entry.target.reference === selectedScopeTarget?.reference) || visible[0] || null);
  }

  function renderScopeDetail(entry) {
    const detail = one('[data-scope-detail]');
    clear(detail);
    if (!entry) {
      detail?.append(emptyState('scope.empty.title', 'scope.empty.description'));
      return;
    }
    const target = entry.target;
    detail?.append(
      canvasHead(
        target.resolved_selector.description,
        target.reason,
        [chip(entry.group), chip(entry.slice.id)],
        [
          actionButton(t('common.copy'), 'a11y.copy_locator', async () => {
            await navigator.clipboard.writeText(target.reference);
            toast(t('toast.locator_copied'));
          }, 'btn compact', 'copy'),
          actionButton(t('common.open'), 'a11y.open_source', () => toast(target.resolved_path), 'btn compact', 'open'),
        ],
      ),
      (() => {
        const locator = el('div', 'target-locator');
        locator.append(el('div', 'path', target.resolved_path), el('div', 'selector', target.reference));
        return locator;
      })(),
      (() => {
        const grid = el('div', 'grid2');
        grid.append(
          summaryCard('scope.why', el('p', '', target.reason)),
          summaryCard('scope.lifecycle', linesList([
            `${t('scope.transition')}: ${target.transition || entry.group}`,
            `${t('scope.access')}: ${target.access || entry.group}`,
            `${t('scope.adapter')}: ${target.adapter || 'anchor'}`,
          ])),
        );
        return grid;
      })(),
    );
  }

  function renderBranchScope() {
    const rail = one('[data-scope-rail]');
    const detail = one('[data-scope-detail]');
    clear(rail);
    clear(detail);
    ['change', 'verify', 'reference', 'intent'].forEach(group => {
      text(one(`[data-tab="${group}"] .mini-count`, one('[data-page="scope"]')), group === 'change' ? (branchScope?.changed?.length || '') : '');
    });
    if (!branchScope) {
      detail?.append(emptyState('scope.branch.loading.title', 'scope.branch.loading.description'));
      return;
    }
    if (branchScope.state !== 'ready') {
      detail?.append(
        canvasHead(t('scope.mode.branch'), branchScope.reason || t('scope.branch.not_applicable.description'), [chip(t('diagnostics.not_applicable'))], []),
        el('div', 'notice warn', branchScope.reason || t('scope.branch.not_applicable.description')),
      );
      return;
    }
    rail?.append(el('div', 'rail-title', `${t('scope.mode.branch')} · ${branchScope.range}`));
    if (!selectedBranchEntry || !branchScope.changed.some(entry => entry.path === selectedBranchEntry.path)) {
      selectedBranchEntry = branchScope.changed[0] || null;
    }
    branchScope.changed.forEach(entry => {
      const button = el('button', `rail-item${selectedBranchEntry?.path === entry.path ? ' active' : ''}`);
      const label = document.createElement('span');
      label.append(el('b', '', entry.path), el('p', '', entry.owners.length ? entry.owners.join(', ') : t('scope.branch.unowned')));
      button.append(el('span', `status-circle ${entry.owners.length ? 'orange' : 'gray'}`), label);
      button.addEventListener('click', () => {
        selectedBranchEntry = entry;
        renderBranchScope();
      });
      rail?.append(button);
    });
    if (!selectedBranchEntry) {
      detail?.append(emptyState('scope.branch.empty.title', 'scope.branch.empty.description'));
      return;
    }
    detail?.append(
      canvasHead(
        selectedBranchEntry.path,
        branchScope.range,
        [chip(selectedBranchEntry.status), chip(selectedBranchEntry.owners.length ? t('scope.branch.owned') : t('scope.branch.unowned'))],
        [],
      ),
      summaryCard('scope.why', el('p', '', selectedBranchEntry.owners.length ? selectedBranchEntry.owners.join(', ') : t('scope.branch.unowned_description'))),
      summaryCard('items.bindings', linesList(selectedBranchEntry.anchors.length ? selectedBranchEntry.anchors : [t('scope.branch.no_anchor')]))
    );
  }

  function renderItemRailRow(rail, item) {
    const button = el('button', `rail-item${item.id === selectedItemId ? ' active' : ''}`);
    const label = document.createElement('span');
    label.append(el('b', '', item.id), el('p', '', item.title));
    button.append(label, el('span', 'n', String(item.anchors.length)));
    button.addEventListener('click', () => {
      selectedItemId = item.id;
      selectedAnchor = item.anchors[0] || null;
      renderItems();
    });
    rail?.append(button);
  }

  function appendItemGroup(rail, kind, items) {
    if (selectedItemKind === 'all') rail?.append(el('div', 'rail-title rail-section', t(`items.${kind}`)));
    items.forEach(item => renderItemRailRow(rail, item));
  }

  function renderItems() {
    const page = one('[data-page="items"]');
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
    if (!selectedAnchor || !item.anchors.includes(selectedAnchor)) selectedAnchor = item.anchors[0] || null;
    renderItemDetail(detail, item);
  }

  function renderItemDetail(detail, item) {
    detail?.append(
      canvasHead(
        item.title,
        item.summary || item.path,
        [chip(item.id), chip(t(`items.${item.kind}`)), item.status ? chip(item.status, 'green-chip') : null, item.priority ? chip(item.priority) : null],
        [
          actionButton(t('common.edit'), 'a11y.edit_item', () => openItemEditor(item), 'btn compact', 'edit'),
          actionButton(t('common.plan'), 'a11y.create_work', async () => {
            if (!selectedAnchor) return toast(t('toast.select_anchor'));
            const request = defaultWorkRequest();
            request.id = `WORK-${Date.now()}`;
            request.summary = t('work.request.summary_from_anchor').replace('{anchor}', selectedAnchor);
            request.seeds = [selectedAnchor];
            draftWorkRequest = request;
            await api('/api/work/request', { method: 'PUT', body: JSON.stringify(request) });
            location.assign('/?page=work&workTab=overview');
          }, 'btn primary compact', 'plan'),
        ],
      ),
      (() => {
        const grid = el('div', 'grid2');
        grid.append(
          summaryCard('items.summary', el('p', '', item.description || item.summary || item.path)),
          summaryCard('items.planner', el('p', '', selectedAnchor ? t('items.exact_seed_available').replace('{anchor}', selectedAnchor) : t('items.no_exact_seed'))),
        );
        return grid;
      })(),
      ...renderItemSections(item),
    );
  }

  function renderItemSections(item) {
    const sections = [];
    if (item.principles.length) sections.push(renderStatementSection(t('items.principles'), item.principles.map(row => ({ title: row.anchor, body: row.statement, meta: row.applies_to.join(', ') }))));
    if (item.rules.length) sections.push(renderStatementSection(t('items.rules'), item.rules.map(row => ({ title: row.anchor, body: row.statement, meta: [row.level, ...row.governed_by].join(' · ') }))));
    if (item.criteria.length) sections.push(renderStatementSection(t('items.criteria'), item.criteria.map(row => ({ title: row.anchor, body: row.statement, meta: [row.kind, ...row.governed_by].join(' · ') }))));
    if (item.bindings.length) sections.push(renderBindingSection(item.bindings));
    if (item.contracts.length) sections.push(renderContractSection(item.contracts));
    return sections;
  }

  function renderStatementSection(label, values) {
    const title = el('div', 'section-label', label);
    const card = el('div', 'card');
    values.forEach(value => {
      const block = el('div', 'target-locator');
      block.append(el('div', 'path', value.title), el('div', 'selector', value.body));
      if (value.meta) block.append(el('p', '', value.meta));
      card.append(block);
    });
    return fragment(title, card);
  }

  function renderBindingSection(bindings) {
    const title = el('div', 'section-label', t('items.bindings'));
    const card = el('div', 'card');
    bindings.forEach(binding => {
      binding.targets.forEach(target => {
        const row = el('div', 'path-row');
        row.append(el('span', 'path', binding.anchor), chip(binding.role), chip(binding.facet), el('span', '', `${target.path} · ${target.selector}`));
        card.append(row);
      });
    });
    return fragment(title, card);
  }

  function renderContractSection(contracts) {
    const title = el('div', 'section-label', t('items.contracts'));
    const card = el('div', 'card');
    contracts.forEach(contract => {
      const block = el('div', 'target-locator');
      block.append(el('div', 'path', contract.anchor), el('div', 'selector', `${contract.kind} · ${contract.source}`), el('p', '', contract.participants.map(row => `${row.role}: ${row.binding}`).join('\n')));
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

  function statementEditor(labelKey, rows, columns, createRow) {
    const wrap = el('div', 'card');
    wrap.append(el('h3', '', t(labelKey)));
    rows.forEach((row, index) => {
      const rowWrap = el('div', 'form two-column');
      columns.forEach(column => {
        rowWrap.append(field(column.labelKey, column.control(row, index)));
      });
      wrap.append(rowWrap);
    });
    const addButton = actionButton(t('common.new'), 'a11y.new_item_row', () => {
      rows.push(createRow(rows.length));
      renderItems();
    }, 'btn compact', 'plan');
    wrap.append(addButton);
    return wrap;
  }

  function openItemEditor(item) {
    const detail = one('[data-items-detail]');
    clear(detail);
    const draft = clone(item);
    draft.expected_hash = item.source_hash || EMPTY_HASH;
    let previewToken = null;
    const apply = actionButton(t('common.apply'), 'common.apply', async () => {
      try {
        const result = await api(`/api/items/${encodeURIComponent(draft.id)}/apply`, { method: 'PUT', body: JSON.stringify({ ...draft, preview_token: previewToken }) });
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
              const result = await api(`/api/items/${encodeURIComponent(draft.id)}/preview`, { method: 'POST', body: JSON.stringify(draft) });
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
    form.append(field('items.field.title', inputControl(draft.title, value => { draft.title = value; })));
    if (draft.kind === 'requirement') {
      form.append(field('items.field.description', textareaControl(draft.description || '', value => { draft.description = value; })));
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
    if (draft.principles?.length) {
      form.append(statementEditor('items.principles', draft.principles, [
        { labelKey: 'items.field.anchor', control: row => inputControl(row.anchor, value => { row.anchor = value; }) },
        { labelKey: 'items.field.statement', control: row => textareaControl(row.statement, value => { row.statement = value; }) },
      ], index => ({ anchor: `${draft.id}#principle.row-${index + 1}`, statement: '', applies_to: [] })));
    }
    if (draft.rules?.length) {
      form.append(statementEditor('items.rules', draft.rules, [
        { labelKey: 'items.field.anchor', control: row => inputControl(row.anchor, value => { row.anchor = value; }) },
        { labelKey: 'items.field.level', control: row => selectControl(['must', 'should', 'may'], row.level, value => { row.level = value; }) },
        { labelKey: 'items.field.statement', control: row => textareaControl(row.statement, value => { row.statement = value; }) },
      ], index => ({ anchor: `${draft.id}#rule.row-${index + 1}`, level: 'must', statement: '', governed_by: [] })));
    }
    if (draft.criteria?.length) {
      form.append(statementEditor('items.criteria', draft.criteria, [
        { labelKey: 'items.field.anchor', control: row => inputControl(row.anchor, value => { row.anchor = value; }) },
        { labelKey: 'items.field.kind', control: row => selectControl(['behavior', 'quality', 'security', 'operational', 'documentation', 'compatibility', 'custom'], row.kind, value => { row.kind = value; }) },
        { labelKey: 'items.field.statement', control: row => textareaControl(row.statement, value => { row.statement = value; }) },
      ], index => ({ anchor: `${draft.id}#criterion.row-${index + 1}`, kind: 'behavior', statement: '', governed_by: [] })));
    }
    if (draft.bindings?.length) form.append(summaryCard('items.bindings', linesList(draft.bindings.map(binding => `${binding.anchor} · ${binding.role} · ${binding.facet}`))));
    if (draft.contracts?.length) form.append(summaryCard('items.contracts', linesList(draft.contracts.map(contract => `${contract.anchor} · ${contract.kind}`))));
    return form;
  }

  function newItemDraft(kind) {
    const idPrefix = { philosophy: 'PHI', policy: 'POL', requirement: 'REQ', feature: 'FEAT' }[kind] || 'REQ';
    const id = `${idPrefix}-NEW-${Date.now().toString().slice(-6)}`;
    const folder = { philosophy: 'philosophy', policy: 'policies', requirement: 'requirements', feature: 'features' }[kind] || 'requirements';
    return {
      id,
      kind,
      path: `docs/syu/${folder}/${id.toLowerCase()}.yaml`,
      source_hash: EMPTY_HASH,
      title: '',
      summary: kind === 'requirement' ? undefined : '',
      description: kind === 'requirement' || kind === 'policy' ? '' : undefined,
      status: kind === 'philosophy' || kind === 'policy' ? null : 'planned',
      priority: kind === 'requirement' ? 'medium' : null,
      principles: kind === 'philosophy' ? [{ anchor: `${id}#principle.intent`, statement: '', applies_to: [] }] : [],
      rules: kind === 'policy' ? [{ anchor: `${id}#rule.default`, level: 'must', statement: '', governed_by: [] }] : [],
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

  function renderValidationStats(run) {
    const grid = el('div', 'grid3');
    [
      [t('diagnostics.evaluated'), run.evaluated_rule_count, t('diagnostics.rules_summary')],
      [t('diagnostics.applicable'), run.applicable_phase_count, t('diagnostics.applicable_summary')],
      [t('diagnostics.skipped'), run.skipped_phase_count, t('diagnostics.skipped_summary')],
    ].forEach(([label, value, summary]) => {
      const card = el('div', 'card');
      card.append(el('h3', '', label), el('div', 'big-stat', String(value)), el('p', '', summary));
      grid.append(card);
    });
    return grid;
  }

  function renderDiagnosticSummary(run) {
    const host = one('[data-diagnostic-result]');
    clear(host);
    const chips = [];
    if (run.state === 'passed') chips.push(chip(t('diagnostics.passed'), 'green-chip'));
    if (run.context) chips.push(chip(titleCase(run.context.replace('-', '_'))));
    if (run.basis) chips.push(chip(run.basis));
    if (run.completed_at) chips.push(chip(window.SyuPreferences.formatDate(run.completed_at), 'blue-chip'));
    host?.append(
      canvasHead(
        diagnosticSummaryTitle(run),
        diagnosticSummaryDescription(run),
        chips,
        [actionButton(t('filter.validate'), 'a11y.validate_context', runValidationFromCurrentControl, 'btn primary compact', 'validate')],
      ),
      renderValidationStats(run),
    );
  }

  function renderDiagnosticIssues(run, phase) {
    const page = one('[data-page="diagnostics"]');
    const workspace = one('.workspace', page);
    let rail = one('.diagnostic-rail', workspace);
    const visible = run.diagnostics.filter(diagnostic => phase === 'all' || diagnostic.phase === phase);
    renderDiagnosticSummary(run);
    if (!visible.length) {
      rail?.remove();
      workspace?.classList.add('no-rail');
      if (run.state === 'passed' || run.state === 'not_run' || run.state === 'not_applicable' || run.state === 'failed') {
        one('[data-diagnostic-result]')?.append(el('div', 'notice', diagnosticSummaryDescription(run)));
      }
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
      label.append(el('b', '', diagnostic.rule_id), el('p', '', diagnostic.message));
      button.append(el('span', `status-circle ${diagnostic.severity === 'error' ? 'red' : diagnostic.severity === 'warning' ? 'orange' : 'blue'}`), label);
      button.addEventListener('click', () => {
        selectedDiagnosticKey = diagnostic.rule_id;
        renderDiagnosticIssues(run, phase);
      });
      rail.append(button);
    });
    renderDiagnosticDetail(run, selected);
  }

  function renderDiagnosticDetail(run, diagnostic) {
    renderDiagnosticSummary(run);
    const host = one('[data-diagnostic-result]');
    host?.append(
      el('div', 'section-label', diagnostic.rule_id),
      (() => {
        const card = el('div', 'card');
        const location = diagnostic.primary ? `${diagnostic.primary.path}:${diagnostic.primary.line ?? '-'}` : diagnostic.rule_id;
        card.append(el('h4', '', diagnostic.message), el('p', '', location), metaLine([chip(diagnostic.severity), chip(diagnostic.phase), chip(diagnostic.rule_id)]), el('div', 'notice', diagnostic.help || diagnostic.message));
        return card;
      })(),
    );
  }

  async function runValidationFromCurrentControl() {
    const page = one('[data-page="diagnostics"]');
    const context = one('[data-diagnostics-context]', page);
    const range = one('[data-validation-range]', page);
    const requested = context?.value || 'workspace';
    try {
      const next = await api('/api/validate', {
        method: 'POST',
        body: JSON.stringify({
          context: requested,
          range: requested === 'git_range' ? range?.value || null : null,
          slice: requested === 'slice' ? currentSlice()?.id || null : null,
        }),
      });
      lastRun = next;
      renderRun(next);
    } catch (error) {
      lastRun = { ...lastRun, state: 'failed', reason: error.message, diagnostics: [] };
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
    context?.addEventListener('change', () => { rangeWrap.hidden = context.value !== 'git_range'; });
    all('[data-diagnostic-phase]', page).forEach(tab => {
      tab.addEventListener('click', () => {
        selectedDiagnosticPhase = tab.dataset.diagnosticPhase;
        renderDiagnosticTabs(lastRun);
        renderDiagnosticIssues(lastRun, selectedDiagnosticPhase);
      });
    });
    buttonByKey('a11y.validate_context')?.addEventListener('click', async () => {
      buttonByKey('a11y.validate_context').disabled = true;
      try {
        await runValidationFromCurrentControl();
      } finally {
        buttonByKey('a11y.validate_context').disabled = false;
      }
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

  function bindActions() {
    one('[data-items-new]')?.addEventListener('click', () => openItemEditor(newItemDraft(selectedItemKind === 'all' ? 'requirement' : selectedItemKind)));
    one('[data-work-new]')?.addEventListener('click', () => {
      draftWorkRequest = defaultWorkRequest();
      renderWorkRequestEditor(one('[data-work-overview]'), true);
    });
    one('[data-work-seed]')?.addEventListener('click', () => {
      if (!selectedAnchor) return toast(t('toast.select_anchor'));
      draftWorkRequest = defaultWorkRequest();
      draftWorkRequest.summary = t('work.request.summary_from_anchor').replace('{anchor}', selectedAnchor);
      draftWorkRequest.seeds = [selectedAnchor];
      location.assign('/?page=work&workTab=overview');
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
        const ownerId = selectedBranchEntry?.owners?.[0];
        const item = projection.items.find(candidate => candidate.id === ownerId);
        const anchor = item?.anchors?.[0];
        if (!anchor) return toast(t('scope.branch.no_anchor'));
        const request = defaultWorkRequest();
        request.summary = t('scope.branch.work_summary').replace('{path}', selectedBranchEntry.path);
        request.seeds = [anchor];
        await api('/api/work/request', { method: 'PUT', body: JSON.stringify(request) });
        location.assign('/?page=work&workTab=overview');
        return;
      }
      if (!selectedScopeTarget?.reference) return toast(t('scope.empty.description'));
      toast(selectedScopeTarget.reference);
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
    const previewOutput = one('[data-settings-preview-output]', page);
    const noticeHost = el('div', 'notice');
    noticeHost.hidden = true;
    one('[data-settings-layer-panel="workspace"] .settings-panel', page)?.prepend(noticeHost);

    const setNotice = (message, kind = '') => {
      noticeHost.hidden = !message;
      noticeHost.className = `notice${kind ? ` ${kind}` : ''}`;
      noticeHost.textContent = message || '';
    };

    const readToggle = node => !node.classList.contains('off');
    const setToggle = (node, value) => node.classList.toggle('off', !value);
    const splitCsv = value => value.split(',').map(part => part.trim()).filter(Boolean);

    const controls = {
      specRoots: one('[data-config-spec-roots]', page),
      artifactRoots: one('[data-config-artifact-roots]', page),
      excludes: one('[data-config-excludes]', page),
      activeProfiles: one('[data-config-active-profiles]', page),
      customFacets: one('[data-config-custom-facets]', page),
      preset: one('[data-config-preset]', page),
      baseline: one('[data-config-baseline]', page),
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

    function populate(source) {
      config = source.config;
      configHash = source.hash;
      text(one('[data-settings-hash]', page), `${t('settings.source_hash')} ${configHash.slice(0, 16)}…`);
      controls.specRoots.value = config.workspace.spec_roots.join(', ');
      controls.artifactRoots.value = config.workspace.artifact_roots.join(', ');
      controls.excludes.value = config.workspace.excludes.join(', ');
      controls.activeProfiles.value = config.profiles.active.join(', ');
      controls.customFacets.value = JSON.stringify(config.profiles.custom, null, 2);
      controls.preset.value = config.validation.preset;
      controls.baseline.value = config.validation.changed.baseline?.against || '';
      controls.ruleOverrides.value = JSON.stringify(config.validation.rules, null, 2);
      setToggle(controls.denyWarnings, config.validation.deny_warnings);
      setToggle(controls.requireOwned, config.validation.changed.require_owned_changes);
      controls.editableFiles.value = config.work.slicing.max_editable_files;
      controls.editableSymbols.value = config.work.slicing.max_editable_symbols;
      controls.verificationTargets.value = config.work.slicing.max_verification_targets;
      controls.readonlyTargets.value = config.work.slicing.max_readonly_targets;
      controls.totalBytes.value = config.work.slicing.max_total_bytes;
      setToggle(controls.includePrinciples, config.work.context.include_parent_principles);
      setToggle(controls.includeRules, config.work.context.include_parent_rules);
      controls.adapters.value = config.adapters.enabled.join(', ');
    }

    function collect() {
      config.workspace.spec_roots = splitCsv(controls.specRoots.value);
      config.workspace.artifact_roots = splitCsv(controls.artifactRoots.value);
      config.workspace.excludes = splitCsv(controls.excludes.value);
      config.profiles.active = splitCsv(controls.activeProfiles.value);
      config.profiles.custom = JSON.parse(controls.customFacets.value || '{}');
      config.validation.preset = controls.preset.value;
      config.validation.changed.baseline = controls.baseline.value.trim() ? { strategy: 'merge-base', against: controls.baseline.value.trim() } : null;
      config.validation.rules = JSON.parse(controls.ruleOverrides.value || '{}');
      config.validation.deny_warnings = readToggle(controls.denyWarnings);
      config.validation.changed.require_owned_changes = readToggle(controls.requireOwned);
      config.work.slicing.max_editable_files = Number(controls.editableFiles.value);
      config.work.slicing.max_editable_symbols = Number(controls.editableSymbols.value);
      config.work.slicing.max_verification_targets = Number(controls.verificationTargets.value);
      config.work.slicing.max_readonly_targets = Number(controls.readonlyTargets.value);
      config.work.slicing.max_total_bytes = Number(controls.totalBytes.value);
      config.work.context.include_parent_principles = readToggle(controls.includePrinciples);
      config.work.context.include_parent_rules = readToggle(controls.includeRules);
      config.adapters.enabled = splitCsv(controls.adapters.value);
      return config;
    }

    all('.toggle', page).forEach(toggle => {
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

    one('[data-settings-preview]', page)?.addEventListener('click', async () => {
      try {
        const result = await api('/api/config/preview', { method: 'POST', body: JSON.stringify({ config: collect(), expected_hash: configHash }) });
        previewToken = result.preview_token;
        one('[data-settings-apply]', page).disabled = !previewToken;
        previewOutput.textContent = `${t('settings.changed_lines')}: ${result.changed_lines}\n${result.new_hash}`;
        setNotice(`${result.changed_lines} ${t('settings.changed_lines')}`, '');
      } catch (error) {
        setNotice(error.message, 'error');
      }
    });

    one('[data-settings-apply]', page)?.addEventListener('click', async () => {
      if (!previewToken) return;
      try {
        const result = await api('/api/config/apply', { method: 'PUT', body: JSON.stringify({ config: collect(), expected_hash: configHash, preview_token: previewToken }) });
        configHash = result.new_hash;
        previewToken = null;
        one('[data-settings-apply]', page).disabled = true;
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
    projection.items.slice(0, 8).forEach(item => {
      const button = el('button', 'palette-result');
      button.append(el('span', 'r-ico', '→'), el('span', '', item.id), el('span', 'route', `items › ${item.kind}`));
      button.addEventListener('click', () => {
        one('[data-route="items"]')?.click();
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
    renderRun(lastRun);
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
  bindPalette();
  if (document.querySelector('[data-page="settings"]')?.hidden === false) ensureSettingsBound();
  document.addEventListener('syu:locale', () => renderAll());
})();
