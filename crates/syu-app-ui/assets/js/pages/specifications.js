import { localizeEnum, localizeSpecificationTitle, translate } from '../i18n.js';
import { SPECIFICATION_DETAIL_TABS, syncSpecificationLocation } from '../router.js';

const t = key => translate(key);

function button(label, icon, onClick, className = 'btn small') {
  const node = document.createElement('button');
  node.type = 'button';
  node.className = className;
  node.setAttribute('aria-label', label);
  node.title = label;
  node.innerHTML = `<span class="btn-icon" aria-hidden="true">${icon}</span><span class="btn-label">${label}</span>`;
  node.addEventListener('click', onClick);
  return node;
}

function status(text) {
  const node = document.createElement('span');
  const normalized = String(text || 'planned').toLowerCase().replace(/\s+/g, '-');
  node.className = `chip status-component status-${normalized}`;
  const translatedStatuses = new Set(['modified', 'added', 'deleted', 'renamed', 'untracked', 'binary', 'planned', 'implemented', 'deprecated', 'ready', 'blocked', 'needs-review', 'unknown']);
  const label = translatedStatuses.has(normalized)
    ? localizeEnum('status', normalized)
    : ['philosophy', 'policy', 'requirement', 'feature'].includes(normalized)
      ? localizeEnum('items', normalized)
      : text || normalized;
  node.innerHTML = '<span class="status-dot" aria-hidden="true"></span>';
  const copy = document.createElement('span');
  copy.textContent = label;
  node.append(copy);
  return node;
}

function enumChip(namespace, value) {
  const node = document.createElement('span');
  node.className = 'chip';
  node.textContent = localizeEnum(namespace, value);
  return node;
}

function localizedOptions(namespace, values) {
  return values.map(value => ({ value, label: localizeEnum(namespace, value) }));
}

async function runBusy(state, task) {
  if (state.busy) return;
  state.busy = true;
  state.busyLabel = t('common.loading');
  state.render();
  try {
    await task();
  } catch (error) {
    state.specificationError = error.message;
  }
  state.busy = false;
  state.busyLabel = '';
  state.render();
}

function field(label, value, name, type = 'text', options = []) {
  const wrapper = document.createElement('label');
  wrapper.className = 'spec-field';
  const caption = document.createElement('span');
  caption.textContent = label;
  wrapper.append(caption);
  if (type === 'textarea') {
    const input = document.createElement('textarea');
    input.name = name;
    input.className = 'textarea';
    input.rows = 3;
    input.value = value || '';
    wrapper.append(input);
  } else if (type === 'select') {
    const input = document.createElement('select');
    input.name = name;
    input.className = 'native-select';
    options.forEach(option => {
      const item = document.createElement('option');
      const entry = typeof option === 'string' ? { value: option, label: option } : option;
      item.value = entry.value;
      item.textContent = entry.label;
      item.selected = entry.value === value;
      input.append(item);
    });
    wrapper.append(input);
  } else {
    const input = document.createElement('input');
    input.name = name;
    input.className = 'input';
    input.type = type;
    input.value = value || '';
    wrapper.append(input);
  }
  return wrapper;
}

function itemFromCandidate(candidate) {
  return candidate?.item || candidate;
}

function candidatesFor(state) {
  const candidates = state.specificationCandidates
    || (state.projection.specifications?.specifications || []).map(item => ({ item, matches: [], relevance: [] }));
  if (!state.specificationKind || state.specificationKind === 'all') return candidates;
  return candidates.filter(candidate => itemFromCandidate(candidate).kind === state.specificationKind);
}

function itemById(state, id) {
  return candidatesFor(state).map(itemFromCandidate).find(item => item.id === id);
}

function formValue(form, name) {
  return typeof form?.get === 'function' ? form.get(name) : form?.[name];
}

function createItemPatch(item, form) {
  const fields = { kind: item.kind };
  if (item.kind === 'philosophy' || item.kind === 'policy') {
    fields.title = formValue(form, 'title');
    fields.summary = formValue(form, 'summary');
  } else if (item.kind === 'requirement') {
    fields.title = formValue(form, 'title');
    fields.description = formValue(form, 'description');
    fields.priority = formValue(form, 'priority');
    fields.status = formValue(form, 'status');
  } else {
    fields.title = formValue(form, 'title');
    fields.summary = formValue(form, 'summary');
    fields.status = formValue(form, 'status');
  }
  return { kind: 'specification', item_id: item.id, fields };
}

function patchFromForm(editor, state, form) {
  const draft = Object.fromEntries(new FormData(form).entries());
  editor.draft = draft;
  return editor.mode === 'create'
    ? createWizardPatch(editor, draft)
    : editor.anchor
      ? createAnchorPatch(editor, draft)
      : createItemPatch(itemById(state, editor.itemId), draft);
}

function draftValue(editor, draft, name, fallback = '') {
  return Object.prototype.hasOwnProperty.call(draft, name) ? draft[name] : fallback;
}

function createAnchorPatch(editor, form) {
  const fields = { anchor_kind: editor.anchorKind };
  fields.statement = formValue(form, 'statement');
  if (editor.anchorKind === 'criterion') fields.kind = formValue(form, 'criterion_kind');
  if (editor.anchorKind === 'principle') fields.applies_to = String(formValue(form, 'applies_to') || '')
    .split(',').map(value => value.trim()).filter(Boolean);
  return { kind: 'anchor', anchor: editor.anchor, fields };
}

function createWizardPatch(editor, form) {
  const base = {
    document: formValue(form, 'document'),
    id: formValue(form, 'id'),
    title: formValue(form, 'title'),
    status: 'planned',
  };
  if (editor.createKind === 'feature') {
    return { kind: 'create_feature', ...base, summary: formValue(form, 'summary') };
  }
  return {
    kind: 'create_requirement',
    ...base,
    description: formValue(form, 'description'),
    priority: formValue(form, 'priority'),
    criteria: [{
      id: formValue(form, 'criterion_id'),
      kind: formValue(form, 'criterion_kind'),
      statement: formValue(form, 'criterion_statement'),
      governed_by: editor.governedBy || [],
    }],
  };
}

function renderImpact(root, preview) {
  const impact = preview?.impact;
  if (!impact) return;
  const card = document.createElement('section');
  card.className = 'card specification-impact';
  const heading = document.createElement('h3');
  heading.textContent = t('items.preview.impact');
  card.append(heading);
  const states = document.createElement('div');
  states.className = 'meta-line';
  states.append(status(`${t('items.preview.before')}: ${localizeEnum('status', impact.readiness_before.status)}`));
  states.append(status(`${t('items.preview.after')}: ${localizeEnum('status', impact.readiness_after.status)}`));
  card.append(states);
  const list = document.createElement('ul');
  list.className = 'compact-list';
  const addEntries = (label, entries) => {
    const values = entries || [];
    const row = document.createElement('li');
    row.textContent = `${label}: ${values.length}`;
    if (values.length) {
      const detail = document.createElement('ul');
      values.forEach(value => {
        const item = document.createElement('li');
        item.textContent = value;
        detail.append(item);
      });
      row.append(detail);
    }
    list.append(row);
  };
  addEntries(t('items.preview.anchors'), impact.changed_anchors);
  addEntries(t('items.preview.ownership'), impact.affected_ownership);
  addEntries(t('items.preview.targets'), impact.implementation_targets);
  addEntries(t('items.preview.tests'), impact.verification_targets);
  const suggested = (impact.target_suggestions || []).flatMap(set => set.suggestions || []);
  addEntries(t('items.suggestions.title'), suggested.map(candidate => `${candidate.ref} (${localizeEnum('suggestion.confidence', candidate.confidence)})`));
  if (impact.work?.reason) {
    const row = document.createElement('li');
    row.textContent = impact.work.reason;
    list.append(row);
  }
  card.append(list);
  root.append(card);
}

