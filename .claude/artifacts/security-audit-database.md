# Security Audit Report: Database Schema & SQL

**Audit Date:** 2026-02-02
**Auditor:** Security Audit Agent
**Scope:** Database migrations, SQL queries, Row-Level Security (RLS) policies

---

## Executive Summary

This audit examined the RampOS database schema across 8 migration files and 7 repository modules. The codebase demonstrates **good security practices** overall, with proper use of prepared statements and RLS implementation. However, several **CRITICAL** and **HIGH** severity issues were identified that require immediate attention.

| Severity | Count |
|----------|-------|
| CRITICAL | 3 |
| HIGH | 6 |
| MEDIUM | 8 |
| LOW | 5 |

---

## 1. SQL Injection Analysis

### 1.1 Prepared Statements Usage

**Status: SECURE**

All SQL queries use parameterized queries via `sqlx::query` with proper bind parameters (`$1`, `$2`, etc.). The codebase correctly uses:

- `sqlx::query().bind()` pattern consistently
- `QueryBuilder::push_bind()` for dynamic queries
- No string concatenation of user input into SQL

**Evidence:**
```rust
// Good: All repositories use prepared statements
sqlx::query("SELECT * FROM users WHERE tenant_id = $1 AND id = $2")
    .bind(&tenant_id.0)
    .bind(&user_id.0)
```

### 1.2 Dynamic Query Building

**Status: SECURE WITH CAVEATS**

The `QueryBuilder` pattern in `user.rs` correctly uses `push_bind()` for dynamic filters:

```rust
// user.rs lines 213-227
let mut builder = QueryBuilder::new("SELECT * FROM users WHERE tenant_id = ");
builder.push_bind(&tenant_id.0);

if let Some(search) = search {
    let pattern = format!("%{}%", search);  // MEDIUM: Potential wildcard injection
    builder.push(" AND id ILIKE ").push_bind(pattern);
}
```

**Finding DB-001 (MEDIUM):** The search pattern uses `%` wildcards without sanitization. While not SQL injection, malicious patterns like `%_%_%_%` could cause performance degradation via regex denial-of-service.

**Recommendation:** Escape special LIKE characters (`%`, `_`) in user input before pattern construction.

---

## 2. Row-Level Security (RLS) Analysis

### 2.1 RLS Policy Configuration

**Status: PARTIALLY SECURE**

**Location:** `migrations/006_enable_rls.sql`

RLS is enabled on 11 tables with tenant isolation policies:

| Table | RLS Enabled | Policy Defined |
|-------|-------------|----------------|
| users | Yes | Yes |
| intents | Yes | Yes |
| ledger_entries | Yes | Yes |
| account_balances | Yes | Yes |
| webhook_events | Yes | Yes |
| rails_adapters | Yes | Yes |
| virtual_accounts | Yes | Yes |
| kyc_records | Yes | Yes |
| aml_cases | Yes | Yes |
| audit_log | Yes | Yes |
| recon_batches | Yes | Yes |

### 2.2 RLS Policy Weaknesses

**Finding DB-002 (CRITICAL): RLS Bypass via Unset Session Variable**

**Location:** `migrations/006_enable_rls.sql` lines 18-59

All RLS policies rely on `current_setting('app.current_tenant')`:

```sql
CREATE POLICY tenant_isolation_users ON users
    USING (tenant_id = current_setting('app.current_tenant')::VARCHAR);
```

**Vulnerability:** If `app.current_tenant` is not set:
- PostgreSQL raises an error by default
- However, if `missing_ok` parameter is used or the setting is empty, the policy could fail open

**Attack Vector:** A misconfigured connection or middleware bypass could result in unset tenant context, potentially exposing all tenant data or blocking all access.

**Recommendation:**
```sql
-- Use COALESCE with a non-matching value to fail closed
USING (tenant_id = COALESCE(NULLIF(current_setting('app.current_tenant', true), ''), 'INVALID_TENANT'))
```

---

**Finding DB-003 (CRITICAL): Missing RLS on New Tables**

The following tables DO NOT have RLS enabled:

| Table | Migration | Security Impact |
|-------|-----------|-----------------|
| `aml_rule_versions` | 003_rule_versions.sql | **HIGH** - AML rules could be accessed cross-tenant |
| `risk_score_history` | 004_score_history.sql | **HIGH** - Risk scores leak sensitive user behavior |
| `case_notes` | 005_case_notes.sql | **HIGH** - Internal compliance notes exposed |
| `compliance_transactions` | 007_compliance_transactions.sql | **CRITICAL** - Transaction history accessible cross-tenant |

