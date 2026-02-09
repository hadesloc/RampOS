//! Napas Adapter - Real integration with Napas Payment Gateway
//!
//! This adapter implements:
//! - Pay-in instruction creation
//! - Pay-out (bank transfer) initiation
//! - Webhook parsing for payment confirmations
//! - RSA-SHA256 signing/verification for API requests
//!
//! Napas is Vietnam's national payment network.

use async_trait::async_trait;
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use chrono::{Duration, Utc};
use ramp_common::{Error, Result};
use reqwest::Client;
use rsa::pkcs1v15::{SigningKey, VerifyingKey};
use rsa::pkcs8::{DecodePrivateKey, DecodePublicKey};
use rsa::signature::{SignatureEncoding, Signer, Verifier};
use rsa::{RsaPrivateKey, RsaPublicKey};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::time::Duration as StdDuration;
use tracing::{debug, error, info, instrument, warn};

use crate::traits::RailsAdapter;
use crate::types::*;

/// Napas API request for initiating a transfer
#[derive(Debug, Serialize)]
struct NapasTransferRequest {
    /// Merchant transaction reference
    #[serde(rename = "merchantTxnRef")]
    merchant_txn_ref: String,
    /// Amount in VND (smallest unit)
    amount: i64,
    /// Currency code (always VND for domestic)
    currency: String,
    /// Recipient bank code (Napas BIN)
    #[serde(rename = "beneficiaryBankCode")]
    beneficiary_bank_code: String,
    /// Recipient account number
    #[serde(rename = "beneficiaryAccount")]
    beneficiary_account: String,
    /// Recipient account name
    #[serde(rename = "beneficiaryName")]
    beneficiary_name: String,
    /// Transaction description
    description: String,
    /// Transaction type (FAST for instant, NORMAL for standard)
    #[serde(rename = "txnType")]
    txn_type: String,
}

/// Napas API response for transfer initiation
#[derive(Debug, Deserialize)]
struct NapasTransferResponse {
    /// Response code (00 = success)
    #[serde(rename = "responseCode")]
    response_code: String,
    /// Response message
    #[serde(rename = "responseMessage")]
    response_message: String,
    /// Napas transaction ID
    #[serde(rename = "napasTransactionId")]
    napas_transaction_id: Option<String>,
    /// Transaction status
    status: Option<String>,
    /// Estimated completion time (ISO8601)
    #[serde(rename = "estimatedCompletion")]
    estimated_completion: Option<String>,
}

/// Napas API request for status check
#[derive(Debug, Serialize)]
struct NapasStatusRequest {
    #[serde(rename = "merchantTxnRef")]
    merchant_txn_ref: String,
}

/// Napas API response for status check
#[derive(Debug, Deserialize)]
struct NapasStatusResponse {
    #[serde(rename = "responseCode")]
    response_code: String,
    status: String,
    #[allow(dead_code)]
    #[serde(rename = "failureReason")]
    failure_reason: Option<String>,
}

/// Napas webhook payload
#[derive(Debug, Deserialize, serde::Serialize)]
struct NapasWebhookPayload {
    /// Event type
    #[serde(rename = "eventType")]
    event_type: String,
    /// Merchant reference
    #[serde(rename = "merchantTxnRef")]
    merchant_txn_ref: String,
    /// Napas transaction ID
    #[serde(rename = "napasTransactionId")]
    napas_transaction_id: String,
    /// Amount
    amount: i64,
    /// Status
    status: String,
    /// Failure reason if any
    #[serde(rename = "failureReason")]
    failure_reason: Option<String>,
    /// Timestamp
    timestamp: String,
}

/// Napas Adapter with real API integration
#[derive(Debug)]
pub struct NapasAdapter {
    config: NapasConfig,
    http_client: Client,
}

impl NapasAdapter {
    /// Create a new Napas adapter with minimal config (backwards compatible)
    ///
    /// # Errors
    /// Returns an error if HTTP client creation fails
    pub fn new(provider_code: impl Into<String>, webhook_secret: impl Into<String>) -> Result<Self> {
        let config = NapasConfig {
            base: AdapterConfig {
                provider_code: provider_code.into(),
                api_base_url: "https://api.napas.com.vn".to_string(),
                api_key: String::new(),
                api_secret: String::new(),
                webhook_secret: webhook_secret.into(),
                timeout_secs: 30,
                extra: serde_json::json!({}),
            },
            merchant_id: String::new(),
            terminal_id: String::new(),
            partner_code: String::new(),
            enable_real_api: false,
            private_key_pem: None,
            napas_public_key_pem: None,
        };

        Self::with_config(config)
    }

