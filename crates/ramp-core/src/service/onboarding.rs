use chrono::Utc;
use ramp_common::{types::TenantId, Result};
use rust_decimal::Decimal;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::sync::Arc;
use uuid::Uuid;

use crate::repository::tenant::{TenantRepository, TenantRow};
use crate::service::ledger::LedgerService;

/// API key pair (public key + secret key)
#[derive(Debug, Clone, Serialize)]
pub struct ApiKeyPair {
    pub public_key: String,
    pub secret_key: String,
}

/// Onboarding service for managing tenant lifecycle
pub struct OnboardingService {
    tenant_repo: Arc<dyn TenantRepository>,
    _ledger_service: Arc<LedgerService>,
}

impl OnboardingService {
    pub fn new(tenant_repo: Arc<dyn TenantRepository>, ledger_service: Arc<LedgerService>) -> Self {
        Self {
            tenant_repo,
            _ledger_service: ledger_service,
        }
    }

    /// Create a new tenant record
    pub async fn create_tenant(&self, name: &str, config: serde_json::Value) -> Result<TenantRow> {
        let tenant_id = TenantId::new(format!("t_{}", Uuid::now_v7()));
        let now = Utc::now();

        // Generate initial API key (will be replaced by generate_api_keys)
        let (_api_key_pair, api_key_hash) = self.generate_api_key_internal();

        // Generate webhook secret (will be exposed to user separately)
        let (webhook_secret, webhook_secret_hash) = self.generate_secret_internal();

        let tenant = TenantRow {
            id: tenant_id.0.clone(),
            name: name.to_string(),
            status: "PENDING".to_string(),
            api_key_hash,
            webhook_secret_hash,
            // Store the encrypted webhook secret for HMAC signing
            // In production, this should be encrypted with a proper encryption key
            webhook_secret_encrypted: Some(webhook_secret.as_bytes().to_vec()),
            webhook_url: None,
            config,
            daily_payin_limit_vnd: None,
            daily_payout_limit_vnd: None,
            created_at: now,
            updated_at: now,
        };

        self.tenant_repo.create(&tenant).await?;

        Ok(tenant)
    }

    /// Generate new API keys for a tenant
    pub async fn generate_api_keys(&self, tenant_id: &TenantId) -> Result<ApiKeyPair> {
        let (key_pair, hash) = self.generate_api_key_internal();
        self.tenant_repo
            .update_api_key_hash(tenant_id, &hash)
            .await?;
        Ok(key_pair)
    }

    /// Configure webhook URL
    pub async fn configure_webhooks(&self, tenant_id: &TenantId, url: &str) -> Result<()> {
        self.tenant_repo.update_webhook_url(tenant_id, url).await
    }

    /// Set daily limits
    pub async fn set_limits(
        &self,
        tenant_id: &TenantId,
        daily_payin: Option<Decimal>,
        daily_payout: Option<Decimal>,
    ) -> Result<()> {
        self.tenant_repo
            .update_limits(tenant_id, daily_payin, daily_payout)
            .await
    }

    /// Activate a tenant and create default ledger accounts
    pub async fn activate_tenant(&self, tenant_id: &TenantId) -> Result<()> {
        // 1. Check if tenant exists
        let tenant = self.tenant_repo.get_by_id(tenant_id).await?;
        if tenant.is_none() {
            return Err(ramp_common::Error::NotFound("Tenant not found".to_string()));
        }

        // 2. Initialize ledger accounts (if not already done)
        // We do this by creating a zero-value transaction that touches all necessary accounts
        // or by explicitly creating accounts if the ledger supported it.
        // For now, we'll rely on the ledger creating accounts on first use,
        // but we might want to "seed" the system with an initial deposit if needed.

        // 3. Update status to ACTIVE
        self.tenant_repo.update_status(tenant_id, "ACTIVE").await?;

        Ok(())
    }

    /// Suspend a tenant
    pub async fn suspend_tenant(&self, tenant_id: &TenantId, reason: &str) -> Result<()> {
        // Log the reason (could be extended to audit log)
        tracing::info!(%tenant_id, reason, "Suspending tenant");

        self.tenant_repo.update_status(tenant_id, "SUSPENDED").await
    }

    // --- Helper functions ---

    fn generate_api_key_internal(&self) -> (ApiKeyPair, String) {
        let public_key = format!("pk_{}", Uuid::new_v4().simple());
        let secret_key = format!("sk_{}", Uuid::new_v4().simple());

        let mut hasher = Sha256::new();
        hasher.update(secret_key.as_bytes());
        let hash = hex::encode(hasher.finalize());

        (
            ApiKeyPair {
                public_key,
                secret_key,
            },
            hash,
        )
    }

    fn generate_secret_internal(&self) -> (String, String) {
        let secret = format!("whsec_{}", Uuid::new_v4().simple());

        let mut hasher = Sha256::new();
        hasher.update(secret.as_bytes());
        let hash = hex::encode(hasher.finalize());

        (secret, hash)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::{MockLedgerRepository, MockTenantRepository};
    use rust_decimal_macros::dec;

    #[tokio::test]
    async fn test_create_tenant() {
        let tenant_repo = Arc::new(MockTenantRepository::new());
        let ledger_repo = Arc::new(MockLedgerRepository::new());
        let ledger_service = Arc::new(LedgerService::new(ledger_repo));
        let service = OnboardingService::new(tenant_repo.clone(), ledger_service);

        let tenant = service
            .create_tenant("Test Tenant", serde_json::json!({}))
            .await
            .unwrap();

        assert_eq!(tenant.name, "Test Tenant");
        assert_eq!(tenant.status, "PENDING");
        assert!(tenant.id.starts_with("t_"));
    }

    #[tokio::test]
    async fn test_generate_api_keys() {
        let tenant_repo = Arc::new(MockTenantRepository::new());
        let ledger_repo = Arc::new(MockLedgerRepository::new());
        let ledger_service = Arc::new(LedgerService::new(ledger_repo));
        let service = OnboardingService::new(tenant_repo.clone(), ledger_service);

        let tenant = service
            .create_tenant("Test Tenant", serde_json::json!({}))
            .await
            .unwrap();
        let tenant_id = TenantId::new(tenant.id);

        let keys = service.generate_api_keys(&tenant_id).await.unwrap();
        assert!(keys.public_key.starts_with("pk_"));
        assert!(keys.secret_key.starts_with("sk_"));
    }
}
