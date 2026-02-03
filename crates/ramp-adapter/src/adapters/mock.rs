//! Mock adapter for testing
//!
//! This adapter provides a fully functional mock implementation for testing
//! workflow activities without real bank/PSP integration.

use async_trait::async_trait;
use chrono::{Duration, Utc};
use ramp_common::Result;
use rust_decimal::Decimal;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

use crate::traits::RailsAdapter;
use crate::types::*;

/// Mock adapter behavior configuration
#[derive(Debug, Clone)]
pub struct MockBehavior {
    /// Delay before returning responses (simulates network latency)
    pub response_delay_ms: u64,
    /// Whether to simulate failures for testing error handling
    pub simulate_failures: bool,
    /// Failure rate (0.0 to 1.0) when simulate_failures is true
    pub failure_rate: f64,
    /// Default payout status to return
    pub default_payout_status: PayoutStatus,
}

impl Default for MockBehavior {
    fn default() -> Self {
        Self {
            response_delay_ms: 0,
            simulate_failures: false,
            failure_rate: 0.0,
            default_payout_status: PayoutStatus::Completed,
        }
    }
}

/// Mock adapter for testing
pub struct MockAdapter {
    provider_code: String,
    webhook_secret: String,
    behavior: MockBehavior,
    /// Store created pay-in instructions for later retrieval
    payin_instructions: Arc<RwLock<HashMap<String, PayinInstruction>>>,
    /// Store initiated payouts for later retrieval
    payouts: Arc<RwLock<HashMap<String, PayoutResult>>>,
    /// Store payout statuses (can be updated for testing)
    payout_statuses: Arc<RwLock<HashMap<String, PayoutStatus>>>,
}

