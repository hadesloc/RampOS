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

-- Fail-closed tenant isolation policies
DROP POLICY IF EXISTS tenant_isolation_users ON users;
CREATE POLICY tenant_isolation_users ON users
  USING (
    current_setting('app.current_tenant', true) IS NOT NULL
    AND tenant_id = current_setting('app.current_tenant', true)::VARCHAR
  );

DROP POLICY IF EXISTS tenant_isolation_intents ON intents;
CREATE POLICY tenant_isolation_intents ON intents
  USING (
    current_setting('app.current_tenant', true) IS NOT NULL
    AND tenant_id = current_setting('app.current_tenant', true)::VARCHAR
  );

DROP POLICY IF EXISTS tenant_isolation_ledger_entries ON ledger_entries;
CREATE POLICY tenant_isolation_ledger_entries ON ledger_entries
  USING (
    current_setting('app.current_tenant', true) IS NOT NULL
    AND tenant_id = current_setting('app.current_tenant', true)::VARCHAR
  );

DROP POLICY IF EXISTS tenant_isolation_account_balances ON account_balances;
CREATE POLICY tenant_isolation_account_balances ON account_balances
  USING (
    current_setting('app.current_tenant', true) IS NOT NULL
    AND tenant_id = current_setting('app.current_tenant', true)::VARCHAR
  );

DROP POLICY IF EXISTS tenant_isolation_webhook_events ON webhook_events;
CREATE POLICY tenant_isolation_webhook_events ON webhook_events
  USING (
    current_setting('app.current_tenant', true) IS NOT NULL
    AND tenant_id = current_setting('app.current_tenant', true)::VARCHAR
  );

DROP POLICY IF EXISTS tenant_isolation_rails_adapters ON rails_adapters;
CREATE POLICY tenant_isolation_rails_adapters ON rails_adapters
  USING (
    current_setting('app.current_tenant', true) IS NOT NULL
    AND tenant_id = current_setting('app.current_tenant', true)::VARCHAR
  );

DROP POLICY IF EXISTS tenant_isolation_virtual_accounts ON virtual_accounts;
CREATE POLICY tenant_isolation_virtual_accounts ON virtual_accounts
  USING (
    current_setting('app.current_tenant', true) IS NOT NULL
    AND tenant_id = current_setting('app.current_tenant', true)::VARCHAR
  );

DROP POLICY IF EXISTS tenant_isolation_kyc_records ON kyc_records;
CREATE POLICY tenant_isolation_kyc_records ON kyc_records
  USING (
    current_setting('app.current_tenant', true) IS NOT NULL
    AND tenant_id = current_setting('app.current_tenant', true)::VARCHAR
  );

DROP POLICY IF EXISTS tenant_isolation_aml_cases ON aml_cases;
CREATE POLICY tenant_isolation_aml_cases ON aml_cases
  USING (
    current_setting('app.current_tenant', true) IS NOT NULL
    AND tenant_id = current_setting('app.current_tenant', true)::VARCHAR
  );

DROP POLICY IF EXISTS tenant_isolation_audit_log ON audit_log;
CREATE POLICY tenant_isolation_audit_log ON audit_log
  USING (
    current_setting('app.current_tenant', true) IS NOT NULL
    AND tenant_id = current_setting('app.current_tenant', true)::VARCHAR
  );

DROP POLICY IF EXISTS tenant_isolation_recon_batches ON recon_batches;
CREATE POLICY tenant_isolation_recon_batches ON recon_batches
  USING (
    current_setting('app.current_tenant', true) IS NOT NULL
    AND tenant_id = current_setting('app.current_tenant', true)::VARCHAR
  );

DROP POLICY IF EXISTS tenant_isolation_aml_rule_versions ON aml_rule_versions;
CREATE POLICY tenant_isolation_aml_rule_versions ON aml_rule_versions
  USING (
    current_setting('app.current_tenant', true) IS NOT NULL
    AND tenant_id = current_setting('app.current_tenant', true)::VARCHAR
  );

DROP POLICY IF EXISTS tenant_isolation_risk_score_history ON risk_score_history;
CREATE POLICY tenant_isolation_risk_score_history ON risk_score_history
  USING (
    current_setting('app.current_tenant', true) IS NOT NULL
    AND tenant_id = current_setting('app.current_tenant', true)::VARCHAR
  );

DROP POLICY IF EXISTS tenant_isolation_case_notes ON case_notes;
CREATE POLICY tenant_isolation_case_notes ON case_notes
  USING (
    current_setting('app.current_tenant', true) IS NOT NULL
    AND tenant_id = current_setting('app.current_tenant', true)::VARCHAR
  );

DROP POLICY IF EXISTS tenant_isolation_compliance_transactions ON compliance_transactions;
CREATE POLICY tenant_isolation_compliance_transactions ON compliance_transactions
  USING (
    current_setting('app.current_tenant', true) IS NOT NULL
    AND tenant_id = current_setting('app.current_tenant', true)::VARCHAR
  );
