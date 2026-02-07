-- Migration: Usage-based Billing
-- Description: Track metered usage and invoices for enterprise billing

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'billing_period') THEN
        CREATE TYPE billing_period AS ENUM ('monthly', 'yearly');
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'meter_type') THEN
        CREATE TYPE meter_type AS ENUM ('api_calls', 'transaction_volume', 'active_users', 'storage_gb');
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'aggregation_type') THEN
        CREATE TYPE aggregation_type AS ENUM ('sum', 'max', 'unique_count');
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'invoice_status') THEN
        CREATE TYPE invoice_status AS ENUM ('draft', 'open', 'paid', 'void', 'uncollectible');
    END IF;
END $$;

-- Pricing Plans (Tier definition extensions)
CREATE TABLE IF NOT EXISTS pricing_plans (
    id VARCHAR(64) PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    currency VARCHAR(3) NOT NULL DEFAULT 'USD',
    period billing_period NOT NULL DEFAULT 'monthly',
    base_fee DECIMAL(18, 2) NOT NULL DEFAULT 0,

    -- Limits & Quotas
    included_api_calls BIGINT NOT NULL DEFAULT 0,
    included_mau BIGINT NOT NULL DEFAULT 0,
    included_volume DECIMAL(36, 18) NOT NULL DEFAULT 0,

    -- Overage Rates
    api_call_unit_price DECIMAL(18, 8) NOT NULL DEFAULT 0, -- Cost per extra call
    mau_unit_price DECIMAL(18, 2) NOT NULL DEFAULT 0,      -- Cost per extra user
    volume_percentage_fee DECIMAL(5, 4) NOT NULL DEFAULT 0, -- % fee on volume (0.001 = 0.1%)

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Billing Meters (Definitions of what we track)
CREATE TABLE IF NOT EXISTS billing_meters (
    id VARCHAR(64) PRIMARY KEY,
    slug VARCHAR(64) NOT NULL UNIQUE, -- e.g., 'api_requests_total'
    name VARCHAR(255) NOT NULL,
    type meter_type NOT NULL,
    aggregation aggregation_type NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Usage Events (Raw high-volume table - partitioned in production)
-- In a real production setup, this would be in TimescaleDB or ClickHouse
CREATE TABLE IF NOT EXISTS usage_events (
    id VARCHAR(64) PRIMARY KEY, -- ULID recommended for sorting
    tenant_id VARCHAR(64) NOT NULL,
    meter_slug VARCHAR(64) NOT NULL REFERENCES billing_meters(slug),
    amount DECIMAL(36, 18) NOT NULL,
    dimensions JSONB DEFAULT '{}'::jsonb, -- Tags like { "endpoint": "/v1/payin", "status": "200" }
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Quick lookups
    CONSTRAINT fk_usage_tenant FOREIGN KEY (tenant_id) REFERENCES tenants(id)
);

-- Daily Usage Aggregates (For fast reporting)
CREATE TABLE IF NOT EXISTS daily_usage (
    tenant_id VARCHAR(64) NOT NULL REFERENCES tenants(id),
    meter_slug VARCHAR(64) NOT NULL REFERENCES billing_meters(slug),
    date DATE NOT NULL,
    total_amount DECIMAL(36, 18) NOT NULL DEFAULT 0,

    PRIMARY KEY (tenant_id, meter_slug, date)
);

-- Invoices
CREATE TABLE IF NOT EXISTS invoices (
    id VARCHAR(64) PRIMARY KEY,
    tenant_id VARCHAR(64) NOT NULL REFERENCES tenants(id),
    period_start TIMESTAMPTZ NOT NULL,
    period_end TIMESTAMPTZ NOT NULL,
    status invoice_status NOT NULL DEFAULT 'draft',

    currency VARCHAR(3) NOT NULL,
    subtotal DECIMAL(18, 2) NOT NULL,
    tax DECIMAL(18, 2) NOT NULL DEFAULT 0,
    total DECIMAL(18, 2) NOT NULL,

    line_items JSONB NOT NULL, -- Snapshot of calculations

    due_date DATE,
    paid_at TIMESTAMPTZ,

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes
CREATE INDEX IF NOT EXISTS idx_usage_events_tenant_time ON usage_events(tenant_id, timestamp);
CREATE INDEX IF NOT EXISTS idx_usage_events_tenant_meter_time ON usage_events(tenant_id, meter_slug, timestamp);
CREATE INDEX IF NOT EXISTS idx_invoices_tenant_period ON invoices(tenant_id, period_start);
CREATE INDEX IF NOT EXISTS idx_pricing_plans_name ON pricing_plans(name);

-- RLS for tenant-scoped tables (pricing_plans and billing_meters are global)
ALTER TABLE usage_events ENABLE ROW LEVEL SECURITY;
ALTER TABLE daily_usage ENABLE ROW LEVEL SECURITY;
ALTER TABLE invoices ENABLE ROW LEVEL SECURITY;

CREATE POLICY tenant_isolation_usage_events ON usage_events
    FOR ALL
    USING (tenant_id = current_setting('app.current_tenant', true));

CREATE POLICY tenant_isolation_daily_usage ON daily_usage
    FOR ALL
    USING (tenant_id = current_setting('app.current_tenant', true));

CREATE POLICY tenant_isolation_invoices ON invoices
    FOR ALL
    USING (tenant_id = current_setting('app.current_tenant', true));

-- Seed basic meters
INSERT INTO billing_meters (id, slug, name, type, aggregation) VALUES
    ('meter_01', 'api_requests', 'API Requests', 'api_calls', 'sum'),
    ('meter_02', 'tx_volume_usd', 'Transaction Volume (USD)', 'transaction_volume', 'sum'),
    ('meter_03', 'active_users', 'Monthly Active Users', 'active_users', 'unique_count');
