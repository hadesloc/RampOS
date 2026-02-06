# Enterprise Configuration Guide

This guide covers all configuration options, environment variables, and settings for RampOS enterprise deployments.

---

## Configuration Methods

RampOS supports multiple configuration methods (in order of precedence):

1. **Environment Variables** - Highest priority
2. **Configuration File** - `config.toml` or `config.yaml`
3. **Command Line Arguments** - For specific overrides
4. **Default Values** - Built-in defaults

---

## Environment Variables

### Core Settings

| Variable | Description | Default | Required |
|----------|-------------|---------|----------|
| `ENVIRONMENT` | Environment name (dev/staging/prod) | `production` | No |
| `BIND_ADDRESS` | Server bind address | `0.0.0.0:8080` | No |
| `RUST_LOG` | Log level (trace/debug/info/warn/error) | `info` | No |
| `WORKERS` | Number of worker threads | CPU cores | No |

### Database

| Variable | Description | Default | Required |
|----------|-------------|---------|----------|
| `DATABASE_URL` | PostgreSQL connection string | - | **Yes** |
| `DATABASE_POOL_SIZE` | Connection pool size | `10` | No |
| `DATABASE_POOL_TIMEOUT` | Pool timeout in seconds | `30` | No |
| `DATABASE_STATEMENT_TIMEOUT` | Query timeout in seconds | `60` | No |
| `DATABASE_SSL_MODE` | SSL mode (disable/require/verify-ca/verify-full) | `require` | No |

**Example:**
```bash
DATABASE_URL="postgres://rampos:password@postgres.example.com:5432/rampos?sslmode=verify-full&sslrootcert=/certs/ca.pem"
DATABASE_POOL_SIZE=25
DATABASE_STATEMENT_TIMEOUT=120
```

### Redis

| Variable | Description | Default | Required |
|----------|-------------|---------|----------|
| `REDIS_URL` | Redis connection string | - | **Yes** |
| `REDIS_POOL_SIZE` | Connection pool size | `10` | No |
| `REDIS_CLUSTER_MODE` | Enable cluster mode | `false` | No |
| `REDIS_SENTINEL_MASTER` | Sentinel master name | - | No |
| `REDIS_SENTINEL_URLS` | Comma-separated sentinel URLs | - | No |

**Example (Single):**
```bash
REDIS_URL="redis://:password@redis.example.com:6379/0"
```

**Example (Sentinel):**
```bash
REDIS_SENTINEL_MASTER="mymaster"
REDIS_SENTINEL_URLS="redis://sentinel1:26379,redis://sentinel2:26379,redis://sentinel3:26379"
```

**Example (Cluster):**
```bash
REDIS_CLUSTER_MODE=true
REDIS_URL="redis://:password@redis-node1:6379,redis-node2:6379,redis-node3:6379"
```

### NATS

| Variable | Description | Default | Required |
|----------|-------------|---------|----------|
| `NATS_URL` | NATS server URL | - | **Yes** |
| `NATS_CLUSTER_URLS` | Additional cluster URLs | - | No |
| `NATS_USER` | NATS username | - | No |
| `NATS_PASSWORD` | NATS password | - | No |
| `NATS_TLS_CERT` | Path to TLS certificate | - | No |
| `NATS_TLS_KEY` | Path to TLS key | - | No |

**Example:**
```bash
NATS_URL="nats://nats1.example.com:4222,nats://nats2.example.com:4222"
NATS_USER="rampos"
NATS_PASSWORD="nats_password"
```

### Security

| Variable | Description | Default | Required |
|----------|-------------|---------|----------|
| `JWT_SECRET` | Secret for JWT signing (min 64 chars) | - | **Yes** |
| `JWT_EXPIRY` | JWT expiry duration | `24h` | No |
| `JWT_REFRESH_EXPIRY` | Refresh token expiry | `7d` | No |
| `WEBHOOK_SECRET` | Secret for webhook signatures | - | **Yes** |
| `ENCRYPTION_KEY` | Key for field-level encryption | - | **Yes** |
| `API_KEY_SALT` | Salt for API key hashing | - | No |

**Example:**
```bash
JWT_SECRET="your-very-long-jwt-secret-that-must-be-at-least-64-characters-long-for-security"
JWT_EXPIRY="12h"
WEBHOOK_SECRET="webhook_secret_32_chars_minimum"
ENCRYPTION_KEY="aes256_encryption_key_32chars"
```

