# Load Testing Guide

This document covers load testing, performance benchmarks, and stress testing for RampOS.

## Overview

RampOS uses [k6](https://k6.io/) for load testing the API, along with custom Rust benchmarks for core services.

## Prerequisites

### Install k6

```bash
# macOS
brew install k6

# Windows (using Chocolatey)
choco install k6

# Windows (using Winget)
winget install k6

# Linux (Debian/Ubuntu)
sudo gpg -k
sudo gpg --no-default-keyring --keyring /usr/share/keyrings/k6-archive-keyring.gpg --keyserver hkp://keyserver.ubuntu.com:80 --recv-keys C5AD17C747E3415A3642D57D77C6C491D6AC1D69
echo "deb [signed-by=/usr/share/keyrings/k6-archive-keyring.gpg] https://dl.k6.io/deb stable main" | sudo tee /etc/apt/sources.list.d/k6.list
sudo apt-get update
sudo apt-get install k6

# Docker
docker pull grafana/k6
```

### Install Rust Benchmarking Tools

```bash
# Install criterion (built into cargo)
# No separate installation needed

# Install hyperfine for CLI benchmarks
cargo install hyperfine
```

## k6 Load Tests

### Directory Structure

```
scripts/load-tests/
├── scenarios/
│   ├── smoke.js          # Basic sanity check
│   ├── load.js           # Normal load test
│   ├── stress.js         # Stress test
│   ├── spike.js          # Spike test
│   └── soak.js           # Endurance test
├── helpers/
│   ├── auth.js           # Authentication helpers
│   └── data.js           # Test data generators
└── config/
    └── options.js        # Shared configuration
```

### Basic Load Test Script

Create `scripts/load-tests/scenarios/load.js`:

```javascript
import http from 'k6/http';
import { check, sleep } from 'k6';
import { Rate, Trend } from 'k6/metrics';

// Custom metrics
const errorRate = new Rate('errors');
const payinDuration = new Trend('payin_duration');
const payoutDuration = new Trend('payout_duration');

// Configuration
export const options = {
  stages: [
    { duration: '1m', target: 10 },   // Ramp up to 10 users
    { duration: '3m', target: 10 },   // Stay at 10 users
    { duration: '2m', target: 50 },   // Ramp up to 50 users
    { duration: '5m', target: 50 },   // Stay at 50 users
    { duration: '2m', target: 0 },    // Ramp down
  ],
  thresholds: {
    http_req_duration: ['p(95)<500'],     // 95% of requests under 500ms
    http_req_failed: ['rate<0.01'],        // Error rate under 1%
    'payin_duration': ['p(95)<300'],       // Payin 95th percentile under 300ms
    'payout_duration': ['p(95)<400'],      // Payout 95th percentile under 400ms
  },
};

// Test configuration
const BASE_URL = __ENV.BASE_URL || 'http://localhost:8080';
const API_KEY = __ENV.API_KEY || 'test_api_key';
const TENANT_ID = __ENV.TENANT_ID || 'tenant_load_test';

// Generate random user ID
function randomUserId() {
  return `user_${Math.random().toString(36).substring(7)}`;
}

// Generate random amount (10,000 - 100,000,000 VND)
function randomAmount() {
  return Math.floor(Math.random() * 99990000) + 10000;
}

export default function () {
  const userId = randomUserId();

  // Test 1: Create Payin Intent
  const payinPayload = JSON.stringify({
    tenantId: TENANT_ID,
    userId: userId,
    amountVnd: randomAmount(),
    railsProvider: 'VIETCOMBANK',
    metadata: { source: 'k6_load_test' }
  });

  const payinStart = Date.now();
  const payinRes = http.post(`${BASE_URL}/v1/intents/payin`, payinPayload, {
    headers: {
      'Content-Type': 'application/json',
      'Authorization': `Bearer ${API_KEY}`,
    },
  });
  payinDuration.add(Date.now() - payinStart);

  check(payinRes, {
    'payin status is 200': (r) => r.status === 200,
    'payin has intentId': (r) => JSON.parse(r.body).intentId !== undefined,
    'payin has referenceCode': (r) => JSON.parse(r.body).referenceCode !== undefined,
  });

  errorRate.add(payinRes.status !== 200);

  sleep(1);

  // Test 2: Get Balances
  const balanceRes = http.get(
    `${BASE_URL}/v1/users/${TENANT_ID}/${userId}/balances`,
    {
      headers: {
        'Authorization': `Bearer ${API_KEY}`,
      },
    }
  );

  check(balanceRes, {
    'balance status is 200': (r) => r.status === 200,
  });

  sleep(1);

  // Test 3: Create Payout Intent
  const payoutPayload = JSON.stringify({
    tenantId: TENANT_ID,
    userId: userId,
    amountVnd: 50000,
    railsProvider: 'VIETCOMBANK',
    bankAccount: {
      bankCode: 'VCB',
      accountNumber: '1234567890',
      accountName: 'TEST USER'
    },
    metadata: { source: 'k6_load_test' }
  });

  const payoutStart = Date.now();
  const payoutRes = http.post(`${BASE_URL}/v1/intents/payout`, payoutPayload, {
    headers: {
      'Content-Type': 'application/json',
      'Authorization': `Bearer ${API_KEY}`,
    },
  });
  payoutDuration.add(Date.now() - payoutStart);

  // Payout may fail due to insufficient balance - that's ok for load test
  check(payoutRes, {
    'payout status is 200 or 400': (r) => r.status === 200 || r.status === 400,
  });

  sleep(2);
}
```

### Smoke Test

Create `scripts/load-tests/scenarios/smoke.js`:

```javascript
import http from 'k6/http';
import { check } from 'k6';

export const options = {
  vus: 1,
  duration: '30s',
  thresholds: {
    http_req_failed: ['rate==0'],
    http_req_duration: ['p(95)<1000'],
  },
};

const BASE_URL = __ENV.BASE_URL || 'http://localhost:8080';

export default function () {
  // Health check
  const healthRes = http.get(`${BASE_URL}/health`);
  check(healthRes, {
    'health check passed': (r) => r.status === 200,
  });
}
```

### Stress Test

Create `scripts/load-tests/scenarios/stress.js`:

```javascript
import http from 'k6/http';
import { check, sleep } from 'k6';

export const options = {
  stages: [
    { duration: '2m', target: 100 },   // Ramp to 100 users
    { duration: '5m', target: 100 },   // Hold at 100
    { duration: '2m', target: 200 },   // Ramp to 200 users
    { duration: '5m', target: 200 },   // Hold at 200
    { duration: '2m', target: 300 },   // Ramp to 300 users
    { duration: '5m', target: 300 },   // Hold at 300
    { duration: '10m', target: 0 },    // Ramp down
  ],
  thresholds: {
    http_req_duration: ['p(99)<2000'],  // 99% under 2s
    http_req_failed: ['rate<0.1'],       // Error rate under 10%
  },
};

const BASE_URL = __ENV.BASE_URL || 'http://localhost:8080';
const API_KEY = __ENV.API_KEY || 'test_api_key';
const TENANT_ID = __ENV.TENANT_ID || 'tenant_stress_test';

export default function () {
  const payload = JSON.stringify({
    tenantId: TENANT_ID,
    userId: `user_${__VU}_${__ITER}`,
    amountVnd: 100000,
    railsProvider: 'VIETCOMBANK',
    metadata: {}
  });

  const res = http.post(`${BASE_URL}/v1/intents/payin`, payload, {
    headers: {
      'Content-Type': 'application/json',
      'Authorization': `Bearer ${API_KEY}`,
    },
  });

  check(res, {
    'status is 200 or 429': (r) => r.status === 200 || r.status === 429,
  });

  sleep(0.5);
}
```

### Spike Test

Create `scripts/load-tests/scenarios/spike.js`:

```javascript
import http from 'k6/http';
import { check, sleep } from 'k6';

export const options = {
  stages: [
    { duration: '10s', target: 100 },   // Quick ramp to 100
    { duration: '1m', target: 100 },    // Hold at 100
    { duration: '10s', target: 1000 },  // Spike to 1000
    { duration: '3m', target: 1000 },   // Hold at 1000
    { duration: '10s', target: 100 },   // Drop back to 100
    { duration: '3m', target: 100 },    // Hold at 100
    { duration: '10s', target: 0 },     // Ramp down
  ],
};

const BASE_URL = __ENV.BASE_URL || 'http://localhost:8080';

export default function () {
  const res = http.get(`${BASE_URL}/health`);
  check(res, {
    'status is 200': (r) => r.status === 200,
  });
  sleep(0.1);
}
```

### Soak Test (Endurance)

Create `scripts/load-tests/scenarios/soak.js`:

```javascript
import http from 'k6/http';
import { check, sleep } from 'k6';

export const options = {
  stages: [
    { duration: '5m', target: 50 },     // Ramp up
    { duration: '4h', target: 50 },     // Stay at 50 for 4 hours
    { duration: '5m', target: 0 },      // Ramp down
  ],
  thresholds: {
    http_req_duration: ['p(95)<500'],
    http_req_failed: ['rate<0.01'],
  },
};

const BASE_URL = __ENV.BASE_URL || 'http://localhost:8080';
const API_KEY = __ENV.API_KEY || 'test_api_key';
const TENANT_ID = __ENV.TENANT_ID || 'tenant_soak_test';

export default function () {
  // Mix of operations
  const rand = Math.random();

  if (rand < 0.3) {
    // 30% health checks
    http.get(`${BASE_URL}/health`);
  } else if (rand < 0.6) {
    // 30% balance checks
    http.get(`${BASE_URL}/v1/users/${TENANT_ID}/user_1/balances`, {
      headers: { 'Authorization': `Bearer ${API_KEY}` },
    });
  } else if (rand < 0.9) {
    // 30% payin
    const payload = JSON.stringify({
      tenantId: TENANT_ID,
      userId: `user_${__VU}`,
      amountVnd: 100000,
      railsProvider: 'VIETCOMBANK',
      metadata: {}
    });
    http.post(`${BASE_URL}/v1/intents/payin`, payload, {
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${API_KEY}`,
      },
    });
  } else {
    // 10% payout
    const payload = JSON.stringify({
      tenantId: TENANT_ID,
      userId: `user_${__VU}`,
      amountVnd: 50000,
      railsProvider: 'VIETCOMBANK',
      bankAccount: {
        bankCode: 'VCB',
        accountNumber: '1234567890',
        accountName: 'TEST'
      },
      metadata: {}
    });
    http.post(`${BASE_URL}/v1/intents/payout`, payload, {
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${API_KEY}`,
      },
    });
  }

  sleep(1);
}
```

## Running Load Tests

### Basic Commands

```bash
# Run smoke test
k6 run scripts/load-tests/scenarios/smoke.js

# Run load test with environment variables
k6 run -e BASE_URL=http://localhost:8080 -e API_KEY=my_key scripts/load-tests/scenarios/load.js

# Run with custom VUs and duration
k6 run --vus 50 --duration 5m scripts/load-tests/scenarios/load.js

# Run with output to InfluxDB
k6 run --out influxdb=http://localhost:8086/k6 scripts/load-tests/scenarios/load.js

# Run with JSON output
k6 run --out json=results.json scripts/load-tests/scenarios/load.js
```

### Docker Commands

```bash
# Run k6 in Docker
docker run --rm -i grafana/k6 run - <scripts/load-tests/scenarios/smoke.js

# Run with environment variables
docker run --rm -i \
  -e BASE_URL=http://host.docker.internal:8080 \
  -e API_KEY=test_key \
  grafana/k6 run - <scripts/load-tests/scenarios/load.js
```

## Performance Benchmarks

### Rust Benchmarks with Criterion

Create `benches/ledger_bench.rs`:

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use ramp_common::ledger::{
    LedgerTransaction, LedgerEntry, AccountType, EntryDirection, LedgerCurrency
};
use rust_decimal_macros::dec;
use ramp_common::types::{TenantId, IntentId, UserId};

fn create_ledger_transaction() -> LedgerTransaction {
    LedgerTransaction {
        id: "tx-bench-1".to_string(),
        tenant_id: TenantId::new("tenant-1"),
        intent_id: IntentId::new("intent-1"),
        entries: vec![
            LedgerEntry {
                account_type: AccountType::LiabilityUserVnd,
                user_id: Some(UserId::new("user-1")),
                direction: EntryDirection::Debit,
                amount: dec!(1_000_000),
                currency: LedgerCurrency::VND,
                description: "Debit user".to_string(),
            },
            LedgerEntry {
                account_type: AccountType::AssetRailsVnd,
                user_id: None,
                direction: EntryDirection::Credit,
                amount: dec!(1_000_000),
                currency: LedgerCurrency::VND,
                description: "Credit rails".to_string(),
            },
        ],
        description: "Benchmark transaction".to_string(),
        metadata: serde_json::json!({}),
    }
}

fn benchmark_transaction_creation(c: &mut Criterion) {
    c.bench_function("create_ledger_transaction", |b| {
        b.iter(|| {
            black_box(create_ledger_transaction())
        })
    });
}

fn benchmark_transaction_validation(c: &mut Criterion) {
    let tx = create_ledger_transaction();
    c.bench_function("validate_ledger_transaction", |b| {
        b.iter(|| {
            black_box(tx.validate())
        })
    });
}

fn benchmark_json_serialization(c: &mut Criterion) {
    let tx = create_ledger_transaction();
    c.bench_function("serialize_transaction", |b| {
        b.iter(|| {
            black_box(serde_json::to_string(&tx).unwrap())
        })
    });
}

criterion_group!(
    benches,
    benchmark_transaction_creation,
    benchmark_transaction_validation,
    benchmark_json_serialization
);
criterion_main!(benches);
```

### Running Benchmarks

```bash
# Run all benchmarks
cargo bench

# Run specific benchmark
cargo bench --bench ledger_bench

# Run with baseline comparison
cargo bench -- --baseline main

# Save baseline
cargo bench -- --save-baseline main
```

## Fuzz Testing (Compliance)

### libFuzzer Setup

RampOS includes fuzz testing for the compliance rule parser.

```rust
// crates/ramp-compliance/fuzz/fuzz_targets/rule_parser_target.rs
#![no_main]
use libfuzzer_sys::fuzz_target;
use ramp_compliance::rule_parser::RuleParser;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        // Try to parse - should not panic
        let _ = RuleParser::parse_json(s);
        let _ = RuleParser::parse(s);
    }
});
```

### Running Fuzz Tests

```bash
# Navigate to fuzz directory
cd crates/ramp-compliance/fuzz

