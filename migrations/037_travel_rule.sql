-- Migration 037: Travel Rule policy and transport foundation
-- Models policy-driven disclosures, VASP registry records, transport attempts,
-- and exception queue state without hardcoding a single jurisdiction or network.

ALTER TYPE compliance_event_type ADD VALUE IF NOT EXISTS 'travel_rule_policy_evaluated';
ALTER TYPE compliance_event_type ADD VALUE IF NOT EXISTS 'travel_rule_disclosure_updated';
ALTER TYPE compliance_event_type ADD VALUE IF NOT EXISTS 'travel_rule_exception_queued';

-- ============================================================================
-- TRAVEL RULE POLICIES
-- ============================================================================

CREATE TABLE travel_rule_policies (
    id TEXT PRIMARY KEY,                                -- "trp_..." prefix
    tenant_id TEXT NOT NULL REFERENCES tenants(id),
    policy_code TEXT NOT NULL,
    display_name TEXT NOT NULL,

    -- Jurisdiction and applicability remain policy-driven.
    jurisdiction_code TEXT,
    direction_scope TEXT NOT NULL DEFAULT 'BOTH',
    asset_scope JSONB NOT NULL DEFAULT '{}'::jsonb,
    threshold_amount NUMERIC,
    threshold_currency TEXT,
    counterparty_scope JSONB NOT NULL DEFAULT '{}'::jsonb,

    -- Outcome behavior and versioning.
    default_transport_profile TEXT,
    default_action TEXT NOT NULL DEFAULT 'REVIEW_REQUIRED',
    policy_version TEXT NOT NULL DEFAULT 'v1',
    is_active BOOLEAN NOT NULL DEFAULT true,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT travel_rule_policies_direction_scope_check CHECK (
        direction_scope IN ('OUTBOUND', 'INBOUND', 'BOTH')
    ),
    CONSTRAINT travel_rule_policies_default_action_check CHECK (
        default_action IN (
            'ALLOW',
            'REVIEW_REQUIRED',
            'DISCLOSE_BEFORE_SETTLEMENT',
            'DISCLOSE_AFTER_SETTLEMENT',
            'BLOCK'
        )
    ),
    CONSTRAINT travel_rule_policies_threshold_amount_check CHECK (
        threshold_amount IS NULL OR threshold_amount >= 0
    ),
    CONSTRAINT travel_rule_policies_asset_scope_object CHECK (
        jsonb_typeof(asset_scope) = 'object'
    ),
    CONSTRAINT travel_rule_policies_counterparty_scope_object CHECK (
        jsonb_typeof(counterparty_scope) = 'object'
    ),
    CONSTRAINT travel_rule_policies_metadata_object CHECK (
        jsonb_typeof(metadata) = 'object'
    ),
    CONSTRAINT travel_rule_policies_unique_code_version UNIQUE (
        tenant_id,
        policy_code,
        policy_version
    )
);

CREATE INDEX idx_travel_rule_policies_tenant_active
    ON travel_rule_policies(tenant_id, is_active, updated_at DESC);

CREATE INDEX idx_travel_rule_policies_jurisdiction
    ON travel_rule_policies(tenant_id, jurisdiction_code, direction_scope)
    WHERE jurisdiction_code IS NOT NULL;

CREATE INDEX idx_travel_rule_policies_asset_scope
    ON travel_rule_policies USING GIN (asset_scope);

CREATE INDEX idx_travel_rule_policies_counterparty_scope
    ON travel_rule_policies USING GIN (counterparty_scope);

-- ============================================================================
-- TRAVEL RULE VASP REGISTRY
-- ============================================================================

CREATE TABLE travel_rule_vasps (
    id TEXT PRIMARY KEY,                                -- "trv_..." prefix
    tenant_id TEXT NOT NULL REFERENCES tenants(id),
    vasp_code TEXT NOT NULL,
    legal_name TEXT NOT NULL,
    display_name TEXT,

    jurisdiction_code TEXT,
    registration_number TEXT,
    travel_rule_profile TEXT,
    transport_profile TEXT,
    endpoint_uri TEXT,
    endpoint_public_key TEXT,

    review_status TEXT NOT NULL DEFAULT 'PENDING',
    interoperability_status TEXT NOT NULL DEFAULT 'UNKNOWN',
    supports_inbound BOOLEAN NOT NULL DEFAULT false,
    supports_outbound BOOLEAN NOT NULL DEFAULT false,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT travel_rule_vasps_review_status_check CHECK (
        review_status IN ('PENDING', 'APPROVED', 'REJECTED', 'SUSPENDED')
    ),
    CONSTRAINT travel_rule_vasps_interop_status_check CHECK (
        interoperability_status IN (
            'UNKNOWN',
            'READY',
            'LIMITED',
            'DEGRADED',
            'DISABLED'
        )
    ),
    CONSTRAINT travel_rule_vasps_metadata_object CHECK (
        jsonb_typeof(metadata) = 'object'
    ),
    CONSTRAINT travel_rule_vasps_unique_code UNIQUE (tenant_id, vasp_code)
);

