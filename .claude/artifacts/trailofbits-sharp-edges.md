# Trail of Bits Sharp Edges Analysis Report

**Date:** 2026-02-06
**Analyst:** Security Analyst (T-005)
**Scope:** Full codebase - Rust crates, Frontend, Kubernetes, Docker

---

## Executive Summary

This report identifies "sharp edges" - dangerous patterns, unsafe code, and potential vulnerabilities across the RampOS codebase. The analysis covers:

- **Rust Backend**: unwrap()/expect() usage, panic!() calls, unsafe blocks
- **Frontend**: XSS vulnerabilities, client-side secrets, localStorage usage
- **Infrastructure**: Kubernetes security, Docker Compose configuration

### Risk Summary

| Category | Critical | High | Medium | Low |
|----------|----------|------|--------|-----|
| Rust panic!() in production | 0 | 6 | 0 | 0 |
| Rust expect() in production | 0 | 8 | 0 | 0 |
| Frontend XSS | 0 | 0 | 0 | 0 |
| Client-side secrets | 0 | 0 | 1 | 0 |
| Infrastructure | 0 | 0 | 1 | 1 |

---

## 1. Rust Backend Analysis

### 1.1 Unsafe Blocks

**Status: PASS**

No `unsafe {}` blocks found in the codebase. This is a positive security indicator.

### 1.2 panic!() Calls in Production Code

**Status: HIGH RISK**

Found **6 panic!() calls** in non-test production code that could cause service crashes:

| File | Line | Context | Severity |
|------|------|---------|----------|
| `crates/ramp-compliance/src/providers/factory.rs` | 16 | `panic!("Onfido provider not yet implemented")` | HIGH |
| `crates/ramp-compliance/src/providers/factory.rs` | 20 | `panic!("Jumio provider not yet implemented")` | HIGH |
| `crates/ramp-compliance/src/providers/factory.rs` | 29 | `panic!("Chainalysis provider not yet implemented")` | HIGH |
| `crates/ramp-compliance/src/providers/factory.rs` | 32 | `panic!("Elliptic provider not yet implemented")` | HIGH |
| `crates/ramp-compliance/src/providers/factory.rs` | 85 | `panic!("S3 provider initialization requires async context")` | HIGH |
| `crates/ramp-compliance/src/providers/factory.rs` | 103 | `panic!("Failed to create S3 storage: {:?}", e)` | HIGH |

**Recommendation:** Replace panic!() with proper Result<T, E> error handling.

### 1.3 expect() Calls in Production Code

**Status: HIGH RISK**

Found **8 critical expect() calls** in production code that could crash the service:

| File | Line | Pattern | Risk |
|------|------|---------|------|
| `crates/ramp-aa/src/paymaster.rs` | 68 | `expect("Signer key must be exactly 32 bytes")` | HIGH - Startup crash |
| `crates/ramp-aa/src/paymaster.rs` | 71 | `expect("Invalid secp256k1 private key")` | HIGH - Startup crash |
| `crates/ramp-aa/src/paymaster.rs` | 132 | `expect("signing should not fail")` | HIGH - Runtime crash |
| `crates/ramp-adapter/src/adapters/vietqr.rs` | 120 | `expect("Failed to create HTTP client")` | HIGH - Startup crash |
| `crates/ramp-adapter/src/adapters/napas.rs` | 144 | `expect("Failed to create HTTP client")` | HIGH - Startup crash |
| `crates/ramp-core/src/service/webhook.rs` | 60 | `expect("Failed to create HTTP client")` | HIGH - Startup crash |
| `crates/ramp-compliance/src/providers/factory.rs` | 44 | `expect("OpenSanctions requires api_key")` | HIGH - Config error crash |
| `crates/ramp-compliance/src/providers/factory.rs` | 54, 97 | `expect("S3 requires bucket name")` | HIGH - Config error crash |

**Recommendation:**
1. For startup-time validations, these may be acceptable if the service cannot operate without valid config
2. For runtime operations (line 132), replace with proper error propagation

### 1.4 unwrap() in Production Code

**Status: MEDIUM RISK**

Found unwrap() calls in production code. Most critical ones are in test files (acceptable), but a few are in source:

| File | Context | Assessment |
|------|---------|------------|
| `crates/ramp-adapter/src/factory.rs:87,97,107,127` | RwLock unwrap | MEDIUM - Lock poisoning possible |
| `crates/ramp-compliance/src/aml/device_anomaly.rs:335,340,341,417` | Mutex lock unwrap | MEDIUM - Lock poisoning possible |

**Recommendation:** Use `lock().expect("Lock poisoned")` or handle PoisonError gracefully.

### 1.5 Infinite Loops

**Status: ACCEPTABLE WITH CAVEATS**

Found **6 loop {}** constructs in production code:

| File | Line | Purpose |
|------|------|---------|
| `crates/ramp-core/src/jobs/intent_timeout.rs` | 25 | Background job worker |
| `crates/ramp-core/src/temporal_worker.rs` | 248, 572, 605 | Temporal workflow workers |
| `crates/ramp-api/src/main.rs` | 214 | Signal handling loop |
| `crates/ramp-compliance/src/kyc/workflow.rs` | 135 | KYC workflow processor |

