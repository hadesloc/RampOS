import { useEffect, useState, useRef, useCallback } from 'react';

type WebSocketStatus = 'CONNECTING' | 'OPEN' | 'CLOSING' | 'CLOSED';

interface UseWebSocketOptions {
  url?: string;
  onMessage?: (event: MessageEvent) => void;
  onOpen?: (event: Event) => void;
  onClose?: (event: CloseEvent) => void;
  onError?: (event: Event) => void;
  reconnectInterval?: number;
  maxReconnectAttempts?: number;
}

export function useWebSocket({
  url,
  onMessage,
  onOpen,
  onClose,
  onError,
  reconnectInterval = 3000,
  maxReconnectAttempts = 5,
}: UseWebSocketOptions) {
  const [status, setStatus] = useState<WebSocketStatus>('CLOSED');
  const ws = useRef<WebSocket | null>(null);
  const reconnectAttempts = useRef(0);
  const reconnectTimer = useRef<NodeJS.Timeout>();

  const connect = useCallback(() => {
    if (!url) return;

    try {
      setStatus('CONNECTING');
      ws.current = new WebSocket(url);

      ws.current.onopen = (event) => {
        setStatus('OPEN');
        reconnectAttempts.current = 0;
        onOpen?.(event);
      };

      ws.current.onclose = (event) => {
        setStatus('CLOSED');
        onClose?.(event);

        if (reconnectAttempts.current < maxReconnectAttempts) {
          reconnectTimer.current = setTimeout(() => {
            reconnectAttempts.current++;
            connect();
          }, reconnectInterval);
        }
      };

      ws.current.onerror = (event) => {
        onError?.(event);
      };

      ws.current.onmessage = (event) => {
        onMessage?.(event);
      };
    } catch (error) {
      console.error('WebSocket connection error:', error);
      setStatus('CLOSED');
    }
  }, [url, onMessage, onOpen, onClose, onError, reconnectInterval, maxReconnectAttempts]);

  useEffect(() => {
    connect();

    return () => {
      if (ws.current) {
        ws.current.close();
      }
      if (reconnectTimer.current) {
        clearTimeout(reconnectTimer.current);
      }
    };
  }, [connect]);

  const sendMessage = useCallback((data: string | ArrayBufferLike | Blob | ArrayBufferView) => {
    if (ws.current?.readyState === WebSocket.OPEN) {
      ws.current.send(data);
    } else {
      console.warn('WebSocket is not open. Unable to send message.');
    }
  }, []);

  return { status, sendMessage };
}

// Specialized hook for dashboard real-time updates
export function useRealtimeDashboard() {
  const [lastUpdate, setLastUpdate] = useState<Date | null>(null);

  // In a real app, this would point to the actual WS endpoint
  // For now, we'll simulate connection to a dummy endpoint or allow it to fail gracefully
  const wsUrl = process.env.NEXT_PUBLIC_WS_URL || 'wss://api.rampos.io/v1/ws/dashboard';

  const { status } = useWebSocket({
    url: wsUrl,
    onMessage: (event) => {
      try {
        const data = JSON.parse(event.data);
        if (data.type === 'dashboard_update') {
          setLastUpdate(new Date());
        }
      } catch (e) {
        console.error('Failed to parse WS message', e);
      }
    },
    // Don't reconnect too aggressively for this demo
    maxReconnectAttempts: 3
  });

  return { isConnected: status === 'OPEN', lastUpdate };
}
