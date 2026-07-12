/** Canonical Workbench API client. Semantic decisions remain server-side. */
export async function request(url, options = {}) {
  const response = await fetch(url, {
    headers: { 'content-type': 'application/json', ...(options.headers || {}) },
    ...options,
  });
  const text = await response.text();
  let body;
  try { body = text ? JSON.parse(text) : null; } catch { body = text; }
  if (!response.ok) throw new Error(body?.error || String(body));
  return body;
}

export const readProjection = () => request('/api/projection');
export const validateWork = basis => request('/api/work/validate', {
  method: 'POST', body: JSON.stringify(basis),
});
