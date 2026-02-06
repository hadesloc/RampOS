# RampOS Security & Infrastructure Remediation Plan
**Created:** 2026-02-06
**Duration:** 4 weeks
**Goal:** Production-ready security and infrastructure

---

## Phase 1: Critical Security Fixes (Week 1)

### TASK-001: Rotate All Secrets
**Priority:** P0 CRITICAL | **Estimate:** 2 hours | **Owner:** security-worker

**Subtasks:**
- [ ] T-001.1: Audit current .env file for all secrets
- [ ] T-001.2: Generate new PostgreSQL password (32+ chars)
- [ ] T-001.3: Generate new Redis password (32+ chars)
- [ ] T-001.4: Generate new NATS credentials
- [ ] T-001.5: Generate new JWT signing keys
- [ ] T-001.6: Generate new API keys for all tenants
- [ ] T-001.7: Update .env.example with new variable names
- [ ] T-001.8: Update docker-compose.yml with new env vars
- [ ] T-001.9: Update k8s secrets manifests
- [ ] T-001.10: Document rotation procedure in docs/SECURITY.md

**Acceptance Criteria:**
- No secrets match previous values
- All services start successfully with new secrets
- .env.example updated without actual values

---

### TASK-002: Fix RLS Bypass Vulnerability (DB-002)
**Priority:** P0 CRITICAL | **Estimate:** 3 hours | **Owner:** security-worker

**Subtasks:**
- [ ] T-002.1: Read current RLS policies in migrations/
- [ ] T-002.2: Identify all policies using `app.current_tenant`
- [ ] T-002.3: Create migration to fix policies with COALESCE/fail-closed
- [ ] T-002.4: Update policy: `WHERE tenant_id = COALESCE(current_setting('app.current_tenant', true)::uuid, '00000000-0000-0000-0000-000000000000'::uuid)`
- [ ] T-002.5: Add CHECK constraint to prevent null tenant_id
- [ ] T-002.6: Write integration test for RLS bypass attempt
- [ ] T-002.7: Test with missing app.current_tenant setting
- [ ] T-002.8: Run full test suite to verify no regressions

**Acceptance Criteria:**
- RLS policies fail closed when tenant not set
- Integration test proves bypass is blocked
- All existing tests pass

---

### TASK-003: Update Next.js to Fix CVE
**Priority:** P0 CRITICAL | **Estimate:** 1 hour | **Owner:** frontend-worker

**Subtasks:**
- [ ] T-003.1: Check current Next.js version in frontend/package.json
- [ ] T-003.2: Check current Next.js version in frontend-landing/package.json
- [ ] T-003.3: Update frontend/package.json to next@15.4.7 or later
- [ ] T-003.4: Update frontend-landing/package.json to next@15.4.7 or later
- [ ] T-003.5: Run `npm install` in frontend/
- [ ] T-003.6: Run `npm install` in frontend-landing/
- [ ] T-003.7: Run `npm run build` in frontend/
- [ ] T-003.8: Run `npm run build` in frontend-landing/
- [ ] T-003.9: Test middleware authorization flows
- [ ] T-003.10: Update package-lock.json files

**Acceptance Criteria:**
- Next.js >= 15.4.7 in both frontends
- Both frontends build without errors
- No CVE warnings in `npm audit`

---

### TASK-004: Enable Redis Authentication
**Priority:** P0 CRITICAL | **Estimate:** 2 hours | **Owner:** devops-worker

**Subtasks:**
- [ ] T-004.1: Generate strong Redis password (32+ chars)
- [ ] T-004.2: Update docker-compose.yml Redis service with password
- [ ] T-004.3: Update k8s/base/redis-statefulset.yaml with auth
- [ ] T-004.4: Create Redis secret in k8s/base/redis-secret.yaml
- [ ] T-004.5: Update Rust connection string in config
- [ ] T-004.6: Update crates/ramp-common redis client initialization
- [ ] T-004.7: Test Redis connection with auth locally
- [ ] T-004.8: Verify all Redis-dependent tests pass

