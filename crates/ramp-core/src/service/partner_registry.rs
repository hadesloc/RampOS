use ramp_common::Result;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::sync::Arc;

use crate::repository::{
    PartnerRegistryRecord, PartnerRegistryRepository, PgPartnerRegistryRepository,
    UpsertApprovalReferenceRequest, UpsertCredentialReferenceRequest,
    UpsertPartnerCapabilityRequest, UpsertPartnerHealthSignalRequest, UpsertPartnerRequest,
    UpsertPartnerRolloutScopeRequest,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PartnerRegistrySnapshot {
    pub action_mode: String,
    pub source: String,
    pub partners: Vec<PartnerRegistryRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpsertPartnerCapabilityBundle {
    pub capability: UpsertPartnerCapabilityRequest,
    pub rollout_scopes: Vec<UpsertPartnerRolloutScopeRequest>,
    pub health_signals: Vec<UpsertPartnerHealthSignalRequest>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpsertPartnerRegistryRecordRequest {
    pub partner: UpsertPartnerRequest,
    pub approval_references: Vec<UpsertApprovalReferenceRequest>,
    pub capabilities: Vec<UpsertPartnerCapabilityBundle>,
    pub credential_references: Vec<UpsertCredentialReferenceRequest>,
}

/// Governance search criteria for filtering partner registry records.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PartnerSearchCriteria {
    pub partner_class: Option<String>,
    pub market: Option<String>,
    pub capability_family: Option<String>,
    pub approval_status: Option<String>,
    pub lifecycle_state: Option<String>,
    pub service_domain: Option<String>,
}

/// Aggregated health summary for a single partner across all capabilities.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PartnerHealthSummary {
    pub partner_id: String,
    pub display_name: String,
    pub partner_class: String,
    pub capability_count: usize,
    pub healthy_capability_count: usize,
    pub degraded_capability_count: usize,
    pub unhealthy_capability_count: usize,
    pub overall_status: String,
    pub lowest_score: Option<i32>,
    pub latest_incident: Option<String>,
}

#[derive(Clone)]
pub struct PartnerRegistryService {
    repository: Option<Arc<dyn PartnerRegistryRepository>>,
}

impl PartnerRegistryService {
    pub fn new() -> Self {
        Self { repository: None }
    }

    pub fn with_pool(pool: PgPool) -> Self {
        Self {
            repository: Some(Arc::new(PgPartnerRegistryRepository::new(pool))),
        }
    }

    pub fn with_repository(repository: Arc<dyn PartnerRegistryRepository>) -> Self {
        Self {
            repository: Some(repository),
        }
    }

    pub async fn list_partners(&self, tenant_id: Option<&str>) -> Result<PartnerRegistrySnapshot> {
        if let Some(repository) = &self.repository {
            let partners = repository.list_registry_records(tenant_id).await?;
            if !partners.is_empty() {
                return Ok(PartnerRegistrySnapshot {
                    action_mode: "registry_backed".to_string(),
                    source: "registry".to_string(),
                    partners,
                });
            }
        }

        Ok(PartnerRegistrySnapshot {
            action_mode: "registry_backed".to_string(),
            source: "fallback".to_string(),
            partners: Vec::new(),
        })
    }

    /// Get a single partner by ID from the registry.
    pub async fn get_partner(
        &self,
        tenant_id: Option<&str>,
        partner_id: &str,
    ) -> Result<Option<PartnerRegistryRecord>> {
        let snapshot = self.list_partners(tenant_id).await?;
        Ok(snapshot
            .partners
            .into_iter()
            .find(|p| p.partner_id == partner_id))
    }

    /// Search partners using governance criteria (class, market, capability, approval state).
    pub async fn search_partners(
        &self,
        tenant_id: Option<&str>,
        criteria: &PartnerSearchCriteria,
    ) -> Result<PartnerRegistrySnapshot> {
        let snapshot = self.list_partners(tenant_id).await?;
        let filtered: Vec<PartnerRegistryRecord> = snapshot
            .partners
            .into_iter()
            .filter(|partner| {
                criteria
                    .partner_class
                    .as_ref()
                    .map_or(true, |cls| partner.partner_class.eq_ignore_ascii_case(cls))
                    && criteria
                        .market
                        .as_ref()
                        .map_or(true, |m| partner.market.as_deref() == Some(m.as_str()))
                    && criteria.approval_status.as_ref().map_or(true, |s| {
                        partner.approval_status.eq_ignore_ascii_case(s)
                    })
                    && criteria.lifecycle_state.as_ref().map_or(true, |s| {
                        partner.lifecycle_state.eq_ignore_ascii_case(s)
                    })
                    && criteria.service_domain.as_ref().map_or(true, |d| {
                        partner.service_domain.eq_ignore_ascii_case(d)
                    })
                    && criteria.capability_family.as_ref().map_or(true, |fam| {
                        partner.capabilities.iter().any(|cap| {
                            cap.capability_family.eq_ignore_ascii_case(fam)
                        })
                    })
            })
            .collect();

        Ok(PartnerRegistrySnapshot {
            action_mode: snapshot.action_mode,
            source: snapshot.source,
            partners: filtered,
        })
    }

    /// List only approved and active partners (governance convenience filter).
    pub async fn list_approved_partners(
        &self,
        tenant_id: Option<&str>,
    ) -> Result<PartnerRegistrySnapshot> {
        self.search_partners(
            tenant_id,
            &PartnerSearchCriteria {
                approval_status: Some("approved".to_string()),
                lifecycle_state: Some("active".to_string()),
                ..Default::default()
            },
        )
        .await
    }

    /// Aggregate health summary across all partners for governance dashboards.
    pub async fn aggregate_health_summary(
        &self,
        tenant_id: Option<&str>,
    ) -> Result<Vec<PartnerHealthSummary>> {
        let snapshot = self.list_partners(tenant_id).await?;
        Ok(snapshot
            .partners
            .into_iter()
            .map(|partner| {
                let capability_count = partner.capabilities.len();
                let mut healthy = 0usize;
                let mut degraded = 0usize;
                let mut unhealthy = 0usize;
                let mut lowest_score: Option<i32> = None;
                let mut latest_incident: Option<String> = None;

                for cap in &partner.capabilities {
                    let cap_status = cap
                        .health_signals
                        .first()
                        .map(|sig| sig.status.as_str())
                        .unwrap_or("unknown");
                    match cap_status {
                        "healthy" | "ok" => healthy += 1,
                        "degraded" | "warning" => degraded += 1,
                        _ => unhealthy += 1,
                    }
                    for signal in &cap.health_signals {
                        if let Some(score) = signal.score {
                            lowest_score = Some(
                                lowest_score.map_or(score, |current| current.min(score)),
                            );
                        }
                        // Always take the latest non-None incident summary
                        if signal.incident_summary.is_some() {
                            latest_incident = signal.incident_summary.clone();
                        }
                    }
                }

                let overall_status = if unhealthy > 0 {
                    "unhealthy"
                } else if degraded > 0 {
                    "degraded"
                } else {
                    "healthy"
                }
                .to_string();

                PartnerHealthSummary {
                    partner_id: partner.partner_id,
                    display_name: partner.display_name,
                    partner_class: partner.partner_class,
                    capability_count,
                    healthy_capability_count: healthy,
                    degraded_capability_count: degraded,
                    unhealthy_capability_count: unhealthy,
                    overall_status,
                    lowest_score,
                    latest_incident,
                }
            })
            .collect())
    }

    pub async fn upsert_partner_record(
        &self,
        request: &UpsertPartnerRegistryRecordRequest,
    ) -> Result<PartnerRegistrySnapshot> {
        let repository = self
            .repository
            .as_ref()
            .ok_or_else(|| ramp_common::Error::Internal("Partner registry repository is not configured".to_string()))?;

        for approval_reference in &request.approval_references {
            repository.upsert_approval_reference(approval_reference).await?;
        }

        repository.upsert_partner(&request.partner).await?;

        for capability_bundle in &request.capabilities {
            repository
                .upsert_capability(&capability_bundle.capability)
                .await?;

            for rollout_scope in &capability_bundle.rollout_scopes {
                repository.upsert_rollout_scope(rollout_scope).await?;
            }

            for health_signal in &capability_bundle.health_signals {
                repository.upsert_health_signal(health_signal).await?;
            }
        }

        for credential_reference in &request.credential_references {
            repository
                .upsert_credential_reference(credential_reference)
                .await?;
        }

        self.list_partners(request.partner.tenant_id.as_deref()).await
    }
}

impl Default for PartnerRegistryService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository::{
        PartnerCapabilityRecord, PartnerHealthSignalRecord,
    };
    use async_trait::async_trait;

    // ── Mock repository with in-memory partner data ──
    #[derive(Clone)]
    struct MockPartnerRepo {
        partners: Vec<PartnerRegistryRecord>,
    }

    #[async_trait]
    impl PartnerRegistryRepository for MockPartnerRepo {
        async fn list_registry_records(
            &self,
            _tenant_id: Option<&str>,
        ) -> Result<Vec<PartnerRegistryRecord>> {
            Ok(self.partners.clone())
        }
        async fn upsert_partner(&self, _req: &UpsertPartnerRequest) -> Result<()> { Ok(()) }
        async fn upsert_capability(&self, _req: &UpsertPartnerCapabilityRequest) -> Result<()> { Ok(()) }
        async fn upsert_rollout_scope(&self, _req: &UpsertPartnerRolloutScopeRequest) -> Result<()> { Ok(()) }
        async fn upsert_health_signal(&self, _req: &UpsertPartnerHealthSignalRequest) -> Result<()> { Ok(()) }
        async fn upsert_approval_reference(&self, _req: &UpsertApprovalReferenceRequest) -> Result<()> { Ok(()) }
        async fn upsert_credential_reference(&self, _req: &UpsertCredentialReferenceRequest) -> Result<()> { Ok(()) }
    }

    fn make_partner(
        id: &str,
        class: &str,
        market: Option<&str>,
        approval: &str,
        lifecycle: &str,
        caps: Vec<PartnerCapabilityRecord>,
    ) -> PartnerRegistryRecord {
        PartnerRegistryRecord {
            partner_id: id.to_string(),
            tenant_id: Some("tenant-1".to_string()),
            partner_class: class.to_string(),
            code: id.to_string(),
            display_name: format!("Partner {}", id),
            legal_name: None,
            market: market.map(|m| m.to_string()),
            jurisdiction: None,
            service_domain: "crypto".to_string(),
            lifecycle_state: lifecycle.to_string(),
            approval_status: approval.to_string(),
            metadata: serde_json::json!({}),
            capabilities: caps,
            credential_references: vec![],
        }
    }

    fn make_cap(family: &str, status: &str, score: Option<i32>) -> PartnerCapabilityRecord {
        PartnerCapabilityRecord {
            capability_id: format!("cap_{}", family),
            capability_family: family.to_string(),
            environment: "production".to_string(),
            adapter_key: None,
            provider_key: None,
            supported_rails: vec![],
            supported_methods: vec![],
            approval_status: "approved".to_string(),
            metadata: serde_json::json!({}),
            rollout_scopes: vec![],
            health_signals: vec![PartnerHealthSignalRecord {
                health_signal_id: "sig_1".to_string(),
                status: status.to_string(),
                source: "monitor".to_string(),
                score,
                incident_summary: if status == "degraded" {
                    Some("Latency spike".to_string())
                } else {
                    None
                },
                evidence: serde_json::json!({}),
                observed_at: chrono::Utc::now(),
            }],
        }
    }

    fn sample_partners() -> Vec<PartnerRegistryRecord> {
        vec![
            make_partner("p1", "payment_rail", Some("VN"), "approved", "active",
                vec![make_cap("settlement", "healthy", Some(95))]),
            make_partner("p2", "liquidity_provider", Some("VN"), "approved", "active",
                vec![make_cap("quoting", "degraded", Some(60))]),
            make_partner("p3", "custodian", Some("SG"), "pending", "onboarding",
                vec![make_cap("custody", "healthy", Some(80))]),
        ]
    }

    fn service_with_data() -> PartnerRegistryService {
        PartnerRegistryService::with_repository(Arc::new(MockPartnerRepo {
            partners: sample_partners(),
        }))
    }

    // ── Fallback tests ──

    #[tokio::test]
    async fn list_partners_returns_empty_fallback_without_repository() {
        let service = PartnerRegistryService::new();
        let snapshot = service.list_partners(Some("t")).await.expect("ok");
        assert_eq!(snapshot.source, "fallback");
        assert!(snapshot.partners.is_empty());
    }

    #[tokio::test]
    async fn get_partner_returns_none_without_repository() {
        let service = PartnerRegistryService::new();
        assert!(service.get_partner(Some("t"), "x").await.expect("ok").is_none());
    }

    // ── Mock-backed search/filter tests ──

    #[tokio::test]
    async fn get_partner_finds_by_id() {
        let service = service_with_data();
        let p = service.get_partner(None, "p2").await.expect("ok").expect("found");
        assert_eq!(p.partner_class, "liquidity_provider");
    }

    #[tokio::test]
    async fn get_partner_returns_none_for_unknown_id() {
        let service = service_with_data();
        assert!(service.get_partner(None, "p999").await.expect("ok").is_none());
    }

    #[tokio::test]
    async fn search_by_partner_class() {
        let service = service_with_data();
        let snap = service
            .search_partners(None, &PartnerSearchCriteria {
                partner_class: Some("payment_rail".to_string()),
                ..Default::default()
            })
            .await
            .expect("ok");
        assert_eq!(snap.partners.len(), 1);
        assert_eq!(snap.partners[0].partner_id, "p1");
    }

    #[tokio::test]
    async fn search_by_market() {
        let service = service_with_data();
        let snap = service
            .search_partners(None, &PartnerSearchCriteria {
                market: Some("SG".to_string()),
                ..Default::default()
            })
            .await
            .expect("ok");
        assert_eq!(snap.partners.len(), 1);
        assert_eq!(snap.partners[0].partner_id, "p3");
    }

    #[tokio::test]
    async fn search_by_capability_family() {
        let service = service_with_data();
        let snap = service
            .search_partners(None, &PartnerSearchCriteria {
                capability_family: Some("quoting".to_string()),
                ..Default::default()
            })
            .await
            .expect("ok");
        assert_eq!(snap.partners.len(), 1);
        assert_eq!(snap.partners[0].partner_id, "p2");
    }

    #[tokio::test]
    async fn search_combined_criteria() {
        let service = service_with_data();
        let snap = service
            .search_partners(None, &PartnerSearchCriteria {
                market: Some("VN".to_string()),
                approval_status: Some("approved".to_string()),
                ..Default::default()
            })
            .await
            .expect("ok");
        assert_eq!(snap.partners.len(), 2); // p1 and p2
    }

    #[tokio::test]
    async fn list_approved_filters_correctly() {
        let service = service_with_data();
        let snap = service.list_approved_partners(None).await.expect("ok");
        assert_eq!(snap.partners.len(), 2); // p1 and p2 are approved+active
        assert!(snap.partners.iter().all(|p| p.approval_status == "approved"));
    }

    #[tokio::test]
    async fn health_summary_aggregates_correctly() {
        let service = service_with_data();
        let summaries = service.aggregate_health_summary(None).await.expect("ok");
        assert_eq!(summaries.len(), 3);

        let p1_health = summaries.iter().find(|s| s.partner_id == "p1").unwrap();
        assert_eq!(p1_health.overall_status, "healthy");
        assert_eq!(p1_health.healthy_capability_count, 1);
        assert_eq!(p1_health.lowest_score, Some(95));

        let p2_health = summaries.iter().find(|s| s.partner_id == "p2").unwrap();
        assert_eq!(p2_health.overall_status, "degraded");
        assert_eq!(p2_health.degraded_capability_count, 1);
        assert_eq!(p2_health.lowest_score, Some(60));
        assert_eq!(p2_health.latest_incident.as_deref(), Some("Latency spike"));
    }
}
