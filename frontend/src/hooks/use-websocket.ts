import { useEffect, useState, useRef, useCallback } from 'react';

export type WebSocketStatus = 'connecting' | 'connected' | 'disconnected' | 'error';

export interface UseWebSocketOptions {
  url?: string;
  protocols?: string | string[];
  onMessage?: (data: unknown) => void;
  onOpen?: (event: Event) => void;
  onClose?: (event: CloseEvent) => void;
  onError?: (event: Event) => void;
  reconnect?: boolean;
  reconnectInterval?: number;
  maxReconnectAttempts?: number;
  authToken?: string | null;
}

export interface UseWebSocketReturn {
  status: WebSocketStatus;
  isConnected: boolean;
  sendMessage: (data: string | object) => void;
  disconnect: () => void;
  reconnect: () => void;
  lastMessage: unknown | null;
}

export function useWebSocket({
  url,
  protocols,
  onMessage,
  onOpen,
  onClose,
  onError,
  reconnect: shouldReconnect = true,
  reconnectInterval = 1000,
  maxReconnectAttempts = 5,
  authToken,
}: UseWebSocketOptions): UseWebSocketReturn {
  const [status, setStatus] = useState<WebSocketStatus>('disconnected');
  const [lastMessage, setLastMessage] = useState<unknown | null>(null);
  const ws = useRef<WebSocket | null>(null);
  const reconnectAttempts = useRef(0);
  const reconnectTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const manualDisconnect = useRef(false);

  const onMessageRef = useRef(onMessage);
  const onOpenRef = useRef(onOpen);
  const onCloseRef = useRef(onClose);
  const onErrorRef = useRef(onError);

  useEffect(() => { onMessageRef.current = onMessage; }, [onMessage]);
  useEffect(() => { onOpenRef.current = onOpen; }, [onOpen]);
  useEffect(() => { onCloseRef.current = onClose; }, [onClose]);
  useEffect(() => { onErrorRef.current = onError; }, [onError]);

  const getBackoffDelay = useCallback((attempt: number) => {
    return Math.min(reconnectInterval * Math.pow(2, attempt), 30000);
  }, [reconnectInterval]);

  const connect = useCallback(() => {
    if (!url) return;

    if (ws.current) {
      ws.current.onopen = null;
      ws.current.onclose = null;
      ws.current.onerror = null;
      ws.current.onmessage = null;
      ws.current.close();
      ws.current = null;
    }

    try {
      const wsUrl = authToken ? `${url}${url.includes('?') ? '&' : '?'}token=${authToken}` : url;
      setStatus('connecting');
      ws.current = new WebSocket(wsUrl, protocols);

      ws.current.onopen = (event) => {
        setStatus('connected');
        reconnectAttempts.current = 0;
        onOpenRef.current?.(event);
      };

      ws.current.onclose = (event) => {
        setStatus('disconnected');
        onCloseRef.current?.(event);

        if (!manualDisconnect.current && shouldReconnect && reconnectAttempts.current < maxReconnectAttempts) {
          const delay = getBackoffDelay(reconnectAttempts.current);
          reconnectTimer.current = setTimeout(() => {
            reconnectAttempts.current++;
            connect();
          }, delay);
        }
      };

      ws.current.onerror = (event) => {
        setStatus('error');
        onErrorRef.current?.(event);
      };

      ws.current.onmessage = (event) => {
        try {
          const data = JSON.parse(event.data);
          setLastMessage(data);
          onMessageRef.current?.(data);
        } catch {
          setLastMessage(event.data);
          onMessageRef.current?.(event.data);
        }
      };
    } catch {
      setStatus('error');
    }
  }, [url, protocols, authToken, shouldReconnect, maxReconnectAttempts, getBackoffDelay]);

  const disconnect = useCallback(() => {
    manualDisconnect.current = true;
    if (reconnectTimer.current) {
      clearTimeout(reconnectTimer.current);
      reconnectTimer.current = null;
    }
    if (ws.current) {
      ws.current.close();
      ws.current = null;
    }
    setStatus('disconnected');
  }, []);

  const reconnectFn = useCallback(() => {
    manualDisconnect.current = false;
    reconnectAttempts.current = 0;
    connect();
  }, [connect]);

  const sendMessage = useCallback((data: string | object) => {
    if (ws.current?.readyState === WebSocket.OPEN) {
      const payload = typeof data === 'string' ? data : JSON.stringify(data);
      ws.current.send(payload);
    }
  }, []);

  useEffect(() => {
    manualDisconnect.current = false;
    connect();

    return () => {
      manualDisconnect.current = true;
      if (ws.current) {
        ws.current.close();
        ws.current = null;
      }
      if (reconnectTimer.current) {
        clearTimeout(reconnectTimer.current);
        reconnectTimer.current = null;
      }
    };
  }, [connect]);

  return {
    status,
    isConnected: status === 'connected',
    sendMessage,
    disconnect,
    reconnect: reconnectFn,
    lastMessage,
  };
}

export function useRealtimeDashboard() {
  const [lastUpdate, setLastUpdate] = useState<Date | null>(null);
  const { isConnected, lastMessage } = useWebSocket({
    url: typeof window !== 'undefined' ? `ws://${window.location.hostname}:8080/v1/ws` : '',
    reconnect: true,
  });

  useEffect(() => {
    if (lastMessage) {
      setLastUpdate(new Date());
    }
  }, [lastMessage]);

  return { isConnected, lastUpdate };
}
