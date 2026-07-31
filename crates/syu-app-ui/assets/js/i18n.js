export function translate(key) {
  return window.SyuPreferences?.t?.(key) || key;
}

function normalizedEnum(value) {
  return String(value || 'unknown')
    .trim()
    .toLowerCase()
    .replace(/[\s-]+/g, '_');
}

/**
 * Translate a stable Workbench enum at the presentation boundary. Unknown
 * values remain visible as technical values so a new server enum cannot be
 * silently hidden before its catalog entry is added.
 */
export function localizeEnum(namespace, value, fallback = value) {
  const normalized = normalizedEnum(value);
  const key = `${namespace}.${normalized}`;
  const translated = translate(key);
  return translated === key ? (fallback ?? value) : translated;
}

/**
 * Built-in specification titles are product copy, while arbitrary workspace
 * titles remain user-authored source text and are never machine-translated.
 */
export function localizeSpecificationTitle(item) {
  if (!item?.id) return item?.title || '';
  const key = `specification.title.${item.id}`;
  const translated = translate(key);
  return translated === key ? (item.title || '') : translated;
}
