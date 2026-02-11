import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { createPayinIntent } from '../src/api/checkout-api';

describe('createPayinIntent', () => {
  const originalFetch = global.fetch;

  beforeEach(() => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
  });

  afterEach(() => {
    global.fetch = originalFetch;
    vi.useRealTimers();
  });

  it('returns CheckoutResult on successful API call', async () => {
    global.fetch = vi.fn().mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => ({
        transactionId: 'tx_abc123',
        timestamp: 1700000000000,
      }),
    });

    const result = await createPayinIntent({
      apiKey: 'test-key',
      amount: 100,
      asset: 'USDC',
    });

    expect(result).toEqual({
      transactionId: 'tx_abc123',
      status: 'success',
      amount: 100,
      asset: 'USDC',
      timestamp: 1700000000000,
    });

    expect(global.fetch).toHaveBeenCalledWith(
      '/api/v1/payin/intent',
      expect.objectContaining({
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          Authorization: 'Bearer test-key',
        },
        body: JSON.stringify({ amount: 100, asset: 'USDC' }),
      }),
    );
  });

  it('uses custom baseUrl when provided', async () => {
    global.fetch = vi.fn().mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => ({
        transactionId: 'tx_xyz',
        timestamp: 1700000000000,
      }),
    });

    await createPayinIntent({
      apiKey: 'key',
      amount: 50,
      asset: 'USDT',
      baseUrl: 'https://api.rampos.com',
    });

    expect(global.fetch).toHaveBeenCalledWith(
      'https://api.rampos.com/api/v1/payin/intent',
      expect.anything(),
    );
  });

  it('throws auth error on 401 response', async () => {
    global.fetch = vi.fn().mockResolvedValue({
      ok: false,
      status: 401,
    });

    await expect(
      createPayinIntent({ apiKey: 'bad-key', amount: 100, asset: 'USDC' }),
    ).rejects.toThrow('Authentication failed: invalid API key');
  });

  it('throws server error on 500 response', async () => {
    global.fetch = vi.fn().mockResolvedValue({
      ok: false,
      status: 500,
    });

    await expect(
      createPayinIntent({ apiKey: 'key', amount: 100, asset: 'USDC' }),
    ).rejects.toThrow('Server error: 500');
  });

  it('throws server error on 503 response', async () => {
    global.fetch = vi.fn().mockResolvedValue({
      ok: false,
      status: 503,
    });

    await expect(
      createPayinIntent({ apiKey: 'key', amount: 100, asset: 'USDC' }),
    ).rejects.toThrow('Server error: 503');
  });

  it('throws on 400 bad request', async () => {
    global.fetch = vi.fn().mockResolvedValue({
      ok: false,
      status: 400,
    });

    await expect(
      createPayinIntent({ apiKey: 'key', amount: -1, asset: 'USDC' }),
    ).rejects.toThrow('Request failed: 400');
  });

  it('throws timeout error when request exceeds timeout', async () => {
    global.fetch = vi.fn().mockImplementation((_url: string, init: any) => {
      return new Promise((_resolve, reject) => {
        init.signal.addEventListener('abort', () => {
          const err = new Error('The operation was aborted');
          err.name = 'AbortError';
          reject(err);
        });
      });
    });

    const promise = createPayinIntent({
      apiKey: 'key',
      amount: 100,
      asset: 'USDC',
      timeoutMs: 1000,
    });

    vi.advanceTimersByTime(1001);

    await expect(promise).rejects.toThrow('Request timed out');
  });

  it('throws on invalid JSON response', async () => {
    global.fetch = vi.fn().mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => {
        throw new SyntaxError('Unexpected token');
      },
    });

    await expect(
      createPayinIntent({ apiKey: 'key', amount: 100, asset: 'USDC' }),
    ).rejects.toThrow('Invalid response from server');
  });

  it('uses Date.now() as fallback timestamp when not in response', async () => {
    const now = 1700000000000;
    vi.setSystemTime(now);

    global.fetch = vi.fn().mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => ({
        transactionId: 'tx_no_ts',
      }),
    });

    const result = await createPayinIntent({
      apiKey: 'key',
      amount: 25,
      asset: 'ETH',
    });

    expect(result.timestamp).toBe(now);
    expect(result.transactionId).toBe('tx_no_ts');
  });

  it('propagates network errors', async () => {
    global.fetch = vi.fn().mockRejectedValue(new TypeError('Failed to fetch'));

    await expect(
      createPayinIntent({ apiKey: 'key', amount: 100, asset: 'USDC' }),
    ).rejects.toThrow('Failed to fetch');
  });
});
