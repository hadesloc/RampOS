-- Down migration for 014_rls_fail_closed.sql
-- Reverts FORCE RLS and restores simpler RLS policies from 006_enable_rls.sql

-- Remove FORCE RLS (revert to standard RLS)
ALTER TABLE users NO FORCE ROW LEVEL SECURITY;
ALTER TABLE intents NO FORCE ROW LEVEL SECURITY;
ALTER TABLE ledger_entries NO FORCE ROW LEVEL SECURITY;
ALTER TABLE account_balances NO FORCE ROW LEVEL SECURITY;
ALTER TABLE webhook_events NO FORCE ROW LEVEL SECURITY;
ALTER TABLE rails_adapters NO FORCE ROW LEVEL SECURITY;
ALTER TABLE virtual_accounts NO FORCE ROW LEVEL SECURITY;
ALTER TABLE kyc_records NO FORCE ROW LEVEL SECURITY;
ALTER TABLE aml_cases NO FORCE ROW LEVEL SECURITY;
ALTER TABLE audit_log NO FORCE ROW LEVEL SECURITY;
ALTER TABLE recon_batches NO FORCE ROW LEVEL SECURITY;
ALTER TABLE aml_rule_versions NO FORCE ROW LEVEL SECURITY;
ALTER TABLE risk_score_history NO FORCE ROW LEVEL SECURITY;
ALTER TABLE case_notes NO FORCE ROW LEVEL SECURITY;
ALTER TABLE compliance_transactions NO FORCE ROW LEVEL SECURITY;
ALTER TABLE smart_accounts NO FORCE ROW LEVEL SECURITY;
ALTER TABLE bank_confirmations NO FORCE ROW LEVEL SECURITY;
ALTER TABLE bank_webhook_secrets NO FORCE ROW LEVEL SECURITY;

-- Revert fail-closed policies back to simple policies
-- (These policies were originally created in 006/008 with simpler USING clauses)

DROP POLICY IF EXISTS tenant_isolation_users ON users;
CREATE POLICY tenant_isolation_users ON users
    USING (tenant_id = current_setting('app.current_tenant')::VARCHAR);

DROP POLICY IF EXISTS tenant_isolation_intents ON intents;
CREATE POLICY tenant_isolation_intents ON intents
    USING (tenant_id = current_setting('app.current_tenant')::VARCHAR);

DROP POLICY IF EXISTS tenant_isolation_ledger_entries ON ledger_entries;
CREATE POLICY tenant_isolation_ledger_entries ON ledger_entries
    USING (tenant_id = current_setting('app.current_tenant')::VARCHAR);

DROP POLICY IF EXISTS tenant_isolation_account_balances ON account_balances;
CREATE POLICY tenant_isolation_account_balances ON account_balances
    USING (tenant_id = current_setting('app.current_tenant')::VARCHAR);

DROP POLICY IF EXISTS tenant_isolation_webhook_events ON webhook_events;
CREATE POLICY tenant_isolation_webhook_events ON webhook_events
    USING (tenant_id = current_setting('app.current_tenant')::VARCHAR);

DROP POLICY IF EXISTS tenant_isolation_rails_adapters ON rails_adapters;
CREATE POLICY tenant_isolation_rails_adapters ON rails_adapters
    USING (tenant_id = current_setting('app.current_tenant')::VARCHAR);

DROP POLICY IF EXISTS tenant_isolation_virtual_accounts ON virtual_accounts;
CREATE POLICY tenant_isolation_virtual_accounts ON virtual_accounts
    USING (tenant_id = current_setting('app.current_tenant')::VARCHAR);

DROP POLICY IF EXISTS tenant_isolation_kyc_records ON kyc_records;
CREATE POLICY tenant_isolation_kyc_records ON kyc_records
    USING (tenant_id = current_setting('app.current_tenant')::VARCHAR);

DROP POLICY IF EXISTS tenant_isolation_aml_cases ON aml_cases;
CREATE POLICY tenant_isolation_aml_cases ON aml_cases
    USING (tenant_id = current_setting('app.current_tenant')::VARCHAR);

DROP POLICY IF EXISTS tenant_isolation_audit_log ON audit_log;
CREATE POLICY tenant_isolation_audit_log ON audit_log
    USING (tenant_id = current_setting('app.current_tenant')::VARCHAR);

DROP POLICY IF EXISTS tenant_isolation_recon_batches ON recon_batches;
CREATE POLICY tenant_isolation_recon_batches ON recon_batches
    USING (tenant_id = current_setting('app.current_tenant')::VARCHAR);

DROP POLICY IF EXISTS tenant_isolation_aml_rule_versions ON aml_rule_versions;
CREATE POLICY tenant_isolation_aml_rule_versions ON aml_rule_versions
    USING (tenant_id = current_setting('app.current_tenant')::VARCHAR);

DROP POLICY IF EXISTS tenant_isolation_risk_score_history ON risk_score_history;
CREATE POLICY tenant_isolation_risk_score_history ON risk_score_history
    USING (tenant_id = current_setting('app.current_tenant')::VARCHAR);

DROP POLICY IF EXISTS tenant_isolation_case_notes ON case_notes;
CREATE POLICY tenant_isolation_case_notes ON case_notes
    USING (tenant_id = current_setting('app.current_tenant')::VARCHAR);

DROP POLICY IF EXISTS tenant_isolation_compliance_transactions ON compliance_transactions;
CREATE POLICY tenant_isolation_compliance_transactions ON compliance_transactions
    USING (tenant_id = current_setting('app.current_tenant')::VARCHAR);

DROP POLICY IF EXISTS smart_accounts_tenant_isolation ON smart_accounts;
CREATE POLICY smart_accounts_tenant_isolation ON smart_accounts
    FOR ALL
    USING (tenant_id = current_setting('app.current_tenant', true))
    WITH CHECK (tenant_id = current_setting('app.current_tenant', true));

DROP POLICY IF EXISTS bank_confirmations_tenant_isolation ON bank_confirmations;
CREATE POLICY bank_confirmations_tenant_isolation ON bank_confirmations
    USING (tenant_id = current_setting('app.current_tenant', true))
    WITH CHECK (tenant_id = current_setting('app.current_tenant', true));

DROP POLICY IF EXISTS bank_webhook_secrets_tenant_isolation ON bank_webhook_secrets;
CREATE POLICY bank_webhook_secrets_tenant_isolation ON bank_webhook_secrets
    USING (tenant_id = current_setting('app.current_tenant', true))
    WITH CHECK (tenant_id = current_setting('app.current_tenant', true));
