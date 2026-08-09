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

function localIdFromAnchor(anchor) {
  return String(anchor || '').split('.').pop();
}

function targetPatchModel(target) {
  if (!['present', 'absent'].includes(target.lifecycle)) {
    throw new Error('Target lifecycle is required and must be present or absent.');
  }
  return {
    id: localIdFromAnchor(String(target.reference || '').split('/target.').pop()),
    adapter: target.adapter,
    path: target.path,
    selector: target.selector,
    lifecycle: target.lifecycle,
    claims: target.claims || [],
  };
}

function ownershipPatchModel(ownership) {
  return {
    id: ownership.id,
    adapter: ownership.adapter,
    path: ownership.path,
    selector: ownership.selector,
    supports: ownership.supports || [],
  };
}

function claimPatchModel(claim) {
  return JSON.parse(JSON.stringify(claim || { kind: 'satisfies', criterion: '' }));
}

function contractPatchModel(contract) {
  return {
    id: localIdFromAnchor(contract.anchor),
    kind: contract.kind,
    source: contract.source,
    participants: (contract.participants || []).map(participant => ({
      target: participant.binding,
      role: participant.role,
    })),
    guarantees: contract.guarantees || [],
  };
}

function bindingPatchModel(binding) {
  return {
    id: localIdFromAnchor(binding.anchor),
    role: String(binding.role || '').replaceAll('_', '-'),
    facet: binding.facet,
    responsibility: binding.responsibility,
    owns: binding.owns || [],
    targets: (binding.targets || []).map(targetPatchModel),
  };
}

function commaValues(value) {
  return String(value || '').split(',').map(entry => entry.trim()).filter(Boolean);
}

function lineValues(value) {
  return String(value || '').split('\n').map(entry => entry.trim()).filter(Boolean);
}

function selectorFromDraft(draft, ownership = false) {
  const kind = String(draft.selector_kind || (ownership ? 'file' : 'file'));
  if (kind === 'file') return { kind };
  if (ownership) {
    if (kind === 'module') return { kind, name: draft.selector_name || '' };
    if (kind === 'path-prefix') return { kind, value: draft.selector_value || '' };
    throw new Error(`Unsupported ownership selector: ${kind}`);
  }
  if (kind === 'symbol') return { kind, name: draft.selector_name || '' };
  if (kind === 'operation') return { kind, method: draft.selector_method || '', path: draft.selector_path || '' };
  if (['heading', 'json-pointer', 'marker'].includes(kind)) return { kind, value: draft.selector_value || '' };
  throw new Error(`Unsupported target selector: ${kind}`);
}

function selectorDraft(selector, ownership = false) {
  const value = selector || { kind: 'file' };
  return {
    selector_kind: value.kind || (ownership ? 'file' : 'file'),
    selector_name: value.name || '',
    selector_method: value.method || '',
    selector_path: value.path || '',
    selector_value: value.value || '',
  };
}

function claimFromDraft(draft) {
  const kind = String(draft.claim_kind || 'satisfies');
  if (kind === 'satisfies') return { kind, criterion: draft.criterion || '' };
  if (kind === 'verifies') {
    let argumentsValue;
    try {
      argumentsValue = JSON.parse(draft.runner_arguments || '{}');
    } catch (error) {
      throw new Error(`Runner arguments must be valid JSON: ${error.message}`);
    }
    if (!argumentsValue || Array.isArray(argumentsValue) || typeof argumentsValue !== 'object'
      || Object.entries(argumentsValue).some(([, value]) => typeof value !== 'string')) {
      throw new Error('Runner arguments must be an object whose values are strings.');
    }
    return {
      kind,
      criterion: draft.criterion || '',
      covers: commaValues(draft.covers).map(value => value),
      runner: { runner: draft.runner || '', arguments: argumentsValue },
    };
  }
  if (kind === 'documents' || kind === 'evidences') return { kind, anchor: draft.anchor || '' };
  if (kind === 'enforces') return { kind, rule: draft.rule || '' };
  if (kind === 'generated-from') return { kind, targets: commaValues(draft.targets) };
  return { kind: 'exposes', target: draft.target || '' };
}

function createNestedPatch(editor, form) {
  const draft = Object.fromEntries(new FormData(form).entries());
  editor.draft = draft;
  const operation = editor.operation || 'upsert';
  const edit = { entity: editor.entity, operation };
  const currentId = editor.currentId || null;
  if (editor.entity === 'binding') edit.binding = {
    ...editor.binding,
    id: draft.id || editor.binding?.id,
    role: draft.role,
    facet: draft.facet,
    responsibility: draft.responsibility,
    owns: editor.binding?.owns || [],
    targets: editor.binding?.targets || [],
  };
  if (currentId && ['binding', 'ownership', 'target', 'contract'].includes(editor.entity)) edit.current_id = currentId;
  if (editor.entity === 'ownership') edit.ownership = {
    ...editor.ownership,
    id: draft.id || editor.ownership?.id,
    adapter: draft.adapter,
    path: draft.path,
    selector: selectorFromDraft(draft, true),
    supports: commaValues(draft.supports),
  };
  if (editor.entity === 'target') edit.target = {
    ...editor.target,
    id: draft.id || editor.target?.id,
    adapter: draft.adapter,
    path: draft.path,
    selector: selectorFromDraft(draft),
    lifecycle: ['present', 'absent'].includes(draft.lifecycle)
      ? draft.lifecycle
      : (() => { throw new Error('Target lifecycle is required and must be present or absent.'); })(),
    claims: editor.target?.claims || [],
  };
  if (editor.entity === 'claim') edit.claim = claimFromDraft(draft);
  if (editor.entity === 'contract') edit.contract = {
    ...editor.contract,
    id: draft.id || editor.contract?.id,
    kind: draft.contract_kind,
    source: draft.source,
    participants: lineValues(draft.participants).map(value => {
      const [target, role = 'participant'] = value.split('|').map(entry => entry.trim());
      return { target, role };
    }),
    guarantees: commaValues(draft.guarantees),
  };
  if (editor.entity === 'ownership' || editor.entity === 'target' || editor.entity === 'claim') {
    edit.binding_id = editor.bindingId;
  }
  if (editor.entity === 'claim') {
    edit.target_id = editor.targetId;
    edit.claim_index = editor.claimIndex;
  }
  if (operation === 'delete') {
    if (editor.entity === 'binding') edit.binding = editor.binding;
    if (editor.entity === 'ownership') edit.ownership = editor.ownership;
    if (editor.entity === 'target') edit.target = editor.target;
    if (editor.entity === 'claim') edit.claim = editor.claim;
    if (editor.entity === 'contract') edit.contract = editor.contract;
  }
  return {
    kind: 'nested',
    item_id: editor.itemId,
    edit,
  };
}

