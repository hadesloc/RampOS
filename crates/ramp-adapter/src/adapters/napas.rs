use async_trait::async_trait;
use chrono::Utc;
use ramp_common::Result;
use rust_decimal::Decimal;

use crate::traits::RailsAdapter;
use crate::types::*;

pub struct NapasAdapter {
    provider_code: String,
    webhook_secret: String,
}

impl NapasAdapter {
    pub fn new(provider_code: impl Into<String>, webhook_secret: impl Into<String>) -> Self {
        Self {
            provider_code: provider_code.into(),
            webhook_secret: webhook_secret.into(),
        }
    }
}

#[async_trait]
impl RailsAdapter for NapasAdapter {
    fn provider_code(&self) -> &str {
        &self.provider_code
    }

    fn provider_name(&self) -> &str {
        "Napas"
    }

    async fn create_payin_instruction(
        &self,
        request: CreatePayinInstructionRequest,
    ) -> Result<PayinInstruction> {
        let account_number = format!("NAPAS{}", uuid::Uuid::now_v7().to_string().replace("-", ""));
        let account_number = if account_number.len() > 15 {
            account_number[..15].to_string()
        } else {
            account_number
        };

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

    async fn parse_payin_webhook(
        &self,
        payload: &[u8],
        signature: Option<&str>,
    ) -> Result<PayinConfirmation> {
        let signature = signature.ok_or_else(|| {
            ramp_common::Error::Validation("Missing webhook signature".to_string())
        })?;
        if !self.verify_webhook_signature(payload, signature) {
            return Err(ramp_common::Error::Validation(
                "Invalid webhook signature".to_string(),
            ));
        }

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

    async fn initiate_payout(&self, request: InitiatePayoutRequest) -> Result<PayoutResult> {
        Ok(PayoutResult {
            reference_code: request.reference_code,
            provider_tx_id: format!("NAPAS_OUT_{}", uuid::Uuid::now_v7()),
            status: PayoutStatus::Processing,
            estimated_completion: Some(Utc::now() + chrono::Duration::minutes(1)),
        })
    }

    async fn parse_payout_webhook(
        &self,
        payload: &[u8],
        signature: Option<&str>,
    ) -> Result<PayoutConfirmation> {
        let signature = signature.ok_or_else(|| {
            ramp_common::Error::Validation("Missing webhook signature".to_string())
        })?;
        if !self.verify_webhook_signature(payload, signature) {
            return Err(ramp_common::Error::Validation(
                "Invalid webhook signature".to_string(),
            ));
        }

        let data: serde_json::Value = serde_json::from_slice(payload)
            .map_err(|e| ramp_common::Error::Validation(e.to_string()))?;

        Ok(PayoutConfirmation {
            reference_code: data["reference_code"].as_str().unwrap_or("").to_string(),
            bank_tx_id: data["bank_tx_id"].as_str().unwrap_or("").to_string(),
            status: PayoutStatus::Completed,
            failure_reason: None,
            completed_at: Some(Utc::now()),
            raw_payload: data,
        })
    }

    async fn check_payout_status(&self, _reference: &str) -> Result<PayoutStatus> {
        Ok(PayoutStatus::Completed)
    }

    fn verify_webhook_signature(&self, payload: &[u8], signature: &str) -> bool {
        ramp_common::crypto::verify_webhook_signature(
            self.webhook_secret.as_bytes(),
            signature,
            payload,
            300,
        )
        .is_ok()
    }
}
