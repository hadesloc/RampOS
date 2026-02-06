# Enterprise API Rate Limits

This guide covers rate limiting configuration, quota management, and best practices for RampOS enterprise deployments.

---

## Rate Limit Overview

RampOS implements multiple layers of rate limiting to ensure fair usage and protect system stability:

| Layer | Scope | Purpose |
|-------|-------|---------|
| Global | System-wide | Protect infrastructure |
| Tenant | Per-tenant | Fair usage between customers |
| User | Per-user | Prevent abuse |
| Endpoint | Per-endpoint | Protect sensitive operations |

---

## Rate Limits by Tier

### Starter Tier

| Limit Type | Value | Window |
|------------|-------|--------|
| API Requests | 100 | per minute |
| Burst Capacity | 20 | instant |
| Intent Creation | 50 | per hour |
| Webhook Deliveries | 100 | per hour |
| Report Generation | 10 | per day |
| File Uploads | 50 MB | per day |

### Professional Tier

| Limit Type | Value | Window |
|------------|-------|--------|
| API Requests | 1,000 | per minute |
| Burst Capacity | 200 | instant |
| Intent Creation | 500 | per hour |
| Webhook Deliveries | 1,000 | per hour |
| Report Generation | 100 | per day |
| File Uploads | 500 MB | per day |

### Enterprise Tier

| Limit Type | Value | Window |
|------------|-------|--------|
| API Requests | Custom | per minute |
| Burst Capacity | Custom | instant |
| Intent Creation | Unlimited | - |
| Webhook Deliveries | Unlimited | - |
| Report Generation | Unlimited | - |
| File Uploads | Custom | per day |

---

## Endpoint-Specific Limits

### High-Frequency Endpoints

| Endpoint | Limit | Window | Notes |
|----------|-------|--------|-------|
| `GET /health` | Unlimited | - | Not rate limited |
| `GET /v1/intents/{id}` | 1,000 | per minute | Status polling |
| `GET /v1/users/{id}/balances` | 500 | per minute | Balance queries |
| `GET /v1/rates` | 100 | per minute | Exchange rates |

### Write Endpoints

| Endpoint | Limit | Window | Notes |
|----------|-------|--------|-------|
| `POST /v1/intents/payin` | 100 | per minute | Intent creation |
| `POST /v1/intents/payout` | 100 | per minute | Intent creation |
| `POST /v1/users` | 50 | per minute | User creation |
| `POST /v1/kyc/verify` | 20 | per minute | KYC submissions |

### Sensitive Endpoints

| Endpoint | Limit | Window | Notes |
|----------|-------|--------|-------|
| `POST /v1/auth/login` | 10 | per minute | Authentication |
| `POST /v1/auth/token` | 20 | per minute | Token refresh |
| `POST /v1/admin/*` | 50 | per minute | Admin operations |
| `DELETE /*` | 20 | per minute | Destructive operations |

---

## Rate Limit Headers

All API responses include rate limit headers:

```http
HTTP/1.1 200 OK
X-RateLimit-Limit: 1000
X-RateLimit-Remaining: 950
X-RateLimit-Reset: 1707220800
X-RateLimit-Policy: "1000;w=60"
Retry-After: 30
```

### Header Descriptions

| Header | Description |
|--------|-------------|
| `X-RateLimit-Limit` | Maximum requests allowed in window |
| `X-RateLimit-Remaining` | Remaining requests in current window |
| `X-RateLimit-Reset` | Unix timestamp when window resets |
| `X-RateLimit-Policy` | Policy string (requests;w=window_seconds) |
| `Retry-After` | Seconds to wait before retrying (on 429) |

---

## Rate Limit Response

When rate limited, the API returns:

