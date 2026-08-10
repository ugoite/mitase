import { translate } from '../i18n.js';

const t = key => translate(key);

function element(tag, className, text) {
  const node = document.createElement(tag);
  if (className) node.className = className;
  if (text !== undefined && text !== null) node.textContent = text;
  return node;
}

function diagnosticTitle(diagnostic) {
  if (document.documentElement.lang !== 'ja') {
    return diagnostic.message || t('diagnostics.unknown');
  }
  const family = String(diagnostic.rule_id || '')
    .replace(/^MITASE-/, '')
    .split('-')[0]
    .toLowerCase();
  const families = new Set([
    'schema', 'id', 'anchor', 'binding', 'contract', 'doc', 'facet', 'feature',
    'generated', 'operation', 'philosophy', 'policy', 'readiness', 'requirement',
    'target', 'verification', 'work', 'change', 'completion',
  ]);
  return families.has(family) ? t(`diagnostics.family.${family}`) : t('diagnostics.unknown');
}

export function renderDiagnostic(diagnostic) {
  const severity = diagnostic.severity || 'info';
  const item = element('details', `diagnostic-card ${severity}`);
  const summary = element('summary', 'diagnostic-card-summary');
  const marker = element('span', 'diagnostic-marker', severity === 'error' ? '!' : severity === 'warning' ? '◇' : 'i');
  marker.setAttribute('aria-hidden', 'true');
  const copy = element('span', 'diagnostic-card-copy');
  copy.append(
    element('strong', null, diagnosticTitle(diagnostic)),
    element('span', 'diagnostic-location', [
      diagnostic.primary?.path,
      diagnostic.primary?.line ? `:${diagnostic.primary.line}` : '',
    ].join('')),
  );
  const meta = element('span', 'diagnostic-card-meta');
  meta.append(
    element('span', `chip severity-${severity}`, t(`diagnostics.severity.${severity}`)),
    element('span', 'chip', t(`diagnostics.phase.${diagnostic.phase || 'plan'}`)),
    element('code', 'chip diagnostic-rule', diagnostic.rule_id),
  );
  summary.append(marker, copy, meta);
  item.append(summary);

  const detail = element('div', 'diagnostic-card-detail');
  if (document.documentElement.lang === 'ja' && diagnostic.message) {
    const meaning = element('section', 'diagnostic-detail-section');
    meaning.append(
      element('h3', null, t('diagnostics.meaning')),
      element('p', null, t(`diagnostics.phase_help.${diagnostic.phase || 'plan'}`)),
    );
    const technical = element('details', 'diagnostic-technical');
    technical.append(
      element('summary', null, t('diagnostics.technical')),
      element('p', null, diagnostic.message),
    );
    meaning.append(technical);
    detail.append(meaning);
  }
  if (diagnostic.help) {
    const help = element('section', 'diagnostic-detail-section');
    help.append(element('h3', null, t('diagnostics.next')), element('p', null, diagnostic.help));
    detail.append(help);
  }
  if (diagnostic.fix?.description) {
    const fix = element('section', 'diagnostic-detail-section fix');
    fix.append(element('h3', null, t('diagnostics.fix')), element('p', null, diagnostic.fix.description));
    detail.append(fix);
  }
  if (diagnostic.anchor || diagnostic.target) {
    const references = element('section', 'diagnostic-detail-section');
    references.append(element('h3', null, t('diagnostics.references')));
    if (diagnostic.anchor) references.append(element('code', null, diagnostic.anchor));
    if (diagnostic.target) references.append(element('code', null, String(diagnostic.target)));
    detail.append(references);
  }
  if (diagnostic.evidence?.length) {
    const evidence = element('section', 'diagnostic-detail-section');
    evidence.append(element('h3', null, t('diagnostics.evidence')));
    const list = element('ul', 'diagnostic-evidence');
    diagnostic.evidence.forEach(value => list.append(
      element('li', null, `${value.kind}: ${value.value}`),
    ));
    evidence.append(list);
    detail.append(evidence);
  }
  if (diagnostic.related?.length) {
    const related = element('section', 'diagnostic-detail-section');
    related.append(element('h3', null, t('diagnostics.related')));
    const list = element('ul', 'diagnostic-evidence');
    diagnostic.related.forEach(value => list.append(element(
      'li',
      null,
      `${value.location?.path || ''}${value.location?.line ? `:${value.location.line}` : ''} — ${value.message}`,
    )));
    related.append(list);
    detail.append(related);
  }
  if (detail.childElementCount) item.append(detail);
  return item;
}
