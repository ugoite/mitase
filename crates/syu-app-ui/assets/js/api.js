/** The browser only transports server-owned DTOs and opaque mutation tokens. */
export async function request(url, options = {}) {
  const response = await fetch(url, {
    headers: { accept: 'application/json', 'content-type': 'application/json', ...(options.headers || {}) },
    ...options,
  });
  const text = await response.text();
  let body;
  try { body = text ? JSON.parse(text) : null; } catch { body = text; }
  if (!response.ok) throw new Error(body?.error || String(body || response.status));
  return body;
}

export const readProjection = () => request('/api/projection');

export function mutationBasis(projection) {
  const snapshot = projection?.snapshot || {};
  return {
    expected_revision: snapshot.revision || '',
    expected_workspace_fingerprint: snapshot.fingerprint || '',
    expected_source_hash: snapshot.source_hash || '',
  };
}

export const post = (url, payload) => request(url, {
  method: 'POST',
  body: JSON.stringify(payload),
});

export const validateWork = (projection) => post('/api/work/validate', mutationBasis(projection));