**Recommendation:** Add RLS policies to all these tables:

```sql
-- Required additions to migrations
ALTER TABLE aml_rule_versions ENABLE ROW LEVEL SECURITY;
CREATE POLICY tenant_isolation_aml_rule_versions ON aml_rule_versions
    USING (tenant_id = current_setting('app.current_tenant')::VARCHAR);

ALTER TABLE risk_score_history ENABLE ROW LEVEL SECURITY;
-- Note: risk_score_history needs tenant_id column added first

ALTER TABLE case_notes ENABLE ROW LEVEL SECURITY;
-- Note: case_notes needs tenant_id column added first

ALTER TABLE compliance_transactions ENABLE ROW LEVEL SECURITY;
CREATE POLICY tenant_isolation_compliance_transactions ON compliance_transactions
    USING (tenant_id = current_setting('app.current_tenant')::VARCHAR);
```

---

**Finding DB-004 (CRITICAL): Missing tenant_id in Schema**

**Location:** `migrations/004_score_history.sql`, `migrations/005_case_notes.sql`

These tables lack `tenant_id` column entirely, making RLS enforcement impossible:

```sql
-- risk_score_history has NO tenant_id
CREATE TABLE risk_score_history (
  id UUID PRIMARY KEY,
  user_id VARCHAR(255) NOT NULL,  -- No tenant context!
  ...
);

-- case_notes has NO tenant_id
CREATE TABLE case_notes (
  id UUID PRIMARY KEY,
  case_id VARCHAR(255) NOT NULL,  -- No tenant context!
  ...
);
```

**Recommendation:** Add `tenant_id` to these tables via migration and join through related entities for lookup.

---

### 2.3 RLS Context Setting

**Status: IMPLEMENTED CORRECTLY**

**Location:** `repository/mod.rs` lines 55-64

```rust
pub async fn set_rls_context(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: &TenantId,
) -> Result<(), sqlx::Error> {
    sqlx::query("SELECT set_config('app.current_tenant', $1, true)")
        .bind(&tenant_id.0)
        .execute(&mut **tx)
        .await?;
    Ok(())
}
```

**Positive:** The third parameter `true` makes the setting local to the transaction, preventing leakage between requests.

---

## 3. Tenant Isolation Vulnerabilities

**Finding DB-005 (HIGH): System Worker Queries Bypass Tenant Isolation**

**Location:** `repository/intent.rs` lines 335-380, `repository/webhook.rs` lines 94-114

```rust
// intent.rs - list_expired() does NOT set RLS context
async fn list_expired(&self, limit: i64) -> Result<Vec<IntentRow>> {
    let rows = sqlx::query_as::<_, IntentRow>(
        r#"
        SELECT * FROM intents
        WHERE expires_at < NOW()
          AND state NOT IN ('COMPLETED', 'EXPIRED', ...)
        ORDER BY expires_at ASC
        LIMIT $1
        "#,
    )
    .bind(limit)
    .fetch_all(&self.pool)  // No RLS context set!
    .await...
}

// webhook.rs - get_pending_events() does NOT set RLS context
async fn get_pending_events(&self, limit: i64) -> Result<Vec<WebhookEventRow>> {
    let rows = sqlx::query_as::<_, WebhookEventRow>(
        r#"
        SELECT * FROM webhook_events
        WHERE status = 'PENDING'
        ...
        FOR UPDATE SKIP LOCKED
        "#,
    )
    .bind(limit)
    .fetch_all(&self.pool)  // No RLS context set!
    .await...
}
```

**Vulnerability:** These system maintenance queries:
1. Either fail silently (return 0 rows) when RLS is enforced without context
2. Or expose ALL tenants' data if the database user has `BYPASSRLS` privilege

**Recommendation:**
1. Create a dedicated `ramp_system` database role with `BYPASSRLS` for background workers only
2. Use separate connection pools for tenant-scoped vs system operations
3. Document which queries require elevated privileges

---

**Finding DB-006 (HIGH): get_case() Missing Tenant Validation**

**Location:** `ramp-compliance/src/store/postgres.rs` lines 92-147