**Acceptance Criteria:**
- Redis requires authentication
- Application connects successfully with password
- Unauthorized connections are rejected

---

### TASK-005: Apply Network Policies
**Priority:** P1 HIGH | **Estimate:** 2 hours | **Owner:** devops-worker

**Subtasks:**
- [ ] T-005.1: Review existing k8s/base/network-policy.yaml
- [ ] T-005.2: Verify default deny-all ingress policy
- [ ] T-005.3: Add explicit allow for API -> PostgreSQL
- [ ] T-005.4: Add explicit allow for API -> Redis
- [ ] T-005.5: Add explicit allow for API -> NATS
- [ ] T-005.6: Add explicit allow for Prometheus -> API metrics
- [ ] T-005.7: Create network policy test script
- [ ] T-005.8: Document network topology in docs/

**Acceptance Criteria:**
- Default deny-all policy active
- Only required paths allowed
- Inter-pod communication verified

---

## Phase 2: Rust Panic Remediation (Week 2)

### TASK-006: Replace .expect() in ramp-api
**Priority:** P1 HIGH | **Estimate:** 4 hours | **Owner:** rust-worker-1

**Subtasks:**
- [ ] T-006.1: Run `grep -rn "\.expect(" crates/ramp-api/src/`
- [ ] T-006.2: List all occurrences with file:line
- [ ] T-006.3: Replace each .expect() with ? operator
- [ ] T-006.4: Update function signatures to return Result where needed
- [ ] T-006.5: Add proper error types in ramp-common
- [ ] T-006.6: Run `cargo check -p ramp-api`
- [ ] T-006.7: Run `cargo test -p ramp-api`
- [ ] T-006.8: Run `cargo clippy -p ramp-api`

**Acceptance Criteria:**
- Zero .expect() in ramp-api/src (excluding tests)
- All tests pass
- No clippy warnings

---

### TASK-007: Replace .expect() in ramp-core
**Priority:** P1 HIGH | **Estimate:** 4 hours | **Owner:** rust-worker-2

**Subtasks:**
- [ ] T-007.1: Run `grep -rn "\.expect(" crates/ramp-core/src/`
- [ ] T-007.2: List all occurrences with file:line
- [ ] T-007.3: Replace each .expect() with ? operator
- [ ] T-007.4: Update function signatures to return Result
- [ ] T-007.5: Propagate errors properly through call chain
- [ ] T-007.6: Run `cargo check -p ramp-core`
- [ ] T-007.7: Run `cargo test -p ramp-core`
- [ ] T-007.8: Run `cargo clippy -p ramp-core`

**Acceptance Criteria:**
- Zero .expect() in ramp-core/src (excluding tests)
- All tests pass
- No clippy warnings

---

### TASK-008: Replace .expect() in ramp-aa
**Priority:** P1 HIGH | **Estimate:** 3 hours | **Owner:** rust-worker-1

**Subtasks:**
- [ ] T-008.1: Run `grep -rn "\.expect(" crates/ramp-aa/src/`
- [ ] T-008.2: Focus on paymaster.rs:66-71 (constructor panics)
- [ ] T-008.3: Change PaymasterService::new() to return Result
- [ ] T-008.4: Update all callers to handle Result
- [ ] T-008.5: Replace remaining .expect() calls
- [ ] T-008.6: Run `cargo check -p ramp-aa`
- [ ] T-008.7: Run `cargo test -p ramp-aa`

**Acceptance Criteria:**
- PaymasterService::new() returns Result
- Zero .expect() in production code
- All tests pass

---

### TASK-009: Replace .expect() in ramp-compliance
**Priority:** P1 HIGH | **Estimate:** 3 hours | **Owner:** rust-worker-2

**Subtasks:**
- [ ] T-009.1: Run `grep -rn "\.expect(" crates/ramp-compliance/src/`
- [ ] T-009.2: List all occurrences
- [ ] T-009.3: Replace with proper error handling
- [ ] T-009.4: Run `cargo check -p ramp-compliance`
- [ ] T-009.5: Run `cargo test -p ramp-compliance`
- [ ] T-009.6: Run fuzz tests to verify

