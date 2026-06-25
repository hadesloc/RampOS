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
use ramp_core::service::crypto::{CryptoService, ENCRYPTED_TEXT_PREFIX};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use tracing::{info, warn};
use uuid::Uuid;
use validator::Validate;

use base64::{engine::general_purpose::STANDARD, Engine};
use ramp_compliance::passport::{passport_summary_from_flags, PassportPortalSummary};
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_rescreening_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document_expiry_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub restriction_status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alert_codes: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub passport_summary: Option<PassportPortalSummary>,
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

#[derive(Debug, Clone, PartialEq, Eq)]
struct KycPiiFields {
    full_name: String,
    date_of_birth: String,
    document_type: String,
    document_number: Option<String>,
    address: String,
}

fn is_production() -> bool {
    ramp_common::onchain_gate::is_production()
}

fn kyc_pii_crypto() -> Result<Option<CryptoService>, ApiError> {
    match CryptoService::from_env() {
        Ok(crypto) => Ok(Some(crypto)),
        Err(error) if is_production() => Err(ApiError::Internal(format!(
            "KYC PII encryption key is required in production: {}",
            error
        ))),
        Err(error) => {
            warn!(
                error = %error,
                "ENCRYPTION_MASTER_KEY not configured; storing KYC PII as plaintext outside production"
            );
            Ok(None)
        }
    }
}

fn encrypt_optional_pii(
    crypto: Option<&CryptoService>,
    value: Option<&str>,
    field_name: &str,
) -> Result<Option<String>, ApiError> {
    value
        .map(|value| encrypt_pii(crypto, value, field_name))
        .transpose()
}

fn encrypt_pii(
    crypto: Option<&CryptoService>,
    value: &str,
    field_name: &str,
) -> Result<String, ApiError> {
    match crypto {
        Some(crypto) => crypto
            .encrypt_text_for_storage(value)
            .map_err(|e| ApiError::Internal(format!("Failed to encrypt KYC {field_name}: {e}"))),
        None => Ok(value.to_string()),
    }
}

fn decrypt_optional_pii(
    crypto: Option<&CryptoService>,
    value: Option<String>,
    field_name: &str,
) -> Result<Option<String>, ApiError> {
    value
        .map(|value| decrypt_pii(crypto, value, field_name))
        .transpose()
}

fn decrypt_pii(
    crypto: Option<&CryptoService>,
    value: String,
    field_name: &str,
) -> Result<String, ApiError> {
    if value.starts_with(ENCRYPTED_TEXT_PREFIX) {
        let crypto = crypto.ok_or_else(|| {
            ApiError::Internal(format!(
                "KYC {field_name} is encrypted but ENCRYPTION_MASTER_KEY is not configured"
            ))
        })?;
        return crypto
            .decrypt_text_from_storage(&value)
            .map_err(|e| ApiError::Internal(format!("Failed to decrypt KYC {field_name}: {e}")));
    }

    if is_production() {
        warn!(
            field = field_name,
            "Reading legacy plaintext KYC PII in production; migrate row to encrypted storage"
        );
    }

    Ok(value)
}

async fn store_kyc_case(
    pool: Option<&PgPool>,
    tenant_id: &str,
    user_id: &str,
    pii: &KycPiiFields,
) -> Result<(), ApiError> {
    let crypto = kyc_pii_crypto()?;

    let Some(pool) = pool else {
        if is_production() {
            return Err(ApiError::Internal(
                "KYC case database is required in production".to_string(),
            ));
        }
        warn!(
            tenant_id = tenant_id,
            user_id = user_id,
            "KYC case database unavailable; skipping local KYC case persistence outside production"
        );
        return Ok(());
    };
    let encrypted_full_name = encrypt_pii(crypto.as_ref(), &pii.full_name, "full_name")?;
    let encrypted_date_of_birth =
        encrypt_pii(crypto.as_ref(), &pii.date_of_birth, "date_of_birth")?;
    let encrypted_document_type =
        encrypt_pii(crypto.as_ref(), &pii.document_type, "document_type")?;
    let encrypted_document_number = encrypt_optional_pii(
        crypto.as_ref(),
        pii.document_number.as_deref(),
        "document_number",
    )?;
    let encrypted_address = encrypt_pii(crypto.as_ref(), &pii.address, "address")?;

    sqlx::query(
        r#"
        INSERT INTO portal_kyc_cases (
            user_id,
            tenant_id,
            status,
            tier,
            full_name,
            date_of_birth,
            document_type,
            document_number,
            address
        )
        VALUES ($1, $2, 'PENDING', 1, $3, $4, $5, $6, $7)
        "#,
    )
    .bind(user_id)
    .bind(tenant_id)
    .bind(encrypted_full_name)
    .bind(encrypted_date_of_birth)
    .bind(encrypted_document_type)
    .bind(encrypted_document_number)
    .bind(encrypted_address)
    .execute(pool)
    .await
    .map_err(|e| ApiError::Internal(format!("Failed to store KYC case: {}", e)))?;

    Ok(())
}

