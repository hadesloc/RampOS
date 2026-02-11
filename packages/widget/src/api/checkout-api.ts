import { CheckoutResult } from '../types';

export interface CreatePayinIntentParams {
  apiKey: string;
  amount: number;
  asset: string;
  baseUrl?: string;
  timeoutMs?: number;
}

export async function createPayinIntent(
  params: CreatePayinIntentParams,
): Promise<CheckoutResult> {
  const {
    apiKey,
    amount,
    asset,
    baseUrl = '',
    timeoutMs = 30000,
  } = params;

  const controller = new AbortController();
  const timeoutId = setTimeout(() => controller.abort(), timeoutMs);

  try {
    const response = await fetch(`${baseUrl}/api/v1/payin/intent`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        Authorization: `Bearer ${apiKey}`,
      },
      body: JSON.stringify({ amount, asset }),
      signal: controller.signal,
    });

    if (!response.ok) {
      if (response.status === 401) {
        throw new Error('Authentication failed: invalid API key');
      }
      if (response.status >= 500) {
        throw new Error(`Server error: ${response.status}`);
      }
      throw new Error(`Request failed: ${response.status}`);
    }

    let data: any;
    try {
      data = await response.json();
    } catch {
      throw new Error('Invalid response from server');
    }

    return {
      transactionId: data.transactionId,
      status: 'success',
      amount,
      asset,
      timestamp: data.timestamp ?? Date.now(),
    };
  } catch (err: any) {
    if (err.name === 'AbortError') {
      throw new Error('Request timed out');
    }
    throw err;
  } finally {
    clearTimeout(timeoutId);
  }
}
