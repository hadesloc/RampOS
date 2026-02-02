# RampOS Security Hardening Guide

**Version:** 1.0
**Last Updated:** 2026-02-02
**Classification:** Internal Use Only

---

## Table of Contents

1. [Security Checklist](#1-security-checklist)
2. [Production Hardening Steps](#2-production-hardening-steps)
3. [Secret Management Best Practices](#3-secret-management-best-practices)
4. [Container Security](#4-container-security)
5. [Database Security](#5-database-security)
6. [Network Security](#6-network-security)
7. [API Security](#7-api-security)
8. [Smart Contract Security](#8-smart-contract-security)
9. [Monitoring and Alerting](#9-monitoring-and-alerting)

---

## 1. Security Checklist

### Pre-Deployment Checklist

#### Infrastructure
- [ ] All containers run as non-root users
- [ ] `readOnlyRootFilesystem: true` set for all containers
- [ ] `capabilities: drop: ["ALL"]` applied to all containers
- [ ] NetworkPolicies deployed for all workloads
- [ ] RBAC configured with dedicated ServiceAccounts
- [ ] Container images pinned to SHA digests
- [ ] Ingress rate limiting configured
- [ ] TLS certificates valid and auto-renewed

#### Secrets
- [ ] No hardcoded credentials in source code
- [ ] All secrets stored in Kubernetes Secrets or Vault
- [ ] Docker Compose uses `.env` file (not committed)
- [ ] Secret rotation procedures documented and tested
- [ ] API keys use unique values per environment
- [ ] Database passwords meet complexity requirements

#### Database
- [ ] Row-Level Security (RLS) enabled on all tenant tables
- [ ] RLS policies use fail-closed pattern
- [ ] Separate database roles for app, readonly, and system
- [ ] KYC/PII data encrypted at rest
- [ ] Audit logging enabled
- [ ] Connection pooling configured with limits

#### API
- [ ] HMAC signature verification enabled server-side
- [ ] CORS restricted to specific origins and methods
- [ ] Rate limiting configured per-tenant and per-endpoint
- [ ] Admin endpoints have RBAC protection
- [ ] Error messages sanitized (no internal details leaked)
- [ ] Request body size limits configured

#### Smart Contracts
- [ ] All signatures verified cryptographically
- [ ] Paymaster centralization risks documented
- [ ] Session key permissions implemented (not full access)
- [ ] Multi-sig ownership for critical contracts
- [ ] Timelock for admin functions

### Runtime Checklist

- [ ] Security monitoring enabled (alerts configured)
- [ ] Log aggregation active (no PII in logs)
- [ ] Intrusion detection system running
- [ ] Regular vulnerability scans scheduled
- [ ] Incident response plan documented

---

## 2. Production Hardening Steps

### 2.1 Kubernetes Hardening

#### Step 1: Add Pod Security Context

Apply to all deployments:

```yaml
spec:
  template:
    spec:
      securityContext:
        runAsNonRoot: true
        runAsUser: 1000
        fsGroup: 2000
        seccompProfile:
          type: RuntimeDefault
      containers:
      - name: app
        securityContext:
          allowPrivilegeEscalation: false
          capabilities:
            drop: ["ALL"]
          readOnlyRootFilesystem: true
```

#### Step 2: Implement NetworkPolicies

Create `k8s/base/network-policy.yaml`:

```yaml
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: rampos-server-policy
  namespace: rampos
spec:
  podSelector:
    matchLabels:
      app: rampos-server
  policyTypes:
  - Ingress
  - Egress
  ingress:
  - from:
    - namespaceSelector:
        matchLabels:
          kubernetes.io/metadata.name: ingress-nginx
    ports:
    - port: 8080
  egress:
  - to:
    - podSelector:
        matchLabels:
          app: rampos-postgres
    ports:
    - port: 5432
  - to:
    - podSelector:
        matchLabels:
          app: rampos-redis
    ports:
    - port: 6379
  - to:
    - podSelector:
        matchLabels:
          app: rampos-nats
    ports:
    - port: 4222
---
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: deny-database-external
  namespace: rampos
spec:
  podSelector:
    matchLabels:
      app: rampos-postgres
  policyTypes:
  - Ingress
  ingress:
  - from:
    - podSelector:
        matchLabels:
          app: rampos-server
    ports:
    - port: 5432
```

#### Step 3: Configure RBAC

Create `k8s/base/rbac.yaml`:

```yaml
apiVersion: v1
kind: ServiceAccount
metadata:
  name: rampos-server
  namespace: rampos
automountServiceAccountToken: false
---
apiVersion: rbac.authorization.k8s.io/v1
kind: Role
metadata:
  name: rampos-server-role
  namespace: rampos
rules:
- apiGroups: [""]
  resources: ["configmaps"]
  verbs: ["get", "list"]
---
apiVersion: rbac.authorization.k8s.io/v1
kind: RoleBinding
metadata:
  name: rampos-server-rolebinding
  namespace: rampos
roleRef:
  apiGroup: rbac.authorization.k8s.io
  kind: Role
  name: rampos-server-role
subjects:
- kind: ServiceAccount
  name: rampos-server
  namespace: rampos
```

#### Step 4: Pin Container Images

Replace mutable tags with SHA digests:

```yaml
# Bad
image: ghcr.io/rampos/rampos:latest

# Good
image: ghcr.io/rampos/rampos:v1.0.0@sha256:abc123...
```

### 2.2 Database Hardening

#### Step 1: Fix RLS Policies

Update RLS to fail-closed:

```sql
-- Update existing policies to use fail-closed pattern
DROP POLICY IF EXISTS tenant_isolation_users ON users;
CREATE POLICY tenant_isolation_users ON users
    USING (tenant_id = COALESCE(
        NULLIF(current_setting('app.current_tenant', true), ''),
        'INVALID_TENANT'
    ));
```

#### Step 2: Add RLS to Missing Tables

```sql
-- Add tenant_id to tables missing it
ALTER TABLE risk_score_history ADD COLUMN tenant_id VARCHAR(255);
ALTER TABLE case_notes ADD COLUMN tenant_id VARCHAR(255);

-- Enable RLS on all tables
ALTER TABLE aml_rule_versions ENABLE ROW LEVEL SECURITY;
ALTER TABLE risk_score_history ENABLE ROW LEVEL SECURITY;
ALTER TABLE case_notes ENABLE ROW LEVEL SECURITY;
ALTER TABLE compliance_transactions ENABLE ROW LEVEL SECURITY;

-- Create isolation policies
CREATE POLICY tenant_isolation ON aml_rule_versions
    USING (tenant_id = COALESCE(NULLIF(current_setting('app.current_tenant', true), ''), 'INVALID'));
```

#### Step 3: Create Separate Database Roles

```sql
-- Application role (least privilege)
CREATE ROLE ramp_app LOGIN PASSWORD 'strong_password_here';
GRANT SELECT, INSERT, UPDATE ON ALL TABLES IN SCHEMA public TO ramp_app;
REVOKE DELETE ON ALL TABLES IN SCHEMA public FROM ramp_app;

-- Readonly role for reporting
CREATE ROLE ramp_readonly;
GRANT SELECT ON ALL TABLES IN SCHEMA public TO ramp_readonly;

-- System role for background workers (bypasses RLS)
CREATE ROLE ramp_system BYPASSRLS LOGIN PASSWORD 'different_strong_password';
GRANT ALL ON ALL TABLES IN SCHEMA public TO ramp_system;
```

#### Step 4: Encrypt Sensitive Data

```sql
-- Enable pgcrypto
CREATE EXTENSION IF NOT EXISTS pgcrypto;

-- For new PII data, use encryption
ALTER TABLE kyc_records
ADD COLUMN verification_data_encrypted BYTEA;

-- Migration to encrypt existing data
UPDATE kyc_records
SET verification_data_encrypted = pgp_sym_encrypt(
    verification_data::text,
    current_setting('app.encryption_key')
);
```

### 2.3 API Hardening

#### Step 1: Implement HMAC Verification

Add to `auth.rs`:

```rust
// Verify HMAC signature
let provided_signature = headers.get("X-Signature")
    .and_then(|v| v.to_str().ok());

if let (Some(sig), Some(secret)) = (provided_signature, tenant.api_secret) {
    let message = format!("{}\n{}\n{}\n{}", method, path, timestamp, body);
    let expected = hmac_sha256(&secret, &message);
    if !constant_time_eq(sig, &expected) {
        return Err(StatusCode::UNAUTHORIZED);
    }
}
```

#### Step 2: Restrict CORS

```rust
CorsLayer::new()
    .allow_origin(origins)
    .allow_methods([Method::GET, Method::POST, Method::PATCH, Method::DELETE, Method::OPTIONS])
    .allow_headers([
        header::CONTENT_TYPE,
        header::AUTHORIZATION,
        header::ACCEPT,
        HeaderName::from_static("x-timestamp"),
        HeaderName::from_static("x-signature"),
        HeaderName::from_static("x-idempotency-key"),
    ])
    .allow_credentials(false)
    .max_age(Duration::from_secs(3600))
```

#### Step 3: Sanitize Error Messages

```rust
// Replace detailed errors with generic messages
ramp_common::Error::Database(e) => {
    tracing::error!(error = %e, "Database error");
    ApiError::Internal("An internal error occurred".to_string())
}
```

---

## 3. Secret Management Best Practices

### 3.1 Secret Storage Hierarchy

| Environment | Method | Tools |
|-------------|--------|-------|
| Local Dev | `.env` file (gitignored) | dotenv |
| CI/CD | GitHub Secrets / GitLab CI Variables | GitHub Actions |
| Staging | Kubernetes Secrets | kubectl |
| Production | HashiCorp Vault or External Secrets Operator | Vault, ESO |

### 3.2 Required Secrets

| Secret Name | Description | Rotation Frequency |
|-------------|-------------|-------------------|
| `DATABASE_URL` | PostgreSQL connection string | 90 days |
| `REDIS_URL` | Redis connection with password | 90 days |
| `JWT_SECRET` | JWT signing key | 30 days |
| `API_KEY_SALT` | Salt for API key hashing | Never (versioned) |
| `WEBHOOK_SIGNING_KEY` | Webhook HMAC key | 90 days |
| `INTERNAL_SERVICE_SECRET` | Internal service auth | 30 days |
| `DEPLOYER_PRIVATE_KEY` | Blockchain deployer key | Per deployment |
| `PAYMASTER_SIGNER` | Paymaster signing key | 90 days |

### 3.3 Secret Rotation Procedure

#### Step 1: Generate New Secret

```bash
# Generate a new random secret
openssl rand -base64 32
```

#### Step 2: Update in Secret Store

```bash
# For Kubernetes
kubectl create secret generic rampos-secrets \
  --from-literal=DATABASE_PASSWORD=new_password \
  --dry-run=client -o yaml | kubectl apply -f -

# For Vault
vault kv put secret/rampos/prod DATABASE_PASSWORD=new_password
```

#### Step 3: Rolling Restart

```bash
# Restart deployments to pick up new secrets
kubectl rollout restart deployment/rampos-server -n rampos
```

#### Step 4: Verify and Revoke Old Secret

```bash
# Verify application is working with new secret
curl -f https://api.rampos.io/health

# Revoke old secret (database password example)
psql -c "ALTER USER rampos PASSWORD 'revoked_$(date +%s)';"
```

### 3.4 Emergency Secret Revocation

In case of secret compromise:

1. **Immediate**: Revoke the compromised credential at its source
2. **Generate**: Create new credentials
3. **Deploy**: Update secrets in all environments
4. **Restart**: Rolling restart all affected services
5. **Audit**: Review access logs for unauthorized use
6. **Report**: Document incident per IR procedure

### 3.5 Secrets Audit Script

Create `scripts/audit-secrets.sh`:

```bash
#!/bin/bash
# Audit for hardcoded secrets in codebase

echo "=== Scanning for hardcoded secrets ==="

# Check for common patterns
grep -rn "password\s*=\s*['\"]" --include="*.rs" --include="*.ts" --include="*.yaml" . || true
grep -rn "api_key\s*=\s*['\"]" --include="*.rs" --include="*.ts" --include="*.yaml" . || true
grep -rn "secret\s*=\s*['\"]" --include="*.rs" --include="*.ts" --include="*.yaml" . || true
grep -rn "sk_live_" --include="*.rs" --include="*.ts" . || true
grep -rn "0x[a-fA-F0-9]{64}" --include="*.rs" --include="*.ts" . || true

echo "=== Checking .env files ==="
find . -name ".env" -not -path "./node_modules/*" -exec echo "WARNING: .env file found: {}" \;

echo "=== Audit complete ==="
```

---

## 4. Container Security

### 4.1 Dockerfile Best Practices

```dockerfile
# Use specific version with SHA
FROM rust:1.75-bookworm@sha256:abc123... as builder

# Create non-root user
RUN useradd -m -u 1000 appuser

# Build application
WORKDIR /app
COPY . .
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim@sha256:def456...

# Copy binary and set ownership
COPY --from=builder /app/target/release/rampos-server /app/rampos-server
COPY --from=builder /etc/passwd /etc/passwd

# Run as non-root
USER appuser

# Health check
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
  CMD ["/app/rampos-server", "healthcheck"]

ENTRYPOINT ["/app/rampos-server"]
```

### 4.2 Image Scanning

Add to CI/CD pipeline:

```yaml
- name: Scan image for vulnerabilities
  uses: aquasecurity/trivy-action@0.16.0
  with:
    image-ref: ${{ env.REGISTRY }}/${{ env.IMAGE_NAME }}:${{ github.sha }}
    format: 'sarif'
    output: 'trivy-results.sarif'
    severity: 'CRITICAL,HIGH'
    exit-code: '1'
```

---

## 5. Database Security

### 5.1 Connection Security

- Use SSL/TLS for all database connections
- Configure connection pooling limits
- Enable query logging for audit

```yaml
# PostgreSQL configuration
ssl: true
ssl_mode: verify-full
max_connections: 100
log_statement: 'ddl'
```

### 5.2 Data Classification

| Classification | Examples | Protection |
|----------------|----------|------------|
| PUBLIC | API documentation | None |
| INTERNAL | Tenant IDs, intent states | Access control |
| CONFIDENTIAL | User emails, KYC status | Encryption + audit |
| RESTRICTED | Private keys, full SSN | HSM + encryption + strict audit |

---

## 6. Network Security

### 6.1 TLS Configuration

- Minimum TLS 1.2
- Prefer TLS 1.3
- Strong cipher suites only
- HSTS enabled with 1-year max-age

### 6.2 Firewall Rules

| Source | Destination | Port | Protocol | Action |
|--------|-------------|------|----------|--------|
| Internet | Ingress | 443 | HTTPS | Allow |
| Ingress | API Server | 8080 | HTTP | Allow |
| API Server | PostgreSQL | 5432 | TCP | Allow |
| API Server | Redis | 6379 | TCP | Allow |
| API Server | NATS | 4222 | TCP | Allow |
| * | * | * | * | Deny |

---

## 7. API Security

### 7.1 Rate Limiting Configuration

| Endpoint Type | Limit | Window |
|---------------|-------|--------|
| Global | 1000 req | 1 minute |
| Per-Tenant | 100 req | 1 minute |
| Authentication | 10 req | 1 minute |
| Admin | 20 req | 1 minute |

### 7.2 Authentication Flow

1. Client sends request with `Authorization: Bearer <api_key>` and `X-Timestamp`
2. Server hashes API key with SHA-256
3. Server looks up tenant by hash
4. Server validates timestamp within tolerance (5 min past, 1 min future)
5. (Optional) Server verifies HMAC signature if present

---

## 8. Smart Contract Security

### 8.1 Deployment Checklist

- [ ] All tests passing
- [ ] External audit completed
- [ ] Multisig ownership configured
- [ ] Timelock for admin functions
- [ ] Emergency pause mechanism
- [ ] Upgrade path documented

### 8.2 Key Management

- Store deployer private key in HSM
- Use separate keys for each environment
- Never reuse keys across chains
- Implement key rotation for paymaster signer

---

## 9. Monitoring and Alerting

### 9.1 Security Alerts

| Alert | Condition | Severity |
|-------|-----------|----------|
| Failed Auth Spike | >50 failures in 5 min | HIGH |
| Rate Limit Hit | >10 429s per tenant/min | MEDIUM |
| Database Error Spike | >10 errors in 1 min | HIGH |
| Secret Access | Any vault access | INFO |
| Admin Action | Any admin endpoint call | INFO |

### 9.2 Log Retention

| Log Type | Retention | Storage |
|----------|-----------|---------|
| Access logs | 90 days | CloudWatch/Loki |
| Security logs | 1 year | S3 + Glacier |
| Audit logs | 7 years | Immutable storage |
| Application logs | 30 days | CloudWatch/Loki |

---

## Appendix A: Quick Reference Commands

```bash
# Check container security context
kubectl get pods -n rampos -o jsonpath='{range .items[*]}{.metadata.name}{"\t"}{.spec.containers[*].securityContext}{"\n"}{end}'

# Verify NetworkPolicies
kubectl get networkpolicies -n rampos

# Check secrets (names only, not values)
kubectl get secrets -n rampos

# Rotate a specific secret
kubectl create secret generic rampos-secrets --from-literal=KEY=new_value --dry-run=client -o yaml | kubectl apply -f -

# Force pod restart after secret update
kubectl rollout restart deployment/rampos-server -n rampos

# Audit database connections
psql -c "SELECT usename, application_name, client_addr, state FROM pg_stat_activity;"
```

---

**Document Owner:** Security Team
**Review Frequency:** Monthly
**Next Review:** 2026-03-02