### API Rate Limiting

| Variable | Description | Default | Required |
|----------|-------------|---------|----------|
| `RATE_LIMIT_ENABLED` | Enable rate limiting | `true` | No |
| `RATE_LIMIT_REQUESTS` | Requests per window | `100` | No |
| `RATE_LIMIT_WINDOW` | Window duration in seconds | `60` | No |
| `RATE_LIMIT_BURST` | Burst capacity | `20` | No |

**Example:**
```bash
RATE_LIMIT_ENABLED=true
RATE_LIMIT_REQUESTS=1000
RATE_LIMIT_WINDOW=60
RATE_LIMIT_BURST=50
```

### Observability

| Variable | Description | Default | Required |
|----------|-------------|---------|----------|
| `OTEL_EXPORTER_OTLP_ENDPOINT` | OpenTelemetry collector endpoint | - | No |
| `OTEL_SERVICE_NAME` | Service name for traces | `rampos` | No |
| `METRICS_ENABLED` | Enable Prometheus metrics | `true` | No |
| `METRICS_PORT` | Metrics endpoint port | `9090` | No |
| `TRACING_SAMPLE_RATE` | Trace sampling rate (0.0-1.0) | `0.1` | No |

**Example:**
```bash
OTEL_EXPORTER_OTLP_ENDPOINT="http://otel-collector:4317"
OTEL_SERVICE_NAME="rampos-production"
TRACING_SAMPLE_RATE=0.05
```

### SSO / Authentication

| Variable | Description | Default | Required |
|----------|-------------|---------|----------|
| `SSO_ENABLED` | Enable SSO | `false` | No |
| `SSO_PROVIDER` | SSO provider (saml/oidc) | `oidc` | No |
| `OIDC_ISSUER_URL` | OIDC issuer URL | - | If OIDC |
| `OIDC_CLIENT_ID` | OIDC client ID | - | If OIDC |
| `OIDC_CLIENT_SECRET` | OIDC client secret | - | If OIDC |
| `SAML_IDP_METADATA_URL` | SAML IdP metadata URL | - | If SAML |
| `SAML_SP_ENTITY_ID` | SAML SP entity ID | - | If SAML |
| `SAML_SP_ACS_URL` | SAML ACS URL | - | If SAML |

See [SSO Setup Guide](./sso-setup.md) for detailed configuration.

### Feature Flags

| Variable | Description | Default | Required |
|----------|-------------|---------|----------|
| `FEATURE_WEBHOOKS` | Enable webhooks | `true` | No |
| `FEATURE_AUDIT_LOGS` | Enable audit logging | `true` | No |
| `FEATURE_KYT` | Enable KYT checks | `true` | No |
| `FEATURE_SANCTIONS` | Enable sanctions screening | `true` | No |
| `FEATURE_MULTI_TENANT` | Enable multi-tenant mode | `true` | No |

### External Services

| Variable | Description | Default | Required |
|----------|-------------|---------|----------|
| `KYC_PROVIDER` | KYC provider (internal/sumsub/onfido) | `internal` | No |
| `KYC_PROVIDER_API_KEY` | KYC provider API key | - | If external |
| `KYT_PROVIDER` | KYT provider (internal/chainalysis/elliptic) | `internal` | No |
| `KYT_PROVIDER_API_KEY` | KYT provider API key | - | If external |
| `SANCTIONS_PROVIDER` | Sanctions provider (opensanctions/dow-jones) | `opensanctions` | No |

---

## Configuration File

### TOML Format

Create `config.toml`:

```toml
[server]
bind_address = "0.0.0.0:8080"
workers = 8
environment = "production"

[database]
url = "postgres://rampos:password@localhost:5432/rampos"
pool_size = 25
statement_timeout = 60
ssl_mode = "require"

[redis]
url = "redis://:password@localhost:6379"
pool_size = 20

[nats]
url = "nats://localhost:4222"

[security]
jwt_expiry = "12h"
jwt_refresh_expiry = "7d"

[rate_limiting]
enabled = true
requests = 1000
window = 60
burst = 50

[observability]
metrics_enabled = true
metrics_port = 9090
tracing_sample_rate = 0.05

[features]
webhooks = true
audit_logs = true
kyt = true
sanctions = true
multi_tenant = true

[tenants.default]
name = "Default Tenant"
limits.daily_payin = 1000000000000
limits.daily_payout = 500000000000
```