function renderTargetSuggestions(root, state) {
  const set = state.targetSuggestions;
  if (!set) return;
  const card = document.createElement('section');
  card.className = 'card target-suggestions';
  const head = document.createElement('div');
  head.className = 'canvas-head';
  const heading = document.createElement('h3');
  heading.textContent = t('items.suggestions.title');
  head.append(heading);
  head.append(button(t('common.close'), '×', () => {
    state.targetSuggestions = null;
    state.targetSuggestionSelection = [];
    state.render();
  }, 'btn small ghost'));
  card.append(head);
  const explanation = document.createElement('p');
  explanation.textContent = t('items.suggestions.advisory');
  card.append(explanation);
  if (state.specificationError) {
    const error = document.createElement('p');
    error.className = 'status-message status-error';
    error.textContent = state.specificationError;
    card.append(error);
  }
  if (set.split_recommendation) {
    const split = document.createElement('p');
    split.className = 'status-message status-warning';
    split.textContent = set.split_recommendation.reason;
    card.append(split);
  }
  const approvedIds = new Set(set.approved_ids || []);
  (set.suggestions || []).forEach(candidate => {
    const row = document.createElement('div');
    row.className = 'specification-row target-suggestion';
    const approved = approvedIds.has(candidate.id);
    if (approved) row.setAttribute('data-target-suggestion-approved', '');
    const selection = document.createElement('input');
    selection.type = 'checkbox';
    selection.checked = !approved && state.targetSuggestionSelection.includes(candidate.id);
    selection.disabled = approved;
    selection.setAttribute('aria-label', `${t('items.suggestions.select')} ${candidate.ref}`);
    selection.addEventListener('change', () => {
      const values = new Set(state.targetSuggestionSelection);
      if (selection.checked) values.add(candidate.id); else values.delete(candidate.id);
      state.targetSuggestionSelection = [...values];
      state.render();
    });
    const detail = document.createElement('div');
    const title = document.createElement('strong');
    title.textContent = `#${candidate.rank} ${candidate.ref}`;
    const meta = document.createElement('div');
    meta.className = 'meta-line';
    meta.append(enumChip('suggestion.confidence', candidate.confidence), enumChip('target.role', candidate.role));
    if (approved) meta.append(status(t('items.suggestions.approved')));
    const evidence = document.createElement('ul');
    evidence.className = 'compact-list';
    (candidate.evidence || []).forEach(value => {
      const item = document.createElement('li');
      item.textContent = value;
      evidence.append(item);
    });
    detail.append(title, meta, evidence);
    const reject = button(t('items.suggestions.reject'), '×', () => runBusy(state, async () => {
        state.targetSuggestions = await state.api.rejectTargetSuggestion(
          state.projection,
          set.criterion,
          set.suggestion_token,
          candidate.id,
        );
        state.targetSuggestionSelection = state.targetSuggestionSelection.filter(id => id !== candidate.id);
        state.specificationError = null;
    }), 'btn small ghost');
    row.append(selection, detail, reject);
    card.append(row);
  });
  if (!(set.suggestions || []).length) {
    const empty = document.createElement('p');
    empty.className = 'empty-state';
    empty.textContent = t('items.suggestions.empty');
    card.append(empty);
  } else {
    const approve = button(t('items.suggestions.approve'), '✓', () => runBusy(state, async () => {
      const result = await state.api.approveTargetSuggestions(
        state.projection,
        set.criterion,
        set.suggestion_token,
        state.targetSuggestionSelection,
      );
      const approved = new Set(result.approved_ids || []);
      const approvedIds = new Set(set.approved_ids || []);
      approved.forEach(id => approvedIds.add(id));
      state.targetSuggestions = {
        ...set,
        approved_ids: [...approvedIds],
        split_recommendation: result.split_recommendation,
      };
      state.targetSuggestionSelection = [];
      state.specificationError = null;
    }), 'btn primary');
    approve.setAttribute('data-approve-target-suggestions', '');
    approve.disabled = !state.targetSuggestionSelection.length;
    card.append(approve);
  }
  root.append(card);
}

