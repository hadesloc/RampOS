//! Bank Webhook Handlers
//!
//! Handles incoming webhooks from various bank providers (VietQR, Napas, etc.)
//! for bank confirmation of pay-in transactions.

use axum::{
    body::Bytes,
    extract::{Path, State},
    http::HeaderMap,
    Json,
};
use chrono::{DateTime, Utc};
use hmac::{Hmac, Mac};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Sha512};
use std::sync::Arc;
use tracing::{info, instrument, warn};

use crate::error::ApiError;
use ramp_core::repository::{BankConfirmationRepository, CreateBankConfirmationRequest};
use ramp_core::service::{
    map_generic_bank_status, map_napas_status, map_vietqr_status, CanonicalPaymentDirection,
    CanonicalPaymentInput, CanonicalPaymentParty, CanonicalPaymentRecord,
};

/// State for bank webhook handlers
#[derive(Clone)]
pub struct BankWebhookState {
    pub confirmation_repo: Arc<dyn BankConfirmationRepository>,
    /// Maps provider to (tenant_id, secret)
    /// In production, this would be loaded from database
    pub provider_secrets: Arc<std::collections::HashMap<String, Vec<ProviderSecret>>>,
}

#[derive(Clone)]
pub struct ProviderSecret {
    pub tenant_id: String,
    pub secret: Vec<u8>,
    pub algorithm: SignatureAlgorithm,
    pub header_name: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SignatureAlgorithm {
    HmacSha256,
    HmacSha512,
}

impl BankWebhookState {
    pub fn new(confirmation_repo: Arc<dyn BankConfirmationRepository>) -> Self {
        Self {
            confirmation_repo,
            provider_secrets: Arc::new(std::collections::HashMap::new()),
        }
    }

    pub fn with_secrets(
        mut self,
        secrets: std::collections::HashMap<String, Vec<ProviderSecret>>,
    ) -> Self {
        self.provider_secrets = Arc::new(secrets);
        self
    }
}

// ============================================================================
// VietQR Webhook Format
// ============================================================================

/// VietQR webhook payload format
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VietQrWebhookPayload {
    /// Transaction ID from VietQR
    pub transaction_id: String,
    /// Reference code (our reference embedded in QR)
    pub reference_code: String,
    /// Amount in VND
    pub amount: i64,
    /// Currency (usually VND)
    #[serde(default = "default_vnd")]
    pub currency: String,
    /// Sender bank code
    pub sender_bank_code: Option<String>,
    /// Sender account number
    pub sender_account: Option<String>,
    /// Sender name
    pub sender_name: Option<String>,
    /// Receiver bank code
    pub receiver_bank_code: Option<String>,
    /// Receiver account number
    pub receiver_account: Option<String>,
    /// Receiver name
    pub receiver_name: Option<String>,
    /// Transaction description/memo
    pub description: Option<String>,
    /// Transaction time
    #[serde(with = "chrono::serde::ts_milliseconds_option", default)]
    pub transaction_time: Option<DateTime<Utc>>,
    /// Status from VietQR
    pub status: String,
}

fn default_vnd() -> String {
    "VND".to_string()
}

/// VietQR webhook response
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VietQrWebhookResponse {
    pub success: bool,
    pub message: String,
    pub confirmation_id: Option<String>,
}

// ============================================================================
// Napas Webhook Format
// ============================================================================

/// Napas webhook payload format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NapasWebhookPayload {
    /// Transaction ID from Napas
    #[serde(rename = "transId")]
    pub trans_id: String,
    /// Reference number (contains our reference)
    #[serde(rename = "refNo")]
    pub ref_no: String,
    /// Amount
    pub amount: String,
    /// Currency
    #[serde(default = "default_vnd")]
    pub currency: String,
    /// Debtor bank BIC
    #[serde(rename = "dbtrBIC")]
    pub debtor_bic: Option<String>,
    /// Debtor account
    #[serde(rename = "dbtrAcct")]
    pub debtor_account: Option<String>,
    /// Debtor name
    #[serde(rename = "dbtrNm")]
    pub debtor_name: Option<String>,
    /// Creditor bank BIC
    #[serde(rename = "cdtrBIC")]
    pub creditor_bic: Option<String>,
    /// Creditor account
    #[serde(rename = "cdtrAcct")]
    pub creditor_account: Option<String>,
    /// Creditor name
    #[serde(rename = "cdtrNm")]
    pub creditor_name: Option<String>,
    /// Transaction date time
    #[serde(rename = "txDtTm")]
    pub tx_datetime: Option<String>,
    /// Status code
    #[serde(rename = "stsCode")]
    pub status_code: String,
}

