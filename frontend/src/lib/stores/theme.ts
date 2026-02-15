import { browser } from '$app/environment';
import { writable } from 'svelte/store';

type Theme = 'light' | 'dark';

const STORAGE_KEY = 'othergirl.theme';

function getInitialTheme(): Theme {
  if (!browser) return 'dark';

  const stored = localStorage.getItem(STORAGE_KEY);
  if (stored === 'light' || stored === 'dark') return stored;

  if (window.matchMedia('(prefers-color-scheme: light)').matches) return 'light';

  return 'dark';
}

export const theme = writable<Theme>(getInitialTheme());

theme.subscribe((value) => {
  if (!browser) return;
  document.documentElement.setAttribute('data-theme', value);
  localStorage.setItem(STORAGE_KEY, value);
});

export function toggleTheme() {
  theme.update((current) => (current === 'dark' ? 'light' : 'dark'));
}
