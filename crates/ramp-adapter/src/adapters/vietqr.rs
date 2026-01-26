use async_trait::async_trait;
use ramp_common::Result;
use rust_decimal::Decimal;
use chrono::Utc;

use crate::traits::RailsAdapter;
use crate::types::*;

pub struct VietQRAdapter {
    provider_code: String,
    // webhook_secret: String, // Unused
}

impl VietQRAdapter {
    pub fn new(provider_code: impl Into<String>, _webhook_secret: impl Into<String>) -> Self {
        Self {
            provider_code: provider_code.into(),
            // webhook_secret: webhook_secret.into(), // Unused
        }
    }
}

#[async_trait]
impl RailsAdapter for VietQRAdapter {
    fn provider_code(&self) -> &str {
        &self.provider_code
    }

    fn provider_name(&self) -> &str {
        "VietQR"
    }

    async fn create_payin_instruction(
        &self,
        request: CreatePayinInstructionRequest,
    ) -> Result<PayinInstruction> {
        let account_number = format!("VQR{}", uuid::Uuid::now_v7().to_string().replace("-", ""));
        let account_number = if account_number.len() > 13 {
            account_number[..13].to_string()
        } else {
            account_number
        };

        Ok(PayinInstruction {
            reference_code: request.reference_code,
            bank_code: "VIETQR".to_string(),
            account_number,
            account_name: "VietQR Merchant".to_string(),
            amount_vnd: request.amount_vnd,
            expires_at: request.expires_at,
            instructions: "Scan QR code to pay".to_string(),
        })
    }

    async fn parse_payin_webhook(
        &self,
        payload: &[u8],
        _signature: Option<&str>,
    ) -> Result<PayinConfirmation> {
        let data: serde_json::Value = serde_json::from_slice(payload)
            .map_err(|e| ramp_common::Error::Validation(e.to_string()))?;

        Ok(PayinConfirmation {
            reference_code: data["reference_code"].as_str().unwrap_or("").to_string(),
            bank_tx_id: data["bank_tx_id"].as_str().unwrap_or("").to_string(),
            amount_vnd: Decimal::from(data["amount"].as_i64().unwrap_or(0)),
            sender_name: data["sender_name"].as_str().map(String::from),
            sender_account: data["sender_account"].as_str().map(String::from),
            settled_at: Utc::now(),
            raw_payload: data,
        })
    }

    async fn initiate_payout(&self, _request: InitiatePayoutRequest) -> Result<PayoutResult> {
        Err(ramp_common::Error::NotImplemented("Payout not supported for VietQR".to_string()))
    }

    async fn parse_payout_webhook(
        &self,
        _payload: &[u8],
        _signature: Option<&str>,
    ) -> Result<PayoutConfirmation> {
        Err(ramp_common::Error::NotImplemented("Payout not supported for VietQR".to_string()))
    }

    async fn check_payout_status(&self, _reference: &str) -> Result<PayoutStatus> {
        Err(ramp_common::Error::NotImplemented("Payout not supported for VietQR".to_string()))
    }

    fn verify_webhook_signature(&self, _payload: &[u8], _signature: &str) -> bool {
        true
    }
}
