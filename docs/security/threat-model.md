# RampOS Threat Model

**Version:** 1.0
**Last Updated:** 2026-02-02
**Classification:** Confidential

---

## Table of Contents

1. [System Overview](#1-system-overview)
2. [Threat Actors](#2-threat-actors)
3. [Assets](#3-assets)
4. [Attack Surface](#4-attack-surface)
5. [Attack Vectors](#5-attack-vectors)
6. [Mitigations](#6-mitigations)
7. [Risk Matrix](#7-risk-matrix)

---

## 1. System Overview

RampOS is a multi-tenant financial infrastructure platform providing:
- Fiat on/off-ramp services
- Account abstraction (ERC-4337) smart wallets
- AML/KYC compliance engine
- Multi-currency ledger system

### Architecture Components

```
[Internet] --> [Ingress/WAF] --> [API Gateway]
                                      |
                    +--------+--------+--------+
                    |        |        |        |
              [Auth MW] [Rate Limit] [Idempotency]
                    |
              [Business Logic]
                    |
        +-----+-----+-----+-----+
        |     |     |     |     |
    [Postgres] [Redis] [NATS] [Blockchain]
```

### Trust Boundaries

| Boundary | Description |
|----------|-------------|
| B1 | Internet to Ingress (untrusted) |
| B2 | Ingress to API (semi-trusted) |
| B3 | API to Database (trusted) |
| B4 | API to Blockchain (external) |
| B5 | Tenant to Tenant (isolated) |

---

## 2. Threat Actors

### 2.1 External Threat Actors

| Actor | Motivation | Capability | Targets |
|-------|------------|------------|---------|
| **Script Kiddies** | Notoriety, opportunism | Low - automated tools | Public endpoints, known CVEs |
| **Cybercriminals** | Financial gain | Medium - custom tooling | Payment systems, user funds |
| **Nation-State Actors** | Espionage, disruption | High - zero-days, APT | Infrastructure, crypto keys |
| **Competitors** | Business intelligence | Medium - social engineering | Trade secrets, customer data |
| **Hacktivists** | Ideology, publicity | Low-Medium | Public reputation, service availability |

### 2.2 Internal Threat Actors

| Actor | Motivation | Access Level | Risk Areas |
|-------|------------|--------------|------------|
| **Disgruntled Employee** | Revenge, financial | Admin access | Data exfiltration, sabotage |
| **Negligent Insider** | None (accidental) | Standard access | Misconfigurations, data leaks |
| **Compromised Account** | External control | Varies | Lateral movement |
| **Privileged Admin** | Financial, coercion | Full access | Key theft, backdoors |

### 2.3 User-Based Threat Actors

| Actor | Motivation | Capability | Targets |
|-------|------------|------------|---------|
| **Malicious Tenant** | Fraud, abuse | High (legitimate access) | Other tenants, platform resources |
| **Money Launderer** | Financial crime | Medium | AML controls, transaction limits |
| **Sanctioned Entity** | Evasion | Low-Medium | Sanctions screening |
| **Account Takeover** | Financial theft | Medium | User credentials, session tokens |

---

## 3. Assets

### 3.1 Critical Assets (Tier 1)

| Asset | Description | Impact if Compromised |
|-------|-------------|----------------------|
| **Private Keys** | Blockchain deployer, paymaster signer | Total loss of smart contract control, fund theft |
| **Database Credentials** | PostgreSQL master password | Full data breach |
| **API Master Keys** | Admin API keys | Platform takeover |
| **Customer Funds** | Fiat balances, crypto holdings | Financial loss, regulatory action |
| **KYC/PII Data** | Identity documents, SSN | Privacy violation, regulatory fines |

### 3.2 High-Value Assets (Tier 2)

| Asset | Description | Impact if Compromised |
|-------|-------------|----------------------|
| **Tenant API Keys** | Per-tenant authentication | Tenant impersonation |
| **Webhook Secrets** | HMAC signing keys | Webhook spoofing |
| **Session Keys** | ERC-4337 session authorization | Limited fund access |
| **Redis Data** | Rate limits, idempotency cache | Service disruption |
| **Audit Logs** | Transaction history, access logs | Compliance failure |

### 3.3 Standard Assets (Tier 3)

| Asset | Description | Impact if Compromised |
|-------|-------------|----------------------|
| **Configuration Data** | Non-secret configs | Service misconfiguration |
| **Application Logs** | Debug information | Information disclosure |
| **Public Keys** | Verification keys | None (public by design) |

---

## 4. Attack Surface

### 4.1 External Attack Surface

| Surface | Exposure | Protocol | Authentication |
|---------|----------|----------|----------------|
| REST API | Internet | HTTPS | Bearer Token + HMAC |
| Webhook Endpoints | Tenant servers | HTTPS | HMAC Signature |
| Smart Contracts | Blockchain | EVM | Signature verification |
| Admin Dashboard | Internal | HTTPS | SSO + MFA |

### 4.2 Internal Attack Surface

| Surface | Exposure | Protocol | Protection |
|---------|----------|----------|------------|
| PostgreSQL | Cluster-only | TCP/5432 | Password + SSL |
| Redis | Cluster-only | TCP/6379 | Password |
| NATS | Cluster-only | TCP/4222 | Token auth |
| Internal APIs | Cluster-only | HTTP/8080 | Service secret |

### 4.3 Supply Chain Attack Surface

| Component | Risk | Mitigation |
|-----------|------|------------|
| Rust crates | Malicious dependencies | Cargo audit, lockfile |
| npm packages | Dependency confusion | npm audit, lockfile |
| Docker base images | Backdoored images | Pin to SHA, scan |
| GitHub Actions | Compromised actions | Pin to SHA |
| OpenZeppelin contracts | Vulnerable versions | Version pinning, audit |

---

## 5. Attack Vectors

### 5.1 Authentication Attacks

#### AV-AUTH-001: API Key Brute Force
- **Description:** Attacker attempts to guess valid API keys
- **Likelihood:** Medium
- **Impact:** High (tenant impersonation)
- **Current Controls:** Rate limiting, key hashing
- **Gaps:** No progressive delay on failures

#### AV-AUTH-002: Session Key Theft
- **Description:** Attacker steals session key to access smart wallet
- **Likelihood:** Medium
- **Impact:** Medium (limited by session permissions)
- **Current Controls:** Time-bounded validity
- **Gaps:** Session keys have full access (no permission scoping)

#### AV-AUTH-003: Replay Attack
- **Description:** Attacker replays valid signed requests
- **Likelihood:** Medium
- **Impact:** High (duplicate transactions)
- **Current Controls:** Idempotency keys, timestamp validation
- **Gaps:** TypeScript SDK lacks timestamp in webhook verification

### 5.2 Authorization Attacks

#### AV-AUTHZ-001: Cross-Tenant Data Access
- **Description:** Tenant A accesses Tenant B's data
- **Likelihood:** Low (RLS in place)
- **Impact:** Critical
- **Current Controls:** Row-Level Security, tenant context
- **Gaps:**
  - RLS missing on 4 tables
  - Some queries missing tenant_id filter
  - RLS fails open if context unset

#### AV-AUTHZ-002: Admin Privilege Escalation
- **Description:** Regular tenant accesses admin endpoints
- **Likelihood:** Medium
- **Impact:** Critical
- **Current Controls:** Auth middleware on admin routes
- **Gaps:** No explicit admin role verification

#### AV-AUTHZ-003: Internal Service Bypass
- **Description:** Attacker spoofs internal service secret
- **Likelihood:** Low
- **Impact:** High
- **Current Controls:** Secret header validation
- **Gaps:** Non-constant-time comparison

### 5.3 Injection Attacks

#### AV-INJ-001: SQL Injection
- **Description:** Malicious SQL in user input
- **Likelihood:** Very Low
- **Impact:** Critical
- **Current Controls:** Parameterized queries (sqlx)
- **Gaps:** None identified

#### AV-INJ-002: LIKE Pattern DoS
- **Description:** Malicious search patterns cause slow queries
- **Likelihood:** Medium
- **Impact:** Low (DoS only)
- **Current Controls:** None
- **Gaps:** LIKE wildcards not escaped in search

#### AV-INJ-003: Smart Contract Input Validation
- **Description:** Malformed calldata to smart contracts
- **Likelihood:** Medium
- **Impact:** Varies
- **Current Controls:** Solidity type checking
- **Gaps:** No explicit input length limits on batches

### 5.4 Financial Attacks

#### AV-FIN-001: Double Spend via Race Condition
- **Description:** Submit duplicate transactions before idempotency lock
- **Likelihood:** Medium
- **Impact:** Critical
- **Current Controls:** Redis-based idempotency
- **Gaps:** Idempotency fails open on Redis error

#### AV-FIN-002: AML Rule Bypass
- **Description:** Structure transactions to avoid AML detection
- **Likelihood:** High
- **Impact:** High (regulatory)
- **Current Controls:** Structuring detection rule
- **Gaps:**
  - Rule only checks 90-100% of threshold
  - Device metadata bypass (defaults to Pass)
  - Sanctions check skipped on missing data

#### AV-FIN-003: Transaction Limit Manipulation
- **Description:** Admin sets unreasonably high limits
- **Likelihood:** Low
- **Impact:** High
- **Current Controls:** Admin auth required
- **Gaps:** No CHECK constraints on limit values

#### AV-FIN-004: Gas Griefing
- **Description:** Attacker drains paymaster funds via sponsored transactions
- **Likelihood:** Medium
- **Impact:** Medium (financial loss)
- **Current Controls:** Daily limits per tenant
- **Gaps:** Paymaster signature validation is placeholder

### 5.5 Cryptographic Attacks

#### AV-CRYPTO-001: Timing Attack on API Key Lookup
- **Description:** Measure response time to enumerate valid keys
- **Likelihood:** Low
- **Impact:** Medium
- **Current Controls:** Key hashing
- **Gaps:** Database lookup timing varies by key existence

#### AV-CRYPTO-002: Weak Webhook Signatures
- **Description:** Forge webhook signatures
- **Likelihood:** Low
- **Impact:** High
- **Current Controls:** HMAC-SHA256
- **Gaps:** Server uses hash of secret instead of actual secret

#### AV-CRYPTO-003: Signature Malleability
- **Description:** Modify signature without invalidating it
- **Likelihood:** Very Low
- **Impact:** Medium
- **Current Controls:** OpenZeppelin ECDSA (handles malleability)
- **Gaps:** None

### 5.6 Infrastructure Attacks

#### AV-INFRA-001: Container Escape
- **Description:** Break out of container to host
- **Likelihood:** Low
- **Impact:** Critical
- **Current Controls:** Non-root containers
- **Gaps:** Missing capabilities drop, writable rootfs

#### AV-INFRA-002: Network Lateral Movement
- **Description:** Compromised pod accesses other services
- **Likelihood:** Medium
- **Impact:** High
- **Current Controls:** Kubernetes namespace isolation
- **Gaps:** No NetworkPolicies defined

#### AV-INFRA-003: Secret Exfiltration
- **Description:** Attacker extracts secrets from environment
- **Likelihood:** Medium
- **Impact:** Critical
- **Current Controls:** Kubernetes Secrets
- **Gaps:**
  - Hardcoded credentials in docker-compose
  - Secret.example.yaml in kustomization
  - Migration scripts contain passwords

#### AV-INFRA-004: Supply Chain Compromise
- **Description:** Malicious code in dependencies
- **Likelihood:** Low
- **Impact:** Critical
- **Current Controls:** Cargo.lock, package-lock.json
- **Gaps:**
  - No container image scanning in CI
  - Actions not pinned to SHA
  - Base images use mutable tags

### 5.7 Denial of Service

#### AV-DOS-001: API Rate Limit Bypass
- **Description:** Exhaust resources despite rate limits
- **Likelihood:** Medium
- **Impact:** Medium
- **Current Controls:** Rate limiting, fail-open
- **Gaps:** Rate limiter fails open on Redis error

#### AV-DOS-002: Large Batch Execution
- **Description:** Submit very large batch transaction
- **Likelihood:** Medium
- **Impact:** Low
- **Current Controls:** Gas limits
- **Gaps:** No array length limits in executeBatch

#### AV-DOS-003: Database Connection Exhaustion
- **Description:** Open many connections to exhaust pool
- **Likelihood:** Medium
- **Impact:** High
- **Current Controls:** Connection pooling
- **Gaps:** No per-tenant connection limits

---

## 6. Mitigations

### 6.1 Authentication Mitigations

| ID | Attack Vector | Mitigation | Priority | Status |
|----|--------------|------------|----------|--------|
| M-AUTH-001 | API Key Brute Force | Implement progressive delays | High | Pending |
| M-AUTH-002 | Session Key Theft | Implement permission scoping | High | Pending |
| M-AUTH-003 | Replay Attack | Add timestamp to TS SDK webhook verify | Medium | Pending |
| M-AUTH-004 | HMAC Bypass | Implement server-side HMAC verification | High | Pending |

### 6.2 Authorization Mitigations

| ID | Attack Vector | Mitigation | Priority | Status |
|----|--------------|------------|----------|--------|
| M-AUTHZ-001 | Cross-Tenant Access | Add RLS to missing tables | Critical | Pending |
| M-AUTHZ-002 | Cross-Tenant Access | Fix fail-open RLS policy | Critical | Pending |
| M-AUTHZ-003 | Admin Escalation | Add admin role middleware | High | Pending |
| M-AUTHZ-004 | Internal Bypass | Use constant-time comparison | Medium | Pending |

### 6.3 Financial Mitigations

| ID | Attack Vector | Mitigation | Priority | Status |
|----|--------------|------------|----------|--------|
| M-FIN-001 | Double Spend | Consider fail-closed for idempotency | High | Pending |
| M-FIN-002 | AML Bypass | Fix structuring rule threshold | High | Pending |
| M-FIN-003 | AML Bypass | Fail-closed on missing device metadata | Critical | Pending |
| M-FIN-004 | AML Bypass | Fail-closed on missing sanctions data | Critical | Pending |
| M-FIN-005 | Limit Manipulation | Add CHECK constraints | Medium | Pending |
| M-FIN-006 | Gas Griefing | Implement paymaster signature verification | Critical | Pending |

### 6.4 Infrastructure Mitigations

| ID | Attack Vector | Mitigation | Priority | Status |
|----|--------------|------------|----------|--------|
| M-INFRA-001 | Container Escape | Add capabilities drop | High | Pending |
| M-INFRA-002 | Container Escape | Set readOnlyRootFilesystem | Medium | Pending |
| M-INFRA-003 | Lateral Movement | Implement NetworkPolicies | Critical | Pending |
| M-INFRA-004 | Secret Exfiltration | Remove hardcoded credentials | Critical | Pending |
| M-INFRA-005 | Supply Chain | Add container image scanning | High | Pending |
| M-INFRA-006 | Supply Chain | Pin GitHub Actions to SHA | Medium | Pending |

### 6.5 Data Protection Mitigations

| ID | Attack Vector | Mitigation | Priority | Status |
|----|--------------|------------|----------|--------|
| M-DATA-001 | PII Exposure | Encrypt KYC data at rest | High | Pending |
| M-DATA-002 | PII Exposure | Sanitize PII from logs | High | Pending |
| M-DATA-003 | Information Leak | Sanitize error messages | Medium | Pending |

---

## 7. Risk Matrix

### 7.1 Risk Scoring Criteria

**Likelihood:**
- Very Low (1): Requires significant resources/expertise
- Low (2): Requires specialized knowledge
- Medium (3): Feasible with moderate effort
- High (4): Easily exploitable

**Impact:**
- Low (1): Minor service disruption
- Medium (2): Limited data exposure or financial loss
- High (3): Significant data breach or financial loss
- Critical (4): Complete system compromise or major regulatory violation

**Risk Score:** Likelihood x Impact

### 7.2 Current Risk Assessment

| Risk ID | Attack Vector | Likelihood | Impact | Score | Priority |
|---------|--------------|------------|--------|-------|----------|
| R-001 | AV-FIN-004 (Paymaster bypass) | 3 | 4 | 12 | Critical |
| R-002 | AV-AUTHZ-001 (Cross-tenant) | 2 | 4 | 8 | Critical |
| R-003 | AV-FIN-002 (AML bypass) | 4 | 3 | 12 | Critical |
| R-004 | AV-INFRA-003 (Secret leak) | 3 | 4 | 12 | Critical |
| R-005 | AV-INFRA-002 (Lateral movement) | 3 | 3 | 9 | High |
| R-006 | AV-FIN-001 (Double spend) | 2 | 4 | 8 | High |
| R-007 | AV-AUTHZ-002 (Admin escalation) | 2 | 4 | 8 | High |
| R-008 | AV-AUTH-002 (Session key) | 3 | 2 | 6 | Medium |
| R-009 | AV-CRYPTO-002 (Webhook) | 2 | 3 | 6 | Medium |
| R-010 | AV-DOS-001 (Rate limit) | 3 | 2 | 6 | Medium |

### 7.3 Risk Heat Map

```
                    IMPACT
              Low  Med  High  Crit
         +----+----+----+----+
   High  |    |    | R3 |    |
Likeli-  +----+----+----+----+
 hood    | R10| R8 | R5 | R1,R4|
  Med    +----+----+----+----+
         |    | R9 | R7 |R2,R6|
   Low   +----+----+----+----+
         |    |    |    |    |
  V.Low  +----+----+----+----+
```

### 7.4 Residual Risk After Mitigations

After implementing all recommended mitigations:

| Risk ID | Current Score | Target Score | Residual Risk |
|---------|---------------|--------------|---------------|
| R-001 | 12 | 2 | Low (signature verification) |
| R-002 | 8 | 2 | Low (RLS + fail-closed) |
| R-003 | 12 | 4 | Low (improved AML rules) |
| R-004 | 12 | 2 | Low (secrets in Vault) |
| R-005 | 9 | 2 | Low (NetworkPolicies) |
| R-006 | 8 | 4 | Low (fail-closed idempotency) |
| R-007 | 8 | 2 | Low (RBAC middleware) |
| R-008 | 6 | 3 | Low (permission scoping) |
| R-009 | 6 | 2 | Low (proper HMAC key) |
| R-010 | 6 | 4 | Accepted (fail-open trade-off) |

---

## Appendix A: STRIDE Analysis Summary

| Threat Type | Applicable Vectors | Overall Risk |
|-------------|-------------------|--------------|
| **Spoofing** | AV-AUTH-*, AV-CRYPTO-002 | Medium |
| **Tampering** | AV-INJ-*, AV-FIN-001 | Low |
| **Repudiation** | (Audit logs in place) | Low |
| **Information Disclosure** | AV-AUTHZ-001, AV-INFRA-003 | High |
| **Denial of Service** | AV-DOS-* | Medium |
| **Elevation of Privilege** | AV-AUTHZ-002, AV-FIN-004 | High |

---

## Appendix B: Compliance Mapping

| Requirement | Standard | Relevant Threats | Status |
|-------------|----------|------------------|--------|
| Data Encryption | PCI DSS 3.4 | AV-INFRA-003 | Partial |
| Access Control | SOC 2 CC6.1 | AV-AUTHZ-* | Partial |
| Network Segmentation | PCI DSS 1.2 | AV-INFRA-002 | Pending |
| Key Management | PCI DSS 3.5 | AV-CRYPTO-* | Partial |
| AML Controls | BSA/FinCEN | AV-FIN-002 | Partial |
| Audit Logging | SOC 2 CC7.2 | (In place) | Compliant |

---

**Document Owner:** Security Team
**Review Frequency:** Quarterly
**Next Review:** 2026-05-02
