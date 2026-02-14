import { apiFetch } from '$lib/api';

type EmoteRecord = Record<string, string>;

const DEFAULT_EMOTES: EmoteRecord = {
  ':wave:': '/emotes/wave.png',
  ':spark:': '/emotes/spark.png'
};

let emoteMap: EmoteRecord = { ...DEFAULT_EMOTES };
let loaded = false;

export async function loadEmotes() {
  if (loaded) return;

  try {
    const response = await apiFetch<{
      emotes: { token: string; image_url: string; is_active: boolean }[];
    }>('/api/emotes');

    const fetched = response.emotes
      .filter((emote) => emote.is_active && Boolean(emote.token) && Boolean(emote.image_url))
      .reduce<EmoteRecord>((next, emote) => {
        next[emote.token] = emote.image_url;
        return next;
      }, {});

    emoteMap = { ...DEFAULT_EMOTES, ...fetched };
  } catch {
    emoteMap = { ...DEFAULT_EMOTES };
  } finally {
    loaded = true;
  }
}

export function applyEmotes(input: string): string {
  return Object.entries(emoteMap).reduce((next, [token, url]) => {
    return next.replaceAll(
      token,
      `<img src="${url}" alt="${token}" class="inline-block h-5 w-5 align-middle" />`
    );
  }, input);
}
