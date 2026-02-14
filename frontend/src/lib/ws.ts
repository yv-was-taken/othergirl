const API_BASE = import.meta.env.PUBLIC_API_BASE_URL ?? 'http://localhost:8080';

type WsEvent = {
  type: string;
  [key: string]: unknown;
};

type WsHandlers = {
  onEvent: (event: WsEvent) => void;
  onOpen?: () => void;
  onClose?: () => void;
  onError?: (error: Event) => void;
};

export type ChatSocket = {
  send: (event: Record<string, unknown>) => void;
  close: () => void;
};

export function connectChatSocket(chatId: string, token: string, handlers: WsHandlers): ChatSocket {
  let socket: WebSocket | null = null;
  let retries = 0;
  let manualClose = false;

  const maxRetries = 5;

  const connect = () => {
    const url = buildWsUrl(chatId, token);
    socket = new WebSocket(url);

    socket.onopen = () => {
      retries = 0;
      handlers.onOpen?.();
    };

    socket.onmessage = (message) => {
      try {
        const data = JSON.parse(message.data) as WsEvent;
        handlers.onEvent(data);
      } catch {
        handlers.onEvent({ type: 'error', code: 'PARSE_ERROR', message: 'Invalid WS payload' });
      }
    };

    socket.onerror = (error) => {
      handlers.onError?.(error);
    };

    socket.onclose = () => {
      handlers.onClose?.();

      if (manualClose || retries >= maxRetries) {
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

function buildWsUrl(chatId: string, token: string): string {
  const base = API_BASE.replace('http://', 'ws://').replace('https://', 'wss://');
  const url = new URL(`${base}/api/chat`);

  url.searchParams.set('token', token);
  url.searchParams.set('chat_id', chatId);

  return url.toString();
}