function renderEditor(root, state) {
  const editor = state.specificationEditor;
  const form = document.createElement('form');
  form.className = 'specification-editor card';
  const heading = document.createElement('div');
  heading.className = 'canvas-head';
  const title = document.createElement('h2');
  title.textContent = editor.mode === 'create' ? t('items.new.title') : t('common.edit');
  heading.append(title);
  heading.append(button(t('common.close'), '×', () => { state.specificationEditor = null; state.specificationPreview = null; state.render(); }, 'btn small ghost'));
  form.append(heading);

  if (editor.mode === 'create') {
    const draft = editor.draft || {};
    const kindChooser = document.createElement('div');
    kindChooser.className = 'seg-select specification-kind-chooser';
    ['requirement', 'feature'].forEach(kind => {
      const choice = button(t(`items.${kind}`), kind === editor.createKind ? '●' : '○', () => {
        editor.createKind = kind;
        state.specificationPreview = null;
        state.render();
      }, `scope-mode-button${kind === editor.createKind ? ' active' : ''}`);
      kindChooser.append(choice);
    });
    form.append(kindChooser);
    const documents = (state.projection.specifications?.documents || [])
      .filter(document => document.kind === editor.createKind)
      .map(document => document.path)
      .filter((path, index, values) => values.indexOf(path) === index);
    form.append(field(t('items.field.document'), draftValue(editor, draft, 'document', documents[0] || ''), 'document', 'select', documents));
    form.append(field(t('items.field.id'), draftValue(editor, draft, 'id'), 'id'));
    form.append(field(t('items.field.title'), draftValue(editor, draft, 'title'), 'title'));
    if (editor.createKind === 'feature') {
      form.append(field(t('items.field.summary'), draftValue(editor, draft, 'summary'), 'summary', 'textarea'));
    } else {
      form.append(field(t('items.field.description'), draftValue(editor, draft, 'description'), 'description', 'textarea'));
      form.append(field(t('items.field.priority'), draftValue(editor, draft, 'priority', 'medium'), 'priority', 'select', localizedOptions('items.priority', ['low', 'medium', 'high', 'critical'])));
      form.append(field(t('items.field.criterion_id'), draftValue(editor, draft, 'criterion_id'), 'criterion_id'));
      form.append(field(t('items.field.criterion_kind'), draftValue(editor, draft, 'criterion_kind', 'behavior'), 'criterion_kind', 'select', localizedOptions('criterion.kind', ['behavior', 'quality', 'security', 'operational', 'documentation', 'compatibility', 'custom'])));
      form.append(field(t('items.field.criterion_statement'), draftValue(editor, draft, 'criterion_statement'), 'criterion_statement', 'textarea'));
      const governance = document.createElement('details');
      const governanceSummary = document.createElement('summary');
      governanceSummary.textContent = t('common.advanced');
      governance.append(governanceSummary);
      const governanceText = document.createElement('p');
      governanceText.textContent = `${t('items.field.governed_by')}: ${editor.governedBy?.join(', ') || '—'}`;
      governance.append(governanceText);
      form.append(governance);
    }
  } else if (editor.anchor) {
    const draft = editor.draft || {};
    form.append(field(t('items.field.statement'), draftValue(editor, draft, 'statement', editor.statement), 'statement', 'textarea'));
    if (editor.anchorKind === 'criterion') {
      form.append(field(t('items.field.kind'), draftValue(editor, draft, 'criterion_kind', editor.kind), 'criterion_kind', 'select', localizedOptions('criterion.kind', ['behavior', 'quality', 'security', 'operational', 'documentation', 'compatibility', 'custom'])));
    }
    if (editor.anchorKind === 'principle') form.append(field(t('items.field.applies_to'), draftValue(editor, draft, 'applies_to', editor.appliesTo), 'applies_to'));
    const advanced = document.createElement('details');
    const summary = document.createElement('summary');
    summary.textContent = t('common.advanced');
    advanced.append(summary);
    const note = document.createElement('p');
    note.textContent = t('items.advanced.preserved');
    advanced.append(note);
    form.append(advanced);
  } else {
    const item = itemById(state, editor.itemId);
    const draft = editor.draft || {};
    const kind = item?.kind || 'requirement';
    form.append(field(t('items.field.title'), draftValue(editor, draft, 'title', item?.title), 'title'));
    if (kind === 'philosophy' || kind === 'policy') form.append(field(t('items.field.summary'), draftValue(editor, draft, 'summary', item?.summary), 'summary', 'textarea'));
    if (kind === 'requirement') {
      form.append(field(t('items.field.description'), draftValue(editor, draft, 'description', item?.description || item?.summary), 'description', 'textarea'));
      form.append(field(t('items.field.priority'), draftValue(editor, draft, 'priority', item?.priority || 'medium'), 'priority', 'select', localizedOptions('items.priority', ['low', 'medium', 'high', 'critical'])));
      form.append(field(t('items.field.status'), draftValue(editor, draft, 'status', item?.status || 'planned'), 'status', 'select', localizedOptions('status', ['planned', 'implemented', 'deprecated'])));
    }
    if (kind === 'feature') {
      form.append(field(t('items.field.summary'), draftValue(editor, draft, 'summary', item?.summary), 'summary', 'textarea'));
      form.append(field(t('items.field.status'), draftValue(editor, draft, 'status', item?.status || 'planned'), 'status', 'select', localizedOptions('status', ['planned', 'implemented', 'deprecated'])));
    }
    const advanced = document.createElement('details');
    const summary = document.createElement('summary');
    summary.textContent = t('common.advanced');
    advanced.append(summary);
    const note = document.createElement('p');
    note.textContent = t('items.advanced.preserved');
    advanced.append(note);
    form.append(advanced);
  }

  const actions = document.createElement('div');
  actions.className = 'actions';
  const previewButton = button(t('common.preview'), '⌕', () => form.requestSubmit(), 'btn primary');
  actions.append(previewButton);
  form.append(actions);
  form.addEventListener('input', () => {
    editor.draft = Object.fromEntries(new FormData(form).entries());
    if (state.specificationPreview) {
      state.specificationPreview = null;
      form.querySelector('.specification-apply')?.remove();
      form.querySelector('.specification-impact')?.remove();
    }
  });
  form.addEventListener('change', () => {
    editor.draft = Object.fromEntries(new FormData(form).entries());
    if (state.specificationPreview) {
      state.specificationPreview = null;
      form.querySelector('.specification-apply')?.remove();
      form.querySelector('.specification-impact')?.remove();
    }
  });
  form.addEventListener('submit', event => {
    event.preventDefault();
    const patch = patchFromForm(editor, state, form);
    state.specificationError = null;
    state.specificationPreview = null;
    runBusy(state, async () => {
      const result = await state.api.previewSpecificationCandidate(state.projection, patch);
      state.specificationPreview = { patch, result };
    });
  });
  if (state.specificationError) {
    const error = document.createElement('p');
    error.className = 'status-message status-error';
    error.textContent = state.specificationError;
    form.append(error);
  }
  if (state.specificationPreview) {
    renderImpact(form, state.specificationPreview.result);
    const apply = button(t('common.apply'), '✓', () => runBusy(state, async () => {
      state.specificationError = null;
        const currentPatch = patchFromForm(editor, state, form);
        if (JSON.stringify(currentPatch) !== JSON.stringify(state.specificationPreview.patch)) {
          throw new Error(t('items.preview.stale'));
        }
        await state.api.applySpecificationCandidate(
          state.projection,
          state.specificationPreview.patch,
          state.specificationPreview.result.preview_token,
        );
        state.projection = await state.api.readProjection();
        state.specificationCandidates = null;
        state.specificationTrace = null;
        state.specificationTraceRoot = null;
        state.specificationEditor = null;
        state.specificationPreview = null;
        state.selectedSpecification = null;
    }), 'btn primary specification-apply');
    actions.append(apply);
  }
  root.append(form);
}

function relatedTargets(state, selected) {
  ensureSpecificationTrace(state, selected);
  const trace = state.specificationTrace;
  if (!trace || trace.error || trace.root_item_id !== selected.id) return [];
  const criteria = new Set((selected.criteria || []).map(value => value.anchor));
  const items = state.projection?.specifications?.specifications || [];
  const related = [];
  const seen = new Set();
  trace.edges
    .filter(edge => ['satisfies', 'verifies', 'covers'].includes(edge.relation))
    .forEach(edge => {
      const targetReference = edge.relation === 'covers'
        ? edge.to
        : edge.from.includes('#claim.')
        ? edge.from.slice(0, edge.from.indexOf('#claim.'))
        : edge.from;
      const target = specificationTarget(state, targetReference);
      if (!target) return;
      const item = items.find(candidate => candidate.id === target.itemId);
      if (!item) return;
      if (item.id !== selected.id && !criteria.has(edge.to) && edge.relation !== 'covers') return;
      const kind = edge.relation === 'verifies' ? 'verifies' : 'satisfies';
      const key = `${kind}:${targetReference}`;
      if (seen.has(key)) return;
      seen.add(key);
      const binding = { anchor: target.bindingAnchor };
      related.push({
        item,
        binding,
        target,
        claim: { kind, criterion: edge.relation === 'covers' ? [...criteria][0] : edge.to },
      });
    });
  return related;
}

