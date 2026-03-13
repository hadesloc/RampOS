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

    #[tokio::test]
    async fn list_partners_returns_empty_fallback_without_repository() {
        let service = PartnerRegistryService::new();
        let snapshot = service
            .list_partners(Some("tenant-test"))
            .await
            .expect("fallback snapshot");

        assert_eq!(snapshot.action_mode, "registry_backed");
        assert_eq!(snapshot.source, "fallback");
        assert!(snapshot.partners.is_empty());
    }
}
