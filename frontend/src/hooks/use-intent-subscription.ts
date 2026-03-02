import { useState, useCallback, useMemo } from 'react';
import { useWebSocket, type WebSocketStatus } from './use-websocket';

const WS_URL = process.env.NEXT_PUBLIC_WS_URL || 'wss://api.rampos.io/v1/portal/ws';

export interface IntentStatusUpdate {
  intentId: string;
  state: string;
  previousState: string;
  updatedAt: string;
  metadata?: Record<string, unknown>;
}

export interface UseIntentSubscriptionReturn {
  intentStatus: IntentStatusUpdate | null;
  isSubscribed: boolean;
  status: WebSocketStatus;
  subscribe: (intentId: string) => void;
  unsubscribe: () => void;
}

interface WsIntentMessage {
  type: 'intent_status' | 'subscribed' | 'unsubscribed';
  payload: unknown;
}

export function useIntentSubscription(authToken?: string | null): UseIntentSubscriptionReturn {
  const [intentStatus, setIntentStatus] = useState<IntentStatusUpdate | null>(null);
  const [subscribedIntentId, setSubscribedIntentId] = useState<string | null>(null);

  const handleMessage = useCallback((data: unknown) => {
    const msg = data as WsIntentMessage;
    if (!msg || typeof msg !== 'object' || !('type' in msg)) return;

    switch (msg.type) {
      case 'intent_status': {
        const update = msg.payload as IntentStatusUpdate;
        if (update.intentId === subscribedIntentId) {
          setIntentStatus(update);
        }
        break;
      }
      case 'unsubscribed':
        setIntentStatus(null);
        break;
    }
  }, [subscribedIntentId]);

  const wsOptions = useMemo(() => ({
    url: subscribedIntentId ? WS_URL : undefined,
    onMessage: handleMessage,
    authToken: authToken ?? null,
    reconnect: true,
    maxReconnectAttempts: 5,
  }), [subscribedIntentId, handleMessage, authToken]);

  const { isConnected, status, sendMessage } = useWebSocket(wsOptions);

  const subscribe = useCallback((intentId: string) => {
    setSubscribedIntentId(intentId);
    setIntentStatus(null);
    if (isConnected) {
      sendMessage({ type: 'subscribe_intent', intentId });
    }
  }, [isConnected, sendMessage]);

  const unsubscribe = useCallback(() => {
    if (isConnected && subscribedIntentId) {
      sendMessage({ type: 'unsubscribe_intent', intentId: subscribedIntentId });
    }
    setSubscribedIntentId(null);
    setIntentStatus(null);
  }, [isConnected, sendMessage, subscribedIntentId]);

  return {
    intentStatus,
    isSubscribed: !!subscribedIntentId && isConnected,
    status,
    subscribe,
    unsubscribe,
  };
}