function patchFromForm(editor, state, form) {
  const draft = Object.fromEntries(new FormData(form).entries());
  editor.draft = draft;
  return editor.mode === 'nested'
    ? createNestedPatch(editor, form)
    : editor.mode === 'create'
    ? createWizardPatch(editor, draft)
    : editor.anchor
      ? createAnchorPatch(editor, draft)
      : createItemPatch(itemById(state, editor.itemId), draft);
}

function draftValue(editor, draft, name, fallback = '') {
  return Object.prototype.hasOwnProperty.call(draft, name) ? draft[name] : fallback;
}

function appendSelectorEditor(form, editor, draft, selector, ownership = false) {
  const values = { ...selectorDraft(selector, ownership), ...draft };
  const kinds = ownership
    ? ['file', 'module', 'path-prefix']
    : ['file', 'symbol', 'operation', 'heading', 'json-pointer', 'marker'];
  const selectorKind = field(t('items.detail.selector_kind'), values.selector_kind, 'selector_kind', 'select', kinds);
  const name = field(t('items.detail.selector_name'), values.selector_name, 'selector_name');
  const method = field(t('items.detail.selector_method'), values.selector_method, 'selector_method');
  const path = field(t('items.detail.selector_path'), values.selector_path, 'selector_path');
  const value = field(t('items.detail.selector_value'), values.selector_value, 'selector_value');
  const refresh = () => {
    const kind = selectorKind.querySelector('[name="selector_kind"]').value;
    name.hidden = !['symbol', 'module'].includes(kind);
    method.hidden = kind !== 'operation';
    path.hidden = kind !== 'operation';
    value.hidden = kind === 'file' || kind === 'symbol' || kind === 'module' || kind === 'operation';
  };
  selectorKind.querySelector('[name="selector_kind"]').addEventListener('change', refresh);
  form.append(selectorKind, name, method, path, value);
  refresh();
}

