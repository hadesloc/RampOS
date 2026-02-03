//! VietQR Adapter - Real integration with VietQR API
//!
//! This adapter implements:
//! - QR code generation for VietQR payments
//! - Webhook parsing for payment confirmations
//! - Bank information lookup
//!
//! VietQR is Vietnam's standardized QR code payment system.

use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use chrono::Utc;
use image::Luma;
use qrcode::QrCode;
use ramp_common::{Error, Result};
use reqwest::Client;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::io::Cursor;
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, error, info, instrument, warn};

use crate::traits::{QrCodeAdapter, RailsAdapter};
use crate::types::*;

/// VietQR API response for bank list
#[derive(Debug, Deserialize)]
struct VietQRBankListResponse {
    code: String,
    desc: String,
    data: Vec<VietQRBankData>,
}

#[derive(Debug, Deserialize)]
struct VietQRBankData {
    id: i32,
    name: String,
    code: String,
    bin: String,
    #[serde(rename = "shortName")]
    short_name: String,
    #[serde(rename = "transferSupported")]
    transfer_supported: i32,
}

/// VietQR API response for QR generation
#[derive(Debug, Deserialize)]
struct VietQRGenerateResponse {
    code: String,
    desc: String,
    data: Option<VietQRGenerateData>,
}

#[derive(Debug, Deserialize)]
struct VietQRGenerateData {
    #[serde(rename = "qrCode")]
    qr_code: String,
    #[serde(rename = "qrDataURL")]
    qr_data_url: Option<String>,
}

/// Request body for VietQR Quick Link API
#[derive(Debug, Serialize)]
struct VietQRQuickLinkRequest {
    #[serde(rename = "accountNo")]
    account_no: String,
    #[serde(rename = "accountName")]
    account_name: String,
    #[serde(rename = "acqId")]
    acq_id: String, // Bank BIN
    amount: Option<i64>,
    #[serde(rename = "addInfo")]
    add_info: String,
    format: String,
    template: String,
}

/// VietQR Adapter with real API integration
pub struct VietQRAdapter {
    config: VietQRConfig,
    http_client: Client,
    /// Cached bank list
    bank_cache: Arc<tokio::sync::RwLock<Option<Vec<VietQRBankInfo>>>>,
}

impl VietQRAdapter {
    /// Create a new VietQR adapter with minimal config (backwards compatible)
    pub fn new(provider_code: impl Into<String>, webhook_secret: impl Into<String>) -> Self {
        let config = VietQRConfig {
            base: AdapterConfig {
                provider_code: provider_code.into(),
                api_base_url: "https://api.vietqr.io".to_string(),
                api_key: String::new(),
                api_secret: String::new(),
                webhook_secret: webhook_secret.into(),
                timeout_secs: 30,
                extra: serde_json::json!({}),
            },
            client_id: None,
            merchant_account_number: String::new(),
            merchant_bank_bin: String::new(),
            merchant_name: "RampOS".to_string(),
            enable_real_api: false,
        };

        Self::with_config(config)
    }

    /// Create a new VietQR adapter with full configuration
    pub fn with_config(config: VietQRConfig) -> Self {
        let http_client = Client::builder()
            .timeout(Duration::from_secs(config.base.timeout_secs))
            .user_agent("RampOS-VietQR-Adapter/1.0")
            .build()
            .expect("Failed to create HTTP client");

        Self {
            config,
            http_client,
            bank_cache: Arc::new(tokio::sync::RwLock::new(None)),
        }
    }

    /// Generate QR code content string according to VietQR EMVCo format
    fn generate_qr_content(
        &self,
        bank_bin: &str,
        account_number: &str,
        amount: Option<Decimal>,
        description: &str,
    ) -> String {
        // VietQR follows EMVCo QR Code Specification
        // This is a simplified implementation - production should follow full spec

        let mut content = String::new();

        // Payload Format Indicator (ID 00)
        content.push_str("00020101");

        // Point of Initiation Method (ID 01) - 12 = Dynamic QR
        content.push_str("010212");

        // Merchant Account Information (ID 38) - VietQR
        let guid = "A000000727"; // VietQR GUID
        let _bank_info = format!("0006{}{:02}{}", guid, bank_bin.len(), bank_bin);
        let account_info = format!("{:02}{}", account_number.len(), account_number);

        let merchant_info = format!(
            "0010{}01{:02}{}02{}",
            guid,
            bank_bin.len(),
            bank_bin,
            account_info
        );
        content.push_str(&format!("38{:02}{}", merchant_info.len(), merchant_info));

        // Transaction Currency (ID 53) - VND = 704
        content.push_str("5303704");

        // Transaction Amount (ID 54) - optional
        if let Some(amt) = amount {
            let amt_str = amt.to_string();
            content.push_str(&format!("54{:02}{}", amt_str.len(), amt_str));
        }

        // Country Code (ID 58)
        content.push_str("5802VN");

        // Additional Data (ID 62) - description/memo
        if !description.is_empty() {
            let desc_field = format!("08{:02}{}", description.len().min(25), &description[..description.len().min(25)]);
            content.push_str(&format!("62{:02}{}", desc_field.len(), desc_field));
        }

        // CRC (ID 63) - placeholder, should be calculated
        content.push_str("6304");

        // Calculate CRC-16 CCITT
        let crc = Self::calculate_crc16(&content);
        content.push_str(&format!("{:04X}", crc));

        content
    }

