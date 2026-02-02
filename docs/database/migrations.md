# RampOS Database Migrations

**Database**: PostgreSQL 15+
**Last Updated**: 2026-02-02

---

## Table of Contents

1. [Overview](#overview)
2. [Migration History](#migration-history)
3. [How to Run Migrations](#how-to-run-migrations)
4. [Rollback Procedures](#rollback-procedures)
5. [Creating New Migrations](#creating-new-migrations)
6. [Best Practices](#best-practices)

---

## Overview

RampOS uses a numbered migration system where each SQL file represents a database version. Migrations are applied sequentially and should be idempotent where possible.

### Migration Directory Structure

```
migrations/
  001_initial_schema.sql      # Core tables and functions
  002_seed_data.sql           # Development seed data
  003_rule_versions.sql       # AML rule versioning
  004_score_history.sql       # Risk score tracking
  005_case_notes.sql          # AML case notes
  006_enable_rls.sql          # Row Level Security
  007_compliance_transactions.sql  # Compliance tracking
  008_add_missing_rls.sql     # Additional RLS policies
  009_add_webhook_secret.sql  # Webhook security fix
  999_seed_data.sql           # Extended test data
```

### Naming Convention

```
{NNN}_{description}.sql

Where:
- NNN: Three-digit sequence number (001-998)
- description: Short snake_case description
- 999: Reserved for seed data
```

---

## Migration History

### 001_initial_schema.sql

**Purpose**: Creates the core database schema.

**Changes**:
- Enables extensions: `uuid-ossp`, `pgcrypto`
- Creates core tables:
  - `tenants` - Multi-tenant root entity
  - `users` - End users per tenant
  - `intents` - Transaction intents
  - `ledger_entries` - Double-entry accounting
  - `account_balances` - Balance materialized view
  - `webhook_events` - Webhook outbox
  - `rails_adapters` - Payment provider configs
  - `virtual_accounts` - Virtual bank accounts
  - `kyc_records` - KYC verification records
  - `aml_cases` - AML case management
  - `audit_log` - Immutable audit trail
  - `recon_batches` - Reconciliation batches
- Creates functions:
  - `update_updated_at()` - Auto-update timestamps
  - `append_state_history()` - Track intent state changes
- Creates triggers for automatic timestamp updates

**Dependencies**: None (initial migration)

---

### 002_seed_data.sql

**Purpose**: Populates development/test data.

**Changes**:
- Creates 3 sample tenants (Exchange A, Wallet B, Startup C)
- Creates rails adapters (VCB_DIRECT, FIREBLOCKS, VN_PAY)
- Creates sample users with various KYC tiers
- Creates virtual accounts
- Creates sample intents (payin, payout scenarios)
- Creates corresponding ledger entries
- Creates sample AML cases

**Dependencies**: 001_initial_schema.sql

**Note**: Uses `ON CONFLICT` for idempotent reruns.

---

### 003_rule_versions.sql

**Purpose**: Adds AML rule versioning capability.

**Changes**:
- Creates `aml_rule_versions` table
- Supports multiple rule versions per tenant
- Only one version can be active at a time

**New Table**:
```sql
CREATE TABLE aml_rule_versions (
    id UUID PRIMARY KEY,
    tenant_id VARCHAR(64) NOT NULL REFERENCES tenants(id),
    version_number INT NOT NULL,
    rules_json JSONB NOT NULL,
    is_active BOOLEAN DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by VARCHAR(255),
    activated_at TIMESTAMPTZ,
    UNIQUE (tenant_id, version_number)
);
```

**Dependencies**: 001_initial_schema.sql

---

### 004_score_history.sql

**Purpose**: Tracks risk score changes over time.

**Changes**:
- Creates `risk_score_history` table
- Enables historical risk analysis
- Links scores to triggering intents

**New Table**:
```sql
CREATE TABLE risk_score_history (
    id UUID PRIMARY KEY,
    user_id VARCHAR(255) NOT NULL,
    intent_id VARCHAR(255),
    score DECIMAL(5,2) NOT NULL,
    triggered_rules JSONB,
    action_taken VARCHAR(50),
    created_at TIMESTAMPTZ DEFAULT NOW()
);
```

**Dependencies**: 001_initial_schema.sql

---

### 005_case_notes.sql

**Purpose**: Adds notes capability to AML cases.

**Changes**:
- Creates `case_notes` table
- Supports internal and external notes
- Links to AML cases

**New Table**:
```sql
CREATE TABLE case_notes (
    id UUID PRIMARY KEY,
    case_id VARCHAR(255) NOT NULL,
    author_id VARCHAR(255),
    content TEXT NOT NULL,
    note_type VARCHAR(50) NOT NULL,
    is_internal BOOLEAN DEFAULT true,
    created_at TIMESTAMPTZ DEFAULT NOW()
);
```

**Dependencies**: 001_initial_schema.sql

---

### 006_enable_rls.sql

**Purpose**: Enables Row Level Security for multi-tenant isolation.

**Changes**:
- Enables RLS on 11 tables
- Creates tenant isolation policies using `app.current_tenant` session variable

**Tables with RLS**:
1. `users`
2. `intents`
3. `ledger_entries`
4. `account_balances`
5. `webhook_events`
6. `rails_adapters`
7. `virtual_accounts`
8. `kyc_records`
9. `aml_cases`
10. `audit_log`
11. `recon_batches`

**Policy Pattern**:
```sql
CREATE POLICY tenant_isolation_{table} ON {table}
    USING (tenant_id = current_setting('app.current_tenant')::VARCHAR);
```

**Dependencies**: 001_initial_schema.sql

---

### 007_compliance_transactions.sql

**Purpose**: Adds compliance transaction tracking for velocity checks.

**Changes**:
- Creates `compliance_transactions` table
- Optimized for time-series queries
- Supports transaction type filtering

**New Table**:
```sql
CREATE TABLE compliance_transactions (
    id UUID PRIMARY KEY,
    tenant_id VARCHAR(64) NOT NULL,
    user_id VARCHAR(64) NOT NULL,
    intent_id VARCHAR(64) NOT NULL,
    transaction_type VARCHAR(32) NOT NULL,
    amount_vnd DECIMAL(30, 8) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

**Dependencies**: 001_initial_schema.sql

---

### 008_add_missing_rls.sql

**Purpose**: Security fix - adds RLS to tables that were missing it.

**Changes**:
- Adds `tenant_id` column to `risk_score_history` (if missing)
- Adds `tenant_id` column to `case_notes` (if missing)
- Enables RLS on:
  - `aml_rule_versions`
  - `risk_score_history`
  - `case_notes`
  - `compliance_transactions`
- Creates `rampos_system` role for background workers (BYPASSRLS)
- Adds policy comments for documentation

**Security Roles**:
```sql
CREATE ROLE rampos_system WITH BYPASSRLS NOLOGIN;
GRANT rampos_system TO rampos;
```

**Dependencies**: 003, 004, 005, 006, 007

---

### 009_add_webhook_secret.sql

**Purpose**: Security fix for proper webhook HMAC signing.

**Changes**:
- Adds `webhook_secret_encrypted` column to `tenants`
- Allows storing encrypted webhook secret (not just hash)
- Adds documentation comments

**Column Added**:
```sql
ALTER TABLE tenants ADD COLUMN IF NOT EXISTS webhook_secret_encrypted BYTEA;
```

**Dependencies**: 001_initial_schema.sql

---

### 999_seed_data.sql

**Purpose**: Extended seed data for comprehensive testing.

**Changes**:
- Adds more users per tenant (5+ each)
- Creates trade intent examples
- Creates expired/cancelled intent examples
- Creates webhook event examples

**Dependencies**: 001_initial_schema.sql, 002_seed_data.sql

---

## How to Run Migrations

### Using psql (Manual)

```bash
# Connect to database
psql -h localhost -U rampos -d rampos

# Run a specific migration
\i migrations/001_initial_schema.sql

# Run all migrations in order
for f in migrations/*.sql; do psql -h localhost -U rampos -d rampos -f "$f"; done
```

### Using Docker Compose

```bash
# Start database
docker-compose up -d postgres

# Run migrations
docker-compose exec postgres psql -U rampos -d rampos -f /migrations/001_initial_schema.sql
```

### Using Application Code (Rust)

```rust
// Example using sqlx
use sqlx::PgPool;

async fn run_migrations(pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::migrate!("./migrations")
        .run(pool)
        .await?;
    Ok(())
}
```

### Verify Migration Status

```sql
-- Check if tables exist
SELECT table_name FROM information_schema.tables
WHERE table_schema = 'public'
ORDER BY table_name;

-- Check RLS status
SELECT tablename, rowsecurity
FROM pg_tables
WHERE schemaname = 'public';

-- Check policies
SELECT schemaname, tablename, policyname
FROM pg_policies
WHERE schemaname = 'public';
```

---

## Rollback Procedures

### Important Notes

1. **Migrations are forward-only** - There are no automatic rollback scripts
2. **Always backup before rollback** - Use `pg_dump` before any destructive operation
3. **Test rollbacks in staging first** - Never test rollbacks in production

### Creating a Backup

```bash
# Full database backup
pg_dump -h localhost -U rampos -d rampos -F c -f backup_$(date +%Y%m%d_%H%M%S).dump

# Schema only
pg_dump -h localhost -U rampos -d rampos --schema-only -f schema_backup.sql

# Data only
pg_dump -h localhost -U rampos -d rampos --data-only -f data_backup.sql
```

### Manual Rollback Examples

#### Rollback 009_add_webhook_secret.sql

```sql
-- Remove webhook_secret_encrypted column
ALTER TABLE tenants DROP COLUMN IF EXISTS webhook_secret_encrypted;
```

#### Rollback 008_add_missing_rls.sql

```sql
-- Drop policies
DROP POLICY IF EXISTS tenant_isolation_aml_rule_versions ON aml_rule_versions;
DROP POLICY IF EXISTS tenant_isolation_risk_score_history ON risk_score_history;
DROP POLICY IF EXISTS tenant_isolation_case_notes ON case_notes;
DROP POLICY IF EXISTS tenant_isolation_compliance_transactions ON compliance_transactions;

-- Disable RLS
ALTER TABLE aml_rule_versions DISABLE ROW LEVEL SECURITY;
ALTER TABLE risk_score_history DISABLE ROW LEVEL SECURITY;
ALTER TABLE case_notes DISABLE ROW LEVEL SECURITY;
ALTER TABLE compliance_transactions DISABLE ROW LEVEL SECURITY;

-- Remove tenant_id columns (optional, may require data migration)
-- ALTER TABLE risk_score_history DROP COLUMN tenant_id;
-- ALTER TABLE case_notes DROP COLUMN tenant_id;

-- Remove system role
REVOKE rampos_system FROM rampos;
DROP ROLE IF EXISTS rampos_system;
```

#### Rollback 007_compliance_transactions.sql

```sql
DROP TABLE IF EXISTS compliance_transactions;
```

#### Rollback 006_enable_rls.sql

```sql
-- Disable RLS on all tables
ALTER TABLE users DISABLE ROW LEVEL SECURITY;
ALTER TABLE intents DISABLE ROW LEVEL SECURITY;
ALTER TABLE ledger_entries DISABLE ROW LEVEL SECURITY;
ALTER TABLE account_balances DISABLE ROW LEVEL SECURITY;
ALTER TABLE webhook_events DISABLE ROW LEVEL SECURITY;
ALTER TABLE rails_adapters DISABLE ROW LEVEL SECURITY;
ALTER TABLE virtual_accounts DISABLE ROW LEVEL SECURITY;
ALTER TABLE kyc_records DISABLE ROW LEVEL SECURITY;
ALTER TABLE aml_cases DISABLE ROW LEVEL SECURITY;
ALTER TABLE audit_log DISABLE ROW LEVEL SECURITY;
ALTER TABLE recon_batches DISABLE ROW LEVEL SECURITY;

-- Drop policies
DROP POLICY IF EXISTS tenant_isolation_users ON users;
DROP POLICY IF EXISTS tenant_isolation_intents ON intents;
DROP POLICY IF EXISTS tenant_isolation_ledger_entries ON ledger_entries;
DROP POLICY IF EXISTS tenant_isolation_account_balances ON account_balances;
DROP POLICY IF EXISTS tenant_isolation_webhook_events ON webhook_events;
DROP POLICY IF EXISTS tenant_isolation_rails_adapters ON rails_adapters;
DROP POLICY IF EXISTS tenant_isolation_virtual_accounts ON virtual_accounts;
DROP POLICY IF EXISTS tenant_isolation_kyc_records ON kyc_records;
DROP POLICY IF EXISTS tenant_isolation_aml_cases ON aml_cases;
DROP POLICY IF EXISTS tenant_isolation_audit_log ON audit_log;
DROP POLICY IF EXISTS tenant_isolation_recon_batches ON recon_batches;
```

#### Rollback 005_case_notes.sql

```sql
DROP TABLE IF EXISTS case_notes;
```

#### Rollback 004_score_history.sql

```sql
DROP TABLE IF EXISTS risk_score_history;
```

#### Rollback 003_rule_versions.sql

```sql
DROP TABLE IF EXISTS aml_rule_versions;
```

#### Full Rollback (Nuclear Option)

```sql
-- WARNING: This drops EVERYTHING!
-- Use only for development/testing

DROP TABLE IF EXISTS
    case_notes,
    risk_score_history,
    aml_rule_versions,
    compliance_transactions,
    recon_batches,
    audit_log,
    aml_cases,
    kyc_records,
    virtual_accounts,
    webhook_events,
    account_balances,
    ledger_entries,
    intents,
    rails_adapters,
    users,
    tenants
CASCADE;

DROP FUNCTION IF EXISTS update_updated_at();
DROP FUNCTION IF EXISTS append_state_history();

DROP EXTENSION IF EXISTS "pgcrypto";
DROP EXTENSION IF EXISTS "uuid-ossp";
```

---

## Creating New Migrations

### Step 1: Determine the Next Number

```bash
ls migrations/*.sql | tail -1
# If last is 009, use 010
```

### Step 2: Create the Migration File

```bash
touch migrations/010_your_feature.sql
```

### Step 3: Write the Migration

```sql
-- ============================================================================
-- FEATURE: Your Feature Name
-- Author: Your Name
-- Date: YYYY-MM-DD
-- Description: Brief description of what this migration does
-- ============================================================================

-- Your SQL statements here

-- Example: Add a new column
ALTER TABLE tenants ADD COLUMN IF NOT EXISTS new_feature_enabled BOOLEAN DEFAULT false;

-- Example: Create a new table
CREATE TABLE IF NOT EXISTS new_table (
    id UUID PRIMARY KEY,
    tenant_id VARCHAR(64) NOT NULL REFERENCES tenants(id),
    -- other columns
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Example: Add RLS policy (if table contains tenant data)
ALTER TABLE new_table ENABLE ROW LEVEL SECURITY;
CREATE POLICY tenant_isolation_new_table ON new_table
    USING (tenant_id = current_setting('app.current_tenant')::VARCHAR);

-- Example: Create index for performance
CREATE INDEX IF NOT EXISTS idx_new_table_tenant ON new_table(tenant_id);
```

### Step 4: Test the Migration

```bash
# In a test database
psql -U rampos -d rampos_test -f migrations/010_your_feature.sql

# Verify
psql -U rampos -d rampos_test -c "\d new_table"
```

### Step 5: Document in This File

Add an entry to the Migration History section above.

---

## Best Practices

### DO

1. **Use IF NOT EXISTS / IF EXISTS** - Make migrations idempotent
   ```sql
   CREATE TABLE IF NOT EXISTS ...
   ALTER TABLE ... ADD COLUMN IF NOT EXISTS ...
   DROP INDEX IF EXISTS ...
   ```

2. **Add indexes for foreign keys** - Improve join performance
   ```sql
   CREATE INDEX idx_table_fk ON table(foreign_key_column);
   ```

3. **Add RLS policies for tenant data** - Maintain security
   ```sql
   ALTER TABLE new_table ENABLE ROW LEVEL SECURITY;
   CREATE POLICY tenant_isolation_new_table ON new_table
       USING (tenant_id = current_setting('app.current_tenant')::VARCHAR);
   ```

4. **Use transactions for complex migrations**
   ```sql
   BEGIN;
   -- multiple statements
   COMMIT;
   ```

5. **Add comments for documentation**
   ```sql
   COMMENT ON TABLE new_table IS 'Description of the table';
   COMMENT ON COLUMN new_table.column IS 'Description of the column';
   ```

6. **Test in staging first** - Always validate before production

### DON'T

1. **Don't modify existing migrations** - Create new ones instead

2. **Don't use destructive operations without backup**
   ```sql
   -- BAD: No safety net
   DROP TABLE users;

   -- BETTER: Add IF EXISTS
   DROP TABLE IF EXISTS old_unused_table;
   ```

3. **Don't skip sequence numbers** - Keep migrations orderly

4. **Don't include sensitive data** - Use environment variables
   ```sql
   -- BAD: Hardcoded secrets
   INSERT INTO tenants VALUES ('test', 'secret123');

   -- BETTER: Use placeholder or environment
   INSERT INTO tenants VALUES ('test', crypt(:secret, gen_salt('bf')));
   ```

5. **Don't create circular dependencies** - Plan your schema carefully

---

## See Also

- [Database Schema](./schema.md)
- [Row Level Security](./rls.md)
