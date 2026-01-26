-- RampOS Extended Seed Data (Task 1.2.6)
-- Supplements 002_seed_data.sql with comprehensive test scenarios.
--
-- Includes:
-- 1. Additional Users (meeting "5+ per tenant" requirement)
-- 2. Trade Intents (simulating exchange activity)
-- 3. Failed/Expired/Cancelled Intents
-- 4. Webhook Events
-- 5. Additional Payins for liquidity

-- ============================================================================
-- 1. ADDITIONAL USERS
-- ============================================================================

INSERT INTO users (id, tenant_id, kyc_tier, kyc_status, status, risk_score)
VALUES
    -- Tenant A (CryptoExchange) - Existing: 4. Adding 4 more.
    ('user_a_5', 'tenant_a_123', 1, 'VERIFIED', 'ACTIVE', 12.0),
    ('user_a_6', 'tenant_a_123', 2, 'VERIFIED', 'ACTIVE', 3.5),
    ('user_a_7', 'tenant_a_123', 3, 'VERIFIED', 'ACTIVE', 0.0),
    ('user_a_8', 'tenant_a_123', 1, 'PENDING', 'ACTIVE', 0.0),

    -- Tenant B (WalletApp) - Existing: 3. Adding 3 more.
    ('user_b_4', 'tenant_b_456', 1, 'VERIFIED', 'ACTIVE', 1.0),
    ('user_b_5', 'tenant_b_456', 0, 'PENDING', 'ACTIVE', 0.0),
    ('user_b_6', 'tenant_b_456', 3, 'VERIFIED', 'ACTIVE', 0.5),

    -- Tenant C (Startup C) - Existing: 0. Adding 5.
    ('user_c_1', 'tenant_c_789', 3, 'VERIFIED', 'ACTIVE', 0.0), -- Admin
    ('user_c_2', 'tenant_c_789', 2, 'VERIFIED', 'ACTIVE', 5.0),
    ('user_c_3', 'tenant_c_789', 1, 'VERIFIED', 'ACTIVE', 15.0),
    ('user_c_4', 'tenant_c_789', 0, 'PENDING', 'ACTIVE', 0.0),
    ('user_c_5', 'tenant_c_789', 0, 'REJECTED', 'BLOCKED', 80.0)
ON CONFLICT (tenant_id, id) DO NOTHING;

-- ============================================================================
-- 2. LIQUIDITY & TRADES (Tenant A)
-- ============================================================================

DO $$
DECLARE
    t_id text := 'tenant_a_123';
    u_id text := 'user_a_5';

    intent_payin_id text := 'intent_payin_a5_1';
    intent_trade_id text := 'intent_trade_a5_1';

    ts_payin timestamptz := NOW() - INTERVAL '3 hours';
    ts_trade timestamptz := NOW() - INTERVAL '2 hours';
