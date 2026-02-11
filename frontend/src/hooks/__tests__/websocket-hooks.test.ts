import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { renderHook, act } from '@testing-library/react';

// ─── WebSocket mock ───────────────────────────────────────────────────
type WSHandler = ((...args: unknown[]) => void) | null;

class MockWebSocket {
  static CONNECTING = 0;
  static OPEN = 1;
  static CLOSING = 2;
  static CLOSED = 3;
  static instances: MockWebSocket[] = [];

  url: string;
  protocols?: string | string[];
  readyState = MockWebSocket.CONNECTING;
  onopen: WSHandler = null;
  onclose: WSHandler = null;
  onerror: WSHandler = null;
  onmessage: WSHandler = null;

  sent: string[] = [];

  constructor(url: string, protocols?: string | string[]) {
    this.url = url;
    this.protocols = protocols;
    MockWebSocket.instances.push(this);
  }

  send(data: string) {
    this.sent.push(data);
  }

  close() {
    this.readyState = MockWebSocket.CLOSED;
    if (this.onclose) {
      (this.onclose as (e: Partial<CloseEvent>) => void)({ code: 1000, reason: '', wasClean: true } as CloseEvent);
    }
  }

  // Test helpers
  simulateOpen() {
    this.readyState = MockWebSocket.OPEN;
    if (this.onopen) {
      (this.onopen as (e: Event) => void)(new Event('open'));
    }
  }

  simulateMessage(data: unknown) {
    if (this.onmessage) {
      (this.onmessage as (e: Partial<MessageEvent>) => void)({
        data: typeof data === 'string' ? data : JSON.stringify(data),
      } as MessageEvent);
    }
  }

  simulateError() {
    if (this.onerror) {
      (this.onerror as (e: Event) => void)(new Event('error'));
    }
  }

  simulateClose(code = 1000) {
    this.readyState = MockWebSocket.CLOSED;
    if (this.onclose) {
      (this.onclose as (e: Partial<CloseEvent>) => void)({ code, reason: '', wasClean: code === 1000 } as CloseEvent);
    }
  }
}

// ─── Global setup ─────────────────────────────────────────────────────
const originalWebSocket = globalThis.WebSocket;

beforeEach(() => {
  vi.useFakeTimers();
  MockWebSocket.instances = [];
  (globalThis as Record<string, unknown>).WebSocket = MockWebSocket as unknown as typeof WebSocket;
});

afterEach(() => {
  vi.useRealTimers();
  vi.restoreAllMocks();
  (globalThis as Record<string, unknown>).WebSocket = originalWebSocket;
});

function getLatestWs(): MockWebSocket {
  return MockWebSocket.instances[MockWebSocket.instances.length - 1];
}

