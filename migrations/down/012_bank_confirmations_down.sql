-- Down migration for 012_bank_confirmations.sql
-- Drops bank confirmations and webhook secrets tables

-- Drop RLS policies
DROP POLICY IF EXISTS bank_webhook_secrets_tenant_isolation ON bank_webhook_secrets;
DROP POLICY IF EXISTS bank_confirmations_tenant_isolation ON bank_confirmations;

-- Drop triggers
DROP TRIGGER IF EXISTS trigger_bank_webhook_secrets_updated_at ON bank_webhook_secrets;
DROP TRIGGER IF EXISTS trigger_bank_confirmations_updated_at ON bank_confirmations;

-- Drop indexes
DROP INDEX IF EXISTS idx_bank_webhook_secrets_lookup;
DROP INDEX IF EXISTS idx_bank_confirmations_unique;
DROP INDEX IF EXISTS idx_bank_confirmations_intent;
DROP INDEX IF EXISTS idx_bank_confirmations_provider;
DROP INDEX IF EXISTS idx_bank_confirmations_status;
DROP INDEX IF EXISTS idx_bank_confirmations_bank_tx;
DROP INDEX IF EXISTS idx_bank_confirmations_reference;
DROP INDEX IF EXISTS idx_bank_confirmations_tenant;

-- Drop tables
DROP TABLE IF EXISTS bank_webhook_secrets;
DROP TABLE IF EXISTS bank_confirmations;