function renderRelated(root, state, selected, onItem, onTarget) {
  const entries = relatedTargets(state, selected);
  const selectedCriteria = new Set((selected.criteria || []).map(value => value.anchor));
  allTargets(state.projection).filter(entry => entry.item.id === selected.id).forEach(entry => {
    (entry.target.claims || []).forEach(claim => { if (claim.criterion) selectedCriteria.add(claim.criterion); });
  });
  if (!entries.length) return;
  const related = {
    specification: [],
    implementation: [],
    verification: [],
  };
  const seenItems = new Set();
  const seenTargets = new Set();
  entries.forEach(({ item, binding, target, claim }) => {
    if (item.id !== selected.id && !seenItems.has(item.id)) {
      seenItems.add(item.id);
      related.specification.push({ item });
    }
    if (item.id !== selected.id && !selectedCriteria.has(claim.criterion)) return;
    const kind = claim.kind === 'verifies' ? 'verification' : 'implementation';
    const reference = target.reference || `${binding.anchor}/target.${target.id}`;
    if (seenTargets.has(`${kind}:${reference}`)) return;
    seenTargets.add(`${kind}:${reference}`);
    related[kind].push({ item, target: { ...target, reference } });
  });
  const availableKinds = Object.keys(related).filter(kind => related[kind].length);
  if (!availableKinds.length) return;
  if (!availableKinds.includes(state.relatedKind)) state.relatedKind = availableKinds[0];

  const section = document.createElement('section');
  section.className = 'specification-related';
  section.append(Object.assign(document.createElement('h3'), { textContent: t('items.related') }));
  const chooser = document.createElement('label');
  chooser.className = 'related-chooser';
  const chooserLabel = document.createElement('span');
  chooserLabel.textContent = t('items.related.choose');
  const select = document.createElement('select');
  select.className = 'native-select';
  availableKinds.forEach(kind => {
    const option = document.createElement('option');
    const icon = kind === 'specification' ? '◆' : kind === 'verification' ? '✓' : '⌘';
    option.value = kind;
    option.textContent = `${icon} ${t(`items.related.${kind}`)} · ${related[kind].length}`;
    option.selected = kind === state.relatedKind;
    select.append(option);
  });
  select.addEventListener('change', () => {
    state.relatedKind = select.value;
    state.render();
  });
  chooser.append(chooserLabel, select);
  section.append(chooser);

  const list = document.createElement('div');
  list.className = 'related-list';
  related[state.relatedKind].forEach(entry => {
    if (state.relatedKind === 'specification') {
      list.append(button(
        localizeSpecificationTitle(entry.item),
        entry.item.kind === 'feature' ? '◆' : '◈',
        () => onItem(entry.item.id),
        'related-row specification',
      ));
      return;
    }
    const target = entry.target;
    const targetName = target.selector?.name || target.path || target.reference;
    const label = target.path && targetName !== target.path
      ? `${targetName} · ${target.path}`
      : targetName;
    list.append(button(
      label,
      state.relatedKind === 'verification' ? '✓' : '⌘',
      () => onTarget(target),
      `related-row ${state.relatedKind}`,
    ));
  });
  section.append(list);
  root.append(section);
}

function specificationTarget(state, reference) {
  return (state.projection?.specifications?.specifications || [])
    .flatMap(item => (item.bindings || []).flatMap(binding => (binding.targets || []).map(target => ({
      ...target,
      itemId: item.id,
      bindingAnchor: binding.anchor,
    }))))
    .find(target => target.reference === reference);
}

function ensureSpecificationTrace(state, selected) {
  const signature = `${selected.id}:${state.specificationTraceMode}:${state.specificationTraceDepth}`;
  if (state.specificationTraceRoot === signature || state.specificationTraceLoading) return;
  state.specificationTraceLoading = true;
  state.api.readSpecificationTrace(selected.id, {
    mode: state.specificationTraceMode,
    depth: state.specificationTraceDepth,
    nodeBudget: 80,
  }).then(trace => {
    state.specificationTrace = trace;
    state.specificationTraceRoot = signature;
    state.specificationTraceLoading = false;
    state.render();
  }).catch(error => {
    state.specificationTrace = { error: error.message };
    state.specificationTraceRoot = signature;
    state.specificationTraceLoading = false;
    state.render();
  });
}

function detailLabel(label, value) {
  const row = document.createElement('div');
  row.className = 'spec-detail-field';
  const caption = document.createElement('dt');
  caption.textContent = label;
  const content = document.createElement('dd');
  content.textContent = value === undefined || value === null || value === '' ? '—' : String(value);
  row.append(caption, content);
  return row;
}

function detailList(label, values) {
  const row = document.createElement('div');
  row.className = 'spec-detail-field';
  const caption = document.createElement('dt');
  caption.textContent = label;
  const content = document.createElement('dd');
  const list = values || [];
  if (!list.length) {
    content.textContent = t('items.detail.empty');
  } else {
    const valueList = document.createElement('ul');
    valueList.className = 'spec-detail-values';
    list.forEach(value => {
      const item = document.createElement('li');
      item.textContent = value;
      valueList.append(item);
    });
    content.append(valueList);
  }
  row.append(caption, content);
  return row;
}

function detailCard(title, fields, className = '') {
  const card = document.createElement('section');
  card.className = `card specification-detail-card${className ? ` ${className}` : ''}`;
  const heading = document.createElement('h3');
  heading.textContent = title;
  card.append(heading);
  const definition = document.createElement('dl');
  definition.className = 'spec-detail-fields';
  fields.forEach(fieldValue => definition.append(fieldValue));
  card.append(definition);
  return card;
}

function claimLabel(claim) {
  if (claim.kind === 'verifies') return `verifies ${claim.criterion || ''}`;
  if (claim.kind === 'satisfies') return `satisfies ${claim.criterion || ''}`;
  if (claim.kind === 'covers') return `covers ${(claim.targets || []).join(', ')}`;
  return claim.kind || 'claim';
}