```http
HTTP/1.1 429 Too Many Requests
Content-Type: application/json
Retry-After: 30
X-RateLimit-Limit: 100
X-RateLimit-Remaining: 0
X-RateLimit-Reset: 1707220800

{
  "error": {
    "code": "RATE_LIMITED",
    "message": "Too many requests. Please retry after 30 seconds.",
    "details": {
      "limit": 100,
      "window": "1m",
      "retryAfter": 30
    }
  },
  "requestId": "req_abc123"
}
```

---

## Configuration

### Environment Variables

```bash
# Enable rate limiting
RATE_LIMIT_ENABLED=true

# Global limits
RATE_LIMIT_REQUESTS=1000
RATE_LIMIT_WINDOW=60
RATE_LIMIT_BURST=200

# Algorithm (token_bucket, sliding_window, fixed_window)
RATE_LIMIT_ALGORITHM=token_bucket

# Redis backend for distributed rate limiting
RATE_LIMIT_REDIS_URL=redis://:password@redis:6379/1
```

### Kubernetes ConfigMap

```yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: rampos-rate-limits
  namespace: rampos
data:
  rate-limits.yaml: |
    global:
      enabled: true
      requests: 10000
      window: 60
      burst: 2000

    tenants:
      default:
        requests: 1000
        window: 60
        burst: 200

      enterprise:
        requests: 10000
        window: 60
        burst: 2000

    endpoints:
      - path: "/v1/auth/login"
        method: "POST"
        requests: 10
        window: 60

      - path: "/v1/intents/*"
        method: "POST"
        requests: 100
        window: 60

      - path: "/v1/admin/*"
        method: "*"
        requests: 50
        window: 60
```

### Per-Tenant Configuration

Configure tenant-specific limits via API:

```bash
curl -X PUT https://api.ramp.vn/v1/admin/tenants/tenant_abc123/rate-limits \
  -H "Authorization: Bearer admin_token" \
  -H "Content-Type: application/json" \
  -d '{
    "apiRequests": {
      "limit": 5000,
      "window": 60,
      "burst": 1000
    },
    "intentCreation": {
      "limit": 1000,
      "window": 3600
    },
    "webhookDelivery": {
      "limit": 5000,
      "window": 3600
    }
  }'
```

---

## Quota Management

### Quota Types

| Quota | Description | Reset Period |
|-------|-------------|--------------|
| Monthly Volume | Transaction volume in VND | Monthly |
| Monthly Transactions | Number of transactions | Monthly |
| Daily API Calls | Total API requests | Daily |
| Storage | Document storage | Cumulative |
| Users | Registered users | Cumulative |

### Quota Limits by Tier

| Quota | Starter | Professional | Enterprise |
|-------|---------|--------------|------------|
| Monthly Volume | 1B VND | 50B VND | Unlimited |
| Monthly Transactions | 10,000 | 500,000 | Unlimited |
| Daily API Calls | 100,000 | 1,000,000 | Custom |
| Storage | 1 GB | 50 GB | Custom |
| Users | 1,000 | 50,000 | Unlimited |

### Quota Monitoring

```bash
# Get current quota usage
curl -X GET https://api.ramp.vn/v1/admin/tenants/tenant_abc123/quota \
  -H "Authorization: Bearer admin_token"
```

**Response:**
```json
{
  "tenantId": "tenant_abc123",
  "tier": "professional",
  "period": "2026-02",
  "quotas": {
    "monthlyVolume": {
      "limit": 50000000000000,
      "used": 12500000000000,
      "remaining": 37500000000000,
      "usagePercent": 25.0
    },
    "monthlyTransactions": {
      "limit": 500000,
      "used": 125000,
      "remaining": 375000,
      "usagePercent": 25.0
    },
    "dailyApiCalls": {
      "limit": 1000000,
      "used": 50000,
      "remaining": 950000,
      "usagePercent": 5.0,
      "resetsAt": "2026-02-07T00:00:00Z"
    },
    "storage": {
      "limit": 53687091200,
      "used": 5368709120,
      "remaining": 48318382080,
      "usagePercent": 10.0
    }
  }
}
```

