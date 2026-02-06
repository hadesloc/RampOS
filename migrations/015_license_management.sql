-- License Management Schema for Multi-tenant Compliance
-- Migration: 015_license_management.sql

-- ============================================================================
-- LICENSE TYPES (Master table for supported license types)
-- ============================================================================

CREATE TABLE license_types (
    id VARCHAR(64) PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    code VARCHAR(32) NOT NULL UNIQUE,  -- EXCHANGE, CUSTODIAL, PAYMENT
    description TEXT,
    jurisdiction VARCHAR(64) NOT NULL DEFAULT 'VN',  -- Country code
    regulatory_body VARCHAR(255),  -- e.g., "State Bank of Vietnam"
    is_active BOOLEAN NOT NULL DEFAULT true,
    metadata JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_license_types_code ON license_types(code);
CREATE INDEX idx_license_types_jurisdiction ON license_types(jurisdiction);

-- ============================================================================
-- LICENSE REQUIREMENTS (Requirements for each license type)
-- ============================================================================

CREATE TABLE license_requirements (
    id VARCHAR(64) PRIMARY KEY,
    license_type_id VARCHAR(64) NOT NULL REFERENCES license_types(id) ON DELETE CASCADE,
    requirement_name VARCHAR(255) NOT NULL,
    requirement_code VARCHAR(64) NOT NULL,
    description TEXT,
    is_mandatory BOOLEAN NOT NULL DEFAULT true,
    document_type VARCHAR(64),  -- Type of document required
    validation_rules JSONB DEFAULT '{}',  -- JSON rules for validation
    display_order INT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    UNIQUE (license_type_id, requirement_code)
);

CREATE INDEX idx_license_requirements_type ON license_requirements(license_type_id);
CREATE INDEX idx_license_requirements_mandatory ON license_requirements(license_type_id, is_mandatory);

-- ============================================================================
-- TENANT LICENSES (License assignments per tenant)
-- ============================================================================

CREATE TABLE tenant_licenses (
    id VARCHAR(64) PRIMARY KEY,
    tenant_id VARCHAR(64) NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    license_type_id VARCHAR(64) NOT NULL REFERENCES license_types(id),

    -- Status workflow: DRAFT -> SUBMITTED -> UNDER_REVIEW -> APPROVED/REJECTED -> ACTIVE/EXPIRED
    status VARCHAR(32) NOT NULL DEFAULT 'DRAFT',

    -- License details
    license_number VARCHAR(128),  -- Official license number when approved
    issued_at TIMESTAMPTZ,
    expires_at TIMESTAMPTZ,

    -- Compliance tracking
    compliance_percentage DECIMAL(5, 2) NOT NULL DEFAULT 0,
    last_compliance_check TIMESTAMPTZ,

    -- Review info
    submitted_at TIMESTAMPTZ,
    reviewed_by VARCHAR(64),
    reviewed_at TIMESTAMPTZ,
    review_notes TEXT,
    rejection_reason TEXT,

    -- Metadata
    metadata JSONB NOT NULL DEFAULT '{}',

    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    UNIQUE (tenant_id, license_type_id),
    CONSTRAINT tenant_licenses_status_check CHECK (status IN (
        'DRAFT', 'SUBMITTED', 'UNDER_REVIEW', 'APPROVED', 'REJECTED', 'ACTIVE', 'EXPIRED', 'SUSPENDED', 'REVOKED'
    ))
);

CREATE INDEX idx_tenant_licenses_tenant ON tenant_licenses(tenant_id);
CREATE INDEX idx_tenant_licenses_type ON tenant_licenses(license_type_id);
CREATE INDEX idx_tenant_licenses_status ON tenant_licenses(tenant_id, status);
CREATE INDEX idx_tenant_licenses_expires ON tenant_licenses(expires_at) WHERE expires_at IS NOT NULL;

-- ============================================================================
-- TENANT LICENSE DOCUMENTS (Documents submitted for license requirements)
-- ============================================================================

CREATE TABLE tenant_license_documents (
    id VARCHAR(64) PRIMARY KEY,
    tenant_id VARCHAR(64) NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    tenant_license_id VARCHAR(64) NOT NULL REFERENCES tenant_licenses(id) ON DELETE CASCADE,
    requirement_id VARCHAR(64) NOT NULL REFERENCES license_requirements(id),

    -- Document info
    document_name VARCHAR(255) NOT NULL,
    document_url VARCHAR(1024) NOT NULL,
    document_hash VARCHAR(128),  -- SHA256 hash for integrity
    file_size BIGINT,
    mime_type VARCHAR(128),

    -- Status: PENDING, APPROVED, REJECTED, EXPIRED
    status VARCHAR(32) NOT NULL DEFAULT 'PENDING',

    -- Review info
    reviewed_by VARCHAR(64),
    reviewed_at TIMESTAMPTZ,
    review_notes TEXT,
    rejection_reason TEXT,

    -- Validity
    valid_from TIMESTAMPTZ,
    valid_until TIMESTAMPTZ,

    -- Metadata
    metadata JSONB NOT NULL DEFAULT '{}',

    -- Timestamps
    uploaded_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT tenant_license_documents_status_check CHECK (status IN (
        'PENDING', 'APPROVED', 'REJECTED', 'EXPIRED'
    ))
);

CREATE INDEX idx_tenant_license_documents_tenant ON tenant_license_documents(tenant_id);
CREATE INDEX idx_tenant_license_documents_license ON tenant_license_documents(tenant_license_id);
CREATE INDEX idx_tenant_license_documents_requirement ON tenant_license_documents(requirement_id);
CREATE INDEX idx_tenant_license_documents_status ON tenant_license_documents(tenant_license_id, status);

-- ============================================================================
-- ROW LEVEL SECURITY
-- ============================================================================

ALTER TABLE tenant_licenses ENABLE ROW LEVEL SECURITY;
ALTER TABLE tenant_license_documents ENABLE ROW LEVEL SECURITY;

-- Policy for tenant_licenses
CREATE POLICY tenant_licenses_isolation ON tenant_licenses
    FOR ALL
    USING (
        tenant_id = current_setting('app.current_tenant', true)
        OR current_setting('app.current_tenant', true) IS NULL
        OR current_setting('app.current_tenant', true) = ''
    );

-- Policy for tenant_license_documents
CREATE POLICY tenant_license_documents_isolation ON tenant_license_documents
    FOR ALL
    USING (
        tenant_id = current_setting('app.current_tenant', true)
        OR current_setting('app.current_tenant', true) IS NULL
        OR current_setting('app.current_tenant', true) = ''
    );

-- ============================================================================
-- TRIGGERS
-- ============================================================================

CREATE TRIGGER trigger_license_types_updated_at
    BEFORE UPDATE ON license_types
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();

CREATE TRIGGER trigger_license_requirements_updated_at
    BEFORE UPDATE ON license_requirements
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();

CREATE TRIGGER trigger_tenant_licenses_updated_at
    BEFORE UPDATE ON tenant_licenses
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();

CREATE TRIGGER trigger_tenant_license_documents_updated_at
    BEFORE UPDATE ON tenant_license_documents
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();

-- ============================================================================
-- SEED DATA: License Types
-- ============================================================================

INSERT INTO license_types (id, name, code, description, jurisdiction, regulatory_body) VALUES
    ('lt_exchange', 'Crypto Exchange License', 'EXCHANGE',
     'License to operate a cryptocurrency exchange platform in Vietnam',
     'VN', 'State Bank of Vietnam'),
    ('lt_custodial', 'Digital Asset Custody License', 'CUSTODIAL',
     'License to provide custodial services for digital assets',
     'VN', 'State Bank of Vietnam'),
    ('lt_payment', 'Payment Service Provider License', 'PAYMENT',
     'License to provide payment processing services',
     'VN', 'State Bank of Vietnam');

-- ============================================================================
-- SEED DATA: License Requirements
-- ============================================================================

-- EXCHANGE License Requirements
INSERT INTO license_requirements (id, license_type_id, requirement_name, requirement_code, description, is_mandatory, document_type, display_order) VALUES
    ('lr_ex_001', 'lt_exchange', 'Business Registration Certificate', 'BRC',
     'Valid business registration certificate from the Ministry of Planning and Investment', true, 'CERTIFICATE', 1),
    ('lr_ex_002', 'lt_exchange', 'Capital Proof', 'CAPITAL_PROOF',
     'Proof of minimum capital requirement (100 billion VND)', true, 'FINANCIAL', 2),
    ('lr_ex_003', 'lt_exchange', 'AML/CFT Policy', 'AML_POLICY',
     'Anti-money laundering and counter-terrorism financing policy document', true, 'POLICY', 3),
    ('lr_ex_004', 'lt_exchange', 'Security Audit Report', 'SECURITY_AUDIT',
     'Third-party security audit report for trading platform', true, 'AUDIT', 4),
    ('lr_ex_005', 'lt_exchange', 'Insurance Certificate', 'INSURANCE',
     'Insurance coverage for digital asset custody', false, 'CERTIFICATE', 5);

-- CUSTODIAL License Requirements
INSERT INTO license_requirements (id, license_type_id, requirement_name, requirement_code, description, is_mandatory, document_type, display_order) VALUES
    ('lr_cu_001', 'lt_custodial', 'Business Registration Certificate', 'BRC',
     'Valid business registration certificate', true, 'CERTIFICATE', 1),
    ('lr_cu_002', 'lt_custodial', 'Cold Storage Policy', 'COLD_STORAGE',
     'Documentation of cold storage procedures and security measures', true, 'POLICY', 2),
    ('lr_cu_003', 'lt_custodial', 'Key Management Procedure', 'KEY_MGMT',
     'Cryptographic key management procedures', true, 'POLICY', 3),
    ('lr_cu_004', 'lt_custodial', 'SOC 2 Type II Report', 'SOC2',
     'SOC 2 Type II compliance report', true, 'AUDIT', 4),
    ('lr_cu_005', 'lt_custodial', 'Disaster Recovery Plan', 'DR_PLAN',
     'Business continuity and disaster recovery plan', true, 'POLICY', 5);

-- PAYMENT License Requirements
INSERT INTO license_requirements (id, license_type_id, requirement_name, requirement_code, description, is_mandatory, document_type, display_order) VALUES
    ('lr_pm_001', 'lt_payment', 'Business Registration Certificate', 'BRC',
     'Valid business registration certificate', true, 'CERTIFICATE', 1),
    ('lr_pm_002', 'lt_payment', 'PCI DSS Compliance', 'PCI_DSS',
     'Payment Card Industry Data Security Standard compliance certificate', true, 'CERTIFICATE', 2),
    ('lr_pm_003', 'lt_payment', 'Bank Partnership Agreement', 'BANK_AGREEMENT',
     'Partnership agreement with a licensed bank', true, 'CONTRACT', 3),
    ('lr_pm_004', 'lt_payment', 'Transaction Monitoring System', 'TXN_MONITORING',
     'Documentation of transaction monitoring capabilities', true, 'POLICY', 4);
