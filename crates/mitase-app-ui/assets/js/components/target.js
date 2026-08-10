import { localizeEnum } from '../i18n.js';

export function renderTarget(target) {
  const item = document.createElement('li');
  item.textContent = `${target.reference} · ${localizeEnum('target.access', target.access)} · ${target.path || target.resolved_path || ''}`;
  return item;
}