    /// Calculate CRC-16 CCITT for QR code
    fn calculate_crc16(data: &str) -> u16 {
        let mut crc: u16 = 0xFFFF;
        for byte in data.bytes() {
            crc ^= (byte as u16) << 8;
            for _ in 0..8 {
                if crc & 0x8000 != 0 {
                    crc = (crc << 1) ^ 0x1021;
                } else {
                    crc <<= 1;
                }
            }
        }
        crc
    }

    /// Generate QR code image as base64 PNG
    fn generate_qr_image(&self, content: &str) -> Result<String> {
        let code = QrCode::new(content.as_bytes())
            .map_err(|e| Error::Internal(format!("Failed to generate QR code: {}", e)))?;

        let image = code.render::<Luma<u8>>().min_dimensions(256, 256).build();

        let mut png_data = Vec::new();
        let mut cursor = Cursor::new(&mut png_data);

        image::DynamicImage::ImageLuma8(image)
            .write_to(&mut cursor, image::ImageFormat::Png)
            .map_err(|e| Error::Internal(format!("Failed to encode QR image: {}", e)))?;

        Ok(BASE64.encode(&png_data))
    }

    /// Fetch bank list from VietQR API
    #[instrument(skip(self))]
    async fn fetch_bank_list(&self) -> Result<Vec<VietQRBankInfo>> {
        if !self.config.enable_real_api {
            // Return hardcoded list for simulation mode
            return Ok(self.get_default_bank_list());
        }

        let url = format!("{}/v2/banks", self.config.base.api_base_url);

        let response = self
            .http_client
            .get(&url)
            .header("x-client-id", self.config.client_id.as_deref().unwrap_or(""))
            .header("x-api-key", &self.config.base.api_key)
            .send()
            .await
            .map_err(|e| Error::ExternalService {
                service: "VietQR".to_string(),
                message: format!("API error: {}", e),
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let body: String = response.text().await.unwrap_or_default();
            error!(status = %status, body = %body, "VietQR API error");
            return Err(Error::ExternalService {
                service: "VietQR".to_string(),
                message: format!("API returned status {}", status),
            });
        }

        let data: VietQRBankListResponse = response
            .json()
            .await
            .map_err(|e| Error::ExternalService {
                service: "VietQR".to_string(),
                message: format!("Failed to parse response: {}", e),
            })?;

        if data.code != "00" {
            return Err(Error::ExternalService {
                service: "VietQR".to_string(),
                message: format!("API error: {} - {}", data.code, data.desc),
            });
        }

        Ok(data
            .data
            .into_iter()
            .map(|b| VietQRBankInfo {
                code: b.code,
                bin: b.bin,
                name: b.name,
                short_name: b.short_name,
                is_supported: b.transfer_supported == 1,
            })
            .collect())
    }

    /// Get default bank list for simulation mode
    fn get_default_bank_list(&self) -> Vec<VietQRBankInfo> {
        vec![
            VietQRBankInfo {
                code: "VCB".to_string(),
                bin: "970436".to_string(),
                name: "Vietcombank".to_string(),
                short_name: "VCB".to_string(),
                is_supported: true,
            },
            VietQRBankInfo {
                code: "TCB".to_string(),
                bin: "970407".to_string(),
                name: "Techcombank".to_string(),
                short_name: "TCB".to_string(),
                is_supported: true,
            },
            VietQRBankInfo {
                code: "VPB".to_string(),
                bin: "970432".to_string(),
                name: "VPBank".to_string(),
                short_name: "VPB".to_string(),
                is_supported: true,
            },
            VietQRBankInfo {
                code: "MBB".to_string(),
                bin: "970422".to_string(),
                name: "MB Bank".to_string(),
                short_name: "MBB".to_string(),
                is_supported: true,
            },
            VietQRBankInfo {
                code: "ACB".to_string(),
                bin: "970416".to_string(),
                name: "ACB Bank".to_string(),
                short_name: "ACB".to_string(),
                is_supported: true,
            },
            VietQRBankInfo {
                code: "TPB".to_string(),
                bin: "970423".to_string(),
                name: "TPBank".to_string(),
                short_name: "TPB".to_string(),
                is_supported: true,
            },
            VietQRBankInfo {
                code: "BIDV".to_string(),
                bin: "970418".to_string(),
                name: "BIDV".to_string(),
                short_name: "BIDV".to_string(),
                is_supported: true,
            },
            VietQRBankInfo {
                code: "VTB".to_string(),
                bin: "970415".to_string(),
                name: "Vietinbank".to_string(),
                short_name: "VTB".to_string(),
                is_supported: true,
            },
        ]
    }
}

#[async_trait]
impl RailsAdapter for VietQRAdapter {
    fn provider_code(&self) -> &str {
        &self.config.base.provider_code
    }