### Quota Alerts

Configure alerts for quota thresholds:

```bash
curl -X PUT https://api.ramp.vn/v1/admin/tenants/tenant_abc123/quota-alerts \
  -H "Authorization: Bearer admin_token" \
  -H "Content-Type: application/json" \
  -d '{
    "alerts": [
      {
        "quota": "monthlyVolume",
        "threshold": 80,
        "channels": ["email", "webhook"]
      },
      {
        "quota": "dailyApiCalls",
        "threshold": 90,
        "channels": ["slack"]
      }
    ],
    "contacts": {
      "email": ["ops@tenant.com"],
      "slack": "https://hooks.slack.com/..."
    }
  }'
```

---

## Client Best Practices

### Implement Exponential Backoff

```typescript
async function requestWithRetry<T>(
  fn: () => Promise<T>,
  maxRetries: number = 5
): Promise<T> {
  let lastError: Error;

  for (let attempt = 0; attempt < maxRetries; attempt++) {
    try {
      return await fn();
    } catch (error: any) {
      lastError = error;

      if (error.status !== 429) {
        throw error;
      }

      const retryAfter = parseInt(error.headers?.['retry-after'] || '1');
      const backoff = Math.min(retryAfter * 1000, Math.pow(2, attempt) * 1000);

      console.log(`Rate limited. Retrying in ${backoff}ms...`);
      await new Promise(resolve => setTimeout(resolve, backoff));
    }
  }

  throw lastError!;
}
```

### Track Rate Limit Headers

```typescript
class RateLimitTracker {
  private remaining: number = Infinity;
  private resetTime: number = 0;

  update(headers: Headers) {
    this.remaining = parseInt(headers.get('X-RateLimit-Remaining') || '0');
    this.resetTime = parseInt(headers.get('X-RateLimit-Reset') || '0');
  }

  async waitIfNeeded() {
    if (this.remaining <= 1) {
      const waitMs = (this.resetTime * 1000) - Date.now();
      if (waitMs > 0) {
        console.log(`Approaching rate limit. Waiting ${waitMs}ms...`);
        await new Promise(resolve => setTimeout(resolve, waitMs));
      }
    }
  }
}
```

### Request Batching

```typescript
// Instead of many individual requests
// BAD:
for (const userId of userIds) {
  await client.users.getBalance(userId);
}

// GOOD: Use batch endpoint
const balances = await client.users.getBalancesBatch(userIds);
```

### Caching

```typescript
import { LRUCache } from 'lru-cache';

const cache = new LRUCache<string, any>({
  max: 1000,
  ttl: 60 * 1000, // 1 minute
});

async function getCachedRate(pair: string): Promise<number> {
  const cached = cache.get(pair);
  if (cached) return cached;

  const rate = await client.rates.get(pair);
  cache.set(pair, rate);
  return rate;
}
```

---

## Rate Limit Algorithms

### Token Bucket (Default)

- Tokens added at fixed rate
- Allows controlled bursting
- Best for most use cases

```
Configuration:
- Rate: 100 requests/minute
- Burst: 20 tokens
- Refill: 1.67 tokens/second
```

### Sliding Window

- Counts requests in rolling window
- More accurate than fixed window
- Slightly higher memory usage

```
Configuration:
- Window: 60 seconds
- Limit: 100 requests
- Precision: 1 second
```

### Fixed Window

- Simple counting per time window
- Lowest overhead
- Can allow 2x burst at window boundary

```
Configuration:
- Window: 60 seconds
- Limit: 100 requests
```

### Selecting Algorithm

```bash
# Token bucket (default, recommended)
RATE_LIMIT_ALGORITHM=token_bucket

# Sliding window (more accurate)
RATE_LIMIT_ALGORITHM=sliding_window

# Fixed window (lowest overhead)
RATE_LIMIT_ALGORITHM=fixed_window
```

