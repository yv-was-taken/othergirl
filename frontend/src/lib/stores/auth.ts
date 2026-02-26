import { browser } from '$app/environment';
import { writable } from 'svelte/store';

import { setAuthToken } from '$lib/api';

export type SessionUser = {
  id: string;
  username: string;
  email?: string | null;
  is_premium: boolean;
  is_age_verified: boolean;
  created_at: string;
};

type AuthState = {
  token: string | null;
  user: SessionUser | null;
  ready: boolean;
};

const initial = readInitialState();

export const auth = writable<AuthState>(initial);

// Clean up legacy standalone token key
if (browser) {
  localStorage.removeItem('othergirl.token');
}

auth.subscribe((state) => {
  setAuthToken(state.token);

  if (!browser || !state.ready) return;

  if (state.token && state.user) {
    localStorage.setItem('othergirl.session', JSON.stringify({ token: state.token, user: state.user }));
  } else {
    localStorage.removeItem('othergirl.session');
  }
});

export function setSession(token: string, user: SessionUser) {
  auth.set({ token, user, ready: true });
}

export function clearSession() {
  auth.set({ token: null, user: null, ready: true });
}

function readInitialState(): AuthState {
  if (!browser) {
    return { token: null, user: null, ready: true };
  }

  const raw = localStorage.getItem('othergirl.session');
  if (!raw) {
    return { token: null, user: null, ready: true };
  }

  try {
    const parsed = JSON.parse(raw) as { token?: string; user?: SessionUser };
    if (!parsed.token || !parsed.user) {
      return { token: null, user: null, ready: true };
    }

    return {
      token: parsed.token,
      user: parsed.user,
      ready: true
    };
  } catch {
    return { token: null, user: null, ready: true };
  }
}