# Run fuzzer (requires nightly)
cargo +nightly fuzz run rule_parser_target

# Run for specific duration
cargo +nightly fuzz run rule_parser_target -- -max_total_time=300

# Run with more workers
cargo +nightly fuzz run rule_parser_target -- -workers=4
```

### Simulated Fuzz Test

For CI environments, use the simulated fuzz test:

```rust
// crates/ramp-compliance/tests/fuzz_simulation.rs
use ramp_compliance::rule_parser::RuleParser;
use rand::{Rng, thread_rng};
use rand::distributions::Alphanumeric;

#[test]
fn fuzz_rule_parser_simulation() {
    let mut rng = thread_rng();
    let start_time = std::time::Instant::now();
    let max_duration = std::time::Duration::from_secs(60);

    let mut iterations = 0;

    while start_time.elapsed() < max_duration {
        // Random alphanumeric strings
        let len = rng.gen_range(0..1024);
        let s: String = (0..len)
            .map(|_| rng.sample(Alphanumeric) as char)
            .collect();

        let _ = RuleParser::parse_json(&s);
        let _ = RuleParser::parse(&s);

        iterations += 1;
    }

    println!("Completed {} iterations without crashing.", iterations);
}
```

Run with:
```bash
cargo test fuzz_rule_parser_simulation -- --nocapture
```

## Performance Targets

### API Response Time Targets

| Endpoint | p50 | p95 | p99 |
|----------|-----|-----|-----|
| `GET /health` | 5ms | 10ms | 20ms |
| `POST /v1/intents/payin` | 50ms | 150ms | 300ms |
| `POST /v1/intents/payout` | 75ms | 200ms | 400ms |
| `GET /v1/users/:id/balances` | 20ms | 50ms | 100ms |
| `POST /v1/events/trade-executed` | 100ms | 250ms | 500ms |

### Throughput Targets

| Scenario | Target RPS | Max Latency p99 |
|----------|------------|-----------------|
| Normal Load | 500 | 500ms |
| Peak Load | 1000 | 1000ms |
| Stress Test | 2000 | 2000ms |

### Resource Limits

| Metric | Warning | Critical |
|--------|---------|----------|
| CPU Usage | 70% | 90% |
| Memory Usage | 70% | 85% |
| DB Connections | 80% pool | 95% pool |
| Redis Connections | 80% pool | 95% pool |

## Monitoring During Tests

### Prometheus Metrics

```bash
# Query request latency
curl -s 'http://localhost:9090/api/v1/query?query=http_request_duration_seconds_bucket'

