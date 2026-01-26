//! Mock adapter for testing

use async_trait::async_trait;
use chrono::{Duration, Utc};
use ramp_common::Result;
use rust_decimal::Decimal;

use crate::traits::RailsAdapter;
use crate::types::*;

/// Mock adapter for testing
pub struct MockAdapter {
    provider_code: String,
    webhook_secret: String,
}

impl MockAdapter {
    pub fn new(provider_code: impl Into<String>, webhook_secret: impl Into<String>) -> Self {
        Self {
            provider_code: provider_code.into(),
            webhook_secret: webhook_secret.into(),
        }
    }
}

#[async_trait]
impl RailsAdapter for MockAdapter {
    fn provider_code(&self) -> &str {
        &self.provider_code
    }

    fn provider_name(&self) -> &str {
        "Mock Bank"
    }

    async fn create_payin_instruction(
        &self,
        request: CreatePayinInstructionRequest,
    ) -> Result<PayinInstruction> {
        Ok(PayinInstruction {
            reference_code: request.reference_code,
            bank_code: "MOCK".to_string(),
            account_number: format!(
                "VA{}",
                uuid::Uuid::now_v7().to_string().replace("-", "")[..12].to_uppercase()
            ),
            account_name: "RAMPOS MOCK VA".to_string(),
            amount_vnd: request.amount_vnd,
            expires_at: request.expires_at,
            instructions: "Transfer to the above account number".to_string(),
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

    async fn initiate_payout(&self, request: InitiatePayoutRequest) -> Result<PayoutResult> {
        Ok(PayoutResult {
            reference_code: request.reference_code,
            provider_tx_id: format!("MOCK_{}", uuid::Uuid::now_v7()),
            status: PayoutStatus::Processing,
            estimated_completion: Some(Utc::now() + Duration::minutes(5)),
        })
    }

    async fn parse_payout_webhook(
        &self,
        payload: &[u8],
        _signature: Option<&str>,
    ) -> Result<PayoutConfirmation> {
        let data: serde_json::Value = serde_json::from_slice(payload)
            .map_err(|e| ramp_common::Error::Validation(e.to_string()))?;

        let status = match data["status"].as_str().unwrap_or("") {
            "completed" => PayoutStatus::Completed,
            "failed" => PayoutStatus::Failed,
            _ => PayoutStatus::Processing,
        };

        Ok(PayoutConfirmation {
            reference_code: data["reference_code"].as_str().unwrap_or("").to_string(),
            bank_tx_id: data["bank_tx_id"].as_str().unwrap_or("").to_string(),
            status,
            failure_reason: data["failure_reason"].as_str().map(String::from),
            completed_at: if status == PayoutStatus::Completed {
                Some(Utc::now())
            } else {
                None
            },
            raw_payload: data,
        })
    }

    async fn check_payout_status(&self, _reference: &str) -> Result<PayoutStatus> {
        // Simulate completed status
        Ok(PayoutStatus::Completed)
    }

    fn verify_webhook_signature(&self, payload: &[u8], signature: &str) -> bool {
        use ramp_common::crypto::verify_webhook_signature;

        verify_webhook_signature(self.webhook_secret.as_bytes(), signature, payload, 300).is_ok()
    }
}
