export function translate(key) {
  return window.MitasePreferences?.t?.(key) || key;
}

function lookup(key) {
  return window.MitasePreferences?.lookup?.(key);
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
  const translated = lookup(key);
  return translated === undefined ? (fallback ?? value) : translated;
}

/**
 * Built-in specification titles are product copy, while arbitrary workspace
 * titles remain user-authored source text and are never machine-translated.
 */
export function localizeSpecificationTitle(item) {
  const key = item?.presentation_title_key;
  if (!key) return item?.title || '';
  const translated = lookup(key);
  return translated === undefined ? (item.title || '') : translated;
}
