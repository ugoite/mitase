/** The browser only transports server-owned DTOs and opaque mutation tokens. */
let csrfToken = '';

export async function request(url, options = {}) {
  const method = (options.method || 'GET').toUpperCase();
  const response = await fetch(url, {
    ...options,
    headers: {
      accept: 'application/json',
      'content-type': 'application/json',
      ...(csrfToken && method !== 'GET'
        ? { 'x-syu-csrf-token': csrfToken }
        : {}),
      ...(options.headers || {}),
    },
  });
  const receivedToken = response.headers?.get?.('x-syu-csrf-token');
  if (receivedToken) csrfToken = receivedToken;
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

export function createModifyWork(projection, criterion) {
  return post('/api/work/request', {
    basis: mutationBasis(projection),
    request: {
      schema: 'syu/work-request/v1',
      id: `WORK-WORKBENCH-${Date.now()}`,
      summary: `Modify ${criterion}`,
      operation: 'modify',
      seeds: [criterion],
      constraints: {},
      requested_targets: [],
    },
  });
}

export const planWork = (projection) => post('/api/work/plan', mutationBasis(projection));

export const exportContext = (projection, sliceId) => post('/api/work/context', {
  basis: mutationBasis(projection),
  slice_id: sliceId,
});

export const validateWork = (projection) => post('/api/work/validate', mutationBasis(projection));

export const verifyWork = (projection, sliceId) => post('/api/work/verify', {
  basis: mutationBasis(projection),
  slice_id: sliceId,
});

export const validateResult = (projection, receipt) => post('/api/work/result', {
  basis: mutationBasis(projection),
  receipt,
});
