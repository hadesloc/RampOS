-- ============================================================================
-- SECURITY FIX: Fail-closed RLS policies + FORCE RLS on tenant-scoped tables
-- ============================================================================

-- Enforce RLS even for table owners
ALTER TABLE users FORCE ROW LEVEL SECURITY;
ALTER TABLE intents FORCE ROW LEVEL SECURITY;
ALTER TABLE ledger_entries FORCE ROW LEVEL SECURITY;
ALTER TABLE account_balances FORCE ROW LEVEL SECURITY;
ALTER TABLE webhook_events FORCE ROW LEVEL SECURITY;
ALTER TABLE rails_adapters FORCE ROW LEVEL SECURITY;
ALTER TABLE virtual_accounts FORCE ROW LEVEL SECURITY;
ALTER TABLE kyc_records FORCE ROW LEVEL SECURITY;
ALTER TABLE aml_cases FORCE ROW LEVEL SECURITY;
ALTER TABLE audit_log FORCE ROW LEVEL SECURITY;
ALTER TABLE recon_batches FORCE ROW LEVEL SECURITY;
ALTER TABLE aml_rule_versions FORCE ROW LEVEL SECURITY;
ALTER TABLE risk_score_history FORCE ROW LEVEL SECURITY;
ALTER TABLE case_notes FORCE ROW LEVEL SECURITY;
ALTER TABLE compliance_transactions FORCE ROW LEVEL SECURITY;
ALTER TABLE smart_accounts FORCE ROW LEVEL SECURITY;
ALTER TABLE bank_confirmations FORCE ROW LEVEL SECURITY;
ALTER TABLE bank_webhook_secrets FORCE ROW LEVEL SECURITY;

-- Fail-closed tenant isolation policies
DROP POLICY IF EXISTS tenant_isolation_users ON users;
CREATE POLICY tenant_isolation_users ON users
  FOR ALL
  USING (
    current_setting('app.current_tenant', true) IS NOT NULL
    AND tenant_id = current_setting('app.current_tenant', true)::VARCHAR
  );

DROP POLICY IF EXISTS tenant_isolation_intents ON intents;
CREATE POLICY tenant_isolation_intents ON intents
  FOR ALL
  USING (
    current_setting('app.current_tenant', true) IS NOT NULL
    AND tenant_id = current_setting('app.current_tenant', true)::VARCHAR
  );

DROP POLICY IF EXISTS tenant_isolation_ledger_entries ON ledger_entries;
CREATE POLICY tenant_isolation_ledger_entries ON ledger_entries
  FOR ALL
  USING (
    current_setting('app.current_tenant', true) IS NOT NULL
    AND tenant_id = current_setting('app.current_tenant', true)::VARCHAR
  );

DROP POLICY IF EXISTS tenant_isolation_account_balances ON account_balances;
CREATE POLICY tenant_isolation_account_balances ON account_balances
  FOR ALL
  USING (
    current_setting('app.current_tenant', true) IS NOT NULL
    AND tenant_id = current_setting('app.current_tenant', true)::VARCHAR
  );

DROP POLICY IF EXISTS tenant_isolation_webhook_events ON webhook_events;
CREATE POLICY tenant_isolation_webhook_events ON webhook_events
  FOR ALL
  USING (
    current_setting('app.current_tenant', true) IS NOT NULL
    AND tenant_id = current_setting('app.current_tenant', true)::VARCHAR
  );

DROP POLICY IF EXISTS tenant_isolation_rails_adapters ON rails_adapters;
CREATE POLICY tenant_isolation_rails_adapters ON rails_adapters
  FOR ALL
  USING (
    current_setting('app.current_tenant', true) IS NOT NULL
    AND tenant_id = current_setting('app.current_tenant', true)::VARCHAR
  );

DROP POLICY IF EXISTS tenant_isolation_virtual_accounts ON virtual_accounts;
CREATE POLICY tenant_isolation_virtual_accounts ON virtual_accounts
  FOR ALL
  USING (
    current_setting('app.current_tenant', true) IS NOT NULL
    AND tenant_id = current_setting('app.current_tenant', true)::VARCHAR
  );

DROP POLICY IF EXISTS tenant_isolation_kyc_records ON kyc_records;
CREATE POLICY tenant_isolation_kyc_records ON kyc_records
  FOR ALL
  USING (
    current_setting('app.current_tenant', true) IS NOT NULL
    AND tenant_id = current_setting('app.current_tenant', true)::VARCHAR
  );