---

## Monitoring Rate Limits

### Prometheus Metrics

```promql
# Rate limit hit rate
rate(rampos_rate_limit_hits_total[5m])

# Rate limit by tenant
sum by (tenant_id) (rate(rampos_rate_limit_hits_total[5m]))

# Quota usage percentage
rampos_quota_usage_percent{quota="monthlyVolume"}
```

### Grafana Dashboard

Import the rate limits dashboard:

```json
{
  "dashboard": {
    "title": "RampOS Rate Limits",
    "panels": [
      {
        "title": "Rate Limit Hits",
        "type": "graph",
        "targets": [
          {
            "expr": "rate(rampos_rate_limit_hits_total[5m])"
          }
        ]
      },
      {
        "title": "Quota Usage",
        "type": "gauge",
        "targets": [
          {
            "expr": "rampos_quota_usage_percent"
          }
        ]
      }
    ]
  }
}
```

### Alerting

```yaml
groups:
  - name: rate-limits
    rules:
      - alert: HighRateLimitHits
        expr: rate(rampos_rate_limit_hits_total[5m]) > 10
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "High rate limit hits for tenant {{ $labels.tenant_id }}"

      - alert: QuotaNearLimit
        expr: rampos_quota_usage_percent > 90
        for: 1h
        labels:
          severity: warning
        annotations:
          summary: "Quota usage above 90% for {{ $labels.tenant_id }}"
```

---

## Request Priority

Enterprise tier customers can use request priority headers:

```http
POST /v1/intents/payout HTTP/1.1
Host: api.ramp.vn
Authorization: Bearer your_token
X-Priority: high
```

### Priority Levels

| Priority | Description | Quota Impact |
|----------|-------------|--------------|
| `low` | Background tasks | Normal |
| `normal` | Default priority | Normal |
| `high` | Time-sensitive | 2x quota consumption |
| `critical` | Emergency operations | 5x quota consumption |

---

## Exemptions

### IP Allowlisting

Exempt specific IPs from rate limiting:

```bash
curl -X PUT https://api.ramp.vn/v1/admin/tenants/tenant_abc123/rate-limit-exemptions \
  -H "Authorization: Bearer admin_token" \
  -H "Content-Type: application/json" \
  -d '{
    "ipAllowlist": [
      "203.0.113.0/24",
      "198.51.100.50"
    ],
    "reason": "Internal monitoring systems"
  }'
```

### API Key Exemptions

Create API keys with custom limits:

```bash
curl -X POST https://api.ramp.vn/v1/admin/api-keys \
  -H "Authorization: Bearer admin_token" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "internal-monitoring",
    "tenantId": "tenant_abc123",
    "rateLimits": {
      "requests": 10000,
      "window": 60,
      "exempt": ["GET /health", "GET /metrics"]
    }
  }'
```

---

## Troubleshooting

### Common Issues

| Issue | Cause | Solution |
|-------|-------|----------|
| Unexpected 429 errors | Distributed client instances | Share rate limit state |
| Inconsistent limits | Clock skew | Sync time with NTP |
| Quota resets early | Timezone misconfiguration | Use UTC |
| High latency on rate checks | Redis connection issues | Check Redis health |

### Debug Rate Limit State

```bash
# Get current rate limit state for a key
curl -X GET "https://api.ramp.vn/v1/admin/rate-limits/debug?key=tenant_abc123" \
  -H "Authorization: Bearer admin_token"
```

**Response:**
```json
{
  "key": "tenant_abc123",
  "algorithm": "token_bucket",
  "state": {
    "tokens": 850,
    "maxTokens": 1000,
    "refillRate": 16.67,
    "lastRefill": "2026-02-06T10:30:00Z"
  },
  "nextReset": "2026-02-06T10:31:00Z"
}
```

---

**Version**: 1.0.0
**Last Updated**: 2026-02-06
