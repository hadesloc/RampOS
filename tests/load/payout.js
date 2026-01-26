import http from 'k6/http';
import { check, sleep } from 'k6';
import { CONFIG, HEADERS, getScenario } from './config.js';
import { randomIntBetween } from 'https://jslib.k6.io/k6-utils/1.4.0/index.js';

export const options = {
  scenarios: {
    payout_test: getScenario(__ENV.SCENARIO || 'smoke'),
  },
  thresholds: CONFIG.thresholds,
};

export default function () {
  const url = `${CONFIG.baseUrl}/intents/payout`;
  const amount = randomIntBetween(50000, 500000);
  const idempotencyKey = `payout_${Date.now()}_${__VU}_${__ITER}`;

  const payload = JSON.stringify({
    tenantId: CONFIG.tenantId,
    userId: CONFIG.userId,
    amountVnd: amount,
    railsProvider: CONFIG.railsProvider,
    bankAccount: {
      bankCode: 'TCB',
      accountNumber: '1234567890',
      accountName: 'TEST USER',
    },
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