#[allow(dead_code)]
async fn load_latest_kyc_case(
    pool: &PgPool,
    tenant_id: &str,
    user_id: &str,
) -> Result<Option<KycPiiFields>, ApiError> {
    let row = sqlx::query_as::<_, (String, String, String, Option<String>, String)>(
        r#"
        SELECT full_name, date_of_birth, document_type, document_number, address
        FROM portal_kyc_cases
        WHERE tenant_id = $1 AND user_id = $2
        ORDER BY submitted_at DESC
        LIMIT 1
        "#,
    )
    .bind(tenant_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| ApiError::Internal(format!("Failed to read KYC case: {}", e)))?;

    let Some((full_name, date_of_birth, document_type, document_number, address)) = row else {
        return Ok(None);
    };

    let crypto = match CryptoService::from_env() {
        Ok(crypto) => Some(crypto),
        Err(error) if is_production() => {
            warn!(
                error = %error,
                "ENCRYPTION_MASTER_KEY not configured while reading KYC PII in production"
            );
            None
        }
        Err(_) => None,
    };

    Ok(Some(KycPiiFields {
        full_name: decrypt_pii(crypto.as_ref(), full_name, "full_name")?,
        date_of_birth: decrypt_pii(crypto.as_ref(), date_of_birth, "date_of_birth")?,
        document_type: decrypt_pii(crypto.as_ref(), document_type, "document_type")?,
        document_number: decrypt_optional_pii(crypto.as_ref(), document_number, "document_number")?,
        address: decrypt_pii(crypto.as_ref(), address, "address")?,
    }))
}

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
    State(app_state): State<AppState>,
    portal_user: PortalUser,
) -> Result<Json<KYCStatus>, ApiError> {
    info!(
        financial_user_id = %portal_user.financial_user_id,
        tenant_id = %portal_user.tenant_id,
        "Get KYC status requested"
    );

    let tenant_id = ramp_common::types::TenantId::new(portal_user.tenant_id.to_string());
    let user_id = ramp_common::types::UserId::new(portal_user.financial_user_id.to_string());
    let user = app_state
        .user_service
        .get_user(&tenant_id, &user_id)
        .await
        .map_err(ApiError::from)?;
    let rescreening = user.risk_flags.get("rescreening");

    let status = KYCStatus {
        status: user.kyc_status,
        tier: i32::from(user.kyc_tier),
        submitted_at: Some(user.created_at.to_rfc3339()),
        verified_at: user.kyc_verified_at.map(|value| value.to_rfc3339()),
        rejection_reason: None,
        next_rescreening_at: rescreening
            .and_then(|value| value.get("nextRunAt"))
            .and_then(|value| value.as_str())
            .map(str::to_string),
        document_expiry_at: rescreening
            .and_then(|value| value.get("documentExpiryAt"))
            .and_then(|value| value.as_str())
            .map(str::to_string),
        restriction_status: rescreening
            .and_then(|value| value.get("restrictionStatus"))
            .and_then(|value| value.as_str())
            .map(str::to_string),
        alert_codes: rescreening
            .and_then(|value| value.get("alertCodes"))
            .and_then(|value| value.as_array())
            .map(|items| {
                items
                    .iter()
                    .filter_map(|value| value.as_str().map(str::to_string))
                    .collect::<Vec<_>>()
            })
            .filter(|items| !items.is_empty()),
        passport_summary: passport_summary_from_flags(&user.risk_flags),
    };

    Ok(Json(status))
}