// ═════════════════════════════════════════════════════════════════════
// useWebSocket tests
// ═════════════════════════════════════════════════════════════════════
describe('useWebSocket', () => {
  // dynamic import so the mock WebSocket is in place
  async function importHook() {
    return (await import('../use-websocket')).useWebSocket;
  }

  it('should connect to given URL', async () => {
    const useWebSocket = await importHook();
    const { result } = renderHook(() =>
      useWebSocket({ url: 'ws://localhost:8080/ws' }),
    );

    expect(result.current.status).toBe('connecting');
    expect(MockWebSocket.instances.length).toBe(1);
    expect(getLatestWs().url).toBe('ws://localhost:8080/ws');
  });

  it('should set status to connected on open', async () => {
    const useWebSocket = await importHook();
    const { result } = renderHook(() =>
      useWebSocket({ url: 'ws://localhost/ws' }),
    );

    act(() => {
      getLatestWs().simulateOpen();
    });

    expect(result.current.status).toBe('connected');
    expect(result.current.isConnected).toBe(true);
  });

  it('should append auth token to URL', async () => {
    const useWebSocket = await importHook();
    renderHook(() =>
      useWebSocket({ url: 'ws://localhost/ws', authToken: 'tok-123' }),
    );

    expect(getLatestWs().url).toBe('ws://localhost/ws?token=tok-123');
  });

  it('should append auth token with & when URL has query params', async () => {
    const useWebSocket = await importHook();
    renderHook(() =>
      useWebSocket({ url: 'ws://localhost/ws?v=2', authToken: 'abc' }),
    );

    expect(getLatestWs().url).toBe('ws://localhost/ws?v=2&token=abc');
  });

  it('should not connect if url is undefined', async () => {
    const useWebSocket = await importHook();
    const { result } = renderHook(() =>
      useWebSocket({ url: undefined }),
    );

    expect(MockWebSocket.instances.length).toBe(0);
    expect(result.current.status).toBe('disconnected');
  });

  it('should receive and parse JSON messages', async () => {
    const useWebSocket = await importHook();
    const onMessage = vi.fn();
    const { result } = renderHook(() =>
      useWebSocket({ url: 'ws://localhost/ws', onMessage }),
    );

    act(() => {
      getLatestWs().simulateOpen();
    });

    act(() => {
      getLatestWs().simulateMessage({ type: 'ping', value: 42 });
    });

    expect(onMessage).toHaveBeenCalledWith({ type: 'ping', value: 42 });
    expect(result.current.lastMessage).toEqual({ type: 'ping', value: 42 });
  });

  it('should send JSON messages', async () => {
    const useWebSocket = await importHook();
    const { result } = renderHook(() =>
      useWebSocket({ url: 'ws://localhost/ws' }),
    );

    act(() => {
      getLatestWs().simulateOpen();
    });

    act(() => {
      result.current.sendMessage({ action: 'subscribe' });
    });

    expect(getLatestWs().sent).toEqual(['{"action":"subscribe"}']);
  });

  it('should send string messages as-is', async () => {
    const useWebSocket = await importHook();
    const { result } = renderHook(() =>
      useWebSocket({ url: 'ws://localhost/ws' }),
    );

    act(() => {
      getLatestWs().simulateOpen();
    });

    act(() => {
      result.current.sendMessage('hello');
    });

    expect(getLatestWs().sent).toEqual(['hello']);
  });

  it('should reconnect with exponential backoff', async () => {
    const useWebSocket = await importHook();
    renderHook(() =>
      useWebSocket({ url: 'ws://localhost/ws', reconnectInterval: 1000, maxReconnectAttempts: 3 }),
    );

    act(() => {
      getLatestWs().simulateOpen();
    });

    // Connection drops - record count before reconnect timer fires
    act(() => {
      getLatestWs().simulateClose(1006);
    });

    const countAfterClose = MockWebSocket.instances.length;

    // First reconnect: 1000ms * 2^0 = 1000ms
    act(() => {
      vi.advanceTimersByTime(999);
    });
    // Not yet
    expect(MockWebSocket.instances.length).toBe(countAfterClose);

    act(() => {
      vi.advanceTimersByTime(1);
    });
    // Now reconnected
    expect(MockWebSocket.instances.length).toBe(countAfterClose + 1);

    // Second drop & reconnect: 1000ms * 2^1 = 2000ms
    act(() => {
      getLatestWs().simulateClose(1006);
    });
    const countAfterClose2 = MockWebSocket.instances.length;

    act(() => {
      vi.advanceTimersByTime(1999);
    });
    expect(MockWebSocket.instances.length).toBe(countAfterClose2);

    act(() => {
      vi.advanceTimersByTime(1);
    });
    expect(MockWebSocket.instances.length).toBe(countAfterClose2 + 1);
  });

  it('should stop reconnecting after max attempts', async () => {
    const useWebSocket = await importHook();
    renderHook(() =>
      useWebSocket({ url: 'ws://localhost/ws', reconnectInterval: 100, maxReconnectAttempts: 2 }),
    );

    // Attempt 1 (initial) -> close -> attempt 2 -> close -> attempt 3 -> close -> no more
    for (let i = 0; i < 3; i++) {
      act(() => {
        getLatestWs().simulateClose(1006);
      });
      act(() => {
        vi.advanceTimersByTime(30000);
      });
    }

    // 1 initial + 2 reconnects = 3
    expect(MockWebSocket.instances.length).toBe(3);
  });

  it('should not reconnect on manual disconnect', async () => {
    const useWebSocket = await importHook();
    const { result } = renderHook(() =>
      useWebSocket({ url: 'ws://localhost/ws' }),
    );

    act(() => {
      getLatestWs().simulateOpen();
    });

    act(() => {
      result.current.disconnect();
    });

    act(() => {
      vi.advanceTimersByTime(30000);
    });

    // Only the original instance
    expect(MockWebSocket.instances.length).toBe(1);
    expect(result.current.status).toBe('disconnected');
  });

  it('should support manual reconnect after disconnect', async () => {
    const useWebSocket = await importHook();
    const { result } = renderHook(() =>
      useWebSocket({ url: 'ws://localhost/ws' }),
    );

    act(() => {
      getLatestWs().simulateOpen();
    });

    act(() => {
      result.current.disconnect();
    });

    act(() => {
      result.current.reconnect();
    });

    expect(MockWebSocket.instances.length).toBe(2);
    expect(result.current.status).toBe('connecting');
  });

  it('should set error status on WebSocket error', async () => {
    const useWebSocket = await importHook();
    const onError = vi.fn();
    const { result } = renderHook(() =>
      useWebSocket({ url: 'ws://localhost/ws', onError }),
    );

    act(() => {
      getLatestWs().simulateError();
    });

    expect(result.current.status).toBe('error');
    expect(onError).toHaveBeenCalled();
  });

  it('should cleanup on unmount', async () => {
    const useWebSocket = await importHook();
    const { unmount } = renderHook(() =>
      useWebSocket({ url: 'ws://localhost/ws' }),
    );

    act(() => {
      getLatestWs().simulateOpen();
    });

    unmount();

    expect(getLatestWs().readyState).toBe(MockWebSocket.CLOSED);
  });
});

