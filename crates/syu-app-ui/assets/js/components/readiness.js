import { localizeEnum, translate } from '../i18n.js';

export function renderReadiness(readiness) {
  const item = document.createElement('output');
  item.textContent = `${translate('readiness.target')}: ${localizeEnum('readiness.target', readiness.target)} · ${localizeEnum('status', readiness.status)} (${localizeEnum('readiness.execution', readiness.execution_state || 'not-run')})`;
  return item;
}
