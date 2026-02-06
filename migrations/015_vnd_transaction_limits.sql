-- Migration: VND Transaction Limits Tracking
-- Task: T-7.4
-- Description: Add tables and functions to track and enforce VND transaction limits per KYC tier

-- ============================================================================
-- USER TRANSACTION LIMITS TABLE
-- Tracks daily/monthly usage and custom limit overrides per user
-- ============================================================================

CREATE TABLE IF NOT EXISTS user_transaction_limits (
    id SERIAL PRIMARY KEY,
    tenant_id VARCHAR(64) NOT NULL REFERENCES tenants(id),
    user_id VARCHAR(64) NOT NULL,

    -- Current tier-based limits (can be overridden per user)
    tier SMALLINT NOT NULL DEFAULT 0,

    -- Custom limit overrides (NULL means use tier defaults)
    custom_single_limit_vnd DECIMAL(20, 2),
    custom_daily_limit_vnd DECIMAL(20, 2),
    custom_monthly_limit_vnd DECIMAL(20, 2),

    -- Manual approval threshold override
    custom_manual_approval_threshold DECIMAL(20, 2),

    -- Reason for custom limits (e.g., "Premium customer", "Business account")
    custom_limit_reason TEXT,
    custom_limit_approved_by VARCHAR(64),
    custom_limit_approved_at TIMESTAMPTZ,

    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Unique per tenant-user
    CONSTRAINT user_limits_unique UNIQUE (tenant_id, user_id),

    -- Foreign key to users
    FOREIGN KEY (tenant_id, user_id) REFERENCES users(tenant_id, id)
);

CREATE INDEX idx_user_limits_tenant ON user_transaction_limits(tenant_id);
CREATE INDEX idx_user_limits_tier ON user_transaction_limits(tenant_id, tier);

-- Trigger for updated_at
CREATE TRIGGER trigger_user_limits_updated_at
    BEFORE UPDATE ON user_transaction_limits
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();

-- ============================================================================
-- LIMIT CONFIG TABLE
-- Stores tenant-level VND limit configuration
-- ============================================================================