/// Napas webhook response
#[derive(Debug, Serialize)]
pub struct NapasWebhookResponse {
    #[serde(rename = "responseCode")]
    pub response_code: String,
    #[serde(rename = "responseMessage")]
    pub response_message: String,
}

// ============================================================================
// Generic Bank Webhook Format
// ============================================================================

/// Generic bank webhook payload (for custom integrations)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenericBankWebhookPayload {
    /// Bank's transaction ID
    pub bank_tx_id: String,
    /// Our reference code
    pub reference_code: String,
    /// Amount
    pub amount: Decimal,
    /// Currency
    #[serde(default = "default_vnd")]
    pub currency: String,
    /// Sender account
    pub sender_account: Option<String>,
    /// Sender name
    pub sender_name: Option<String>,
    /// Receiver account
    pub receiver_account: Option<String>,
    /// Transaction time
    pub transaction_time: Option<DateTime<Utc>>,
    /// Additional metadata
    #[serde(default)]
    pub metadata: serde_json::Value,
}

/// Generic webhook response
#[derive(Debug, Serialize)]
pub struct GenericWebhookResponse {
    pub success: bool,
    pub message: String,
    pub confirmation_id: Option<String>,
}

// ============================================================================
// Webhook Handler
// ============================================================================

/// Handle incoming bank webhook
///
/// POST /v1/webhooks/bank/:provider
#[utoipa::path(
    post,
    path = "/v1/webhooks/bank/{provider}",
    tag = "webhooks",
    params(
        ("provider" = String, Path, description = "Bank provider name (e.g., vietqr, napas)")
    ),
    responses(
        (status = 200, description = "Webhook processed", body = Object),
        (status = 400, description = "Invalid payload", body = ErrorResponse),
        (status = 401, description = "Invalid signature", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(
        ("hmac_signature" = [])
    )
)]
#[instrument(skip_all, fields(provider = %provider))]
pub async fn handle_bank_webhook(
    State(state): State<BankWebhookState>,
    Path(provider): Path<String>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<serde_json::Value>, ApiError> {
    let provider_lower = provider.to_lowercase();

    info!(
        provider = %provider,
        body_size = body.len(),
        "Received bank webhook"
    );

    // Get provider secrets for signature verification
    let provider_secrets = state.provider_secrets.get(&provider_lower);

    // Verify signature if secrets are configured
    let verified_tenant = if let Some(secrets) = provider_secrets {
        verify_webhook_signature(&headers, &body, secrets)?
    } else {
        warn!(provider = %provider, "No webhook secrets configured for provider, skipping signature verification");
        None
    };

    // Route to provider-specific handler
    match provider_lower.as_str() {
        "vietqr" => {
            let payload: VietQrWebhookPayload = serde_json::from_slice(&body)
                .map_err(|e| ApiError::BadRequest(format!("Invalid VietQR payload: {}", e)))?;
            let response = handle_vietqr_webhook(state, payload, verified_tenant).await?;
            let json = serde_json::to_value(response)
                .map_err(|_| ApiError::Internal("Serialization failed".to_string()))?;
            Ok(Json(json))
        }
        "napas" => {
            let payload: NapasWebhookPayload = serde_json::from_slice(&body)
                .map_err(|e| ApiError::BadRequest(format!("Invalid Napas payload: {}", e)))?;
            let response = handle_napas_webhook(state, payload, verified_tenant).await?;
            let json = serde_json::to_value(response)
                .map_err(|_| ApiError::Internal("Serialization failed".to_string()))?;
            Ok(Json(json))
        }
        _ => {
            // Try generic format
            let payload: GenericBankWebhookPayload = serde_json::from_slice(&body)
                .map_err(|e| ApiError::BadRequest(format!("Invalid webhook payload: {}", e)))?;
            let response =
                handle_generic_webhook(state, &provider, payload, verified_tenant).await?;
            let json = serde_json::to_value(response)
                .map_err(|_| ApiError::Internal("Serialization failed".to_string()))?;
            Ok(Json(json))
        }
    }
}

/// Verify webhook signature
fn verify_webhook_signature(
    headers: &HeaderMap,
    body: &[u8],
    secrets: &[ProviderSecret],
) -> Result<Option<String>, ApiError> {
    if secrets.is_empty() {
        return Ok(None);
    }

    let has_signature = secrets
        .iter()
        .any(|s| headers.get(&s.header_name).is_some());

    if !has_signature {
        return Err(ApiError::Unauthorized("Missing signature".to_string()));
    }

    for secret in secrets {
        let signature = headers
            .get(&secret.header_name)
            .and_then(|v| v.to_str().ok());

        if let Some(sig) = signature {
            let valid = match secret.algorithm {
                SignatureAlgorithm::HmacSha256 => {
                    verify_hmac_sha256(&secret.secret, body, sig).unwrap_or(false)
                }
                SignatureAlgorithm::HmacSha512 => {
                    verify_hmac_sha512(&secret.secret, body, sig).unwrap_or(false)
                }
            };

            if valid {
                info!(
                    tenant_id = %secret.tenant_id,
                    "Webhook signature verified"
                );
                return Ok(Some(secret.tenant_id.clone()));
            }
        }
    }

    warn!("Webhook signature verification failed");
    Err(ApiError::Unauthorized(
        "Invalid webhook signature".to_string(),
    ))
}

fn verify_hmac_sha256(secret: &[u8], data: &[u8], signature: &str) -> Result<bool, ApiError> {
    let mut mac = Hmac::<Sha256>::new_from_slice(secret)
        .map_err(|_| ApiError::Internal("Invalid HMAC key".to_string()))?;
    mac.update(data);

    // Try hex-encoded signature
    if let Ok(sig_bytes) = hex::decode(signature) {
        if mac.clone().verify_slice(&sig_bytes).is_ok() {
            return Ok(true);
        }
    }

    // Try base64-encoded signature
    if let Ok(sig_bytes) =
        base64::Engine::decode(&base64::engine::general_purpose::STANDARD, signature)
    {
        if mac.verify_slice(&sig_bytes).is_ok() {
            return Ok(true);
        }
    }

    Ok(false)
}

fn verify_hmac_sha512(secret: &[u8], data: &[u8], signature: &str) -> Result<bool, ApiError> {
    let mut mac = Hmac::<Sha512>::new_from_slice(secret)
        .map_err(|_| ApiError::Internal("Invalid HMAC key".to_string()))?;
    mac.update(data);

    // Try hex-encoded signature
    if let Ok(sig_bytes) = hex::decode(signature) {
        if mac.clone().verify_slice(&sig_bytes).is_ok() {
            return Ok(true);
        }
    }

    // Try base64-encoded signature
    if let Ok(sig_bytes) =
        base64::Engine::decode(&base64::engine::general_purpose::STANDARD, signature)
    {
        if mac.verify_slice(&sig_bytes).is_ok() {
            return Ok(true);
        }
    }

    Ok(false)
}

fn canonicalize_vietqr_payment(
    tenant_id: &str,
    payload: &VietQrWebhookPayload,
) -> CanonicalPaymentRecord {
    CanonicalPaymentRecord::from_input(
        CanonicalPaymentInput {
            provider: "vietqr".to_string(),
            provider_reference: payload.transaction_id.clone(),
            reference_code: payload.reference_code.clone(),
            tenant_id: tenant_id.to_string(),
            direction: CanonicalPaymentDirection::Inbound,
            amount: Decimal::from(payload.amount),
            currency: payload.currency.clone(),
            raw_status: Some(payload.status.clone()),
            payer: CanonicalPaymentParty {
                account_id: payload.sender_account.clone(),
                bank_identifier: payload.sender_bank_code.clone(),
                display_name: payload.sender_name.clone(),
            },
            beneficiary: CanonicalPaymentParty {
                account_id: payload.receiver_account.clone(),
                bank_identifier: payload.receiver_bank_code.clone(),
                display_name: payload.receiver_name.clone(),
            },
            occurred_at: payload.transaction_time,
            metadata: serde_json::json!({
                "description": payload.description,
            }),
        },
        map_vietqr_status(&payload.status),
    )
}

fn canonicalize_napas_payment(
    tenant_id: &str,
    payload: &NapasWebhookPayload,
    amount: Decimal,
    tx_time: Option<DateTime<Utc>>,
) -> CanonicalPaymentRecord {
    CanonicalPaymentRecord::from_input(
        CanonicalPaymentInput {
            provider: "napas".to_string(),
            provider_reference: payload.trans_id.clone(),
            reference_code: payload.ref_no.clone(),
            tenant_id: tenant_id.to_string(),
            direction: CanonicalPaymentDirection::Inbound,
            amount,
            currency: payload.currency.clone(),
            raw_status: Some(payload.status_code.clone()),
            payer: CanonicalPaymentParty {
                account_id: payload.debtor_account.clone(),
                bank_identifier: payload.debtor_bic.clone(),
                display_name: payload.debtor_name.clone(),
            },
            beneficiary: CanonicalPaymentParty {
                account_id: payload.creditor_account.clone(),
                bank_identifier: payload.creditor_bic.clone(),
                display_name: payload.creditor_name.clone(),
            },
            occurred_at: tx_time,
            metadata: serde_json::json!({}),
        },
        map_napas_status(&payload.status_code),
    )
}

fn canonicalize_generic_bank_payment(
    tenant_id: &str,
    provider: &str,
    payload: &GenericBankWebhookPayload,
) -> CanonicalPaymentRecord {
    let raw_status = payload
        .metadata
        .get("status")
        .and_then(|value| value.as_str())
        .map(ToString::to_string);

    CanonicalPaymentRecord::from_input(
        CanonicalPaymentInput {
            provider: provider.to_string(),
            provider_reference: payload.bank_tx_id.clone(),
            reference_code: payload.reference_code.clone(),
            tenant_id: tenant_id.to_string(),
            direction: CanonicalPaymentDirection::Inbound,
            amount: payload.amount,
            currency: payload.currency.clone(),
            raw_status: raw_status.clone(),
            payer: CanonicalPaymentParty {
                account_id: payload.sender_account.clone(),
                bank_identifier: None,
                display_name: payload.sender_name.clone(),
            },
            beneficiary: CanonicalPaymentParty {
                account_id: payload.receiver_account.clone(),
                bank_identifier: None,
                display_name: None,
            },
            occurred_at: payload.transaction_time,
            metadata: payload.metadata.clone(),
        },
        map_generic_bank_status(raw_status.as_deref(), &payload.metadata),
    )
}

// ============================================================================
// Provider-specific handlers
// ============================================================================

async fn handle_vietqr_webhook(
    state: BankWebhookState,
    payload: VietQrWebhookPayload,
    verified_tenant: Option<String>,
) -> Result<VietQrWebhookResponse, ApiError> {
    info!(
        transaction_id = %payload.transaction_id,
        reference_code = %payload.reference_code,
        amount = payload.amount,
        "Processing VietQR webhook"
    );

    if state
        .confirmation_repo
        .check_duplicate("vietqr", &payload.transaction_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
    {
        return Ok(VietQrWebhookResponse {
            success: true,
            message: "Duplicate transaction, already processed".to_string(),
            confirmation_id: None,
        });
    }

    let signature_verified = verified_tenant.is_some();

    let tenant_id = verified_tenant
        .or_else(|| extract_tenant_from_reference(&payload.reference_code))
        .ok_or_else(|| ApiError::BadRequest("Unknown tenant".to_string()))?;

    let canonical_payment = canonicalize_vietqr_payment(&tenant_id, &payload);
    info!(
        provider = "vietqr",
        reference_code = %canonical_payment.reference_code,
        status_family = canonical_payment.status_family.as_str(),
        "Normalized canonical payment from VietQR webhook"
    );

    let raw_payload = serde_json::to_value(&payload)
        .map_err(|_| ApiError::Internal("Serialization failed".to_string()))?;

    let req = CreateBankConfirmationRequest {
        tenant_id: tenant_id.clone(),
        provider: "vietqr".to_string(),
        reference_code: payload.reference_code.clone(),
        bank_reference: Some(payload.transaction_id.clone()),
        bank_tx_id: Some(payload.transaction_id.clone()),
        amount: Decimal::from(payload.amount),
        currency: payload.currency,
        sender_account: payload.sender_account,
        sender_name: payload.sender_name,
        receiver_account: payload.receiver_account,
        receiver_name: payload.receiver_name,
        webhook_signature: None,
        webhook_signature_verified: signature_verified,
        raw_payload,
        transaction_time: payload.transaction_time,
    };

    let confirmation = state
        .confirmation_repo
        .create(&req)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    info!(
        confirmation_id = %confirmation.id,
        "VietQR confirmation stored"
    );

    Ok(VietQrWebhookResponse {
        success: true,
        message: "Confirmation received".to_string(),
        confirmation_id: Some(confirmation.id),
    })
}

async fn handle_napas_webhook(
    state: BankWebhookState,
    payload: NapasWebhookPayload,
    verified_tenant: Option<String>,
) -> Result<NapasWebhookResponse, ApiError> {
    info!(
        trans_id = %payload.trans_id,
        ref_no = %payload.ref_no,
        "Processing Napas webhook"
    );

    if state
        .confirmation_repo
        .check_duplicate("napas", &payload.trans_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
    {
        return Ok(NapasWebhookResponse {
            response_code: "00".to_string(),
            response_message: "Duplicate transaction".to_string(),
        });
    }

    let amount: Decimal = payload
        .amount
        .parse()
        .map_err(|_| ApiError::BadRequest("Invalid amount format".to_string()))?;

    let tx_time = payload.tx_datetime.as_ref().and_then(|s| {
        DateTime::parse_from_rfc3339(s)
            .ok()
            .map(|dt| dt.with_timezone(&Utc))
    });

    let signature_verified = verified_tenant.is_some();

    let tenant_id = verified_tenant
        .or_else(|| extract_tenant_from_reference(&payload.ref_no))
        .ok_or_else(|| ApiError::BadRequest("Unknown tenant".to_string()))?;

    let canonical_payment = canonicalize_napas_payment(&tenant_id, &payload, amount, tx_time);
    info!(
        provider = "napas",
        reference_code = %canonical_payment.reference_code,
        status_family = canonical_payment.status_family.as_str(),
        "Normalized canonical payment from Napas webhook"
    );

    let raw_payload = serde_json::to_value(&payload)
        .map_err(|_| ApiError::Internal("Serialization failed".to_string()))?;

    let req = CreateBankConfirmationRequest {
        tenant_id: tenant_id.clone(),
        provider: "napas".to_string(),
        reference_code: payload.ref_no.clone(),
        bank_reference: Some(payload.trans_id.clone()),
        bank_tx_id: Some(payload.trans_id.clone()),
        amount,
        currency: payload.currency,
        sender_account: payload.debtor_account,
        sender_name: payload.debtor_name,
        receiver_account: payload.creditor_account,
        receiver_name: payload.creditor_name,
        webhook_signature: None,
        webhook_signature_verified: signature_verified,
        raw_payload,
        transaction_time: tx_time,
    };

    let confirmation = state
        .confirmation_repo
        .create(&req)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    info!(
        confirmation_id = %confirmation.id,
        "Napas confirmation stored"
    );

    Ok(NapasWebhookResponse {
        response_code: "00".to_string(),
        response_message: "Success".to_string(),
    })
}

async fn handle_generic_webhook(
    state: BankWebhookState,
    provider: &str,
    payload: GenericBankWebhookPayload,
    verified_tenant: Option<String>,
) -> Result<GenericWebhookResponse, ApiError> {
    info!(
        provider = %provider,
        bank_tx_id = %payload.bank_tx_id,
        reference_code = %payload.reference_code,
        "Processing generic bank webhook"
    );

    if state
        .confirmation_repo
        .check_duplicate(provider, &payload.bank_tx_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
    {
        return Ok(GenericWebhookResponse {
            success: true,
            message: "Duplicate transaction".to_string(),
            confirmation_id: None,
        });
    }

    let signature_verified = verified_tenant.is_some();

    let tenant_id = verified_tenant
        .or_else(|| extract_tenant_from_reference(&payload.reference_code))
        .ok_or_else(|| ApiError::BadRequest("Unknown tenant".to_string()))?;

    let canonical_payment = canonicalize_generic_bank_payment(&tenant_id, provider, &payload);
    info!(
        provider = %provider,
        reference_code = %canonical_payment.reference_code,
        status_family = canonical_payment.status_family.as_str(),
        "Normalized canonical payment from generic bank webhook"
    );

    let raw_payload = serde_json::to_value(&payload)
        .map_err(|_| ApiError::Internal("Serialization failed".to_string()))?;

    let req = CreateBankConfirmationRequest {
        tenant_id: tenant_id.clone(),
        provider: provider.to_string(),
        reference_code: payload.reference_code.clone(),
        bank_reference: Some(payload.bank_tx_id.clone()),
        bank_tx_id: Some(payload.bank_tx_id.clone()),
        amount: payload.amount,
        currency: payload.currency,
        sender_account: payload.sender_account,
        sender_name: payload.sender_name,
        receiver_account: payload.receiver_account,
        receiver_name: None,
        webhook_signature: None,
        webhook_signature_verified: signature_verified,
        raw_payload,
        transaction_time: payload.transaction_time,
    };

    let confirmation = state
        .confirmation_repo
        .create(&req)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    info!(
        confirmation_id = %confirmation.id,
        "Generic bank confirmation stored"
    );

    Ok(GenericWebhookResponse {
        success: true,
        message: "Confirmation received".to_string(),
        confirmation_id: Some(confirmation.id),
    })
}

/// Extract tenant ID from reference code
/// Reference codes may be formatted as: TENANT_xxxxx or TENANT-REF-xxxxx
fn extract_tenant_from_reference(reference: &str) -> Option<String> {
    // Try TENANT_xxx format
    if let Some(idx) = reference.find('_') {
        let tenant = &reference[..idx];
        if !tenant.is_empty() && tenant.len() <= 64 {
            return Some(tenant.to_string());
        }
    }

    // Try TENANT-REF-xxx format
    let parts: Vec<&str> = reference.split('-').collect();
    if parts.len() >= 2 && !parts[0].is_empty() {
        return Some(parts[0].to_string());
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_tenant_from_reference() {
        assert_eq!(
            extract_tenant_from_reference("TENANT1_REF123456"),
            Some("TENANT1".to_string())
        );
        assert_eq!(
            extract_tenant_from_reference("MYEXCHANGE-REF-123456"),
            Some("MYEXCHANGE".to_string())
        );
        assert_eq!(extract_tenant_from_reference("NOPREFIX"), None);
    }

    #[test]
    fn test_vietqr_payload_deserialization() {
        let json = r#"{
            "transactionId": "VQR123456",
            "referenceCode": "TENANT1_REF001",
            "amount": 1000000,
            "currency": "VND",
            "senderBankCode": "VCB",
            "senderAccount": "1234567890",
            "senderName": "NGUYEN VAN A",
            "status": "SUCCESS"
        }"#;

        let payload: VietQrWebhookPayload =
            serde_json::from_str(json).expect("deserialization failed");
        assert_eq!(payload.transaction_id, "VQR123456");
        assert_eq!(payload.reference_code, "TENANT1_REF001");
        assert_eq!(payload.amount, 1000000);
    }

    #[test]
    fn test_napas_payload_deserialization() {
        let json = r#"{
            "transId": "NAPAS123456",
            "refNo": "TENANT1-REF-001",
            "amount": "1000000.00",
            "currency": "VND",
            "dbtrBIC": "VCBVVNVX",
            "dbtrAcct": "1234567890",
            "dbtrNm": "NGUYEN VAN A",
            "stsCode": "00"
        }"#;

        let payload: NapasWebhookPayload =
            serde_json::from_str(json).expect("deserialization failed");
        assert_eq!(payload.trans_id, "NAPAS123456");
        assert_eq!(payload.ref_no, "TENANT1-REF-001");
    }

    #[test]
    fn test_canonicalize_vietqr_success_maps_to_settled() {
        let payload = VietQrWebhookPayload {
            transaction_id: "VQR123".to_string(),
            reference_code: "TENANT1_REF001".to_string(),
            amount: 1000000,
            currency: "VND".to_string(),
            sender_bank_code: Some("VCB".to_string()),
            sender_account: Some("1234567890".to_string()),
            sender_name: Some("NGUYEN VAN A".to_string()),
            receiver_bank_code: Some("TCB".to_string()),
            receiver_account: Some("99887766".to_string()),
            receiver_name: Some("RampOS".to_string()),
            description: Some("invoice".to_string()),
            transaction_time: None,
            status: "SUCCESS".to_string(),
        };

        let record = canonicalize_vietqr_payment("TENANT1", &payload);
        assert_eq!(record.provider, "vietqr");
        assert_eq!(record.status_family.as_str(), "settled");
        assert_eq!(record.direction, CanonicalPaymentDirection::Inbound);
    }

    #[test]
    fn test_canonicalize_napas_review_status_family() {
        let payload = NapasWebhookPayload {
            trans_id: "NAPAS123".to_string(),
            ref_no: "TENANT1-REF-001".to_string(),
            amount: "1000000.00".to_string(),
            currency: "VND".to_string(),
            debtor_bic: Some("VCBVVNVX".to_string()),
            debtor_account: Some("1234567890".to_string()),
            debtor_name: Some("NGUYEN VAN A".to_string()),
            creditor_bic: Some("TCBVVNVX".to_string()),
            creditor_account: Some("99887766".to_string()),
            creditor_name: Some("RampOS".to_string()),
            tx_datetime: None,
            status_code: "94".to_string(),
        };

        let record = canonicalize_napas_payment(
            "TENANT1",
            &payload,
            Decimal::from(1_000_000_i64),
            None,
        );
        assert_eq!(record.provider, "napas");
        assert_eq!(record.status_family.as_str(), "review");
    }

    #[test]
    fn test_canonicalize_generic_payload_uses_metadata_status() {
        let payload = GenericBankWebhookPayload {
            bank_tx_id: "GEN123".to_string(),
            reference_code: "TENANT1_REF001".to_string(),
            amount: Decimal::from(500000_i64),
            currency: "VND".to_string(),
            sender_account: Some("111222".to_string()),
            sender_name: Some("A".to_string()),
            receiver_account: Some("333444".to_string()),
            transaction_time: None,
            metadata: serde_json::json!({
                "status": "returned",
                "channel": "manual_import"
            }),
        };

        let record = canonicalize_generic_bank_payment("TENANT1", "custom-bank", &payload);
        assert_eq!(record.provider, "custom-bank");
        assert_eq!(record.status_family.as_str(), "returned");
        assert_eq!(record.metadata["channel"], "manual_import");
    }
}
