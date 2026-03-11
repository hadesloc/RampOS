use chrono::Utc;
use ramp_common::{types::TenantId, Result};
use rust_decimal::Decimal;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::sync::Arc;
use std::{net::IpAddr, str::FromStr};
use uuid::Uuid;

use crate::repository::tenant::{TenantRepository, TenantRow};
use crate::service::crypto::encode_secret_for_storage;
use crate::service::ledger::LedgerService;

/// API credentials (api_key for Bearer auth + api_secret for HMAC signing)
#[derive(Debug, Clone, Serialize)]
pub struct ApiCredentials {
    /// API key for Bearer authentication (sent in Authorization header)
    pub api_key: String,
    /// API secret for HMAC signature verification (used to sign requests)
    pub api_secret: String,
}

/// API key pair (public key + secret key) - DEPRECATED, use ApiCredentials
#[derive(Debug, Clone, Serialize)]
pub struct ApiKeyPair {
    pub public_key: String,
    pub secret_key: String,
}

#[derive(Debug, Clone)]
pub struct TenantBootstrapRequest {
    pub name: String,
    pub config: serde_json::Value,
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
        self.bootstrap_tenant(TenantBootstrapRequest {
            name: name.to_string(),
            config,
        })
        .await
    }

    /// Shared bootstrap path for admin onboarding and sandbox seeding.
    pub async fn bootstrap_tenant(&self, request: TenantBootstrapRequest) -> Result<TenantRow> {
        let tenant_id = TenantId::new(format!("t_{}", Uuid::now_v7()));
        let now = Utc::now();

        // Generate initial API credentials (will be replaced by generate_api_credentials)
        let (credentials, api_key_hash, api_secret_encrypted) =
            self.generate_api_credentials_internal()?;

        // Generate webhook secret (will be exposed to user separately)
        let (webhook_secret_encrypted, webhook_secret_hash) = self.generate_secret_internal()?;

        let tenant = TenantRow {
            id: tenant_id.0.clone(),
            name: request.name.clone(),
            status: "PENDING".to_string(),
            api_key_hash,
            // Store the encrypted API secret for HMAC verification
            // In production, this should be encrypted with a proper encryption key
            api_secret_encrypted: Some(api_secret_encrypted),
            webhook_secret_hash,
            webhook_secret_encrypted: Some(webhook_secret_encrypted),
            webhook_url: None,
            config: request.config,
            daily_payin_limit_vnd: None,
            daily_payout_limit_vnd: None,
            api_version: None,
            created_at: now,
            updated_at: now,
        };

        self.tenant_repo.create(&tenant).await?;

        // Log that credentials were generated (don't log the actual secrets!)
        tracing::info!(
            tenant_id = %tenant_id,
            "Created tenant with API credentials (api_key: {}...)",
            &credentials.api_key[..12]
        );

        Ok(tenant)
    }

    /// Generate new API credentials for a tenant
    /// Returns ApiCredentials containing api_key (for Bearer) and api_secret (for HMAC)
    pub async fn generate_api_credentials(&self, tenant_id: &TenantId) -> Result<ApiCredentials> {
        let (credentials, api_key_hash, api_secret_encrypted) =
            self.generate_api_credentials_internal()?;
        self.tenant_repo
            .update_api_credentials(tenant_id, &api_key_hash, &api_secret_encrypted)
            .await?;

        tracing::info!(
            %tenant_id,
            "Regenerated API credentials (api_key: {}...)",
            &credentials.api_key[..12]
        );

        Ok(credentials)
    }

    /// Generate new API keys for a tenant (DEPRECATED: use generate_api_credentials)
    #[deprecated(note = "Use generate_api_credentials instead")]
    pub async fn generate_api_keys(&self, tenant_id: &TenantId) -> Result<ApiKeyPair> {
        let credentials = self.generate_api_credentials(tenant_id).await?;
        Ok(ApiKeyPair {
            public_key: credentials.api_key,
            secret_key: credentials.api_secret,
        })
    }

    /// Configure webhook URL
    pub async fn configure_webhooks(&self, tenant_id: &TenantId, url: &str) -> Result<()> {
        validate_webhook_url(url)?;
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

    /// Generate API credentials for SDK authentication
    /// Returns: (ApiCredentials, api_key_hash, api_secret_encrypted)
    fn generate_api_credentials_internal(&self) -> Result<(ApiCredentials, String, Vec<u8>)> {
        // api_key: Used in Bearer token for authentication
        let api_key = format!("ramp_{}", Uuid::new_v4().simple());
        // api_secret: Used for HMAC signing
        let api_secret = format!("ramp_secret_{}", Uuid::new_v4().simple());

        // Hash the api_key for storage/lookup
        let mut hasher = Sha256::new();
        hasher.update(api_key.as_bytes());
        let api_key_hash = hex::encode(hasher.finalize());

        // In production, this should use proper encryption (AES-GCM, etc.)
        // For now, we store it as bytes (simulating encryption)
        // TODO: Implement proper encryption using application-level encryption key
        let api_secret_encrypted = encrypt_secret_for_storage(api_secret.as_bytes())?;

        Ok((
            ApiCredentials {
                api_key,
                api_secret,
            },
            api_key_hash,
            api_secret_encrypted,
        ))
    }

    #[allow(dead_code)]
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

    fn generate_secret_internal(&self) -> Result<(Vec<u8>, String)> {
        let secret = format!("whsec_{}", Uuid::new_v4().simple());

        let mut hasher = Sha256::new();
        hasher.update(secret.as_bytes());
        let hash = hex::encode(hasher.finalize());

        Ok((encrypt_secret_for_storage(secret.as_bytes())?, hash))
    }
}