/// POST /v1/portal/kyc/submit - Submit KYC data
pub async fn submit_kyc(
    State(app_state): State<AppState>,
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
        financial_user_id = %portal_user.financial_user_id,
        tenant_id = %portal_user.tenant_id,
        status = "PENDING",
        "KYC submission received"
    );

    let now = Utc::now();
    let tenant_id = ramp_common::types::TenantId::new(portal_user.tenant_id.to_string());
    let user_id = ramp_common::types::UserId::new(portal_user.financial_user_id.to_string());
    let pii_fields = KycPiiFields {
        full_name: format!("{} {}", req.first_name, req.last_name),
        date_of_birth: req.date_of_birth.clone(),
        document_type: req.id_document_type.clone(),
        document_number: req.id_document_number.clone(),
        address: req.address.clone(),
    };

    store_kyc_case(
        app_state.db_pool.as_ref(),
        tenant_id.0.as_str(),
        user_id.0.as_str(),
        &pii_fields,
    )
    .await?;

    // 1. Update user KYC status to PENDING
    app_state
        .user_service
        .update_user(
            &tenant_id,
            &user_id,
            Some("PENDING".to_string()),
            None,
            None,
            None,
        )
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to update user KYC status: {}", e)))?;

    // 2. Store non-PII submission metadata in risk_flags for audit trail.
    // PII lives in portal_kyc_cases with app-layer encryption when persistence is available.
    let submission_metadata = serde_json::json!({
        "kycSubmission": {
            "kyc_submitted": true,
            "submitted_at": now.to_rfc3339(),
            "pii_storage": "portal_kyc_cases_enc_v1",
            "status": "PENDING"
        }
    });

    app_state
        .user_service
        .update_user_risk_flags(&tenant_id, &user_id, submission_metadata)
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to store KYC submission data: {}", e)))?;

    // 3. Trigger real KYC verification in background (Onfido / Mock provider)
    if let Some(ref kyc_service) = app_state.kyc_service {
        let kyc_service = kyc_service.clone();
        let user_service = app_state.user_service.clone();
        let bg_tenant_id = tenant_id.clone();
        let bg_user_id = user_id.clone();

        let verification_request = ramp_compliance::kyc::KycVerificationRequest {
            tenant_id: bg_tenant_id.clone(),
            user_id: bg_user_id.clone(),
            tier: KycTier::from_i16(1), // Request Tier 1 for basic KYC
            full_name: format!("{} {}", req.first_name, req.last_name),
            date_of_birth: req.date_of_birth.clone(),
            id_number: req.id_document_number.clone().unwrap_or_default(),
            id_type: req.id_document_type.clone(),
            documents: vec![],
        };

        tokio::spawn(async move {
            match kyc_service.submit_verification(verification_request).await {
                Ok(result) => {
                    let new_status = match result.status {
                        ramp_compliance::types::KycStatus::Approved => "VERIFIED",
                        ramp_compliance::types::KycStatus::Rejected => "REJECTED",
                        _ => "PENDING",
                    };

                    // Update user KYC status and tier
                    let new_tier = result.verified_tier.map(|t| t as i16);
                    if let Err(e) = user_service
                        .update_user(
                            &bg_tenant_id,
                            &bg_user_id,
                            Some(new_status.to_string()),
                            new_tier,
                            None,
                            None,
                        )
                        .await
                    {
                        tracing::error!(error = %e, "Failed to update user after KYC verification");
                    } else {
                        info!(
                            user_id = %bg_user_id,
                            status = new_status,
                            "KYC verification completed via provider"
                        );
                    }
                }
                Err(e) => {
                    tracing::error!(error = %e, "KYC provider verification failed");
                }
            }
        });
    }

    // 4. Return PENDING status with real tier from user
    let user = app_state
        .user_service
        .get_user(&tenant_id, &user_id)
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to get user: {}", e)))?;

    let status = KYCStatus {
        status: "PENDING".to_string(),
        tier: i32::from(user.kyc_tier),
        submitted_at: Some(now.to_rfc3339()),
        verified_at: None,
        rejection_reason: None,
        next_rescreening_at: None,
        document_expiry_at: None,
        restriction_status: None,
        alert_codes: None,
        passport_summary: passport_summary_from_flags(&user.risk_flags),
    };

    Ok(Json(status))
}

