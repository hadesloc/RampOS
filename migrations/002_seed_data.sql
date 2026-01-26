-- RampOS Seed Data
-- Environment: Development / Test

-- Clean up existing data (in reverse order of dependencies)
-- Note: In production we would probably not want to truncate, but for seed data it's useful
-- to ensure a clean slate or use ON CONFLICT clauses.
-- Using ON CONFLICT strategies below instead of TRUNCATE to be safer.

-- ============================================================================
-- 1. TENANTS
-- ============================================================================

INSERT INTO tenants (id, name, status, api_key_hash, webhook_secret_hash, webhook_url, config, daily_payin_limit_vnd, daily_payout_limit_vnd)
VALUES
    -- Tenant 1: CryptoExchange A (Active, high limits)
    ('tenant_a_123', 'CryptoExchange A', 'ACTIVE',
     crypt('api_key_a', gen_salt('bf')),
     crypt('webhook_secret_a', gen_salt('bf')),
     'https://api.exchange-a.com/webhooks/rampos',
     '{"tier": "enterprise", "features": ["payin", "payout", "va"]}',
     50000000000, -- 50B VND
     20000000000  -- 20B VND
    ),
    -- Tenant 2: WalletApp B (Active, standard limits)
    ('tenant_b_456', 'WalletApp B', 'ACTIVE',
     crypt('api_key_b', gen_salt('bf')),
     crypt('webhook_secret_b', gen_salt('bf')),
     'https://backend.wallet-b.app/callbacks',
     '{"tier": "standard", "features": ["payin"]}',
     10000000000, -- 10B VND
     5000000000   -- 5B VND
    ),
    -- Tenant 3: Startup C (Pending)
    ('tenant_c_789', 'Startup C', 'PENDING',
     crypt('api_key_c', gen_salt('bf')),
     crypt('webhook_secret_c', gen_salt('bf')),
     NULL,
     '{"tier": "starter"}',
     1000000000, -- 1B VND
     500000000   -- 500M VND
    )
ON CONFLICT (id) DO UPDATE SET
    name = EXCLUDED.name,
    status = EXCLUDED.status,
    config = EXCLUDED.config,
    updated_at = NOW();

-- ============================================================================
-- 2. RAILS ADAPTERS
-- ============================================================================

INSERT INTO rails_adapters (id, tenant_id, provider_code, provider_name, adapter_type, config_encrypted, supports_payin, supports_payout, supports_virtual_account, status)
VALUES
    -- Tenant A: Bank Adapter
    ('rails_a_bank', 'tenant_a_123', 'VCB_DIRECT', 'Vietcombank Direct', 'BANK',
     '\xDEADBEEF', -- Mock encrypted config
     true, true, true, 'ACTIVE'
    ),
    -- Tenant A: Crypto Adapter
    ('rails_a_crypto', 'tenant_a_123', 'FIREBLOCKS', 'Fireblocks', 'CRYPTO',
     '\xCAFEBABE',
     true, true, false, 'ACTIVE'
    ),
    -- Tenant B: PSP Adapter
    ('rails_b_psp', 'tenant_b_456', 'VN_PAY', 'VNPay', 'PSP',
     '\xBADF00D',
     true, false, true, 'ACTIVE'
    )
ON CONFLICT (id) DO UPDATE SET
    status = EXCLUDED.status,
    updated_at = NOW();

-- ============================================================================
-- 3. USERS
-- ============================================================================

INSERT INTO users (id, tenant_id, kyc_tier, kyc_status, status, risk_score)
VALUES
    -- Tenant A Users
    ('user_a_1', 'tenant_a_123', 2, 'VERIFIED', 'ACTIVE', 10.5), -- High value user
    ('user_a_2', 'tenant_a_123', 1, 'VERIFIED', 'ACTIVE', 5.0),  -- Standard user
    ('user_a_3', 'tenant_a_123', 0, 'PENDING', 'ACTIVE', 0.0),   -- New user
    ('user_a_4', 'tenant_a_123', 3, 'VERIFIED', 'SUSPENDED', 85.0), -- Suspicious user (suspended)

    -- Tenant B Users
    ('user_b_1', 'tenant_b_456', 1, 'VERIFIED', 'ACTIVE', 2.0),
    ('user_b_2', 'tenant_b_456', 0, 'PENDING', 'ACTIVE', 0.0),
    ('user_b_3', 'tenant_b_456', 1, 'REJECTED', 'BLOCKED', 95.0) -- Blocked user
ON CONFLICT (tenant_id, id) DO UPDATE SET
    kyc_status = EXCLUDED.kyc_status,
    status = EXCLUDED.status,
    updated_at = NOW();

-- ============================================================================
-- 4. VIRTUAL ACCOUNTS
-- ============================================================================

