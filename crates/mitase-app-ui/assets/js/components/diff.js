import { localizeEnum, translate } from '../i18n.js';

const t = key => translate(key);

function element(tag, className, text) {
  const node = document.createElement(tag);
  if (className) node.className = className;
  if (text !== undefined && text !== null) node.textContent = text;
  return node;
}

function statusLabel(status) {
  const normalized = String(status || 'modified').toLowerCase();
  const statuses = new Set(['modified', 'added', 'deleted', 'renamed', 'untracked', 'binary']);
  return statuses.has(normalized) ? localizeEnum('status', normalized) : status;
}

function diffLine(line) {
  const kind = line.startsWith('+++') || line.startsWith('---')
    ? 'meta'
    : line.startsWith('+')
      ? 'added'
      : line.startsWith('-')
        ? 'removed'
        : line.startsWith('@@')
          ? 'hunk'
          : 'context';
  return element('span', `diff-line ${kind}`, line || ' ');
}

export function renderDiff(diff, root, options = {}) {
  if (!root) return;
  root.replaceChildren();
  if (options.loading) {
    const loading = element('div', 'context-empty-state');
    loading.append(
      element('span', 'context-empty-icon diff-loading', '↻'),
      element('h2', null, t('diff.loading.title')),
      element('p', null, t('diff.loading.description')),
    );
    root.append(loading);
    return;
  }
  if (options.error) {
    const error = element('details', 'diagnostic-card error');
    const summary = element('summary', 'diagnostic-card-summary');
    summary.append(
      element('span', 'diagnostic-marker', '!'),
      element('span', 'diagnostic-card-copy', t('diff.error.title')),
      element('span', 'chip severity-error', t('diagnostics.severity.error')),
    );
    error.append(summary, element('div', 'diagnostic-card-detail', options.error));
    root.append(error);
    return;
  }
  const files = diff?.files || [];
  const summary = element('section', `diff-summary ${files.length ? 'active' : 'muted'}`);
  const summaryCopy = element('div');
  summaryCopy.append(
    element('h2', null, files.length ? t('diff.title') : t('diff.empty.title')),
    element('p', null, files.length
      ? t('diff.summary').replace('{count}', String(files.length))
      : t('diff.empty.description')),
  );
  const totals = element('div', 'diff-totals');
  totals.append(
    element('span', 'chip diff-additions', `+${diff?.additions || 0}`),
    element('span', 'chip diff-deletions', `−${diff?.deletions || 0}`),
  );
  summary.append(summaryCopy, totals);
  root.append(summary);
  if (!files.length) return;

  const list = element('div', `diff-files${options.compact ? ' compact' : ''}`);
  files.forEach((file, index) => {
    const details = element('details', `diff-file status-${file.status || 'modified'}`);
    if (options.openFirst && index === 0) details.open = true;
    const head = element('summary', 'diff-file-head');
    const copy = element('span', 'diff-file-copy');
    copy.append(
      element('strong', null, file.path),
      element('span', 'chip', statusLabel(file.status)),
    );
    const annotation = options.annotations?.[file.path];
    if (annotation) {
      copy.append(element(
        'span',
        `chip ${annotation.tone || ''}`.trim(),
        annotation.label,
      ));
    }
    const counts = element('span', 'diff-file-counts');
    counts.append(
      element('span', 'diff-additions', `+${file.additions || 0}`),
      element('span', 'diff-deletions', `−${file.deletions || 0}`),
    );
    head.append(element('span', 'status-marker', '±'), copy, counts);
    details.append(head);
    const code = element('pre', 'diff-code');
    String(file.patch || t('diff.binary'))
      .split('\n')
      .filter(line => !line.startsWith('diff --git ')
        && !line.startsWith('index ')
        && !line.startsWith('--- ')
        && !line.startsWith('+++ '))
      .forEach(line => code.append(diffLine(line)));
    details.append(code);
    list.append(details);
  });
  root.append(list);
}
