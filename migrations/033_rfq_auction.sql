-- Migration 033: RFQ Auction Layer (Bidirectional)
-- Adds tables for Request-For-Quote auction mechanism for both
-- off-ramp (USDT -> VND) and on-ramp (VND -> USDT) directions.
-- Does NOT modify any existing tables.

-- ============================================================================
-- RFQ REQUESTS: User broadcasts a request for quotes
-- ============================================================================

CREATE TABLE rfq_requests (
    id              TEXT PRIMARY KEY,                        -- "rfq_..." prefix
    tenant_id       TEXT NOT NULL REFERENCES tenants(id),
    user_id         TEXT NOT NULL,

    -- Direction: OFFRAMP (crypto->VND) or ONRAMP (VND->crypto)
    direction       TEXT NOT NULL DEFAULT 'OFFRAMP',

    -- Link to an existing offramp_intent (optional, for OFFRAMP direction)
    offramp_id      TEXT REFERENCES offramp_intents(id),

    -- Asset being exchanged
    crypto_asset    TEXT NOT NULL,                           -- e.g. USDT, ETH
    crypto_amount   NUMERIC NOT NULL,                        -- for OFFRAMP: crypto to sell
    vnd_amount      NUMERIC,                                 -- for ONRAMP: VND budget

    -- Auction state machine
    -- OPEN -> MATCHED | EXPIRED | CANCELLED
    state           TEXT NOT NULL DEFAULT 'OPEN',

    -- Winner (set when state = MATCHED)
    winning_bid_id  TEXT,
    winning_lp_id   TEXT,
    final_rate      NUMERIC,                                 -- final agreed VND/crypto rate

    -- Timing
    expires_at      TIMESTAMPTZ NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT rfq_requests_direction_check CHECK (direction IN ('OFFRAMP', 'ONRAMP')),
    CONSTRAINT rfq_requests_state_check CHECK (state IN ('OPEN', 'MATCHED', 'EXPIRED', 'CANCELLED'))
);

CREATE INDEX idx_rfq_requests_tenant_state ON rfq_requests(tenant_id, state, created_at DESC);
CREATE INDEX idx_rfq_requests_user ON rfq_requests(tenant_id, user_id);
CREATE INDEX idx_rfq_requests_offramp ON rfq_requests(offramp_id) WHERE offramp_id IS NOT NULL;

-- ============================================================================
-- RFQ BIDS: Liquidity Providers submit price quotes
-- ============================================================================

CREATE TABLE rfq_bids (
    id              TEXT PRIMARY KEY,                        -- "bid_..." prefix
    rfq_id          TEXT NOT NULL REFERENCES rfq_requests(id),
    tenant_id       TEXT NOT NULL REFERENCES tenants(id),

    -- LP identifier (mapped to a tenant or external LP API key)
    lp_id           TEXT NOT NULL,
    lp_name         TEXT,

    -- Price quote
    -- For OFFRAMP: how much VND the LP will pay per unit of crypto (higher = better for user)
    -- For ONRAMP:  how much VND the LP wants per unit of crypto (lower = better for user)
    exchange_rate   NUMERIC NOT NULL,
    vnd_amount      NUMERIC NOT NULL,                        -- total VND in the deal

    -- Validity window for this bid
    valid_until     TIMESTAMPTZ NOT NULL,

    -- Bid state machine
    -- PENDING -> ACCEPTED | REJECTED | EXPIRED
    state           TEXT NOT NULL DEFAULT 'PENDING',

    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT rfq_bids_state_check CHECK (state IN ('PENDING', 'ACCEPTED', 'REJECTED', 'EXPIRED'))
);

-- Best bid lookup: order by exchange_rate DESC for OFFRAMP, ASC for ONRAMP
CREATE INDEX idx_rfq_bids_rfq_rate ON rfq_bids(rfq_id, state, exchange_rate DESC);
CREATE INDEX idx_rfq_bids_lp ON rfq_bids(tenant_id, lp_id, created_at DESC);

-- ============================================================================
-- ROW LEVEL SECURITY
-- ============================================================================

ALTER TABLE rfq_requests ENABLE ROW LEVEL SECURITY;
ALTER TABLE rfq_bids ENABLE ROW LEVEL SECURITY;

CREATE POLICY rfq_requests_tenant_isolation ON rfq_requests
    USING (tenant_id = current_setting('app.current_tenant', true));

CREATE POLICY rfq_bids_tenant_isolation ON rfq_bids
    USING (tenant_id = current_setting('app.current_tenant', true));

-- ============================================================================
-- AUTO-UPDATE updated_at
-- ============================================================================

CREATE TRIGGER trigger_rfq_requests_updated_at
    BEFORE UPDATE ON rfq_requests
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();
