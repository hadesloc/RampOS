-- Down migration for 001_initial_schema.sql
-- Drops ALL tables, functions, and triggers created in the initial schema
-- WARNING: This will destroy all data. Only use for clean reset.

-- Drop triggers first
DROP TRIGGER IF EXISTS trigger_intent_state_history ON intents;
DROP TRIGGER IF EXISTS trigger_aml_cases_updated_at ON aml_cases;
DROP TRIGGER IF EXISTS trigger_intents_updated_at ON intents;
DROP TRIGGER IF EXISTS trigger_users_updated_at ON users;
DROP TRIGGER IF EXISTS trigger_tenants_updated_at ON tenants;

-- Drop functions
DROP FUNCTION IF EXISTS append_state_history();
DROP FUNCTION IF EXISTS update_updated_at();

-- Drop tables in reverse dependency order
-- (tables with FKs to other tables must be dropped first)

-- Recon batches (depends on tenants, rails_adapters)
DROP TABLE IF EXISTS recon_batches;

-- Audit log (depends on nothing, but has tenant_id)
DROP TABLE IF EXISTS audit_log;

-- AML Cases (depends on tenants, intents)
DROP TABLE IF EXISTS aml_cases;

-- KYC Records (depends on users via composite FK)
DROP TABLE IF EXISTS kyc_records;

-- Virtual Accounts (depends on tenants, rails_adapters)
DROP TABLE IF EXISTS virtual_accounts;

-- Rails Adapters (depends on tenants)
DROP TABLE IF EXISTS rails_adapters;

-- Webhook Events (depends on tenants, intents)
DROP TABLE IF EXISTS webhook_events;

-- Account Balances (depends on tenants)
DROP TABLE IF EXISTS account_balances;

-- Ledger Entries (depends on tenants, intents)
DROP TABLE IF EXISTS ledger_entries;

-- Intents (depends on tenants)
DROP TABLE IF EXISTS intents;

-- Users (depends on tenants)
DROP TABLE IF EXISTS users;

-- Tenants (base table)
DROP TABLE IF EXISTS tenants;

-- Note: Extensions (uuid-ossp, pgcrypto) are NOT dropped
-- as they may be used by other schemas/applications.
