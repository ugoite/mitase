import { translate } from '../i18n.js';

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
  const translatedStatuses = new Set(['modified', 'added', 'deleted', 'renamed', 'untracked', 'planned', 'implemented', 'deprecated', 'ready', 'blocked', 'unknown']);
  const label = translatedStatuses.has(normalized) ? t(`status.${normalized}`) : text || normalized;
  node.innerHTML = '<span class="status-dot" aria-hidden="true"></span>';
  const copy = document.createElement('span');
  copy.textContent = label;
  node.append(copy);
  return node;
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
      item.value = option;
      item.textContent = option;
      item.selected = option === value;
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
  states.append(status(`${t('items.preview.before')}: ${impact.readiness_before.status}`));
  states.append(status(`${t('items.preview.after')}: ${impact.readiness_after.status}`));
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
  addEntries(t('items.suggestions.title'), suggested.map(candidate => `${candidate.ref} (${candidate.confidence})`));
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
  (set.suggestions || []).forEach(candidate => {
    const row = document.createElement('div');
    row.className = 'specification-row target-suggestion';
    const selection = document.createElement('input');
    selection.type = 'checkbox';
    selection.checked = state.targetSuggestionSelection.includes(candidate.id);
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
    meta.append(status(candidate.confidence), status(candidate.role));
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
        if (result.request) {
          state.projection = await state.api.readProjection();
          state.targetSuggestions = null;
          state.targetSuggestionSelection = [];
          state.selectedSlice = null;
          state.go('work');
          return;
        }
        state.targetSuggestions = { ...set, split_recommendation: result.split_recommendation };
        state.specificationError = null;
    }), 'btn primary');
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
      form.append(field(t('items.field.priority'), draftValue(editor, draft, 'priority', 'medium'), 'priority', 'select', ['low', 'medium', 'high', 'critical']));
      form.append(field(t('items.field.criterion_id'), draftValue(editor, draft, 'criterion_id'), 'criterion_id'));
      form.append(field(t('items.field.criterion_kind'), draftValue(editor, draft, 'criterion_kind', 'behavior'), 'criterion_kind', 'select', ['behavior', 'quality', 'security', 'operational', 'documentation', 'compatibility', 'custom']));
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
      form.append(field(t('items.field.kind'), draftValue(editor, draft, 'criterion_kind', editor.kind), 'criterion_kind', 'select', ['behavior', 'quality', 'security', 'operational', 'documentation', 'compatibility', 'custom']));
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
      form.append(field(t('items.field.priority'), draftValue(editor, draft, 'priority', item?.priority || 'medium'), 'priority', 'select', ['low', 'medium', 'high', 'critical']));
      form.append(field(t('items.field.status'), draftValue(editor, draft, 'status', item?.status || 'planned'), 'status', 'select', ['planned', 'implemented', 'deprecated']));
    }
    if (kind === 'feature') {
      form.append(field(t('items.field.summary'), draftValue(editor, draft, 'summary', item?.summary), 'summary', 'textarea'));
      form.append(field(t('items.field.status'), draftValue(editor, draft, 'status', item?.status || 'planned'), 'status', 'select', ['planned', 'implemented', 'deprecated']));
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
        state.specificationEditor = null;
        state.specificationPreview = null;
        state.selectedSpecification = null;
    }), 'btn primary specification-apply');
    actions.append(apply);
  }
  root.append(form);
}

function allTargets(projection) {
  return (projection?.specifications?.specifications || []).flatMap(item =>
    (item.bindings || []).flatMap(binding => (binding.targets || []).map(target => ({ item, binding, target }))),
  );
}

function relatedTargets(state, selected) {
  const criteria = new Set((selected.criteria || []).map(value => value.anchor));
  allTargets(state.projection).forEach(entry => {
    if (entry.item.id !== selected.id) return;
    (entry.target.claims || []).forEach(claim => {
      if (claim.criterion) criteria.add(claim.criterion);
    });
  });
  const related = [];
  allTargets(state.projection).forEach(entry => {
    (entry.target.claims || []).forEach(claim => {
      const isOwnCriterion = criteria.has(claim.criterion);
      const isOwnTarget = entry.item.id === selected.id;
      if (isOwnCriterion || isOwnTarget) related.push({ ...entry, claim });
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
        entry.item.title,
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

export function renderSpecificationDetail(root, state, selected, options = {}) {
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
  heading.textContent = selected.title;
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
      if (!readOnly && kind === 'criterion' && selected.status === 'implemented') {
        row.append(button(t('items.create_work'), '→', () => state.runAction(
          () => state.api.runJourneyAction(state.projection, {
            action: 'create',
            anchor: value.anchor,
            summary: `Change ${selected.title}`,
          }),
          () => { state.selectedSlice = null; state.go('work'); },
        ), 'btn small'));
      }
      if (!readOnly && kind === 'criterion') {
        row.append(button(t('items.suggestions.review'), '◎', () => runBusy(state, async () => {
            const suggestions = await state.api.readTargetSuggestions(value.anchor);
            state.targetSuggestions = suggestions;
            state.targetSuggestionSelection = suggestions.suggestions.map(candidate => candidate.id);
            state.specificationError = null;
        }), 'btn small'));
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
      name.textContent = item.title;
      title.append(id, name);
      buttonNode.append(title);
      if (candidate.relevance?.length) {
        const reason = document.createElement('small');
        reason.textContent = candidate.relevance[0];
        buttonNode.append(reason);
      }
      buttonNode.addEventListener('click', () => {
        state.selectedSpecification = item.id;
        state.specificationEditor = null;
        state.specificationPreview = null;
        state.targetSuggestions = null;
        state.targetSuggestionSelection = [];
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
  if (state) state.selectedSpecification = selected.id;
  if (state?.specificationSourceTarget) {
    renderSourceDetail(root, state, state.specificationSourceTarget, () => {
      state.specificationSourceTarget = null;
      state.specificationSource = null;
      state.render();
    });
  } else {
    renderSpecificationDetail(root, state, selected);
  }
  return entries;
}