CREATE INDEX idx_travel_rule_vasps_tenant_status
    ON travel_rule_vasps(tenant_id, review_status, interoperability_status, updated_at DESC);

CREATE INDEX idx_travel_rule_vasps_jurisdiction
    ON travel_rule_vasps(tenant_id, jurisdiction_code)
    WHERE jurisdiction_code IS NOT NULL;

CREATE INDEX idx_travel_rule_vasps_metadata
    ON travel_rule_vasps USING GIN (metadata);

-- ============================================================================
-- TRAVEL RULE DISCLOSURES
-- ============================================================================

CREATE TABLE travel_rule_disclosures (
    id TEXT PRIMARY KEY,                                -- "trd_..." prefix
    tenant_id TEXT NOT NULL REFERENCES tenants(id),
    policy_id TEXT REFERENCES travel_rule_policies(id),

    -- Optional links to lifecycle resources remain additive and nullable.
    intent_id TEXT,
    settlement_id TEXT REFERENCES settlements(id),
    transaction_reference TEXT,

    direction TEXT NOT NULL DEFAULT 'OUTBOUND',
    lifecycle_stage TEXT NOT NULL DEFAULT 'PENDING',
    asset_symbol TEXT NOT NULL,
    asset_amount NUMERIC NOT NULL DEFAULT 0,
    asset_network TEXT,
    fiat_currency TEXT,
    fiat_amount NUMERIC,

    originator_vasp_id TEXT REFERENCES travel_rule_vasps(id),
    beneficiary_vasp_id TEXT REFERENCES travel_rule_vasps(id),
    transport_profile TEXT,
    disclosure_payload JSONB NOT NULL DEFAULT '{}'::jsonb,
    redaction_profile TEXT,
    correlation_id TEXT,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT travel_rule_disclosures_direction_check CHECK (
        direction IN ('OUTBOUND', 'INBOUND')
    ),
    CONSTRAINT travel_rule_disclosures_lifecycle_stage_check CHECK (
        lifecycle_stage IN (
            'PENDING',
            'READY',
            'SENT',
            'ACKNOWLEDGED',
            'FAILED',
            'EXCEPTION',
            'WAIVED'
        )
    ),
    CONSTRAINT travel_rule_disclosures_asset_amount_check CHECK (asset_amount >= 0),
    CONSTRAINT travel_rule_disclosures_fiat_amount_check CHECK (
        fiat_amount IS NULL OR fiat_amount >= 0
    ),
    CONSTRAINT travel_rule_disclosures_payload_object CHECK (
        jsonb_typeof(disclosure_payload) = 'object'
    ),
    CONSTRAINT travel_rule_disclosures_metadata_object CHECK (
        jsonb_typeof(metadata) = 'object'
    )
);

CREATE INDEX idx_travel_rule_disclosures_tenant_stage
    ON travel_rule_disclosures(tenant_id, lifecycle_stage, created_at DESC);

CREATE INDEX idx_travel_rule_disclosures_intent
    ON travel_rule_disclosures(tenant_id, intent_id)
    WHERE intent_id IS NOT NULL;

CREATE INDEX idx_travel_rule_disclosures_settlement
    ON travel_rule_disclosures(tenant_id, settlement_id)
    WHERE settlement_id IS NOT NULL;

CREATE INDEX idx_travel_rule_disclosures_correlation
    ON travel_rule_disclosures(tenant_id, correlation_id)
    WHERE correlation_id IS NOT NULL;

CREATE INDEX idx_travel_rule_disclosures_payload
    ON travel_rule_disclosures USING GIN (disclosure_payload);

-- ============================================================================
-- TRANSPORT ATTEMPTS
-- ============================================================================

CREATE TABLE travel_rule_transport_attempts (
    id TEXT PRIMARY KEY,                                -- "trta_..." prefix
    tenant_id TEXT NOT NULL REFERENCES tenants(id),
    disclosure_id TEXT NOT NULL REFERENCES travel_rule_disclosures(id),
    attempt_number INTEGER NOT NULL DEFAULT 1,

    transport_kind TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'PENDING',
    endpoint_uri TEXT,
    request_payload JSONB NOT NULL DEFAULT '{}'::jsonb,
    response_payload JSONB,
    response_status_code INTEGER,
    error_code TEXT,
    error_message TEXT,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,

    attempted_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ,

    CONSTRAINT travel_rule_transport_attempts_attempt_number_check CHECK (
        attempt_number >= 1
    ),
    CONSTRAINT travel_rule_transport_attempts_status_check CHECK (
        status IN ('PENDING', 'SENT', 'ACKNOWLEDGED', 'FAILED', 'TIMEOUT', 'REJECTED')
    ),
    CONSTRAINT travel_rule_transport_attempts_request_payload_object CHECK (
        jsonb_typeof(request_payload) = 'object'
    ),
    CONSTRAINT travel_rule_transport_attempts_response_payload_object CHECK (
        response_payload IS NULL OR jsonb_typeof(response_payload) = 'object'
    ),
    CONSTRAINT travel_rule_transport_attempts_metadata_object CHECK (
        jsonb_typeof(metadata) = 'object'
    ),
    CONSTRAINT travel_rule_transport_attempts_completed_after_attempted CHECK (
        completed_at IS NULL OR completed_at >= attempted_at
    ),
    CONSTRAINT travel_rule_transport_attempts_unique_attempt UNIQUE (
        disclosure_id,
        attempt_number
    )
);