function appendNestedEditorFields(form, editor) {
  const draft = editor.draft || {};
  if (editor.operation === 'delete') {
    const warning = document.createElement('p');
    warning.className = 'notice warn';
    warning.textContent = t('items.detail.delete_confirmation');
    form.append(warning);
    return;
  }
  const immutableId = (value) => {
    const wrapper = field(t('items.detail.id'), value, 'id');
    const input = wrapper.querySelector('[name="id"]');
    if (editor.currentId) {
      input.readOnly = true;
      input.setAttribute('aria-readonly', 'true');
      wrapper.classList.add('spec-field-readonly');
    }
    return wrapper;
  };
  if (editor.entity === 'binding') {
    form.append(immutableId(draftValue(editor, draft, 'id', editor.binding?.id)));
    form.append(field(t('items.detail.role'), draftValue(editor, draft, 'role', editor.binding?.role || 'implementation'), 'role', 'select', localizedOptions('target.role', ['implementation', 'verification', 'documentation', 'enforcement', 'contract-source', 'configuration', 'generated', 'migration', 'operation', 'evidence'])));
    form.append(field(t('items.detail.facet'), draftValue(editor, draft, 'facet', editor.binding?.facet), 'facet'));
    form.append(field(t('items.detail.responsibility'), draftValue(editor, draft, 'responsibility', editor.binding?.responsibility), 'responsibility', 'textarea'));
    return;
  }
  if (editor.entity === 'ownership') {
    form.append(immutableId(draftValue(editor, draft, 'id', editor.ownership?.id)));
    form.append(field(t('items.detail.adapter'), draftValue(editor, draft, 'adapter', editor.ownership?.adapter), 'adapter'));
    form.append(field(t('items.detail.path'), draftValue(editor, draft, 'path', editor.ownership?.path), 'path'));
    form.append(field(t('items.detail.supports'), draftValue(editor, draft, 'supports', (editor.ownership?.supports || []).join(', ')), 'supports'));
    appendSelectorEditor(form, editor, draft, editor.ownership?.selector, true);
    return;
  }
  if (editor.entity === 'target') {
    form.append(immutableId(draftValue(editor, draft, 'id', editor.target?.id)));
    form.append(field(t('items.detail.adapter'), draftValue(editor, draft, 'adapter', editor.target?.adapter), 'adapter'));
    form.append(field(t('items.detail.path'), draftValue(editor, draft, 'path', editor.target?.path), 'path'));
    form.append(field(t('items.detail.lifecycle'), draftValue(editor, draft, 'lifecycle', editor.target?.lifecycle), 'lifecycle', 'select', [
      { value: 'present', label: t('items.detail.lifecycle_present') },
      { value: 'absent', label: t('items.detail.lifecycle_absent') },
    ]));
    appendSelectorEditor(form, editor, draft, editor.target?.selector);
    return;
  }
  if (editor.entity === 'claim') {
    const claim = editor.claim || {};
    const kind = draftValue(editor, draft, 'claim_kind', claim.kind || 'satisfies');
    const kindField = field(t('items.detail.claim_kind'), kind, 'claim_kind', 'select', ['satisfies', 'verifies', 'documents', 'enforces', 'generated-from', 'exposes', 'evidences']);
    const criterion = field(t('items.detail.criterion'), draftValue(editor, draft, 'criterion', claim.criterion), 'criterion');
    const covers = field(t('items.detail.covers'), draftValue(editor, draft, 'covers', (claim.covers || []).join(', ')), 'covers');
    const runner = field(t('items.detail.runner'), draftValue(editor, draft, 'runner', claim.runner?.runner), 'runner');
    const runnerArguments = field(t('items.detail.runner_arguments'), draftValue(editor, draft, 'runner_arguments', JSON.stringify(claim.runner?.arguments || {})), 'runner_arguments', 'textarea');
    const anchor = field(t('items.detail.anchor'), draftValue(editor, draft, 'anchor', claim.anchor), 'anchor');
    const rule = field(t('items.detail.rule'), draftValue(editor, draft, 'rule', claim.rule), 'rule');
    const targets = field(t('items.detail.targets'), draftValue(editor, draft, 'targets', (claim.targets || []).join(', ')), 'targets');
    const target = field(t('items.detail.target'), draftValue(editor, draft, 'target', claim.target), 'target');
    const refresh = () => {
      const selected = kindField.querySelector('[name="claim_kind"]').value;
      criterion.hidden = !['satisfies', 'verifies'].includes(selected);
      [covers, runner, runnerArguments].forEach(node => { node.hidden = selected !== 'verifies'; });
      [anchor].forEach(node => { node.hidden = !['documents', 'evidences'].includes(selected); });
      rule.hidden = selected !== 'enforces';
      targets.hidden = selected !== 'generated-from';
      target.hidden = selected !== 'exposes';
    };
    kindField.querySelector('[name="claim_kind"]').addEventListener('change', refresh);
    form.append(kindField, criterion, covers, runner, runnerArguments, anchor, rule, targets, target);
    refresh();
    return;
  }
  if (editor.entity === 'contract') {
    const contract = editor.contract || {};
    form.append(immutableId(draftValue(editor, draft, 'id', contract.id)));
    form.append(field(t('items.detail.contract_kind'), draftValue(editor, draft, 'contract_kind', contract.kind || 'custom'), 'contract_kind', 'select', ['http', 'event', 'function', 'schema', 'cli', 'file', 'custom']));
    form.append(field(t('items.detail.source'), draftValue(editor, draft, 'source', contract.source), 'source'));
    const participants = (contract.participants || []).map(participant => `${participant.target || participant.binding}|${participant.role}`).join('\n');
    form.append(field(t('items.detail.participants'), draftValue(editor, draft, 'participants', participants), 'participants', 'textarea'));
    form.append(field(t('items.detail.guarantees'), draftValue(editor, draft, 'guarantees', (contract.guarantees || []).join(', ')), 'guarantees'));
  }
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
    return {
      kind: 'create_feature',
      ...base,
      summary: formValue(form, 'summary'),
      ...(editor.draft?.criterion_anchor ? { criterion_anchor: editor.draft.criterion_anchor } : {}),
      ...(editor.draft?.target ? { target: editor.draft.target } : {}),
    };
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
      if (state.journeyIntentSearch && approvedIds.size) {
        const createWork = button(t('items.create_work'), '→', () => state.runAction(
          () => state.api.runJourneyAction(state.projection, {
            action: 'create',
            schema: 'syu/work-origin-capability/v1',
            origin: { kind: 'requirement-criterion', criterion: set.criterion },
            title: `${t('work.request.title_from_origin').replace('{anchor}', set.criterion)}`,
          }),
          () => {
            state.targetSuggestions = null;
            state.targetSuggestionSelection = [];
            state.selectedSlice = null;
            state.go('work');
          },
        ), 'btn primary');
        createWork.setAttribute('data-create-work-from-suggestions', '');
        card.append(createWork);
      }
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
  } else if (editor.mode === 'nested') {
    appendNestedEditorFields(form, editor);
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
    let patch;
    try {
      patch = patchFromForm(editor, state, form);
    } catch (error) {
      state.specificationError = error.message;
      state.specificationPreview = null;
      state.render();
      return;
    }
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
        const appliedPatch = state.specificationPreview.patch;
        const previousItemId = state.specificationEditor?.itemId || null;
        const afterApply = state.specificationEditor?.afterApply;
        state.projection = await state.api.readProjection();
        state.specificationCandidates = null;
        state.specificationTrace = null;
        state.specificationTraceRoot = null;
        state.specificationEditor = null;
        state.specificationPreview = null;
        if (afterApply) {
          await afterApply(state, appliedPatch);
          return;
        }
        const patchItemId = appliedPatch.kind === 'create_requirement' || appliedPatch.kind === 'create_feature'
          ? appliedPatch.id
          : appliedPatch.item_id || appliedPatch.itemId || previousItemId;
        const items = state.projection.specifications?.specifications || [];
        const selected = items.find(item => item.id === patchItemId)
          || items.find(item => item.id === previousItemId)
          || items.find(item => item.status === 'implemented' && item.criteria?.length)
          || items[0]
          || null;
        state.selectedSpecification = selected?.id || null;
        state.specificationDetailTab = 'information';
        state.specificationTraceNode = null;
        syncSpecificationLocation(state, false);
    }), 'btn primary specification-apply');
    actions.append(apply);
  }
  root.append(form);
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
  if (claim.kind === 'verifies') return `verifies ${claim.criterion || ''} · covers ${(claim.covers || []).join(', ') || '—'}`;
  if (claim.kind === 'satisfies') return `satisfies ${claim.criterion || ''}`;
  if (claim.kind === 'covers') return `covers ${(claim.targets || []).join(', ')}`;
  if (claim.kind === 'documents') return `documents ${claim.anchor || '—'}`;
  if (claim.kind === 'enforces') return `enforces ${claim.rule || '—'}`;
  if (claim.kind === 'generated-from') return `generated from ${(claim.targets || []).join(', ') || '—'}`;
  if (claim.kind === 'exposes') return `exposes ${claim.target || '—'}`;
  if (claim.kind === 'evidences') return `evidences ${claim.anchor || '—'}`;
  return `${claim.kind || 'claim'} · ${JSON.stringify(claim)}`;
}

