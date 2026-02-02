# RampOS Rate Limiting

This document describes the rate limiting policies used by the RampOS API.

## Overview

RampOS implements multi-tier rate limiting to ensure fair usage and protect the platform from abuse. Rate limits are applied at multiple levels:

1. **Global Rate Limit** - Protects the entire platform
2. **Tenant Rate Limit** - Per-tenant request limits
3. **Endpoint Rate Limit** - Specific limits for sensitive endpoints

## Rate Limit Tiers

### Default Limits

| Tier | Limit | Window | Description |
|------|-------|--------|-------------|
| Global | 1,000 requests | 60 seconds | Platform-wide limit |
| Tenant | 100 requests | 60 seconds | Per-tenant limit |
| Endpoint | Varies | 60 seconds | Per-endpoint limit |

### Endpoint-Specific Limits

Certain endpoints have additional rate limits:

| Endpoint | Limit | Window | Reason |
|----------|-------|--------|--------|
| POST /v1/intents/payin | 30/min | 60 seconds | Transaction creation |
| POST /v1/intents/payout | 30/min | 60 seconds | Transaction creation |
| POST /v1/events/trade-executed | 60/min | 60 seconds | Trade recording |
| GET /v1/intents | 100/min | 60 seconds | List queries |

## Response Headers

All API responses include rate limit information in headers:

| Header | Description |
|--------|-------------|
| `X-RateLimit-Limit` | Maximum requests allowed in the window |
| `X-RateLimit-Remaining` | Remaining requests in current window |
| `X-RateLimit-Reset` | Seconds until the window resets |
| `Retry-After` | Seconds to wait before retrying (only when rate limited) |

### Example Response Headers

**Normal Response:**
```
HTTP/1.1 200 OK
X-RateLimit-Limit: 100
X-RateLimit-Remaining: 95
X-RateLimit-Reset: 45
Content-Type: application/json
```

**Rate Limited Response:**
```
HTTP/1.1 429 Too Many Requests
X-RateLimit-Limit: 100
X-RateLimit-Remaining: 0
X-RateLimit-Reset: 30
Retry-After: 30
Content-Type: application/json

{
  "error": {
    "code": "TOO_MANY_REQUESTS",
    "message": "Rate limit exceeded. Please retry after 30 seconds."
  }
}
```

## Rate Limit Algorithm

RampOS uses a sliding window algorithm implemented with Redis sorted sets:

```
Sliding Window Algorithm

Timeline: ────────────────────────────────────────────>
              │                    │
         window_start             now
              │                    │
              │<─── window_size ──>│
              │                    │
         ┌────┴────────────────────┴────┐
         │  Count requests in window    │
         │  If count < limit: ALLOW     │
         │  Else: REJECT                │
         └──────────────────────────────┘
```

### Algorithm Details

1. Remove all entries older than `window_start` from the sorted set
2. Count remaining entries
3. If count < limit:
   - Add current request timestamp
   - Set TTL on the key
   - Return ALLOWED
4. Else:
   - Calculate reset time from oldest entry
   - Return REJECTED with reset time

## Handling Rate Limits

### Best Practices

#### 1. Implement Exponential Backoff

```typescript
async function makeRequestWithRetry<T>(
  request: () => Promise<T>,
  maxRetries: number = 5
): Promise<T> {
  for (let attempt = 0; attempt < maxRetries; attempt++) {
    try {
      return await request();
    } catch (error) {
      if (error.status === 429) {
        const retryAfter = parseInt(error.headers['retry-after'] || '1', 10);
        const delay = retryAfter * 1000 * Math.pow(2, attempt);
        console.log(`Rate limited. Retrying in ${delay}ms...`);
        await sleep(delay);
      } else {
        throw error;
      }
    }
  }
  throw new Error('Max retries exceeded');
}
```

#### 2. Monitor Rate Limit Headers

```typescript
class RampClient {
  private remainingRequests: number = 100;
  private resetTime: number = 0;

  async request<T>(endpoint: string): Promise<T> {
    // Check if we're likely to be rate limited
    if (this.remainingRequests <= 0 && Date.now() < this.resetTime) {
      const waitTime = this.resetTime - Date.now();
      console.log(`Proactively waiting ${waitTime}ms to avoid rate limit`);
      await sleep(waitTime);
    }

    const response = await fetch(endpoint, this.getOptions());

    // Update rate limit tracking
    this.remainingRequests = parseInt(
      response.headers.get('X-RateLimit-Remaining') || '100',
      10
    );
    this.resetTime = Date.now() +
      parseInt(response.headers.get('X-RateLimit-Reset') || '60', 10) * 1000;

    return response.json();
  }
}
```

