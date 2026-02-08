-- Down migration for 006_enable_rls.sql
-- Disables RLS and drops all tenant isolation policies

-- Drop policies
DROP POLICY IF EXISTS tenant_isolation_recon_batches ON recon_batches;
DROP POLICY IF EXISTS tenant_isolation_audit_log ON audit_log;
DROP POLICY IF EXISTS tenant_isolation_aml_cases ON aml_cases;
DROP POLICY IF EXISTS tenant_isolation_kyc_records ON kyc_records;
DROP POLICY IF EXISTS tenant_isolation_virtual_accounts ON virtual_accounts;
DROP POLICY IF EXISTS tenant_isolation_rails_adapters ON rails_adapters;
DROP POLICY IF EXISTS tenant_isolation_webhook_events ON webhook_events;
DROP POLICY IF EXISTS tenant_isolation_account_balances ON account_balances;
DROP POLICY IF EXISTS tenant_isolation_ledger_entries ON ledger_entries;
DROP POLICY IF EXISTS tenant_isolation_intents ON intents;
DROP POLICY IF EXISTS tenant_isolation_users ON users;

-- Disable RLS
ALTER TABLE recon_batches DISABLE ROW LEVEL SECURITY;
ALTER TABLE audit_log DISABLE ROW LEVEL SECURITY;
ALTER TABLE aml_cases DISABLE ROW LEVEL SECURITY;
ALTER TABLE kyc_records DISABLE ROW LEVEL SECURITY;
ALTER TABLE virtual_accounts DISABLE ROW LEVEL SECURITY;
ALTER TABLE rails_adapters DISABLE ROW LEVEL SECURITY;
ALTER TABLE webhook_events DISABLE ROW LEVEL SECURITY;
ALTER TABLE account_balances DISABLE ROW LEVEL SECURITY;
ALTER TABLE ledger_entries DISABLE ROW LEVEL SECURITY;
ALTER TABLE intents DISABLE ROW LEVEL SECURITY;
ALTER TABLE users DISABLE ROW LEVEL SECURITY;
