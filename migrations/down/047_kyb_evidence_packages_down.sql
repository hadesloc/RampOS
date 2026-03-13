DROP TRIGGER IF EXISTS trigger_kyb_ubo_evidence_links_updated_at ON kyb_ubo_evidence_links;
DROP TRIGGER IF EXISTS trigger_kyb_evidence_sources_updated_at ON kyb_evidence_sources;
DROP TRIGGER IF EXISTS trigger_kyb_evidence_packages_updated_at ON kyb_evidence_packages;

DROP TABLE IF EXISTS kyb_ubo_evidence_links;
DROP TABLE IF EXISTS kyb_evidence_sources;
DROP TABLE IF EXISTS kyb_evidence_packages;