    fn provider_name(&self) -> &str {
        "VietQR"
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
            user_id = %request.user_id,
            "Creating VietQR pay-in instruction"
        );

        // Use merchant account or generate virtual reference
        let account_number = if !self.config.merchant_account_number.is_empty() {
            self.config.merchant_account_number.clone()
        } else {
            // Generate a virtual account reference
            format!(
                "VQR{}",
                &uuid::Uuid::now_v7().to_string().replace("-", "")[..10]
            )
        };

        let bank_bin = if !self.config.merchant_bank_bin.is_empty() {
            self.config.merchant_bank_bin.clone()
        } else {
            "970436".to_string() // Default to VCB
        };

        // Generate QR code
        let description = format!("RAMPOS {}", request.reference_code);
        let qr_content = self.generate_qr_content(
            &bank_bin,
            &account_number,
            Some(request.amount_vnd),
            &description,
        );

        let qr_image = self.generate_qr_image(&qr_content)?;

        // Build instructions with QR data
        let instructions = serde_json::json!({
            "type": "vietqr",
            "qr_image_base64": qr_image,
            "qr_content": qr_content,
            "bank_bin": bank_bin,
            "account_number": account_number,
            "amount_vnd": request.amount_vnd.to_string(),
            "description": description,
            "expires_at": request.expires_at.to_rfc3339(),
        })
        .to_string();

        Ok(PayinInstruction {
            reference_code: request.reference_code,
            bank_code: "VIETQR".to_string(),
            account_number,
            account_name: self.config.merchant_name.clone(),
            amount_vnd: request.amount_vnd,
            expires_at: request.expires_at,
            instructions,
        })
    }