**Assessment:** These are intentional infinite loops for background workers. Ensure:
- All loops have proper exit conditions (shutdown signals)
- Timeouts are configured for any blocking operations within loops

### 1.6 SQL Injection Analysis

**Status: PASS**

All SQL queries use parameterized queries via SQLx bind parameters (`$1`, `$2`, etc.). No string interpolation in SQL detected.

```rust
// Example of safe pattern found:
sqlx::query("UPDATE intents SET state = $1 WHERE id = $2 AND tenant_id = $3")
```

---

## 2. Frontend Security Analysis

### 2.1 XSS Vulnerabilities (dangerouslySetInnerHTML)

**Status: PASS**

No `dangerouslySetInnerHTML` usage found in `frontend/src/` directory. All matches were in node_modules (third-party libraries).

### 2.2 localStorage Usage

**Status: PASS**

No direct `localStorage` usage found in `frontend/src/` directory. Authentication appears to use httpOnly cookies (secure pattern).

### 2.3 Client-Side Secret Exposure

**Status: MEDIUM RISK**

Found environment variable usage that needs review:

| File | Line | Variable | Assessment |
|------|------|----------|------------|
| `frontend/src/lib/portal-api.ts` | 12 | `NEXT_PUBLIC_API_URL` | SAFE - Public URL |
| `frontend/src/lib/api.ts` | 11 | `API_KEY` | SAFE - Server-side only check |
| `frontend/src/app/api/proxy/[...path]/route.ts` | 5-7 | `API_URL`, `API_KEY`, `RAMPOS_ADMIN_KEY` | SAFE - Server-side route |

**Positive Pattern Found:**
```typescript
// Server-side only access pattern (SAFE)
const API_KEY = typeof window === 'undefined' ? (process.env.API_KEY || '') : '';
```

### 2.4 eval() / innerHTML Usage

**Status: PASS**

No `eval()` or direct `innerHTML` assignment found in `frontend/src/` directory.

---

## 3. Infrastructure Security Analysis

### 3.1 Docker Compose (docker-compose.yml)

**Status: MOSTLY SECURE**

**Positive Findings:**
- Resource limits configured for all services
- Ports bound to 127.0.0.1 (localhost only) for databases
- Required secrets use `${VAR:?message}` syntax (fail if not set)
- Health checks configured

**Issues Found:**

| Issue | Severity | Location |
|-------|----------|----------|
| Redis default password in command | LOW | Line 46 |
| API port exposed to all interfaces | MEDIUM | Line 103 |

**Recommendations:**
1. Line 46: Use `${REDIS_PASSWORD:?required}` instead of default value
2. Line 103: Consider binding to 127.0.0.1:8080 if not behind load balancer

### 3.2 Kubernetes Security

**Status: EXCELLENT**

**Positive Findings in `k8s/base/deployment.yaml`:**
- `runAsNonRoot: true` - Containers cannot run as root
- `runAsUser: 1000` - Specific non-root user
- `fsGroup: 2000` - Filesystem group isolation
- `seccompProfile: RuntimeDefault` - Seccomp enabled
- `allowPrivilegeEscalation: false` - Cannot escalate privileges
- `capabilities: drop: ["ALL"]` - All Linux capabilities dropped
- Resource requests and limits configured
- Liveness and readiness probes configured

**Network Policies (`k8s/base/network-policy.yaml`):**
- Default deny all policy in place
- Per-service network policies configured
- Egress restricted to specific internal services
- External egress limited to HTTPS (port 443) only
- Private IP ranges explicitly blocked for external egress

### 3.3 Secrets Management

**Status: ACCEPTABLE**

`k8s/base/secret.example.yaml` contains:
- Clear warning: "DO NOT COMMIT REAL SECRETS"
- Recommendation to use SealedSecrets or External Secrets Operator
- Placeholder values only

**No actual secrets found committed to the repository.**

---

## 4. Recommendations Summary

### Critical (Fix Immediately)

1. **Replace panic!() in factory.rs** - Return proper errors instead of panicking when providers are not implemented

### High Priority

2. **Review expect() usage in paymaster.rs** - Ensure key validation happens at startup, not runtime
3. **Consider graceful degradation** for HTTP client creation failures

### Medium Priority

4. **Lock poisoning handling** - Use proper error handling for Mutex/RwLock unwrap
5. **Docker API port binding** - Bind to localhost if not behind reverse proxy

### Low Priority

6. **Redis default password** - Enforce required password in Docker Compose

---

## 5. Positive Security Patterns Observed

1. **No unsafe code** - Entire Rust codebase avoids unsafe blocks
2. **Parameterized SQL** - All queries use bind parameters
3. **HttpOnly cookies** - Authentication uses secure cookie patterns
4. **Server-side secrets** - Client code properly guards secret access
5. **Kubernetes hardening** - Comprehensive security context and network policies
6. **Defense in depth** - Multiple layers of security controls

---

## Appendix: Files Analyzed

### Rust Crates
- ramp-api
- ramp-core
- ramp-common
- ramp-ledger
- ramp-adapter
- ramp-aa
- ramp-compliance

### Frontend
- frontend/src/**/*.{ts,tsx}

### Infrastructure
- docker-compose.yml
- k8s/base/*.yaml
- k8s/monitoring/*.yaml
- k8s/jobs/*.yaml

---

*Report generated by Sharp Edges Security Analyst*