```rust
async fn get_case(&self, case_id: &str) -> Result<Option<AmlCase>> {
    let row = sqlx::query(
        r#"
        SELECT ...
        FROM compliance_cases
        WHERE id = $1  -- NO tenant_id filter!
        "#,
    )
    .bind(case_id)
    .fetch_optional(&self.pool)
    .await?;
```

**Vulnerability:** Any tenant can fetch any case by ID if they guess or enumerate case IDs.

**Recommendation:**
```rust
async fn get_case(&self, tenant_id: &TenantId, case_id: &str) -> Result<Option<AmlCase>> {
    sqlx::query("... WHERE id = $1 AND tenant_id = $2")
        .bind(case_id)
        .bind(tenant_id.to_string())
```

---

**Finding DB-007 (HIGH): get_version() Missing Tenant Validation**

**Location:** `ramp-compliance/src/rules/version.rs` lines 196-224

```rust
pub async fn get_version(&self, version_id: Uuid) -> Result<RuleVersion> {
    let row = sqlx::query_as::<_, RuleVersion>(
        r#"
        SELECT ... FROM aml_rule_versions
        WHERE id = $1  -- NO tenant_id filter!
        "#,
    )
    .bind(version_id)
    ...
}
```

**Vulnerability:** Cross-tenant access to AML rule configurations.

**Recommendation:** Add `tenant_id` parameter and filter to all lookup methods.

---

**Finding DB-008 (HIGH): get_notes() Missing Tenant Validation**

**Location:** `ramp-compliance/src/store/postgres.rs` lines 318-349

```rust
async fn get_notes(&self, case_id: &str) -> Result<Vec<CaseNote>> {
    let rows = sqlx::query(
        r#"
        SELECT ... FROM case_notes
        WHERE case_id = $1  -- No tenant isolation
        ORDER BY created_at DESC
        "#,
    )
    .bind(case_id)
    ...
}
```

**Vulnerability:** Internal compliance notes from other tenants could be accessed.

---

## 4. Privilege Escalation Risks

**Finding DB-009 (MEDIUM): No Role Separation in Schema**

**Location:** `migrations/001_initial_schema.sql`

The schema does not define:
- Separate database roles for application vs admin operations
- GRANT/REVOKE statements for principle of least privilege
- Read-only roles for reporting

**Recommendation:**
```sql
-- Add to migrations
CREATE ROLE ramp_app LOGIN PASSWORD 'xxx';
CREATE ROLE ramp_readonly;
CREATE ROLE ramp_system BYPASSRLS;

GRANT SELECT, INSERT, UPDATE ON ALL TABLES IN SCHEMA public TO ramp_app;
GRANT SELECT ON ALL TABLES IN SCHEMA public TO ramp_readonly;
GRANT ALL ON ALL TABLES IN SCHEMA public TO ramp_system;

-- Revoke direct table access
REVOKE ALL ON tenants FROM ramp_app;
-- ... etc
```

---

**Finding DB-010 (MEDIUM): Tenant Limit Updates Not Validated**

**Location:** `repository/tenant.rs` lines 131-148

```rust
async fn update_limits(
    &self,
    id: &TenantId,
    daily_payin: Option<Decimal>,
    daily_payout: Option<Decimal>,
) -> Result<()> {
    sqlx::query(
        "UPDATE tenants SET daily_payin_limit_vnd = $1, daily_payout_limit_vnd = $2 WHERE id = $3",
    )
    ...
}
```

**Vulnerability:** No business logic validation prevents setting unreasonably high limits (e.g., $10 trillion) that could facilitate money laundering.

**Recommendation:** Add CHECK constraints:
```sql
ALTER TABLE tenants ADD CONSTRAINT check_reasonable_limits
    CHECK (daily_payin_limit_vnd <= 100000000000 AND daily_payout_limit_vnd <= 100000000000);
```

---

## 5. Sensitive Data Encryption

**Finding DB-011 (HIGH): Inconsistent Encryption for Credentials**

**Location:** `migrations/001_initial_schema.sql` lines 17-20

```sql
CREATE TABLE tenants (
    api_key_hash VARCHAR(255) NOT NULL,     -- Hashed (good)
    webhook_secret_hash VARCHAR(255) NOT NULL, -- Hashed (good)
    ...
);

CREATE TABLE rails_adapters (
    config_encrypted BYTEA NOT NULL,  -- Encrypted (good)
    ...
);
```

**Status:** API keys and webhook secrets are hashed; adapter configs are encrypted.

**Finding DB-012 (MEDIUM): KYC Data Not Encrypted at Rest**