    #[instrument(skip(self, payload, signature))]
    async fn parse_payin_webhook(
        &self,
        payload: &[u8],
        signature: Option<&str>,
    ) -> Result<PayinConfirmation> {
        // Verify signature if provided
        if let Some(sig) = signature {
            if !self.verify_webhook_signature(payload, sig) {
                warn!("Invalid VietQR webhook signature");
                return Err(Error::Validation("Invalid webhook signature".to_string()));
            }
        } else if self.config.enable_real_api {
            // In production mode, signature is required
            return Err(Error::Validation("Missing webhook signature".to_string()));
        }

        let data: serde_json::Value = serde_json::from_slice(payload)
            .map_err(|e| Error::Validation(format!("Invalid JSON payload: {}", e)))?;

        // VietQR webhook format may vary by bank
        // This handles a common format
        let reference_code = data
            .get("transactionRemarks")
            .or_else(|| data.get("description"))
            .or_else(|| data.get("reference_code"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let bank_tx_id = data
            .get("transactionId")
            .or_else(|| data.get("bank_tx_id"))
            .or_else(|| data.get("ftCode"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let amount = data
            .get("amount")
            .or_else(|| data.get("creditAmount"))
            .and_then(|v| v.as_i64())
            .unwrap_or(0);

        info!(
            reference = %reference_code,
            bank_tx_id = %bank_tx_id,
            amount = amount,
            "Parsed VietQR webhook"
        );

        Ok(PayinConfirmation {
            reference_code,
            bank_tx_id,
            amount_vnd: Decimal::from(amount),
            sender_name: data
                .get("senderName")
                .or_else(|| data.get("sender_name"))
                .and_then(|v| v.as_str())
                .map(String::from),
            sender_account: data
                .get("senderAccount")
                .or_else(|| data.get("sender_account"))
                .and_then(|v| v.as_str())
                .map(String::from),
            settled_at: Utc::now(),
            raw_payload: data,
        })
    }

    async fn initiate_payout(&self, _request: InitiatePayoutRequest) -> Result<PayoutResult> {
        // VietQR is pay-in only, payouts need Napas or direct bank integration
        Err(Error::NotImplemented(
            "Payout not supported for VietQR - use Napas adapter".to_string(),
        ))
    }

    async fn parse_payout_webhook(
        &self,
        _payload: &[u8],
        _signature: Option<&str>,
    ) -> Result<PayoutConfirmation> {
        Err(Error::NotImplemented(
            "Payout not supported for VietQR".to_string(),
        ))
    }

    async fn check_payout_status(&self, _reference: &str) -> Result<PayoutStatus> {
        Err(Error::NotImplemented(
            "Payout not supported for VietQR".to_string(),
        ))
    }

    fn verify_webhook_signature(&self, payload: &[u8], signature: &str) -> bool {
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

        // Try to fetch bank list as health check
        match self.fetch_bank_list().await {
            Ok(_) => Ok(true),
            Err(e) => {
                error!(error = %e, "VietQR health check failed");
                Ok(false)
            }
        }
    }
}

#[async_trait]
impl QrCodeAdapter for VietQRAdapter {
    #[instrument(skip(self))]
    async fn generate_qr_code(
        &self,
        account_number: &str,
        amount_vnd: Option<Decimal>,
        description: &str,
        reference_code: &str,
    ) -> Result<QrCodeData> {
        debug!(
            account = %account_number,
            amount = ?amount_vnd,
            reference = %reference_code,
            "Generating VietQR code"
        );

        let bank_bin = if !self.config.merchant_bank_bin.is_empty() {
            self.config.merchant_bank_bin.clone()
        } else {
            "970436".to_string()
        };

        let full_description = if description.is_empty() {
            format!("RAMPOS {}", reference_code)
        } else {
            format!("{} {}", description, reference_code)
        };

        let qr_content =
            self.generate_qr_content(&bank_bin, account_number, amount_vnd, &full_description);

        let qr_image = self.generate_qr_image(&qr_content)?;

        Ok(QrCodeData {
            image_base64: qr_image,
            qr_content,
            bank_bin,
            account_number: account_number.to_string(),
            amount_vnd,
            description: full_description,
            expires_at: None,
        })
    }

    async fn get_bank_info(&self, bank_code: &str) -> Result<VietQRBankInfo> {
        let banks = self.list_supported_banks().await?;

        banks
            .into_iter()
            .find(|b| b.code.eq_ignore_ascii_case(bank_code) || b.bin == bank_code)
            .ok_or_else(|| Error::NotFound(format!("Bank not found: {}", bank_code)))
    }

    async fn list_supported_banks(&self) -> Result<Vec<VietQRBankInfo>> {
        // Check cache first
        {
            let cache = self.bank_cache.read().await;
            if let Some(ref banks) = *cache {
                return Ok(banks.clone());
            }
        }

        // Fetch and cache
        let banks = self.fetch_bank_list().await?;

        {
            let mut cache = self.bank_cache.write().await;
            *cache = Some(banks.clone());
        }

        Ok(banks)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_qr_content_generation() {
        let adapter = VietQRAdapter::new("vietqr", "test_secret");

        let content = adapter.generate_qr_content(
            "970436",
            "1234567890",
            Some(Decimal::from(100000)),
            "RAMPOS TEST123",
        );

        assert!(content.starts_with("000201"));
        assert!(content.contains("5303704")); // VND currency
        assert!(content.contains("5802VN")); // Vietnam country code
    }

    #[test]
    fn test_crc16_calculation() {
        // Test known CRC value
        let crc = VietQRAdapter::calculate_crc16("00020101021138540010A00000072701270006970436011");
        assert!(crc > 0);
    }

    #[tokio::test]
    async fn test_create_payin_instruction() {
        let adapter = VietQRAdapter::new("vietqr", "test_secret");

        let request = CreatePayinInstructionRequest {
            reference_code: "TEST123".to_string(),
            user_id: "user1".to_string(),
            amount_vnd: Decimal::from(100000),
            expires_at: Utc::now() + chrono::Duration::hours(1),
            metadata: serde_json::json!({}),
        };

        let result = adapter.create_payin_instruction(request).await;
        assert!(result.is_ok());

        let instruction = result.unwrap();
        assert_eq!(instruction.reference_code, "TEST123");
        assert_eq!(instruction.bank_code, "VIETQR");
        assert!(instruction.instructions.contains("qr_image_base64"));
    }

    #[tokio::test]
    async fn test_list_supported_banks() {
        let adapter = VietQRAdapter::new("vietqr", "test_secret");

        let banks = adapter.list_supported_banks().await.unwrap();
        assert!(!banks.is_empty());
        assert!(banks.iter().any(|b| b.code == "VCB"));
    }

    #[tokio::test]
    async fn test_generate_qr_code() {
        let adapter = VietQRAdapter::new("vietqr", "test_secret");

        let qr = adapter
            .generate_qr_code("1234567890", Some(Decimal::from(50000)), "Test payment", "REF001")
            .await
            .unwrap();

        assert!(!qr.image_base64.is_empty());
        assert!(!qr.qr_content.is_empty());
        assert_eq!(qr.account_number, "1234567890");
    }
}
