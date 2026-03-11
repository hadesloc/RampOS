# RampOS Database Migrations

**Database**: PostgreSQL 16+
**Last Updated**: 2026-03-11
**Total Migrations**: 42 up + 32 down

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
  001_initial_schema.sql          # Core tables and functions
  002_seed_data.sql               # Development seed data
  003_rule_versions.sql           # AML rule versioning
  004_score_history.sql           # Risk score tracking
  005_case_notes.sql              # AML case notes
  006_enable_rls.sql              # Row Level Security
  007_compliance_transactions.sql # Compliance tracking
  008_add_missing_rls.sql         # Additional RLS policies
  009_add_webhook_secret.sql      # Webhook security fix
  010_smart_accounts.sql          # ERC-4337 smart accounts
  011_add_api_secret.sql          # API key management
  012_bank_confirmations.sql      # Bank confirmation tracking
  013_compliance_integrity.sql    # Compliance data integrity
  014_rls_fail_closed.sql         # RLS fail-closed policies
  015_license_management.sql      # Tenant license management
  016_multi_stablecoin.sql        # Multi-stablecoin support
  017_custom_domains.sql          # Custom domain management
  018_enterprise_sso.sql          # Enterprise SSO
  019_usage_billing.sql           # Usage metering & billing
  020_compliance_audit_trail.sql  # Compliance audit trail
  021_vnd_transaction_limits.sql  # VND transaction limits
  022_licensing_requirements.sql  # Licensing requirements
  023_encrypt_secrets_nonce.sql   # Encryption nonce column
  024_webauthn_credentials.sql    # WebAuthn/Passkey support
  025_portal_kyc_cases.sql        # Portal KYC case management
  026_webhook_configs.sql         # Webhook configuration
  027_offramp_intents.sql         # Off-ramp intent support
  028_magic_link_tokens.sql       # Magic link auth tokens
  029_refresh_tokens.sql          # JWT refresh tokens
  030_tenant_rate_limits.sql      # Per-tenant rate limiting
  031_tenant_api_version.sql      # Per-tenant API versioning
  032_settlements.sql             # Settlement engine
  033_rfq_auction.sql             # RFQ auction tables
  034_lp_keys.sql                 # LP API key management
  035_sandbox_presets.sql         # Sandbox preset system (W1)
  036_lp_reliability_snapshots.sql # LP reliability scoring (W4)
  037_travel_rule.sql             # Travel Rule FATF (W5)
  038_risk_lab_replay_metadata.sql # Risk Lab replay (W6)
  039_rescreening_runs.sql        # Continuous rescreening (W12)
  040_kyc_passport.sql            # KYC passport portability (W13)
  041_kyb_graph.sql               # KYB corporate graph (W14)
  999_seed_data.sql               # Extended test data
  down/                           # 32 rollback scripts
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

### 003–009: Foundation Migrations

| Migration | Purpose | Key Tables/Columns |
|-----------|---------|-------------------|
| 003_rule_versions | AML rule versioning | `aml_rule_versions` |
| 004_score_history | Risk score tracking | `risk_score_history` |
| 005_case_notes | AML case notes | `case_notes` |
| 006_enable_rls | Row Level Security on 11 tables | RLS policies |
| 007_compliance_transactions | Velocity check tracking | `compliance_transactions` |
| 008_add_missing_rls | Security fix: RLS gaps + `rampos_system` role | Additional RLS |
| 009_add_webhook_secret | HMAC webhook signing | `webhook_secret_encrypted` column |

---

### 010–019: Platform Infrastructure