**Acceptance Criteria:**
- Zero .expect() in production code
- All tests including fuzz pass

---

### TASK-010: Replace .expect() in remaining crates
**Priority:** P1 HIGH | **Estimate:** 4 hours | **Owner:** rust-worker-1

**Subtasks:**
- [ ] T-010.1: Fix ramp-ledger .expect() calls
- [ ] T-010.2: Fix ramp-adapter .expect() calls
- [ ] T-010.3: Fix ramp-common .expect() calls
- [ ] T-010.4: Run `cargo check --workspace`
- [ ] T-010.5: Run `cargo test --workspace`
- [ ] T-010.6: Run `cargo clippy --workspace`

**Acceptance Criteria:**
- Zero .expect() in any production code
- Full workspace builds and tests pass

---

## Phase 3: Contract Security Tests (Week 2-3)

### TASK-011: Add Contract Invariant Tests
**Priority:** P0 CRITICAL | **Estimate:** 6 hours | **Owner:** contract-worker

**Subtasks:**
- [ ] T-011.1: Create contracts/test/invariants/InvariantBase.t.sol
- [ ] T-011.2: Add invariant_TotalLiabilityNeverNegative
- [ ] T-011.3: Add invariant_SessionKeyCannotEscalatePrivileges
- [ ] T-011.4: Add invariant_PaymasterBalanceConsistent
- [ ] T-011.5: Add invariant_TimelockAlwaysEnforced
- [ ] T-011.6: Add invariant_OnlyOwnerCanUpgrade
- [ ] T-011.7: Configure foundry.toml for invariant runs
- [ ] T-011.8: Run `forge test --match-contract Invariant`
- [ ] T-011.9: Fix any invariant violations found
- [ ] T-011.10: Add invariant tests to CI

**Acceptance Criteria:**
- 5+ invariant tests defined
- All invariants pass with 10000+ runs
- CI runs invariants on every PR

---

### TASK-012: Add Contract Fuzz Tests
**Priority:** P0 CRITICAL | **Estimate:** 6 hours | **Owner:** contract-worker

**Subtasks:**
- [ ] T-012.1: Add fuzz test for execute() with random calldata
- [ ] T-012.2: Add fuzz test for executeBatch() with random operations
- [ ] T-012.3: Add fuzz test for addSessionKey() with random params
- [ ] T-012.4: Add fuzz test for validatePaymasterUserOp()
- [ ] T-012.5: Add fuzz test for deposit/withdraw flows
- [ ] T-012.6: Configure foundry.toml fuzz runs (10000+)
- [ ] T-012.7: Run `forge test --fuzz-runs 10000`
- [ ] T-012.8: Document any edge cases found

**Acceptance Criteria:**
- Fuzz tests for all public functions
- No failures in 10000 runs
- Edge cases documented

---

### TASK-013: Add Reentrancy Attack Tests
**Priority:** P1 HIGH | **Estimate:** 3 hours | **Owner:** contract-worker

**Subtasks:**
- [ ] T-013.1: Create ReentrancyAttacker.sol mock contract
- [ ] T-013.2: Test reentrancy on execute()
- [ ] T-013.3: Test reentrancy on executeBatch()
- [ ] T-013.4: Test reentrancy on Paymaster withdraw
- [ ] T-013.5: Verify CEI pattern blocks all attacks
- [ ] T-013.6: Document attack vectors tested

**Acceptance Criteria:**
- All reentrancy attempts blocked
- Test proves CEI pattern effectiveness

---

## Phase 4: Infrastructure HA (Week 3)

### TASK-014: PostgreSQL High Availability
**Priority:** P0 CRITICAL | **Estimate:** 8 hours | **Owner:** devops-worker