CREATE INDEX idx_travel_rule_transport_attempts_disclosure
    ON travel_rule_transport_attempts(tenant_id, disclosure_id, attempt_number DESC);

CREATE INDEX idx_travel_rule_transport_attempts_status
    ON travel_rule_transport_attempts(tenant_id, status, attempted_at DESC);

CREATE INDEX idx_travel_rule_transport_attempts_transport_kind
    ON travel_rule_transport_attempts(tenant_id, transport_kind, attempted_at DESC);

-- ============================================================================
-- EXCEPTION QUEUE
-- ============================================================================

CREATE TABLE travel_rule_exception_queue (
    id TEXT PRIMARY KEY,                                -- "tre_..." prefix
    tenant_id TEXT NOT NULL REFERENCES tenants(id),
    disclosure_id TEXT NOT NULL REFERENCES travel_rule_disclosures(id),
    latest_attempt_id TEXT REFERENCES travel_rule_transport_attempts(id),

    queue_status TEXT NOT NULL DEFAULT 'OPEN',
    severity TEXT NOT NULL DEFAULT 'MEDIUM',
    reason_code TEXT NOT NULL,
    reason_details TEXT,
    assigned_to TEXT,
    due_at TIMESTAMPTZ,
    resolved_at TIMESTAMPTZ,
    resolution_notes TEXT,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT travel_rule_exception_queue_status_check CHECK (
        queue_status IN ('OPEN', 'IN_REVIEW', 'ESCALATED', 'RESOLVED', 'DISMISSED')
    ),
    CONSTRAINT travel_rule_exception_queue_severity_check CHECK (
        severity IN ('LOW', 'MEDIUM', 'HIGH', 'CRITICAL')
    ),
    CONSTRAINT travel_rule_exception_queue_resolution_order CHECK (
        resolved_at IS NULL OR resolved_at >= created_at
    ),
    CONSTRAINT travel_rule_exception_queue_metadata_object CHECK (
        jsonb_typeof(metadata) = 'object'
    )
);

CREATE INDEX idx_travel_rule_exception_queue_status
    ON travel_rule_exception_queue(tenant_id, queue_status, severity, created_at DESC);

CREATE INDEX idx_travel_rule_exception_queue_assignee
    ON travel_rule_exception_queue(tenant_id, assigned_to, queue_status)
    WHERE assigned_to IS NOT NULL;

CREATE INDEX idx_travel_rule_exception_queue_disclosure
    ON travel_rule_exception_queue(tenant_id, disclosure_id);

-- ============================================================================
-- ROW LEVEL SECURITY + UPDATED_AT
-- ============================================================================

ALTER TABLE travel_rule_policies ENABLE ROW LEVEL SECURITY;
ALTER TABLE travel_rule_vasps ENABLE ROW LEVEL SECURITY;
ALTER TABLE travel_rule_disclosures ENABLE ROW LEVEL SECURITY;
ALTER TABLE travel_rule_transport_attempts ENABLE ROW LEVEL SECURITY;
ALTER TABLE travel_rule_exception_queue ENABLE ROW LEVEL SECURITY;

CREATE POLICY travel_rule_policies_tenant_isolation ON travel_rule_policies
    USING (tenant_id = current_setting('app.current_tenant', true));

CREATE POLICY travel_rule_vasps_tenant_isolation ON travel_rule_vasps
    USING (tenant_id = current_setting('app.current_tenant', true));

CREATE POLICY travel_rule_disclosures_tenant_isolation ON travel_rule_disclosures
    USING (tenant_id = current_setting('app.current_tenant', true));

CREATE POLICY travel_rule_transport_attempts_tenant_isolation ON travel_rule_transport_attempts
    USING (tenant_id = current_setting('app.current_tenant', true));

CREATE POLICY travel_rule_exception_queue_tenant_isolation ON travel_rule_exception_queue
    USING (tenant_id = current_setting('app.current_tenant', true));

CREATE TRIGGER trigger_travel_rule_policies_updated_at
    BEFORE UPDATE ON travel_rule_policies
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();

CREATE TRIGGER trigger_travel_rule_vasps_updated_at
    BEFORE UPDATE ON travel_rule_vasps
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();

CREATE TRIGGER trigger_travel_rule_disclosures_updated_at
    BEFORE UPDATE ON travel_rule_disclosures
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();

CREATE TRIGGER trigger_travel_rule_exception_queue_updated_at
    BEFORE UPDATE ON travel_rule_exception_queue
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();