/// POST /v1/portal/kyc/documents - Upload KYC document (JSON with base64 file)
pub async fn upload_document(
    State(app_state): State<AppState>,
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
        financial_user_id = %portal_user.financial_user_id,
        tenant_id = %portal_user.tenant_id,
        status = "PENDING",
        "Document upload processing"
    );

    // Map string document type to storage enum
    let storage_doc_type = match req.document_type.as_str() {
        "ID_FRONT" => ramp_compliance::storage::DocumentType::IdFront,
        "ID_BACK" => ramp_compliance::storage::DocumentType::IdBack,
        "SELFIE" => ramp_compliance::storage::DocumentType::Selfie,
        "PROOF_OF_ADDRESS" => ramp_compliance::storage::DocumentType::ProofOfAddress,
        _ => ramp_compliance::storage::DocumentType::IdFront, // already validated above
    };

    // Derive file extension from content type
    let extension = match req.content_type.as_str() {
        "image/jpeg" => "jpg",
        "image/png" => "png",
        "image/webp" => "webp",
        "application/pdf" => "pdf",
        _ => "bin",
    };

    // Upload to document storage if available, otherwise generate local reference
    let document_url = if let Some(ref storage) = app_state.document_storage {
        storage
            .upload(
                portal_user.tenant_id.to_string(),
                portal_user.financial_user_id.to_string(),
                storage_doc_type,
                file_bytes,
                extension,
            )
            .await
            .map_err(|e| ApiError::Internal(format!("Failed to upload document: {}", e)))?
    } else {
        // Fallback: generate reference ID without persistent storage
        let document_id = Uuid::new_v4().to_string();
        format!("/v1/portal/kyc/documents/{}", document_id)
    };

    let document_id = Uuid::new_v4().to_string();

    let response = DocumentUploadResponse {
        document_id,
        url: document_url,
    };

    Ok(Json(response))
}

