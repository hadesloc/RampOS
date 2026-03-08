# Load Tests

This directory contains [k6](https://k6.io/) load testing scripts for RampOS.

## Prerequisites

1. Install k6:
   - Mac: `brew install k6`
   - Windows: `winget install k6`
   - Linux: `sudo apt-get install k6`

## Configuration

Configuration is managed in `config.js`. You can override defaults using environment variables:

- `BASE_URL`: API base URL (default: `http://localhost:3000/v1`)
- `API_KEY`: API key (default: `test_key`)
- `SCENARIO`: Test scenario to run (default: `smoke`)

## Scenarios

Available scenarios (defined in `config.js`):

1. `smoke`: 1 VU, 1 minute (Basic connectivity check)
2. `load`: 50 VUs, 5 minutes (Average load)
3. `stress`: 100 VUs, 10 minutes (Heavy load)
4. `spike`: 0->200 VUs in 1 minute (Sudden burst)

## Running Tests

### Smoke Test (Payin)
```bash
k6 run tests/load/payin.js
```

### Load Test (Payout)
```bash
k6 run -e SCENARIO=load tests/load/payout.js
```

### With Custom URL
```bash
k6 run -e BASE_URL=http://api.staging.rampos.com/v1 tests/load/payin.js
```

### Generate HTML Report
```bash
k6 run --out json=results.json tests/load/payin.js
# Note: Requires k6-reporter or similar tool to convert JSON to HTML
```

## Thresholds

Tests will fail if:
- p95 response time > 300ms (reads) / 500ms (writes)
- Error rate > 1%