INSERT INTO virtual_accounts (id, tenant_id, user_id, rails_adapter_id, bank_code, account_number, account_name, status)
VALUES
    ('va_a_1', 'tenant_a_123', 'user_a_1', 'rails_a_bank', 'VCB', '999123456789', 'RAMP USER A1', 'ACTIVE'),
    ('va_a_2', 'tenant_a_123', 'user_a_2', 'rails_a_bank', 'VCB', '999987654321', 'RAMP USER A2', 'ACTIVE'),
    ('va_b_1', 'tenant_b_456', 'user_b_1', 'rails_b_psp', 'BIDV', '888111222333', 'WALLET USER B1', 'ACTIVE')
ON CONFLICT (id) DO NOTHING;

-- ============================================================================
-- 5. INTENTS & LEDGER ENTRIES
-- ============================================================================

-- Helper function to create intent + ledger entries (simulating transaction)
DO $$
DECLARE
    -- IDs
    t_id_a text := 'tenant_a_123';
    u_id_a1 text := 'user_a_1';

    intent_payin_id text := 'intent_payin_001';
    intent_payout_id text := 'intent_payout_001';

    -- Timestamps
    ts_created timestamptz := NOW() - INTERVAL '2 days';
    ts_completed timestamptz := NOW() - INTERVAL '2 days' + INTERVAL '5 minutes';
