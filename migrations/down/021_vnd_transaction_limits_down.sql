-- Down migration for 021_vnd_transaction_limits.sql
-- Drops VND transaction limits tables and functions

-- Drop RLS policies
DROP POLICY IF EXISTS tx_history_tenant_isolation ON transaction_limit_history;
DROP POLICY IF EXISTS vnd_config_tenant_isolation ON vnd_limit_config;
DROP POLICY IF EXISTS user_limits_tenant_isolation ON user_transaction_limits;

-- Drop functions
DROP FUNCTION IF EXISTS record_transaction_for_limits(VARCHAR, VARCHAR, VARCHAR, VARCHAR, DECIMAL);
DROP FUNCTION IF EXISTS check_vnd_transaction_limit(VARCHAR, VARCHAR, DECIMAL);
DROP FUNCTION IF EXISTS get_user_monthly_used_vnd(VARCHAR, VARCHAR, VARCHAR);
DROP FUNCTION IF EXISTS get_user_daily_used_vnd(VARCHAR, VARCHAR, DATE);

-- Drop triggers
DROP TRIGGER IF EXISTS trigger_vnd_limit_config_updated_at ON vnd_limit_config;
DROP TRIGGER IF EXISTS trigger_user_limits_updated_at ON user_transaction_limits;

-- Drop tables (in dependency order)
DROP INDEX IF EXISTS idx_tx_limit_history_type;
DROP INDEX IF EXISTS idx_tx_limit_history_intent;
DROP INDEX IF EXISTS idx_tx_limit_history_monthly;
DROP INDEX IF EXISTS idx_tx_limit_history_daily;
DROP TABLE IF EXISTS transaction_limit_history;

DROP INDEX IF EXISTS idx_vnd_limit_config_tenant;
DROP TABLE IF EXISTS vnd_limit_config;

DROP INDEX IF EXISTS idx_user_limits_tier;
DROP INDEX IF EXISTS idx_user_limits_tenant;
DROP TABLE IF EXISTS user_transaction_limits;
