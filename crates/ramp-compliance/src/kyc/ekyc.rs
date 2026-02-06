//! eKYC Integration Service
//!
//! This module provides integration with Vietnam eKYC providers (VNPay, FPT.AI)
//! for the KYC verification flow.

use async_trait::async_trait;
use ramp_common::{Error, Result};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, info, warn};

use super::{KycProvider, KycVerificationRequest, KycVerificationResult};
use crate::types::{KycStatus, KycTier};

// Re-export eKYC types for convenience
pub use ramp_adapter::{
    EkycProvider as EkycProviderTrait, EkycProviderConfig, FaceMatch, FaceMatchConfidence,
    FaceMatchRequest, FptAiEkycProvider, FullEkycResult, IdDocumentType, IdVerification,
    IdVerificationRequest, LivenessCheckType, LivenessRequest, LivenessResult, MockEkycProvider,
    SpoofingType, VnpayEkycProvider,
};

/// eKYC provider selection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum EkycProviderType {
    /// VNPay eKYC
    Vnpay,
    /// FPT.AI eKYC
    FptAi,
    /// Mock provider for testing
    #[default]
    Mock,
}

impl EkycProviderType {
    pub fn as_str(&self) -> &'static str {
        match self {
            EkycProviderType::Vnpay => "vnpay",
            EkycProviderType::FptAi => "fpt_ai",
            EkycProviderType::Mock => "mock",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "vnpay" | "vnpay_ekyc" => EkycProviderType::Vnpay,
            "fpt" | "fpt_ai" | "fptai" | "fpt_ai_ekyc" => EkycProviderType::FptAi,
            _ => EkycProviderType::Mock,
        }
    }
}

/// Tenant eKYC configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantEkycConfig {
    /// Primary eKYC provider
    pub primary_provider: EkycProviderType,
    /// Fallback provider (used when primary fails)
    pub fallback_provider: Option<EkycProviderType>,
    /// Whether to enable liveness detection
    pub enable_liveness: bool,
    /// Liveness check type
    pub liveness_check_type: LivenessCheckType,
    /// Minimum face match similarity score (0.0 - 1.0)
    pub min_face_similarity: f64,
    /// Minimum overall confidence score (0.0 - 1.0)
    pub min_confidence_score: f64,
    /// Whether to fall back to manual review on provider failure
    pub fallback_to_manual_on_failure: bool,
    /// Maximum retry attempts for transient failures
    pub max_retries: u32,
    /// Retry delay in milliseconds
    pub retry_delay_ms: u64,
}

impl Default for TenantEkycConfig {
    fn default() -> Self {
        Self {
            primary_provider: EkycProviderType::Mock,
            fallback_provider: None,
            enable_liveness: true,
            liveness_check_type: LivenessCheckType::Active,
            min_face_similarity: 0.75,
            min_confidence_score: 0.70,
            fallback_to_manual_on_failure: true,
            max_retries: 3,
            retry_delay_ms: 1000,
        }
    }
}

/// eKYC verification request with images
#[derive(Debug, Clone)]
pub struct EkycVerificationRequest {
    /// Tenant ID
    pub tenant_id: String,
    /// User ID
    pub user_id: String,
    /// ID document front image
    pub id_front_image: Vec<u8>,
    /// ID document back image (optional for passport)
    pub id_back_image: Option<Vec<u8>>,
    /// ID document type
    pub id_document_type: IdDocumentType,
    /// Selfie image for face matching
    pub selfie_image: Vec<u8>,
    /// Video/frames for liveness detection
    pub liveness_data: Option<Vec<u8>>,
    /// Requested KYC tier
    pub requested_tier: KycTier,
    /// User-provided data for verification
    pub user_provided_data: UserProvidedData,
}

/// User-provided data for cross-verification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProvidedData {
    pub full_name: String,
    pub id_number: String,
    pub date_of_birth: String,
    pub address: Option<String>,
}

/// eKYC verification result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EkycVerificationResponse {
    /// Unique verification reference
    pub reference_id: String,
    /// Overall verification status
    pub status: KycStatus,
    /// Verified tier (if approved)
    pub verified_tier: Option<KycTier>,
    /// Provider used
    pub provider: EkycProviderType,
    /// Detailed results
    pub details: EkycVerificationDetails,
    /// Rejection reasons (if any)
    pub rejection_reasons: Vec<String>,
    /// Whether manual review is needed
    pub needs_manual_review: bool,
}