**Location:** `migrations/001_initial_schema.sql` lines 297-321

```sql
CREATE TABLE kyc_records (
    verification_data JSONB NOT NULL DEFAULT '{}',  -- Contains PII!
    documents JSONB DEFAULT '[]',  -- Document references
    ...
);
```

**Vulnerability:** `verification_data` likely contains:
- Full legal names
- Date of birth
- National ID numbers
- Address information

This is stored as plain JSONB without column-level encryption.

**Recommendation:**
1. Enable PostgreSQL `pgcrypto` for column-level encryption
2. Encrypt PII fields before storage
3. Use envelope encryption with key rotation support

---

**Finding DB-013 (MEDIUM): Audit Log Details May Contain Sensitive Data**

**Location:** `migrations/001_initial_schema.sql` lines 365-396

```sql
CREATE TABLE audit_log (
    details JSONB NOT NULL DEFAULT '{}',  -- Could contain anything
    ...
);
```

**Vulnerability:** The `details` field could log sensitive data like partial card numbers, account details, or PII.

**Recommendation:** Implement structured audit logging with PII sanitization before storage.

---

## 6. Index Timing Attacks

**Finding DB-014 (LOW): Potential Timing Attack on api_key_hash Lookup**

**Location:** `repository/tenant.rs` lines 58-68

```rust
async fn get_by_api_key_hash(&self, hash: &str) -> Result<Option<TenantRow>> {
    let row = sqlx::query_as::<_, TenantRow>(
        "SELECT * FROM tenants WHERE api_key_hash = $1 AND status = 'ACTIVE'",
    )
    .bind(hash)
    .fetch_optional(&self.pool)
    .await...
}
```

The index on `api_key_hash` could allow timing-based API key enumeration.

**Recommendation:**
1. Add constant-time comparison at application layer
2. Rate limit authentication attempts
3. Consider bloom filter pre-check

---

**Finding DB-015 (LOW): Indexed Columns for User Search**

**Location:** `migrations/001_initial_schema.sql` lines 71-73

```sql
CREATE INDEX idx_users_tenant ON users(tenant_id);
CREATE INDEX idx_users_risk_score ON users(tenant_id, risk_score DESC);
```

The `risk_score` index ordering could leak information about high-risk users through query timing.

**Recommendation:** Consider removing score ordering from public-facing queries; use only in batch reporting.

---

## 7. Constraint Bypass Vulnerabilities

**Finding DB-016 (MEDIUM): State Constraint Too Permissive**

**Location:** `migrations/001_initial_schema.sql` lines 117-121

```sql
CONSTRAINT intents_type_check CHECK (intent_type IN (
    'PAYIN_VND', 'PAYOUT_VND', 'TRADE_EXECUTED',
    'DEPOSIT_ONCHAIN', 'WITHDRAW_ONCHAIN'
))
```

**Issue:** No constraint on `state` column, allowing arbitrary state strings:

```sql
-- This would succeed
UPDATE intents SET state = 'HACKED_STATE' WHERE id = '...';
```

**Recommendation:** Add state validation constraint:
```sql
ALTER TABLE intents ADD CONSTRAINT valid_intent_state
    CHECK (state IN ('INITIATED', 'PROCESSING', 'COMPLETED', 'CANCELLED', 'EXPIRED', 'REJECTED', ...));
```

---

**Finding DB-017 (MEDIUM): Nullable Amount Fields**

**Location:** `migrations/001_initial_schema.sql` line 92

```sql
actual_amount DECIMAL(30, 8),  -- Nullable, no validation
```

**Issue:** Negative amounts or NULL could bypass AML checks.

**Recommendation:**
```sql
ALTER TABLE intents ADD CONSTRAINT positive_amounts
    CHECK (amount > 0 AND (actual_amount IS NULL OR actual_amount >= 0));
```

---

**Finding DB-018 (LOW): Missing Foreign Key on risk_score_history**

**Location:** `migrations/004_score_history.sql`

```sql
CREATE TABLE risk_score_history (
  user_id VARCHAR(255) NOT NULL,  -- No FK to users table
  intent_id VARCHAR(255),         -- No FK to intents table
  ...
);
```

**Issue:** Orphaned records could accumulate; referential integrity not enforced.

**Recommendation:** Add foreign key constraints with appropriate ON DELETE behavior.

---

## 8. Default Permissions Issues

**Finding DB-019 (MEDIUM): No Default Value Restrictions**

