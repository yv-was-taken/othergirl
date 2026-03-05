import { browser } from '$app/environment';
import { writable } from 'svelte/store';

import { setAuthToken } from '$lib/api';
import type { UserResponse } from '$lib/types';

export type SessionUser = Pick<
  UserResponse,
  'id' | 'username' | 'email' | 'avatar_url' | 'is_premium' | 'is_age_verified' | 'created_at'
>;

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

let lastWritten: string | null = browser
  ? localStorage.getItem('othergirl.session')
  : null;

auth.subscribe((state) => {
  setAuthToken(state.token);

  if (!browser || !state.ready) return;

  if (state.token && state.user) {
    const serialized = JSON.stringify({ token: state.token, user: state.user });
    if (serialized !== lastWritten) {
      localStorage.setItem('othergirl.session', serialized);
      lastWritten = serialized;
    }
  } else {
    if (lastWritten !== null) {
      localStorage.removeItem('othergirl.session');
      lastWritten = null;
    }
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
