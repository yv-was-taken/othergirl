const API_BASE = import.meta.env.PUBLIC_API_BASE_URL ?? 'http://localhost:8080';

// Mirrors backend AuthResponse enum (serde tag="status", rename_all="snake_case")
type AuthResponse =
  | { status: 'ok' }
  | { status: 'failed'; code: string; message: string };

// Mirrors backend ServerEvent enum (serde tag="type", rename_all="snake_case")
export type ServerEvent =
  | { type: 'queued'; position: number; queue: string }
  | { type: 'matched'; chat_id: string; partner: { id: string; username: string; bio?: string; badges?: string[]; interests?: string[]; flare?: Record<string, unknown> } }
  | { type: 'message'; id: string; chat_id: string; sender_id: string; content: string; timestamp: string; flagged: boolean }
  | { type: 'typing'; user_id: string; is_typing: boolean }
  | { type: 'read_receipt'; message_id: string; reader_id: string }
  | { type: 'partner_keep_vote' }
  | { type: 'keep_result'; both_kept: boolean }
  | { type: 'award_received'; recipient_id: string; award_type: string; spark_amount: number; your_cut: number }
  | { type: 'partner_left'; user_id: string }
  | { type: 'cooldown'; seconds: number }
  | { type: 'error'; code: string; message: string };

type WsHandlers = {
  onEvent: (event: ServerEvent) => void;
  onOpen?: () => void;
  onClose?: () => void;
  onError?: (error: Event) => void;
};

export type ChatSocket = {
  send: (event: Record<string, unknown>) => void;
  close: () => void;
};

export function connectChatSocket(chatId: string, getToken: () => string | null, handlers: WsHandlers): ChatSocket {
  let socket: WebSocket | null = null;
  let retries = 0;
  let manualClose = false;

  const maxRetries = 5;

  const connect = () => {
    const token = getToken();
    if (!token) {
      handlers.onEvent({ type: 'error', code: 'NO_TOKEN', message: 'No auth token available' });
      return;
    }

    const url = buildWsUrl();
    socket = new WebSocket(url);
    let authenticated = false;

    socket.onopen = () => {
      socket!.send(JSON.stringify({ type: 'auth', token, chat_id: chatId }));
    };

    socket.onmessage = (message) => {
      try {
        if (!authenticated) {
          const response = JSON.parse(message.data) as AuthResponse;
          switch (response.status) {
            case 'ok':
              authenticated = true;
              retries = 0;
              handlers.onOpen?.();
              break;
            case 'failed':
              manualClose = true;
              handlers.onEvent({ type: 'error', code: response.code, message: response.message });
              break;
          }
          return;
        }

        const event = JSON.parse(message.data) as ServerEvent;
        handlers.onEvent(event);
      } catch {
        handlers.onEvent({ type: 'error', code: 'PARSE_ERROR', message: 'Invalid WS payload' });
      }
    };

    socket.onerror = (error) => {
      handlers.onError?.(error);
    };

    socket.onclose = () => {
      handlers.onClose?.();

      if (manualClose) return;

      if (retries >= maxRetries) {
        handlers.onEvent({ type: 'error', code: 'CONNECTION_LOST', message: 'Unable to connect. Please refresh the page.' });
        return;
      }

      retries += 1;
      setTimeout(connect, 750 * retries);
    };
  };

  connect();

  return {
    send(event) {
      if (!socket || socket.readyState !== WebSocket.OPEN) return;
      socket.send(JSON.stringify(event));
    },
    close() {
      manualClose = true;
      socket?.close();
      socket = null;
    }
  };
}

function buildWsUrl(): string {
  const base = API_BASE.replace('http://', 'ws://').replace('https://', 'wss://');
  return `${base}/api/chat`;
}
