import { writable } from 'svelte/store';

type UiState = {
  keepVoteOpen: boolean;
  loading: boolean;
};

const initial: UiState = {
  keepVoteOpen: false,
  loading: false
};

export const ui = writable<UiState>(initial);

export function setLoading(loading: boolean) {
  ui.update((state) => ({ ...state, loading }));
}

export function openKeepVote() {
  ui.update((state) => ({ ...state, keepVoteOpen: true }));
}

export function closeKeepVote() {
  ui.update((state) => ({ ...state, keepVoteOpen: false }));
}
