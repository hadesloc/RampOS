-- Migration 035: Sandbox presets for programmable replay environments
-- Stores sandbox-only preset definitions that can seed deterministic tenants
-- and reset them back to a known package version without modeling production
-- workflow state.

CREATE TABLE sandbox_presets (
    id TEXT PRIMARY KEY,                               -- "sandbox_preset_..." prefix
    preset_code TEXT NOT NULL UNIQUE,                  -- stable external code, e.g. BASELINE
    name TEXT NOT NULL,
    description TEXT,

    -- Preset contract
    seed_package_version TEXT NOT NULL,                -- fixture bundle version consumed by sandbox seeding
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,       -- bounded sandbox metadata only

    -- Reset contract
    reset_strategy TEXT NOT NULL DEFAULT 'RESET_TO_PRESET',
    reset_semantics JSONB NOT NULL DEFAULT '{}'::jsonb,

    -- Lifecycle
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT sandbox_presets_code_format CHECK (preset_code ~ '^[A-Z0-9_]+$'),
    CONSTRAINT sandbox_presets_reset_strategy_check CHECK (
        reset_strategy IN (
            'RESET_TO_PRESET',
            'RESET_SCENARIO_DATA',
            'RESET_RUNTIME_ARTIFACTS'
        )
    ),
    CONSTRAINT sandbox_presets_metadata_object CHECK (jsonb_typeof(metadata) = 'object'),
    CONSTRAINT sandbox_presets_reset_semantics_object CHECK (jsonb_typeof(reset_semantics) = 'object')
);

CREATE INDEX idx_sandbox_presets_active ON sandbox_presets(is_active, preset_code);
CREATE INDEX idx_sandbox_presets_metadata ON sandbox_presets USING GIN (metadata);

CREATE TRIGGER trigger_sandbox_presets_updated_at
    BEFORE UPDATE ON sandbox_presets
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();

CREATE TABLE sandbox_preset_scenarios (
    id BIGSERIAL PRIMARY KEY,
    preset_id TEXT NOT NULL REFERENCES sandbox_presets(id) ON DELETE CASCADE,
    scenario_code TEXT NOT NULL,
    sort_order SMALLINT NOT NULL DEFAULT 0,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT sandbox_preset_scenarios_code_format CHECK (scenario_code ~ '^[A-Z0-9_]+$'),
    CONSTRAINT sandbox_preset_scenarios_metadata_object CHECK (jsonb_typeof(metadata) = 'object'),
    CONSTRAINT sandbox_preset_scenarios_unique UNIQUE (preset_id, scenario_code)
);

CREATE INDEX idx_sandbox_preset_scenarios_lookup
    ON sandbox_preset_scenarios(preset_id, sort_order, scenario_code);

INSERT INTO sandbox_presets (
    id,
    preset_code,
    name,
    description,
    seed_package_version,
    metadata,
    reset_strategy,
    reset_semantics
)
VALUES
    (
        'sandbox_preset_baseline',
        'BASELINE',
        'Baseline Sandbox',
        'Balanced preset for common pay-in, payout, RFQ, and webhook replay coverage.',
        '2026-03-08',
        '{"category":"general","operator_surface":"admin","supports_replay":true}'::jsonb,
        'RESET_TO_PRESET',
        '{
            "drop_runtime_events": true,
            "drop_seeded_users": true,
            "drop_seeded_intents": true,
            "drop_seeded_balances": true,
            "preserve_admin_credentials": true
        }'::jsonb
    ),
    (
        'sandbox_preset_payin_failure',
        'PAYIN_FAILURE_DRILL',
        'Pay-in Failure Drill',
        'Preset for deterministic payment failure drills and post-failure replay export.',
        '2026-03-08',
        '{"category":"payin","operator_surface":"admin","supports_replay":true}'::jsonb,
        'RESET_SCENARIO_DATA',
        '{
            "drop_runtime_events": true,
            "drop_seeded_intents": true,
            "drop_seeded_webhooks": true,
            "preserve_seeded_users": true,
            "preserve_admin_credentials": true
        }'::jsonb
    ),
    (
        'sandbox_preset_liquidity_drill',
        'LIQUIDITY_DRILL',
        'Liquidity Drill',
        'Preset for RFQ, no-fill, and delayed-settlement liquidity exercises.',
        '2026-03-08',
        '{"category":"liquidity","operator_surface":"admin","supports_replay":true}'::jsonb,
        'RESET_RUNTIME_ARTIFACTS',
        '{
            "drop_runtime_events": true,
            "drop_rfq_artifacts": true,
            "drop_settlement_attempts": true,
            "preserve_seeded_users": true,
            "preserve_admin_credentials": true
        }'::jsonb
    )
ON CONFLICT (preset_code) DO NOTHING;

INSERT INTO sandbox_preset_scenarios (preset_id, scenario_code, sort_order, metadata)
VALUES
    (
        'sandbox_preset_baseline',
        'PAYIN_BASELINE',
        10,
        '{"flow":"payin","mode":"happy_path"}'::jsonb
    ),
    (
        'sandbox_preset_baseline',
        'OFFRAMP_BASELINE',
        20,
        '{"flow":"offramp","mode":"happy_path"}'::jsonb
    ),
    (
        'sandbox_preset_baseline',
        'WEBHOOK_RETRY_BASELINE',
        30,
        '{"flow":"webhook","mode":"retry"}'::jsonb
    ),
    (
        'sandbox_preset_payin_failure',
        'PAYIN_BANK_TIMEOUT',
        10,
        '{"flow":"payin","mode":"failure_drill"}'::jsonb
    ),
    (
        'sandbox_preset_payin_failure',
        'PAYIN_COMPLIANCE_REVIEW',
        20,
        '{"flow":"payin","mode":"manual_review"}'::jsonb
    ),
    (
        'sandbox_preset_liquidity_drill',
        'RFQ_BASELINE',
        10,
        '{"flow":"rfq","mode":"auction"}'::jsonb
    ),
    (
        'sandbox_preset_liquidity_drill',
        'LP_NO_FILL',
        20,
        '{"flow":"rfq","mode":"failure_drill"}'::jsonb
    ),
    (
        'sandbox_preset_liquidity_drill',
        'SETTLEMENT_DELAY',
        30,
        '{"flow":"settlement","mode":"recovery"}'::jsonb
    )
ON CONFLICT (preset_id, scenario_code) DO NOTHING;
