export const SCENARIOS = {
  smoke: {
    executor: 'constant-vus',
    vus: 1,
    duration: '1m',
  },
  load: {
    executor: 'constant-vus',
    vus: 50,
    duration: '5m',
  },
  stress: {
    executor: 'constant-vus',
    vus: 100,
    duration: '10m',
  },
  spike: {
    executor: 'ramping-vus',
    startVUs: 0,
    stages: [
      { duration: '30s', target: 200 },
      { duration: '1m', target: 200 },
      { duration: '30s', target: 0 },
    ],
  },
};
