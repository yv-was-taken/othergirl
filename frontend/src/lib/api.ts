const API_BASE = import.meta.env.PUBLIC_API_BASE_URL ?? 'http://localhost:8080';
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

    try {
      const parsed = JSON.parse(text) as { error?: string };
      throw new Error(parsed.error ?? fallback);
    } catch {
      throw new Error(text || fallback);
    }
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
