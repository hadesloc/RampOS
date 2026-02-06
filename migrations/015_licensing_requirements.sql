-- Migration: 015_licensing_requirements.sql
-- Vietnam Licensing Requirements Tables

-- License Requirements table (system-wide requirements)
CREATE TABLE IF NOT EXISTS license_requirements (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT NOT NULL,
    license_type TEXT NOT NULL,
    regulatory_body TEXT NOT NULL,
    deadline TIMESTAMPTZ,
    renewal_period_days INTEGER,
    required_documents JSONB NOT NULL DEFAULT '[]',
    is_mandatory BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Index for deadline queries
CREATE INDEX IF NOT EXISTS idx_license_requirements_deadline
    ON license_requirements(deadline)
    WHERE deadline IS NOT NULL;

-- Tenant License Status table (tenant-specific status per requirement)
CREATE TABLE IF NOT EXISTS tenant_license_status (
    id TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL REFERENCES tenants(id),
    requirement_id TEXT NOT NULL REFERENCES license_requirements(id),
    status TEXT NOT NULL DEFAULT 'PENDING',
    license_number TEXT,
    issue_date TIMESTAMPTZ,
    expiry_date TIMESTAMPTZ,
    last_submission_id TEXT,
    notes TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    UNIQUE(tenant_id, requirement_id)
);

-- Indexes for tenant license status
CREATE INDEX IF NOT EXISTS idx_tenant_license_status_tenant
    ON tenant_license_status(tenant_id);
CREATE INDEX IF NOT EXISTS idx_tenant_license_status_expiry
    ON tenant_license_status(expiry_date)
    WHERE expiry_date IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_tenant_license_status_status
    ON tenant_license_status(status);

-- License Submissions table (document submissions)
CREATE TABLE IF NOT EXISTS license_submissions (
    id TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL REFERENCES tenants(id),
    requirement_id TEXT NOT NULL REFERENCES license_requirements(id),
    documents JSONB NOT NULL DEFAULT '[]',
    status TEXT NOT NULL DEFAULT 'SUBMITTED',
    submitted_by TEXT NOT NULL,
    submitted_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    reviewed_at TIMESTAMPTZ,
    reviewer_notes TEXT
);

-- Indexes for license submissions
CREATE INDEX IF NOT EXISTS idx_license_submissions_tenant
    ON license_submissions(tenant_id);
CREATE INDEX IF NOT EXISTS idx_license_submissions_requirement
    ON license_submissions(requirement_id);
CREATE INDEX IF NOT EXISTS idx_license_submissions_status
    ON license_submissions(status);

-- Enable RLS on all tables
ALTER TABLE license_requirements ENABLE ROW LEVEL SECURITY;
ALTER TABLE tenant_license_status ENABLE ROW LEVEL SECURITY;
ALTER TABLE license_submissions ENABLE ROW LEVEL SECURITY;

-- RLS Policies for license_requirements (read by all authenticated, write by admin)
CREATE POLICY license_requirements_select ON license_requirements
    FOR SELECT USING (true);

-- RLS Policies for tenant_license_status (tenant isolation)
CREATE POLICY tenant_license_status_select ON tenant_license_status
    FOR SELECT USING (
        tenant_id = current_setting('app.current_tenant', true)
        OR current_setting('app.current_tenant', true) IS NULL
    );

CREATE POLICY tenant_license_status_insert ON tenant_license_status
    FOR INSERT WITH CHECK (
        tenant_id = current_setting('app.current_tenant', true)
        OR current_setting('app.current_tenant', true) IS NULL
    );

CREATE POLICY tenant_license_status_update ON tenant_license_status
    FOR UPDATE USING (
        tenant_id = current_setting('app.current_tenant', true)
        OR current_setting('app.current_tenant', true) IS NULL
    );

-- RLS Policies for license_submissions (tenant isolation)
CREATE POLICY license_submissions_select ON license_submissions
    FOR SELECT USING (
        tenant_id = current_setting('app.current_tenant', true)
        OR current_setting('app.current_tenant', true) IS NULL
    );

CREATE POLICY license_submissions_insert ON license_submissions
    FOR INSERT WITH CHECK (
        tenant_id = current_setting('app.current_tenant', true)
        OR current_setting('app.current_tenant', true) IS NULL
    );

CREATE POLICY license_submissions_update ON license_submissions
    FOR UPDATE USING (
        tenant_id = current_setting('app.current_tenant', true)
        OR current_setting('app.current_tenant', true) IS NULL
    );

-- Seed Vietnam-specific licensing requirements
INSERT INTO license_requirements (id, name, description, license_type, regulatory_body, deadline, renewal_period_days, required_documents, is_mandatory)
VALUES
    ('lic_sbv_payment', 'SBV Payment Service License',
     'License from State Bank of Vietnam to provide intermediary payment services per Decree 101/2012/ND-CP',
     'SBV_PAYMENT_LICENSE', 'State Bank of Vietnam',
     NULL, 365,
     '["Business registration certificate", "Charter capital proof (min 50B VND)", "Technical infrastructure documentation", "AML/CFT policy", "Risk management policy", "Board of directors CVs"]'::jsonb,
     true),

    ('lic_aml_reg', 'AML Registration',
     'Registration with State Bank of Vietnam for anti-money laundering compliance per Law 07/2022/QH15',
     'AML_REGISTRATION', 'State Bank of Vietnam',
     NULL, 365,
     '["AML/CFT policy", "Customer identification procedures", "Transaction monitoring procedures", "Suspicious activity reporting procedures", "Staff training plan"]'::jsonb,
     true),

    ('lic_data_protection', 'Data Protection Registration',
     'Registration with Ministry of Public Security for personal data processing per Decree 13/2023/ND-CP',
     'DATA_PROTECTION', 'Ministry of Public Security',
     NULL, NULL,
     '["Data processing impact assessment", "Data protection policy", "Security measures documentation", "Cross-border transfer safeguards (if applicable)"]'::jsonb,
     true),

    ('lic_business_reg', 'Business Registration Certificate',
     'Enterprise registration certificate from Department of Planning and Investment',
     'BUSINESS_REGISTRATION', 'Department of Planning and Investment',
     NULL, NULL,
     '["Application form", "Charter", "List of founding members", "Head office lease agreement"]'::jsonb,
     true),

    ('lic_forex', 'Foreign Exchange License',
     'License for foreign exchange activities per Ordinance on Foreign Exchange',
     'FOREX_LICENSE', 'State Bank of Vietnam',
     NULL, 365,
     '["SBV payment license", "Forex trading procedures", "Risk management for FX", "Qualified personnel documentation"]'::jsonb,
     false)
ON CONFLICT (id) DO NOTHING;

-- Comments for documentation
COMMENT ON TABLE license_requirements IS 'Vietnam regulatory licensing requirements that tenants must fulfill';
COMMENT ON TABLE tenant_license_status IS 'Tracks each tenant''s compliance status for licensing requirements';
COMMENT ON TABLE license_submissions IS 'Document submissions from tenants for licensing applications';