function beginNestedEditor(state, config) {
  const entityValue = config[config.entity];
  state.specificationEditor = {
    mode: 'nested',
    operation: 'upsert',
    currentId: config.currentId || (config.operation !== 'delete' && entityValue?.id ? String(entityValue.id) : null),
    ...config,
  };
  state.specificationPreview = null;
  state.specificationError = null;
  state.render();
}

function beginNestedDelete(state, config) {
  beginNestedEditor(state, { ...config, operation: 'delete' });
}

function renderInformation(root, state, selected, onItem, onTarget, options = {}) {
  const counts = selected.bindings || [];
  const targets = counts.flatMap(binding => binding.targets || []);
  const verificationTargets = targets.filter(target => target.claims?.some(claim => claim.kind === 'verifies'));
  root.append(detailCard(t('items.detail.basic'), [
    detailLabel(t('items.detail.kind'), selected.kind),
    detailLabel(t('items.detail.title'), selected.title),
    detailLabel(t('items.detail.presentation_title_key'), selected.presentation_title_key),
    detailLabel(t('items.detail.status'), selected.status),
    detailLabel(t('items.detail.priority'), selected.priority),
    detailLabel(t('items.detail.summary'), selected.summary),
    detailLabel(t('items.detail.description'), selected.description),
    detailLabel(t('items.detail.source'), selected.path),
    detailLabel(t('items.detail.source_hash'), selected.source_hash),
    detailList(t('items.detail.anchors'), selected.anchors),
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
      row.className = `specification-detail-anchor${kind === 'criterion' ? ' specification-criterion' : ''}${options.highlightedAnchor === value.anchor ? ' is-highlighted' : ''}`;
      const head = document.createElement('div');
      head.className = 'spec-detail-anchor-head';
      const exact = document.createElement('code');
      exact.textContent = value.anchor;
      const badge = document.createElement('span');
      badge.className = 'chip';
      badge.textContent = value.kind || value.level || kind;
      if (kind === 'criterion') {
        const anchor = document.createElement('strong');
        anchor.append(exact);
        head.append(anchor, badge);
      } else {
        head.append(exact, badge);
      }
      row.append(head);
      const statement = document.createElement('p');
      statement.textContent = value.statement;
      if (options.highlightedAnchor === value.anchor) statement.setAttribute('data-work-specification-criterion', '');
      row.append(statement);
      if (!options.readOnly && kind === 'criterion') {
        const capability = (state.projection.specifications?.origin_capabilities || []).find(candidate =>
          candidate.origin?.kind === 'requirement-criterion'
          && candidate.origin.criterion === value.anchor
        );
        if (capability) {
          const createWork = button(t('items.create_work'), '→', () => state.runAction(
            () => state.api.runJourneyAction(state.projection, {
              action: 'create',
              schema: capability.schema,
              origin: capability.origin,
              title: `${t('work.request.title_from_origin').replace('{anchor}', value.anchor)}`,
            }),
            () => { state.selectedSlice = null; state.go('work'); },
          ), 'btn small');
          createWork.setAttribute('data-create-work', '');
          createWork.setAttribute('data-create-work-anchor', value.anchor);
          createWork.disabled = !capability.enabled;
          if (!capability.enabled) createWork.title = capability.disabled_message || '';
          row.append(createWork);
        }
      }
      if (!options.readOnly && kind === 'criterion') {
        const reviewSuggestions = button(t('items.suggestions.review'), '◎', () => runBusy(state, async () => {
          const suggestions = await state.api.readTargetSuggestions(value.anchor);
          state.targetSuggestions = suggestions;
          const approved = new Set(suggestions.approved_ids || []);
          state.targetSuggestionSelection = (suggestions.suggestions || [])
            .filter(candidate => !approved.has(candidate.id))
            .map(candidate => candidate.id);
          state.specificationError = null;
        }), 'btn small');
        reviewSuggestions.setAttribute('data-review-target-suggestions', '');
        row.append(reviewSuggestions);
      }
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
  if (!options.readOnly) bindingHeading.append(button(t('items.detail.add_binding'), '+', () => beginNestedEditor(state, {
    entity: 'binding', itemId: selected.id,
    binding: { id: '', role: 'implementation', facet: '', responsibility: '', owns: [], targets: [] },
  }), 'btn small ghost'));
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
    const bindingHead = document.createElement('div');
    bindingHead.className = 'spec-detail-anchor-head';
    bindingHead.append(bindingTitle);
    if (!options.readOnly) {
      bindingHead.append(button(t('common.edit'), '✎', () => beginNestedEditor(state, {
        entity: 'binding', itemId: selected.id, binding: bindingPatchModel(binding),
      }), 'btn small ghost'));
      bindingHead.append(button(t('common.delete'), '×', () => beginNestedDelete(state, {
        entity: 'binding', itemId: selected.id, binding: bindingPatchModel(binding),
      }), 'btn small ghost'));
    }
    bindingSection.append(bindingHead, detailCard('', [
      detailLabel(t('items.detail.role'), binding.role),
      detailLabel(t('items.detail.facet'), binding.facet),
      detailLabel(t('items.detail.responsibility'), binding.responsibility),
    ], 'specification-detail-inline-card'));
    const ownerships = document.createElement('div');
    ownerships.className = 'specification-detail-ownerships';
    const ownershipHead = document.createElement('div');
    ownershipHead.className = 'spec-detail-anchor-head';
    ownershipHead.append(Object.assign(document.createElement('strong'), { textContent: t('items.detail.owns') }));
    if (!options.readOnly) ownershipHead.append(button(t('items.detail.add_ownership'), '+', () => beginNestedEditor(state, {
      entity: 'ownership', itemId: selected.id, bindingId: localIdFromAnchor(binding.anchor),
      ownership: { id: '', adapter: '', path: '', selector: { kind: 'file' }, supports: [] },
    }), 'btn small ghost'));
    ownerships.append(ownershipHead);
    if (!(binding.owns || []).length) ownerships.append(Object.assign(document.createElement('p'), { className: 'spec-detail-empty', textContent: t('items.detail.empty') }));
    (binding.owns || []).forEach(scope => {
      const row = document.createElement('div');
      row.className = 'specification-detail-ownership';
      row.append(detailLabel(t('items.detail.id'), scope.id), detailLabel(t('items.detail.adapter'), scope.adapter), detailLabel(t('items.detail.path'), scope.path), detailLabel(t('items.detail.selector'), JSON.stringify(scope.selector)), detailList(t('items.detail.supports'), scope.supports));
      if (!options.readOnly) row.append(button(t('common.edit'), '✎', () => beginNestedEditor(state, {
        entity: 'ownership', itemId: selected.id, bindingId: localIdFromAnchor(binding.anchor), ownership: ownershipPatchModel(scope),
      }), 'btn small ghost'), button(t('common.delete'), '×', () => beginNestedDelete(state, {
        entity: 'ownership', itemId: selected.id, bindingId: localIdFromAnchor(binding.anchor), ownership: ownershipPatchModel(scope),
      }), 'btn small ghost'));
      ownerships.append(row);
    });
    bindingSection.append(ownerships);
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
      open.dataset.sourceTarget = target.reference;
      targetHead.append(targetRef, open);
      if (!options.readOnly) {
        targetHead.append(button(t('common.edit'), '✎', () => beginNestedEditor(state, {
          entity: 'target', itemId: selected.id, bindingId: localIdFromAnchor(binding.anchor), target: targetPatchModel(target),
        }), 'btn small ghost'));
        targetHead.append(button(t('common.delete'), '×', () => beginNestedDelete(state, {
          entity: 'target', itemId: selected.id, bindingId: localIdFromAnchor(binding.anchor), target: targetPatchModel(target),
        }), 'btn small ghost'));
      }
      targetCard.append(targetHead);
      const targetFields = document.createElement('dl');
      targetFields.className = 'spec-detail-fields compact';
      targetFields.append(
        detailLabel(t('items.detail.adapter'), target.adapter),
        detailLabel(t('items.detail.path'), target.path),
        detailLabel(t('items.detail.lifecycle'), target.lifecycle),
        detailLabel(t('items.detail.selector'), JSON.stringify(target.selector)),
        detailList(t('items.detail.claims'), (target.claims || []).map(claimLabel)),
      );
      targetCard.append(targetFields);
      if (!options.readOnly) targetCard.append(button(t('items.detail.add_claim'), '+', () => beginNestedEditor(state, {
        entity: 'claim', itemId: selected.id, bindingId: localIdFromAnchor(binding.anchor), targetId: localIdFromAnchor(target.reference),
        claimIndex: (target.claims || []).length, claim: { kind: 'satisfies', criterion: '' },
      }), 'btn small ghost'));
      (target.claims || []).forEach((claim, claimIndex) => {
        if (options.readOnly) return;
        targetCard.append(button(`${t('common.edit')} ${claimLabel(claim)}`, '✎', () => beginNestedEditor(state, {
          entity: 'claim', itemId: selected.id, bindingId: localIdFromAnchor(binding.anchor), targetId: localIdFromAnchor(target.reference),
          claimIndex, claim: claimPatchModel(claim),
        }), 'btn small ghost'));
        targetCard.append(button(`${t('common.delete')} ${claimLabel(claim)}`, '×', () => beginNestedDelete(state, {
          entity: 'claim', itemId: selected.id, bindingId: localIdFromAnchor(binding.anchor), targetId: localIdFromAnchor(target.reference),
          claimIndex, claim: claimPatchModel(claim),
        }), 'btn small ghost'));
      });
      targetList.append(targetCard);
    });
    if (!options.readOnly) bindingSection.append(button(t('items.detail.add_target'), '+', () => beginNestedEditor(state, {
      entity: 'target', itemId: selected.id, bindingId: localIdFromAnchor(binding.anchor),
      target: { id: '', adapter: '', path: '', selector: { kind: 'file' }, lifecycle: 'present', claims: [] },
    }), 'btn small ghost'));
    bindingSection.append(targetList);
    bindingCard.append(bindingSection);
  });
  root.append(bindingCard);

  const contracts = document.createElement('section');
  contracts.className = 'card specification-detail-card';
  const contractsHeading = document.createElement('h3');
  contractsHeading.textContent = t('items.detail.contracts');
  contracts.append(contractsHeading);
  if (!options.readOnly) contractsHeading.append(button(t('items.detail.add_contract'), '+', () => beginNestedEditor(state, {
    entity: 'contract', itemId: selected.id,
    contract: { id: '', kind: 'custom', source: '', participants: [], guarantees: [] },
  }), 'btn small ghost'));
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
    if (!options.readOnly) card.append(
      button(t('common.edit'), '✎', () => beginNestedEditor(state, {
        entity: 'contract', itemId: selected.id, contract: contractPatchModel(contract),
      }), 'btn small ghost'),
      button(t('common.delete'), '×', () => beginNestedDelete(state, {
        entity: 'contract', itemId: selected.id, contract: contractPatchModel(contract),
      }), 'btn small ghost'),
    );
    contracts.append(card);
  });
  root.append(contracts);
}

