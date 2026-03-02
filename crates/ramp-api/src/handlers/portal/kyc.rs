//! Portal KYC Handlers
//!
//! Endpoints for Know Your Customer (KYC) management:
//! - Get KYC status
//! - Submit KYC data
//! - Upload documents
//! - Get tier information

use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use tracing::info;
use uuid::Uuid;
use validator::Validate;

use base64::{engine::general_purpose::STANDARD, Engine};
use ramp_compliance::types::KycTier;
use ramp_compliance::zkkyc::{ZkCredential, ZkCredentialIssuer, ZkKycProof, ZkKycService};
use std::sync::OnceLock;

use crate::error::ApiError;
use crate::middleware::PortalUser;
use crate::router::AppState;

// ============================================================================
// DTOs
// ============================================================================

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KYCStatus {
    pub status: String,
    pub tier: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub submitted_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verified_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rejection_reason: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct KYCSubmission {
    #[validate(length(min = 1, max = 100, message = "First name must be 1-100 characters"))]
    pub first_name: String,

    #[validate(length(min = 1, max = 100, message = "Last name must be 1-100 characters"))]
    pub last_name: String,

    #[validate(length(
        min = 10,
        max = 10,
        message = "Date of birth must be YYYY-MM-DD format"
    ))]
    pub date_of_birth: String,

    #[validate(length(min = 1, max = 500, message = "Address must be 1-500 characters"))]
    pub address: String,

    #[validate(length(min = 1, max = 50, message = "Document type is required"))]
    pub id_document_type: String,

    #[validate(length(max = 50, message = "Document number must be max 50 characters"))]
    pub id_document_number: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct DocumentUploadRequest {
    #[validate(length(min = 1, message = "Document type is required"))]
    pub document_type: String,

    #[validate(length(min = 1, message = "File data is required"))]
    pub file_data: String, // Base64 encoded file

    #[validate(length(min = 1, message = "File name is required"))]
    pub file_name: String,

    #[validate(length(min = 1, message = "Content type is required"))]
    pub content_type: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentUploadResponse {
    pub document_id: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TierInfo {
    pub current_tier: i32,
    pub tier_name: String,
    pub limits: TierLimits,
    pub next_tier: Option<NextTierInfo>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TierLimits {
    pub daily_deposit_limit: String,
    pub daily_withdrawal_limit: String,
    pub monthly_deposit_limit: String,
    pub monthly_withdrawal_limit: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NextTierInfo {
    pub tier: i32,
    pub tier_name: String,
    pub requirements: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct CreateZkChallengeRequest {
    #[validate(range(min = 0, max = 3, message = "requiredKycLevel must be between 0 and 3"))]
    pub required_kyc_level: i16,
    #[serde(default)]
    pub allowed_nationalities: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateZkChallengeResponse {
    pub challenge: String,
    pub required_kyc_level: i16,
    pub allowed_nationalities: Vec<String>,
    pub expires_at: String,
}

#[derive(Debug, Clone, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct VerifyZkProofRequest {
    #[validate(length(equal = 64, message = "commitmentHash must be 64 hex chars"))]
    pub commitment_hash: String,
    #[validate(length(min = 1, message = "proofData is required"))]
    pub proof_data: String,
    #[validate(length(min = 1, message = "publicInputs must contain challenge"))]
    pub public_inputs: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VerifyZkProofResponse {
    pub valid: bool,
    pub commitment_hash: String,
    pub proven_tier: i16,
    pub verified_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rejection_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ZkCredentialResponse {
    pub id: String,
    pub commitment_hash: String,
    pub issued_at: String,
    pub expires_at: String,
    pub issuer_signature: String,
}

static ZK_KYC_SERVICE: OnceLock<ZkKycService> = OnceLock::new();
static ZK_CREDENTIAL_ISSUER: OnceLock<ZkCredentialIssuer> = OnceLock::new();

fn zk_kyc_service() -> &'static ZkKycService {
    ZK_KYC_SERVICE.get_or_init(|| {
        let key = std::env::var("ZK_KYC_VERIFICATION_KEY")
            .unwrap_or_else(|_| "rampos-zk-kyc-verification-key-dev".to_string())
            .into_bytes();
        ZkKycService::new(key)
    })
}

fn zk_credential_issuer() -> &'static ZkCredentialIssuer {
    ZK_CREDENTIAL_ISSUER.get_or_init(|| {
        let key = std::env::var("ZK_KYC_ISSUER_KEY")
            .unwrap_or_else(|_| "rampos-zk-kyc-issuer-key-dev".to_string())
            .into_bytes();
        ZkCredentialIssuer::new(key)
    })
}

fn map_tier(level: i16) -> KycTier {
    KycTier::from_i16(level)
}

fn to_credential_response(credential: ZkCredential) -> ZkCredentialResponse {
    ZkCredentialResponse {
        id: credential.id,
        commitment_hash: credential.commitment_hash,
        issued_at: credential.issued_at.to_rfc3339(),
        expires_at: credential.expires_at.to_rfc3339(),
        issuer_signature: credential.issuer_signature,
    }
}

// ============================================================================
// Router
// ============================================================================

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/status", get(get_kyc_status))
        .route("/submit", post(submit_kyc))
        .route("/documents", post(upload_document))
        .route("/tier", get(get_tier))
        .route("/zk/challenge", post(create_zk_kyc_challenge))
        .route("/zk/verify", post(verify_zk_kyc_proof))
        .route("/zk/credential", get(get_zk_credential_status))
}

// ============================================================================
// Handlers
// ============================================================================

/// GET /v1/portal/kyc/status - Get current KYC status
pub async fn get_kyc_status(
    State(_app_state): State<AppState>,
    portal_user: PortalUser,
) -> Result<Json<KYCStatus>, ApiError> {
    info!(
        user_id = %portal_user.user_id,
        tenant_id = %portal_user.tenant_id,
        "Get KYC status requested"
    );

    // In production, this would:
    // 1. Extract user from auth middleware
    // 2. Query user's KYC status from database
    // 3. Return current status and tier

    // Mock response
    let status = KYCStatus {
        status: "PENDING".to_string(),
        tier: 0,
        submitted_at: Some(Utc::now().to_rfc3339()),
        verified_at: None,
        rejection_reason: None,
    };

    Ok(Json(status))
}

/// POST /v1/portal/kyc/submit - Submit KYC data
pub async fn submit_kyc(
    State(_app_state): State<AppState>,
    portal_user: PortalUser,
    Json(req): Json<KYCSubmission>,
) -> Result<Json<KYCStatus>, ApiError> {
    // Validate request
    req.validate()
        .map_err(|e| ApiError::Validation(e.to_string()))?;

    // Validate document type
    let valid_doc_types = ["PASSPORT", "DRIVERS_LICENSE", "NATIONAL_ID"];
    if !valid_doc_types.contains(&req.id_document_type.as_str()) {
        return Err(ApiError::Validation(format!(
            "Invalid document type. Must be one of: {}",
            valid_doc_types.join(", ")
        )));
    }

    // Validate date format (basic check)
    if !is_valid_date(&req.date_of_birth) {
        return Err(ApiError::Validation(
            "Invalid date of birth format. Use YYYY-MM-DD".to_string(),
        ));
    }

    info!(
        user_id = %portal_user.user_id,
        tenant_id = %portal_user.tenant_id,
        first_name = %req.first_name,
        last_name = %req.last_name,
        doc_type = %req.id_document_type,
        "KYC submission received"
    );

    // In production, this would:
    // 1. Extract user from auth middleware
    // 2. Validate the submission data
    // 3. Store KYC data securely (encrypted)
    // 4. Trigger KYC verification workflow
    // 5. Update user's KYC status to PENDING

    let now = Utc::now();

    let status = KYCStatus {
        status: "PENDING".to_string(),
        tier: 0,
        submitted_at: Some(now.to_rfc3339()),
        verified_at: None,
        rejection_reason: None,
    };

    Ok(Json(status))
}

/// POST /v1/portal/kyc/documents - Upload KYC document (JSON with base64 file)
pub async fn upload_document(
    State(_app_state): State<AppState>,
    portal_user: PortalUser,
    Json(req): Json<DocumentUploadRequest>,
) -> Result<Json<DocumentUploadResponse>, ApiError> {
    // Validate request
    req.validate()
        .map_err(|e| ApiError::Validation(e.to_string()))?;

    // Validate document type
    let valid_doc_types = ["ID_FRONT", "ID_BACK", "SELFIE", "PROOF_OF_ADDRESS"];
    if !valid_doc_types.contains(&req.document_type.as_str()) {
        return Err(ApiError::Validation(format!(
            "Invalid document type. Must be one of: {}",
            valid_doc_types.join(", ")
        )));
    }

    // Validate content type
    let valid_content_types = ["image/jpeg", "image/png", "image/webp", "application/pdf"];
    if !valid_content_types.contains(&req.content_type.as_str()) {
        return Err(ApiError::Validation(format!(
            "Invalid content type. Must be one of: {}",
            valid_content_types.join(", ")
        )));
    }

    // Decode and validate file data
    use base64::{engine::general_purpose::STANDARD, Engine};
    let file_bytes = STANDARD
        .decode(&req.file_data)
        .map_err(|_| ApiError::Validation("Invalid base64 file data".to_string()))?;

    // Check file size (max 10MB)
    const MAX_FILE_SIZE: usize = 10 * 1024 * 1024;
    if file_bytes.len() > MAX_FILE_SIZE {
        return Err(ApiError::Validation(
            "File size exceeds 10MB limit".to_string(),
        ));
    }

    info!(
        user_id = %portal_user.user_id,
        tenant_id = %portal_user.tenant_id,
        doc_type = %req.document_type,
        file_name = %req.file_name,
        file_size = file_bytes.len(),
        "Document upload processing"
    );

    // In production, this would:
    // 1. Validate file type by magic bytes
    // 2. Scan for malware
    // 3. Upload to secure storage (S3, etc.)
    // 4. Store document reference in database
    // 5. Return signed URL for viewing

    let document_id = Uuid::new_v4().to_string();

    let response = DocumentUploadResponse {
        document_id: document_id.clone(),
        url: format!("/v1/portal/kyc/documents/{}", document_id),
    };

    Ok(Json(response))
}

/// GET /v1/portal/kyc/tier - Get current tier information
pub async fn get_tier(
    State(_app_state): State<AppState>,
    portal_user: PortalUser,
) -> Result<Json<TierInfo>, ApiError> {
    info!(
        user_id = %portal_user.user_id,
        tenant_id = %portal_user.tenant_id,
        "Get tier info requested"
    );

    // In production, this would:
    // 1. Extract user from auth middleware
    // 2. Get current tier and limits
    // 3. Calculate next tier requirements

    let tier_info = TierInfo {
        current_tier: 1,
        tier_name: "Basic".to_string(),
        limits: TierLimits {
            daily_deposit_limit: "10000000".to_string(),    // 10M VND
            daily_withdrawal_limit: "5000000".to_string(),  // 5M VND
            monthly_deposit_limit: "100000000".to_string(), // 100M VND
            monthly_withdrawal_limit: "50000000".to_string(), // 50M VND
        },
        next_tier: Some(NextTierInfo {
            tier: 2,
            tier_name: "Verified".to_string(),
            requirements: vec![
                "Complete ID verification".to_string(),
                "Upload proof of address".to_string(),
                "Complete selfie verification".to_string(),
            ],
        }),
    };

    Ok(Json(tier_info))
}

/// POST /v1/portal/kyc/zk/challenge - Create ZK-KYC challenge
pub async fn create_zk_kyc_challenge(
    State(_app_state): State<AppState>,
    portal_user: PortalUser,
    Json(req): Json<CreateZkChallengeRequest>,
) -> Result<Json<CreateZkChallengeResponse>, ApiError> {
    req.validate()
        .map_err(|e| ApiError::Validation(e.to_string()))?;

    let user_id = portal_user.user_id.to_string();
    let service = zk_kyc_service();
    let challenge = service.create_challenge(
        &user_id,
        map_tier(req.required_kyc_level),
        req.allowed_nationalities.clone(),
    );

    info!(
        user_id = %portal_user.user_id,
        tenant_id = %portal_user.tenant_id,
        required_kyc_level = req.required_kyc_level,
        "ZK-KYC challenge created via portal API"
    );

    Ok(Json(CreateZkChallengeResponse {
        challenge: challenge.challenge,
        required_kyc_level: challenge.required_kyc_level as i16,
        allowed_nationalities: challenge.allowed_nationalities,
        expires_at: challenge.expires_at.to_rfc3339(),
    }))
}

/// POST /v1/portal/kyc/zk/verify - Verify ZK-KYC proof and issue credential
pub async fn verify_zk_kyc_proof(
    State(_app_state): State<AppState>,
    portal_user: PortalUser,
    Json(req): Json<VerifyZkProofRequest>,
) -> Result<Json<VerifyZkProofResponse>, ApiError> {
    req.validate()
        .map_err(|e| ApiError::Validation(e.to_string()))?;

    let user_id = portal_user.user_id.to_string();

    let proof_bytes = STANDARD
        .decode(&req.proof_data)
        .map_err(|_| ApiError::Validation("Invalid base64 proofData".to_string()))?;

    let proof = ZkKycProof {
        commitment_hash: req.commitment_hash,
        proof_data: proof_bytes,
        public_inputs: req.public_inputs,
    };

    let service = zk_kyc_service();
    let result = service.verify_proof(&user_id, &proof);

    if result.valid {
        service.store_verification(
            &user_id,
            &result.commitment_hash,
            result.proven_tier,
        );

        let issuer = zk_credential_issuer();
        let _ = issuer.issue_credential(&user_id, &result.commitment_hash);

        info!(
            user_id = %portal_user.user_id,
            tenant_id = %portal_user.tenant_id,
            commitment_hash = %result.commitment_hash,
            "ZK-KYC proof verified and credential issued"
        );
    }

    Ok(Json(VerifyZkProofResponse {
        valid: result.valid,
        commitment_hash: result.commitment_hash,
        proven_tier: result.proven_tier as i16,
        verified_at: result.verified_at.to_rfc3339(),
        rejection_reason: result.rejection_reason,
    }))
}

/// GET /v1/portal/kyc/zk/credential - Get latest credential for current user
pub async fn get_zk_credential_status(
    State(_app_state): State<AppState>,
    portal_user: PortalUser,
) -> Result<Json<Option<ZkCredentialResponse>>, ApiError> {
    let user_id = portal_user.user_id.to_string();
    let service = zk_kyc_service();
    if !service.is_verified(&user_id) {
        return Ok(Json(None));
    }

    let tier = service
        .get_verified_tier(&user_id)
        .unwrap_or(KycTier::Tier0);

    let issuer = zk_credential_issuer();
    let issued = issuer.issue_credential(
        &user_id,
        &format!("{:064x}", tier as i16),
    );

    Ok(Json(Some(to_credential_response(issued))))
}

// ============================================================================
// Helper Functions
// ============================================================================

fn is_valid_date(date_str: &str) -> bool {
    if date_str.len() != 10 {
        return false;
    }

    let parts: Vec<&str> = date_str.split('-').collect();
    if parts.len() != 3 {
        return false;
    }

    // Basic validation of year, month, day
    let year: Result<i32, _> = parts[0].parse();
    let month: Result<u32, _> = parts[1].parse();
    let day: Result<u32, _> = parts[2].parse();

    match (year, month, day) {
        (Ok(y), Ok(m), Ok(d)) => y >= 1900 && y <= 2100 && m >= 1 && m <= 12 && d >= 1 && d <= 31,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_valid_date() {
        assert!(is_valid_date("1990-01-15"));
        assert!(is_valid_date("2000-12-31"));
        assert!(!is_valid_date("1990-13-15")); // Invalid month
        assert!(!is_valid_date("1990-01-32")); // Invalid day
        assert!(!is_valid_date("19900115")); // Wrong format
        assert!(!is_valid_date("1990/01/15")); // Wrong separator
    }

    #[test]
    fn test_kyc_submission_validation() {
        let valid_submission = KYCSubmission {
            first_name: "John".to_string(),
            last_name: "Doe".to_string(),
            date_of_birth: "1990-01-15".to_string(),
            address: "123 Main St, City".to_string(),
            id_document_type: "PASSPORT".to_string(),
            id_document_number: Some("AB123456".to_string()),
        };
        assert!(valid_submission.validate().is_ok());

        let invalid_submission = KYCSubmission {
            first_name: "".to_string(), // Empty
            last_name: "Doe".to_string(),
            date_of_birth: "1990-01-15".to_string(),
            address: "123 Main St".to_string(),
            id_document_type: "PASSPORT".to_string(),
            id_document_number: None,
        };
        assert!(invalid_submission.validate().is_err());
    }
}
