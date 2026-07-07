(() => {
  'use strict';
  const stateNode = document.querySelector('#syu-projection');
  if (!stateNode) return;
  const projection = JSON.parse(stateNode.textContent);
  const one = (selector, root = document) => root.querySelector(selector);
  const all = (selector, root = document) => [...root.querySelectorAll(selector)];
  const text = (node, value) => { if (node) node.textContent = value ?? ''; };
  const buttonByKey = key => one(`button[data-i18n-aria="${key}"]`);
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
  const t = key => window.SyuPreferences.t(key);
  const statusLabel = status => ({ ready: t('work.status.ready'), needs_review: t('work.status.needs_review'), blocked: t('work.status.blocked') })[status] || status;
  const plan = projection.plan;
  let lastRun = projection.validation;
  one('[data-route="work"] .nav-badge')?.remove();

  function bindWork() {
    const page = one('[data-page="work"]');
    if (!plan) {
      const canvas = one('[data-panel="overview"] .canvas', page); const empty = document.createElement('div'); empty.className = 'empty-state';
      const copy = document.createElement('div'); const title = document.createElement('h2'); title.textContent = t('work.empty.title'); const description = document.createElement('p'); description.textContent = t('work.empty.description'); copy.append(title, description); empty.append(copy); canvas?.replaceChildren(empty);
      all('[data-tab-group="work"]', page).forEach(tab => { if (tab.dataset.tab !== 'overview') tab.disabled = true; }); return;
    }
    text(one('[data-focus-id="work-plan-selector"] span', page), `${plan.id} · ${plan.request.summary}`);
    const overview = one('[data-panel="overview"]', page);
    text(one('.canvas-head h2', overview), plan.request.summary);
    const chips = all('.meta-line .chip', overview);
    text(chips[0], statusLabel(plan.status));
    text(chips[1], plan.request.operation);
    text(chips[2], `basis ${plan.basis.revision.slice(0, 9)}`);
    text(one('[data-tab="slices"] .mini-count', page), plan.slices.length);
    text(one('[data-tab="validation"] .mini-count', page), plan.diagnostics.length);
    renderSlice(plan.slices[0]);
    const rail = one('[data-panel="slices"] .rail', page);
    if (rail) {
      rail.replaceChildren();
      const title = document.createElement('div'); title.className = 'rail-title'; title.textContent = '▣ Execution slices'; rail.append(title);
      plan.slices.forEach((slice, index) => {
        const button = document.createElement('button');
        button.className = `rail-item${index === 0 ? ' active' : ''}`;
        button.dataset.sliceId = slice.id;
        const dot = document.createElement('span'); dot.className = 'status-circle green'; dot.setAttribute('aria-label', 'exact');
        const copy = document.createElement('span');
        const heading = document.createElement('b'); heading.textContent = `${slice.id} · ${slice.goal}`;
        const summary = document.createElement('p'); summary.textContent = slice.acceptance[0]?.statement || slice.goal;
        copy.append(heading, summary);
        const count = document.createElement('span'); count.className = 'n'; count.textContent = slice.editable_targets.length;
        button.append(dot, copy, count);
        button.addEventListener('click', () => {
          all('.rail-item', rail).forEach(item => item.classList.toggle('active', item === button));
          renderSlice(slice);
        });
        rail.append(button);
      });
    }
  }

  function renderSlice(slice) {
    if (!slice) return;
    const canvas = one('[data-page="work"] [data-panel="slices"] .canvas');
    if (!canvas) return;
    text(one('.canvas-head h2', canvas), `${slice.id} · ${slice.goal}`);
    text(one('.canvas-head p', canvas), slice.acceptance[0]?.statement || slice.goal);
    const chips = all('.canvas-head .chip', canvas);
    text(chips[0], 'Exact');
    text(chips[1], `${slice.editable_targets.length} editable`);
    text(chips[2], `${slice.verification_targets.length} verification`);
    text(chips[3], `${slice.readonly_context.length} reference`);
    const rows = one('.card[style]') || one('[data-panel="slices"] .card');
    const targetHost = one('[data-panel="slices"] .section-label')?.nextElementSibling;
    if (targetHost) {
      targetHost.replaceChildren();
      [...slice.editable_targets, ...slice.verification_targets, ...slice.readonly_context].forEach(target => {
        const row = document.createElement('div'); row.className = 'path-row';
        for (const value of [target.resolved_path, target.transition, target.access, target.reason]) {
          const cell = document.createElement('span'); cell.textContent = value; row.append(cell);
        }
        targetHost.append(row);
      });
    }
    void rows;
  }

  function bindContext() {
    if (!plan?.slices.length) return;
    const panel = one('[data-page="work"] [data-panel="context"]');
    const slice = plan.slices[0];
    text(one('.canvas-head h2', panel), `Context Pack · ${slice.id}`);
    const groups = [slice.editable_targets, slice.verification_targets, slice.readonly_context];
    all('.rail-item .n', panel).slice(0, 3).forEach((count, index) => text(count, groups[index].length));
    const targetGrid = one('.grid2', panel);
    if (targetGrid) {
      targetGrid.replaceChildren();
      [...slice.editable_targets, ...slice.verification_targets, ...slice.readonly_context].forEach(target => {
        const card = document.createElement('div'); card.className = 'target-locator';
        const path = document.createElement('div'); path.className = 'path'; path.textContent = target.resolved_path;
        const selector = document.createElement('div'); selector.className = 'selector'; selector.textContent = target.reference;
        const reason = document.createElement('p'); reason.className = 'muted'; reason.textContent = target.reason;
        card.append(path, selector, reason); targetGrid.append(card);
      });
    }
    const download = buttonByKey('a11y.download_context');
    download?.addEventListener('click', async () => {
      const yaml = await api(`/api/context/${encodeURIComponent(slice.id)}`, { method: 'POST' });
      const link = document.createElement('a'); link.href = URL.createObjectURL(new Blob([yaml], { type: 'application/yaml' })); link.download = `${slice.id}-context.yaml`; link.click(); URL.revokeObjectURL(link.href);
    });
  }

  let selectedTarget = null;
  function bindScope() {
    const page = one('[data-page="scope"]');
    if (!plan) {
      const canvas = one('.canvas', page); const empty = document.createElement('div'); empty.className = 'empty-state'; const title = document.createElement('h2'); title.textContent = t('scope.empty.title'); const description = document.createElement('p'); description.textContent = t('scope.empty.description'); empty.append(title, description); canvas?.replaceChildren(empty); return;
    }
    const targets = plan.slices.flatMap(slice => [
      ...slice.editable_targets.map(target => ({ slice, target, group: 'change' })),
      ...slice.verification_targets.map(target => ({ slice, target, group: 'verify' })),
      ...slice.readonly_context.map(target => ({ slice, target, group: 'reference' })),
    ]);
    const rail = one('.rail', page);
    const renderRail = group => {
      if (!rail) return;
      rail.replaceChildren();
      targets.filter(entry => group === 'intent' || entry.group === group).forEach((entry, index) => {
        const button = document.createElement('button'); button.className = `rail-item${index === 0 ? ' active' : ''}`;
        const dot = document.createElement('span'); dot.className = 'status-circle green'; dot.setAttribute('aria-label', 'exact target');
        const label = document.createElement('span');
        const heading = document.createElement('b'); heading.textContent = entry.target.resolved_selector.description;
        const detail = document.createElement('p'); detail.textContent = `${entry.slice.id} · ${entry.target.resolved_path}`;
        label.append(heading, detail); button.append(dot, label);
        button.addEventListener('click', () => { all('.rail-item', rail).forEach(item => item.classList.toggle('active', item === button)); renderTarget(entry); });
        rail.append(button);
        if (index === 0) renderTarget(entry);
      });
    };
    all('[data-tab-group="scope"]', page).forEach(tab => tab.addEventListener('click', () => renderRail(tab.dataset.tab)));
    renderRail('change');
  }

  function renderTarget(entry) {
    if (!entry) return;
    selectedTarget = entry.target;
    const canvas = one('[data-page="scope"] .canvas');
    text(one('.canvas-head h2', canvas), entry.target.resolved_selector.description);
    text(one('.canvas-head p', canvas), entry.target.reason);
    text(one('.target-locator .path', canvas), entry.target.resolved_path);
    text(one('.target-locator .selector', canvas), entry.target.reference);
  }

  let selectedAnchor = null;
  let selectedItem = null;
  function bindItems() {
    const page = one('[data-page="items"]');
    const rail = one('.rail', page);
    if (!rail) return;
    rail.replaceChildren();
    projection.items.forEach((item, index) => {
      const button = document.createElement('button'); button.className = `rail-item${index === 0 ? ' active' : ''}`;
      button.dataset.kind = item.kind;
      const label = document.createElement('span');
      const heading = document.createElement('b'); heading.textContent = item.id;
      const detail = document.createElement('p'); detail.textContent = item.path;
      label.append(heading, detail); button.append(label);
      button.addEventListener('click', () => {
        all('.rail-item', rail).forEach(node => node.classList.toggle('active', node === button));
        selectedAnchor = item.anchors[0] || null;
        selectedItem = item;
        const canvas = one('.canvas', page);
        text(one('.canvas-head h2', canvas), item.id);
        text(one('.canvas-head p', canvas), item.path);
        const host = one('.list', canvas);
        if (host) {
          host.replaceChildren();
          item.anchors.forEach(anchor => {
            const row = document.createElement('li');
            const radio = document.createElement('input'); radio.type = 'radio'; radio.name = 'work-seed'; radio.value = anchor; radio.checked = anchor === selectedAnchor;
            radio.addEventListener('change', () => { selectedAnchor = anchor; });
            const value = document.createElement('span'); value.textContent = anchor;
            row.append(radio, value); host.append(row);
          });
        }
      });
      rail.append(button);
      if (index === 0) button.click();
    });
    const filterKind = kind => {
      const candidates = all('.rail-item', rail); candidates.forEach(button => { button.hidden = button.dataset.kind !== kind; });
      const selected = candidates.find(button => !button.hidden); selected?.click();
    };
    all('[data-tab-group="items"]', page).forEach(tab => tab.addEventListener('click', () => filterKind(tab.dataset.tab)));
    filterKind('requirement');
    const planButton = buttonByKey('a11y.create_work');
    planButton?.addEventListener('click', async () => {
      if (!selectedAnchor) return toast(t('toast.select_anchor'));
      const request = { schema: 'syu/work-request/v1', id: `WORK-${Date.now()}`, summary: `Implement ${selectedAnchor}`, operation: 'modify', seeds: [selectedAnchor], constraints: { include_facets: [], exclude_paths: [] }, requested_targets: [] };
      await api('/api/work/request', { method: 'PUT', body: JSON.stringify(request) });
      toast(t('toast.work_created'));
      location.assign('/?page=work');
    });
    buttonByKey('a11y.edit_requirement')?.addEventListener('click', () => selectedItem && openItemEditor(selectedItem.path, selectedItem.id));
    buttonByKey('a11y.new_requirement')?.addEventListener('click', () => {
      const id = `REQ-NEW-${Date.now().toString().slice(-6)}`;
      openItemEditor(`docs/syu/requirements/${id.toLowerCase()}.yaml`, id, `schema: syu/spec/v1\nkind: requirements\nnamespace: workbench\ncategory: Workbench\nrequirements:\n  - id: ${id}\n    title: New requirement\n    description: Describe the requirement.\n    priority: medium\n    status: planned\n    criteria:\n      - id: acceptance\n        kind: behavior\n        statement: Describe the acceptance condition.\n        governed_by: []\n    bindings: []\n`);
    });
  }

  async function openItemEditor(path, id, template = '') {
    const page = one('[data-page="items"]');
    const canvas = one('.canvas', page);
    const source = template ? { content: template, hash: await hashEmptySource(path) } : await api(`/api/source?path=${encodeURIComponent(path)}`);
    let previewToken = null;
    const heading = document.createElement('div'); heading.className = 'canvas-head';
    const copy = document.createElement('div'); const title = document.createElement('h2'); title.textContent = id;
    const summary = document.createElement('p'); summary.textContent = path; copy.append(title, summary); heading.append(copy);
    const notice = document.createElement('div'); notice.className = 'notice'; notice.textContent = t('items.editor_notice');
    const details = document.createElement('details');
    const detailsTitle = document.createElement('summary'); detailsTitle.textContent = t('items.yaml_source');
    const editor = document.createElement('textarea'); editor.className = 'textarea code'; editor.value = source.content;
    details.append(detailsTitle, editor);
    const actions = document.createElement('div'); actions.className = 'actions';
    const preview = document.createElement('button'); preview.className = 'btn'; preview.textContent = t('common.preview');
    const apply = document.createElement('button'); apply.className = 'btn primary'; apply.textContent = t('common.apply'); apply.disabled = true;
    const cancel = document.createElement('button'); cancel.className = 'btn ghost'; cancel.textContent = t('common.reset');
    actions.append(cancel, preview, apply); canvas.replaceChildren(heading, notice, details, actions);
    editor.addEventListener('input', () => { previewToken = null; apply.disabled = true; });
    preview.addEventListener('click', async () => {
      const result = await api('/api/file/preview', { method: 'POST', body: JSON.stringify({ path, content: editor.value, expected_hash: source.hash }) });
      previewToken = result.preview_token; apply.disabled = !previewToken; notice.textContent = previewToken ? `${result.changed_lines} ${t('settings.changed_lines')}` : result.validation_errors.join('\n');
    });
    apply.addEventListener('click', async () => {
      await api('/api/file/apply', { method: 'PUT', body: JSON.stringify({ path, content: editor.value, expected_hash: source.hash, preview_token: previewToken }) });
      location.assign('/?page=items');
    });
    cancel.addEventListener('click', () => location.reload());
  }

  async function hashEmptySource(path) {
    const source = await api(`/api/source?path=${encodeURIComponent(path)}`);
    return source.hash;
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
      all('[data-diagnostic-phase] .status-circle', page).forEach(status => { status.className = 'status-circle blue running tab-status'; status.setAttribute('aria-label', t('a11y.running')); });
      try {
        const run = await api('/api/validate', { method: 'POST', body: JSON.stringify({ context: requested, range: requested === 'git_range' ? range?.value || null : null, slice: requested === 'slice' ? plan?.slices[0]?.id : null }) });
        renderRun(run);
      } finally { validate.disabled = false; }
    });
    renderRun(projection.validation);
  }

  function renderRun(run) {
    lastRun = run;
    const page = one('[data-page="diagnostics"]');
    const diagnosticsNav = one('[data-route="diagnostics"]');
    diagnosticsNav?.querySelector('.nav-badge')?.remove();
    const actionableIssues = (run.issue_counts?.error || 0) + (run.issue_counts?.warning || 0);
    if (diagnosticsNav && actionableIssues > 0) { const badge = document.createElement('span'); badge.className = 'nav-badge'; badge.textContent = actionableIssues; diagnosticsNav.append(badge); }
    text(one('[data-diagnostic-state]', page), run.state);
    const canvas = one('[data-diagnostic-result]', page);
    if (!canvas) return;
    const title = one('.canvas-head h2', canvas);
    const description = one('.canvas-head p', canvas);
    if (run.state === 'not_run') {
      text(title, t('diagnostics.not_run.title')); text(description, t('diagnostics.not_run.description'));
    } else if (run.state === 'not_applicable' || run.state === 'failed') {
      text(title, run.state === 'failed' ? t('diagnostics.failed') : t('diagnostics.not_applicable')); text(description, run.reason);
    } else if (run.diagnostics.length === 0) {
      text(title, window.SYU_I18N[document.documentElement.lang]['diagnostics.zero.title']);
      text(description, window.SYU_I18N[document.documentElement.lang]['diagnostics.zero.description']);
    } else {
      text(title, `${run.diagnostics.length} ${t('diagnostics.issues_found')}`);
      text(description, t('diagnostics.inspect_phases'));
    }
    const stats = all('.validation-summary .big-stat', canvas);
    text(stats[0], run.evaluated_rule_count);
    text(stats[1], run.applicable_phase_count);
    text(stats[2], run.skipped_phase_count);
    all('[data-diagnostic-phase]', page).forEach(node => {
      const phase = run.phases.find(item => item.id === node.dataset.diagnosticPhase);
      const aggregateState = node.dataset.diagnosticPhase === 'all' ? run.state : null;
      node.dataset.state = aggregateState || phase?.state || 'not_applicable';
      const status = one('.status-circle', node);
      if (status) {
        const state = aggregateState || phase?.state || run.state;
        status.className = `status-circle tab-status ${state === 'passed' ? 'green' : state === 'issues' ? 'red' : state === 'running' ? 'blue running' : 'gray'}`;
        const stateKey = { passed: 'a11y.passed', issues: 'a11y.issues', running: 'a11y.running', not_run: 'a11y.not_run', not_applicable: 'a11y.not_applicable', failed: 'diagnostics.failed' }[state];
        status.setAttribute('aria-label', stateKey ? t(stateKey) : state);
      }
      node.onclick = () => renderDiagnosticIssues(run, node.dataset.diagnosticPhase);
    });
    renderDiagnosticIssues(run, 'all');
  }

  function renderDiagnosticIssues(run, phase) {
    const page = one('[data-page="diagnostics"]');
    const workspace = one('.workspace', page);
    let rail = one('.diagnostic-rail', workspace);
    if (!run.diagnostics.length) {
      rail?.remove(); workspace?.classList.add('no-rail'); return;
    }
    workspace?.classList.remove('no-rail');
    if (!rail) { rail = document.createElement('aside'); rail.className = 'rail diagnostic-rail'; workspace?.prepend(rail); }
    rail.replaceChildren();
    const visible = run.diagnostics.filter(diagnostic => phase === 'all' || diagnostic.phase === phase);
    visible.forEach((diagnostic, index) => {
      const button = document.createElement('button'); button.className = `rail-item${index === 0 ? ' active' : ''}`;
      const dot = document.createElement('span'); dot.className = `status-circle ${diagnostic.severity === 'error' ? 'red' : diagnostic.severity === 'warning' ? 'orange' : 'blue'}`; dot.setAttribute('aria-label', diagnostic.severity);
      const label = document.createElement('span'); const id = document.createElement('b'); id.textContent = diagnostic.rule_id; const message = document.createElement('p'); message.textContent = diagnostic.message; label.append(id, message); button.append(dot, label);
      button.addEventListener('click', () => { all('.rail-item', rail).forEach(item => item.classList.toggle('active', item === button)); renderDiagnosticDetail(diagnostic); });
      rail.append(button); if (index === 0) renderDiagnosticDetail(diagnostic);
    });
  }

  function renderDiagnosticDetail(diagnostic) {
    const canvas = one('[data-page="diagnostics"] [data-diagnostic-result]');
    if (!canvas) return;
    const location = `${diagnostic.primary.path}:${diagnostic.primary.line ?? '-'}`;
    canvas.replaceChildren();
    const head = document.createElement('div'); head.className = 'canvas-head';
    const copy = document.createElement('div'); const title = document.createElement('h2'); title.textContent = diagnostic.message; const path = document.createElement('p'); path.textContent = location; copy.append(title, path); head.append(copy);
    const meta = document.createElement('div'); meta.className = 'meta-line';
    for (const value of [diagnostic.severity, diagnostic.phase, diagnostic.rule_id]) { const chip = document.createElement('span'); chip.className = 'chip'; chip.textContent = value; meta.append(chip); }
    const help = document.createElement('div'); help.className = 'notice'; help.textContent = diagnostic.help || diagnostic.message;
    canvas.append(head, meta, help);
  }

  function bindActions() {
    buttonByKey('a11y.copy_locator')?.addEventListener('click', async () => {
      if (selectedTarget) await navigator.clipboard.writeText(selectedTarget.reference);
      toast(t('toast.locator_copied'));
    });
    buttonByKey('a11y.open_source')?.addEventListener('click', () => selectedTarget && toast(selectedTarget.resolved_path));
    buttonByKey('a11y.replan_work')?.addEventListener('click', async () => { await api('/api/work/replan', { method: 'POST' }); location.reload(); });
    buttonByKey('a11y.edit_request')?.addEventListener('click', () => openRequestEditor());
  }

  function openRequestEditor() {
    if (!plan) return;
    const canvas = one('[data-page="work"] [data-panel="overview"] .canvas');
    const form = document.createElement('form'); form.className = 'form';
    const title = document.createElement('h2'); title.textContent = t('a11y.edit_request');
    const summaryLabel = document.createElement('label'); summaryLabel.textContent = t('work.request.summary'); summaryLabel.htmlFor = 'work-request-summary';
    const summary = document.createElement('textarea'); summary.id = 'work-request-summary'; summary.className = 'textarea'; summary.value = plan.request.summary;
    const operationLabel = document.createElement('label'); operationLabel.textContent = t('work.request.operation'); operationLabel.htmlFor = 'work-request-operation';
    const operation = document.createElement('select'); operation.id = 'work-request-operation'; operation.className = 'native-select';
    for (const value of ['add', 'modify', 'remove', 'refactor', 'document', 'investigate']) { const option = document.createElement('option'); option.value = value; option.textContent = value; option.selected = value === plan.request.operation; operation.append(option); }
    const save = document.createElement('button'); save.className = 'btn primary'; save.type = 'submit'; save.textContent = t('work.request.save');
    form.append(title, summaryLabel, summary, operationLabel, operation, save); canvas?.replaceChildren(form);
    form.addEventListener('submit', async event => { event.preventDefault(); const request = structuredClone(plan.request); request.summary = summary.value; request.operation = operation.value; await api('/api/work/request', { method: 'PUT', body: JSON.stringify(request) }); location.assign('/?page=work'); });
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
    yamlEditor.className = 'textarea code'; yamlEditor.dataset.configYaml = '';
    yamlPreview?.replaceWith(yamlEditor);
    const ruleField = document.createElement('textarea'); ruleField.className = 'textarea'; ruleField.id = 'config-rule-overrides';
    const ruleContainer = document.createElement('div'); ruleContainer.className = 'field';
    const ruleLabel = document.createElement('label'); ruleLabel.htmlFor = ruleField.id; ruleLabel.dataset.i18n = 'settings.rule_overrides'; ruleLabel.textContent = t('settings.rule_overrides');
    ruleContainer.append(ruleLabel, ruleField); validation?.append(ruleContainer);
    const totalField = document.createElement('input'); totalField.className = 'input'; totalField.type = 'number'; totalField.min = '1'; totalField.id = 'config-total-bytes';
    const totalContainer = document.createElement('div'); totalContainer.className = 'field';
    const totalLabel = document.createElement('label'); totalLabel.htmlFor = totalField.id; totalLabel.dataset.i18n = 'settings.total_bytes'; totalLabel.textContent = t('settings.total_bytes'); totalContainer.append(totalLabel, totalField); planning?.querySelector('.form-row')?.append(totalContainer);
    const contextPrinciples = document.createElement('input'); contextPrinciples.type = 'checkbox';
    const contextRules = document.createElement('input'); contextRules.type = 'checkbox';
    for (const [key, input] of [['settings.include_principles', contextPrinciples], ['settings.include_rules', contextRules]]) {
      const row = document.createElement('label'); row.className = 'toggle-row'; const label = document.createElement('span'); label.dataset.i18n = key; label.textContent = t(key); row.append(label, input); planning?.append(row);
    }
    const split = value => value.split(',').map(item => item.trim()).filter(Boolean);
    const baselineText = baseline => baseline?.strategy === 'merge-base' ? baseline.against : baseline?.strategy === 'revision' ? baseline.revision : baseline?.strategy === 'parent' ? 'parent' : '';
    const populate = source => {
      config = source.config; configHash = source.hash;
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
      populate(structured); yamlEditor.value = source.content;
    };
    all('.toggle', validation).forEach(toggle => { toggle.setAttribute('role', 'switch'); toggle.tabIndex = 0; toggle.addEventListener('click', () => { toggle.classList.toggle('off'); previewToken = null; }); });
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
      configHash = result.new_hash; previewToken = null; applyButton.disabled = true; toast(t('common.apply'));
    });
    buttonByKey('a11y.open_yaml')?.addEventListener('click', () => window.SyuPreferences.settingsPage('workspace', 'yaml'));
    load().catch(error => toast(error.message));
  }

  function bindPalette() {
    const dialog = one('.palette-dialog');
    if (!dialog) return;
    const addTarget = (title, route, tab, focus) => {
      const button = document.createElement('button'); button.className = 'palette-result';
      const icon = document.createElement('span'); icon.className = 'r-ico'; icon.textContent = '→';
      const copy = document.createElement('span'); const heading = document.createElement('b'); heading.textContent = title; copy.append(heading);
      const path = document.createElement('span'); path.className = 'route'; path.textContent = tab ? `${route} › ${tab}` : route;
      button.append(icon, copy, path);
      button.addEventListener('click', () => {
        one(`[data-route="${route}"]`)?.click();
        if (tab) one(`[data-tab-group="${route}"][data-tab="${tab}"]`)?.click();
        const target = focus && one(`[data-focus-id="${focus}"]`); target?.focus(); target?.classList.add('focus-ring'); setTimeout(() => target?.classList.remove('focus-ring'), 1800);
        one('.palette-overlay')?.classList.remove('open');
      });
      dialog.append(button);
    };
    plan?.slices.forEach(slice => addTarget(slice.goal, 'work', 'slices', null));
    projection.items.slice(0, 12).forEach(item => addTarget(item.id, 'items', item.kind, null));
  }

  bindWork();
  bindContext();
  bindScope();
  bindItems();
  bindDiagnostics();
  bindActions();
  bindSettings();
  bindPalette();
  document.addEventListener('syu:locale', () => { bindWork(); bindScope(); renderRun(lastRun); });
})();