/// Detailed eKYC verification results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EkycVerificationDetails {
    /// ID verification result
    pub id_verification: Option<IdVerificationSummary>,
    /// Face match result
    pub face_match: Option<FaceMatchSummary>,
    /// Liveness result
    pub liveness: Option<LivenessSummary>,
    /// Data cross-verification result
    pub data_verification: Option<DataVerificationResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdVerificationSummary {
    pub success: bool,
    pub extracted_name: Option<String>,
    pub extracted_id_number: Option<String>,
    pub confidence_score: f64,
    pub provider_reference: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaceMatchSummary {
    pub is_match: bool,
    pub similarity_score: f64,
    pub confidence: FaceMatchConfidence,
    pub provider_reference: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LivenessSummary {
    pub is_live: bool,
    pub score: f64,
    pub spoofing_detected: Vec<SpoofingType>,
    pub provider_reference: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataVerificationResult {
    pub name_matches: bool,
    pub id_number_matches: bool,
    pub dob_matches: bool,
    pub overall_match: bool,
}

/// eKYC Service for managing provider integrations
pub struct EkycService {
    /// Available providers
    mock_provider: Arc<MockEkycProvider>,
    vnpay_provider: Option<Arc<dyn EkycProviderTrait>>,
    fpt_ai_provider: Option<Arc<dyn EkycProviderTrait>>,
}

impl EkycService {
    /// Create a new eKYC service with mock provider only
    pub fn new_mock() -> Self {
        Self {
            mock_provider: Arc::new(MockEkycProvider::with_defaults()),
            vnpay_provider: None,
            fpt_ai_provider: None,
        }
    }

    /// Create eKYC service with configured providers
    pub fn new(
        vnpay_provider: Option<Arc<dyn EkycProviderTrait>>,
        fpt_ai_provider: Option<Arc<dyn EkycProviderTrait>>,
    ) -> Self {
        Self {
            mock_provider: Arc::new(MockEkycProvider::with_defaults()),
            vnpay_provider,
            fpt_ai_provider,
        }
    }

    /// Get provider by type
    fn get_provider(&self, provider_type: EkycProviderType) -> Arc<dyn EkycProviderTrait> {
        match provider_type {
            EkycProviderType::Vnpay => self
                .vnpay_provider
                .clone()
                .unwrap_or_else(|| self.mock_provider.clone()),
            EkycProviderType::FptAi => self
                .fpt_ai_provider
                .clone()
                .unwrap_or_else(|| self.mock_provider.clone()),
            EkycProviderType::Mock => self.mock_provider.clone(),
        }
    }

    /// Perform full eKYC verification
    pub async fn verify(
        &self,
        request: EkycVerificationRequest,
        config: &TenantEkycConfig,
    ) -> Result<EkycVerificationResponse> {
        let reference_id = format!(
            "ekyc_{}_{}_{}",
            request.tenant_id,
            request.user_id,
            chrono::Utc::now().timestamp_millis()
        );

        info!(
            reference_id = %reference_id,
            tenant_id = %request.tenant_id,
            user_id = %request.user_id,
            provider = ?config.primary_provider,
            "Starting eKYC verification"
        );

        // Try primary provider first
        let result = self
            .verify_with_provider(
                &request,
                config,
                config.primary_provider,
                &reference_id,
            )
            .await;

        // If primary fails and fallback is configured, try fallback
        let result = match result {
            Ok(r) => r,
            Err(e) => {
                if let Some(fallback) = config.fallback_provider {
                    warn!(
                        reference_id = %reference_id,
                        error = %e,
                        fallback = ?fallback,
                        "Primary provider failed, trying fallback"
                    );
                    self.verify_with_provider(&request, config, fallback, &reference_id)
                        .await?
                } else if config.fallback_to_manual_on_failure {
                    warn!(
                        reference_id = %reference_id,
                        error = %e,
                        "Provider failed, falling back to manual review"
                    );
                    return Ok(EkycVerificationResponse {
                        reference_id,
                        status: KycStatus::Pending,
                        verified_tier: None,
                        provider: config.primary_provider,
                        details: EkycVerificationDetails {
                            id_verification: None,
                            face_match: None,
                            liveness: None,
                            data_verification: None,
                        },
                        rejection_reasons: vec![format!("Provider error: {}", e)],
                        needs_manual_review: true,
                    });
                } else {
                    return Err(e);
                }
            }
        };

        Ok(result)
    }

    /// Verify with a specific provider
    async fn verify_with_provider(
        &self,
        request: &EkycVerificationRequest,
        config: &TenantEkycConfig,
        provider_type: EkycProviderType,
        reference_id: &str,
    ) -> Result<EkycVerificationResponse> {
        let provider = self.get_provider(provider_type);
        let mut rejection_reasons = Vec::new();
        let mut needs_manual_review = false;

        // Step 1: ID Verification (OCR)
        let id_request = IdVerificationRequest {
            id_front_image: request.id_front_image.clone(),
            id_back_image: request.id_back_image.clone(),
            document_type: request.id_document_type,
            request_id: format!("{}_id", reference_id),
        };

        let id_result = self
            .retry_with_backoff(config, || provider.verify_id(id_request.clone()))
            .await?;

        let id_summary = IdVerificationSummary {
            success: id_result.success,
            extracted_name: id_result.full_name.clone(),
            extracted_id_number: id_result.id_number.clone(),
            confidence_score: id_result.confidence_score,
            provider_reference: id_result.verification_id.clone(),
        };

        if !id_result.success {
            rejection_reasons.push(format!(
                "ID verification failed: {}",
                id_result.error_message.as_deref().unwrap_or("Unknown error")
            ));
        } else if id_result.confidence_score < config.min_confidence_score {
            rejection_reasons.push(format!(
                "ID confidence score {} below threshold {}",
                id_result.confidence_score, config.min_confidence_score
            ));
            needs_manual_review = true;
        }

        // Step 2: Face Matching
        let face_request = FaceMatchRequest {
            selfie_image: request.selfie_image.clone(),
            id_photo_image: request.id_front_image.clone(), // Extract face from ID
            request_id: format!("{}_face", reference_id),
        };

        let face_result = self
            .retry_with_backoff(config, || provider.match_face(face_request.clone()))
            .await?;

        let face_summary = FaceMatchSummary {
            is_match: face_result.is_match,
            similarity_score: face_result.similarity_score,
            confidence: face_result.confidence.clone(),
            provider_reference: face_result.match_id.clone(),
        };

        if !face_result.is_match {
            rejection_reasons.push(format!(
                "Face match failed: similarity {}%",
                (face_result.similarity_score * 100.0) as u32
            ));
        } else if face_result.similarity_score < config.min_face_similarity {
            rejection_reasons.push(format!(
                "Face similarity {} below threshold {}",
                face_result.similarity_score, config.min_face_similarity
            ));
            needs_manual_review = true;
        }

        // Step 3: Liveness Detection (if enabled)
        let liveness_summary = if config.enable_liveness {
            let liveness_data = request.liveness_data.clone().unwrap_or_else(|| {
                // Use selfie as fallback for passive liveness
                request.selfie_image.clone()
            });

            let liveness_request = LivenessRequest {
                video_data: liveness_data,
                request_id: format!("{}_live", reference_id),
                check_type: config.liveness_check_type.clone(),
            };

            let liveness_result = self
                .retry_with_backoff(config, || provider.check_liveness(liveness_request.clone()))
                .await?;

            let summary = LivenessSummary {
                is_live: liveness_result.is_live,
                score: liveness_result.liveness_score,
                spoofing_detected: liveness_result.spoofing_types.clone(),
                provider_reference: liveness_result.liveness_id.clone(),
            };

            if !liveness_result.is_live {
                let spoofing_types: Vec<_> = liveness_result
                    .spoofing_types
                    .iter()
                    .map(|t| format!("{:?}", t))
                    .collect();
                if spoofing_types.is_empty() {
                    rejection_reasons.push("Liveness check failed".to_string());
                } else {
                    rejection_reasons.push(format!(
                        "Liveness failed: detected {}",
                        spoofing_types.join(", ")
                    ));
                }
            }

            Some(summary)
        } else {
            None
        };

        // Step 4: Cross-verify user-provided data with OCR results
        let data_verification = Self::verify_user_data(
            &request.user_provided_data,
            &id_result,
        );

        if !data_verification.overall_match {
            if !data_verification.name_matches {
                rejection_reasons.push("Name mismatch between provided data and ID".to_string());
            }
            if !data_verification.id_number_matches {
                rejection_reasons
                    .push("ID number mismatch between provided data and document".to_string());
            }
            if !data_verification.dob_matches {
                rejection_reasons
                    .push("Date of birth mismatch between provided data and ID".to_string());
            }
            needs_manual_review = true;
        }

        // Determine final status
        let (status, verified_tier) = if rejection_reasons.is_empty() {
            (KycStatus::Approved, Some(request.requested_tier))
        } else if needs_manual_review {
            (KycStatus::Pending, None)
        } else {
            (KycStatus::Rejected, None)
        };

        info!(
            reference_id = %reference_id,
            status = ?status,
            rejection_count = rejection_reasons.len(),
            "eKYC verification completed"
        );

        Ok(EkycVerificationResponse {
            reference_id: reference_id.to_string(),
            status,
            verified_tier,
            provider: provider_type,
            details: EkycVerificationDetails {
                id_verification: Some(id_summary),
                face_match: Some(face_summary),
                liveness: liveness_summary,
                data_verification: Some(data_verification),
            },
            rejection_reasons,
            needs_manual_review,
        })
    }

    /// Verify user-provided data against OCR results
    fn verify_user_data(
        user_data: &UserProvidedData,
        id_result: &IdVerification,
    ) -> DataVerificationResult {
        // Name comparison (case-insensitive, remove diacritics for Vietnam names)
        let name_matches = id_result
            .full_name
            .as_ref()
            .map(|ocr_name| {
                Self::normalize_name(&user_data.full_name)
                    == Self::normalize_name(ocr_name)
            })
            .unwrap_or(false);

        // ID number comparison (exact match, ignore spaces/dashes)
        let id_number_matches = id_result
            .id_number
            .as_ref()
            .map(|ocr_id| {
                Self::normalize_id(&user_data.id_number) == Self::normalize_id(ocr_id)
            })
            .unwrap_or(false);

        // Date of birth comparison (normalize format)
        let dob_matches = id_result
            .date_of_birth
            .as_ref()
            .map(|ocr_dob| {
                Self::normalize_date(&user_data.date_of_birth)
                    == Self::normalize_date(ocr_dob)
            })
            .unwrap_or(false);

        let overall_match = name_matches && id_number_matches && dob_matches;

        DataVerificationResult {
            name_matches,
            id_number_matches,
            dob_matches,
            overall_match,
        }
    }

    /// Normalize name for comparison
    fn normalize_name(name: &str) -> String {
        name.to_uppercase()
            .chars()
            .filter(|c| c.is_alphanumeric() || c.is_whitespace())
            .collect::<String>()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Normalize ID number for comparison
    fn normalize_id(id: &str) -> String {
        id.chars()
            .filter(|c| c.is_alphanumeric())
            .collect::<String>()
            .to_uppercase()
    }

    /// Normalize date for comparison (convert to YYYY-MM-DD)
    fn normalize_date(date: &str) -> String {
        // Try to parse various formats and normalize to YYYY-MM-DD
        let cleaned: String = date.chars().filter(|c| c.is_numeric()).collect();

        if cleaned.len() == 8 {
            // Could be DDMMYYYY or YYYYMMDD
            let first_four: i32 = cleaned[..4].parse().unwrap_or(0);
            if first_four > 1900 {
                // Likely YYYYMMDD
                format!(
                    "{}-{}-{}",
                    &cleaned[..4],
                    &cleaned[4..6],
                    &cleaned[6..8]
                )
            } else {
                // Likely DDMMYYYY
                format!(
                    "{}-{}-{}",
                    &cleaned[4..8],
                    &cleaned[2..4],
                    &cleaned[..2]
                )
            }
        } else {
            cleaned
        }
    }

    /// Retry operation with exponential backoff
    async fn retry_with_backoff<F, Fut, T>(
        &self,
        config: &TenantEkycConfig,
        operation: F,
    ) -> Result<T>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        let mut last_error = None;

        for attempt in 0..=config.max_retries {
            match operation().await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    if attempt < config.max_retries {
                        let delay = config.retry_delay_ms * 2u64.pow(attempt);
                        debug!(
                            attempt = attempt + 1,
                            max_retries = config.max_retries,
                            delay_ms = delay,
                            error = %e,
                            "eKYC operation failed, retrying..."
                        );
                        tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
                    }
                    last_error = Some(e);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| {
            Error::Internal("Operation failed with no error details".to_string())
        }))
    }
}

/// Implement KycProvider for EkycService to integrate with existing KYC flow
#[async_trait]
impl KycProvider for EkycService {
    async fn verify(&self, request: &KycVerificationRequest) -> Result<KycVerificationResult> {
        // This is a simplified integration - in practice, you'd extract
        // the actual image data from the document storage URLs
        info!(
            tenant_id = %request.tenant_id,
            user_id = %request.user_id,
            "EkycService: Processing KYC verification request"
        );

        // For the existing KYC flow, we return a pending status
        // and require the caller to use the full verify() method
        // with actual image data
        Ok(KycVerificationResult {
            status: KycStatus::Pending,
            verified_tier: None,
            rejection_reason: Some(
                "Use EkycService::verify() with EkycVerificationRequest for full eKYC flow"
                    .to_string(),
            ),
            provider_reference: Some(format!(
                "ekyc_pending_{}_{}",
                request.user_id,
                chrono::Utc::now().timestamp()
            )),
        })
    }

    async fn check_status(&self, reference: &str) -> Result<KycVerificationResult> {
        // For async verification, check status with the provider
        info!(reference = %reference, "Checking eKYC verification status");

        // In a real implementation, you would:
        // 1. Parse the reference to determine the provider
        // 2. Query the provider for the verification status
        // 3. Return the updated status

        Ok(KycVerificationResult {
            status: KycStatus::Pending,
            verified_tier: None,
            rejection_reason: None,
            provider_reference: Some(reference.to_string()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_ekyc_service_mock_verification() {
        let service = EkycService::new_mock();
        let config = TenantEkycConfig::default();

        let request = EkycVerificationRequest {
            tenant_id: "tenant_1".to_string(),
            user_id: "user_1".to_string(),
            id_front_image: vec![0u8; 100],
            id_back_image: Some(vec![0u8; 100]),
            id_document_type: IdDocumentType::Cccd,
            selfie_image: vec![0u8; 100],
            liveness_data: Some(vec![0u8; 100]),
            requested_tier: KycTier::Tier1,
            user_provided_data: UserProvidedData {
                full_name: "NGUYEN VAN A".to_string(),
                id_number: "001234567890".to_string(),
                date_of_birth: "01/01/1990".to_string(),
                address: None,
            },
        };

        let result = service.verify(request, &config).await.unwrap();
        assert!(!result.reference_id.is_empty());
        assert!(result.details.id_verification.is_some());
        assert!(result.details.face_match.is_some());
    }

    #[test]
    fn test_name_normalization() {
        assert_eq!(
            EkycService::normalize_name("  Nguyen Van  A  "),
            "NGUYEN VAN A"
        );
        assert_eq!(
            EkycService::normalize_name("NGUYEN VAN A"),
            EkycService::normalize_name("nguyen van a")
        );
    }

    #[test]
    fn test_id_normalization() {
        assert_eq!(EkycService::normalize_id("001-234-567-890"), "001234567890");
        assert_eq!(EkycService::normalize_id("001 234 567 890"), "001234567890");
    }

    #[test]
    fn test_date_normalization() {
        assert_eq!(EkycService::normalize_date("01/01/1990"), "1990-01-01");
        assert_eq!(EkycService::normalize_date("19900101"), "1990-01-01");
    }

    #[test]
    fn test_provider_type_from_str() {
        assert_eq!(EkycProviderType::from_str("vnpay"), EkycProviderType::Vnpay);
        assert_eq!(EkycProviderType::from_str("fpt_ai"), EkycProviderType::FptAi);
        assert_eq!(EkycProviderType::from_str("unknown"), EkycProviderType::Mock);
    }
}
