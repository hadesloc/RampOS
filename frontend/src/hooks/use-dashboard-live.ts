import { useState, useCallback, useMemo } from 'react';
import { useWebSocket, type WebSocketStatus } from './use-websocket';
import type { DashboardStats, Intent } from '@/lib/api';

const WS_URL = process.env.NEXT_PUBLIC_WS_URL || 'wss://api.rampos.io/v1/portal/ws';

export interface DashboardLiveData {
  stats: DashboardStats | null;
  volume: DashboardStats['volume'] | null;
  recentIntents: Intent[];
}

export interface UseDashboardLiveReturn extends DashboardLiveData {
  isConnected: boolean;
  status: WebSocketStatus;
  lastUpdate: Date | null;
}

interface WsDashboardMessage {
  type: 'dashboard_update' | 'stats_update' | 'volume_update' | 'intent_created' | 'intent_updated';
  payload: unknown;
}

export function useDashboardLive(authToken?: string | null): UseDashboardLiveReturn {
  const [stats, setStats] = useState<DashboardStats | null>(null);
  const [volume, setVolume] = useState<DashboardStats['volume'] | null>(null);
  const [recentIntents, setRecentIntents] = useState<Intent[]>([]);
  const [lastUpdate, setLastUpdate] = useState<Date | null>(null);

  const handleMessage = useCallback((data: unknown) => {
    const msg = data as WsDashboardMessage;
    if (!msg || typeof msg !== 'object' || !('type' in msg)) return;

    setLastUpdate(new Date());

    switch (msg.type) {
      case 'dashboard_update':
      case 'stats_update':
        setStats(msg.payload as DashboardStats);
        break;
      case 'volume_update':
        setVolume(msg.payload as DashboardStats['volume']);
        break;
      case 'intent_created':
      case 'intent_updated': {
        const intent = msg.payload as Intent;
        setRecentIntents((prev) => {
          const filtered = prev.filter((i) => i.id !== intent.id);
          return [intent, ...filtered].slice(0, 20);
        });
        break;
      }
    }
  }, []);

  const wsOptions = useMemo(() => ({
    url: WS_URL,
    onMessage: handleMessage,
    authToken: authToken ?? null,
    maxReconnectAttempts: 10,
    reconnectInterval: 2000,
  }), [handleMessage, authToken]);

  const { isConnected, status } = useWebSocket(wsOptions);

  return { stats, volume, recentIntents, isConnected, status, lastUpdate };
}
