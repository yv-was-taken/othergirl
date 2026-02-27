import { API_BASE_URL } from '$lib/config';

const API_BASE = API_BASE_URL;
let authToken: string | null = null;

export function setAuthToken(token: string | null) {
  authToken = token;
}

export async function apiFetch<T>(path: string, init: RequestInit = {}): Promise<T> {
  const headers = new Headers(init.headers);
  const isFormData = typeof FormData !== 'undefined' && init.body instanceof FormData;
  const isUrlEncoded = typeof URLSearchParams !== 'undefined' && init.body instanceof URLSearchParams;
  const isStringBody = typeof init.body === 'string';

  if (!headers.has('Content-Type') && init.body && !isFormData && !isUrlEncoded && !isStringBody) {
    headers.set('Content-Type', 'application/json');
  }

  if (authToken) {
    headers.set('Authorization', `Bearer ${authToken}`);
  }

  const response = await fetch(resolveUrl(path), {
    ...init,
    headers
  });

  if (!response.ok) {
    const fallback = `Request failed (${response.status})`;
    const text = await response.text();

    let errorMessage = fallback;
    try {
      const parsed = JSON.parse(text) as { error?: string };
      errorMessage = parsed.error ?? fallback;
    } catch {
      errorMessage = text || fallback;
    }
    throw new Error(errorMessage);
  }

  if (response.status === 204) {
    return undefined as T;
  }

  return (await response.json()) as T;
}

function resolveUrl(path: string): string {
  if (path.startsWith('http://') || path.startsWith('https://')) {
    return path;
  }

  return `${API_BASE}${path.startsWith('/') ? path : `/${path}`}`;
}