BEGIN
    -- ---------------------------------------------------------
    -- Step 1: Payin 50,000,000 VND for User A5
    -- ---------------------------------------------------------
    INSERT INTO intents (id, tenant_id, user_id, intent_type, state, amount, currency, actual_amount, rails_provider, created_at, updated_at, completed_at)
    VALUES (
        intent_payin_id, t_id, u_id,
        'PAYIN_VND', 'COMPLETED',
        50000000, 'VND', 50000000,
        'VCB_DIRECT',
        ts_payin, ts_payin, ts_payin
    ) ON CONFLICT (id) DO NOTHING;

    -- Ledger: Debit Bank (System)
    INSERT INTO ledger_entries (id, tenant_id, user_id, intent_id, transaction_id, account_type, direction, amount, currency, balance_after, created_at)
    VALUES (
        'ledger_pi_a5_1', t_id, NULL, intent_payin_id, 'tx_pi_a5_1',
        'ASSET_BANK_VCB', 'DEBIT', 50000000, 'VND', 50000000, ts_payin
    ) ON CONFLICT (id) DO NOTHING;

    -- Ledger: Credit User
    INSERT INTO ledger_entries (id, tenant_id, user_id, intent_id, transaction_id, account_type, direction, amount, currency, balance_after, created_at)
    VALUES (
        'ledger_pi_a5_2', t_id, u_id, intent_payin_id, 'tx_pi_a5_1',
        'LIABILITY_USER_MAIN', 'CREDIT', 50000000, 'VND', 50000000, ts_payin
    ) ON CONFLICT (id) DO NOTHING;

    -- Update Balances (Payin)
    INSERT INTO account_balances (tenant_id, user_id, account_type, currency, balance)
    VALUES
        (t_id, '', 'ASSET_BANK_VCB', 'VND', 50000000),
        (t_id, u_id, 'LIABILITY_USER_MAIN', 'VND', 50000000)
    ON CONFLICT (tenant_id, user_id, account_type, currency)
    DO UPDATE SET balance = account_balances.balance + EXCLUDED.balance;


    -- ---------------------------------------------------------
    -- Step 2: Trade 25,000,000 VND -> 1000 USDT
    -- ---------------------------------------------------------
    INSERT INTO intents (id, tenant_id, user_id, intent_type, state, amount, currency, actual_amount, metadata, created_at, updated_at, completed_at)
    VALUES (
        intent_trade_id, t_id, u_id,
        'TRADE_EXECUTED', 'COMPLETED',
        25000000, 'VND', 25000000,
        '{"buy_currency": "USDT", "buy_amount": 1000, "rate": 25000}',
        ts_trade, ts_trade, ts_trade
    ) ON CONFLICT (id) DO NOTHING;

    -- Ledger: Debit VND from User (Liability -)
    INSERT INTO ledger_entries (id, tenant_id, user_id, intent_id, transaction_id, account_type, direction, amount, currency, balance_after, created_at)
    VALUES (
        'ledger_tr_a5_1', t_id, u_id, intent_trade_id, 'tx_tr_a5_1',
        'LIABILITY_USER_MAIN', 'DEBIT', 25000000, 'VND', 25000000, ts_trade
    ) ON CONFLICT (id) DO NOTHING;

    -- Ledger: Credit USDT to User (Liability +)
    INSERT INTO ledger_entries (id, tenant_id, user_id, intent_id, transaction_id, account_type, direction, amount, currency, balance_after, created_at)
    VALUES (
        'ledger_tr_a5_2', t_id, u_id, intent_trade_id, 'tx_tr_a5_1',
        'LIABILITY_USER_MAIN', 'CREDIT', 1000, 'USDT', 1000, ts_trade
    ) ON CONFLICT (id) DO NOTHING;

    -- Update Balances (Trade)
    -- VND: Decrease
    UPDATE account_balances SET balance = balance - 25000000
    WHERE tenant_id = t_id AND user_id = u_id AND account_type = 'LIABILITY_USER_MAIN' AND currency = 'VND';

    -- USDT: Increase (Insert if not exists)
    INSERT INTO account_balances (tenant_id, user_id, account_type, currency, balance)
    VALUES (t_id, u_id, 'LIABILITY_USER_MAIN', 'USDT', 1000)
    ON CONFLICT (tenant_id, user_id, account_type, currency)
    DO UPDATE SET balance = account_balances.balance + 1000;

END $$;

-- ============================================================================
-- 3. EDGE CASE INTENTS
-- ============================================================================

-- Expired Payin (Tenant A)
INSERT INTO intents (id, tenant_id, user_id, intent_type, state, amount, currency, created_at, expires_at)
VALUES (
    'intent_exp_001', 'tenant_a_123', 'user_a_6',
    'PAYIN_VND', 'EXPIRED',
    200000, 'VND',
    NOW() - INTERVAL '3 days', NOW() - INTERVAL '2 days'
) ON CONFLICT (id) DO NOTHING;

-- Cancelled Payout (Tenant B)
INSERT INTO intents (id, tenant_id, user_id, intent_type, state, amount, currency, created_at)
VALUES (
    'intent_can_001', 'tenant_b_456', 'user_b_4',
    'PAYOUT_VND', 'CANCELLED',
    500000, 'VND',
    NOW() - INTERVAL '1 day'
) ON CONFLICT (id) DO NOTHING;

-- ============================================================================
-- 4. WEBHOOK CONFIGURATIONS & EVENTS
-- ============================================================================

-- Ensure webhook URLs
UPDATE tenants SET webhook_url = 'https://api.startup-c.com/webhooks' WHERE id = 'tenant_c_789';

-- Sample Events
INSERT INTO webhook_events (id, tenant_id, event_type, intent_id, payload, status, created_at, delivered_at, response_status)
VALUES
    -- Successful delivery
    ('wh_evt_1', 'tenant_a_123', 'intent.completed', 'intent_payin_a5_1',
     '{"id": "intent_payin_a5_1", "type": "PAYIN_VND", "status": "COMPLETED", "amount": 50000000}',
     'DELIVERED', NOW() - INTERVAL '3 hours', NOW() - INTERVAL '3 hours', 200),

    -- Failed delivery
    ('wh_evt_2', 'tenant_a_123', 'intent.failed', 'intent_exp_001',
     '{"id": "intent_exp_001", "type": "PAYIN_VND", "status": "EXPIRED"}',
     'FAILED', NOW() - INTERVAL '2 days', NULL, 500),

    -- Pending delivery
    ('wh_evt_3', 'tenant_b_456', 'intent.cancelled', 'intent_can_001',
     '{"id": "intent_can_001", "type": "PAYOUT_VND", "status": "CANCELLED"}',
     'PENDING', NOW() - INTERVAL '1 minute', NULL, NULL)
ON CONFLICT (id) DO NOTHING;