function renderRelatedFromTrace(root, state, selected, onItem, onTarget) {
  ensureSpecificationTrace(state, selected);
  const trace = state.specificationTrace;
  if (state.specificationTraceLoading || !trace || trace.error || trace.root_item_id !== selected.id) return;
  const related = trace.related || {};
  const groups = {
    specification: related.specification || [],
    implementation: related.implementation || [],
    verification: related.verification || [],
  };
  const availableKinds = Object.keys(groups).filter(kind => groups[kind].length);
  if (!availableKinds.length) return;
  if (!availableKinds.includes(state.relatedKind)) state.relatedKind = availableKinds[0];
  const section = document.createElement('section');
  section.className = 'specification-related';
  section.append(Object.assign(document.createElement('h3'), { textContent: t('items.related') }));
  const chooser = document.createElement('label');
  chooser.className = 'related-chooser';
  chooser.append(Object.assign(document.createElement('span'), { textContent: t('items.related.choose') }));
  const select = document.createElement('select');
  select.className = 'native-select';
  availableKinds.forEach(kind => {
    const option = document.createElement('option');
    option.value = kind;
    option.textContent = `${kind === 'verification' ? '✓' : kind === 'implementation' ? '⌘' : '◆'} ${t(`items.related.${kind}`)} · ${groups[kind].length}`;
    option.selected = kind === state.relatedKind;
    select.append(option);
  });
  select.addEventListener('change', () => {
    state.relatedKind = select.value;
    state.render();
  });
  chooser.append(select);
  section.append(chooser);
  const list = document.createElement('div');
  list.className = 'related-list';
  groups[state.relatedKind].forEach(entry => {
    if (state.relatedKind === 'specification') {
      list.append(button(
        localizeSpecificationTitle(entry),
        entry.kind === 'feature' ? '◆' : '◈',
        () => onItem(entry.item_id),
        'related-row specification',
      ));
      return;
    }
    const target = entry.target;
    const targetButton = button(
      target.path || target.reference,
      state.relatedKind === 'verification' ? '✓' : '⌘',
      () => onTarget(target),
      `related-row ${state.relatedKind}`,
    );
    targetButton.dataset.sourceTarget = target.reference;
    list.append(targetButton);
  });
  section.append(list);
  root.append(section);
}