function renderInformation(root, state, selected, onItem, onTarget) {
  const counts = selected.bindings || [];
  const targets = counts.flatMap(binding => binding.targets || []);
  const verificationTargets = targets.filter(target => target.claims?.some(claim => claim.kind === 'verifies'));
  root.append(detailCard(t('items.detail.basic'), [
    detailLabel(t('items.detail.kind'), selected.kind),
    detailLabel(t('items.detail.status'), selected.status),
    detailLabel(t('items.detail.priority'), selected.priority),
    detailLabel(t('items.detail.source'), selected.path),
    detailLabel(t('items.detail.source_hash'), selected.source_hash),
    detailLabel(t('items.detail.count.criteria'), selected.criteria?.length || 0),
    detailLabel(t('items.detail.count.bindings'), selected.bindings?.length || 0),
    detailLabel(t('items.detail.count.targets'), targets.length),
    detailLabel(t('items.detail.count.verification'), verificationTargets.length),
  ], 'specification-detail-basic'));

  const anchors = [
    ['principles', selected.principles || [], 'principle'],
    ['rules', selected.rules || [], 'rule'],
    ['criteria', selected.criteria || [], 'criterion'],
  ];
  anchors.forEach(([label, values, kind]) => {
    const card = document.createElement('section');
    card.className = 'card specification-detail-card';
    const heading = document.createElement('h3');
    heading.textContent = t(`items.${label}`);
    card.append(heading);
    if (!values.length) {
      const empty = document.createElement('p');
      empty.className = 'spec-detail-empty';
      empty.textContent = t('items.detail.empty');
      card.append(empty);
    }
    values.forEach(value => {
      const row = document.createElement('article');
      row.className = 'specification-detail-anchor';
      const head = document.createElement('div');
      head.className = 'spec-detail-anchor-head';
      const exact = document.createElement('code');
      exact.textContent = value.anchor;
      const badge = document.createElement('span');
      badge.className = 'chip';
      badge.textContent = value.kind || value.level || kind;
      head.append(exact, badge);
      row.append(head);
      const statement = document.createElement('p');
      statement.textContent = value.statement;
      row.append(statement);
      const fields = document.createElement('dl');
      fields.className = 'spec-detail-fields compact';
      if (kind === 'principle') fields.append(detailList('applies_to', value.applies_to));
      if (kind === 'rule') {
        fields.append(detailLabel('level', value.level));
        fields.append(detailList('governed_by', value.governed_by));
        fields.append(detailList('applies_to_roles', value.applies_to_roles));
        fields.append(detailLabel('enforcement', value.enforcement));
      }
      if (kind === 'criterion') {
        fields.append(detailLabel('kind', value.kind));
        fields.append(detailList('governed_by', value.governed_by));
      }
      row.append(fields);
      card.append(row);
    });
    root.append(card);
  });

  const bindingCard = document.createElement('section');
  bindingCard.className = 'card specification-detail-card';
  const bindingHeading = document.createElement('h3');
  bindingHeading.textContent = t('items.detail.bindings');
  bindingCard.append(bindingHeading);
  if (!selected.bindings?.length) {
    const empty = document.createElement('p');
    empty.className = 'spec-detail-empty';
    empty.textContent = t('items.detail.empty');
    bindingCard.append(empty);
  }
  (selected.bindings || []).forEach(binding => {
    const bindingSection = document.createElement('article');
    bindingSection.className = 'specification-detail-binding';
    const bindingTitle = document.createElement('code');
    bindingTitle.textContent = binding.anchor;
    bindingSection.append(bindingTitle, detailCard('', [
      detailLabel(t('items.detail.role'), binding.role),
      detailLabel(t('items.detail.facet'), binding.facet),
      detailLabel(t('items.detail.responsibility'), binding.responsibility),
      detailList(t('items.detail.owns'), (binding.owns || []).map(scope => `${scope.id} · ${scope.path}`)),
    ], 'specification-detail-inline-card'));
    const targetList = document.createElement('div');
    targetList.className = 'specification-detail-targets';
    if (!binding.targets?.length) {
      const empty = document.createElement('p');
      empty.className = 'spec-detail-empty';
      empty.textContent = t('items.detail.empty');
      targetList.append(empty);
    }
    (binding.targets || []).forEach(target => {
      const targetCard = document.createElement('article');
      targetCard.className = 'specification-detail-target';
      const targetHead = document.createElement('div');
      targetHead.className = 'spec-detail-anchor-head';
      const targetRef = document.createElement('code');
      targetRef.textContent = target.reference;
      const open = button(t('items.detail.open_source'), '⌘', () => onTarget(target), 'btn small ghost');
      targetHead.append(targetRef, open);
      targetCard.append(targetHead);
      const targetFields = document.createElement('dl');
      targetFields.className = 'spec-detail-fields compact';
      targetFields.append(
        detailLabel(t('items.detail.adapter'), target.adapter),
        detailLabel(t('items.detail.path'), target.path),
        detailLabel(t('items.detail.selector'), JSON.stringify(target.selector)),
        detailList(t('items.detail.claims'), (target.claims || []).map(claimLabel)),
      );
      targetCard.append(targetFields);
      targetList.append(targetCard);
    });
    bindingSection.append(targetList);
    bindingCard.append(bindingSection);
  });
  root.append(bindingCard);

  const contracts = document.createElement('section');
  contracts.className = 'card specification-detail-card';
  const contractsHeading = document.createElement('h3');
  contractsHeading.textContent = t('items.detail.contracts');
  contracts.append(contractsHeading);
  if (!selected.contracts?.length) {
    const empty = document.createElement('p');
    empty.className = 'spec-detail-empty';
    empty.textContent = t('items.detail.empty');
    contracts.append(empty);
  }
  (selected.contracts || []).forEach(contract => {
    const card = document.createElement('article');
    card.className = 'specification-detail-contract';
    card.append(Object.assign(document.createElement('code'), { textContent: contract.anchor }));
    const fields = document.createElement('dl');
    fields.className = 'spec-detail-fields compact';
    fields.append(
      detailLabel('kind', contract.kind),
      detailLabel('source', contract.source),
      detailList('participants', (contract.participants || []).map(participant => `${participant.role} · ${participant.binding}`)),
      detailList('guarantees', contract.guarantees),
    );
    card.append(fields);
    contracts.append(card);
  });
  root.append(contracts);
}

function traceNodeButton(state, node, onSelect) {
  const buttonNode = document.createElement('button');
  buttonNode.type = 'button';
  buttonNode.className = `trace-node trace-node-${node.kind}${state.specificationTraceNode === node.id ? ' is-selected' : ''}`;
  buttonNode.dataset.traceNode = node.id;
  buttonNode.setAttribute('aria-label', `${t('items.detail.inspect_node')}: ${node.label}`);
  const kind = document.createElement('span');
  kind.className = 'trace-node-kind';
  kind.textContent = node.kind;
  const label = document.createElement('strong');
  label.textContent = node.label;
  buttonNode.append(kind, label);
  if (node.secondary_label) buttonNode.append(Object.assign(document.createElement('small'), { textContent: node.secondary_label }));
  buttonNode.addEventListener('click', () => onSelect(node));
  return buttonNode;
}

