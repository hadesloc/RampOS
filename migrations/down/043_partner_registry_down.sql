DROP TRIGGER IF EXISTS trigger_credential_references_updated_at ON credential_references;
DROP TRIGGER IF EXISTS trigger_partner_approval_references_updated_at ON partner_approval_references;
DROP TRIGGER IF EXISTS trigger_partner_rollout_scopes_updated_at ON partner_rollout_scopes;
DROP TRIGGER IF EXISTS trigger_partner_capabilities_updated_at ON partner_capabilities;
DROP TRIGGER IF EXISTS trigger_partners_updated_at ON partners;

DROP TABLE IF EXISTS credential_references;
DROP TABLE IF EXISTS partner_approval_references;
DROP TABLE IF EXISTS partner_health_signals;
DROP TABLE IF EXISTS partner_rollout_scopes;
DROP TABLE IF EXISTS partner_capabilities;
DROP TABLE IF EXISTS partners;