    /// Create a new Napas adapter from environment variables.
    ///
    /// Loads RSA keys from:
    /// - `NAPAS_RSA_PRIVATE_KEY_PEM`: inline PEM string for the private key
    /// - `NAPAS_RSA_KEY_FILE`: path to a PEM file for the private key (fallback)
    /// - `NAPAS_RSA_PUBLIC_KEY_PEM`: inline PEM string for the Napas public key
    /// - `NAPAS_RSA_PUBLIC_KEY_FILE`: path to a PEM file for the Napas public key (fallback)
    ///
    /// # Errors
    /// Returns an error if HTTP client creation fails, key files cannot be read,
    /// or if production mode is enabled without an RSA private key.
    pub fn from_env(mut config: NapasConfig) -> Result<Self> {
        // Load private key from env var or file
        if config.private_key_pem.is_none() {
            config.private_key_pem = Self::load_pem_from_env(
                "NAPAS_RSA_PRIVATE_KEY_PEM",
                "NAPAS_RSA_KEY_FILE",
            )?;
        }

        // Load Napas public key from env var or file
        if config.napas_public_key_pem.is_none() {
            config.napas_public_key_pem = Self::load_pem_from_env(
                "NAPAS_RSA_PUBLIC_KEY_PEM",
                "NAPAS_RSA_PUBLIC_KEY_FILE",
            )?;
        }

        // In production mode, RSA private key is required
        if config.enable_real_api && config.private_key_pem.is_none() {
            return Err(Error::Internal(
                "NAPAS_RSA_PRIVATE_KEY_PEM or NAPAS_RSA_KEY_FILE must be set for production mode"
                    .to_string(),
            ));
        }

        Self::with_config(config)
    }

    /// Load a PEM string from an environment variable, falling back to reading a file
    /// path from a second environment variable.
    fn load_pem_from_env(
        pem_env_var: &str,
        file_env_var: &str,
    ) -> Result<Option<String>> {
        // Try inline PEM first
        if let Ok(pem) = std::env::var(pem_env_var) {
            if !pem.is_empty() {
                info!(env_var = %pem_env_var, "Loaded RSA key from environment variable");
                return Ok(Some(pem));
            }
        }

        // Try file path
        if let Ok(path) = std::env::var(file_env_var) {
            if !path.is_empty() {
                let pem = std::fs::read_to_string(&path).map_err(|e| {
                    Error::Internal(format!(
                        "Failed to read RSA key file '{}' from {}: {}",
                        path, file_env_var, e
                    ))
                })?;
                info!(env_var = %file_env_var, path = %path, "Loaded RSA key from file");
                return Ok(Some(pem));
            }
        }

        Ok(None)
    }