### YAML Format

Create `config.yaml`:

```yaml
server:
  bind_address: "0.0.0.0:8080"
  workers: 8
  environment: production

database:
  url: "postgres://rampos:password@localhost:5432/rampos"
  pool_size: 25
  statement_timeout: 60
  ssl_mode: require

redis:
  url: "redis://:password@localhost:6379"
  pool_size: 20

nats:
  url: "nats://localhost:4222"

security:
  jwt_expiry: "12h"
  jwt_refresh_expiry: "7d"

rate_limiting:
  enabled: true
  requests: 1000
  window: 60
  burst: 50

observability:
  metrics_enabled: true
  metrics_port: 9090
  tracing_sample_rate: 0.05

features:
  webhooks: true
  audit_logs: true
  kyt: true
  sanctions: true
  multi_tenant: true
```

---

## Kubernetes ConfigMaps

### Base ConfigMap

```yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: rampos-config
  namespace: rampos
data:
  ENVIRONMENT: "production"
  RUST_LOG: "info"
  BIND_ADDRESS: "0.0.0.0:8080"
  WORKERS: "8"

  # Database
  DATABASE_POOL_SIZE: "25"
  DATABASE_STATEMENT_TIMEOUT: "60"
  DATABASE_SSL_MODE: "require"

  # Redis
  REDIS_POOL_SIZE: "20"

  # Rate Limiting
  RATE_LIMIT_ENABLED: "true"
  RATE_LIMIT_REQUESTS: "1000"
  RATE_LIMIT_WINDOW: "60"

  # Observability
  METRICS_ENABLED: "true"
  METRICS_PORT: "9090"
  TRACING_SAMPLE_RATE: "0.05"

  # Features
  FEATURE_WEBHOOKS: "true"
  FEATURE_AUDIT_LOGS: "true"
  FEATURE_KYT: "true"
  FEATURE_SANCTIONS: "true"
```

### Using with Deployment

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: rampos-server
spec:
  template:
    spec:
      containers:
      - name: rampos
        envFrom:
        - configMapRef:
            name: rampos-config
        - secretRef:
            name: rampos-secrets
```

---

## Tenant Configuration

### Per-Tenant Settings

Configure tenant-specific settings in the database or via API:

```json
{
  "tenantId": "tenant_abc123",
  "name": "Acme Exchange",
  "settings": {
    "limits": {
      "dailyPayinVnd": 100000000000,
      "dailyPayoutVnd": 50000000000,
      "singleTransactionVnd": 10000000000
    },
    "kyc": {
      "provider": "sumsub",
      "autoApprovalEnabled": false,
      "requiredDocuments": ["ID_FRONT", "ID_BACK", "SELFIE"]
    },
    "webhooks": {
      "url": "https://acme.example.com/webhooks/rampos",
      "secret": "webhook_secret_here",
      "events": ["intent.completed", "intent.failed", "kyc.status_changed"]
    },
    "branding": {
      "primaryColor": "#1a73e8",
      "logo": "https://acme.example.com/logo.png"
    },
    "rateLimit": {
      "requestsPerMinute": 500,
      "burstCapacity": 100
    }
  }
}
```

### API for Tenant Configuration

```bash
# Get tenant settings
curl -X GET https://api.ramp.vn/v1/admin/tenants/tenant_abc123/settings \
  -H "Authorization: Bearer admin_token"

# Update tenant settings
curl -X PATCH https://api.ramp.vn/v1/admin/tenants/tenant_abc123/settings \
  -H "Authorization: Bearer admin_token" \
  -H "Content-Type: application/json" \
  -d '{
    "limits": {
      "dailyPayinVnd": 200000000000
    }
  }'
```

---

## Environment-Specific Configurations

### Development

```bash
# .env.development
ENVIRONMENT=development
RUST_LOG=debug
BIND_ADDRESS=127.0.0.1:8080

DATABASE_URL=postgres://rampos:rampos@localhost:5432/rampos_dev
DATABASE_POOL_SIZE=5

REDIS_URL=redis://localhost:6379

NATS_URL=nats://localhost:4222

JWT_SECRET=dev_jwt_secret_not_for_production_use_only_development
WEBHOOK_SECRET=dev_webhook_secret

RATE_LIMIT_ENABLED=false
FEATURE_SANCTIONS=false
```

### Staging

```bash
# .env.staging
ENVIRONMENT=staging
RUST_LOG=info