function renderTrace(root, state, selected, onSelect) {
  ensureSpecificationTrace(state, selected);
  const trace = state.specificationTrace;
  if (state.specificationTraceLoading || !trace || state.specificationTraceRoot !== `${selected.id}:${state.specificationTraceMode}:${state.specificationTraceDepth}`) {
    const loading = document.createElement('p');
    loading.className = 'empty-state';
    loading.textContent = t('items.detail.trace_loading');
    root.append(loading);
    return;
  }
  if (trace.error) {
    const error = document.createElement('p');
    error.className = 'status-message status-error';
    error.textContent = trace.error;
    root.append(error);
    return;
  }
  const controls = document.createElement('div');
  controls.className = 'trace-controls';
  [['readable', t('items.detail.readable')], ['exact', t('items.detail.exact')]].forEach(([mode, label]) => {
    const modeButton = button(label, mode === state.specificationTraceMode ? '●' : '○', () => {
      state.specificationTraceMode = mode;
      state.specificationTraceRoot = null;
      syncSpecificationLocation(state);
      state.render();
    }, `btn small ${mode === state.specificationTraceMode ? 'primary' : 'ghost'}`);
    controls.append(modeButton);
  });
  const depth = document.createElement('label');
  depth.className = 'trace-depth';
  depth.append(Object.assign(document.createElement('span'), { textContent: t('items.detail.depth') }));
  const select = document.createElement('select');
  select.className = 'native-select';
  [1, 2, 3].forEach(value => {
    const option = document.createElement('option');
    option.value = value;
    option.textContent = String(value);
    option.selected = value === state.specificationTraceDepth;
    select.append(option);
  });
  select.addEventListener('change', () => {
    state.specificationTraceDepth = Number(select.value);
    state.specificationTraceRoot = null;
    syncSpecificationLocation(state);
    state.render();
  });
  depth.append(select);
  controls.append(depth);
  root.append(controls);

  const lanes = ['governance', 'specification', 'implementation', 'evidence'];
  const graph = document.createElement('div');
  graph.className = 'trace-lanes';
  lanes.forEach(lane => {
    const column = document.createElement('section');
    column.className = `trace-lane trace-lane-${lane}`;
    const heading = document.createElement('h3');
    heading.textContent = lane;
    column.append(heading);
    const laneNodes = trace.nodes
      .filter(node => node.lane === lane)
      .sort((left, right) => left.stable_order - right.stable_order);
    laneNodes.forEach(node => {
      column.append(traceNodeButton(state, node, onSelect));
    });
    if (!laneNodes.length) column.append(Object.assign(document.createElement('p'), { className: 'spec-detail-empty', textContent: t('items.detail.empty') }));
    graph.append(column);
  });
  root.append(graph);
  if (trace.truncated) {
    const notice = document.createElement('p');
    notice.className = 'notice warn';
    notice.textContent = t('items.detail.hidden').replace('{count}', String(trace.hidden_node_count));
    root.append(notice);
  }
  const edges = document.createElement('section');
  edges.className = 'card trace-edge-card';
  edges.append(Object.assign(document.createElement('h3'), { textContent: t('items.detail.edge') }));
  const list = document.createElement('ol');
  list.className = 'trace-edge-list';
  trace.edges.forEach(edge => {
    const item = document.createElement('li');
    item.textContent = `${edge.from} → ${edge.display_label} → ${edge.to}`;
    list.append(item);
  });
  if (!trace.edges.length) list.append(Object.assign(document.createElement('li'), { textContent: t('items.detail.trace_empty') }));
  edges.append(list);
  root.append(edges);
}

function closureLabel(state) {
  return t(`items.detail.${state}`) || state;
}

function renderEvidence(root, state, selected) {
  ensureSpecificationTrace(state, selected);
  const trace = state.specificationTrace;
  if (state.specificationTraceLoading || !trace || state.specificationTraceRoot !== `${selected.id}:${state.specificationTraceMode}:${state.specificationTraceDepth}`) {
    root.append(Object.assign(document.createElement('p'), { className: 'empty-state', textContent: t('items.detail.trace_loading') }));
    return;
  }
  if (trace.error) {
    root.append(Object.assign(document.createElement('p'), { className: 'status-message status-error', textContent: trace.error }));
    return;
  }
  (trace.closures || []).forEach(closure => {
    const card = document.createElement('article');
    card.className = `card closure-card closure-${closure.state}`;
    const heading = document.createElement('h3');
    heading.textContent = closure.criterion;
    const stateChip = document.createElement('span');
    stateChip.className = 'chip';
    stateChip.textContent = closureLabel(closure.state);
    heading.append(' ', stateChip);
    card.append(heading);
    card.append(detailList(t('items.detail.target_definition'), closure.implementation_targets));
    card.append(detailList(t('items.detail.latest_run'), closure.verification_targets));
    const note = document.createElement('p');
    note.className = 'spec-detail-evidence-note';
    note.textContent = t('items.detail.runtime_unavailable');
    card.append(note);
    if (closure.reasons?.length) card.append(detailList(t('items.detail.reasons'), closure.reasons));
    root.append(card);
  });
  if (!(trace.closures || []).length) root.append(Object.assign(document.createElement('p'), { className: 'empty-state', textContent: t('items.detail.trace_empty') }));
}

function renderSourceInspector(root, state) {
  root.className = 'canvas specification-source-inspector';
  const target = state.specificationSourceTarget;
  if (!target) {
    root.append(Object.assign(document.createElement('p'), { className: 'spec-detail-empty', textContent: t('items.detail.open_source') }));
    return;
  }
  const head = document.createElement('div');
  head.className = 'source-inspector-head';
  head.append(Object.assign(document.createElement('h3'), { textContent: target.reference }));
  head.append(button(t('items.back'), '×', () => {
    state.specificationSourceTarget = null;
    state.specificationSource = null;
    state.specificationTraceNode = null;
    syncSpecificationLocation(state);
    state.render();
  }, 'btn small ghost'));
  root.append(head);
  const details = document.createElement('dl');
  details.className = 'spec-detail-fields compact';
  details.append(
    detailLabel(t('items.detail.adapter'), target.adapter),
    detailLabel(t('items.detail.path'), target.path),
    detailLabel(t('items.detail.selector'), JSON.stringify(target.selector)),
  );
  root.append(details);
  const full = Boolean(state.specificationSourceFull);
  root.append(button(full ? t('items.show_excerpt') : t('items.show_file'), full ? '↙' : '↗', () => {
    state.specificationSourceFull = !full;
    state.specificationSource = null;
    state.render();
  }, 'btn small ghost'));
  const key = `${target.reference}:${full ? 'file' : 'excerpt'}`;
  if (state.specificationSource?.key === key) {
    appendSource(root, state.specificationSource.value);
    return;
  }
  const loading = document.createElement('p');
  loading.className = 'empty-state';
  loading.textContent = t('common.loading');
  root.append(loading);
  const request = full ? state.api.readSource(target.path) : state.api.readTargetSource(target.reference);
  request.then(value => {
    if (state.specificationSourceTarget?.reference !== target.reference || Boolean(state.specificationSourceFull) !== full) return;
    state.specificationSource = { key, value };
    state.render();
  }).catch(() => {
    if (state.specificationSourceTarget?.reference !== target.reference) return;
    state.specificationSource = { key, value: null };
    state.render();
  });
}

