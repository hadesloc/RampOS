import { test, expect } from '@playwright/test';
import { WebSocketServer } from 'ws';

interface SubscriptionFrame {
  type: string;
  topic?: string;
  intentId?: string;
}

test.describe('WebSocket subscriptions (real browser client)', () => {
  test('establishes subscription flow and handles a pushed event in-browser', async ({ page }) => {
    const wss = new WebSocketServer({ port: 0 });
    const wsPort = (wss.address() as { port: number }).port;
    const wsUrl = `ws://127.0.0.1:${wsPort}`;

    let subscribeFrameSeen = false;

    wss.on('connection', (socket) => {
      socket.on('message', (raw) => {
        const text = raw.toString();
        let frame: SubscriptionFrame | null = null;

        try {
          frame = JSON.parse(text) as SubscriptionFrame;
        } catch {
          frame = null;
        }

        if (frame?.type === 'subscribe_intent' && frame.intentId === 'intent-e2e-001') {
          subscribeFrameSeen = true;

          socket.send(
            JSON.stringify({
              type: 'subscribed',
              payload: { intentId: frame.intentId },
            }),
          );

          setTimeout(() => {
            socket.send(
              JSON.stringify({
                type: 'intent_status',
                payload: {
                  intentId: frame.intentId,
                  state: 'COMPLETED',
                  previousState: 'PENDING_RAILS',
                  updatedAt: '2026-02-23T00:00:00Z',
                },
              }),
            );
          }, 50);
        }
      });
    });

    try {
      const browserResult = await page.evaluate(async (url: string) => {
        return await new Promise<{
          opened: boolean;
          subscribed: boolean;
          finalEvent: { type: string; payload?: { intentId?: string; state?: string } } | null;
        }>((resolve, reject) => {
          const ws = new WebSocket(url);
          let opened = false;
          let subscribed = false;

          const timeout = setTimeout(() => {
            ws.close();
            reject(new Error('Timed out waiting for websocket subscription event'));
          }, 4000);

          ws.onopen = () => {
            opened = true;
            ws.send(
              JSON.stringify({
                type: 'subscribe_intent',
                intentId: 'intent-e2e-001',
              }),
            );
          };

          ws.onmessage = (event) => {
            const message = JSON.parse(String(event.data)) as {
              type: string;
              payload?: { intentId?: string; state?: string };
            };

            if (message.type === 'subscribed') {
              subscribed = true;
              return;
            }

            if (message.type === 'intent_status') {
              clearTimeout(timeout);
              ws.close();
              resolve({
                opened,
                subscribed,
                finalEvent: message,
              });
            }
          };

          ws.onerror = () => {
            clearTimeout(timeout);
            reject(new Error('Browser websocket client encountered an error'));
          };
        });
      }, wsUrl);

      expect(subscribeFrameSeen).toBe(true);
      expect(browserResult.opened).toBe(true);
      expect(browserResult.subscribed).toBe(true);
      expect(browserResult.finalEvent?.type).toBe('intent_status');
      expect(browserResult.finalEvent?.payload?.intentId).toBe('intent-e2e-001');
      expect(browserResult.finalEvent?.payload?.state).toBe('COMPLETED');
    } finally {
      await new Promise<void>((resolve, reject) => {
        wss.close((error?: Error) => {
          if (error) {
            reject(error);
            return;
          }
          resolve();
        });
      });
    }
  });
});