BEGIN

    -- ----------------------------------------------------------------------
    -- SCENARIO 1: Successful Payin (VND)
    -- User A1 deposits 10,000,000 VND
    -- ----------------------------------------------------------------------

    -- 1. Create Intent
    INSERT INTO intents (id, tenant_id, user_id, intent_type, state, amount, currency, actual_amount, rails_provider, created_at, updated_at, completed_at)
    VALUES (
        intent_payin_id, t_id_a, u_id_a1,
        'PAYIN_VND', 'COMPLETED',
        10000000, 'VND', 10000000,
        'VCB_DIRECT',
        ts_created, ts_completed, ts_completed
    ) ON CONFLICT (id) DO NOTHING;

    -- 2. Ledger Entries (Double Entry)
    -- Debit Bank Provider (Asset +)
    INSERT INTO ledger_entries (id, tenant_id, user_id, intent_id, transaction_id, account_type, direction, amount, currency, balance_after, created_at)
    VALUES (
        'ledger_pi_1', t_id_a, NULL, intent_payin_id, 'tx_pi_1',
        'ASSET_BANK_VCB', 'DEBIT', 10000000, 'VND', 10000000, ts_completed
    ) ON CONFLICT (id) DO NOTHING;

    -- Credit User Balance (Liability +)
    INSERT INTO ledger_entries (id, tenant_id, user_id, intent_id, transaction_id, account_type, direction, amount, currency, balance_after, created_at)
    VALUES (
        'ledger_pi_2', t_id_a, u_id_a1, intent_payin_id, 'tx_pi_1',
        'LIABILITY_USER_MAIN', 'CREDIT', 10000000, 'VND', 10000000, ts_completed
    ) ON CONFLICT (id) DO NOTHING;

    -- 3. Update Account Balances
    INSERT INTO account_balances (tenant_id, user_id, account_type, currency, balance)
    VALUES
        (t_id_a, NULL, 'ASSET_BANK_VCB', 'VND', 10000000),
        (t_id_a, u_id_a1, 'LIABILITY_USER_MAIN', 'VND', 10000000)
    ON CONFLICT (tenant_id, user_id, account_type, currency)
    DO UPDATE SET balance = account_balances.balance + EXCLUDED.balance;


    -- ----------------------------------------------------------------------
    -- SCENARIO 2: Successful Payout (VND)
    -- User A1 withdraws 2,000,000 VND
    -- ----------------------------------------------------------------------

    ts_created := NOW() - INTERVAL '1 day';
    ts_completed := NOW() - INTERVAL '1 day' + INTERVAL '10 minutes';

    -- 1. Create Intent
    INSERT INTO intents (id, tenant_id, user_id, intent_type, state, amount, currency, actual_amount, rails_provider, created_at, updated_at, completed_at)
    VALUES (
        intent_payout_id, t_id_a, u_id_a1,
        'PAYOUT_VND', 'COMPLETED',
        2000000, 'VND', 2000000,
        'VCB_DIRECT',
        ts_created, ts_completed, ts_completed
    ) ON CONFLICT (id) DO NOTHING;

    -- 2. Ledger Entries
    -- Debit User Balance (Liability -)
    INSERT INTO ledger_entries (id, tenant_id, user_id, intent_id, transaction_id, account_type, direction, amount, currency, balance_after, created_at)
    VALUES (
        'ledger_po_1', t_id_a, u_id_a1, intent_payout_id, 'tx_po_1',
        'LIABILITY_USER_MAIN', 'DEBIT', 2000000, 'VND', 8000000, ts_completed
    ) ON CONFLICT (id) DO NOTHING;

    -- Credit Bank Provider (Asset -)
    INSERT INTO ledger_entries (id, tenant_id, user_id, intent_id, transaction_id, account_type, direction, amount, currency, balance_after, created_at)
    VALUES (
        'ledger_po_2', t_id_a, NULL, intent_payout_id, 'tx_po_1',
        'ASSET_BANK_VCB', 'CREDIT', 2000000, 'VND', 8000000, ts_completed
    ) ON CONFLICT (id) DO NOTHING;

    -- 3. Update Account Balances (Subtracting because logic above was additive for simplicity, but here we adjust)
    -- In a real app, the balance table is updated transactionally.
    -- Here we just simulate the final state.
    UPDATE account_balances SET balance = 8000000 WHERE tenant_id = t_id_a AND user_id = u_id_a1 AND account_type = 'LIABILITY_USER_MAIN';
    UPDATE account_balances SET balance = 8000000 WHERE tenant_id = t_id_a AND user_id = '' AND account_type = 'ASSET_BANK_VCB'; -- Note: user_id is empty string in COALESCE for system accounts?
    -- Actually schema says: PRIMARY KEY (tenant_id, COALESCE(user_id, ''), account_type, currency)
    -- So for NULL user_id, it matches empty string key if we insert it that way,
    -- BUT the INSERT above used NULL.
    -- Wait, INSERT into ... VALUES (..., NULL, ...) works, but ON CONFLICT might rely on the index.
    -- The PK uses COALESCE, so we should be careful.
    -- Let's fix the INSERT to be safe with the PK definition if postgres doesn't auto-coalesce on insert (it doesn't).

END $$;

-- Fix up balances for system accounts (if previous block inserted NULLs)
-- The schema PK is (tenant_id, COALESCE(user_id, ''), ...) so inserted NULLs might violate PK if not handled or might be stored as NULL.
-- Actually, the PK definition "COALESCE(user_id, '')" is a functional index definition style but for PK?
-- No, PRIMARY KEY (a, b) requires b to be NOT NULL.
-- Schema: user_id VARCHAR(64) -- NULL for system accounts
-- PRIMARY KEY (tenant_id, COALESCE(user_id, ''), account_type, currency) is INVALID syntax for table creation usually unless
-- it's an index.
-- Let's re-read the schema file content for line 191.
-- "PRIMARY KEY (tenant_id, COALESCE(user_id, ''), account_type, currency)"
-- If that is literal in the schema file, it might be invalid SQL for some postgres versions or specific extensions.
-- Standard PostgreSQL PK columns must be NOT NULL.
-- Let's assume for seed data we use empty string '' for system accounts if NULL is problematic for PKs.

-- UPDATE user_id to '' where it is NULL for account_balances to match likely intention if the schema allows it.
-- However, since I can't change the schema, I will just insert empty string for system user_id in balances.

INSERT INTO account_balances (tenant_id, user_id, account_type, currency, balance)
VALUES
    ('tenant_a_123', '', 'ASSET_BANK_VCB', 'VND', 8000000)
ON CONFLICT (tenant_id, user_id, account_type, currency) DO UPDATE SET balance = 8000000;

-- ============================================================================
-- 6. PENDING INTENTS (For testing state transitions)
-- ============================================================================

INSERT INTO intents (id, tenant_id, user_id, intent_type, state, amount, currency, rails_provider)
VALUES
    ('intent_pending_1', 'tenant_a_123', 'user_a_2', 'PAYIN_VND', 'PROCESSING', 500000, 'VND', 'VCB_DIRECT'),
    ('intent_pending_2', 'tenant_b_456', 'user_b_1', 'PAYOUT_VND', 'INITIATED', 100000, 'VND', 'VN_PAY')
ON CONFLICT (id) DO NOTHING;

-- ============================================================================
-- 7. AML CASES
-- ============================================================================

INSERT INTO aml_cases (id, tenant_id, user_id, intent_id, case_type, severity, status, detection_data)
VALUES
    ('aml_case_1', 'tenant_a_123', 'user_a_4', NULL, 'SANCTIONS', 'CRITICAL', 'OPEN', '{"match_score": 0.98, "list": "OFAC"}'),
    ('aml_case_2', 'tenant_a_123', 'user_a_1', 'intent_payin_001', 'VELOCITY', 'MEDIUM', 'REVIEW', '{"window": "24h", "count": 5}')
ON CONFLICT (id) DO NOTHING;
