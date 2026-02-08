-- Down migration for 002_seed_data.sql
-- Removes seed data inserted by 002_seed_data.sql
-- SAFETY: Only run in non-production environments

DO $$
BEGIN
    -- Guard: Do NOT run in production
    IF current_setting('app.environment', true) = 'production' THEN
        RAISE EXCEPTION 'Refusing to delete seed data in production environment';
    END IF;

    -- Delete in reverse dependency order

    -- AML Cases
    DELETE FROM aml_cases WHERE id IN ('aml_case_1', 'aml_case_2');

    -- Intents (pending)
    DELETE FROM intents WHERE id IN ('intent_pending_1', 'intent_pending_2');

    -- Account balances for seed data
    DELETE FROM account_balances WHERE tenant_id IN ('tenant_a_123', 'tenant_b_456', 'tenant_c_789');

    -- Ledger entries for seed intents
    DELETE FROM ledger_entries WHERE intent_id IN ('intent_payin_001', 'intent_payout_001');

    -- Intents
    DELETE FROM intents WHERE id IN ('intent_payin_001', 'intent_payout_001');

    -- Virtual accounts
    DELETE FROM virtual_accounts WHERE id IN ('va_a_1', 'va_a_2', 'va_b_1');

    -- Users
    DELETE FROM users WHERE tenant_id IN ('tenant_a_123', 'tenant_b_456', 'tenant_c_789');

    -- Rails adapters
    DELETE FROM rails_adapters WHERE id IN ('rails_a_bank', 'rails_a_crypto', 'rails_b_psp');

    -- Tenants
    DELETE FROM tenants WHERE id IN ('tenant_a_123', 'tenant_b_456', 'tenant_c_789');
END $$;