function traceNodeButton(state, node, onSelect) {
  const buttonNode = document.createElement('button');
  buttonNode.type = 'button';
  buttonNode.className = `trace-node trace-node-${node.kind}${state.specificationTraceNode === node.id ? ' is-selected' : ''}`;
  buttonNode.dataset.traceNode = node.id;
  if (node.source_target) buttonNode.dataset.sourceTarget = node.source_target;
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

function renderTrace(root, state, selected, onSelect, onLocationChange = syncSpecificationLocation) {
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
      onLocationChange(state);
      state.render();
    }, `btn small ${mode === state.specificationTraceMode ? 'primary' : 'ghost'}`);
    controls.append(modeButton);
  });
  const depth = document.createElement('label');
  depth.className = 'trace-depth';
  depth.append(Object.assign(document.createElement('span'), { textContent: t('items.detail.depth') }));
  const select = document.createElement('select');
  select.className = 'native-select';
  [1, 2, 3, 4, 5, 6, 7, 8].forEach(value => {
    const option = document.createElement('option');
    option.value = value;
    option.textContent = String(value);
    option.selected = value === state.specificationTraceDepth;
    select.append(option);
  });
  select.addEventListener('change', () => {
    state.specificationTraceDepth = Number(select.value);
    state.specificationTraceRoot = null;
    onLocationChange(state);
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
  if (trace.truncated || trace.hidden_edge_count) {
    const notice = document.createElement('p');
    notice.className = 'notice warn';
    const hidden = (trace.hidden_node_count || 0) + (trace.hidden_edge_count || 0);
    notice.textContent = t('items.detail.hidden').replace('{count}', String(hidden));
    root.append(notice);
  }
  const edges = document.createElement('section');
  edges.className = 'card trace-edge-card';
  edges.append(Object.assign(document.createElement('h3'), { textContent: t('items.detail.edge') }));
  const list = document.createElement('ol');
  list.className = 'trace-edge-list';
  trace.edges.forEach(edge => {
    const item = document.createElement('li');
    const edgeButton = document.createElement('button');
    edgeButton.type = 'button';
    edgeButton.className = 'trace-edge';
    edgeButton.textContent = `${edge.from} → ${edge.display_label} → ${edge.to}`;
    edgeButton.setAttribute('aria-label', `${edge.from} ${edge.display_label} ${edge.to}`);
    edgeButton.addEventListener('click', () => {
      const node = trace.nodes.find(candidate => candidate.id === edge.to)
        || trace.nodes.find(candidate => candidate.id === edge.from);
      if (node) onSelect(node);
    });
    item.append(edgeButton);
    list.append(item);
  });
  if (!trace.edges.length) list.append(Object.assign(document.createElement('li'), { textContent: t('items.detail.trace_empty') }));
  edges.append(list);
  root.append(edges);
}

function closureLabel(state) {
  const key = {
    'declaration-only': 'items.detail.declaration_only',
    'implementation-missing': 'items.detail.implementation_missing',
    'verification-missing': 'items.detail.verification_missing',
    'target-unresolved': 'items.detail.target_unresolved',
  }[state];
  return key ? t(key) : state;
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
    card.append(detailList(t('items.detail.declared_implementation'), closure.implementation_targets));
    card.append(detailList(t('items.detail.declared_verification'), closure.verification_targets));
    card.append(detailLabel(t('items.detail.runtime_status'), closure.runtime_status));
    card.append(detailLabel(t('items.detail.runtime_timestamp'), closure.runtime_timestamp || t('items.detail.not_available')));
    card.append(detailLabel(t('items.detail.runtime_revision'), closure.runtime_revision || t('items.detail.not_available')));
    card.append(detailLabel(t('items.detail.runtime_receipt'), closure.runtime_receipt || t('items.detail.not_available')));
    if (closure.runtime_executions?.length) {
      card.append(detailList(t('items.detail.runtime_executions'), closure.runtime_executions.map(execution =>
        `${execution.identity} · ${execution.target} · ${execution.status}`,
      )));
    }
    if (closure.runtime_status === 'unavailable') {
      const note = document.createElement('p');
      note.className = 'spec-detail-evidence-note';
      note.textContent = t('items.detail.runtime_unavailable');
      card.append(note);
    }
    if (closure.readiness_blockers?.length) {
      card.append(detailList(t('readiness.blockers'), closure.readiness_blockers));
    }
    if (closure.diagnostics?.length) {
      const diagnostics = closure.diagnostics.map(diagnostic =>
        `${diagnostic.identity} · ${diagnostic.severity} · ${diagnostic.message}`,
      );
      card.append(detailList(t('diagnostics.title'), diagnostics));
      const reasons = closure.diagnostics
        .map(diagnostic => diagnostic.reason)
        .filter(Boolean);
      if (reasons.length) card.append(detailList(t('items.detail.reason'), reasons));
    }
    if (closure.reasons?.length) card.append(detailList(t('items.detail.reasons'), closure.reasons));
    const hiddenClosure = (closure.hidden_target_count || 0)
      + (closure.hidden_reason_count || 0)
      + (closure.hidden_readiness_count || 0)
      + (closure.hidden_diagnostic_count || 0);
    if (hiddenClosure) {
      const notice = document.createElement('p');
      notice.className = 'notice warn';
      notice.textContent = t('items.detail.hidden').replace('{count}', String(hiddenClosure));
      card.append(notice);
    }
    root.append(card);
  });
  const hidden = (trace.hidden_related_count || 0)
    + (trace.hidden_related_claim_count || 0)
    + (trace.hidden_closure_count || 0)
    + (trace.hidden_closure_target_count || 0)
    + (trace.hidden_reason_count || 0)
    + (trace.hidden_readiness_count || 0)
    + (trace.hidden_diagnostic_count || 0);
  if (hidden) {
    const notice = document.createElement('p');
    notice.className = 'notice warn';
    notice.textContent = t('items.detail.hidden').replace('{count}', String(hidden));
    root.append(notice);
  }
  if (!(trace.closures || []).length) root.append(Object.assign(document.createElement('p'), { className: 'empty-state', textContent: t('items.detail.trace_empty') }));
}

