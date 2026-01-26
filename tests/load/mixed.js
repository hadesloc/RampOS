import http from 'k6/http';
import { check, sleep } from 'k6';
import { CONFIG, HEADERS, getScenario } from './config.js';
import { randomIntBetween } from 'https://jslib.k6.io/k6-utils/1.4.0/index.js';
import payin from './payin.js';
import payout from './payout.js';
import trade from './trade.js';

// Mixed scenario: Users doing random actions
// We define a single scenario but distribute work in the default function
// OR we can define multiple scenarios running in parallel

export const options = {
  scenarios: {
    mixed_traffic: getScenario(__ENV.SCENARIO || 'load'),
  },
  thresholds: CONFIG.thresholds,
};

export default function () {
  const rand = Math.random();

  if (rand < 0.4) {
    // 40% Payins
    payin();
  } else if (rand < 0.7) {
    // 30% Payouts
    payout();
  } else {
    // 30% Trades
    trade();
  }
}
