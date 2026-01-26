import http from 'k6/http';
import { check, sleep } from 'k6';
import { CONFIG, HEADERS, getScenario } from './config.js';
import { randomIntBetween } from 'https://jslib.k6.io/k6-utils/1.4.0/index.js';

export const options = {
  scenarios: {
    trade_test: getScenario(__ENV.SCENARIO || 'smoke'),
  },
  thresholds: CONFIG.thresholds,
};

export default function () {
  const url = `${CONFIG.baseUrl}/intents/trade`;
  const amount = randomIntBetween(100000, 10000000); // Trade amounts usually larger
  const idempotencyKey = `trade_${Date.now()}_${__VU}_${__ITER}`;

  // Randomize direction
  const direction = Math.random() > 0.5 ? 'buy' : 'sell';

  const payload = JSON.stringify({
    tenantId: CONFIG.tenantId,
    userId: CONFIG.userId,
    pair: 'USDT/VND',
    direction: direction,
    amountVnd: direction === 'buy' ? amount : undefined,
    quantity: direction === 'sell' ? (amount / 25000) : undefined, // Approx rate
    quoteId: `quote_${Date.now()}`,
    metadata: {
      source: 'load_test',
    },
  });

  const params = {
    headers: {
      ...HEADERS,
      'Idempotency-Key': idempotencyKey,
    },
  };

  const res = http.post(url, payload, params);

  check(res, {
    'is status 200': (r) => r.status === 200,
    'has intent_id': (r) => r.json('intentId') !== undefined,
  });

  sleep(1);
}