DATABASE_URL=postgres://rampos:PASSWORD@staging-db.example.com:5432/rampos_staging
DATABASE_POOL_SIZE=15
DATABASE_SSL_MODE=require

REDIS_URL=redis://:PASSWORD@staging-redis.example.com:6379

RATE_LIMIT_REQUESTS=500
TRACING_SAMPLE_RATE=0.2
```

### Production

```bash
# .env.production
ENVIRONMENT=production
RUST_LOG=info
WORKERS=16

DATABASE_URL=postgres://rampos:PASSWORD@prod-db.example.com:5432/rampos
DATABASE_POOL_SIZE=50
DATABASE_SSL_MODE=verify-full

REDIS_SENTINEL_MASTER=mymaster
REDIS_SENTINEL_URLS=redis://sentinel1:26379,redis://sentinel2:26379,redis://sentinel3:26379

NATS_URL=nats://nats1:4222,nats://nats2:4222,nats://nats3:4222

RATE_LIMIT_REQUESTS=1000
TRACING_SAMPLE_RATE=0.05

OTEL_EXPORTER_OTLP_ENDPOINT=http://otel-collector:4317
```

---

## Configuration Validation

### Validate Configuration

```bash
# Check configuration
./rampos-server config validate

# Show effective configuration
./rampos-server config show

# Test database connection
./rampos-server config test-db

# Test Redis connection
./rampos-server config test-redis
```

### Health Check Response

The `/health` endpoint returns configuration status:

```json
{
  "status": "healthy",
  "version": "1.0.0",
  "environment": "production",
  "checks": {
    "database": "ok",
    "redis": "ok",
    "nats": "ok"
  },
  "config": {
    "workers": 16,
    "rate_limiting": true,
    "features": {
      "webhooks": true,
      "audit_logs": true
    }
  }
}
```

---

## Secrets Management

### Using Kubernetes Secrets

```yaml
apiVersion: v1
kind: Secret
metadata:
  name: rampos-secrets
  namespace: rampos
type: Opaque
stringData:
  DATABASE_URL: "postgres://rampos:password@db:5432/rampos"
  REDIS_URL: "redis://:password@redis:6379"
  JWT_SECRET: "your-64-character-jwt-secret-here"
  WEBHOOK_SECRET: "your-webhook-secret"
  ENCRYPTION_KEY: "your-encryption-key"
```

### Using HashiCorp Vault

```bash
# Store secrets in Vault
vault kv put secret/rampos/production \
  DATABASE_URL="postgres://..." \
  JWT_SECRET="..." \
  WEBHOOK_SECRET="..."

# Use with External Secrets Operator
apiVersion: external-secrets.io/v1beta1
kind: ExternalSecret
metadata:
  name: rampos-secrets
spec:
  refreshInterval: 1h
  secretStoreRef:
    kind: ClusterSecretStore
    name: vault
  target:
    name: rampos-secrets
  dataFrom:
  - extract:
      key: secret/rampos/production
```

### Using AWS Secrets Manager

```bash
# Store secrets
aws secretsmanager create-secret \
  --name rampos/production \
  --secret-string '{"DATABASE_URL":"postgres://...","JWT_SECRET":"..."}'

# Use with External Secrets Operator
apiVersion: external-secrets.io/v1beta1
kind: ExternalSecret
metadata:
  name: rampos-secrets
spec:
  refreshInterval: 1h
  secretStoreRef:
    kind: ClusterSecretStore
    name: aws-secrets-manager
  target:
    name: rampos-secrets
  dataFrom:
  - extract:
      key: rampos/production
```

---

## Configuration Best Practices

### Security

1. **Never commit secrets** - Use environment variables or secret managers
2. **Rotate secrets regularly** - At least every 90 days
3. **Use strong secrets** - Minimum 64 characters for JWT, 32 for others
4. **Enable TLS** - Always use `sslmode=require` or higher for databases

### Performance

1. **Tune pool sizes** - Match database pool to expected concurrent connections
2. **Set timeouts** - Prevent runaway queries with statement timeouts
3. **Enable connection pooling** - Use PgBouncer for high-load scenarios

### Reliability

1. **Use HA configurations** - Redis Sentinel, PostgreSQL replicas
2. **Configure health checks** - Ensure Kubernetes probes are set
3. **Set resource limits** - Prevent resource exhaustion

---

**Version**: 1.0.0
**Last Updated**: 2026-02-06
