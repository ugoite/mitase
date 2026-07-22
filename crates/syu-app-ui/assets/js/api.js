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
export const establishSession = () => request('/api/work/session');
export const readSource = path => request(`/api/source?path=${encodeURIComponent(path)}`);
export const readTargetSource = target => request(`/api/source?target=${encodeURIComponent(target)}`);

export const searchSpecificationCandidates = (query = '', kind = '') => {
  const params = new URLSearchParams();
  if (query.trim()) params.set('q', query.trim());
  if (kind && kind !== 'all') params.set('kind', kind);
  params.set('limit', '100');
  return request(`/api/specifications/candidates?${params.toString()}`);
};

export const previewSpecificationCandidate = (projection, patch) => post(
  '/api/specifications/candidates/preview',
  { basis: mutationBasis(projection), patch, preview_token: null },
);

export const applySpecificationCandidate = (projection, patch, previewToken) => request(
  '/api/specifications/candidates/apply',
  {
    method: 'PUT',
    body: JSON.stringify({
      basis: mutationBasis(projection),
      patch,
      preview_token: previewToken,
    }),
  },
);

export const readTargetSuggestions = criterion => request(
  `/api/specifications/${encodeURIComponent(criterion)}/target-suggestions`,
);

export const rejectTargetSuggestion = (projection, criterion, suggestionToken, suggestionId) => post(
  `/api/specifications/${encodeURIComponent(criterion)}/target-suggestions/reject`,
  {
    basis: mutationBasis(projection),
    suggestion_token: suggestionToken,
    suggestion_id: suggestionId,
  },
);

export const approveTargetSuggestions = (projection, criterion, suggestionToken, suggestionIds) => post(
  `/api/specifications/${encodeURIComponent(criterion)}/target-suggestions/approve`,
  {
    basis: mutationBasis(projection),
    suggestion_token: suggestionToken,
    suggestion_ids: suggestionIds,
  },
);

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

export const runJourneyAction = (projection, action) => post('/api/work/action', {
  basis: mutationBasis(projection),
  ...action,
});
