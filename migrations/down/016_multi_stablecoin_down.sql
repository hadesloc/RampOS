-- Down migration for 016_multi_stablecoin.sql
-- Drops multi-stablecoin tables, view, triggers, and function

-- Drop RLS policies
DROP POLICY IF EXISTS token_transactions_tenant_isolation ON token_transactions;
DROP POLICY IF EXISTS token_balances_tenant_isolation ON token_balances;
DROP POLICY IF EXISTS token_deployments_tenant_isolation ON token_chain_deployments;
DROP POLICY IF EXISTS supported_tokens_tenant_isolation ON supported_tokens;

-- Drop triggers
DROP TRIGGER IF EXISTS token_transactions_updated_at ON token_transactions;
DROP TRIGGER IF EXISTS token_balances_updated_at ON token_balances;
DROP TRIGGER IF EXISTS token_chain_deployments_updated_at ON token_chain_deployments;
DROP TRIGGER IF EXISTS supported_tokens_updated_at ON supported_tokens;

-- Drop view
DROP VIEW IF EXISTS user_token_balances;

-- Drop indexes
DROP INDEX IF EXISTS idx_token_transactions_status;
DROP INDEX IF EXISTS idx_token_transactions_intent;
DROP INDEX IF EXISTS idx_token_transactions_hash;
DROP INDEX IF EXISTS idx_token_transactions_user;
DROP INDEX IF EXISTS idx_token_balances_symbol;
DROP INDEX IF EXISTS idx_token_balances_user;
DROP INDEX IF EXISTS idx_token_deployments_chain;
DROP INDEX IF EXISTS idx_supported_tokens_symbol;
DROP INDEX IF EXISTS idx_supported_tokens_tenant;

-- Drop tables (in dependency order)
DROP TABLE IF EXISTS token_transactions;
DROP TABLE IF EXISTS token_balances;
DROP TABLE IF EXISTS token_chain_deployments;
DROP TABLE IF EXISTS supported_tokens;

-- Drop the function created in this migration
DROP FUNCTION IF EXISTS update_updated_at_column();