function renderSpecificationWorkspace(root, state, selected, options) {
  if (!state.specificationSourceTarget && state.specificationTraceNode) {
    state.specificationSourceTarget = specificationTarget(state, state.specificationTraceNode) || null;
  }
  const shell = document.createElement('div');
  shell.className = `specification-detail-workspace${state.specificationSourceTarget ? ' has-inspector' : ''}`;
  const main = document.createElement('div');
  main.className = 'specification-detail-main';
  const breadcrumb = document.createElement('p');
  breadcrumb.className = 'specification-breadcrumb';
  breadcrumb.textContent = `${t('nav.specifications')} › ${selected.kind} › ${selected.id}`;
  main.append(breadcrumb);
  const head = document.createElement('div');
  head.className = 'canvas-head specification-detail-head';
  const title = document.createElement('div');
  title.append(Object.assign(document.createElement('h2'), { textContent: selected.title }));
  const meta = document.createElement('div');
  meta.className = 'meta-line';
  meta.append(status(selected.status || selected.kind));
  meta.append(Object.assign(document.createElement('span'), { className: 'chip', textContent: selected.id }));
  if (selected.priority) meta.append(Object.assign(document.createElement('span'), { className: 'chip', textContent: selected.priority }));
  title.append(meta);
  head.append(title);
  const actions = document.createElement('div');
  actions.className = 'actions';
  actions.append(button(t('items.detail.copy_ref'), '⧉', () => navigator.clipboard?.writeText(selected.id), 'btn small'));
  if (!options.readOnly) actions.append(button(t('common.edit'), '✎', () => {
    state.specificationEditor = { mode: 'edit', itemId: selected.id };
    state.specificationPreview = null;
    state.render();
  }, 'btn small primary'));
  head.append(actions);
  main.append(head);
  const description = document.createElement('p');
  description.className = 'specification-summary';
  description.textContent = selected.summary || selected.description || t('items.detail.empty');
  main.append(description);
  const tabs = document.createElement('div');
  tabs.className = 'specification-detail-tabs';
  tabs.setAttribute('role', 'tablist');
  SPECIFICATION_DETAIL_TABS.forEach(tabName => {
    const tab = document.createElement('button');
    tab.type = 'button';
    tab.className = `tab${state.specificationDetailTab === tabName ? ' active' : ''}`;
    tab.setAttribute('role', 'tab');
    tab.setAttribute('aria-selected', String(state.specificationDetailTab === tabName));
    tab.textContent = t(`items.detail.${tabName}`);
    tab.addEventListener('click', () => {
      state.specificationDetailTab = tabName;
      state.specificationTraceNode = null;
      state.specificationSourceTarget = null;
      syncSpecificationLocation(state);
      state.render();
    });
    tabs.append(tab);
  });
  main.append(tabs);
  const body = document.createElement('div');
  body.className = 'specification-detail-tab-body';
  const onItem = itemId => {
    state.selectedSpecification = itemId;
    state.specificationTrace = null;
    state.specificationTraceRoot = null;
    state.specificationSourceTarget = null;
    syncSpecificationLocation(state);
    state.render();
  };
  const onTarget = target => {
    state.specificationTraceNode = target.reference;
    state.specificationSourceTarget = target;
    syncSpecificationLocation(state);
    state.render();
  };
  if (state.specificationDetailTab === 'information') renderInformation(body, state, selected, onItem, onTarget);
  else if (state.specificationDetailTab === 'trace') {
    renderTrace(body, state, selected, node => {
      state.specificationTraceNode = node.id;
      state.specificationSourceTarget = node.source_target ? specificationTarget(state, node.source_target) : null;
      if (node.item_id && node.item_id !== selected.id && node.kind === 'item') state.selectedSpecification = node.item_id;
      syncSpecificationLocation(state);
      state.render();
    });
  } else renderEvidence(body, state, selected);
  main.append(body);
  shell.append(main);
  if (state.specificationSourceTarget) {
    const inspector = document.createElement('aside');
    inspector.className = 'canvas specification-detail-inspector';
    renderSourceInspector(inspector, state);
    shell.append(inspector);
  }
  root.append(shell);
}

export function renderSpecificationDetail(root, state, selected, options = {}) {
  if (options.detailWorkspace) {
    renderSpecificationWorkspace(root, state, selected, options);
    return;
  }
  const readOnly = Boolean(options.readOnly);
  const onItem = options.onItem || (itemId => {
    state.selectedSpecification = itemId;
    state.specificationSourceTarget = null;
    state.specificationSource = null;
    state.specificationSourceFull = false;
    state.render();
  });
  const onTarget = options.onTarget || (target => {
    state.specificationSourceTarget = target;
    state.specificationSource = null;
    state.specificationSourceFull = false;
    state.render();
  });
  const head = document.createElement('div');
  head.className = 'canvas-head';
  const title = document.createElement('div');
  const heading = document.createElement('h2');
  heading.textContent = localizeSpecificationTitle(selected);
  title.append(heading);
  const meta = document.createElement('div');
  meta.className = 'meta-line';
  meta.append(status(selected.status || selected.kind));
  const id = document.createElement('span');
  id.className = 'chip';
  id.textContent = selected.id;
  meta.append(id);
  title.append(meta);
  if (!options.hideHeading) head.append(title);
  if (!readOnly || options.action) {
    const actions = document.createElement('div');
    actions.className = 'actions';
    if (options.action) {
      actions.append(button(
        options.action.label,
        options.action.icon || '→',
        options.action.onClick,
        options.action.className || 'btn small primary',
      ));
    }
    if (!readOnly) {
      actions.append(button(t('common.edit'), '✎', () => {
        state.specificationEditor = { mode: 'edit', itemId: selected.id };
        state.specificationPreview = null;
        state.targetSuggestions = null;
        state.targetSuggestionSelection = [];
        state.render();
      }, 'btn small'));
    }
    head.append(actions);
  }
  if (head.childElementCount) root.append(head);
  if (selected.summary || selected.description) {
    const summary = document.createElement('p');
    summary.className = 'specification-summary';
    summary.textContent = selected.summary || selected.description;
    root.append(summary);
  }
  const groups = [
    ['principles', selected.principles || [], 'principle'],
    ['rules', selected.rules || [], 'rule'],
    ['criteria', selected.criteria || [], 'criterion'],
  ];
  groups.filter(([, values]) => values.length).forEach(([label, values, kind]) => {
    const card = document.createElement('section');
    card.className = 'card specification-group';
    const heading = document.createElement('h3');
    heading.textContent = t(`items.${label}`);
    card.append(heading);
    values.forEach(value => {
      const row = document.createElement('div');
      row.className = `specification-row${kind === 'criterion' ? ' specification-criterion' : ''}${value.anchor === options.highlightedAnchor ? ' is-highlighted' : ''}`;
      const text = document.createElement('div');
      const anchor = document.createElement('strong');
      anchor.textContent = value.anchor;
      const statement = document.createElement('p');
      statement.textContent = value.statement;
      text.append(anchor, statement);
      row.append(text);
      if (kind === 'criterion' && value.kind) row.append(enumChip('criterion.kind', value.kind));
      if (!readOnly && kind === 'criterion' && selected.status === 'implemented') {
        const createWork = button(t('items.create_work'), '→', () => state.runAction(
          () => state.api.runJourneyAction(state.projection, {
            action: 'create',
            anchor: value.anchor,
            summary: `${t('work.request.summary_from_anchor').replace('{anchor}', localizeSpecificationTitle(selected))}`,
          }),
          () => { state.selectedSlice = null; state.go('work'); },
        ), 'btn small');
        createWork.setAttribute('data-create-work', '');
        createWork.setAttribute('data-create-work-anchor', value.anchor);
        row.append(createWork);
      }
      if (!readOnly && kind === 'criterion') {
        const reviewSuggestions = button(t('items.suggestions.review'), '◎', () => runBusy(state, async () => {
            const suggestions = await state.api.readTargetSuggestions(value.anchor);
            state.targetSuggestions = suggestions;
            const approved = new Set(suggestions.approved_ids || []);
            state.targetSuggestionSelection = suggestions.suggestions
              .filter(candidate => !approved.has(candidate.id))
              .map(candidate => candidate.id);
            state.specificationError = null;
        }), 'btn small');
        reviewSuggestions.setAttribute('data-review-target-suggestions', '');
        row.append(reviewSuggestions);
      }
      if (!readOnly) row.append(button(t('common.edit'), '✎', () => {
        state.specificationEditor = {
          mode: 'edit',
          anchor: value.anchor,
          anchorKind: kind,
          statement: value.statement,
          kind: value.kind,
          appliesTo: (value.applies_to || []).join(', '),
        };
        state.specificationPreview = null;
        state.targetSuggestions = null;
        state.targetSuggestionSelection = [];
        state.render();
      }, 'btn small ghost'));
      card.append(row);
    });
    root.append(card);
  });
  renderRelated(root, state, selected, onItem, onTarget);
  if (!readOnly) renderTargetSuggestions(root, state);
  const advanced = document.createElement('details');
  const summary = document.createElement('summary');
  summary.textContent = t('common.advanced');
  advanced.append(summary);
  const path = document.createElement('p');
  path.className = 'path';
  path.textContent = selected.path;
  advanced.append(path);
  const anchors = document.createElement('p');
  anchors.textContent = `${t('items.bindings')}: ${(selected.anchors || []).join(', ') || '—'}`;
  advanced.append(anchors);
  if (!readOnly) root.append(advanced);
}

