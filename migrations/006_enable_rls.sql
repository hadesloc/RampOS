-- Enable RLS on tables
ALTER TABLE users ENABLE ROW LEVEL SECURITY;
ALTER TABLE intents ENABLE ROW LEVEL SECURITY;
ALTER TABLE ledger_entries ENABLE ROW LEVEL SECURITY;
ALTER TABLE account_balances ENABLE ROW LEVEL SECURITY;
ALTER TABLE webhook_events ENABLE ROW LEVEL SECURITY;
ALTER TABLE rails_adapters ENABLE ROW LEVEL SECURITY;
ALTER TABLE virtual_accounts ENABLE ROW LEVEL SECURITY;
ALTER TABLE kyc_records ENABLE ROW LEVEL SECURITY;
ALTER TABLE aml_cases ENABLE ROW LEVEL SECURITY;
ALTER TABLE audit_log ENABLE ROW LEVEL SECURITY;
ALTER TABLE recon_batches ENABLE ROW LEVEL SECURITY;

-- Create policies
-- We use a session variable 'app.current_tenant' which should be set by the application

-- Users
CREATE POLICY tenant_isolation_users ON users
    USING (tenant_id = current_setting('app.current_tenant')::VARCHAR);

-- Intents
CREATE POLICY tenant_isolation_intents ON intents
    USING (tenant_id = current_setting('app.current_tenant')::VARCHAR);

-- Ledger Entries
CREATE POLICY tenant_isolation_ledger_entries ON ledger_entries
    USING (tenant_id = current_setting('app.current_tenant')::VARCHAR);

-- Account Balances
CREATE POLICY tenant_isolation_account_balances ON account_balances
    USING (tenant_id = current_setting('app.current_tenant')::VARCHAR);

-- Webhook Events
CREATE POLICY tenant_isolation_webhook_events ON webhook_events
    USING (tenant_id = current_setting('app.current_tenant')::VARCHAR);

-- Rails Adapters
CREATE POLICY tenant_isolation_rails_adapters ON rails_adapters
    USING (tenant_id = current_setting('app.current_tenant')::VARCHAR);

-- Virtual Accounts
CREATE POLICY tenant_isolation_virtual_accounts ON virtual_accounts
    USING (tenant_id = current_setting('app.current_tenant')::VARCHAR);

-- KYC Records
CREATE POLICY tenant_isolation_kyc_records ON kyc_records
    USING (tenant_id = current_setting('app.current_tenant')::VARCHAR);

-- AML Cases
CREATE POLICY tenant_isolation_aml_cases ON aml_cases
    USING (tenant_id = current_setting('app.current_tenant')::VARCHAR);

-- Audit Log
CREATE POLICY tenant_isolation_audit_log ON audit_log
    USING (tenant_id = current_setting('app.current_tenant')::VARCHAR);

-- Recon Batches
CREATE POLICY tenant_isolation_recon_batches ON recon_batches
    USING (tenant_id = current_setting('app.current_tenant')::VARCHAR);
