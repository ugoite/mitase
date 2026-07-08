(() => {
  'use strict';
  const stateNode = document.querySelector('#syu-projection');
  if (!stateNode) return;

  const projection = JSON.parse(stateNode.textContent);
  const one = (selector, root = document) => root.querySelector(selector);
  const all = (selector, root = document) => [...root.querySelectorAll(selector)];
  const text = (node, value) => { if (node) node.textContent = value ?? ''; };
  const clear = node => { if (node) node.replaceChildren(); };
  const buttonByKey = key => one(`button[data-i18n-aria="${key}"]`);
  const t = key => window.SyuPreferences.t(key);
  const api = async (url, options = {}) => {
    const response = await fetch(url, { headers: { 'content-type': 'application/json' }, ...options });
    const body = await response.text();
    if (!response.ok) throw new Error(JSON.parse(body).error || body);
    try { return JSON.parse(body); } catch { return body; }
  };
  const toast = message => {
    const host = one('.toast');
    text(host, message);
    host?.classList.add('show');
    setTimeout(() => host?.classList.remove('show'), 3000);
  };

  const requestedWork = projection.requested_work || null;
  const plan = projection.plan || null;
  let lastRun = projection.validation;
  let selectedSliceId = plan?.slices[0]?.id || null;
  let selectedScopeGroup = 'change';
  let selectedScopeTarget = null;
  let selectedItemKind = 'requirement';
  let selectedItemId = null;
  let selectedAnchor = null;

  const statusLabel = status => ({
    ready: t('work.status.ready'),
    needs_review: t('work.status.needs_review'),
    blocked: t('work.status.blocked'),
  })[status] || status;

  const titleCase = value => value ? value.replaceAll('_', ' ').replace(/\b\w/g, m => m.toUpperCase()) : '';
  const phaseStateClass = state => state === 'passed' ? 'green' : state === 'issues' ? 'red' : state === 'running' ? 'blue running' : 'gray';
  const phaseStateA11y = state => ({
    passed: 'a11y.passed',
    issues: 'a11y.issues',
    running: 'a11y.running',
    not_run: 'a11y.not_run',
    not_applicable: 'a11y.not_applicable',
    failed: 'diagnostics.failed',
  })[state];
  const workTargets = () => plan ? plan.slices.flatMap(slice => [
    ...slice.editable_targets.map(target => ({ slice, target, group: 'change' })),
    ...slice.verification_targets.map(target => ({ slice, target, group: 'verify' })),
    ...slice.readonly_context.map(target => ({ slice, target, group: 'reference' })),
    ...slice.anchors.map(anchor => ({ slice, target: { reference: anchor.toString?.() || String(anchor), resolved_path: slice.id, resolved_selector: { description: anchor.toString?.() || String(anchor) }, reason: slice.goal }, group: 'intent' })),
  ]) : [];

  function currentSlice() {
    return plan?.slices.find(slice => slice.id === selectedSliceId) || plan?.slices[0] || null;
  }

  function fragment(...nodes) {
    const out = document.createDocumentFragment();
    nodes.filter(Boolean).forEach(node => out.append(node));
    return out;
  }

  function el(tag, className, textValue) {
    const node = document.createElement(tag);
    if (className) node.className = className;
    if (textValue !== undefined) node.textContent = textValue;
    return node;
  }

  function actionButton(label, ariaKey, onClick, className = 'btn compact') {
    const button = el('button', className, label);
    button.setAttribute('aria-label', t(ariaKey));
    button.dataset.i18nAria = ariaKey;
    button.dataset.i18nTitle = ariaKey;
    button.title = t(ariaKey);
    button.addEventListener('click', onClick);
    return button;
  }

  function chip(textValue, className = '') {
    return el('span', `chip${className ? ` ${className}` : ''}`, textValue);
  }

  function emptyState(titleKey, descriptionKey) {
    const wrap = el('div', 'empty-state');
    wrap.append(el('h2', '', t(titleKey)), el('p', '', t(descriptionKey)));
    return wrap;
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

  function metaLine(values) {
    const line = el('div', 'meta-line');
    values.filter(Boolean).forEach(value => line.append(value));
    return line;
  }

  function canvasHead(title, description, chips = [], actions = []) {
    const head = el('div', 'canvas-head');
    const copy = document.createElement('div');
    copy.append(el('h2', '', title), el('p', '', description));
    if (chips.length) copy.append(metaLine(chips));
    head.append(copy);
    const actionWrap = el('div', 'actions');
    actions.forEach(action => actionWrap.append(action));
    head.append(actionWrap);
    return head;
  }

  function summaryCard(titleKey, content) {
    const card = el('div', 'card');
    card.append(el('h3', '', t(titleKey)), content);
    return card;
  }

  function renderWork() {
    const page = one('[data-page="work"]');
    text(one('[data-work-plan-label]', page), plan?.id || requestedWork?.id || t('common.request'));
    text(one('[data-tab="slices"] .mini-count', page), plan?.slices.length || '');
    text(one('[data-tab="validation"] .mini-count', page), plan?.diagnostics.length || '');
    renderWorkOverview();
    renderWorkSlices();
    renderWorkContext();
    renderWorkValidation();
  }

  function renderWorkOverview() {
    const host = one('[data-work-overview]');
    clear(host);
    if (!plan) {
      host?.append(emptyState('work.empty.title', 'work.empty.description'));
      return;
    }
    const seeds = plan.request.seeds.map(seed => String(seed));
    const constraints = [];
    if (plan.request.constraints.include_facets.length) constraints.push(`Facets: ${plan.request.constraints.include_facets.join(', ')}`);
    if (plan.request.constraints.exclude_paths.length) constraints.push(`Exclude: ${plan.request.constraints.exclude_paths.join(', ')}`);
    if (plan.request.constraints.max_slices) constraints.push(`Maximum ${plan.request.constraints.max_slices} execution slices`);
    if (!constraints.length) constraints.push('No additional planner constraints');

    const body = document.createDocumentFragment();
    body.append(
      canvasHead(
        plan.request.summary,
        `Canonical ${titleCase(String(plan.request.operation))} request for ${projection.workspace.root}`,
        [
          chip(statusLabel(plan.status), plan.status === 'blocked' ? 'red-chip' : plan.status === 'ready' ? 'green-chip' : 'orange-chip'),
          chip(titleCase(String(plan.request.operation))),
          chip(`basis ${plan.basis.revision.slice(0, 9)}`),
        ],
        [
          actionButton(t('common.edit'), 'a11y.edit_request', () => openRequestEditor()),
          actionButton(t('common.replan'), 'a11y.replan_work', async () => {
            await api('/api/work/replan', { method: 'POST' });
            location.reload();
          }, 'btn primary compact'),
        ],
      ),
    );

    const grid = el('div', 'grid2');
    grid.append(
      summaryCard('work.card.changes', el('p', '', `This request expands to ${plan.slices.length} execution slice(s) and ${plan.diagnostics.length} plan diagnostic(s).`)),
      summaryCard('work.card.reason', el('p', '', `Seeds, exact targets, and completion checks come from the canonical workspace graph for revision ${plan.basis.revision.slice(0, 9)}.`)),
    );
    body.append(grid);

    const grid2 = el('div', 'grid2');
    grid2.style.marginTop = '12px';
    grid2.append(
      summaryCard('work.card.seed', linesList([
        ...seeds.map(seed => `Seed: ${seed}`),
        ...constraints,
      ])),
      summaryCard('work.card.limits', linesList([
        `Workspace fingerprint ${projection.workspace.fingerprint.slice(0, 16)}...`,
        `Operation ${String(plan.request.operation)}`,
        `Completion checks ${plan.slices.reduce((sum, slice) => sum + slice.completion.length, 0)}`,
      ])),
    );
    body.append(grid2);

    if (plan.diagnostics.length) {
      body.append(el('div', 'notice warn', t('work.notice.review')));
    }
    host?.append(body);
  }

  function renderWorkSlices() {
    const rail = one('[data-work-slices-rail]');
    const detail = one('[data-work-slice-detail]');
    clear(rail);
    clear(detail);
    if (!plan) {
      detail?.append(emptyState('work.empty.title', 'work.empty.description'));
      return;
    }
    rail?.append(el('div', 'rail-title', t('work.slices.title')));
    plan.slices.forEach((slice, index) => {
      const button = el('button', `rail-item${(selectedSliceId || plan.slices[0].id) === slice.id ? ' active' : ''}`);
      button.dataset.sliceId = slice.id;
      button.append(el('span', 'status-circle green'));
      const label = document.createElement('span');
      label.append(el('b', '', slice.id), el('p', '', slice.goal));
      button.append(label, el('span', 'n', String(slice.editable_targets.length)));
      button.addEventListener('click', () => { selectedSliceId = slice.id; renderWorkSlices(); renderWorkContext(); renderScope(); });
      rail?.append(button);
      if (index === 0 && !selectedSliceId) selectedSliceId = slice.id;
    });

    const slice = currentSlice();
    if (!slice) return;
    const targetsCard = el('div', 'card');
    targetsCard.style.padding = '8px 12px';
    [...slice.editable_targets, ...slice.verification_targets, ...slice.readonly_context].forEach(target => {
      const row = el('div', 'path-row');
      row.append(
        el('span', 'path', target.resolved_path),
        chip(target.transition),
        chip(target.access),
        el('span', 'muted', target.reason),
      );
      targetsCard.append(row);
    });
    detail?.append(
      canvasHead(
        `${slice.id} · ${slice.goal}`,
        slice.acceptance[0]?.statement || slice.goal,
        [
          chip(titleCase(String(slice.confidence || 'exact')), 'green-chip'),
          chip(`${slice.editable_targets.length} editable`),
          chip(`${slice.verification_targets.length} verification`),
          chip(`${slice.readonly_context.length} reference`),
        ],
        [
          actionButton(t('common.copy'), 'a11y.copy_slice', async () => {
            if (!slice) return;
            await navigator.clipboard.writeText([slice.id, slice.goal, ...slice.anchors.map(String)].join('\n'));
            toast(t('toast.locator_copied'));
          }),
          actionButton(t('common.export'), 'a11y.export_slice', async () => {
            if (!slice) return;
            const yaml = await api(`/api/context/${encodeURIComponent(slice.id)}`, { method: 'POST' });
            const link = document.createElement('a');
            link.href = URL.createObjectURL(new Blob([yaml], { type: 'application/yaml' }));
            link.download = `${slice.id}-context.yaml`;
            link.click();
            URL.revokeObjectURL(link.href);
          }, 'btn small primary compact'),
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
      (() => {
        const grid = el('div', 'grid2');
        grid.style.marginTop = '12px';
        grid.append(
          summaryCard('work.in_scope', linesList(slice.editable_targets.map(target => `${target.resolved_path} · ${target.resolved_selector.description}`))),
          summaryCard('work.non_goals', linesList(slice.non_goals.map(item => `${item.code}: ${item.statement}`))),
        );
        return grid;
      })(),
      el('div', 'section-label', t('scope.exact_targets')),
      targetsCard,
      el('div', 'section-label', t('work.completion_checks')),
      metaLine(slice.completion.map(check => chip(check.kind || Object.keys(check)[0] || 'check', 'dark'))),
    );
  }

  function renderWorkContext() {
    const rail = one('[data-work-context-rail]');
    const detail = one('[data-work-context-detail]');
    clear(rail);
    clear(detail);
    if (!plan) {
      detail?.append(emptyState('work.empty.title', 'work.empty.description'));
      return;
    }
    const slice = currentSlice();
    if (!slice) return;
    const groups = [
      { key: 'editable', label: t('work.context.editable'), items: slice.editable_targets, color: 'red' },
      { key: 'verification', label: t('work.context.verification'), items: slice.verification_targets, color: 'blue' },
      { key: 'reference', label: t('work.context.reference'), items: slice.readonly_context, color: 'gray' },
      { key: 'specification', label: t('work.context.specification'), items: slice.anchors.map(anchor => ({ resolved_path: String(anchor), resolved_selector: { description: String(anchor) }, reason: slice.goal })), color: 'purple' },
    ];
    rail?.append(el('div', 'rail-title', t('work.context.groups')));
    groups.forEach((group, index) => {
      const button = el('button', `rail-item${index === 0 ? ' active' : ''}`);
      button.append(el('span', `status-circle ${group.color}`));
      const label = document.createElement('span');
      label.append(el('b', '', group.label), el('p', '', `${group.items.length} item(s)`));
      button.append(label, el('span', 'n', String(group.items.length)));
      rail?.append(button);
    });
    detail?.append(
      canvasHead(
        `Context Pack · ${slice.id}`,
        `Human-readable projection of ${slice.id} context and supporting specification anchors.`,
        [chip(t('work.context.ready'), 'green-chip')],
        [
          actionButton(t('common.raw'), 'a11y.preview_manifest', () => toast(slice.id)),
          actionButton(t('common.download'), 'a11y.download_context', async () => {
            const yaml = await api(`/api/context/${encodeURIComponent(slice.id)}`, { method: 'POST' });
            const link = document.createElement('a');
            link.href = URL.createObjectURL(new Blob([yaml], { type: 'application/yaml' }));
            link.download = `${slice.id}-context.yaml`;
            link.click();
            URL.revokeObjectURL(link.href);
          }, 'btn small primary compact'),
        ],
      ),
      (() => {
        const grid = el('div', 'grid2');
        groups.forEach(group => {
          const card = el('div', 'target-locator');
          card.append(el('div', 'path', group.label), el('div', 'selector', group.items.map(item => item.resolved_path).join('\n') || '-'));
          grid.append(card);
        });
        return grid;
      })(),
      el('div', 'section-label', t('work.context.instruction')),
      (() => {
        const card = el('div', 'card');
        card.append(el('p', '', `This slice carries ${slice.anchors.length} anchor(s), ${slice.editable_targets.length} editable target(s), ${slice.verification_targets.length} verification target(s), and ${slice.readonly_context.length} readonly target(s).`));
        return card;
      })(),
      el('div', 'section-label', t('work.context.source')),
      (() => {
        const code = el('div', 'code');
        code.textContent = slice.editable_targets.map(target => `${target.resolved_path}\n  ${target.reference}`).join('\n\n') || slice.goal;
        return code;
      })(),
    );
  }

  function renderWorkValidation() {
    const rail = one('[data-work-validation-rail]');
    const detail = one('[data-work-validation-detail]');
    clear(rail);
    clear(detail);
    if (!plan) {
      detail?.append(emptyState('work.empty.title', 'work.empty.description'));
      return;
    }
    rail?.append(el('div', 'rail-title', t('work.validation.plan_diagnostics')));
    if (!plan.diagnostics.length) {
      detail?.append(emptyState('diagnostics.not_run.title', 'diagnostics.not_run.description'));
      return;
    }
    plan.diagnostics.forEach((diagnostic, index) => {
      const button = el('button', `rail-item${index === 0 ? ' active' : ''}`);
      button.append(el('span', `status-circle ${diagnostic.severity === 'error' ? 'red' : diagnostic.severity === 'warning' ? 'orange' : 'blue'}`));
      const label = document.createElement('span');
      label.append(el('b', '', diagnostic.rule_id), el('p', '', diagnostic.message));
      button.append(label, el('span', 'n', String(index + 1)));
      button.addEventListener('click', () => renderPlanDiagnostic(diagnostic));
      rail?.append(button);
      if (index === 0) renderPlanDiagnostic(diagnostic);
    });
  }

  function renderPlanDiagnostic(diagnostic) {
    const detail = one('[data-work-validation-detail]');
    clear(detail);
    detail?.append(
      canvasHead(
        diagnostic.rule_id,
        diagnostic.message,
        [chip(diagnostic.severity, diagnostic.severity === 'error' ? 'red-chip' : diagnostic.severity === 'warning' ? 'orange-chip' : 'blue-chip')],
        [actionButton(t('filter.validate'), 'a11y.validate_plan', () => toast(diagnostic.rule_id), 'btn small primary compact')],
      ),
      (() => {
        const card = el('div', 'card');
        card.append(el('h4', '', diagnostic.message), el('p', '', diagnostic.help || diagnostic.message));
        return card;
      })(),
    );
  }

  function renderScope() {
    const page = one('[data-page="scope"]');
    text(one('[data-scope-plan-label]', page), plan?.id || requestedWork?.id || t('common.request'));
    text(one('[data-scope-slice-label]', page), currentSlice()?.id || '');
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
    if (!selectedScopeTarget || !visible.some(entry => entry.target.reference === selectedScopeTarget.reference)) selectedScopeTarget = visible[0]?.target || null;
    visible.forEach(entry => {
      const button = el('button', `rail-item${selectedScopeTarget?.reference === entry.target.reference ? ' active' : ''}`);
      button.append(el('span', `status-circle ${entry.group === 'change' ? 'red' : entry.group === 'verify' ? 'blue' : entry.group === 'reference' ? 'gray' : 'purple'}`));
      const label = document.createElement('span');
      label.append(el('b', '', entry.target.resolved_selector.description), el('p', '', entry.target.resolved_path));
      button.append(label);
      button.addEventListener('click', () => { selectedScopeTarget = entry.target; renderScopeDetail(entry); renderScope(); });
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
            if (selectedScopeTarget?.reference) await navigator.clipboard.writeText(selectedScopeTarget.reference);
            toast(t('toast.locator_copied'));
          }),
          actionButton(t('common.open'), 'a11y.open_source', () => selectedScopeTarget && toast(selectedScopeTarget.resolved_path), 'btn small primary compact'),
        ],
      ),
      (() => {
        const locator = el('div', 'target-locator');
        locator.append(el('div', 'path', target.resolved_path), el('div', 'selector', target.reference));
        return locator;
      })(),
      (() => {
        const grid = el('div', 'grid2');
        grid.style.marginTop = '12px';
        grid.append(
          summaryCard('scope.why', el('p', '', target.reason)),
          summaryCard('scope.lifecycle', linesList([
            `Transition: ${target.transition || entry.group}`,
            `Access: ${target.access || entry.group}`,
            `Adapter: ${target.adapter || 'anchor'}`,
          ])),
        );
        return grid;
      })(),
      el('div', 'section-label', t('scope.linked_intent')),
      (() => {
        const card = el('div', 'card');
        const seed = plan?.request?.seeds?.length ? String(plan.request.seeds[0]) : '';
        card.append(metaLine([chip(entry.slice.id), chip(seed)]), el('p', '', entry.slice.goal));
        return card;
      })(),
      el('div', 'section-label', t('scope.resolved_excerpt')),
      (() => {
        const code = el('div', 'code');
        code.textContent = `${target.resolved_path}\n${target.reference}\n${target.reason}`;
        return code;
      })(),
    );
  }

  function renderItems() {
    const page = one('[data-page="items"]');
    const kinds = ['philosophy', 'policy', 'requirement', 'feature'];
    kinds.forEach(kind => {
      text(one(`[data-tab="${kind}"] .mini-count`, page), projection.items.filter(item => item.kind === kind).length || '');
    });
    const rail = one('[data-items-rail]');
    const detail = one('[data-items-detail]');
    clear(rail);
    clear(detail);
    const visible = projection.items.filter(item => item.kind === selectedItemKind);
    if (!selectedItemId || !visible.some(item => item.id === selectedItemId)) {
      selectedItemId = visible[0]?.id || projection.items[0]?.id || null;
    }
    rail?.append(el('div', 'rail-title', titleCase(selectedItemKind)));
    visible.forEach(item => {
      const button = el('button', `rail-item${item.id === selectedItemId ? ' active' : ''}`);
      button.dataset.kind = item.kind;
      const label = document.createElement('span');
      label.append(el('b', '', item.id), el('p', '', item.title));
      button.append(label, el('span', 'n', String(item.anchors.length)));
      button.addEventListener('click', () => { selectedItemId = item.id; selectedAnchor = item.anchors[0] || null; renderItems(); });
      rail?.append(button);
    });
    const item = projection.items.find(candidate => candidate.id === selectedItemId) || visible[0];
    if (!item) return;
    if (!selectedAnchor || !item.anchors.includes(selectedAnchor)) selectedAnchor = item.anchors[0] || null;
    detail?.append(
      canvasHead(
        item.title,
        item.summary || item.path,
        [
          chip(item.id),
          item.status ? chip(item.status, 'green-chip') : null,
          item.priority ? chip(item.priority) : null,
        ],
        [
          actionButton(t('common.edit'), 'a11y.edit_requirement', () => {
            const current = projection.items.find(candidate => candidate.id === selectedItemId);
            if (current) openItemEditor(current.path, current.id);
          }),
          actionButton(t('diagnostics.phase.plan'), 'a11y.create_work', async () => {
            if (!selectedAnchor) return toast(t('toast.select_anchor'));
            const request = { schema: 'syu/work-request/v1', id: `WORK-${Date.now()}`, summary: `Implement ${selectedAnchor}`, operation: 'modify', seeds: [selectedAnchor], constraints: { include_facets: [], exclude_paths: [] }, requested_targets: [] };
            await api('/api/work/request', { method: 'PUT', body: JSON.stringify(request) });
            toast(t('toast.work_created'));
            location.assign('/?page=work');
          }, 'btn small primary compact'),
        ],
      ),
      (() => {
        const grid = el('div', 'grid2');
        const details = summaryCard('items.summary', el('p', '', item.description || item.summary || item.path));
        const planner = summaryCard('items.planner', (() => {
          const wrap = document.createElement('div');
          wrap.append(el('p', '', selectedAnchor ? `Exact seed available: ${selectedAnchor}` : 'No exact seed available'));
          return wrap;
        })());
        grid.append(details, planner);
        return grid;
      })(),
      ...renderItemSections(item),
      (() => {
        const note = el('div', 'notice');
        note.textContent = t('items.editor_notice');
        return note;
      })(),
    );
  }

  function renderItemSections(item) {
    const sections = [];
    if (item.principles.length) sections.push(renderStatementSection(t('items.principles'), item.principles.map(value => ({ title: value.anchor, body: value.statement, meta: value.applies_to.join(', ') }))));
    if (item.rules.length) sections.push(renderStatementSection(t('items.rules'), item.rules.map(value => ({ title: value.anchor, body: value.statement, meta: [value.level, ...value.governed_by].join(' · ') }))));
    if (item.criteria.length) sections.push(renderStatementSection(t('items.criteria'), item.criteria.map(value => ({ title: value.anchor, body: value.statement, meta: [value.kind, ...value.governed_by].join(' · ') }))));
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
      if (value.meta) block.append(el('p', 'muted', value.meta));
      card.append(block);
    });
    return fragment(title, card);
  }

  function renderBindingSection(bindings) {
    const title = el('div', 'section-label', t('items.bindings'));
    const card = el('div', 'card');
    card.style.padding = '8px 12px';
    bindings.forEach(binding => {
      binding.targets.forEach(target => {
        const row = el('div', 'path-row');
        row.append(
          el('span', 'path', binding.anchor),
          chip(binding.role),
          chip(binding.facet),
          el('span', 'muted', `${target.path} · ${target.selector}`),
        );
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
      block.append(
        el('div', 'path', contract.anchor),
        el('div', 'selector', `${contract.kind} · ${contract.source}`),
        el('p', 'muted', contract.participants.map(participant => `${participant.role}: ${participant.binding}`).join('\n')),
      );
      card.append(block);
    });
    return fragment(title, card);
  }

  function bindItemsTabs() {
    all('[data-tab-group="items"]').forEach(button => {
      button.addEventListener('click', () => {
        selectedItemKind = button.dataset.tab;
        renderItems();
      });
    });
  }

  function bindScopeTabs() {
    all('[data-tab-group="scope"]').forEach(button => {
      button.addEventListener('click', () => {
        selectedScopeGroup = button.dataset.tab;
        renderScope();
      });
    });
  }

  function bindDiagnostics() {
    const page = one('[data-page="diagnostics"]');
    const context = one('select', page);
    const range = one('[data-validation-range]', page);
    const validate = buttonByKey('a11y.validate_context');
    const planOption = context && [...context.options].find(option => option.value === 'work-plan');
    const sliceOption = context && [...context.options].find(option => option.value === 'slice');
    if (planOption) planOption.disabled = !plan;
    if (sliceOption) sliceOption.disabled = !plan?.slices.length;
    context?.addEventListener('change', () => { if (range) range.hidden = context.value !== 'git_range'; });
    const phaseTabs = all('[role="tab"]', page);
    ['all', 'config', 'graph', 'targets', 'scope', 'plan'].forEach((phase, index) => { if (phaseTabs[index]) phaseTabs[index].dataset.diagnosticPhase = phase; });
    validate?.addEventListener('click', async () => {
      const requested = context?.value || 'workspace';
      validate.disabled = true;
      all('[data-diagnostic-phase]', page).forEach(node => updatePhaseStatus(node, 'running'));
      try {
        const run = await api('/api/validate', { method: 'POST', body: JSON.stringify({ context: requested, range: requested === 'git_range' ? range?.value || null : null, slice: requested === 'slice' ? currentSlice()?.id || null : null }) });
        renderRun(run);
      } finally {
        validate.disabled = false;
      }
    });
    renderRun(lastRun);
  }

  function updatePhaseStatus(node, state) {
    const dot = one('.status-circle', node);
    if (!dot) return;
    dot.className = `status-circle tab-status ${phaseStateClass(state)}`;
    const key = phaseStateA11y(state);
    dot.setAttribute('aria-label', key ? t(key) : state);
  }

  function renderRun(run) {
    lastRun = run;
    const page = one('[data-page="diagnostics"]');
    const diagnosticsNav = one('[data-route="diagnostics"]');
    diagnosticsNav?.querySelector('.nav-badge')?.remove();
    const actionableIssues = (run.issue_counts?.error || 0) + (run.issue_counts?.warning || 0);
    if (diagnosticsNav && actionableIssues > 0) {
      const badge = el('span', 'nav-badge', String(actionableIssues));
      diagnosticsNav.append(badge);
    }
    all('[data-diagnostic-phase]', page).forEach(node => {
      const phase = run.phases.find(item => item.id === node.dataset.diagnosticPhase);
      const state = node.dataset.diagnosticPhase === 'all' ? run.state : (phase?.state || 'not_run');
      updatePhaseStatus(node, state);
      node.onclick = () => renderDiagnosticIssues(run, node.dataset.diagnosticPhase);
    });
    renderDiagnosticSummary(run);
    renderDiagnosticIssues(run, 'all');
  }

  function renderDiagnosticSummary(run) {
    const host = one('[data-diagnostic-result]');
    clear(host);
    const chips = [];
    if (run.state === 'passed') chips.push(chip(t('diagnostics.passed'), 'green-chip'));
    if (run.context) chips.push(chip(titleCase(run.context.replace('-', '_'))));
    if (run.basis) chips.push(chip(run.basis));
    host?.append(
      canvasHead(
        run.state === 'not_run' ? t('diagnostics.not_run.title')
          : run.state === 'failed' ? t('diagnostics.failed')
          : run.state === 'not_applicable' ? t('diagnostics.not_applicable')
          : run.diagnostics.length === 0 ? t('diagnostics.zero.title')
          : `${run.diagnostics.length} ${t('diagnostics.issues_found')}`,
        run.state === 'not_run' ? t('diagnostics.not_run.description')
          : run.state === 'failed' || run.state === 'not_applicable' ? (run.reason || '')
          : run.diagnostics.length === 0 ? t('diagnostics.zero.description')
          : t('diagnostics.inspect_phases'),
        chips,
        run.completed_at ? [chip(window.SyuPreferences.formatDate(run.completed_at), 'muted')] : [],
        [
          actionButton(t('filter.validate'), 'a11y.validate_context', async () => {
            const page = one('[data-page="diagnostics"]');
            const context = one('select', page);
            const range = one('[data-validation-range]', page);
            const requested = context?.value || 'workspace';
            const next = await api('/api/validate', { method: 'POST', body: JSON.stringify({ context: requested, range: requested === 'git_range' ? range?.value || null : null, slice: requested === 'slice' ? currentSlice()?.id || null : null }) });
            renderRun(next);
          }, 'btn primary compact'),
        ],
      ),
      (() => {
        const grid = el('div', 'grid3 validation-summary');
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
      })(),
    );
  }

  function renderDiagnosticIssues(run, phase) {
    const page = one('[data-page="diagnostics"]');
    const workspace = one('.workspace', page);
    let rail = one('.diagnostic-rail', workspace);
    const visible = run.diagnostics.filter(diagnostic => phase === 'all' || diagnostic.phase === phase);
    if (!visible.length) {
      rail?.remove();
      workspace?.classList.add('no-rail');
      return;
    }
    workspace?.classList.remove('no-rail');
    if (!rail) {
      rail = el('aside', 'rail diagnostic-rail');
      workspace?.prepend(rail);
    }
    clear(rail);
    rail.append(el('div', 'rail-title', t('diagnostics.title')));
    visible.forEach((diagnostic, index) => {
      const button = el('button', `rail-item${index === 0 ? ' active' : ''}`);
      button.append(el('span', `status-circle ${diagnostic.severity === 'error' ? 'red' : diagnostic.severity === 'warning' ? 'orange' : 'blue'}`));
      const label = document.createElement('span');
      label.append(el('b', '', diagnostic.rule_id), el('p', '', diagnostic.message));
      button.append(label);
      button.addEventListener('click', () => {
        all('.rail-item', rail).forEach(item => item.classList.toggle('active', item === button));
        renderDiagnosticDetail(run, diagnostic);
      });
      rail.append(button);
      if (index === 0) renderDiagnosticDetail(run, diagnostic);
    });
  }

  function renderDiagnosticDetail(run, diagnostic) {
    renderDiagnosticSummary(run);
    const host = one('[data-diagnostic-result]');
    const location = `${diagnostic.primary.path}:${diagnostic.primary.line ?? '-'}`;
    host?.append(
      el('div', 'section-label', diagnostic.rule_id),
      (() => {
        const card = el('div', 'card');
        card.append(el('h4', '', diagnostic.message), el('p', '', location), metaLine([chip(diagnostic.severity), chip(diagnostic.phase), chip(diagnostic.rule_id)]), el('div', 'notice', diagnostic.help || diagnostic.message));
        return card;
      })(),
    );
  }

  function bindActions() {
    buttonByKey('a11y.copy_locator')?.addEventListener('click', async () => {
      if (selectedScopeTarget?.reference) await navigator.clipboard.writeText(selectedScopeTarget.reference);
      toast(t('toast.locator_copied'));
    });
    buttonByKey('a11y.open_source')?.addEventListener('click', () => selectedScopeTarget && toast(selectedScopeTarget.resolved_path));
    buttonByKey('a11y.replan_work')?.addEventListener('click', async () => { await api('/api/work/replan', { method: 'POST' }); location.reload(); });
    buttonByKey('a11y.edit_request')?.addEventListener('click', () => openRequestEditor());
    buttonByKey('a11y.export_slice')?.addEventListener('click', async () => {
      const slice = currentSlice();
      if (!slice) return;
      const yaml = await api(`/api/context/${encodeURIComponent(slice.id)}`, { method: 'POST' });
      const link = document.createElement('a');
      link.href = URL.createObjectURL(new Blob([yaml], { type: 'application/yaml' }));
      link.download = `${slice.id}-context.yaml`;
      link.click();
      URL.revokeObjectURL(link.href);
    });
    buttonByKey('a11y.create_work')?.addEventListener('click', async () => {
      if (!selectedAnchor) return toast(t('toast.select_anchor'));
      const request = { schema: 'syu/work-request/v1', id: `WORK-${Date.now()}`, summary: `Implement ${selectedAnchor}`, operation: 'modify', seeds: [selectedAnchor], constraints: { include_facets: [], exclude_paths: [] }, requested_targets: [] };
      await api('/api/work/request', { method: 'PUT', body: JSON.stringify(request) });
      toast(t('toast.work_created'));
      location.assign('/?page=work');
    });
    buttonByKey('a11y.edit_requirement')?.addEventListener('click', () => {
      const item = projection.items.find(candidate => candidate.id === selectedItemId);
      if (item) openItemEditor(item.path, item.id);
    });
    buttonByKey('a11y.new_requirement')?.addEventListener('click', () => {
      const id = `REQ-NEW-${Date.now().toString().slice(-6)}`;
      openItemEditor(`docs/syu/requirements/${id.toLowerCase()}.yaml`, id, `schema: syu/spec/v1\nkind: requirements\nnamespace: workbench\ncategory: Workbench\nrequirements:\n  - id: ${id}\n    title: New requirement\n    description: Describe the requirement.\n    priority: medium\n    status: planned\n    criteria:\n      - id: acceptance\n        kind: behavior\n        statement: Describe the acceptance condition.\n        governed_by: []\n    bindings: []\n`);
    });
  }

  function openRequestEditor() {
    if (!plan) return;
    const canvas = one('[data-work-overview]');
    const form = el('form', 'form');
    const title = el('h2', '', t('a11y.edit_request'));
    const summaryLabel = el('label', '', t('work.request.summary'));
    summaryLabel.htmlFor = 'work-request-summary';
    const summary = el('textarea', 'textarea');
    summary.id = 'work-request-summary';
    summary.value = plan.request.summary;
    const operationLabel = el('label', '', t('work.request.operation'));
    operationLabel.htmlFor = 'work-request-operation';
    const operation = el('select', 'native-select');
    operation.id = 'work-request-operation';
    ['add', 'modify', 'remove', 'refactor', 'document', 'investigate'].forEach(value => {
      const option = document.createElement('option');
      option.value = value;
      option.textContent = value;
      option.selected = value === plan.request.operation;
      operation.append(option);
    });
    const save = el('button', 'btn primary', t('work.request.save'));
    save.type = 'submit';
    form.append(title, summaryLabel, summary, operationLabel, operation, save);
    clear(canvas);
    canvas?.append(form);
    form.addEventListener('submit', async event => {
      event.preventDefault();
      const request = structuredClone(plan.request);
      request.summary = summary.value;
      request.operation = operation.value;
      await api('/api/work/request', { method: 'PUT', body: JSON.stringify(request) });
      location.assign('/?page=work');
    });
  }

  async function openItemEditor(path, id, template = '') {
    const canvas = one('[data-items-detail]');
    const source = template ? { content: template, hash: await hashEmptySource(path) } : await api(`/api/source?path=${encodeURIComponent(path)}`);
    let previewToken = null;
    const heading = canvasHead(id, path);
    const notice = el('div', 'notice', t('items.editor_notice'));
    const details = document.createElement('details');
    const summary = document.createElement('summary');
    summary.textContent = t('items.yaml_source');
    const editor = el('textarea', 'textarea code');
    editor.value = source.content;
    details.append(summary, editor);
    const actions = el('div', 'actions');
    const preview = el('button', 'btn', t('common.preview'));
    const apply = el('button', 'btn primary', t('common.apply'));
    const cancel = el('button', 'btn ghost', t('common.reset'));
    apply.disabled = true;
    actions.append(cancel, preview, apply);
    clear(canvas);
    canvas?.append(heading, notice, details, actions);
    editor.addEventListener('input', () => { previewToken = null; apply.disabled = true; });
    preview.addEventListener('click', async () => {
      const result = await api('/api/file/preview', { method: 'POST', body: JSON.stringify({ path, content: editor.value, expected_hash: source.hash }) });
      previewToken = result.preview_token;
      apply.disabled = !previewToken;
      notice.textContent = previewToken ? `${result.changed_lines} ${t('settings.changed_lines')}` : result.validation_errors.join('\n');
    });
    apply.addEventListener('click', async () => {
      await api('/api/file/apply', { method: 'PUT', body: JSON.stringify({ path, content: editor.value, expected_hash: source.hash, preview_token: previewToken }) });
      location.assign('/?page=items');
    });
    cancel.addEventListener('click', () => renderItems());
  }

  async function hashEmptySource(path) {
    const source = await api(`/api/source?path=${encodeURIComponent(path)}`);
    return source.hash;
  }

  function bindSettings() {
    const page = one('[data-page="settings"]');
    if (!page) return;
    let config = structuredClone(projection.config);
    let configHash = null;
    let previewToken = null;
    let previewMode = 'structured';
    const general = one('[data-settings-page-panel="general"]', page);
    const generalFields = all('input, textarea', general);
    const profiles = one('[data-settings-page-panel="profiles"]', page);
    const profileFields = all('input, textarea', profiles);
    const validation = one('[data-settings-page-panel="validation"]', page);
    const validationFields = all('select, input', validation);
    const planning = one('[data-settings-page-panel="planning"]', page);
    const planningFields = all('input', planning);
    const adapters = one('[data-settings-page-panel="adapters"] input', page);
    const yamlPanel = one('[data-settings-page-panel="yaml"]', page);
    const yamlPreview = one('pre.code', yamlPanel);
    const yamlEditor = document.createElement('textarea');
    yamlEditor.className = 'textarea code';
    yamlPreview?.replaceWith(yamlEditor);
    const ruleField = el('textarea', 'textarea');
    ruleField.id = 'config-rule-overrides';
    const ruleContainer = el('div', 'field');
    const ruleLabel = document.createElement('label');
    ruleLabel.htmlFor = ruleField.id;
    ruleLabel.dataset.i18n = 'settings.rule_overrides';
    ruleLabel.textContent = t('settings.rule_overrides');
    ruleContainer.append(ruleLabel, ruleField);
    validation?.append(ruleContainer);
    const totalField = el('input', 'input');
    totalField.type = 'number';
    totalField.min = '1';
    totalField.id = 'config-total-bytes';
    const totalContainer = el('div', 'field');
    const totalLabel = document.createElement('label');
    totalLabel.htmlFor = totalField.id;
    totalLabel.dataset.i18n = 'settings.total_bytes';
    totalLabel.textContent = t('settings.total_bytes');
    totalContainer.append(totalLabel, totalField);
    planning?.querySelector('.form-row')?.append(totalContainer);
    const contextPrinciples = document.createElement('input');
    contextPrinciples.type = 'checkbox';
    const contextRules = document.createElement('input');
    contextRules.type = 'checkbox';
    [['settings.include_principles', contextPrinciples], ['settings.include_rules', contextRules]].forEach(([key, input]) => {
      const row = el('label', 'toggle-row');
      const label = document.createElement('span');
      label.dataset.i18n = key;
      label.textContent = t(key);
      row.append(label, input);
      planning?.append(row);
    });
    const split = value => value.split(',').map(item => item.trim()).filter(Boolean);
    const baselineText = baseline => baseline?.strategy === 'merge-base' ? baseline.against : baseline?.strategy === 'revision' ? baseline.revision : baseline?.strategy === 'parent' ? 'parent' : '';
    const populate = source => {
      config = source.config;
      configHash = source.hash;
      text(one('[data-settings-toolbar="workspace"] .select span:last-child', page), `source hash ${configHash.slice(0, 16)}…`);
      generalFields[0].value = config.workspace.spec_roots.join(', ');
      generalFields[1].value = config.workspace.artifact_roots.join(', ');
      generalFields[2].value = config.workspace.excludes.join(', ');
      profileFields[0].value = config.profiles.active.join(', ');
      profileFields[1].value = JSON.stringify(config.profiles.custom, null, 2);
      validationFields[0].value = config.validation.preset;
      validationFields[1].value = baselineText(config.validation.changed.baseline);
      ruleField.value = JSON.stringify(config.validation.rules, null, 2);
      const toggles = all('.toggle', validation);
      toggles[0]?.classList.toggle('off', !config.validation.deny_warnings);
      toggles[1]?.classList.toggle('off', !config.validation.changed.require_owned_changes);
      planningFields.slice(0, 4).forEach((field, index) => { field.value = [config.work.slicing.max_editable_files, config.work.slicing.max_editable_symbols, config.work.slicing.max_verification_targets, config.work.slicing.max_readonly_targets][index]; });
      totalField.value = config.work.slicing.max_total_bytes;
      contextPrinciples.checked = config.work.context.include_parent_principles;
      contextRules.checked = config.work.context.include_parent_rules;
      adapters.value = config.adapters.enabled.join(', ');
    };
    const collect = () => {
      config.workspace.spec_roots = split(generalFields[0].value);
      config.workspace.artifact_roots = split(generalFields[1].value);
      config.workspace.excludes = split(generalFields[2].value);
      config.profiles.active = split(profileFields[0].value);
      config.profiles.custom = JSON.parse(profileFields[1].value || '{}');
      config.validation.preset = validationFields[0].value;
      const baseline = validationFields[1].value.trim();
      config.validation.changed.baseline = baseline ? (baseline === 'parent' ? { strategy: 'parent' } : { strategy: 'merge-base', against: baseline }) : null;
      config.validation.rules = JSON.parse(ruleField.value || '{}');
      const toggles = all('.toggle', validation);
      config.validation.deny_warnings = !toggles[0]?.classList.contains('off');
      config.validation.changed.require_owned_changes = !toggles[1]?.classList.contains('off');
      [config.work.slicing.max_editable_files, config.work.slicing.max_editable_symbols, config.work.slicing.max_verification_targets, config.work.slicing.max_readonly_targets] = planningFields.slice(0, 4).map(field => Number(field.value));
      config.work.slicing.max_total_bytes = Number(totalField.value);
      config.work.context.include_parent_principles = contextPrinciples.checked;
      config.work.context.include_parent_rules = contextRules.checked;
      config.adapters.enabled = split(adapters.value);
      return config;
    };
    const load = async () => {
      const [source, structured] = await Promise.all([api('/api/source?path=syu.yaml'), api('/api/config')]);
      populate(structured);
      yamlEditor.value = source.content;
    };
    all('.toggle', validation).forEach(toggle => {
      toggle.setAttribute('role', 'switch');
      toggle.tabIndex = 0;
      toggle.addEventListener('click', () => { toggle.classList.toggle('off'); previewToken = null; });
    });
    const previewButton = buttonByKey('a11y.preview_config');
    const applyButton = buttonByKey('a11y.apply_config');
    if (applyButton) applyButton.disabled = true;
    previewButton?.addEventListener('click', async () => {
      const rawMode = !one('[data-settings-page-panel="yaml"]', page).hidden;
      previewMode = rawMode ? 'yaml' : 'structured';
      const result = rawMode
        ? await api('/api/file/preview', { method: 'POST', body: JSON.stringify({ path: 'syu.yaml', content: yamlEditor.value, expected_hash: configHash }) })
        : await api('/api/config/preview', { method: 'POST', body: JSON.stringify({ config: collect(), expected_hash: configHash }) });
      previewToken = result.preview_token;
      if (applyButton) applyButton.disabled = !previewToken;
      toast(previewToken ? `${result.changed_lines} ${t('settings.changed_lines')}` : result.validation_errors.join('\n'));
    });
    applyButton?.addEventListener('click', async () => {
      if (!previewToken) return;
      const result = previewMode === 'yaml'
        ? await api('/api/file/apply', { method: 'PUT', body: JSON.stringify({ path: 'syu.yaml', content: yamlEditor.value, expected_hash: configHash, preview_token: previewToken }) })
        : await api('/api/config/apply', { method: 'PUT', body: JSON.stringify({ config: collect(), expected_hash: configHash, preview_token: previewToken }) });
      configHash = result.new_hash;
      previewToken = null;
      applyButton.disabled = true;
      toast(t('common.apply'));
    });
    buttonByKey('a11y.open_yaml')?.addEventListener('click', () => {
      window.SyuPreferences.settingsLayer('workspace');
      window.SyuPreferences.settingsPage('workspace', 'yaml');
    });
    load().catch(error => toast(error.message));
  }

  function bindPalette() {
    const dialog = one('.palette-dialog');
    if (!dialog) return;
    const addTarget = (title, route, tab, focus) => {
      const button = el('button', 'palette-result');
      const icon = el('span', 'r-ico', '→');
      const copy = document.createElement('span');
      copy.append(el('b', '', title));
      const path = el('span', 'route', tab ? `${route} › ${tab}` : route);
      button.append(icon, copy, path);
      button.addEventListener('click', () => {
        one(`[data-route="${route}"]`)?.click();
        if (tab) one(`[data-tab-group="${route}"][data-tab="${tab}"]`)?.click();
        const target = focus && one(`[data-focus-id="${focus}"]`);
        target?.focus();
        target?.classList.add('focus-ring');
        setTimeout(() => target?.classList.remove('focus-ring'), 1800);
        one('.palette-overlay')?.classList.remove('open');
      });
      dialog.append(button);
    };
    plan?.slices.forEach(slice => addTarget(slice.goal, 'work', 'slices', null));
    projection.items.slice(0, 12).forEach(item => addTarget(item.id, 'items', item.kind, null));
  }

  function renderAll() {
    one('[data-route="work"] .nav-badge')?.remove();
    renderWork();
    renderScope();
    renderItems();
    renderRun(lastRun);
  }

  bindItemsTabs();
  bindScopeTabs();
  renderAll();
  bindDiagnostics();
  bindActions();
  bindSettings();
  bindPalette();
  document.addEventListener('syu:locale', () => renderAll());
})();