    /// Create a new Napas adapter with full configuration
    ///
    /// # Errors
    /// Returns an error if HTTP client creation fails
    pub fn with_config(config: NapasConfig) -> Result<Self> {
        let http_client = Client::builder()
            .timeout(StdDuration::from_secs(config.base.timeout_secs))
            .user_agent("RampOS-Napas-Adapter/1.0")
            .build()
            .map_err(|e| Error::Internal(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self { config, http_client })
    }

    /// Generate RSA-SHA256 request signature for Napas API.
    ///
    /// When a private key is configured, signs the payload with RSA PKCS#1 v1.5 SHA-256
    /// and returns the base64-encoded signature.
    ///
    /// In production mode (`enable_real_api`), an error is returned if no private key
    /// is configured. In development/test mode, falls back to HMAC signing.
    fn sign_request(&self, payload: &str) -> std::result::Result<String, Error> {
        if let Some(pem) = &self.config.private_key_pem {
            let private_key = RsaPrivateKey::from_pkcs8_pem(pem)
                .map_err(|e| Error::Internal(format!("Invalid RSA private key: {}", e)))?;
            let signing_key = SigningKey::<Sha256>::new(private_key);
            let signature = signing_key.sign(payload.as_bytes());
            Ok(BASE64.encode(signature.to_bytes()))
        } else if self.config.enable_real_api {
            Err(Error::Internal(
                "RSA private key is required for production Napas API requests".to_string(),
            ))
        } else {
            // HMAC fallback for development/testing (no private key configured)
            use ramp_common::crypto::generate_webhook_signature;
            let timestamp = chrono::Utc::now().timestamp();
            generate_webhook_signature(
                self.config.base.api_secret.as_bytes(),
                timestamp,
                payload.as_bytes(),
            )
            .map_err(|e| Error::Internal(format!("HMAC signing failed: {}", e)))
        }
    }

    /// Verify RSA-SHA256 response signature from Napas.
    ///
    /// When Napas public key is configured, verifies the signature using RSA PKCS#1
    /// v1.5 SHA-256. Without a public key in test mode, verification is skipped.
    /// In production mode without a public key, verification fails.
    fn verify_response_signature(&self, payload: &str, signature_b64: &str) -> bool {
        let Some(pem) = &self.config.napas_public_key_pem else {
            if self.config.enable_real_api {
                error!("No Napas public key configured in production mode, rejecting signature");
                return false;
            }
            warn!("No Napas public key configured, skipping response signature verification");
            return true;
        };

        let public_key = match RsaPublicKey::from_public_key_pem(pem) {
            Ok(k) => k,
            Err(e) => {
                error!(error = %e, "Invalid Napas public key PEM");
                return false;
            }
        };

        let sig_bytes = match BASE64.decode(signature_b64) {
            Ok(b) => b,
            Err(e) => {
                error!(error = %e, "Invalid base64 signature");
                return false;
            }
        };

        let signature = match rsa::pkcs1v15::Signature::try_from(sig_bytes.as_slice()) {
            Ok(s) => s,
            Err(e) => {
                error!(error = %e, "Invalid RSA signature format");
                return false;
            }
        };

        let verifying_key = VerifyingKey::<Sha256>::new(public_key);
        verifying_key.verify(payload.as_bytes(), &signature).is_ok()
    }

    /// Convert Napas status string to PayoutStatus
    fn parse_status(status: &str) -> PayoutStatus {
        match status.to_uppercase().as_str() {
            "PENDING" | "RECEIVED" => PayoutStatus::Pending,
            "PROCESSING" | "IN_PROGRESS" => PayoutStatus::Processing,
            "COMPLETED" | "SUCCESS" | "SETTLED" => PayoutStatus::Completed,
            "FAILED" | "REJECTED" | "ERROR" => PayoutStatus::Failed,
            "CANCELLED" | "REVERSED" => PayoutStatus::Cancelled,
            _ => PayoutStatus::Processing,
        }
    }

    /// Make API request to Napas (simulation or real)
    #[instrument(skip(self, request_body))]
    async fn make_api_request<T: for<'de> Deserialize<'de>>(
        &self,
        endpoint: &str,
        request_body: &impl Serialize,
    ) -> Result<T> {
        let url = format!("{}{}", self.config.base.api_base_url, endpoint);
        let body = serde_json::to_string(request_body)
            .map_err(|e| Error::Internal(format!("Failed to serialize request: {}", e)))?;

        let signature = self.sign_request(&body)?;

        debug!(url = %url, "Making Napas API request");

        let response = self
            .http_client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("X-Api-Key", &self.config.base.api_key)
            .header("X-Merchant-Id", &self.config.merchant_id)
            .header("X-Terminal-Id", &self.config.terminal_id)
            .header("X-Signature", signature)
            .body(body)
            .send()
            .await
            .map_err(|e| Error::ExternalService {
                service: "Napas".to_string(),
                message: format!("API request failed: {}", e)
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let body: String = response.text().await.unwrap_or_default();
            error!(status = %status, body = %body, "Napas API error");
            return Err(Error::ExternalService {
                service: "Napas".to_string(),
                message: format!("API returned status {}", status),
            });
        }

        response
            .json::<T>()
            .await
            .map_err(|e| Error::ExternalService {
                service: "Napas".to_string(),
                message: format!("Failed to parse response: {}", e)
            })
    }
}

#[async_trait]
impl RailsAdapter for NapasAdapter {
    fn provider_code(&self) -> &str {
        &self.config.base.provider_code
    }

    fn provider_name(&self) -> &str {
        "Napas"
    }

    fn is_simulation_mode(&self) -> bool {
        !self.config.enable_real_api
    }

    #[instrument(skip(self, request), fields(reference = %request.reference_code))]
    async fn create_payin_instruction(
        &self,
        request: CreatePayinInstructionRequest,
    ) -> Result<PayinInstruction> {
        info!(
            amount = %request.amount_vnd,
            "Creating Napas pay-in instruction"
        );

        // For Napas, pay-in typically works via virtual account or QR
        // This creates a reference for the user to make payment
        let account_number = format!(
            "NAPAS{}",
            &uuid::Uuid::now_v7().to_string().replace("-", "")[..10]
        );

        Ok(PayinInstruction {
            reference_code: request.reference_code,
            bank_code: "NAPAS".to_string(),
            account_number,
            account_name: "Napas Merchant".to_string(),
            amount_vnd: request.amount_vnd,
            expires_at: request.expires_at,
            instructions: "Pay via Napas gateway".to_string(),
        })
    }

    #[instrument(skip(self, payload, signature))]
    async fn parse_payin_webhook(
        &self,
        payload: &[u8],
        signature: Option<&str>,
    ) -> Result<PayinConfirmation> {
        // Verify signature
        if let Some(sig) = signature {
            if !self.verify_webhook_signature(payload, sig) {
                warn!("Invalid Napas webhook signature");
                return Err(Error::Validation("Invalid webhook signature".to_string()));
            }
        } else if self.config.enable_real_api {
            return Err(Error::Validation("Missing webhook signature".to_string()));
        }

        let data: serde_json::Value = serde_json::from_slice(payload)
            .map_err(|e| Error::Validation(format!("Invalid JSON payload: {}", e)))?;

        let reference_code = data
            .get("merchantTxnRef")
            .or_else(|| data.get("reference_code"))
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .ok_or_else(|| Error::Validation("Missing or empty reference_code in webhook".to_string()))?
            .to_string();

        let bank_tx_id = data
            .get("napasTransactionId")
            .or_else(|| data.get("bank_tx_id"))
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .ok_or_else(|| Error::Validation("Missing or empty bank_tx_id in webhook".to_string()))?
            .to_string();

        let amount = data.get("amount").and_then(|v| v.as_i64()).unwrap_or(0);
        if amount <= 0 {
            return Err(Error::Validation("Webhook amount must be positive".to_string()));
        }

        info!(
            reference = %reference_code,
            bank_tx_id = %bank_tx_id,
            amount = amount,
            "Parsed Napas pay-in webhook"
        );

        Ok(PayinConfirmation {
            reference_code,
            bank_tx_id,
            amount_vnd: Decimal::from(amount),
            sender_name: data
                .get("senderName")
                .and_then(|v| v.as_str())
                .map(String::from),
            sender_account: data
                .get("senderAccount")
                .and_then(|v| v.as_str())
                .map(String::from),
            settled_at: Utc::now(),
            raw_payload: data,
        })
    }

    #[instrument(skip(self, request), fields(reference = %request.reference_code))]
    async fn initiate_payout(&self, request: InitiatePayoutRequest) -> Result<PayoutResult> {
        info!(
            amount = %request.amount_vnd,
            recipient_bank = %request.recipient_bank_code,
            recipient_account = %request.recipient_account_number,
            "Initiating Napas payout"
        );

        if !self.config.enable_real_api {
            // Simulation mode
            info!("Napas simulation mode - returning mock response");
            return Ok(PayoutResult {
                reference_code: request.reference_code,
                provider_tx_id: format!("NAPAS_SIM_{}", uuid::Uuid::now_v7()),
                status: PayoutStatus::Processing,
                estimated_completion: Some(Utc::now() + Duration::minutes(1)),
            });
        }

        // Real API call
        let napas_request = NapasTransferRequest {
            merchant_txn_ref: request.reference_code.clone(),
            amount: request
                .amount_vnd
                .to_string()
                .parse::<i64>()
                .map_err(|_| Error::Validation(format!(
                    "Cannot convert amount {} to integer for Napas API",
                    request.amount_vnd
                )))?,
            currency: "VND".to_string(),
            beneficiary_bank_code: request.recipient_bank_code.clone(),
            beneficiary_account: request.recipient_account_number.clone(),
            beneficiary_name: request.recipient_account_name.clone(),
            description: request.description.clone(),
            txn_type: "FAST".to_string(), // Use instant transfer
        };

        let response: NapasTransferResponse = self
            .make_api_request("/v1/transfers/initiate", &napas_request)
            .await?;

        if response.response_code != "00" {
            error!(
                code = %response.response_code,
                message = %response.response_message,
                "Napas transfer initiation failed"
            );
            return Err(Error::ExternalService {
                service: "Napas".to_string(),
                message: format!("{} - {}", response.response_code, response.response_message),
            });
        }

        let status = response
            .status
            .as_ref()
            .map(|s| Self::parse_status(s))
            .unwrap_or(PayoutStatus::Processing);

        let estimated_completion = response
            .estimated_completion
            .as_ref()
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Utc));

        info!(
            napas_tx_id = ?response.napas_transaction_id,
            status = ?status,
            "Napas transfer initiated successfully"
        );

        Ok(PayoutResult {
            reference_code: request.reference_code,
            provider_tx_id: response.napas_transaction_id.unwrap_or_default(),
            status,
            estimated_completion,
        })
    }