| Migration | Purpose | Key Tables/Columns |
|-----------|---------|-------------------|
| 010_smart_accounts | ERC-4337 smart account tracking | `smart_accounts`, `smart_account_ops` |
| 011_add_api_secret | Tenant API key management | `api_secret` column |
| 012_bank_confirmations | Bank callback tracking | `bank_confirmations` |
| 013_compliance_integrity | Data integrity constraints | Additional constraints |
| 014_rls_fail_closed | Fail-closed RLS default | `default_deny` policies |
| 015_license_management | Tenant licensing | `licenses`, `license_features` |
| 016_multi_stablecoin | Multi-stablecoin support | Stablecoin columns |
| 017_custom_domains | White-label domains | `custom_domains`, `domain_ssl_certs` |
| 018_enterprise_sso | SAML/OIDC SSO | `sso_configurations` |
| 019_usage_billing | Metering & billing | `usage_records`, `billing_events` |

---

### 020–029: Compliance & Authentication

| Migration | Purpose | Key Tables/Columns |
|-----------|---------|-------------------|
| 020_compliance_audit_trail | Append-only compliance audit | `compliance_audit_events` |
| 021_vnd_transaction_limits | VND-specific limits | `vnd_transaction_limits` |
| 022_licensing_requirements | License requirement registry | `licensing_requirements` |
| 023_encrypt_secrets_nonce | Encryption nonce management | `nonce` column on secrets |
| 024_webauthn_credentials | Passkey/WebAuthn auth | `webauthn_credentials` |
| 025_portal_kyc_cases | Portal KYC case workflow | `portal_kyc_cases` |
| 026_webhook_configs | Webhook configuration | `webhook_configs`, retry settings |
| 027_offramp_intents | Off-ramp intent tracking | `offramp_intents`, escrow fields |
| 028_magic_link_tokens | Passwordless auth | `magic_link_tokens` |
| 029_refresh_tokens | JWT refresh tokens | `refresh_tokens` |

---

### 030–034: Rate Limiting, Versioning, RFQ

| Migration | Purpose | Key Tables/Columns |
|-----------|---------|-------------------|
| 030_tenant_rate_limits | Per-tenant rate limit overrides | `tenant_rate_limits` |
| 031_tenant_api_version | API version pinning | `api_version` column |
| 032_settlements | Settlement engine | `settlements`, `settlement_items` |
| 033_rfq_auction | RFQ auction marketplace | `rfq_requests`, `rfq_bids` |
| 034_lp_keys | LP API key auth | `lp_keys` (X-LP-Key header) |

---

### 035–041: World-Class Roadmap (W1-W16)

These migrations implement the W1-W16 World-Class Roadmap features:

| Migration | Workstream | Purpose | Key Tables |
|-----------|------------|---------|------------|
| 035_sandbox_presets | W1 | Programmable sandbox environments | `sandbox_presets`, `sandbox_preset_scenarios` (3 presets, 8 scenarios) |
| 036_lp_reliability_snapshots | W4 | LP reliability scoring | `lp_reliability_snapshots` (fill_rate, reject_rate, dispute_rate, slippage, latency, rolling windows) |
| 037_travel_rule | W5 | FATF R.16 Travel Rule | `travel_rule_policies`, `travel_rule_vasps`, `travel_rule_disclosures`, `travel_rule_transport_attempts`, `travel_rule_exception_queue` |
| 038_risk_lab_replay_metadata | W6 | AML replay & explainability | Extends `aml_rule_versions` (version_state, shadow), extends `risk_score_history` (feature_vector, score_explanation, decision_snapshot) |
| 039_rescreening_runs | W12 | Continuous KYC/PEP rescreening | `compliance_rescreening_runs` (SCHEDULED, WATCHLIST_DELTA, DOCUMENT_EXPIRY triggers) |
| 040_kyc_passport | W13 | Cross-tenant KYC portability | `kyc_passport_vault`, `kyc_passport_consent_grants`, `kyc_passport_acceptance_policies` |
| 041_kyb_graph | W14 | Corporate ownership graph | `kyb_entities`, `kyb_ownership_edges` (ownership_pct, jurisdiction) |

All W1-W16 migrations include RLS policies and tenant isolation.

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