// ═════════════════════════════════════════════════════════════════════
// useDashboardLive tests
// ═════════════════════════════════════════════════════════════════════
describe('useDashboardLive', () => {
  async function importHook() {
    return (await import('../use-dashboard-live')).useDashboardLive;
  }

  it('should initialize with null values', async () => {
    const useDashboardLive = await importHook();
    const { result } = renderHook(() => useDashboardLive());

    expect(result.current.stats).toBeNull();
    expect(result.current.volume).toBeNull();
    expect(result.current.recentIntents).toEqual([]);
    expect(result.current.lastUpdate).toBeNull();
  });

  it('should update stats on dashboard_update message', async () => {
    const useDashboardLive = await importHook();
    const { result } = renderHook(() => useDashboardLive());

    act(() => {
      getLatestWs().simulateOpen();
    });

    const mockStats = {
      intents: { totalToday: 10, payinCount: 5, payoutCount: 3, pendingCount: 2, completedCount: 8, failedCount: 0 },
      cases: { total: 3, open: 1, inReview: 1, onHold: 0, resolved: 1, avgResolutionHours: 4 },
      users: { total: 100, active: 80, kycPending: 5, newToday: 3 },
      volume: { totalPayinVnd: '1000000', totalPayoutVnd: '500000', totalTradeVnd: '200000', period: '24h' },
    };

    act(() => {
      getLatestWs().simulateMessage({ type: 'dashboard_update', payload: mockStats });
    });

    expect(result.current.stats).toEqual(mockStats);
    expect(result.current.lastUpdate).toBeInstanceOf(Date);
  });

  it('should update volume on volume_update message', async () => {
    const useDashboardLive = await importHook();
    const { result } = renderHook(() => useDashboardLive());

    act(() => {
      getLatestWs().simulateOpen();
    });

    const mockVolume = { totalPayinVnd: '2000000', totalPayoutVnd: '1000000', totalTradeVnd: '500000', period: '1h' };

    act(() => {
      getLatestWs().simulateMessage({ type: 'volume_update', payload: mockVolume });
    });

    expect(result.current.volume).toEqual(mockVolume);
  });

  it('should add intents on intent_created message', async () => {
    const useDashboardLive = await importHook();
    const { result } = renderHook(() => useDashboardLive());

    act(() => {
      getLatestWs().simulateOpen();
    });

    const intent1 = { id: 'i-1', state: 'PENDING', amount: '100', currency: 'VND' };
    const intent2 = { id: 'i-2', state: 'COMPLETED', amount: '200', currency: 'VND' };

    act(() => {
      getLatestWs().simulateMessage({ type: 'intent_created', payload: intent1 });
    });

    act(() => {
      getLatestWs().simulateMessage({ type: 'intent_created', payload: intent2 });
    });

    expect(result.current.recentIntents).toHaveLength(2);
    expect(result.current.recentIntents[0].id).toBe('i-2');
    expect(result.current.recentIntents[1].id).toBe('i-1');
  });

  it('should deduplicate intents on intent_updated', async () => {
    const useDashboardLive = await importHook();
    const { result } = renderHook(() => useDashboardLive());

    act(() => {
      getLatestWs().simulateOpen();
    });

    act(() => {
      getLatestWs().simulateMessage({ type: 'intent_created', payload: { id: 'i-1', state: 'PENDING' } });
    });

    act(() => {
      getLatestWs().simulateMessage({ type: 'intent_updated', payload: { id: 'i-1', state: 'COMPLETED' } });
    });

    expect(result.current.recentIntents).toHaveLength(1);
    expect(result.current.recentIntents[0].state).toBe('COMPLETED');
  });

  it('should cap recent intents at 20', async () => {
    const useDashboardLive = await importHook();
    const { result } = renderHook(() => useDashboardLive());

    act(() => {
      getLatestWs().simulateOpen();
    });

    for (let i = 0; i < 25; i++) {
      act(() => {
        getLatestWs().simulateMessage({
          type: 'intent_created',
          payload: { id: `i-${i}`, state: 'PENDING' },
        });
      });
    }

    expect(result.current.recentIntents).toHaveLength(20);
  });

  it('should ignore unknown message types', async () => {
    const useDashboardLive = await importHook();
    const { result } = renderHook(() => useDashboardLive());

    act(() => {
      getLatestWs().simulateOpen();
    });

    act(() => {
      getLatestWs().simulateMessage({ type: 'unknown_type', payload: {} });
    });

    expect(result.current.stats).toBeNull();
    expect(result.current.volume).toBeNull();
    expect(result.current.recentIntents).toEqual([]);
  });

  it('should pass auth token to WebSocket', async () => {
    const useDashboardLive = await importHook();
    renderHook(() => useDashboardLive('my-token'));

    expect(getLatestWs().url).toContain('token=my-token');
  });
});

