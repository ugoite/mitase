export function translate(key) {
  return window.SyuPreferences?.t?.(key) || key;
}