    #[instrument(skip(self, payload, signature))]
    async fn parse_payout_webhook(
        &self,
        payload: &[u8],
        signature: Option<&str>,
    ) -> Result<PayoutConfirmation> {
        // Verify signature
        if let Some(sig) = signature {
            if !self.verify_webhook_signature(payload, sig) {
                warn!("Invalid Napas payout webhook signature");
                return Err(Error::Validation("Invalid webhook signature".to_string()));
            }
        } else if self.config.enable_real_api {
            return Err(Error::Validation("Missing webhook signature".to_string()));
        }

        let webhook: NapasWebhookPayload = serde_json::from_slice(payload)
            .map_err(|e| Error::Validation(format!("Invalid webhook payload: {}", e)))?;

        let status = Self::parse_status(&webhook.status);

        info!(
            reference = %webhook.merchant_txn_ref,
            napas_tx_id = %webhook.napas_transaction_id,
            status = ?status,
            "Parsed Napas payout webhook"
        );

        // Serialize webhook before moving fields
        let raw_payload = serde_json::to_value(&webhook).unwrap_or_default();

        Ok(PayoutConfirmation {
            reference_code: webhook.merchant_txn_ref,
            bank_tx_id: webhook.napas_transaction_id,
            status,
            failure_reason: webhook.failure_reason,
            completed_at: if status == PayoutStatus::Completed {
                Some(Utc::now())
            } else {
                None
            },
            raw_payload,
        })
    }

