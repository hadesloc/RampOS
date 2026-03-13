DROP TRIGGER IF EXISTS trigger_corridor_eligibility_rules_updated_at ON corridor_eligibility_rules;
DROP TRIGGER IF EXISTS trigger_corridor_rollout_scopes_updated_at ON corridor_rollout_scopes;
DROP TRIGGER IF EXISTS trigger_corridor_compliance_hooks_updated_at ON corridor_compliance_hooks;
DROP TRIGGER IF EXISTS trigger_corridor_cutoff_policies_updated_at ON corridor_cutoff_policies;
DROP TRIGGER IF EXISTS trigger_corridor_fee_profiles_updated_at ON corridor_fee_profiles;
DROP TRIGGER IF EXISTS trigger_corridor_pack_endpoints_updated_at ON corridor_pack_endpoints;
DROP TRIGGER IF EXISTS trigger_corridor_packs_updated_at ON corridor_packs;

DROP TABLE IF EXISTS corridor_eligibility_rules;
DROP TABLE IF EXISTS corridor_rollout_scopes;
DROP TABLE IF EXISTS corridor_compliance_hooks;
DROP TABLE IF EXISTS corridor_cutoff_policies;
DROP TABLE IF EXISTS corridor_fee_profiles;
DROP TABLE IF EXISTS corridor_pack_endpoints;
DROP TABLE IF EXISTS corridor_packs;