**Subtasks:**
- [ ] T-014.1: Choose HA solution (Patroni vs managed)
- [ ] T-014.2: Create k8s/base/postgres-ha/ directory
- [ ] T-014.3: Configure Patroni ConfigMap
- [ ] T-014.4: Create StatefulSet with 3 replicas
- [ ] T-014.5: Configure PodDisruptionBudget (minAvailable: 2)
- [ ] T-014.6: Set up streaming replication
- [ ] T-014.7: Configure automatic failover
- [ ] T-014.8: Test failover scenario
- [ ] T-014.9: Update application connection string for HA
- [ ] T-014.10: Document HA architecture

**Acceptance Criteria:**
- 3-node PostgreSQL cluster running
- Automatic failover works
- Zero data loss on primary failure

---

### TASK-015: Redis High Availability
**Priority:** P1 HIGH | **Estimate:** 4 hours | **Owner:** devops-worker

**Subtasks:**
- [ ] T-015.1: Create k8s/base/redis-sentinel/ directory
- [ ] T-015.2: Configure Redis Sentinel with 3 sentinels
- [ ] T-015.3: Configure 3 Redis replicas
- [ ] T-015.4: Set up automatic failover
- [ ] T-015.5: Update application to use Sentinel
- [ ] T-015.6: Test failover scenario
- [ ] T-015.7: Configure PodDisruptionBudget

**Acceptance Criteria:**
- 3-node Redis cluster with Sentinel
- Automatic failover works
- Application reconnects on failover

---

### TASK-016: NATS JetStream Cluster
**Priority:** P1 HIGH | **Estimate:** 4 hours | **Owner:** devops-worker

**Subtasks:**
- [ ] T-016.1: Update k8s/base/nats-statefulset.yaml to 3 replicas
- [ ] T-016.2: Configure JetStream clustering
- [ ] T-016.3: Set up stream replication factor: 3
- [ ] T-016.4: Configure PodDisruptionBudget
- [ ] T-016.5: Test message durability on node failure
- [ ] T-016.6: Update monitoring for cluster health

**Acceptance Criteria:**
- 3-node NATS cluster
- Messages survive node failure
- Automatic rebalancing works

---

### TASK-017: Backup and Disaster Recovery
**Priority:** P0 CRITICAL | **Estimate:** 6 hours | **Owner:** devops-worker

**Subtasks:**
- [ ] T-017.1: Install Velero in cluster
- [ ] T-017.2: Configure S3-compatible backup storage
- [ ] T-017.3: Create backup schedule (hourly for 24h, daily for 7d)
- [ ] T-017.4: Configure PostgreSQL WAL archiving
- [ ] T-017.5: Create backup verification job
- [ ] T-017.6: Document restore procedure
- [ ] T-017.7: Test full cluster restore
- [ ] T-017.8: Set RTO < 15min, RPO < 1min targets

**Acceptance Criteria:**
- Automated hourly backups
- Verified restore procedure
- RTO/RPO targets documented

---

## Phase 5: CI/CD Pipeline (Week 3-4)

### TASK-018: Create Rust CI Workflow
**Priority:** P1 HIGH | **Estimate:** 4 hours | **Owner:** devops-worker

**Subtasks:**
- [ ] T-018.1: Create .github/workflows/rust-ci.yml
- [ ] T-018.2: Add cargo check step
- [ ] T-018.3: Add cargo test step
- [ ] T-018.4: Add cargo clippy step
- [ ] T-018.5: Add cargo fmt --check step
- [ ] T-018.6: Add cargo audit step
- [ ] T-018.7: Configure test matrix (stable, nightly)
- [ ] T-018.8: Add coverage reporting with tarpaulin
- [ ] T-018.9: Set up caching for faster builds

**Acceptance Criteria:**
- CI runs on every PR
- Tests, clippy, fmt all pass
- Coverage reported

---

### TASK-019: Create Solidity CI Workflow
**Priority:** P1 HIGH | **Estimate:** 3 hours | **Owner:** devops-worker

**Subtasks:**
- [ ] T-019.1: Create .github/workflows/solidity-ci.yml
- [ ] T-019.2: Add forge build step
- [ ] T-019.3: Add forge test step
- [ ] T-019.4: Add forge test --fuzz-runs 1000 step
- [ ] T-019.5: Add forge coverage step
- [ ] T-019.6: Add slither static analysis
- [ ] T-019.7: Configure Foundry caching