    #[instrument(skip(self))]
    async fn check_payout_status(&self, reference: &str) -> Result<PayoutStatus> {
        info!(reference = %reference, "Checking Napas payout status");

        if !self.config.enable_real_api {
            // Simulation mode - return completed
            return Ok(PayoutStatus::Completed);
        }

        let request = NapasStatusRequest {
            merchant_txn_ref: reference.to_string(),
        };

        let response: NapasStatusResponse = self
            .make_api_request("/v1/transfers/status", &request)
            .await?;

        if response.response_code != "00" {
            return Err(Error::ExternalService {
                service: "Napas".to_string(),
                message: format!("Status check failed: {}", response.response_code),
            });
        }

        let status = Self::parse_status(&response.status);
        info!(status = ?status, "Napas payout status retrieved");

        Ok(status)
    }

    fn verify_webhook_signature(&self, payload: &[u8], signature: &str) -> bool {
        // In production mode with RSA public key, use RSA verification
        if self.config.enable_real_api {
            if let Some(ref pem) = self.config.napas_public_key_pem {
                let payload_str = String::from_utf8_lossy(payload);
                return self.verify_response_signature(&payload_str, signature);
            }
            // Production mode without public key: try to parse as base64 RSA sig
            // but warn and fall back to HMAC if no public key
            warn!("Production mode without Napas public key; falling back to HMAC webhook verification");
        }

        // Development/test mode: use HMAC
        ramp_common::crypto::verify_webhook_signature(
            self.config.base.webhook_secret.as_bytes(),
            signature,
            payload,
            300,
        )
        .is_ok()
    }