impl MockAdapter {
    pub fn new(provider_code: impl Into<String>, webhook_secret: impl Into<String>) -> Self {
        Self {
            provider_code: provider_code.into(),
            webhook_secret: webhook_secret.into(),
            behavior: MockBehavior::default(),
            payin_instructions: Arc::new(RwLock::new(HashMap::new())),
            payouts: Arc::new(RwLock::new(HashMap::new())),
            payout_statuses: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a mock adapter with custom behavior
    pub fn with_behavior(
        provider_code: impl Into<String>,
        webhook_secret: impl Into<String>,
        behavior: MockBehavior,
    ) -> Self {
        Self {
            provider_code: provider_code.into(),
            webhook_secret: webhook_secret.into(),
            behavior,
            payin_instructions: Arc::new(RwLock::new(HashMap::new())),
            payouts: Arc::new(RwLock::new(HashMap::new())),
            payout_statuses: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Set the status for a specific payout (for testing)
    pub async fn set_payout_status(&self, reference: &str, status: PayoutStatus) {
        let mut statuses = self.payout_statuses.write().await;
        statuses.insert(reference.to_string(), status);
    }

    /// Get a stored pay-in instruction
    pub async fn get_payin_instruction(&self, reference: &str) -> Option<PayinInstruction> {
        let instructions = self.payin_instructions.read().await;
        instructions.get(reference).cloned()
    }

    /// Get a stored payout
    pub async fn get_payout(&self, reference: &str) -> Option<PayoutResult> {
        let payouts = self.payouts.read().await;
        payouts.get(reference).cloned()
    }

    /// Simulate a bank confirmation webhook for a pay-in
    pub fn create_payin_webhook_payload(
        reference_code: &str,
        amount: i64,
        bank_tx_id: &str,
    ) -> Vec<u8> {
        serde_json::json!({
            "reference_code": reference_code,
            "bank_tx_id": bank_tx_id,
            "amount": amount,
            "status": "completed",
            "sender_name": "MOCK SENDER",
            "sender_account": "1234567890"
        })
        .to_string()
        .into_bytes()
    }

    /// Simulate a payout webhook
    pub fn create_payout_webhook_payload(
        reference_code: &str,
        status: &str,
        bank_tx_id: &str,
    ) -> Vec<u8> {
        serde_json::json!({
            "reference_code": reference_code,
            "bank_tx_id": bank_tx_id,
            "status": status
        })
        .to_string()
        .into_bytes()
    }

    /// Apply simulated delay if configured
    async fn maybe_delay(&self) {
        if self.behavior.response_delay_ms > 0 {
            tokio::time::sleep(std::time::Duration::from_millis(
                self.behavior.response_delay_ms,
            ))
            .await;
        }
    }

    /// Check if this request should fail (for testing error handling)
    fn should_fail(&self) -> bool {
        if !self.behavior.simulate_failures {
            return false;
        }
        rand::random::<f64>() < self.behavior.failure_rate
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

    fn is_simulation_mode(&self) -> bool {
        true
    }

    async fn create_payin_instruction(
        &self,
        request: CreatePayinInstructionRequest,
    ) -> Result<PayinInstruction> {
        self.maybe_delay().await;

        if self.should_fail() {
            return Err(ramp_common::Error::ExternalService {
                service: "Mock".to_string(),
                message: "Simulated mock failure".to_string(),
            });
        }

        debug!(
            reference = %request.reference_code,
            amount = %request.amount_vnd,
            "Mock: Creating pay-in instruction"
        );

        let instruction = PayinInstruction {
            reference_code: request.reference_code.clone(),
            bank_code: "MOCK".to_string(),
            account_number: format!(
                "VA{}",
                uuid::Uuid::now_v7().to_string().replace("-", "")[..12].to_uppercase()
            ),
            account_name: "RAMPOS MOCK VA".to_string(),
            amount_vnd: request.amount_vnd,
            expires_at: request.expires_at,
            instructions: "Transfer to the above account number".to_string(),
        };

        // Store for later retrieval
        let mut instructions = self.payin_instructions.write().await;
        instructions.insert(request.reference_code, instruction.clone());

        info!(
            account_number = %instruction.account_number,
            "Mock: Pay-in instruction created"
        );

        Ok(instruction)
    }

    async fn parse_payin_webhook(
        &self,
        payload: &[u8],
        _signature: Option<&str>,
    ) -> Result<PayinConfirmation> {
        self.maybe_delay().await;

        let data: serde_json::Value = serde_json::from_slice(payload)
            .map_err(|e| ramp_common::Error::Validation(e.to_string()))?;

        debug!(payload = ?data, "Mock: Parsing pay-in webhook");

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
        self.maybe_delay().await;

        if self.should_fail() {
            return Err(ramp_common::Error::ExternalService {
                service: "Mock".to_string(),
                message: "Simulated mock failure".to_string(),
            });
        }

        debug!(
            reference = %request.reference_code,
            amount = %request.amount_vnd,
            recipient = %request.recipient_account_number,
            "Mock: Initiating payout"
        );

        let result = PayoutResult {
            reference_code: request.reference_code.clone(),
            provider_tx_id: format!("MOCK_{}", uuid::Uuid::now_v7()),
            status: PayoutStatus::Processing,
            estimated_completion: Some(Utc::now() + Duration::minutes(5)),
        };

        // Store for later retrieval
        let mut payouts = self.payouts.write().await;
        payouts.insert(request.reference_code.clone(), result.clone());

        // Set initial status
        let mut statuses = self.payout_statuses.write().await;
        statuses.insert(
            request.reference_code,
            self.behavior.default_payout_status,
        );

        info!(
            provider_tx_id = %result.provider_tx_id,
            "Mock: Payout initiated"
        );

        Ok(result)
    }

    async fn parse_payout_webhook(
        &self,
        payload: &[u8],
        _signature: Option<&str>,
    ) -> Result<PayoutConfirmation> {
        self.maybe_delay().await;

        let data: serde_json::Value = serde_json::from_slice(payload)
            .map_err(|e| ramp_common::Error::Validation(e.to_string()))?;

        debug!(payload = ?data, "Mock: Parsing payout webhook");

        let status = match data["status"].as_str().unwrap_or("") {
            "completed" | "success" => PayoutStatus::Completed,
            "failed" | "error" => PayoutStatus::Failed,
            "cancelled" => PayoutStatus::Cancelled,
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

    async fn check_payout_status(&self, reference: &str) -> Result<PayoutStatus> {
        self.maybe_delay().await;

        // Check if we have a specific status set for this reference
        let statuses = self.payout_statuses.read().await;
        if let Some(status) = statuses.get(reference) {
            return Ok(*status);
        }

        // Default to configured behavior
        Ok(self.behavior.default_payout_status)
    }

    fn verify_webhook_signature(&self, payload: &[u8], signature: &str) -> bool {
        use ramp_common::crypto::verify_webhook_signature;

        verify_webhook_signature(self.webhook_secret.as_bytes(), signature, payload, 300).is_ok()
    }

    async fn health_check(&self) -> Result<bool> {
        self.maybe_delay().await;
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_adapter_payin() {
        let adapter = MockAdapter::new("mock", "secret");

        let request = CreatePayinInstructionRequest {
            reference_code: "REF123".to_string(),
            user_id: "user1".to_string(),
            amount_vnd: Decimal::from(100000),
            expires_at: Utc::now() + Duration::hours(1),
            metadata: serde_json::json!({}),
        };

        let result = adapter.create_payin_instruction(request).await.unwrap();
        assert_eq!(result.reference_code, "REF123");
        assert!(result.account_number.starts_with("VA"));

        // Verify it was stored
        let stored = adapter.get_payin_instruction("REF123").await;
        assert!(stored.is_some());
    }

    #[tokio::test]
    async fn test_mock_adapter_payout() {
        let adapter = MockAdapter::new("mock", "secret");

        let request = InitiatePayoutRequest {
            reference_code: "PAYOUT123".to_string(),
            amount_vnd: Decimal::from(500000),
            recipient_bank_code: "VCB".to_string(),
            recipient_account_number: "1234567890".to_string(),
            recipient_account_name: "TEST".to_string(),
            description: "Test".to_string(),
            metadata: serde_json::json!({}),
        };

        let result = adapter.initiate_payout(request).await.unwrap();
        assert_eq!(result.reference_code, "PAYOUT123");
        assert!(result.provider_tx_id.starts_with("MOCK_"));
    }

    #[tokio::test]
    async fn test_mock_adapter_set_status() {
        let adapter = MockAdapter::new("mock", "secret");

        // Default status
        let status = adapter.check_payout_status("REF123").await.unwrap();
        assert_eq!(status, PayoutStatus::Completed);

        // Set custom status
        adapter
            .set_payout_status("REF123", PayoutStatus::Failed)
            .await;
        let status = adapter.check_payout_status("REF123").await.unwrap();
        assert_eq!(status, PayoutStatus::Failed);
    }

    #[tokio::test]
    async fn test_mock_webhook_payloads() {
        let adapter = MockAdapter::new("mock", "secret");

        // Test pay-in webhook
        let payload = MockAdapter::create_payin_webhook_payload("REF123", 100000, "TX123");
        let result = adapter.parse_payin_webhook(&payload, None).await.unwrap();
        assert_eq!(result.reference_code, "REF123");
        assert_eq!(result.amount_vnd, Decimal::from(100000));

        // Test payout webhook
        let payload = MockAdapter::create_payout_webhook_payload("PAYOUT123", "completed", "TX456");
        let result = adapter.parse_payout_webhook(&payload, None).await.unwrap();
        assert_eq!(result.reference_code, "PAYOUT123");
        assert_eq!(result.status, PayoutStatus::Completed);
    }
}