DROP POLICY IF EXISTS tenant_isolation_aml_cases ON aml_cases;
CREATE POLICY tenant_isolation_aml_cases ON aml_cases
  FOR ALL
  USING (
    current_setting('app.current_tenant', true) IS NOT NULL
    AND tenant_id = current_setting('app.current_tenant', true)::VARCHAR
  );

DROP POLICY IF EXISTS tenant_isolation_audit_log ON audit_log;
CREATE POLICY tenant_isolation_audit_log ON audit_log
  FOR ALL
  USING (
    current_setting('app.current_tenant', true) IS NOT NULL
    AND tenant_id = current_setting('app.current_tenant', true)::VARCHAR
  );

DROP POLICY IF EXISTS tenant_isolation_recon_batches ON recon_batches;
CREATE POLICY tenant_isolation_recon_batches ON recon_batches
  FOR ALL
  USING (
    current_setting('app.current_tenant', true) IS NOT NULL
    AND tenant_id = current_setting('app.current_tenant', true)::VARCHAR
  );

DROP POLICY IF EXISTS tenant_isolation_aml_rule_versions ON aml_rule_versions;
CREATE POLICY tenant_isolation_aml_rule_versions ON aml_rule_versions
  FOR ALL
  USING (
    current_setting('app.current_tenant', true) IS NOT NULL
    AND tenant_id = current_setting('app.current_tenant', true)::VARCHAR
  );

DROP POLICY IF EXISTS tenant_isolation_risk_score_history ON risk_score_history;
CREATE POLICY tenant_isolation_risk_score_history ON risk_score_history
  FOR ALL
  USING (
    current_setting('app.current_tenant', true) IS NOT NULL
    AND tenant_id = current_setting('app.current_tenant', true)::VARCHAR
  );

DROP POLICY IF EXISTS tenant_isolation_case_notes ON case_notes;
CREATE POLICY tenant_isolation_case_notes ON case_notes
  FOR ALL
  USING (
    current_setting('app.current_tenant', true) IS NOT NULL
    AND tenant_id = current_setting('app.current_tenant', true)::VARCHAR
  );

DROP POLICY IF EXISTS tenant_isolation_compliance_transactions ON compliance_transactions;
CREATE POLICY tenant_isolation_compliance_transactions ON compliance_transactions
  FOR ALL
  USING (
    current_setting('app.current_tenant', true) IS NOT NULL
    AND tenant_id = current_setting('app.current_tenant', true)::VARCHAR
  );

-- Smart Accounts
DROP POLICY IF EXISTS smart_accounts_tenant_isolation ON smart_accounts;
CREATE POLICY smart_accounts_tenant_isolation ON smart_accounts
    FOR ALL
    USING (
        current_setting('app.current_tenant', true) IS NOT NULL
        AND tenant_id = current_setting('app.current_tenant', true)::VARCHAR
    );

-- Bank Confirmations
DROP POLICY IF EXISTS bank_confirmations_tenant_isolation ON bank_confirmations;
CREATE POLICY bank_confirmations_tenant_isolation ON bank_confirmations
    FOR ALL
    USING (
        current_setting('app.current_tenant', true) IS NOT NULL
        AND tenant_id = current_setting('app.current_tenant', true)::VARCHAR
    );

-- Bank Webhook Secrets
DROP POLICY IF EXISTS bank_webhook_secrets_tenant_isolation ON bank_webhook_secrets;
CREATE POLICY bank_webhook_secrets_tenant_isolation ON bank_webhook_secrets
    FOR ALL
    USING (
        current_setting('app.current_tenant', true) IS NOT NULL
        AND tenant_id = current_setting('app.current_tenant', true)::VARCHAR
    );

-- Create default deny policies for security hardening
-- NOTE: The explicit policies above handle tenant isolation.
-- If RLS is enabled and no policy matches (e.g. if app.current_tenant is not set),
-- PostgreSQL denies access by default.
-- However, we adding an explicit deny policy for 'public' role if needed,
-- but 'FOR ALL' policies above combined with 'FORCE ROW LEVEL SECURITY' covers it.
-- With FORCE ROW LEVEL SECURITY, superusers/owners are also subject to RLS,
-- but typically we want system processes to bypass.
-- System processes should use a user with BYPASSRLS attribute (like `rampos_system` created in 008).