function renderSourceInspector(root, state, onClose, onLocationChange = syncSpecificationLocation) {
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
    const focusKey = state.specificationSourceFocusKey || target.reference;
    state.specificationSourceTarget = null;
    state.specificationSource = null;
    state.specificationTraceNode = null;
    state.specificationSourceFocusKey = focusKey;
    onClose?.();
    onLocationChange(state);
    state.render();
    setTimeout(() => {
      const control = [...document.querySelectorAll('[data-source-target]')]
        .find(node => node.dataset.sourceTarget === focusKey);
      control?.focus();
    }, 0);
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
  const workspaceAdapter = options.workspaceAdapter;
  const onLocationChange = workspaceAdapter?.syncLocation
    || options.onLocationChange
    || ((nextState, push = true) => syncSpecificationLocation(nextState, push));
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
  const heading = Object.assign(document.createElement('h2'), { textContent: localizeSpecificationTitle(selected) });
  title.append(heading);
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
  if (options.action) actions.append(button(
    options.action.label,
    options.action.icon || '→',
    options.action.onClick,
    options.action.className || 'btn small primary',
  ));
  if (!options.readOnly) actions.append(button(t('common.edit'), '✎', () => {
    state.specificationEditor = { mode: 'edit', itemId: selected.id };
    state.specificationPreview = null;
    state.render();
  }, 'btn small primary'));
  if (actions.childElementCount) head.append(actions);
  if (options.hideHeading) {
    head.hidden = true;
    head.setAttribute('aria-hidden', 'true');
  }
  main.append(head);
  const description = document.createElement('p');
  description.className = 'specification-summary';
  description.textContent = selected.summary || selected.description || t('items.detail.empty');
  main.append(description);
  if (!options.readOnly && selected.kind === 'feature') {
    const capabilities = (selected.origin_capabilities || [])
      .filter(capability => capability.label === 'Feature implementation' || capability.label === 'Implementation target');
    const origins = document.createElement('section');
    origins.className = 'card specification-group feature-work-origins';
    origins.append(Object.assign(document.createElement('h3'), { textContent: t('items.create_work') }));
    capabilities.forEach(capability => {
      const origin = capability.origin;
      const reference = origin?.kind === 'feature-implementation-binding'
        ? origin.binding
        : origin?.target;
      const action = button(
        `${capability.label}: ${reference || t('journey.advanced.none')}`,
        origin?.kind === 'feature-implementation-binding' ? '→' : '↗',
        () => state.runAction(
          () => state.api.runJourneyAction(state.projection, {
            schema: capability.schema,
            action: 'create',
            origin,
            title: `${localizeSpecificationTitle(selected)} · ${reference || capability.label}`,
          }),
          () => { state.selectedSlice = null; state.go('work'); },
        ),
        `btn small ${capability.enabled ? '' : 'disabled'}`,
      );
      action.disabled = !capability.enabled;
      if (!capability.enabled) action.title = capability.disabled_message || '';
      origins.append(action);
    });
    if (origins.childElementCount > 1) main.append(origins);
  }
  const tabs = document.createElement('div');
  tabs.className = 'specification-detail-tabs';
  tabs.setAttribute('role', 'tablist');
  const selectDetailTab = (tabName, focus = false) => {
    state.specificationDetailTab = tabName;
    state.specificationTraceNode = null;
    state.specificationSourceTarget = null;
    onLocationChange(state);
    state.render();
    if (focus) setTimeout(() => document.querySelector(`[data-detail-tab="${tabName}"]`)?.focus(), 0);
  };
  SPECIFICATION_DETAIL_TABS.forEach(tabName => {
    const tab = document.createElement('button');
    tab.type = 'button';
    tab.className = `tab${state.specificationDetailTab === tabName ? ' active' : ''}`;
    tab.id = `specification-detail-tab-${tabName}`;
    tab.dataset.detailTab = tabName;
    tab.setAttribute('role', 'tab');
    tab.setAttribute('aria-selected', String(state.specificationDetailTab === tabName));
    tab.setAttribute('aria-controls', `specification-detail-panel-${tabName}`);
    tab.tabIndex = state.specificationDetailTab === tabName ? 0 : -1;
    tab.textContent = t(`items.detail.${tabName}`);
    tab.addEventListener('click', () => selectDetailTab(tabName));
    tabs.append(tab);
  });
  tabs.addEventListener('keydown', event => {
    const current = SPECIFICATION_DETAIL_TABS.indexOf(state.specificationDetailTab);
    if (current < 0) return;
    const delta = { ArrowRight: 1, ArrowDown: 1, ArrowLeft: -1, ArrowUp: -1 }[event.key];
    const next = event.key === 'Home'
      ? 0
      : event.key === 'End'
        ? SPECIFICATION_DETAIL_TABS.length - 1
        : delta === undefined
          ? -1
          : (current + delta + SPECIFICATION_DETAIL_TABS.length) % SPECIFICATION_DETAIL_TABS.length;
    if (next < 0) return;
    event.preventDefault();
    selectDetailTab(SPECIFICATION_DETAIL_TABS[next], true);
  });
  main.append(tabs);
  const body = document.createElement('div');
  body.className = 'specification-detail-tab-body';
  body.id = `specification-detail-panel-${state.specificationDetailTab}`;
  body.setAttribute('role', 'tabpanel');
  body.setAttribute('aria-labelledby', `specification-detail-tab-${state.specificationDetailTab}`);
  body.tabIndex = 0;
  const onItem = workspaceAdapter?.setSelectedItem || options.onItem || (itemId => {
    state.selectedSpecification = itemId;
    state.specificationTrace = null;
    state.specificationTraceRoot = null;
    state.specificationSourceTarget = null;
    onLocationChange(state);
    state.render();
  });
  const onTarget = workspaceAdapter?.openTarget || options.onTarget || (target => {
    state.specificationTraceNode = target.reference;
    state.specificationSourceTarget = target;
    onLocationChange(state);
    state.render();
  });
  const rememberTarget = target => {
    state.specificationSourceFocusKey = target?.reference || null;
    onTarget(target);
  };
  if (state.specificationDetailTab === 'information') {
    renderInformation(body, state, selected, onItem, rememberTarget, options);
    renderRelatedFromTrace(body, state, selected, onItem, rememberTarget);
    if (!options.readOnly) renderTargetSuggestions(body, state);
  }
  else if (state.specificationDetailTab === 'trace') {
    renderTrace(body, state, selected, node => {
      state.specificationTraceNode = node.id;
      state.specificationSourceTarget = node.source_target ? specificationTarget(state, node.source_target) : null;
      if (node.source_target) state.specificationSourceFocusKey = node.source_target;
      if (node.item_id && node.item_id !== selected.id && node.kind === 'item') {
        onItem(node.item_id);
        return;
      }
      if (workspaceAdapter) workspaceAdapter.setSelectedNode(node.id);
      else onLocationChange(state);
      state.render();
    }, onLocationChange);
  } else renderEvidence(body, state, selected);
  main.append(body);
  shell.append(main);
  if (state.specificationSourceTarget) {
    const inspector = document.createElement('aside');
    inspector.className = 'canvas specification-detail-inspector';
    renderSourceInspector(inspector, state, options.onSourceClose, onLocationChange);
    shell.append(inspector);
  }
  root.append(shell);
}