// ═════════════════════════════════════════════════════════════════════
// useIntentSubscription tests
// ═════════════════════════════════════════════════════════════════════
describe('useIntentSubscription', () => {
  async function importHook() {
    return (await import('../use-intent-subscription')).useIntentSubscription;
  }

  it('should initialize with no subscription', async () => {
    const useIntentSubscription = await importHook();
    const { result } = renderHook(() => useIntentSubscription());

    expect(result.current.intentStatus).toBeNull();
    expect(result.current.isSubscribed).toBe(false);
    // url is undefined so no WS created
    expect(MockWebSocket.instances.length).toBe(0);
  });

  it('should create WebSocket when subscribing', async () => {
    const useIntentSubscription = await importHook();
    const { result } = renderHook(() => useIntentSubscription());

    act(() => {
      result.current.subscribe('intent-001');
    });

    expect(MockWebSocket.instances.length).toBeGreaterThanOrEqual(1);
  });

  it('should send subscribe message when connected', async () => {
    const useIntentSubscription = await importHook();
    const { result } = renderHook(() => useIntentSubscription());

    act(() => {
      result.current.subscribe('intent-001');
    });

    act(() => {
      getLatestWs().simulateOpen();
    });

    // After connected, subscribe again to send the message
    act(() => {
      result.current.subscribe('intent-001');
    });

    const sent = getLatestWs().sent;
    expect(sent.length).toBeGreaterThanOrEqual(1);
    const parsed = JSON.parse(sent[sent.length - 1]);
    expect(parsed).toEqual({ type: 'subscribe_intent', intentId: 'intent-001' });
  });

  it('should update intent status on matching message', async () => {
    const useIntentSubscription = await importHook();
    const { result } = renderHook(() => useIntentSubscription());

    act(() => {
      result.current.subscribe('intent-001');
    });

    act(() => {
      getLatestWs().simulateOpen();
    });

    const statusUpdate = {
      intentId: 'intent-001',
      state: 'COMPLETED',
      previousState: 'PENDING',
      updatedAt: '2026-01-15T10:00:00Z',
    };

    act(() => {
      getLatestWs().simulateMessage({ type: 'intent_status', payload: statusUpdate });
    });

    expect(result.current.intentStatus).toEqual(statusUpdate);
  });

  it('should clear state on unsubscribe', async () => {
    const useIntentSubscription = await importHook();
    const { result } = renderHook(() => useIntentSubscription());

    act(() => {
      result.current.subscribe('intent-001');
    });

    act(() => {
      getLatestWs().simulateOpen();
    });

    act(() => {
      getLatestWs().simulateMessage({
        type: 'intent_status',
        payload: { intentId: 'intent-001', state: 'COMPLETED', previousState: 'PENDING', updatedAt: '' },
      });
    });

    expect(result.current.intentStatus).not.toBeNull();

    act(() => {
      result.current.unsubscribe();
    });

    expect(result.current.intentStatus).toBeNull();
    expect(result.current.isSubscribed).toBe(false);
  });

  it('should pass auth token to WebSocket', async () => {
    const useIntentSubscription = await importHook();
    const { result } = renderHook(() => useIntentSubscription('auth-tok'));

    act(() => {
      result.current.subscribe('intent-001');
    });

    expect(getLatestWs().url).toContain('token=auth-tok');
  });

  it('should report isSubscribed correctly', async () => {
    const useIntentSubscription = await importHook();
    const { result } = renderHook(() => useIntentSubscription());

    expect(result.current.isSubscribed).toBe(false);

    act(() => {
      result.current.subscribe('intent-001');
    });

    act(() => {
      getLatestWs().simulateOpen();
    });

    expect(result.current.isSubscribed).toBe(true);
  });
});
