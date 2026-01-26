# Deployment Security Checklist

## 1. Infrastructure & Environment
- [ ] **Secrets Management**
  - [ ] All secrets injected via environment variables (no `.env` files in image).
  - [ ] Database credentials rotated.
  - [ ] API keys for external services (banking rails, nodes) verified.
  - [ ] `JWT_SECRET` is at least 32 characters and high entropy.
- [ ] **Network Security**
  - [ ] TLS 1.2+ enforced on all ingress points.
  - [ ] Database not publicly accessible (VPC peering / private network).
  - [ ] Redis not publicly accessible.
  - [ ] Administrative ports (SSH, RDP) closed or VPN-gated.
- [ ] **Container Security**
  - [ ] Image scanned for vulnerabilities (Trivy).
  - [ ] Running as non-root user.
  - [ ] Read-only root filesystem enabled (where possible).
  - [ ] Resource limits (CPU/RAM) configured in Kubernetes.

## 2. Application Configuration
- [ ] **Database**
  - [ ] Migrations applied successfully.
  - [ ] App user has least-privilege access (no DDL permissions).
  - [ ] Connection pool size configured appropriately.
- [ ] **Logging & Monitoring**
  - [ ] Log level set to `INFO` (not `DEBUG`).
  - [ ] PII masking enabled in logs.
  - [ ] Sentry/Error tracking DSN configured.
  - [ ] Prometheus metrics endpoint protected or internal-only.
- [ ] **Feature Flags**
  - [ ] Debug endpoints disabled.
  - [ ] Swagger UI disabled in production (or behind auth).
  - [ ] "Test Mode" flags disabled for banking connectors.

## 3. Compliance & Logic
- [ ] **Limits**
  - [ ] Default transaction limits set for all tenants.
  - [ ] Rate limiting enabled and tuned.
  - [ ] Velocity checks enabled.
- [ ] **Sanctions**
  - [ ] Sanctions list updated (if using local cache).
  - [ ] PEP screening enabled.
- [ ] **Ledger**
  - [ ] Initial reconciliation passed (if migrating data).
  - [ ] Idempotency keys TTL configured correctly in Redis.

## 4. Smart Contracts (If deploying new contracts)
- [ ] **Verification**
  - [ ] Contracts verified on Etherscan/Blockscout.
  - [ ] Owner wallet keys secured (Hardware wallet/Multisig).
  - [ ] Paymaster deposit funded.
- [ ] **Parameters**
  - [ ] Gas limits configured.
  - [ ] Supported tokens allowed in Paymaster.

## 5. Final Verification
- [ ] **Penetration Test**
  - [ ] Critical findings resolved.
  - [ ] Retest confirmed.
- [ ] **Backup**
  - [ ] Automated backup schedule active.
  - [ ] Restore procedure tested.
- [ ] **Emergency**
  - [ ] Kill switch (circuit breaker) functionality verified.
  - [ ] On-call roster confirmed.
