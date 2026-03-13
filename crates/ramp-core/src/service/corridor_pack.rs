use ramp_common::Result;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::sync::Arc;

use crate::repository::{
    CorridorPackRecord, CorridorPackRepository, PgCorridorPackRepository,
    UpsertCorridorComplianceHookRequest, UpsertCorridorEligibilityRuleRequest,
    UpsertCorridorEndpointRequest, UpsertCorridorFeeProfileRequest, UpsertCorridorPackRequest,
    UpsertCorridorRolloutScopeRequest, UpsertCorridorCutoffPolicyRequest,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CorridorPackSnapshot {
    pub action_mode: String,
    pub source: String,
    pub corridor_packs: Vec<CorridorPackRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpsertCorridorPackBundle {
    pub corridor_pack: UpsertCorridorPackRequest,
    pub endpoints: Vec<UpsertCorridorEndpointRequest>,
    pub fee_profiles: Vec<UpsertCorridorFeeProfileRequest>,
    pub cutoff_policies: Vec<UpsertCorridorCutoffPolicyRequest>,
    pub compliance_hooks: Vec<UpsertCorridorComplianceHookRequest>,
    pub rollout_scopes: Vec<UpsertCorridorRolloutScopeRequest>,
    pub eligibility_rules: Vec<UpsertCorridorEligibilityRuleRequest>,
}

#[derive(Clone)]
pub struct CorridorPackService {
    repository: Option<Arc<dyn CorridorPackRepository>>,
}

impl CorridorPackService {
    pub fn new() -> Self {
        Self { repository: None }
    }

    pub fn with_pool(pool: PgPool) -> Self {
        Self {
            repository: Some(Arc::new(PgCorridorPackRepository::new(pool))),
        }
    }

    pub fn with_repository(repository: Arc<dyn CorridorPackRepository>) -> Self {
        Self {
            repository: Some(repository),
        }
    }

    pub async fn list_corridor_packs(&self, tenant_id: Option<&str>) -> Result<CorridorPackSnapshot> {
        if let Some(repository) = &self.repository {
            let corridor_packs = repository.list_corridor_packs(tenant_id).await?;
            if !corridor_packs.is_empty() {
                return Ok(CorridorPackSnapshot {
                    action_mode: "corridor_pack_registry".to_string(),
                    source: "registry".to_string(),
                    corridor_packs,
                });
            }
        }

        Ok(CorridorPackSnapshot {
            action_mode: "corridor_pack_registry".to_string(),
            source: "fallback".to_string(),
            corridor_packs: Vec::new(),
        })
    }

    pub async fn get_corridor_pack(
        &self,
        tenant_id: Option<&str>,
        corridor_code: &str,
    ) -> Result<Option<CorridorPackRecord>> {
        let Some(repository) = &self.repository else {
            return Ok(None);
        };
        repository.get_corridor_pack(tenant_id, corridor_code).await
    }

    pub async fn upsert_corridor_pack_bundle(
        &self,
        bundle: &UpsertCorridorPackBundle,
    ) -> Result<Option<CorridorPackRecord>> {
        let repository = self
            .repository
            .as_ref()
            .ok_or_else(|| ramp_common::Error::Internal("Corridor pack repository is not configured".to_string()))?;

        repository.upsert_corridor_pack(&bundle.corridor_pack).await?;
        for endpoint in &bundle.endpoints {
            repository.upsert_endpoint(endpoint).await?;
        }
        for fee_profile in &bundle.fee_profiles {
            repository.upsert_fee_profile(fee_profile).await?;
        }
        for cutoff_policy in &bundle.cutoff_policies {
            repository.upsert_cutoff_policy(cutoff_policy).await?;
        }
        for compliance_hook in &bundle.compliance_hooks {
            repository.upsert_compliance_hook(compliance_hook).await?;
        }
        for rollout_scope in &bundle.rollout_scopes {
            repository.upsert_rollout_scope(rollout_scope).await?;
        }
        for eligibility_rule in &bundle.eligibility_rules {
            repository.upsert_eligibility_rule(eligibility_rule).await?;
        }

        repository
            .get_corridor_pack(
                bundle.corridor_pack.tenant_id.as_deref(),
                &bundle.corridor_pack.corridor_code,
            )
            .await
    }
}

impl Default for CorridorPackService {
    fn default() -> Self {
        Self::new()
    }
}