# Query error rate
curl -s 'http://localhost:9090/api/v1/query?query=rate(http_requests_total{status=~"5.."}[5m])'
```

### Grafana Dashboard

Import the k6 dashboard for real-time visualization:
- Dashboard ID: 2587 (k6 Load Testing Results)

### Resource Monitoring

```bash
# Monitor API server
docker stats ramp-api

# Monitor database
docker stats postgres

# Monitor Redis
docker stats redis
```

## CI Integration

### GitHub Actions Workflow

```yaml
name: Load Tests

on:
  schedule:
    - cron: '0 2 * * *'  # Daily at 2 AM
  workflow_dispatch:

jobs:
  load-test:
    runs-on: ubuntu-latest
    services:
      postgres:
        image: postgres:15
        env:
          POSTGRES_PASSWORD: postgres
        ports:
          - 5432:5432

    steps:
      - uses: actions/checkout@v4

      - name: Setup k6
        run: |
          sudo gpg -k
          sudo gpg --no-default-keyring --keyring /usr/share/keyrings/k6-archive-keyring.gpg --keyserver hkp://keyserver.ubuntu.com:80 --recv-keys C5AD17C747E3415A3642D57D77C6C491D6AC1D69
          echo "deb [signed-by=/usr/share/keyrings/k6-archive-keyring.gpg] https://dl.k6.io/deb stable main" | sudo tee /etc/apt/sources.list.d/k6.list
          sudo apt-get update
          sudo apt-get install k6

      - name: Build and Start API
        run: |
          cargo build --release
          ./target/release/ramp-api &
          sleep 5

      - name: Run Smoke Test
        run: k6 run scripts/load-tests/scenarios/smoke.js

      - name: Run Load Test
        run: k6 run scripts/load-tests/scenarios/load.js

      - name: Upload Results
        uses: actions/upload-artifact@v3
        with:
          name: k6-results
          path: results.json
```

## Best Practices

1. **Start with smoke tests**: Verify basic functionality before load testing
2. **Warm up the system**: Allow caches to populate before measuring
3. **Use realistic data**: Generate test data that matches production patterns
4. **Monitor all components**: Watch API, database, cache, and infrastructure
5. **Run in isolated environment**: Avoid testing on shared infrastructure
6. **Compare against baseline**: Track performance over time
7. **Test failure scenarios**: Include rate limiting and error handling tests
8. **Clean up after tests**: Reset database and cache state
9. **Document thresholds**: Define and enforce performance requirements
10. **Automate in CI**: Run performance tests on every deployment
