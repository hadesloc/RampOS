# RampOS Row Level Security (RLS) Documentation

**Database**: PostgreSQL 15+
**Last Updated**: 2026-02-02

---

## Table of Contents

1. [Overview](#overview)
2. [How RLS Works](#how-rls-works)
3. [Tenant Isolation Implementation](#tenant-isolation-implementation)
4. [Tables with RLS](#tables-with-rls)
5. [Setting the Tenant Context](#setting-the-tenant-context)
6. [Background Workers & System Access](#background-workers--system-access)
7. [Testing RLS](#testing-rls)
8. [Security Considerations](#security-considerations)
9. [Best Practices](#best-practices)
10. [Troubleshooting](#troubleshooting)

---

## Overview

RampOS implements PostgreSQL Row Level Security (RLS) to enforce multi-tenant data isolation at the database level. This provides a defense-in-depth approach where even if application-level bugs exist, the database prevents cross-tenant data access.

### Key Benefits

- **Defense in Depth**: Database-level enforcement prevents data leaks from application bugs
- **Transparent**: Application queries don't need to include tenant filters
- **Auditable**: Policies are defined in SQL and visible in the database
- **Performant**: PostgreSQL optimizes queries with RLS policies

---

## How RLS Works

### Basic Concept

```sql
-- Without RLS: Application must always filter by tenant
SELECT * FROM users WHERE tenant_id = 'tenant_a' AND status = 'ACTIVE';

-- With RLS: Tenant filter is automatic
SET app.current_tenant = 'tenant_a';
SELECT * FROM users WHERE status = 'ACTIVE';
-- PostgreSQL automatically adds: AND tenant_id = 'tenant_a'
```

### Policy Enforcement

When RLS is enabled on a table:
1. PostgreSQL checks all defined policies before returning rows
2. Only rows matching the policy predicate are visible
3. INSERT/UPDATE/DELETE operations also respect policies

---

## Tenant Isolation Implementation

### Policy Pattern

All RampOS tenant isolation policies follow this pattern:

```sql
CREATE POLICY tenant_isolation_{table} ON {table}
    USING (tenant_id = current_setting('app.current_tenant')::VARCHAR);
```

### How It Works

1. **Application sets tenant context** via session variable:
   ```sql
   SET app.current_tenant = 'tenant_abc';
   ```

2. **Policy checks every row** against the condition:
   ```sql
   tenant_id = current_setting('app.current_tenant')::VARCHAR
   ```

3. **Only matching rows are returned** - other tenant data is invisible

### Complete Example

```sql
-- Enable RLS on the table
ALTER TABLE users ENABLE ROW LEVEL SECURITY;

-- Create the isolation policy
CREATE POLICY tenant_isolation_users ON users
    USING (tenant_id = current_setting('app.current_tenant')::VARCHAR);

-- Now queries are automatically filtered:
SET app.current_tenant = 'tenant_a';
SELECT * FROM users;  -- Only sees tenant_a users
```

---

## Tables with RLS

### Core Business Tables

| Table | RLS Enabled | Policy Name | Migration |
|-------|-------------|-------------|-----------|
| `users` | Yes | `tenant_isolation_users` | 006 |
| `intents` | Yes | `tenant_isolation_intents` | 006 |
| `ledger_entries` | Yes | `tenant_isolation_ledger_entries` | 006 |
| `account_balances` | Yes | `tenant_isolation_account_balances` | 006 |
| `webhook_events` | Yes | `tenant_isolation_webhook_events` | 006 |
| `rails_adapters` | Yes | `tenant_isolation_rails_adapters` | 006 |
| `virtual_accounts` | Yes | `tenant_isolation_virtual_accounts` | 006 |

### Compliance Tables

| Table | RLS Enabled | Policy Name | Migration |
|-------|-------------|-------------|-----------|
| `kyc_records` | Yes | `tenant_isolation_kyc_records` | 006 |
| `aml_cases` | Yes | `tenant_isolation_aml_cases` | 006 |
| `aml_rule_versions` | Yes | `tenant_isolation_aml_rule_versions` | 008 |
| `risk_score_history` | Yes | `tenant_isolation_risk_score_history` | 008 |
| `case_notes` | Yes | `tenant_isolation_case_notes` | 008 |
| `compliance_transactions` | Yes | `tenant_isolation_compliance_transactions` | 008 |

### Audit Tables

| Table | RLS Enabled | Policy Name | Migration |
|-------|-------------|-------------|-----------|
| `audit_log` | Yes | `tenant_isolation_audit_log` | 006 |
| `recon_batches` | Yes | `tenant_isolation_recon_batches` | 006 |

### Tables WITHOUT RLS

| Table | Reason |
|-------|--------|
| `tenants` | Root table - no tenant_id column |

---

## Setting the Tenant Context

### In Application Code (Rust)

```rust
use sqlx::PgPool;

pub async fn set_tenant_context(pool: &PgPool, tenant_id: &str) -> Result<(), sqlx::Error> {
    sqlx::query(&format!("SET app.current_tenant = '{}'", tenant_id))
        .execute(pool)
        .await?;
    Ok(())
}

// Usage in API handler
async fn get_users(pool: &PgPool, tenant_id: &str) -> Result<Vec<User>, Error> {
    // Set tenant context first
    set_tenant_context(pool, tenant_id).await?;

    // Query without explicit tenant filter - RLS handles it
    let users = sqlx::query_as::<_, User>("SELECT * FROM users WHERE status = 'ACTIVE'")
        .fetch_all(pool)
        .await?;

    Ok(users)
}
```

### Per-Transaction Pattern

```rust
use sqlx::PgPool;

pub async fn with_tenant<T, F, Fut>(pool: &PgPool, tenant_id: &str, f: F) -> Result<T, Error>
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = Result<T, Error>>,
{
    // Begin transaction
    let mut tx = pool.begin().await?;

    // Set tenant context for this transaction
    sqlx::query(&format!("SET LOCAL app.current_tenant = '{}'", tenant_id))
        .execute(&mut *tx)
        .await?;

    // Execute the operation
    let result = f().await?;

    // Commit
    tx.commit().await?;

    Ok(result)
}
```

### Using SET LOCAL vs SET

| Command | Scope | Use Case |
|---------|-------|----------|
| `SET app.current_tenant = 'x'` | Session | Long-lived connections |
| `SET LOCAL app.current_tenant = 'x'` | Transaction | Per-request isolation |

**Recommendation**: Use `SET LOCAL` in transactions for better isolation.

### Direct SQL Example

```sql
-- Start transaction
BEGIN;

-- Set tenant context (transaction-scoped)
SET LOCAL app.current_tenant = 'tenant_abc';

-- All queries in this transaction are filtered to tenant_abc
SELECT * FROM users;
SELECT * FROM intents WHERE state = 'PENDING';

-- Commit
COMMIT;

-- After commit, context is cleared
```

---

## Background Workers & System Access

### The Problem

Background workers (like expired intent cleanup) need cross-tenant access:

```rust
// This worker needs to see ALL expired intents
async fn cleanup_expired_intents() {
    // If RLS is active, this only sees one tenant!
    let expired = query("SELECT * FROM intents WHERE expires_at < NOW()");
}
```

### Solution: System Role

Migration `008` creates a system role with RLS bypass:

```sql
CREATE ROLE rampos_system WITH BYPASSRLS NOLOGIN;
GRANT rampos_system TO rampos;
```

### Using the System Role

```rust
// In background worker
async fn cleanup_expired_intents(pool: &PgPool) -> Result<(), Error> {
    // Switch to system role (bypasses RLS)
    sqlx::query("SET ROLE rampos_system")
        .execute(pool)
        .await?;

    // Now sees ALL tenants
    let expired = sqlx::query_as::<_, Intent>(
        "SELECT * FROM intents WHERE expires_at < NOW() AND state = 'PENDING'"
    )
    .fetch_all(pool)
    .await?;

    for intent in expired {
        // Process each expired intent
        mark_expired(pool, &intent.id).await?;
    }

    // Reset to normal role
    sqlx::query("RESET ROLE")
        .execute(pool)
        .await?;

    Ok(())
}
```

### Best Practices for System Access

1. **Separate connection pool** for background workers
2. **Audit all system role usage** in logs
3. **Minimize scope** - only use system role when absolutely necessary
4. **Reset role immediately** after cross-tenant operation

---

## Testing RLS

### Verify RLS is Enabled

```sql
-- Check RLS status on all tables
SELECT tablename, rowsecurity
FROM pg_tables
WHERE schemaname = 'public';

-- Expected output:
--    tablename    | rowsecurity
-- ----------------+-------------
--  users          | t
--  intents        | t
--  ...
```

### Verify Policies Exist

```sql
-- List all RLS policies
SELECT schemaname, tablename, policyname, cmd, qual
FROM pg_policies
WHERE schemaname = 'public';
```

### Test Isolation

```sql
-- Test: Tenant A should not see Tenant B data

-- First, as Tenant A
BEGIN;
SET LOCAL app.current_tenant = 'tenant_a_123';

SELECT COUNT(*) FROM users;          -- Should show Tenant A count
SELECT COUNT(*) FROM intents;        -- Should show Tenant A count

COMMIT;

-- Then, as Tenant B
BEGIN;
SET LOCAL app.current_tenant = 'tenant_b_456';

SELECT COUNT(*) FROM users;          -- Should show Tenant B count (different!)
SELECT COUNT(*) FROM intents;        -- Should show Tenant B count

COMMIT;
```

### Test Cross-Tenant Protection

```sql
-- This should return 0 rows (not Tenant A's data)
BEGIN;
SET LOCAL app.current_tenant = 'tenant_b_456';

SELECT * FROM users WHERE id = 'user_a_1';  -- Should return empty!

COMMIT;
```

### Integration Test Example (Rust)

```rust
#[tokio::test]
async fn test_rls_isolation() {
    let pool = create_test_pool().await;

    // Create users in different tenants
    create_user(&pool, "tenant_a", "user_1").await;
    create_user(&pool, "tenant_b", "user_2").await;

    // Query as Tenant A
    set_tenant_context(&pool, "tenant_a").await.unwrap();
    let users_a = get_all_users(&pool).await.unwrap();

    assert_eq!(users_a.len(), 1);
    assert_eq!(users_a[0].id, "user_1");

    // Query as Tenant B
    set_tenant_context(&pool, "tenant_b").await.unwrap();
    let users_b = get_all_users(&pool).await.unwrap();

    assert_eq!(users_b.len(), 1);
    assert_eq!(users_b[0].id, "user_2");

    // Verify Tenant B cannot see Tenant A's user
    let cross_tenant = get_user_by_id(&pool, "user_1").await;
    assert!(cross_tenant.is_none());
}
```

---

## Security Considerations

### 1. Always Set Tenant Context

**Risk**: If `app.current_tenant` is not set, queries may fail or return unexpected results.

**Mitigation**:
```sql
-- Set a default that blocks all access
ALTER DATABASE rampos SET app.current_tenant = '';

-- Or use COALESCE in policies (less secure)
CREATE POLICY tenant_isolation_users ON users
    USING (tenant_id = COALESCE(current_setting('app.current_tenant', true), '')::VARCHAR);
```

### 2. SQL Injection in Tenant ID

**Risk**: If tenant_id is user-controlled and not validated, SQL injection is possible.

**Mitigation**:
```rust
// ALWAYS validate tenant_id format
fn validate_tenant_id(id: &str) -> Result<(), Error> {
    if !id.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return Err(Error::InvalidTenantId);
    }
    if id.len() > 64 {
        return Err(Error::TenantIdTooLong);
    }
    Ok(())
}
```

### 3. Superuser Bypass

**Risk**: PostgreSQL superusers and table owners bypass RLS by default.

**Mitigation**:
```sql
-- Force RLS for table owners too
ALTER TABLE users FORCE ROW LEVEL SECURITY;
ALTER TABLE intents FORCE ROW LEVEL SECURITY;
-- ... for all tables
```

### 4. Connection Reuse

**Risk**: Connection pooling may reuse connections with stale tenant context.

**Mitigation**:
```rust
// Always set tenant context at start of each request
async fn handle_request(req: Request) -> Response {
    let tenant_id = extract_tenant_from_token(&req)?;

    // ALWAYS set context, even if you think it's already set
    set_tenant_context(&pool, &tenant_id).await?;

    // ... handle request
}
```

### 5. Backup and Restore

**Risk**: Restoring backups to wrong database could expose data.

**Mitigation**:
- Use separate databases per environment (dev/staging/prod)
- Encrypt backups
- Validate tenant_id after restore

---

## Best Practices

### 1. Defense in Depth

Even with RLS, add application-level tenant checks:

```rust
async fn get_intent(pool: &PgPool, tenant_id: &str, intent_id: &str) -> Result<Intent, Error> {
    set_tenant_context(pool, tenant_id).await?;

    let intent = sqlx::query_as::<_, Intent>(
        "SELECT * FROM intents WHERE id = $1"
    )
    .bind(intent_id)
    .fetch_optional(pool)
    .await?;

    // Double-check tenant (defense in depth)
    match intent {
        Some(i) if i.tenant_id == tenant_id => Ok(i),
        Some(_) => Err(Error::Forbidden),  // RLS should prevent this, but check anyway
        None => Err(Error::NotFound),
    }
}
```

### 2. Audit Cross-Tenant Operations

```rust
async fn system_operation(pool: &PgPool, operation: &str) {
    // Log before switching to system role
    log::warn!("Entering system mode for: {}", operation);

    sqlx::query("SET ROLE rampos_system")
        .execute(pool)
        .await?;

    // Do operation...

    // Log when leaving system mode
    log::info!("Exiting system mode for: {}", operation);

    sqlx::query("RESET ROLE")
        .execute(pool)
        .await?;
}
```

### 3. Test RLS in CI/CD

```yaml
# In your CI pipeline
- name: Run RLS Tests
  run: |
    cargo test --test rls_isolation_tests
```

### 4. Monitor Policy Violations

```sql
-- Create a function to log RLS denials (for debugging)
CREATE OR REPLACE FUNCTION log_rls_denial()
RETURNS trigger AS $$
BEGIN
    INSERT INTO audit_log (
        tenant_id,
        actor_type,
        action,
        resource_type,
        details
    ) VALUES (
        current_setting('app.current_tenant', true),
        'SYSTEM',
        'RLS_DENIAL',
        TG_TABLE_NAME,
        jsonb_build_object('attempted_row_id', NEW.id)
    );
    RETURN NULL;
END;
$$ LANGUAGE plpgsql;
```

### 5. Document All Policies

Add comments to policies for clarity:

```sql
COMMENT ON POLICY tenant_isolation_users ON users
    IS 'Restricts user visibility to the current tenant set via app.current_tenant';
```

---

## Troubleshooting

### Problem: "permission denied" Error

**Symptom**: Queries fail with permission denied.

**Cause**: RLS is enabled but no policy matches.

**Solution**:
```sql
-- Check if tenant context is set
SELECT current_setting('app.current_tenant', true);

-- If NULL or empty, set it:
SET app.current_tenant = 'your_tenant_id';
```

### Problem: Empty Results When Data Exists

**Symptom**: Query returns 0 rows but data exists in table.

**Cause**: Tenant context doesn't match any rows.

**Solution**:
```sql
-- Verify the tenant_id in your data
SELECT DISTINCT tenant_id FROM users;

-- Compare with current setting
SELECT current_setting('app.current_tenant');

-- They must match exactly (case-sensitive!)
```

### Problem: Cross-Tenant Data Visible

**Symptom**: User can see other tenant's data.

**Cause**: RLS not enabled, policy missing, or using superuser.

**Solution**:
```sql
-- Check if RLS is enabled
SELECT rowsecurity FROM pg_tables WHERE tablename = 'users';

-- If false, enable it:
ALTER TABLE users ENABLE ROW LEVEL SECURITY;

-- Check if policy exists
SELECT * FROM pg_policies WHERE tablename = 'users';

-- Check if you're a superuser (bypasses RLS)
SELECT current_user, usesuper FROM pg_user WHERE usename = current_user;
```

### Problem: Background Worker Sees No Data

**Symptom**: Background job can't find any records.

**Cause**: RLS is filtering because tenant context is set.

**Solution**:
```sql
-- Use system role for cross-tenant operations
SET ROLE rampos_system;

-- Your query here (sees all tenants)
SELECT * FROM intents WHERE state = 'PENDING';

-- Reset when done
RESET ROLE;
```

### Problem: Performance Issues

**Symptom**: Queries are slow after enabling RLS.

**Cause**: Missing indexes on tenant_id.

**Solution**:
```sql
-- Add index on tenant_id
CREATE INDEX IF NOT EXISTS idx_table_tenant ON your_table(tenant_id);

-- For composite queries, add compound indexes
CREATE INDEX IF NOT EXISTS idx_table_tenant_status
    ON your_table(tenant_id, status);
```

---

## See Also

- [Database Schema](./schema.md)
- [Migration History](./migrations.md)
- [PostgreSQL RLS Documentation](https://www.postgresql.org/docs/current/ddl-rowsecurity.html)