Several JSONB columns have empty defaults that could be exploited:

```sql
config JSONB NOT NULL DEFAULT '{}',       -- tenants
metadata JSONB NOT NULL DEFAULT '{}',     -- intents
risk_flags JSONB DEFAULT '[]',            -- users
```

**Issue:** Applications must validate JSONB structure; malformed JSON could cause parsing errors or injection.

**Recommendation:** Add JSONB schema validation via CHECK constraints or triggers.

---

**Finding DB-020 (LOW): Overly Permissive Status Defaults**

**Location:** `migrations/001_initial_schema.sql`

```sql
status VARCHAR(32) NOT NULL DEFAULT 'ACTIVE',  -- tenants
status VARCHAR(32) NOT NULL DEFAULT 'ACTIVE',  -- users
```

**Issue:** New records are ACTIVE by default, requiring explicit deactivation rather than explicit activation.

**Recommendation:** Consider `PENDING` as default for security-sensitive entities.

---

## 9. Seed Data Security Issues

**Finding DB-021 (HIGH): Weak Secrets in Seed Data**

**Location:** `migrations/002_seed_data.sql` lines 16-23

```sql
INSERT INTO tenants (..., api_key_hash, webhook_secret_hash, ...)
VALUES
    (..., crypt('api_key_a', gen_salt('bf')), crypt('webhook_secret_a', gen_salt('bf')), ...),
```

**Issue:**
1. Predictable secrets like `api_key_a` could persist to production
2. Using `crypt()` with `bf` (bcrypt) is good, but cost factor not specified

**Recommendation:**
1. Use environment-based seed data separation
2. Add explicit bcrypt cost factor: `gen_salt('bf', 12)`
3. Generate random secrets in seed scripts

---

**Finding DB-022 (LOW): Mock Encrypted Config Values**

**Location:** `migrations/002_seed_data.sql` lines 55-67

```sql
config_encrypted BYTEA NOT NULL,
...
VALUES (..., '\\xDEADBEEF', ...),  -- Mock encrypted config
```

**Issue:** Obviously invalid encryption that could bypass production checks if seed data leaks.

**Recommendation:** Use properly encrypted mock configs even in test data.

---

## 10. Summary of Recommendations

### Immediate Actions (CRITICAL)

1. **Add RLS to missing tables:** `aml_rule_versions`, `risk_score_history`, `case_notes`, `compliance_transactions`
2. **Add tenant_id column** to `risk_score_history` and `case_notes`
3. **Fix RLS policy** to fail closed when session variable is unset
4. **Add tenant_id filters** to `get_case()`, `get_version()`, `get_notes()`

### Short-term Actions (HIGH)

5. Create separate database roles for app, readonly, and system operations
6. Encrypt KYC `verification_data` at rest
7. Implement tenant isolation for system worker queries
8. Add CHECK constraints for transaction limits

### Medium-term Actions (MEDIUM)

9. Sanitize search patterns to prevent LIKE injection DoS
10. Add state validation constraints
11. Implement JSONB schema validation
12. Add audit log PII sanitization

### Long-term Actions (LOW)

13. Review index timing attack surfaces
14. Add foreign key constraints to history tables
15. Implement constant-time API key comparison
16. Review default permission patterns

---

## Appendix: Files Audited

### Migrations
- `migrations/001_initial_schema.sql` (480 lines)
- `migrations/002_seed_data.sql` (250 lines)
- `migrations/003_rule_versions.sql` (21 lines)
- `migrations/004_score_history.sql` (12 lines)
- `migrations/005_case_notes.sql` (31 lines)
- `migrations/006_enable_rls.sql` (60 lines)
- `migrations/007_compliance_transactions.sql` (16 lines)
- `migrations/999_seed_data.sql` (172 lines)

### Repository Modules
- `crates/ramp-core/src/repository/mod.rs`
- `crates/ramp-core/src/repository/audit.rs`
- `crates/ramp-core/src/repository/intent.rs`
- `crates/ramp-core/src/repository/tenant.rs`
- `crates/ramp-core/src/repository/ledger.rs`
- `crates/ramp-core/src/repository/webhook.rs`
- `crates/ramp-core/src/repository/user.rs`

### Compliance Modules
- `crates/ramp-compliance/src/store/postgres.rs`
- `crates/ramp-compliance/src/transaction_history.rs`
- `crates/ramp-compliance/src/rules/version.rs`

---

*Report generated by Security Audit Agent*