export function renderSourceDetail(root, state, target, onBack) {
  const head = document.createElement('div');
  head.className = 'canvas-head source-head';
  head.append(button(t('items.back'), '←', onBack, 'btn small ghost'));
  const title = document.createElement('div');
  title.append(Object.assign(document.createElement('h2'), { textContent: t('items.open_code') }));
  head.append(title);
  const full = Boolean(state.specificationSourceFull);
  head.append(button(full ? t('items.show_excerpt') : t('items.show_file'), full ? '↙' : '↗', () => {
    state.specificationSourceFull = !full;
    state.specificationSource = null;
    state.render();
  }, 'btn small ghost'));
  root.append(head);
  const key = `${target.reference}:${full ? 'file' : 'excerpt'}`;
  if (state.specificationSource?.key === key) {
    appendSource(root, state.specificationSource.value);
    return;
  }
  const loading = document.createElement('p');
  loading.className = 'empty-state';
  loading.textContent = t('common.loading');
  root.append(loading);
  const request = full ? state.api.readSource(target.path) : state.api.readTargetSource(target.reference);
  request.then(value => {
    if (state.specificationSourceTarget?.reference !== target.reference || Boolean(state.specificationSourceFull) !== full) return;
    state.specificationSource = { key, value };
    state.render();
  }).catch(() => {
    if (state.specificationSourceTarget?.reference !== target.reference) return;
    state.specificationSource = { key, value: null };
    state.render();
  });
}

function appendSource(root, source) {
  if (!source) {
    const error = document.createElement('p');
    error.className = 'status-message status-error';
    error.textContent = t('items.source_unavailable');
    root.append(error);
    return;
  }
  const meta = document.createElement('p');
  meta.className = 'path source-path';
  meta.textContent = `${source.path}:${source.line_start}${source.line_end !== source.line_start ? `-${source.line_end}` : ''}`;
  root.append(meta);
  const code = document.createElement('pre');
  code.className = 'source-code';
  code.textContent = source.content;
  root.append(code);
}

export function initSpecifications(state) {
  const input = document.querySelector('[data-specifications-search]');
  const newButton = document.querySelector('[data-specifications-new]');
  let timer;
  let requestSequence = 0;
  const load = async () => {
    const sequence = ++requestSequence;
    if (!state.specificationQuery.trim()) {
      state.specificationCandidates = null;
      state.specificationError = null;
      state.render();
      return;
    }
    try {
      const candidates = await state.api.searchSpecificationCandidates(state.specificationQuery, state.specificationKind);
      if (sequence !== requestSequence) return;
      state.specificationCandidates = candidates;
      state.specificationError = null;
    } catch (error) {
      if (sequence !== requestSequence) return;
      state.specificationError = error.message;
    }
    state.render();
  };
  input?.addEventListener('input', () => {
    state.specificationQuery = input.value;
    requestSequence += 1;
    clearTimeout(timer);
    timer = setTimeout(load, 180);
  });
  document.querySelectorAll('[data-tab-group="specifications"]').forEach(tab => tab.addEventListener('click', () => {
    state.specificationKind = tab.dataset.tab || 'all';
    requestSequence += 1;
    clearTimeout(timer);
    if (state.specificationQuery.trim()) timer = setTimeout(load, 0);
    else {
      state.specificationCandidates = null;
      state.specificationError = null;
      state.render();
    }
  }));
  newButton?.addEventListener('click', () => {
    state.specificationEditor = { mode: 'create', createKind: 'requirement', governedBy: [] };
    state.specificationPreview = null;
    state.targetSuggestions = null;
    state.targetSuggestionSelection = [];
    state.render();
  });
  document.addEventListener('syu:locale', () => state.render());
}

export function renderSpecifications(specifications, stateOrRoot = document.querySelector('[data-specifications-detail]')) {
  const state = stateOrRoot?.api ? stateOrRoot : null;
  const root = state ? document.querySelector('[data-specifications-detail]') : stateOrRoot;
  const rail = document.querySelector('[data-specifications-rail]');
  const candidates = state ? candidatesFor(state) : (specifications?.specifications || []).map(item => ({ item }));
  const entries = candidates.map(itemFromCandidate);
  if (rail) {
    rail.replaceChildren();
    candidates.forEach(candidate => {
      const item = itemFromCandidate(candidate);
      const buttonNode = document.createElement('button');
      buttonNode.type = 'button';
      buttonNode.className = `rail-item${item.id === state?.selectedSpecification ? ' active' : ''}`;
      const title = document.createElement('div');
      const id = document.createElement('b');
      id.textContent = item.id;
      const name = document.createElement('p');
      name.textContent = localizeSpecificationTitle(item);
      title.append(id, name);
      buttonNode.append(title);
      if (candidate.relevance?.length) {
        const reason = document.createElement('small');
        reason.textContent = candidate.relevance[0];
        buttonNode.append(reason);
      }
      buttonNode.addEventListener('click', () => {
        state.selectedSpecification = item.id;
        state.specificationDetailTab = 'information';
        state.specificationTrace = null;
        state.specificationTraceRoot = null;
        state.specificationEditor = null;
        state.specificationPreview = null;
        state.targetSuggestions = null;
        state.targetSuggestionSelection = [];
        syncSpecificationLocation(state);
        state.render();
      });
      rail.append(buttonNode);
    });
  }
  if (!root) return entries;
  root.replaceChildren();
  if (state?.specificationEditor) {
    renderEditor(root, state);
    return entries;
  }
  const selected = entries.find(item => item.id === state?.selectedSpecification)
    || entries.find(item => item.status === 'implemented' && item.criteria?.length)
    || entries[0];
  if (!selected) {
    const empty = document.createElement('p');
    empty.className = 'empty-state';
    empty.textContent = t('items.empty.description');
    root.append(empty);
    return entries;
  }
  if (state) {
    const changed = state.selectedSpecification !== selected.id;
    state.selectedSpecification = selected.id;
    if (changed && !new URL(location.href).searchParams.has('item')) syncSpecificationLocation(state, false);
  }
  renderSpecificationDetail(root, state, selected, { detailWorkspace: true });
  return entries;
}
