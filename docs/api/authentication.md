# RampOS API Authentication

This document describes the authentication mechanisms used by the RampOS API.

## Overview

RampOS uses a combination of API key authentication with timestamp validation to secure API requests. All authenticated endpoints require:

1. **Bearer Token Authentication** - API key passed in the Authorization header
2. **Timestamp Validation** - Request timestamp to prevent replay attacks
3. **Tenant Context** - Automatic tenant isolation based on API key

## Authentication Flow

```
Client                                  RampOS API
   |                                        |
   |  POST /v1/intents/payin               |
   |  Authorization: Bearer <api_key>       |
   |  X-Timestamp: <timestamp>              |
   |  Content-Type: application/json        |
   | -------------------------------------> |
   |                                        |
   |           1. Validate timestamp        |
   |           2. Hash API key (SHA-256)    |
   |           3. Lookup tenant by hash     |
   |           4. Verify tenant is ACTIVE   |
   |           5. Set tenant context        |
   |           6. Process request           |
   |                                        |
   |  200 OK                                |
   |  { "intentId": "...", ... }            |
   | <------------------------------------- |
```

## Required Headers

### Authorization Header

All authenticated requests must include a Bearer token:

```
Authorization: Bearer <your_api_key>
```

**Example:**
```bash
curl -X POST https://api.ramp.vn/v1/intents/payin \
  -H "Authorization: Bearer ramp_live_sk_abc123..."
```

### X-Timestamp Header

Every request must include a timestamp to prevent replay attacks:

```
X-Timestamp: <timestamp>
```

**Supported Formats:**
- **ISO 8601**: `2026-01-23T10:30:00Z`
- **Unix seconds**: `1737628200`
- **Unix milliseconds**: `1737628200000`

**Example:**
```bash
# Using ISO 8601
curl -X POST https://api.ramp.vn/v1/intents/payin \
  -H "Authorization: Bearer ramp_live_sk_abc123..." \
  -H "X-Timestamp: $(date -u +%Y-%m-%dT%H:%M:%SZ)"

# Using Unix timestamp
curl -X POST https://api.ramp.vn/v1/intents/payin \
  -H "Authorization: Bearer ramp_live_sk_abc123..." \
  -H "X-Timestamp: $(date +%s)"
```

## Timestamp Validation

The server validates that the request timestamp is within an acceptable window:

| Direction | Maximum Drift | Description |
|-----------|--------------|-------------|
| Past | 5 minutes (300s) | Request timestamp cannot be more than 5 minutes in the past |
| Future | 1 minute (60s) | Request timestamp cannot be more than 1 minute in the future |

**Clock Synchronization:** Ensure your server's clock is synchronized with NTP. Requests outside the allowed window will be rejected.

## API Key Management

### Key Structure

RampOS API keys follow a specific format:

```
ramp_<environment>_<type>_<random>
```

| Component | Values | Description |
|-----------|--------|-------------|
| environment | `live`, `test` | Production or sandbox environment |
| type | `sk` (secret key) | Server-side secret key |
| random | 32+ characters | Cryptographically random string |

**Examples:**
- Live secret key: `ramp_live_sk_a1b2c3d4e5f6g7h8i9j0...`
- Test secret key: `ramp_test_sk_x1y2z3a4b5c6d7e8f9g0...`

### Key Security

API keys are stored as SHA-256 hashes in the database:

```rust
// Pseudocode showing how API keys are validated
let api_key_hash = sha256(provided_api_key);
let tenant = lookup_by_api_key_hash(api_key_hash);
```

**Security Best Practices:**
- Never expose API keys in client-side code
- Store API keys in environment variables or secure vaults
- Rotate API keys periodically (recommended: every 90 days)
- Use separate keys for test and production environments
- Revoke compromised keys immediately

### Generating New API Keys

API keys are generated during tenant onboarding:

```bash
# Admin endpoint to generate new API keys for a tenant
curl -X POST https://api.ramp.vn/v1/admin/tenants/{tenant_id}/api-keys \
  -H "Authorization: Bearer <admin_api_key>" \
  -H "X-Timestamp: $(date +%s)" \
  -H "Content-Type: application/json"
```

**Response:**
```json
{
  "apiKey": "ramp_live_sk_...",
  "createdAt": "2026-01-23T10:00:00Z",
  "expiresAt": null
}
```