#### 3. Queue Requests

For high-volume applications, queue requests:

```typescript
import { Queue, Worker } from 'bullmq';

const requestQueue = new Queue('api-requests', {
  defaultJobOptions: {
    attempts: 5,
    backoff: {
      type: 'exponential',
      delay: 1000,
    },
  },
});

// Add requests to queue
await requestQueue.add('create-payin', { userId: 'user_123', amount: 1000000 });

// Process queue with rate awareness
new Worker('api-requests', async (job) => {
  const response = await rampClient.createPayin(job.data);
  return response;
}, {
  limiter: {
    max: 25, // Stay under 30/min limit
    duration: 60000,
  },
});
```

#### 4. Batch Operations

Where possible, batch multiple operations:

```typescript
// Instead of:
for (const userId of userIds) {
  await client.getBalance(userId);  // Multiple requests
}

// Use batch endpoint:
const balances = await client.getBalances(userIds);  // Single request
```

### Go Example

```go
package main

import (
    "context"
    "fmt"
    "net/http"
    "strconv"
    "time"
)

type RateLimitInfo struct {
    Limit     int
    Remaining int
    Reset     time.Duration
}

func extractRateLimitInfo(resp *http.Response) RateLimitInfo {
    limit, _ := strconv.Atoi(resp.Header.Get("X-RateLimit-Limit"))
    remaining, _ := strconv.Atoi(resp.Header.Get("X-RateLimit-Remaining"))
    reset, _ := strconv.Atoi(resp.Header.Get("X-RateLimit-Reset"))

    return RateLimitInfo{
        Limit:     limit,
        Remaining: remaining,
        Reset:     time.Duration(reset) * time.Second,
    }
}

func (c *Client) doWithRetry(ctx context.Context, req *http.Request) (*http.Response, error) {
    maxRetries := 5

    for attempt := 0; attempt < maxRetries; attempt++ {
        resp, err := c.httpClient.Do(req.WithContext(ctx))
        if err != nil {
            return nil, err
        }

        if resp.StatusCode != http.StatusTooManyRequests {
            return resp, nil
        }

        // Rate limited - wait and retry
        retryAfter := resp.Header.Get("Retry-After")
        waitSeconds, _ := strconv.Atoi(retryAfter)
        if waitSeconds == 0 {
            waitSeconds = 1
        }

        // Exponential backoff
        delay := time.Duration(waitSeconds) * time.Second * time.Duration(1<<attempt)
        fmt.Printf("Rate limited. Retrying in %v...\n", delay)

        select {
        case <-ctx.Done():
            return nil, ctx.Err()
        case <-time.After(delay):
            continue
        }
    }

    return nil, fmt.Errorf("max retries exceeded")
}
```

### Python Example

```python
import time
import requests
from functools import wraps

class RateLimitError(Exception):
    def __init__(self, retry_after: int):
        self.retry_after = retry_after
        super().__init__(f"Rate limited. Retry after {retry_after} seconds.")

def with_rate_limit_retry(max_retries: int = 5):
    def decorator(func):
        @wraps(func)
        def wrapper(*args, **kwargs):
            for attempt in range(max_retries):
                try:
                    return func(*args, **kwargs)
                except RateLimitError as e:
                    if attempt == max_retries - 1:
                        raise

                    delay = e.retry_after * (2 ** attempt)
                    print(f"Rate limited. Retrying in {delay}s...")
                    time.sleep(delay)

            raise Exception("Max retries exceeded")
        return wrapper
    return decorator

class RampClient:
    def __init__(self, api_key: str):
        self.api_key = api_key
        self.remaining = 100
        self.reset_at = 0

    def _request(self, method: str, path: str, **kwargs):
        response = requests.request(
            method,
            f"https://api.ramp.vn{path}",
            headers={
                "Authorization": f"Bearer {self.api_key}",
                "X-Timestamp": str(int(time.time())),
            },
            **kwargs
        )

        # Update rate limit tracking
        self.remaining = int(response.headers.get("X-RateLimit-Remaining", 100))
        reset_seconds = int(response.headers.get("X-RateLimit-Reset", 60))
        self.reset_at = time.time() + reset_seconds

        if response.status_code == 429:
            retry_after = int(response.headers.get("Retry-After", 1))
            raise RateLimitError(retry_after)

        response.raise_for_status()
        return response.json()

    @with_rate_limit_retry(max_retries=5)
    def create_payin(self, user_id: str, amount_vnd: int):
        return self._request("POST", "/v1/intents/payin", json={
            "tenantId": self.tenant_id,
            "userId": user_id,
            "amountVnd": amount_vnd,
            "railsProvider": "vietqr",
        })
```