/// GET /v1/portal/kyc/tier - Get current tier information
pub async fn get_tier(
    State(app_state): State<AppState>,
    portal_user: PortalUser,
) -> Result<Json<TierInfo>, ApiError> {
    info!(
        financial_user_id = %portal_user.financial_user_id,
        tenant_id = %portal_user.tenant_id,
        "Get tier info requested"
    );

    // Read real user tier from user service
    let tenant_id = ramp_common::types::TenantId::new(portal_user.tenant_id.to_string());
    let user_id = ramp_common::types::UserId::new(portal_user.financial_user_id.to_string());
    let user = app_state
        .user_service
        .get_user(&tenant_id, &user_id)
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to get user: {}", e)))?;

    let current_tier = KycTier::from_i16(user.kyc_tier);

    // Compute tier name
    let tier_name = match current_tier {
        KycTier::Tier0 => "Unverified",
        KycTier::Tier1 => "Basic",
        KycTier::Tier2 => "Verified",
        KycTier::Tier3 => "Business",
    };

    // Compute limits dynamically from KycTier
    let limits = TierLimits {
        daily_deposit_limit: current_tier.daily_payin_limit_vnd().to_string(),
        daily_withdrawal_limit: current_tier.daily_payout_limit_vnd().to_string(),
        monthly_deposit_limit: (current_tier.daily_payin_limit_vnd()
            * rust_decimal::Decimal::from(30))
        .to_string(),
        monthly_withdrawal_limit: (current_tier.daily_payout_limit_vnd()
            * rust_decimal::Decimal::from(30))
        .to_string(),
    };

    // Compute next tier requirements
    let next_tier = match current_tier {
        KycTier::Tier0 => Some(NextTierInfo {
            tier: 1,
            tier_name: "Basic".to_string(),
            requirements: vec![
                "Submit basic KYC information".to_string(),
                "Upload ID document (front)".to_string(),
            ],
        }),
        KycTier::Tier1 => Some(NextTierInfo {
            tier: 2,
            tier_name: "Verified".to_string(),
            requirements: vec![
                "Complete ID verification".to_string(),
                "Upload proof of address".to_string(),
                "Complete selfie verification".to_string(),
            ],
        }),
        KycTier::Tier2 => Some(NextTierInfo {
            tier: 3,
            tier_name: "Business".to_string(),
            requirements: vec![
                "Complete business verification (KYB)".to_string(),
                "Submit source of funds documentation".to_string(),
            ],
        }),
        KycTier::Tier3 => None, // Already at highest tier
    };

    let tier_info = TierInfo {
        current_tier: current_tier as i32,
        tier_name: tier_name.to_string(),
        limits,
        next_tier,
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

    let user_id = portal_user.financial_user_id.to_string();
    let service = zk_kyc_service();
    let challenge = service.create_challenge(
        &user_id,
        map_tier(req.required_kyc_level),
        req.allowed_nationalities.clone(),
    );

    info!(
        financial_user_id = %portal_user.financial_user_id,
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

    let user_id = portal_user.financial_user_id.to_string();

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
        service.store_verification(&user_id, &result.commitment_hash, result.proven_tier);

        let issuer = zk_credential_issuer();
        let _ = issuer.issue_credential(&user_id, &result.commitment_hash);

        info!(
            financial_user_id = %portal_user.financial_user_id,
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
    let user_id = portal_user.financial_user_id.to_string();
    let service = zk_kyc_service();
    if !service.is_verified(&user_id) {
        return Ok(Json(None));
    }

    let tier = service
        .get_verified_tier(&user_id)
        .unwrap_or(KycTier::Tier0);

    let issuer = zk_credential_issuer();
    let issued = issuer.issue_credential(&user_id, &format!("{:064x}", tier as i16));

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

    fn test_key() -> [u8; 32] {
        let mut key = [0u8; 32];
        for (i, byte) in key.iter_mut().enumerate() {
            *byte = i as u8;
        }
        key
    }

    #[test]
    fn test_kyc_pii_round_trip_encrypt_decrypt() {
        let crypto = CryptoService::from_key(&test_key());
        let pii = KycPiiFields {
            full_name: "John Doe".to_string(),
            date_of_birth: "1990-01-15".to_string(),
            document_type: "PASSPORT".to_string(),
            document_number: Some("AB123456".to_string()),
            address: "123 Main St".to_string(),
        };

        let encrypted_full_name = encrypt_pii(Some(&crypto), &pii.full_name, "full_name").unwrap();
        let encrypted_dob =
            encrypt_pii(Some(&crypto), &pii.date_of_birth, "date_of_birth").unwrap();
        let encrypted_doc_type =
            encrypt_pii(Some(&crypto), &pii.document_type, "document_type").unwrap();
        let encrypted_doc_number = encrypt_optional_pii(
            Some(&crypto),
            pii.document_number.as_deref(),
            "document_number",
        )
        .unwrap();
        let encrypted_address = encrypt_pii(Some(&crypto), &pii.address, "address").unwrap();

        assert!(encrypted_full_name.starts_with(ENCRYPTED_TEXT_PREFIX));
        assert_ne!(encrypted_full_name, pii.full_name);

        let decrypted = KycPiiFields {
            full_name: decrypt_pii(Some(&crypto), encrypted_full_name, "full_name").unwrap(),
            date_of_birth: decrypt_pii(Some(&crypto), encrypted_dob, "date_of_birth").unwrap(),
            document_type: decrypt_pii(Some(&crypto), encrypted_doc_type, "document_type").unwrap(),
            document_number: decrypt_optional_pii(
                Some(&crypto),
                encrypted_doc_number,
                "document_number",
            )
            .unwrap(),
            address: decrypt_pii(Some(&crypto), encrypted_address, "address").unwrap(),
        };

        assert_eq!(decrypted, pii);
    }

    #[test]
    fn test_kyc_pii_legacy_plaintext_readable() {
        let value = decrypt_pii(None, "legacy plaintext".to_string(), "full_name").unwrap();
        assert_eq!(value, "legacy plaintext");
    }

    #[test]
    fn test_kyc_pii_production_without_key_fails_closed() {
        let _guard = ramp_common::onchain_gate::test_env_lock();
        std::env::set_var("RUST_ENV", "production");
        std::env::remove_var("RAMPOS_ENV");
        std::env::remove_var("ENCRYPTION_MASTER_KEY");

        let err = match kyc_pii_crypto() {
            Ok(_) => panic!("expected KYC PII encryption to fail without a production key"),
            Err(err) => err,
        };
        assert!(format!("{err:?}").contains("KYC PII encryption key is required"));

        std::env::remove_var("RUST_ENV");
    }

    #[tokio::test]
    async fn test_store_kyc_case_production_without_db_fails_closed() {
        let _guard = ramp_common::onchain_gate::test_env_lock();
        std::env::set_var("RUST_ENV", "production");
        std::env::remove_var("RAMPOS_ENV");
        std::env::set_var("ENCRYPTION_MASTER_KEY", hex::encode(test_key()));

        let pii = KycPiiFields {
            full_name: "John Doe".to_string(),
            date_of_birth: "1990-01-15".to_string(),
            document_type: "PASSPORT".to_string(),
            document_number: Some("AB123456".to_string()),
            address: "123 Main St".to_string(),
        };

        let err = match store_kyc_case(None, "tenant_1", "user_1", &pii).await {
            Ok(_) => panic!("expected KYC case storage to fail without a production database"),
            Err(err) => err,
        };
        assert!(format!("{err:?}").contains("KYC case database is required in production"));

        std::env::remove_var("RUST_ENV");
        std::env::remove_var("ENCRYPTION_MASTER_KEY");
    }

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