**Important:** The API key is only returned once during generation. Store it securely immediately.

## Tenant Context

Once authenticated, all requests are automatically scoped to the tenant:

1. **Automatic Isolation**: Queries are filtered by tenant_id
2. **Row-Level Security (RLS)**: Database enforces tenant isolation
3. **Cross-Tenant Protection**: Requests cannot access other tenants' data

### X-Tenant-ID Header (Optional)

For additional security, you can include the tenant ID header:

```
X-Tenant-ID: tenant_abc123
```

If provided, the server verifies it matches the authenticated tenant. A mismatch results in a `403 Forbidden` response.

## Error Responses

### 400 Bad Request - Missing/Invalid Timestamp

```json
{
  "error": "missing_timestamp",
  "message": "X-Timestamp header is missing"
}
```

```json
{
  "error": "invalid_format",
  "message": "Invalid timestamp format"
}
```

### 401 Unauthorized - Invalid/Missing API Key

```json
{
  "error": {
    "code": "UNAUTHORIZED",
    "message": "Invalid or missing API key"
  }
}
```

### 401 Unauthorized - Timestamp Out of Range

```json
{
  "error": "timestamp_expired",
  "message": "Request timestamp is outside acceptable range",
  "server_time": "2026-01-23T10:30:00Z"
}
```

### 403 Forbidden - Tenant Suspended

```json
{
  "error": {
    "code": "FORBIDDEN",
    "message": "Tenant is suspended"
  }
}
```

## Code Examples

### cURL

```bash
#!/bin/bash
API_KEY="ramp_live_sk_your_api_key_here"
TIMESTAMP=$(date +%s)

curl -X POST https://api.ramp.vn/v1/intents/payin \
  -H "Authorization: Bearer $API_KEY" \
  -H "X-Timestamp: $TIMESTAMP" \
  -H "Content-Type: application/json" \
  -d '{
    "tenantId": "tenant_abc123",
    "userId": "user_xyz789",
    "amountVnd": 1000000,
    "railsProvider": "vietqr"
  }'
```

### TypeScript/JavaScript

```typescript
import { RampClient } from '@ramp/sdk';

const client = new RampClient({
  apiKey: process.env.RAMP_API_KEY,
  environment: 'live', // or 'test'
});

// SDK automatically handles authentication headers
const intent = await client.intents.createPayin({
  userId: 'user_xyz789',
  amountVnd: 1000000,
  railsProvider: 'vietqr',
});
```

### Go

```go
package main

import (
    "github.com/ramp/sdk-go"
)

func main() {
    client := ramp.NewClient(ramp.Config{
        APIKey:      os.Getenv("RAMP_API_KEY"),
        Environment: ramp.EnvironmentLive,
    })

    // SDK automatically handles authentication headers
    intent, err := client.Intents.CreatePayin(ctx, &ramp.CreatePayinRequest{
        UserID:        "user_xyz789",
        AmountVND:     1000000,
        RailsProvider: "vietqr",
    })
}
```

### Python

```python
import time
import requests
import os

API_KEY = os.environ['RAMP_API_KEY']
BASE_URL = 'https://api.ramp.vn'

def make_request(method, path, data=None):
    headers = {
        'Authorization': f'Bearer {API_KEY}',
        'X-Timestamp': str(int(time.time())),
        'Content-Type': 'application/json',
    }

    response = requests.request(
        method,
        f'{BASE_URL}{path}',
        headers=headers,
        json=data
    )
    return response.json()

# Create a pay-in intent
intent = make_request('POST', '/v1/intents/payin', {
    'tenantId': 'tenant_abc123',
    'userId': 'user_xyz789',
    'amountVnd': 1000000,
    'railsProvider': 'vietqr'
})
```

## Security Considerations

### Transport Security

- All API requests must use HTTPS (TLS 1.2+)
- HTTP requests are rejected
- Strict Transport Security (HSTS) is enforced

### Key Rotation

Recommended key rotation schedule:

| Environment | Rotation Period | Grace Period |
|-------------|-----------------|--------------|
| Production | 90 days | 7 days |
| Test | 180 days | 30 days |

### Audit Logging

All authentication events are logged:

- Successful authentications
- Failed authentication attempts
- API key rotations
- Tenant status changes

---

**Next:** [API Endpoints](./endpoints.md) | [Webhooks](./webhooks.md) | [Rate Limiting](./rate-limiting.md)