## Rate Limit Errors

### 429 Too Many Requests

**Global Rate Limit Exceeded:**
```json
{
  "error": {
    "code": "TOO_MANY_REQUESTS",
    "message": "Global rate limit exceeded"
  }
}
```

**Tenant Rate Limit Exceeded:**
```json
{
  "error": {
    "code": "TOO_MANY_REQUESTS",
    "message": "Tenant rate limit exceeded"
  }
}
```

**Endpoint Rate Limit Exceeded:**
```json
{
  "error": {
    "code": "TOO_MANY_REQUESTS",
    "message": "Endpoint rate limit exceeded for /v1/intents/payin"
  }
}
```

## Increasing Rate Limits

### Enterprise Plans

Higher rate limits are available on enterprise plans:

| Plan | Tenant Limit | Endpoint Limits |
|------|--------------|-----------------|
| Starter | 100/min | Standard |
| Growth | 500/min | 2x standard |
| Enterprise | 2,000/min | Customizable |

### Requesting Limit Increases

Contact support for temporary or permanent limit increases:

1. Describe your use case
2. Provide expected request volume
3. Share traffic patterns (burst vs. steady)

## Fail-Open Policy

RampOS rate limiting uses a fail-open policy:

- If the rate limiting system (Redis) is unavailable, requests are **allowed**
- This prevents system-wide outages due to rate limiter failures
- Abuse detection will catch any anomalies post-hoc

```
Rate Limit Check Flow

    Request
       │
       ▼
┌──────────────┐
│ Check Redis  │
└──────┬───────┘
       │
    ┌──┴──┐
    │     │
  OK?   Error?
    │     │
    ▼     ▼
┌───────┐ ┌───────────┐
│ Count │ │ Fail Open │
│ Check │ │  (Allow)  │
└───┬───┘ └───────────┘
    │
 ┌──┴──┐
 │     │
Under Over
Limit Limit
 │     │
 ▼     ▼
Allow  Reject
```

## Monitoring Your Usage

### Dashboard

View your rate limit usage in the tenant dashboard:

- Current usage vs. limits
- Historical usage patterns
- Rate limit incidents

### API

Query your current rate limit status:

```bash
curl https://api.ramp.vn/v1/admin/rate-limits/status \
  -H "Authorization: Bearer <api_key>"
```

**Response:**
```json
{
  "tenant": {
    "limit": 100,
    "used": 45,
    "remaining": 55,
    "resetsAt": "2026-01-23T10:01:00Z"
  },
  "endpoints": {
    "/v1/intents/payin": {
      "limit": 30,
      "used": 12,
      "remaining": 18,
      "resetsAt": "2026-01-23T10:01:00Z"
    }
  }
}
```

## Configuration Reference

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `RATE_LIMIT_GLOBAL_MAX` | 1000 | Global max requests per window |
| `RATE_LIMIT_TENANT_MAX` | 100 | Tenant max requests per window |
| `RATE_LIMIT_WINDOW_SECONDS` | 60 | Rate limit window size |
| `RATE_LIMIT_KEY_PREFIX` | `ramp:rate_limit` | Redis key prefix |

### Redis Keys

Rate limiting uses the following Redis key patterns:

```
ramp:rate_limit:global              # Global counter
ramp:rate_limit:<tenant_id>          # Tenant counter
ramp:rate_limit:<tenant_id>:<path>   # Endpoint counter
```

---

**See Also:**
- [Authentication](./authentication.md)
- [API Endpoints](./endpoints.md)
- [Webhooks](./webhooks.md)
