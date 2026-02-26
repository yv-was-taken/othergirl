import { writable } from 'svelte/store';

export type ChatMessage = {
  id: string;
  chat_id?: string;
  sender_id: string;
  content: string;
  timestamp: string;
  flagged?: boolean;
  is_read?: boolean;
};

type ChatState = {
  chatId: string | null;
  partner: {
    id: string;
    username: string;
    bio?: string;
    badges?: string[];
    interests?: string[];
    flare?: Record<string, unknown>;
  } | null;
  messages: ChatMessage[];
  queued: boolean;
  queuePosition: number | null;
  queueWaitSeconds: number | null;
  partnerTyping: boolean;
  cooldownSeconds: number;
};

const initial: ChatState = {
  chatId: null,
  partner: null,
  messages: [],
  queued: false,
  queuePosition: null,
  queueWaitSeconds: null,
  partnerTyping: false,
  cooldownSeconds: 0
};

export const chat = writable<ChatState>(initial);

export function setQueue(position: number, waitSeconds: number) {
  chat.update((state) => ({
    ...state,
    queued: true,
    queuePosition: position,
    queueWaitSeconds: waitSeconds
  }));
}

export function setMatch(
  chatId: string,
  partner: {
    id: string;
    username: string;
    bio?: string;
    badges?: string[];
    interests?: string[];
    flare?: Record<string, unknown>;
  }
) {
  chat.update((state) => ({
    ...state,
    chatId,
    partner,
    queued: false,
    queuePosition: null,
    queueWaitSeconds: null,
    messages: [],
    partnerTyping: false,
    cooldownSeconds: 0
  }));
}

const MAX_MESSAGES = 200;

export function pushMessage(message: ChatMessage) {
  chat.update((state) => {
    const updated = [...state.messages, message];
    if (updated.length > MAX_MESSAGES) {
      updated.splice(0, updated.length - MAX_MESSAGES);
    }
    return { ...state, messages: updated };
  });
}

export function setPartnerTyping(isTyping: boolean) {
  chat.update((state) => ({ ...state, partnerTyping: isTyping }));
}

export function setCooldown(seconds: number) {
  chat.update((state) => ({ ...state, cooldownSeconds: seconds }));
}

export function resetChatState() {
  chat.set(initial);
}
