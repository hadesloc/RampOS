# RampOS Database Schema Documentation

**Database**: PostgreSQL 15+
**Last Updated**: 2026-02-02
**Version**: 1.0

---

## Table of Contents

1. [Overview](#overview)
2. [Extensions](#extensions)
3. [Tables](#tables)
   - [tenants](#tenants)
   - [users](#users)
   - [intents](#intents)
   - [ledger_entries](#ledger_entries)
   - [account_balances](#account_balances)
   - [webhook_events](#webhook_events)
   - [rails_adapters](#rails_adapters)
   - [virtual_accounts](#virtual_accounts)
   - [kyc_records](#kyc_records)
   - [aml_cases](#aml_cases)
   - [aml_rule_versions](#aml_rule_versions)
   - [risk_score_history](#risk_score_history)
   - [case_notes](#case_notes)
   - [compliance_transactions](#compliance_transactions)
   - [audit_log](#audit_log)
   - [recon_batches](#recon_batches)
4. [Functions & Triggers](#functions--triggers)
5. [Entity Relationship Diagram](#entity-relationship-diagram)

---

## Overview

RampOS uses a multi-tenant PostgreSQL database with Row Level Security (RLS) for tenant isolation. The schema is designed around the concept of "Intents" - representing payment transactions that flow through various states.

### Key Design Principles

- **Multi-tenancy**: All data is isolated per tenant using RLS policies
- **Double-entry Accounting**: All financial movements are tracked via ledger entries
- **Audit Trail**: Comprehensive logging with hash chain integrity
- **Compliance First**: Built-in KYC/AML tracking capabilities

---

## Extensions

```sql
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";  -- UUID generation
CREATE EXTENSION IF NOT EXISTS "pgcrypto";   -- Cryptographic functions
```

---

## Tables

### tenants

**Description**: Exchanges and platforms using RampOS. This is the root entity for multi-tenancy.

| Column | Type | Nullable | Default | Description |
|--------|------|----------|---------|-------------|
| `id` | VARCHAR(64) | NO | - | Primary key, tenant identifier |
| `name` | VARCHAR(255) | NO | - | Display name of the tenant |
| `status` | VARCHAR(32) | NO | 'ACTIVE' | Tenant status |
| `api_key_hash` | VARCHAR(255) | NO | - | Hashed API key for authentication |
| `webhook_secret_hash` | VARCHAR(255) | NO | - | Hashed webhook secret (for verification) |
| `webhook_secret_encrypted` | BYTEA | YES | NULL | Encrypted webhook secret (for HMAC signing) |
| `webhook_url` | VARCHAR(512) | YES | NULL | URL for webhook delivery |
| `config` | JSONB | NO | '{}' | Tenant-specific configuration |
| `daily_payin_limit_vnd` | DECIMAL(20,2) | YES | 10000000000 | Daily pay-in limit in VND (10B default) |
| `daily_payout_limit_vnd` | DECIMAL(20,2) | YES | 5000000000 | Daily payout limit in VND (5B default) |
| `created_at` | TIMESTAMPTZ | NO | NOW() | Record creation timestamp |
| `updated_at` | TIMESTAMPTZ | NO | NOW() | Last update timestamp |

**Constraints**:
- PRIMARY KEY: `id`
- CHECK: `status IN ('ACTIVE', 'SUSPENDED', 'PENDING')`

**Indexes**:
```sql
CREATE INDEX idx_tenants_status ON tenants(status);
```

**Status Values**:
| Status | Description |
|--------|-------------|
| `ACTIVE` | Fully operational tenant |
| `SUSPENDED` | Temporarily disabled |
| `PENDING` | Awaiting activation |

---

### users

**Description**: End users on tenant platforms. Users are scoped to a tenant.

| Column | Type | Nullable | Default | Description |
|--------|------|----------|---------|-------------|
| `id` | VARCHAR(64) | NO | - | User identifier (unique per tenant) |
| `tenant_id` | VARCHAR(64) | NO | - | Reference to owning tenant |
| `kyc_tier` | SMALLINT | NO | 0 | KYC verification level (0-3) |
| `kyc_status` | VARCHAR(32) | NO | 'PENDING' | Current KYC status |
| `kyc_verified_at` | TIMESTAMPTZ | YES | NULL | When KYC was verified |
| `risk_score` | DECIMAL(5,2) | YES | 0 | Calculated risk score (0-100) |
| `risk_flags` | JSONB | YES | '[]' | Array of risk flags |
| `daily_payin_limit_vnd` | DECIMAL(20,2) | YES | NULL | User-specific pay-in limit override |
| `daily_payout_limit_vnd` | DECIMAL(20,2) | YES | NULL | User-specific payout limit override |
| `status` | VARCHAR(32) | NO | 'ACTIVE' | User account status |
| `created_at` | TIMESTAMPTZ | NO | NOW() | Record creation timestamp |
| `updated_at` | TIMESTAMPTZ | NO | NOW() | Last update timestamp |

**Constraints**:
- PRIMARY KEY: `(tenant_id, id)` (composite key)
- FOREIGN KEY: `tenant_id` REFERENCES `tenants(id)`
- CHECK: `kyc_tier BETWEEN 0 AND 3`
- CHECK: `status IN ('ACTIVE', 'SUSPENDED', 'BLOCKED')`

**Indexes**:
```sql
CREATE INDEX idx_users_tenant ON users(tenant_id);
CREATE INDEX idx_users_kyc_status ON users(tenant_id, kyc_status);
CREATE INDEX idx_users_risk_score ON users(tenant_id, risk_score DESC);
```

**KYC Tiers**:
| Tier | Name | Description |
|------|------|-------------|
| 0 | None | No verification |
| 1 | Basic | Email/phone verified |
| 2 | Enhanced | ID document verified |
| 3 | Business | Full business verification |

---

### intents

**Description**: Core transaction entities representing payment intents (pay-in, payout, trades, crypto transfers).

| Column | Type | Nullable | Default | Description |
|--------|------|----------|---------|-------------|
| `id` | VARCHAR(64) | NO | - | Unique intent identifier |
| `tenant_id` | VARCHAR(64) | NO | - | Reference to tenant |
| `user_id` | VARCHAR(64) | NO | - | User initiating the intent |
| `intent_type` | VARCHAR(32) | NO | - | Type of transaction |
| `state` | VARCHAR(64) | NO | - | Current state machine state |
| `state_history` | JSONB | NO | '[]' | History of state transitions |
| `amount` | DECIMAL(30,8) | NO | - | Requested amount |
| `currency` | VARCHAR(16) | NO | - | Currency code |
| `actual_amount` | DECIMAL(30,8) | YES | NULL | Final settled amount |
| `rails_provider` | VARCHAR(64) | YES | NULL | Payment provider code |
| `reference_code` | VARCHAR(64) | YES | NULL | External reference code |
| `bank_tx_id` | VARCHAR(128) | YES | NULL | Bank transaction ID |
| `chain_id` | VARCHAR(32) | YES | NULL | Blockchain chain ID |
| `tx_hash` | VARCHAR(128) | YES | NULL | On-chain transaction hash |
| `from_address` | VARCHAR(128) | YES | NULL | Source crypto address |
| `to_address` | VARCHAR(128) | YES | NULL | Destination crypto address |
| `metadata` | JSONB | NO | '{}' | Additional metadata |
| `idempotency_key` | VARCHAR(128) | YES | NULL | Key for idempotent requests |
| `created_at` | TIMESTAMPTZ | NO | NOW() | Intent creation time |
| `updated_at` | TIMESTAMPTZ | NO | NOW() | Last update time |
| `expires_at` | TIMESTAMPTZ | YES | NULL | When intent expires |
| `completed_at` | TIMESTAMPTZ | YES | NULL | Completion timestamp |

**Constraints**:
- PRIMARY KEY: `id`
- FOREIGN KEY: `tenant_id` REFERENCES `tenants(id)`
- CHECK: `intent_type IN ('PAYIN_VND', 'PAYOUT_VND', 'TRADE_EXECUTED', 'DEPOSIT_ONCHAIN', 'WITHDRAW_ONCHAIN')`

**Indexes**:
```sql
CREATE UNIQUE INDEX idx_intents_idempotency ON intents(tenant_id, idempotency_key)
    WHERE idempotency_key IS NOT NULL;
CREATE INDEX idx_intents_tenant_user ON intents(tenant_id, user_id);
CREATE INDEX idx_intents_state ON intents(tenant_id, state);
CREATE INDEX idx_intents_type ON intents(tenant_id, intent_type);
CREATE INDEX idx_intents_reference ON intents(tenant_id, reference_code)
    WHERE reference_code IS NOT NULL;
CREATE INDEX idx_intents_created ON intents(tenant_id, created_at DESC);
CREATE INDEX idx_intents_expires ON intents(expires_at)
    WHERE expires_at IS NOT NULL AND state NOT IN ('COMPLETED', 'EXPIRED', 'CANCELLED');
```

**Intent Types**:
| Type | Description |
|------|-------------|
| `PAYIN_VND` | Fiat deposit (VND) |
| `PAYOUT_VND` | Fiat withdrawal (VND) |
| `TRADE_EXECUTED` | Trading activity |
| `DEPOSIT_ONCHAIN` | Crypto deposit |
| `WITHDRAW_ONCHAIN` | Crypto withdrawal |

---

### ledger_entries

**Description**: Double-entry accounting records. Every financial movement creates matching debit/credit entries.

| Column | Type | Nullable | Default | Description |
|--------|------|----------|---------|-------------|
| `id` | VARCHAR(64) | NO | - | Unique entry identifier |
| `tenant_id` | VARCHAR(64) | NO | - | Reference to tenant |
| `user_id` | VARCHAR(64) | YES | NULL | User (NULL for system accounts) |
| `intent_id` | VARCHAR(64) | NO | - | Related intent |
| `transaction_id` | VARCHAR(64) | NO | - | Groups related entries |
| `account_type` | VARCHAR(64) | NO | - | Account identifier |
| `direction` | VARCHAR(8) | NO | - | DEBIT or CREDIT |
| `amount` | DECIMAL(30,8) | NO | - | Entry amount |
| `currency` | VARCHAR(16) | NO | - | Currency code |
| `balance_after` | DECIMAL(30,8) | NO | - | Balance after this entry |
| `sequence` | BIGSERIAL | - | - | Global ordering sequence |
| `description` | TEXT | YES | NULL | Human-readable description |
| `metadata` | JSONB | YES | '{}' | Additional metadata |
| `created_at` | TIMESTAMPTZ | NO | NOW() | Entry creation time |

**Constraints**:
- PRIMARY KEY: `id`
- FOREIGN KEY: `tenant_id` REFERENCES `tenants(id)`
- FOREIGN KEY: `intent_id` REFERENCES `intents(id)`
- CHECK: `direction IN ('DEBIT', 'CREDIT')`
- CHECK: `amount >= 0`

**Indexes**:
```sql
CREATE INDEX idx_ledger_tenant ON ledger_entries(tenant_id);
CREATE INDEX idx_ledger_intent ON ledger_entries(intent_id);
CREATE INDEX idx_ledger_transaction ON ledger_entries(transaction_id);
CREATE INDEX idx_ledger_account ON ledger_entries(tenant_id, account_type, currency);
CREATE INDEX idx_ledger_user ON ledger_entries(tenant_id, user_id) WHERE user_id IS NOT NULL;
CREATE INDEX idx_ledger_sequence ON ledger_entries(sequence);
```

**Account Types**:
| Account Type | Category | Description |
|--------------|----------|-------------|
| `ASSET_BANK_*` | Asset | Bank holdings (e.g., ASSET_BANK_VCB) |
| `LIABILITY_USER_MAIN` | Liability | User balances |

---

### account_balances

**Description**: Materialized view for fast balance queries. Updated transactionally with ledger entries.

| Column | Type | Nullable | Default | Description |
|--------|------|----------|---------|-------------|
| `tenant_id` | VARCHAR(64) | NO | - | Reference to tenant |
| `user_id` | VARCHAR(64) | YES | NULL | User ID (NULL or '' for system) |
| `account_type` | VARCHAR(64) | NO | - | Account identifier |
| `currency` | VARCHAR(16) | NO | - | Currency code |
| `balance` | DECIMAL(30,8) | NO | 0 | Current balance |
| `last_entry_id` | VARCHAR(64) | YES | NULL | Last ledger entry ID |
| `last_sequence` | BIGINT | YES | NULL | Last ledger sequence |
| `updated_at` | TIMESTAMPTZ | NO | NOW() | Last update time |

**Constraints**:
- PRIMARY KEY: `(tenant_id, COALESCE(user_id, ''), account_type, currency)`
- FOREIGN KEY: `tenant_id` REFERENCES `tenants(id)`

**Indexes**:
```sql
CREATE INDEX idx_balances_user ON account_balances(tenant_id, user_id)
    WHERE user_id IS NOT NULL;
```

---

### webhook_events

**Description**: Outbox pattern implementation for reliable webhook delivery.

| Column | Type | Nullable | Default | Description |
|--------|------|----------|---------|-------------|
| `id` | VARCHAR(64) | NO | - | Unique event identifier |
| `tenant_id` | VARCHAR(64) | NO | - | Reference to tenant |
| `event_type` | VARCHAR(64) | NO | - | Type of event |
| `intent_id` | VARCHAR(64) | YES | NULL | Related intent |
| `payload` | JSONB | NO | - | Event payload |
| `status` | VARCHAR(32) | NO | 'PENDING' | Delivery status |
| `attempts` | INT | NO | 0 | Delivery attempts made |
| `max_attempts` | INT | NO | 10 | Maximum retry attempts |
| `last_attempt_at` | TIMESTAMPTZ | YES | NULL | Last attempt time |
| `next_attempt_at` | TIMESTAMPTZ | YES | NULL | Next scheduled attempt |
| `last_error` | TEXT | YES | NULL | Last error message |
| `delivered_at` | TIMESTAMPTZ | YES | NULL | Successful delivery time |
| `response_status` | INT | YES | NULL | HTTP response status |
| `created_at` | TIMESTAMPTZ | NO | NOW() | Event creation time |

**Constraints**:
- PRIMARY KEY: `id`
- FOREIGN KEY: `tenant_id` REFERENCES `tenants(id)`
- FOREIGN KEY: `intent_id` REFERENCES `intents(id)`
- CHECK: `status IN ('PENDING', 'DELIVERED', 'FAILED', 'CANCELLED')`

**Indexes**:
```sql
CREATE INDEX idx_webhooks_pending ON webhook_events(next_attempt_at)
    WHERE status = 'PENDING';
CREATE INDEX idx_webhooks_tenant ON webhook_events(tenant_id, created_at DESC);
CREATE INDEX idx_webhooks_intent ON webhook_events(intent_id);
```

---

### rails_adapters

**Description**: Bank/PSP/Crypto provider configurations per tenant.

| Column | Type | Nullable | Default | Description |
|--------|------|----------|---------|-------------|
| `id` | VARCHAR(64) | NO | - | Unique adapter identifier |
| `tenant_id` | VARCHAR(64) | NO | - | Reference to tenant |
| `provider_code` | VARCHAR(64) | NO | - | Provider code (e.g., VCB_DIRECT) |
| `provider_name` | VARCHAR(255) | NO | - | Human-readable provider name |
| `adapter_type` | VARCHAR(32) | NO | - | Type: BANK, PSP, or CRYPTO |
| `config_encrypted` | BYTEA | NO | - | Encrypted configuration |
| `supports_payin` | BOOLEAN | NO | true | Supports pay-in operations |
| `supports_payout` | BOOLEAN | NO | true | Supports payout operations |
| `supports_virtual_account` | BOOLEAN | NO | false | Supports virtual accounts |
| `status` | VARCHAR(32) | NO | 'ACTIVE' | Adapter status |
| `created_at` | TIMESTAMPTZ | NO | NOW() | Creation timestamp |
| `updated_at` | TIMESTAMPTZ | NO | NOW() | Last update timestamp |

**Constraints**:
- PRIMARY KEY: `id`
- FOREIGN KEY: `tenant_id` REFERENCES `tenants(id)`
- UNIQUE: `(tenant_id, provider_code)`
- CHECK: `status IN ('ACTIVE', 'DISABLED', 'TESTING')`

---

### virtual_accounts

**Description**: Virtual bank accounts for pay-in tracking.

| Column | Type | Nullable | Default | Description |
|--------|------|----------|---------|-------------|
| `id` | VARCHAR(64) | NO | - | Unique VA identifier |
| `tenant_id` | VARCHAR(64) | NO | - | Reference to tenant |
| `user_id` | VARCHAR(64) | NO | - | Owner user ID |
| `rails_adapter_id` | VARCHAR(64) | NO | - | Reference to rails adapter |
| `bank_code` | VARCHAR(32) | NO | - | Bank code |
| `account_number` | VARCHAR(64) | NO | - | Virtual account number |
| `account_name` | VARCHAR(255) | NO | - | Account holder name |
| `status` | VARCHAR(32) | NO | 'ACTIVE' | VA status |
| `created_at` | TIMESTAMPTZ | NO | NOW() | Creation timestamp |
| `expires_at` | TIMESTAMPTZ | YES | NULL | Expiration time |

**Constraints**:
- PRIMARY KEY: `id`
- FOREIGN KEY: `tenant_id` REFERENCES `tenants(id)`
- FOREIGN KEY: `rails_adapter_id` REFERENCES `rails_adapters(id)`
- UNIQUE: `(bank_code, account_number)`

**Indexes**:
```sql
CREATE INDEX idx_va_tenant_user ON virtual_accounts(tenant_id, user_id);
CREATE INDEX idx_va_account ON virtual_accounts(bank_code, account_number);
```

---

### kyc_records

**Description**: KYC verification records for users.

| Column | Type | Nullable | Default | Description |
|--------|------|----------|---------|-------------|
| `id` | VARCHAR(64) | NO | - | Unique record identifier |
| `tenant_id` | VARCHAR(64) | NO | - | Reference to tenant |
| `user_id` | VARCHAR(64) | NO | - | Reference to user |
| `tier` | SMALLINT | NO | - | KYC tier being verified |
| `provider` | VARCHAR(64) | YES | NULL | KYC provider name |
| `provider_reference` | VARCHAR(128) | YES | NULL | External reference |
| `status` | VARCHAR(32) | NO | - | Verification status |
| `verification_data` | JSONB | NO | '{}' | Verification details |
| `rejection_reason` | TEXT | YES | NULL | Reason if rejected |
| `documents` | JSONB | YES | '[]' | Document references |
| `submitted_at` | TIMESTAMPTZ | NO | NOW() | Submission time |
| `verified_at` | TIMESTAMPTZ | YES | NULL | Verification completion time |
| `expires_at` | TIMESTAMPTZ | YES | NULL | Verification expiration |

**Constraints**:
- PRIMARY KEY: `id`
- FOREIGN KEY: `(tenant_id, user_id)` REFERENCES `users(tenant_id, id)`

**Indexes**:
```sql
CREATE INDEX idx_kyc_user ON kyc_records(tenant_id, user_id);
CREATE INDEX idx_kyc_status ON kyc_records(tenant_id, status);
```

**KYC Status Values**:
| Status | Description |
|--------|-------------|
| `PENDING` | Awaiting verification |
| `APPROVED` | Verified successfully |
| `REJECTED` | Verification failed |
| `EXPIRED` | Verification expired |

---

### aml_cases

**Description**: Anti-Money Laundering case management.

| Column | Type | Nullable | Default | Description |
|--------|------|----------|---------|-------------|
| `id` | VARCHAR(64) | NO | - | Unique case identifier |
| `tenant_id` | VARCHAR(64) | NO | - | Reference to tenant |
| `user_id` | VARCHAR(64) | YES | NULL | Related user |
| `intent_id` | VARCHAR(64) | YES | NULL | Related intent |
| `case_type` | VARCHAR(64) | NO | - | Type of AML case |
| `severity` | VARCHAR(16) | NO | - | Case severity |
| `status` | VARCHAR(32) | NO | 'OPEN' | Case status |
| `rule_id` | VARCHAR(64) | YES | NULL | Triggering rule ID |
| `rule_name` | VARCHAR(255) | YES | NULL | Triggering rule name |
| `detection_data` | JSONB | NO | - | Detection details |
| `assigned_to` | VARCHAR(64) | YES | NULL | Assigned reviewer |
| `resolution` | TEXT | YES | NULL | Resolution notes |
| `resolved_at` | TIMESTAMPTZ | YES | NULL | Resolution timestamp |
| `created_at` | TIMESTAMPTZ | NO | NOW() | Case creation time |
| `updated_at` | TIMESTAMPTZ | NO | NOW() | Last update time |

**Constraints**:
- PRIMARY KEY: `id`
- FOREIGN KEY: `tenant_id` REFERENCES `tenants(id)`
- FOREIGN KEY: `intent_id` REFERENCES `intents(id)`

**Indexes**:
```sql
CREATE INDEX idx_aml_tenant ON aml_cases(tenant_id);
CREATE INDEX idx_aml_user ON aml_cases(tenant_id, user_id) WHERE user_id IS NOT NULL;
CREATE INDEX idx_aml_status ON aml_cases(tenant_id, status);
CREATE INDEX idx_aml_severity ON aml_cases(tenant_id, severity, status);
```

**Case Types**:
| Type | Description |
|------|-------------|
| `VELOCITY` | High transaction velocity |
| `STRUCTURING` | Potential structuring behavior |
| `NAME_MISMATCH` | Name mismatch in transaction |
| `SANCTIONS` | Sanctions list match |
| `PEP` | Politically Exposed Person |

**Severity Levels**:
| Severity | Description |
|----------|-------------|
| `LOW` | Minor risk indicator |
| `MEDIUM` | Moderate concern |
| `HIGH` | Significant risk |
| `CRITICAL` | Immediate action required |

**Case Status**:
| Status | Description |
|--------|-------------|
| `OPEN` | Newly created case |
| `REVIEW` | Under investigation |
| `HOLD` | Transaction on hold |
| `RELEASED` | Case cleared |
| `REPORTED` | Reported to authorities |

---

### aml_rule_versions

**Description**: Versioned AML rule configurations per tenant.

| Column | Type | Nullable | Default | Description |
|--------|------|----------|---------|-------------|
| `id` | UUID | NO | - | Unique version identifier |
| `tenant_id` | VARCHAR(64) | NO | - | Reference to tenant |
| `version_number` | INT | NO | - | Version number |
| `rules_json` | JSONB | NO | - | Rule configuration |
| `is_active` | BOOLEAN | YES | false | Whether version is active |
| `created_at` | TIMESTAMPTZ | NO | NOW() | Creation timestamp |
| `created_by` | VARCHAR(255) | YES | NULL | Creator identifier |
| `activated_at` | TIMESTAMPTZ | YES | NULL | Activation timestamp |

**Constraints**:
- PRIMARY KEY: `id`
- FOREIGN KEY: `tenant_id` REFERENCES `tenants(id)`
- UNIQUE: `(tenant_id, version_number)`

**Indexes**:
```sql
CREATE INDEX idx_rule_versions_tenant ON aml_rule_versions(tenant_id, version_number DESC);
CREATE INDEX idx_rule_versions_active ON aml_rule_versions(tenant_id) WHERE is_active = true;
```

---

### risk_score_history

**Description**: Historical record of user risk score changes.

| Column | Type | Nullable | Default | Description |
|--------|------|----------|---------|-------------|
| `id` | UUID | NO | - | Unique record identifier |
| `tenant_id` | VARCHAR(64) | YES | NULL | Reference to tenant |
| `user_id` | VARCHAR(255) | NO | - | Reference to user |
| `intent_id` | VARCHAR(255) | YES | NULL | Triggering intent |
| `score` | DECIMAL(5,2) | NO | - | Calculated risk score |
| `triggered_rules` | JSONB | YES | NULL | Rules that affected score |
| `action_taken` | VARCHAR(50) | YES | NULL | Action taken (if any) |
| `created_at` | TIMESTAMPTZ | YES | NOW() | Record timestamp |

**Constraints**:
- PRIMARY KEY: `id`

**Indexes**:
```sql
CREATE INDEX idx_score_history_user ON risk_score_history(user_id, created_at DESC);
CREATE INDEX idx_risk_score_history_tenant ON risk_score_history(tenant_id);
```

---

### case_notes

**Description**: Notes and comments on AML cases.

| Column | Type | Nullable | Default | Description |
|--------|------|----------|---------|-------------|
| `id` | UUID | NO | - | Unique note identifier |
| `tenant_id` | VARCHAR(64) | YES | NULL | Reference to tenant |
| `case_id` | VARCHAR(255) | NO | - | Reference to AML case |
| `author_id` | VARCHAR(255) | YES | NULL | Note author |
| `content` | TEXT | NO | - | Note content |
| `note_type` | VARCHAR(50) | NO | - | Type of note |
| `is_internal` | BOOLEAN | YES | true | Internal visibility only |
| `created_at` | TIMESTAMPTZ | YES | NOW() | Note creation time |

**Constraints**:
- PRIMARY KEY: `id`

**Indexes**:
```sql
CREATE INDEX idx_case_notes_tenant ON case_notes(tenant_id);
```

---

### compliance_transactions

**Description**: Transactions tracked for compliance purposes (velocity checks, limits).

| Column | Type | Nullable | Default | Description |
|--------|------|----------|---------|-------------|
| `id` | UUID | NO | - | Unique transaction identifier |
| `tenant_id` | VARCHAR(64) | NO | - | Reference to tenant |
| `user_id` | VARCHAR(64) | NO | - | Reference to user |
| `intent_id` | VARCHAR(64) | NO | - | Reference to intent |
| `transaction_type` | VARCHAR(32) | NO | - | Transaction type |
| `amount_vnd` | DECIMAL(30,8) | NO | - | Amount in VND |
| `created_at` | TIMESTAMPTZ | NO | NOW() | Transaction timestamp |

**Constraints**:
- PRIMARY KEY: `id`

**Indexes**:
```sql
CREATE INDEX idx_compliance_tx_tenant_user_time
    ON compliance_transactions(tenant_id, user_id, created_at DESC);
CREATE INDEX idx_compliance_tx_type ON compliance_transactions(transaction_type);
```

---

### audit_log

**Description**: Append-only audit trail with hash chain integrity.

| Column | Type | Nullable | Default | Description |
|--------|------|----------|---------|-------------|
| `id` | BIGSERIAL | NO | - | Unique entry identifier |
| `tenant_id` | VARCHAR(64) | NO | - | Reference to tenant |
| `actor_type` | VARCHAR(32) | NO | - | Type of actor |
| `actor_id` | VARCHAR(64) | YES | NULL | Actor identifier |
| `action` | VARCHAR(64) | NO | - | Action performed |
| `resource_type` | VARCHAR(64) | NO | - | Resource type affected |
| `resource_id` | VARCHAR(64) | YES | NULL | Resource identifier |
| `details` | JSONB | NO | '{}' | Action details |
| `ip_address` | INET | YES | NULL | Client IP address |
| `user_agent` | TEXT | YES | NULL | Client user agent |
| `request_id` | VARCHAR(64) | YES | NULL | Request correlation ID |
| `created_at` | TIMESTAMPTZ | NO | NOW() | Entry timestamp |
| `prev_hash` | VARCHAR(64) | YES | NULL | Previous entry hash |
| `entry_hash` | VARCHAR(64) | NO | - | Current entry hash |

**Constraints**:
- PRIMARY KEY: `id`

**Indexes**:
```sql
CREATE INDEX idx_audit_tenant ON audit_log(tenant_id, created_at DESC);
CREATE INDEX idx_audit_resource ON audit_log(tenant_id, resource_type, resource_id);
CREATE INDEX idx_audit_actor ON audit_log(tenant_id, actor_type, actor_id);
```

**Actor Types**:
| Type | Description |
|------|-------------|
| `SYSTEM` | Automated system action |
| `USER` | End user action |
| `ADMIN` | Admin action |
| `API` | API client action |

---

### recon_batches

**Description**: Reconciliation batch records for provider matching.

| Column | Type | Nullable | Default | Description |
|--------|------|----------|---------|-------------|
| `id` | VARCHAR(64) | NO | - | Unique batch identifier |
| `tenant_id` | VARCHAR(64) | NO | - | Reference to tenant |
| `rails_adapter_id` | VARCHAR(64) | YES | NULL | Reference to adapter |
| `period_start` | TIMESTAMPTZ | NO | - | Batch period start |
| `period_end` | TIMESTAMPTZ | NO | - | Batch period end |
| `status` | VARCHAR(32) | NO | 'PENDING' | Batch status |
| `total_intents` | INT | NO | 0 | Total intents in batch |
| `matched_intents` | INT | NO | 0 | Successfully matched |
| `unmatched_intents` | INT | NO | 0 | Unmatched intents |
| `discrepancy_amount` | DECIMAL(30,8) | YES | 0 | Amount discrepancy |
| `our_file_hash` | VARCHAR(128) | YES | NULL | Hash of our file |
| `provider_file_hash` | VARCHAR(128) | YES | NULL | Hash of provider file |
| `report_url` | TEXT | YES | NULL | Report URL |
| `created_at` | TIMESTAMPTZ | NO | NOW() | Batch creation time |
| `completed_at` | TIMESTAMPTZ | YES | NULL | Completion time |

**Constraints**:
- PRIMARY KEY: `id`
- FOREIGN KEY: `tenant_id` REFERENCES `tenants(id)`
- FOREIGN KEY: `rails_adapter_id` REFERENCES `rails_adapters(id)`

**Indexes**:
```sql
CREATE INDEX idx_recon_tenant ON recon_batches(tenant_id, created_at DESC);
CREATE INDEX idx_recon_status ON recon_batches(status) WHERE status = 'PENDING';
```

---

## Functions & Triggers

### update_updated_at()

Automatically updates the `updated_at` timestamp on record modification.

```sql
CREATE OR REPLACE FUNCTION update_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;
```

**Applied to tables**:
- `tenants`
- `users`
- `intents`
- `aml_cases`

### append_state_history()

Automatically appends state transitions to the `state_history` JSONB array.

```sql
CREATE OR REPLACE FUNCTION append_state_history()
RETURNS TRIGGER AS $$
BEGIN
    IF OLD.state IS DISTINCT FROM NEW.state THEN
        NEW.state_history = OLD.state_history || jsonb_build_object(
            'from', OLD.state,
            'to', NEW.state,
            'at', NOW()
        );
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;
```

**Applied to**:
- `intents` table

---

## Entity Relationship Diagram

```
+----------------+       +---------------+       +------------------+
|    tenants     |<------+    users      |       |   kyc_records    |
+----------------+       +---------------+       +------------------+
| id (PK)        |       | id            |------>| id (PK)          |
| name           |       | tenant_id (FK)|       | tenant_id        |
| status         |       | kyc_tier      |       | user_id          |
| api_key_hash   |       | kyc_status    |       | tier             |
| config         |       | risk_score    |       | status           |
+-------+--------+       +-------+-------+       +------------------+
        |                        |
        |                        |
        v                        v
+----------------+       +---------------+       +------------------+
| rails_adapters |       |    intents    |------>| ledger_entries   |
+----------------+       +---------------+       +------------------+
| id (PK)        |       | id (PK)       |       | id (PK)          |
| tenant_id (FK) |       | tenant_id (FK)|       | tenant_id        |
| provider_code  |       | user_id       |       | intent_id (FK)   |
| adapter_type   |       | intent_type   |       | account_type     |
| config_encrypted|      | state         |       | direction        |
+-------+--------+       | amount        |       | amount           |
        |                +-------+-------+       +------------------+
        |                        |
        v                        v
+------------------+     +------------------+    +------------------+
| virtual_accounts |     |  webhook_events  |    | account_balances |
+------------------+     +------------------+    +------------------+
| id (PK)          |     | id (PK)          |    | tenant_id        |
| tenant_id        |     | tenant_id        |    | user_id          |
| user_id          |     | intent_id (FK)   |    | account_type     |
| rails_adapter_id |     | event_type       |    | currency         |
| bank_code        |     | payload          |    | balance          |
| account_number   |     | status           |    +------------------+
+------------------+     +------------------+

+------------------+     +------------------+    +------------------+
|    aml_cases     |     |aml_rule_versions |    |risk_score_history|
+------------------+     +------------------+    +------------------+
| id (PK)          |     | id (PK)          |    | id (PK)          |
| tenant_id        |     | tenant_id        |    | tenant_id        |
| user_id          |     | version_number   |    | user_id          |
| intent_id        |     | rules_json       |    | score            |
| case_type        |     | is_active        |    | triggered_rules  |
| severity         |     +------------------+    +------------------+
| status           |
+------------------+     +------------------+    +------------------+
        |                |   case_notes     |    |compliance_txs    |
        |                +------------------+    +------------------+
        +--------------->| id (PK)          |    | id (PK)          |
                         | tenant_id        |    | tenant_id        |
                         | case_id          |    | user_id          |
                         | content          |    | intent_id        |
                         +------------------+    | amount_vnd       |
                                                 +------------------+

+------------------+     +------------------+
|    audit_log     |     |  recon_batches   |
+------------------+     +------------------+
| id (PK)          |     | id (PK)          |
| tenant_id        |     | tenant_id        |
| actor_type       |     | rails_adapter_id |
| action           |     | period_start     |
| resource_type    |     | period_end       |
| entry_hash       |     | status           |
+------------------+     +------------------+
```

---

## See Also

- [Migration History](./migrations.md)
- [Row Level Security](./rls.md)
