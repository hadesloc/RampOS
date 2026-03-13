use ramp_common::Result;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::sync::Arc;

use crate::repository::{
    PaymentMethodCapabilityRecord, PaymentMethodCapabilityRepository,
    PgPaymentMethodCapabilityRepository, UpsertPaymentMethodCapabilityRequest,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentMethodCapabilitySnapshot {
    pub action_mode: String,
    pub source: String,
    pub capabilities: Vec<PaymentMethodCapabilityRecord>,
}

#[derive(Clone)]
pub struct PaymentMethodCapabilityService {
    repository: Option<Arc<dyn PaymentMethodCapabilityRepository>>,
}

impl PaymentMethodCapabilityService {
    pub fn new() -> Self {
        Self { repository: None }
    }

    pub fn with_pool(pool: PgPool) -> Self {
        Self {
            repository: Some(Arc::new(PgPaymentMethodCapabilityRepository::new(pool))),
        }
    }

    pub fn with_repository(repository: Arc<dyn PaymentMethodCapabilityRepository>) -> Self {
        Self {
            repository: Some(repository),
        }
    }

    pub async fn list_capabilities(
        &self,
        corridor_pack_id: Option<&str>,
        partner_capability_id: Option<&str>,
    ) -> Result<PaymentMethodCapabilitySnapshot> {
        if let Some(repository) = &self.repository {
            let capabilities = repository
                .list_payment_method_capabilities(corridor_pack_id, partner_capability_id)
                .await?;
            if !capabilities.is_empty() {
                return Ok(PaymentMethodCapabilitySnapshot {
                    action_mode: "registry_backed".to_string(),
                    source: "registry".to_string(),
                    capabilities,
                });
            }
        }

        Ok(PaymentMethodCapabilitySnapshot {
            action_mode: "registry_backed".to_string(),
            source: "fallback".to_string(),
            capabilities: Vec::new(),
        })
    }

    pub async fn upsert_capability(
        &self,
        request: &UpsertPaymentMethodCapabilityRequest,
    ) -> Result<PaymentMethodCapabilitySnapshot> {
        let repository = self.repository.as_ref().ok_or_else(|| {
            ramp_common::Error::Internal(
                "Payment method capability repository is not configured".to_string(),
            )
        })?;

        repository.upsert_payment_method_capability(request).await?;
        self.list_capabilities(
            Some(&request.corridor_pack_id),
            request.partner_capability_id.as_deref(),
        )
        .await
    }
}

impl Default for PaymentMethodCapabilityService {
    fn default() -> Self {
        Self::new()
    }
}
