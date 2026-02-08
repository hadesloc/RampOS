-- Down migration for 999_seed_data.sql
-- Removes extended seed data
-- SAFETY: Only run in non-production environments

DO $$
BEGIN
    -- Guard: Do NOT run in production
    IF current_setting('app.environment', true) = 'production' THEN
        RAISE EXCEPTION 'Refusing to delete seed data in production environment';
    END IF;

    -- Delete webhook events
    DELETE FROM webhook_events WHERE id IN ('wh_evt_1', 'wh_evt_2', 'wh_evt_3');

    -- Delete edge case intents
    DELETE FROM intents WHERE id IN ('intent_exp_001', 'intent_can_001');

    -- Delete trade/payin ledger entries and intents
    DELETE FROM ledger_entries WHERE intent_id IN ('intent_payin_a5_1', 'intent_trade_a5_1');
    DELETE FROM intents WHERE id IN ('intent_payin_a5_1', 'intent_trade_a5_1');

    -- Delete account balances for extended seed users
    DELETE FROM account_balances WHERE tenant_id = 'tenant_a_123' AND user_id = 'user_a_5';

    -- Delete extended seed users
    DELETE FROM users WHERE (tenant_id, id) IN (
        ('tenant_a_123', 'user_a_5'),
        ('tenant_a_123', 'user_a_6'),
        ('tenant_a_123', 'user_a_7'),
        ('tenant_a_123', 'user_a_8'),
        ('tenant_b_456', 'user_b_4'),
        ('tenant_b_456', 'user_b_5'),
        ('tenant_b_456', 'user_b_6'),
        ('tenant_c_789', 'user_c_1'),
        ('tenant_c_789', 'user_c_2'),
        ('tenant_c_789', 'user_c_3'),
        ('tenant_c_789', 'user_c_4'),
        ('tenant_c_789', 'user_c_5')
    );

    -- Revert webhook URL for tenant C
    UPDATE tenants SET webhook_url = NULL WHERE id = 'tenant_c_789';
END $$;
