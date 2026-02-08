-- Down migration for 007_compliance_transactions.sql
-- Drops compliance_transactions table

DROP INDEX IF EXISTS idx_compliance_tx_type;
DROP INDEX IF EXISTS idx_compliance_tx_tenant_user_time;
DROP TABLE IF EXISTS compliance_transactions;