fn encrypt_secret_for_storage(secret: &[u8]) -> Result<Vec<u8>> {
    encode_secret_for_storage(secret)
}

pub(crate) fn validate_webhook_url(url: &str) -> Result<()> {
    let parsed = reqwest::Url::parse(url).map_err(|_| {
        ramp_common::Error::Validation("Webhook URL must be a valid absolute URL".to_string())
    })?;

    if parsed.scheme() != "https" {
        return Err(ramp_common::Error::Validation(
            "Webhook URL must use https".to_string(),
        ));
    }

    if !parsed.username().is_empty() || parsed.password().is_some() {
        return Err(ramp_common::Error::Validation(
            "Webhook URL must not include embedded credentials".to_string(),
        ));
    }

    let host = parsed.host_str().ok_or_else(|| {
        ramp_common::Error::Validation("Webhook URL must include a host".to_string())
    })?;

    if host.eq_ignore_ascii_case("localhost") || host.ends_with(".localhost") {
        return Err(ramp_common::Error::Validation(
            "Webhook URL host is not allowed".to_string(),
        ));
    }

    if let Ok(ip) = IpAddr::from_str(host) {
        let blocked = match ip {
            IpAddr::V4(addr) => {
                addr.is_private()
                    || addr.is_loopback()
                    || addr.is_link_local()
                    || addr.is_broadcast()
                    || addr.is_unspecified()
                    || addr.is_documentation()
            }
            IpAddr::V6(addr) => {
                addr.is_loopback()
                    || addr.is_unspecified()
                    || addr.is_unique_local()
                    || addr.is_unicast_link_local()
            }
        };

        if blocked {
            return Err(ramp_common::Error::Validation(
                "Webhook URL host is not allowed".to_string(),
            ));
        }
    }

    Ok(())
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
    async fn test_bootstrap_tenant_preserves_config_for_reuse_paths() {
        let tenant_repo = Arc::new(MockTenantRepository::new());
        let ledger_repo = Arc::new(MockLedgerRepository::new());
        let ledger_service = Arc::new(LedgerService::new(ledger_repo));
        let service = OnboardingService::new(tenant_repo.clone(), ledger_service);

        let tenant = service
            .bootstrap_tenant(TenantBootstrapRequest {
                name: "Sandbox Tenant".to_string(),
                config: serde_json::json!({
                    "environment": "sandbox",
                    "sandbox": {
                        "preset_code": "BASELINE"
                    }
                }),
            })
            .await
            .unwrap();

        let stored = tenant_repo
            .get_by_id(&TenantId::new(tenant.id.clone()))
            .await
            .unwrap()
            .unwrap();

        assert_eq!(stored.name, "Sandbox Tenant");
        assert_eq!(stored.config["environment"], "sandbox");
        assert_eq!(stored.config["sandbox"]["preset_code"], "BASELINE");
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
        assert!(keys.public_key.starts_with("ramp_"));
        assert!(keys.secret_key.starts_with("ramp_secret_"));
    }

    #[tokio::test]
    async fn test_configure_webhooks_rejects_loopback_target() {
        let tenant_repo = Arc::new(MockTenantRepository::new());
        let ledger_repo = Arc::new(MockLedgerRepository::new());
        let ledger_service = Arc::new(LedgerService::new(ledger_repo));
        let service = OnboardingService::new(tenant_repo.clone(), ledger_service);

        let tenant = service
            .create_tenant("Webhook Validation Tenant", serde_json::json!({}))
            .await
            .unwrap();
        let tenant_id = TenantId::new(tenant.id);

        let err = service
            .configure_webhooks(&tenant_id, "http://127.0.0.1:8080/internal")
            .await
            .unwrap_err();

        assert!(format!("{err}").contains("Webhook URL"));
    }

    #[test]
    fn test_validate_webhook_url_rejects_private_and_credentialed_targets() {
        let private_err = validate_webhook_url("https://10.0.0.5/hook").unwrap_err();
        assert!(format!("{private_err}").contains("Webhook URL host is not allowed"));

        let credential_err =
            validate_webhook_url("https://user:pass@example.com/hook").unwrap_err();
        assert!(format!("{credential_err}").contains("embedded credentials"));
    }
}