CREATE TABLE IF NOT EXISTS vnd_limit_config (
    id SERIAL PRIMARY KEY,
    tenant_id VARCHAR(64) NOT NULL REFERENCES tenants(id) UNIQUE,

    -- Per-tier limits (JSON structure)
    tier_limits JSONB NOT NULL DEFAULT '{}'::jsonb,

    -- Configuration flags
    reset_at_vietnam_midnight BOOLEAN NOT NULL DEFAULT true,
    enforce_on_payin BOOLEAN NOT NULL DEFAULT true,
    enforce_on_payout BOOLEAN NOT NULL DEFAULT true,
    timezone VARCHAR(64) NOT NULL DEFAULT 'Asia/Ho_Chi_Minh',

    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_vnd_limit_config_tenant ON vnd_limit_config(tenant_id);

-- Trigger for updated_at
CREATE TRIGGER trigger_vnd_limit_config_updated_at
    BEFORE UPDATE ON vnd_limit_config
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();

-- ============================================================================
-- TRANSACTION LIMIT HISTORY TABLE
-- Tracks all transactions for limit calculation
-- ============================================================================

CREATE TABLE IF NOT EXISTS transaction_limit_history (
    id BIGSERIAL PRIMARY KEY,
    tenant_id VARCHAR(64) NOT NULL REFERENCES tenants(id),
    user_id VARCHAR(64) NOT NULL,
    intent_id VARCHAR(64) REFERENCES intents(id),

    -- Transaction details
    transaction_type VARCHAR(32) NOT NULL,
    amount_vnd DECIMAL(20, 2) NOT NULL,
    currency VARCHAR(16) NOT NULL DEFAULT 'VND',

    -- For limit calculation
    transaction_date DATE NOT NULL DEFAULT CURRENT_DATE,
    transaction_month VARCHAR(7) NOT NULL DEFAULT TO_CHAR(NOW(), 'YYYY-MM'),

    -- Timestamps (in Vietnam time for reset calculation)
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    vietnam_date DATE NOT NULL DEFAULT (NOW() AT TIME ZONE 'Asia/Ho_Chi_Minh')::DATE,
    vietnam_month VARCHAR(7) NOT NULL DEFAULT TO_CHAR(NOW() AT TIME ZONE 'Asia/Ho_Chi_Minh', 'YYYY-MM'),

    -- Foreign key to users
    FOREIGN KEY (tenant_id, user_id) REFERENCES users(tenant_id, id)
);

-- Indexes for efficient limit queries
CREATE INDEX idx_tx_limit_history_daily ON transaction_limit_history(tenant_id, user_id, vietnam_date);
CREATE INDEX idx_tx_limit_history_monthly ON transaction_limit_history(tenant_id, user_id, vietnam_month);
CREATE INDEX idx_tx_limit_history_intent ON transaction_limit_history(intent_id);
CREATE INDEX idx_tx_limit_history_type ON transaction_limit_history(tenant_id, transaction_type);

-- ============================================================================
-- FUNCTION: Get daily used amount for a user
-- ============================================================================

CREATE OR REPLACE FUNCTION get_user_daily_used_vnd(
    p_tenant_id VARCHAR(64),
    p_user_id VARCHAR(64),
    p_date DATE DEFAULT NULL
) RETURNS DECIMAL(20, 2) AS $$
DECLARE
    v_total DECIMAL(20, 2);
    v_date DATE;
BEGIN
    -- Use provided date or current Vietnam date
    v_date := COALESCE(p_date, (NOW() AT TIME ZONE 'Asia/Ho_Chi_Minh')::DATE);

    SELECT COALESCE(SUM(amount_vnd), 0)
    INTO v_total
    FROM transaction_limit_history
    WHERE tenant_id = p_tenant_id
      AND user_id = p_user_id
      AND vietnam_date = v_date;

    RETURN v_total;
END;
$$ LANGUAGE plpgsql STABLE;

-- ============================================================================
-- FUNCTION: Get monthly used amount for a user
-- ============================================================================

CREATE OR REPLACE FUNCTION get_user_monthly_used_vnd(
    p_tenant_id VARCHAR(64),
    p_user_id VARCHAR(64),
    p_month VARCHAR(7) DEFAULT NULL
) RETURNS DECIMAL(20, 2) AS $$
DECLARE
    v_total DECIMAL(20, 2);
    v_month VARCHAR(7);
BEGIN
    -- Use provided month or current Vietnam month
    v_month := COALESCE(p_month, TO_CHAR(NOW() AT TIME ZONE 'Asia/Ho_Chi_Minh', 'YYYY-MM'));

    SELECT COALESCE(SUM(amount_vnd), 0)
    INTO v_total
    FROM transaction_limit_history
    WHERE tenant_id = p_tenant_id
      AND user_id = p_user_id
      AND vietnam_month = v_month;

    RETURN v_total;
END;
$$ LANGUAGE plpgsql STABLE;

-- ============================================================================
-- FUNCTION: Check if transaction is within limits
-- Returns: 0 = OK, 1 = single limit exceeded, 2 = daily exceeded, 3 = monthly exceeded
-- ============================================================================

CREATE OR REPLACE FUNCTION check_vnd_transaction_limit(
    p_tenant_id VARCHAR(64),
    p_user_id VARCHAR(64),
    p_amount_vnd DECIMAL(20, 2)
) RETURNS JSONB AS $$
DECLARE
    v_user RECORD;
    v_limits RECORD;
    v_config RECORD;
    v_daily_used DECIMAL(20, 2);
    v_monthly_used DECIMAL(20, 2);
    v_single_limit DECIMAL(20, 2);
    v_daily_limit DECIMAL(20, 2);
    v_monthly_limit DECIMAL(20, 2);
    v_result JSONB;
BEGIN
    -- Get user info
    SELECT kyc_tier INTO v_user
    FROM users
    WHERE tenant_id = p_tenant_id AND id = p_user_id;

    IF NOT FOUND THEN
        RETURN jsonb_build_object(
            'approved', false,
            'error', 'USER_NOT_FOUND',
            'message', 'User not found'
        );
    END IF;

    -- Tier 0 cannot transact
    IF v_user.kyc_tier = 0 THEN
        RETURN jsonb_build_object(
            'approved', false,
            'error', 'TIER_NOT_ALLOWED',
            'message', 'Tier 0 users are not allowed to perform transactions'
        );
    END IF;

    -- Get custom limits or use defaults
    SELECT
        COALESCE(custom_single_limit_vnd,
            CASE v_user.kyc_tier
                WHEN 1 THEN 50000000  -- 50M
                WHEN 2 THEN 200000000 -- 200M
                WHEN 3 THEN 1000000000 -- 1B
                ELSE 0
            END),
        COALESCE(custom_daily_limit_vnd,
            CASE v_user.kyc_tier
                WHEN 1 THEN 100000000   -- 100M
                WHEN 2 THEN 500000000   -- 500M
                WHEN 3 THEN 9999999999999 -- Unlimited
                ELSE 0
            END),
        COALESCE(custom_monthly_limit_vnd,
            CASE v_user.kyc_tier
                WHEN 1 THEN 1000000000    -- 1B
                WHEN 2 THEN 5000000000    -- 5B
                WHEN 3 THEN 9999999999999 -- Unlimited
                ELSE 0
            END)
    INTO v_single_limit, v_daily_limit, v_monthly_limit
    FROM user_transaction_limits
    WHERE tenant_id = p_tenant_id AND user_id = p_user_id;

    -- Use defaults if no custom limits
    IF NOT FOUND THEN
        v_single_limit := CASE v_user.kyc_tier
            WHEN 1 THEN 50000000
            WHEN 2 THEN 200000000
            WHEN 3 THEN 1000000000
            ELSE 0
        END;
        v_daily_limit := CASE v_user.kyc_tier
            WHEN 1 THEN 100000000
            WHEN 2 THEN 500000000
            WHEN 3 THEN 9999999999999
            ELSE 0
        END;
        v_monthly_limit := CASE v_user.kyc_tier
            WHEN 1 THEN 1000000000
            WHEN 2 THEN 5000000000
            WHEN 3 THEN 9999999999999
            ELSE 0
        END;
    END IF;

    -- Check single transaction limit
    IF p_amount_vnd > v_single_limit THEN
        RETURN jsonb_build_object(
            'approved', false,
            'error', 'SINGLE_LIMIT_EXCEEDED',
            'message', format('Amount %s VND exceeds single transaction limit of %s VND', p_amount_vnd, v_single_limit),
            'limit', v_single_limit,
            'requested', p_amount_vnd
        );
    END IF;

    -- Get current usage
    v_daily_used := get_user_daily_used_vnd(p_tenant_id, p_user_id);
    v_monthly_used := get_user_monthly_used_vnd(p_tenant_id, p_user_id);

    -- Check daily limit
    IF (v_daily_used + p_amount_vnd) > v_daily_limit THEN
        RETURN jsonb_build_object(
            'approved', false,
            'error', 'DAILY_LIMIT_EXCEEDED',
            'message', format('Daily limit exceeded. Used: %s VND, Requested: %s VND, Limit: %s VND',
                v_daily_used, p_amount_vnd, v_daily_limit),
            'used', v_daily_used,
            'limit', v_daily_limit,
            'requested', p_amount_vnd
        );
    END IF;

    -- Check monthly limit
    IF (v_monthly_used + p_amount_vnd) > v_monthly_limit THEN
        RETURN jsonb_build_object(
            'approved', false,
            'error', 'MONTHLY_LIMIT_EXCEEDED',
            'message', format('Monthly limit exceeded. Used: %s VND, Requested: %s VND, Limit: %s VND',
                v_monthly_used, p_amount_vnd, v_monthly_limit),
            'used', v_monthly_used,
            'limit', v_monthly_limit,
            'requested', p_amount_vnd
        );
    END IF;

    -- All checks passed
    RETURN jsonb_build_object(
        'approved', true,
        'daily_remaining', v_daily_limit - v_daily_used - p_amount_vnd,
        'monthly_remaining', v_monthly_limit - v_monthly_used - p_amount_vnd,
        'daily_used', v_daily_used,
        'monthly_used', v_monthly_used,
        'daily_limit', v_daily_limit,
        'monthly_limit', v_monthly_limit
    );
END;
$$ LANGUAGE plpgsql STABLE;

-- ============================================================================
-- FUNCTION: Record a transaction for limit tracking
-- ============================================================================

CREATE OR REPLACE FUNCTION record_transaction_for_limits(
    p_tenant_id VARCHAR(64),
    p_user_id VARCHAR(64),
    p_intent_id VARCHAR(64),
    p_transaction_type VARCHAR(32),
    p_amount_vnd DECIMAL(20, 2)
) RETURNS BIGINT AS $$
DECLARE
    v_id BIGINT;
BEGIN
    INSERT INTO transaction_limit_history (
        tenant_id, user_id, intent_id, transaction_type, amount_vnd
    ) VALUES (
        p_tenant_id, p_user_id, p_intent_id, p_transaction_type, p_amount_vnd
    )
    RETURNING id INTO v_id;

    RETURN v_id;
END;
$$ LANGUAGE plpgsql;

-- ============================================================================
-- RLS POLICIES
-- ============================================================================

ALTER TABLE user_transaction_limits ENABLE ROW LEVEL SECURITY;
ALTER TABLE vnd_limit_config ENABLE ROW LEVEL SECURITY;
ALTER TABLE transaction_limit_history ENABLE ROW LEVEL SECURITY;

-- User transaction limits policies
CREATE POLICY user_limits_tenant_isolation ON user_transaction_limits
    FOR ALL
    USING (tenant_id = current_setting('app.current_tenant', true)::VARCHAR)
    WITH CHECK (tenant_id = current_setting('app.current_tenant', true)::VARCHAR);

-- VND limit config policies
CREATE POLICY vnd_config_tenant_isolation ON vnd_limit_config
    FOR ALL
    USING (tenant_id = current_setting('app.current_tenant', true)::VARCHAR)
    WITH CHECK (tenant_id = current_setting('app.current_tenant', true)::VARCHAR);

-- Transaction limit history policies
CREATE POLICY tx_history_tenant_isolation ON transaction_limit_history
    FOR ALL
    USING (tenant_id = current_setting('app.current_tenant', true)::VARCHAR)
    WITH CHECK (tenant_id = current_setting('app.current_tenant', true)::VARCHAR);

-- ============================================================================
-- DEFAULT DATA: Insert default tier limits for reference
-- ============================================================================

COMMENT ON TABLE user_transaction_limits IS 'Tracks user-specific transaction limit overrides and tier information';
COMMENT ON TABLE vnd_limit_config IS 'Tenant-level VND transaction limit configuration';
COMMENT ON TABLE transaction_limit_history IS 'Historical record of all transactions for limit calculation';
COMMENT ON FUNCTION get_user_daily_used_vnd IS 'Calculate total VND amount used by a user on a specific date';
COMMENT ON FUNCTION get_user_monthly_used_vnd IS 'Calculate total VND amount used by a user in a specific month';
COMMENT ON FUNCTION check_vnd_transaction_limit IS 'Check if a transaction is within VND limits for a user';
COMMENT ON FUNCTION record_transaction_for_limits IS 'Record a completed transaction for limit tracking';
