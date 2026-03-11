pub mod ekyc;
pub mod mock;
pub mod onfido;
pub mod tier;
pub mod workflow;

pub use ekyc::{
    EkycProviderType, EkycService, EkycVerificationRequest, EkycVerificationResponse,
    TenantEkycConfig, UserProvidedData,
};
pub use mock::{KycWorkflowState, MockKycConfig, MockKycProvider};
pub use onfido::OnfidoKycProvider;
pub use tier::{TierDataProvider, TierManager, UserKycInfo};

use async_trait::async_trait;
use ramp_common::{
    types::{TenantId, UserId},
    Result,
};
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::storage::{DocumentStorage, DocumentType, StorageResult};
use crate::types::{KycStatus, KycTier};

/// KYC verification request
#[derive(Debug, Clone)]
pub struct KycVerificationRequest {
    pub tenant_id: TenantId,
    pub user_id: UserId,
    pub tier: KycTier,
    pub full_name: String,
    pub date_of_birth: String,
    pub id_number: String,
    pub id_type: String, // CCCD, PASSPORT, etc.
    pub documents: Vec<KycDocument>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KycDocument {
    pub doc_type: String,
    pub file_hash: String,
    pub storage_url: String,
}

/// KYC verification result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KycVerificationResult {
    pub status: KycStatus,
    pub verified_tier: Option<KycTier>,
    pub rejection_reason: Option<String>,
    pub provider_reference: Option<String>,
}

#[async_trait]
pub trait KycProvider: Send + Sync {
    async fn verify(&self, request: &KycVerificationRequest) -> Result<KycVerificationResult>;
    async fn check_status(&self, reference: &str) -> Result<KycVerificationResult>;
}

/// KYC Service
pub struct KycService {
    provider: Box<dyn KycProvider>,
    storage: Box<dyn DocumentStorage>,
}

impl KycService {
    pub fn new(provider: Box<dyn KycProvider>, storage: Box<dyn DocumentStorage>) -> Self {
        Self { provider, storage }
    }

    /// Upload a KYC document
    pub async fn upload_document(
        &self,
        tenant_id: String,
        user_id: String,
        doc_type: DocumentType,
        data: Vec<u8>,
        extension: &str,
    ) -> StorageResult<String> {
        self.storage
            .upload(tenant_id, user_id, doc_type, data, extension)
            .await
    }

    /// Download a KYC document
    pub async fn download_document(&self, url: &str) -> StorageResult<Vec<u8>> {
        self.storage.download(url).await
    }

    /// Generate a presigned URL for a KYC document
    pub async fn get_document_presigned_url(
        &self,
        url: &str,
        expiry: std::time::Duration,
    ) -> StorageResult<String> {
        // Cap presigned URL expiry to 1 hour maximum to limit exposure
        const MAX_EXPIRY: std::time::Duration = std::time::Duration::from_secs(3600);
        let capped_expiry = if expiry > MAX_EXPIRY {
            MAX_EXPIRY
        } else {
            expiry
        };
        self.storage
            .generate_presigned_url(url, capped_expiry)
            .await
    }

    /// Submit KYC verification
    pub async fn submit_verification(
        &self,
        request: KycVerificationRequest,
    ) -> Result<KycVerificationResult> {
        info!(
            tenant_id = %request.tenant_id,
            user_id = %request.user_id,
            tier = ?request.tier,
            "Submitting KYC verification"
        );

        let result = self.provider.verify(&request).await?;

        info!(
            user_id = %request.user_id,
            status = ?result.status,
            "KYC verification completed"
        );

        Ok(result)
    }

    /// Check verification status
    pub async fn check_status(&self, reference: &str) -> Result<KycVerificationResult> {
        self.provider.check_status(reference).await
    }

    /// Get limits for a KYC tier
    pub fn get_tier_limits(&self, tier: KycTier) -> TierLimits {
        TierLimits {
            tier,
            daily_payin_limit_vnd: tier.daily_payin_limit_vnd(),
            daily_payout_limit_vnd: tier.daily_payout_limit_vnd(),
            single_transaction_limit_vnd: tier.single_transaction_limit_vnd(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TierLimits {
    pub tier: KycTier,
    pub daily_payin_limit_vnd: rust_decimal::Decimal,
    pub daily_payout_limit_vnd: rust_decimal::Decimal,
    pub single_transaction_limit_vnd: rust_decimal::Decimal,
}
