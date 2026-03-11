-- Migration 036: LP reliability snapshots
-- Stores additive liquidity-provider reliability snapshots so ranking logic can
-- combine realized performance with best-price fallback in later waves.

CREATE TABLE lp_reliability_snapshots (
    id TEXT PRIMARY KEY,                                -- "lprs_..." prefix
    tenant_id TEXT NOT NULL REFERENCES tenants(id),
    lp_id TEXT NOT NULL,                                -- LP identifier from RFQ/bid lifecycle
    direction TEXT NOT NULL DEFAULT 'OFFRAMP',

    -- Windowing contract
    window_kind TEXT NOT NULL DEFAULT 'ROLLING_30D',
    window_started_at TIMESTAMPTZ NOT NULL,
    window_ended_at TIMESTAMPTZ NOT NULL,
    snapshot_version TEXT NOT NULL DEFAULT 'v1',

    -- Sample counts from realized outcomes
    quote_count INTEGER NOT NULL DEFAULT 0,
    fill_count INTEGER NOT NULL DEFAULT 0,
    reject_count INTEGER NOT NULL DEFAULT 0,
    settlement_count INTEGER NOT NULL DEFAULT 0,
    dispute_count INTEGER NOT NULL DEFAULT 0,

    -- Normalized factor fields (0.0 .. 1.0 unless otherwise noted)
    fill_rate NUMERIC(6,5) NOT NULL DEFAULT 0,
    reject_rate NUMERIC(6,5) NOT NULL DEFAULT 0,
    dispute_rate NUMERIC(6,5) NOT NULL DEFAULT 0,
    avg_slippage_bps NUMERIC(10,2) NOT NULL DEFAULT 0,
    p95_settlement_latency_seconds INTEGER NOT NULL DEFAULT 0,

    -- Reserved output for later ranking layers
    reliability_score NUMERIC(10,4),
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT lp_reliability_snapshots_direction_check CHECK (
        direction IN ('OFFRAMP', 'ONRAMP')
    ),
    CONSTRAINT lp_reliability_snapshots_window_kind_check CHECK (
        window_kind IN ('ROLLING_24H', 'ROLLING_7D', 'ROLLING_30D', 'CALENDAR_DAY')
    ),
    CONSTRAINT lp_reliability_snapshots_window_order CHECK (
        window_started_at <= window_ended_at
    ),
    CONSTRAINT lp_reliability_snapshots_quote_count_check CHECK (quote_count >= 0),
    CONSTRAINT lp_reliability_snapshots_fill_count_check CHECK (
        fill_count >= 0 AND fill_count <= quote_count
    ),
    CONSTRAINT lp_reliability_snapshots_reject_count_check CHECK (
        reject_count >= 0 AND reject_count <= quote_count
    ),
    CONSTRAINT lp_reliability_snapshots_settlement_count_check CHECK (
        settlement_count >= 0 AND settlement_count <= fill_count
    ),
    CONSTRAINT lp_reliability_snapshots_dispute_count_check CHECK (
        dispute_count >= 0 AND dispute_count <= settlement_count
    ),
    CONSTRAINT lp_reliability_snapshots_fill_rate_check CHECK (
        fill_rate >= 0 AND fill_rate <= 1
    ),
    CONSTRAINT lp_reliability_snapshots_reject_rate_check CHECK (
        reject_rate >= 0 AND reject_rate <= 1
    ),
    CONSTRAINT lp_reliability_snapshots_dispute_rate_check CHECK (
        dispute_rate >= 0 AND dispute_rate <= 1
    ),
    CONSTRAINT lp_reliability_snapshots_slippage_check CHECK (avg_slippage_bps >= 0),
    CONSTRAINT lp_reliability_snapshots_latency_p95_check CHECK (
        p95_settlement_latency_seconds >= 0
    ),
    CONSTRAINT lp_reliability_snapshots_metadata_object CHECK (
        jsonb_typeof(metadata) = 'object'
    ),
    CONSTRAINT lp_reliability_snapshots_unique_window UNIQUE (
        tenant_id,
        lp_id,
        direction,
        window_kind,
        window_started_at,
        window_ended_at,
        snapshot_version
    )
);

CREATE INDEX idx_lp_reliability_snapshots_lp_window
    ON lp_reliability_snapshots(tenant_id, lp_id, direction, window_kind, window_ended_at DESC);

CREATE INDEX idx_lp_reliability_snapshots_window
    ON lp_reliability_snapshots(tenant_id, window_kind, window_ended_at DESC);

CREATE INDEX idx_lp_reliability_snapshots_score
    ON lp_reliability_snapshots(tenant_id, direction, window_kind, reliability_score DESC)
    WHERE reliability_score IS NOT NULL;

CREATE INDEX idx_lp_reliability_snapshots_metadata
    ON lp_reliability_snapshots USING GIN (metadata);

ALTER TABLE lp_reliability_snapshots ENABLE ROW LEVEL SECURITY;

CREATE POLICY lp_reliability_snapshots_tenant_isolation ON lp_reliability_snapshots
    USING (tenant_id = current_setting('app.current_tenant', true));

CREATE TRIGGER trigger_lp_reliability_snapshots_updated_at
    BEFORE UPDATE ON lp_reliability_snapshots
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();