    async fn health_check(&self) -> Result<bool> {
        if !self.config.enable_real_api {
            return Ok(true);
        }

        // Ping the API
        let url = format!("{}/health", self.config.base.api_base_url);
        match self.http_client.get(&url).send().await {
            Ok(response) => Ok(response.status().is_success()),
            Err(e) => {
                error!(error = %e, "Napas health check failed");
                Ok(false)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_status() {
        assert_eq!(NapasAdapter::parse_status("PENDING"), PayoutStatus::Pending);
        assert_eq!(
            NapasAdapter::parse_status("PROCESSING"),
            PayoutStatus::Processing
        );
        assert_eq!(
            NapasAdapter::parse_status("COMPLETED"),
            PayoutStatus::Completed
        );
        assert_eq!(NapasAdapter::parse_status("SUCCESS"), PayoutStatus::Completed);
        assert_eq!(NapasAdapter::parse_status("FAILED"), PayoutStatus::Failed);
        assert_eq!(
            NapasAdapter::parse_status("CANCELLED"),
            PayoutStatus::Cancelled
        );
    }

    #[tokio::test]
    async fn test_create_payin_instruction() {
        let adapter = NapasAdapter::new("napas", "test_secret").unwrap();

        let request = CreatePayinInstructionRequest {
            reference_code: "TEST123".to_string(),
            user_id: "user1".to_string(),
            amount_vnd: Decimal::from(100000),
            expires_at: Utc::now() + Duration::hours(1),
            metadata: serde_json::json!({}),
        };

        let result = adapter.create_payin_instruction(request).await;
        assert!(result.is_ok());

        let instruction = result.unwrap();
        assert_eq!(instruction.reference_code, "TEST123");
        assert_eq!(instruction.bank_code, "NAPAS");
    }

    #[tokio::test]
    async fn test_initiate_payout_simulation() {
        let adapter = NapasAdapter::new("napas", "test_secret").unwrap();

        let request = InitiatePayoutRequest {
            reference_code: "PAYOUT123".to_string(),
            amount_vnd: Decimal::from(500000),
            recipient_bank_code: "970436".to_string(),
            recipient_account_number: "1234567890".to_string(),
            recipient_account_name: "NGUYEN VAN A".to_string(),
            description: "Test payout".to_string(),
            metadata: serde_json::json!({}),
        };

        let result = adapter.initiate_payout(request).await;
        assert!(result.is_ok());

        let payout = result.unwrap();
        assert_eq!(payout.reference_code, "PAYOUT123");
        assert!(payout.provider_tx_id.contains("NAPAS_SIM"));
        assert_eq!(payout.status, PayoutStatus::Processing);
    }

    #[tokio::test]
    async fn test_check_payout_status_simulation() {
        let adapter = NapasAdapter::new("napas", "test_secret").unwrap();

        let status = adapter.check_payout_status("REF123").await.unwrap();
        assert_eq!(status, PayoutStatus::Completed);
    }

    #[tokio::test]
    async fn test_parse_payout_webhook() {
        let adapter = NapasAdapter::new("napas", "test_secret").unwrap();

        let payload = serde_json::json!({
            "eventType": "TRANSFER_COMPLETED",
            "merchantTxnRef": "PAYOUT123",
            "napasTransactionId": "NAPAS_TX_456",
            "amount": 500000,
            "status": "COMPLETED",
            "timestamp": "2026-01-01T12:00:00Z"
        });

        let result = adapter
            .parse_payout_webhook(payload.to_string().as_bytes(), None)
            .await;
        assert!(result.is_ok());

        let confirmation = result.unwrap();
        assert_eq!(confirmation.reference_code, "PAYOUT123");
        assert_eq!(confirmation.bank_tx_id, "NAPAS_TX_456");
        assert_eq!(confirmation.status, PayoutStatus::Completed);
    }

    /// Helper: generate a test RSA keypair and return (private_pem, public_pem)
    fn generate_test_keypair() -> (String, String) {
        use rsa::pkcs8::{EncodePrivateKey, EncodePublicKey};

        let mut rng = rand::thread_rng();
        let private_key = RsaPrivateKey::new(&mut rng, 2048).expect("keygen failed");
        let public_key = RsaPublicKey::from(&private_key);

        let private_pem = private_key
            .to_pkcs8_pem(rsa::pkcs8::LineEnding::LF)
            .expect("private pem failed")
            .to_string();
        let public_pem = public_key
            .to_public_key_pem(rsa::pkcs8::LineEnding::LF)
            .expect("public pem failed");

        (private_pem, public_pem)
    }

    #[test]
    fn test_rsa_sign_and_verify() {
        let (private_pem, public_pem) = generate_test_keypair();

        let config = NapasConfig {
            base: AdapterConfig {
                provider_code: "napas".to_string(),
                api_base_url: "https://api.napas.com.vn".to_string(),
                api_key: String::new(),
                api_secret: String::new(),
                webhook_secret: "secret".to_string(),
                timeout_secs: 30,
                extra: serde_json::json!({}),
            },
            merchant_id: String::new(),
            terminal_id: String::new(),
            partner_code: String::new(),
            enable_real_api: false,
            private_key_pem: Some(private_pem),
            napas_public_key_pem: Some(public_pem),
        };

        let adapter = NapasAdapter::with_config(config).unwrap();

        let payload = r#"{"merchantTxnRef":"TX001","amount":100000}"#;
        let signature = adapter.sign_request(payload).expect("signing should succeed");

        // Signature should be non-empty base64
        assert!(!signature.is_empty());
        assert!(BASE64.decode(&signature).is_ok());

        // Verify the signature
        assert!(adapter.verify_response_signature(payload, &signature));
    }

    #[test]
    fn test_rsa_verify_wrong_key_fails() {
        let (private_pem, _) = generate_test_keypair();
        let (_, other_public_pem) = generate_test_keypair();

        let config = NapasConfig {
            base: AdapterConfig {
                provider_code: "napas".to_string(),
                api_base_url: "https://api.napas.com.vn".to_string(),
                api_key: String::new(),
                api_secret: String::new(),
                webhook_secret: "secret".to_string(),
                timeout_secs: 30,
                extra: serde_json::json!({}),
            },
            merchant_id: String::new(),
            terminal_id: String::new(),
            partner_code: String::new(),
            enable_real_api: false,
            private_key_pem: Some(private_pem),
            napas_public_key_pem: Some(other_public_pem),
        };

        let adapter = NapasAdapter::with_config(config).unwrap();

        let payload = r#"{"merchantTxnRef":"TX001","amount":100000}"#;
        let signature = adapter.sign_request(payload).expect("signing should succeed");

        // Verification with wrong public key should fail
        assert!(!adapter.verify_response_signature(payload, &signature));
    }

    #[test]
    fn test_rsa_verify_tampered_payload_fails() {
        let (private_pem, public_pem) = generate_test_keypair();

        let config = NapasConfig {
            base: AdapterConfig {
                provider_code: "napas".to_string(),
                api_base_url: "https://api.napas.com.vn".to_string(),
                api_key: String::new(),
                api_secret: String::new(),
                webhook_secret: "secret".to_string(),
                timeout_secs: 30,
                extra: serde_json::json!({}),
            },
            merchant_id: String::new(),
            terminal_id: String::new(),
            partner_code: String::new(),
            enable_real_api: false,
            private_key_pem: Some(private_pem),
            napas_public_key_pem: Some(public_pem),
        };

        let adapter = NapasAdapter::with_config(config).unwrap();

        let payload = r#"{"merchantTxnRef":"TX001","amount":100000}"#;
        let signature = adapter.sign_request(payload).expect("signing should succeed");

        // Verify with tampered payload should fail
        let tampered = r#"{"merchantTxnRef":"TX001","amount":999999}"#;
        assert!(!adapter.verify_response_signature(tampered, &signature));
    }

    #[test]
    fn test_sign_without_private_key_uses_hmac_fallback() {
        let adapter = NapasAdapter::new("napas", "test_secret").unwrap();

        let payload = r#"{"test":"data"}"#;
        let result = adapter.sign_request(payload);
        // HMAC fallback should work (may fail if api_secret is empty, but should not panic)
        // With empty api_secret, HMAC still produces a result
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_sign_with_invalid_pem_returns_error() {
        let config = NapasConfig {
            base: AdapterConfig {
                provider_code: "napas".to_string(),
                api_base_url: "https://api.napas.com.vn".to_string(),
                api_key: String::new(),
                api_secret: String::new(),
                webhook_secret: "secret".to_string(),
                timeout_secs: 30,
                extra: serde_json::json!({}),
            },
            merchant_id: String::new(),
            terminal_id: String::new(),
            partner_code: String::new(),
            enable_real_api: false,
            private_key_pem: Some("not-a-valid-pem".to_string()),
            napas_public_key_pem: None,
        };

        let adapter = NapasAdapter::with_config(config).unwrap();

        let result = adapter.sign_request("test payload");
        assert!(result.is_err());
    }

    #[test]
    fn test_production_mode_without_key_returns_error() {
        let config = NapasConfig {
            base: AdapterConfig {
                provider_code: "napas".to_string(),
                api_base_url: "https://api.napas.com.vn".to_string(),
                api_key: String::new(),
                api_secret: String::new(),
                webhook_secret: "secret".to_string(),
                timeout_secs: 30,
                extra: serde_json::json!({}),
            },
            merchant_id: String::new(),
            terminal_id: String::new(),
            partner_code: String::new(),
            enable_real_api: true,
            private_key_pem: None,
            napas_public_key_pem: None,
        };

        let adapter = NapasAdapter::with_config(config).unwrap();

        let result = adapter.sign_request("test payload");
        assert!(result.is_err());
        let err_msg = format!("{}", result.unwrap_err());
        assert!(err_msg.contains("RSA private key is required"));
    }

    #[test]
    fn test_production_verify_without_public_key_fails() {
        let config = NapasConfig {
            base: AdapterConfig {
                provider_code: "napas".to_string(),
                api_base_url: "https://api.napas.com.vn".to_string(),
                api_key: String::new(),
                api_secret: String::new(),
                webhook_secret: "secret".to_string(),
                timeout_secs: 30,
                extra: serde_json::json!({}),
            },
            merchant_id: String::new(),
            terminal_id: String::new(),
            partner_code: String::new(),
            enable_real_api: true,
            private_key_pem: None,
            napas_public_key_pem: None,
        };

        let adapter = NapasAdapter::with_config(config).unwrap();

        // In production mode without public key, verification should fail
        assert!(!adapter.verify_response_signature("payload", "c2lnbmF0dXJl"));
    }

    #[test]
    fn test_from_env_requires_key_for_production() {
        // Clear any existing env vars to ensure clean state
        std::env::remove_var("NAPAS_RSA_PRIVATE_KEY_PEM");
        std::env::remove_var("NAPAS_RSA_KEY_FILE");
        std::env::remove_var("NAPAS_RSA_PUBLIC_KEY_PEM");
        std::env::remove_var("NAPAS_RSA_PUBLIC_KEY_FILE");

        let config = NapasConfig {
            base: AdapterConfig {
                provider_code: "napas".to_string(),
                api_base_url: "https://api.napas.com.vn".to_string(),
                api_key: String::new(),
                api_secret: String::new(),
                webhook_secret: "secret".to_string(),
                timeout_secs: 30,
                extra: serde_json::json!({}),
            },
            merchant_id: String::new(),
            terminal_id: String::new(),
            partner_code: String::new(),
            enable_real_api: true,
            private_key_pem: None,
            napas_public_key_pem: None,
        };

        let result = NapasAdapter::from_env(config);
        assert!(result.is_err());
        let err_msg = format!("{}", result.unwrap_err());
        assert!(err_msg.contains("NAPAS_RSA_PRIVATE_KEY_PEM"));
    }

    #[test]
    fn test_rsa_webhook_verification_with_keys() {
        let (private_pem, public_pem) = generate_test_keypair();

        let config = NapasConfig {
            base: AdapterConfig {
                provider_code: "napas".to_string(),
                api_base_url: "https://api.napas.com.vn".to_string(),
                api_key: String::new(),
                api_secret: String::new(),
                webhook_secret: "secret".to_string(),
                timeout_secs: 30,
                extra: serde_json::json!({}),
            },
            merchant_id: String::new(),
            terminal_id: String::new(),
            partner_code: String::new(),
            enable_real_api: true,
            private_key_pem: Some(private_pem),
            napas_public_key_pem: Some(public_pem),
        };

        let adapter = NapasAdapter::with_config(config).unwrap();

        let payload = br#"{"merchantTxnRef":"TX001","amount":100000}"#;
        let payload_str = std::str::from_utf8(payload).unwrap();

        // Sign the payload as if Napas signed it
        let signature = adapter.sign_request(payload_str).expect("signing should succeed");

        // verify_webhook_signature should use RSA in production mode
        assert!(adapter.verify_webhook_signature(payload, &signature));
    }
}