export function renderSpecificationDetail(root, state, selected, options = {}) {
  renderSpecificationWorkspace(root, state, selected, options);
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
      const buttonNode = document.createElement('div');
      buttonNode.className = `rail-item${item.id === state?.selectedSpecification ? ' active' : ''}`;
      const selectButton = document.createElement('button');
      selectButton.type = 'button';
      selectButton.className = 'rail-item-select';
      const title = document.createElement('div');
      const id = document.createElement('b');
      id.textContent = item.id;
      const name = document.createElement('p');
      name.textContent = localizeSpecificationTitle(item);
      title.append(id, name);
      selectButton.append(title);
      if (candidate.relevance?.length) {
        const reason = document.createElement('small');
        reason.textContent = candidate.relevance[0];
        selectButton.append(reason);
      }
      selectButton.addEventListener('click', () => {
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
      buttonNode.append(selectButton);
      const originActions = document.createElement('div');
      originActions.className = 'rail-item-actions';
      (item.origin_capabilities || []).forEach(capability => {
        const action = document.createElement('button');
        action.type = 'button';
        action.className = 'btn compact';
        action.textContent = capability.label;
        action.disabled = !capability.enabled;
        action.title = capability.enabled
          ? capability.label
          : (capability.disabled_message || capability.disabled_code || capability.label);
        action.addEventListener('click', () => state.runAction(
          () => state.api.runJourneyAction(state.projection, {
            schema: capability.schema,
            action: 'create',
            origin: capability.origin,
            title: `${localizeSpecificationTitle(item)} · ${capability.label}`,
          }),
          () => { state.selectedSlice = null; state.go('work'); },
        ));
        originActions.append(action);
      });
      if (originActions.childElementCount) buttonNode.append(originActions);
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
  renderSpecificationDetail(root, state, selected);
  return entries;
}