**Acceptance Criteria:**
- CI runs on every PR
- All tests pass
- Coverage > 80%

---

### TASK-020: Create Frontend CI Workflow
**Priority:** P2 MEDIUM | **Estimate:** 3 hours | **Owner:** devops-worker

**Subtasks:**
- [ ] T-020.1: Create .github/workflows/frontend-ci.yml
- [ ] T-020.2: Add npm ci step for both frontends
- [ ] T-020.3: Add npm run lint step
- [ ] T-020.4: Add npm run build step
- [ ] T-020.5: Add npm run test step (when tests exist)
- [ ] T-020.6: Add npm audit step
- [ ] T-020.7: Configure Node.js caching

**Acceptance Criteria:**
- CI runs on every PR
- Both frontends build successfully
- No high/critical npm audit issues

---

### TASK-021: Create Deployment Pipeline
**Priority:** P1 HIGH | **Estimate:** 6 hours | **Owner:** devops-worker

**Subtasks:**
- [ ] T-021.1: Create .github/workflows/deploy.yml
- [ ] T-021.2: Add Docker build and push step
- [ ] T-021.3: Add staging deployment trigger
- [ ] T-021.4: Add production deployment (manual approval)
- [ ] T-021.5: Configure ArgoCD sync
- [ ] T-021.6: Add smoke test after deployment
- [ ] T-021.7: Add rollback capability
- [ ] T-021.8: Document deployment process

**Acceptance Criteria:**
- Automated staging deployments
- Manual approval for production
- Rollback tested

---

## Phase 6: Observability (Week 4)

### TASK-022: Add OpenTelemetry Tracing
**Priority:** P2 MEDIUM | **Estimate:** 6 hours | **Owner:** rust-worker-1

**Subtasks:**
- [ ] T-022.1: Add opentelemetry dependencies to Cargo.toml
- [ ] T-022.2: Configure OTLP exporter in ramp-api
- [ ] T-022.3: Add tracing spans to intent flows
- [ ] T-022.4: Add tracing to compliance checks
- [ ] T-022.5: Add tracing to ledger operations
- [ ] T-022.6: Deploy Jaeger or Tempo collector
- [ ] T-022.7: Create trace dashboards in Grafana
- [ ] T-022.8: Document trace IDs for debugging

**Acceptance Criteria:**
- Full request tracing visible
- Spans for all critical operations
- Grafana dashboards working

---

### TASK-023: Add Log Aggregation
**Priority:** P2 MEDIUM | **Estimate:** 4 hours | **Owner:** devops-worker

**Subtasks:**
- [ ] T-023.1: Deploy Loki stack in k8s
- [ ] T-023.2: Configure Promtail for log collection
- [ ] T-023.3: Set up log retention (30 days)
- [ ] T-023.4: Create Grafana dashboards for logs
- [ ] T-023.5: Add log-based alerts
- [ ] T-023.6: Document log query patterns

**Acceptance Criteria:**
- All pod logs aggregated
- Searchable in Grafana
- Alerts for error patterns

---

## Summary

| Phase | Duration | Tasks | Priority |
|-------|----------|-------|----------|
| Phase 1 | Week 1 | T-001 to T-005 | P0 CRITICAL |
| Phase 2 | Week 2 | T-006 to T-010 | P1 HIGH |
| Phase 3 | Week 2-3 | T-011 to T-013 | P0/P1 |
| Phase 4 | Week 3 | T-014 to T-017 | P0/P1 |
| Phase 5 | Week 3-4 | T-018 to T-021 | P1/P2 |
| Phase 6 | Week 4 | T-022 to T-023 | P2 |

**Total Tasks:** 23
**Total Subtasks:** 180+
**Estimated Duration:** 4 weeks
**Team Size:** 5-6 workers

---

*Implementation Plan generated by Ultimate Workflow*
*2026-02-06*
