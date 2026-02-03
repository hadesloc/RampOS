//! Withdraw Policy Data Provider
//!
//! Provides withdrawal history data to the WithdrawPolicyEngine from the Intent Repository.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use ramp_common::{
    types::{TenantId, UserId},
    Result,
};
use ramp_compliance::WithdrawPolicyDataProvider;
use rust_decimal::Decimal;
use std::sync::Arc;

use crate::repository::intent::IntentRepository;

/// Data provider that uses the Intent Repository to provide withdrawal history
/// for the WithdrawPolicyEngine to perform policy checks.
pub struct IntentBasedWithdrawPolicyDataProvider {
    intent_repo: Arc<dyn IntentRepository>,
    // Note: Conversion rates from crypto to VND would come from a price feed in production.
    // For now, we store amounts directly in VND equivalent when querying.
}

impl IntentBasedWithdrawPolicyDataProvider {
    pub fn new(intent_repo: Arc<dyn IntentRepository>) -> Self {
        Self { intent_repo }
    }
}

#[async_trait]
impl WithdrawPolicyDataProvider for IntentBasedWithdrawPolicyDataProvider {
    async fn get_daily_withdraw_amount(
        &self,
        tenant_id: &TenantId,
        user_id: &UserId,
    ) -> Result<Decimal> {
        // The intent repository stores amounts in crypto units.
        // In a production system, we would convert to VND using real-time rates.
        // For now, we return the raw amount (policy engine handles conversion).
        self.intent_repo
            .get_daily_withdraw_amount(tenant_id, user_id)
            .await
    }

    async fn get_monthly_withdraw_amount(
        &self,
        tenant_id: &TenantId,
        user_id: &UserId,
    ) -> Result<Decimal> {
        self.intent_repo
            .get_monthly_withdraw_amount(tenant_id, user_id)
            .await
    }

    async fn get_hourly_withdraw_count(
        &self,
        tenant_id: &TenantId,
        user_id: &UserId,
    ) -> Result<u32> {
        self.intent_repo
            .get_hourly_withdraw_count(tenant_id, user_id)
            .await
    }

    async fn get_daily_withdraw_count(
        &self,
        tenant_id: &TenantId,
        user_id: &UserId,
    ) -> Result<u32> {
        self.intent_repo
            .get_daily_withdraw_count(tenant_id, user_id)
            .await
    }

    async fn get_last_withdraw_time(
        &self,
        tenant_id: &TenantId,
        user_id: &UserId,
    ) -> Result<Option<DateTime<Utc>>> {
        self.intent_repo
            .get_last_withdraw_time(tenant_id, user_id)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository::intent::IntentRow;
    use crate::test_utils::MockIntentRepository;
    use chrono::Duration;
    use rust_decimal_macros::dec;

    #[tokio::test]
    async fn test_get_daily_withdraw_amount_empty() {
        let intent_repo = Arc::new(MockIntentRepository::new());
        let provider = IntentBasedWithdrawPolicyDataProvider::new(intent_repo);

        let amount = provider
            .get_daily_withdraw_amount(&TenantId::new("tenant1"), &UserId::new("user1"))
            .await
            .unwrap();

        assert_eq!(amount, Decimal::ZERO);
    }

    #[tokio::test]
    async fn test_get_daily_withdraw_amount_with_intents() {
        let intent_repo = Arc::new(MockIntentRepository::new());

        // Add a withdrawal intent
        let intent = IntentRow {
            id: "withdraw-1".to_string(),
            tenant_id: "tenant1".to_string(),
            user_id: "user1".to_string(),
            intent_type: "WITHDRAW_ONCHAIN".to_string(),
            state: "POLICY_APPROVED".to_string(),
            state_history: serde_json::json!([]),
            amount: dec!(1.5),
            currency: "ETH".to_string(),
            actual_amount: None,
            rails_provider: None,
            reference_code: None,
            bank_tx_id: None,
            chain_id: Some("1".to_string()),
            tx_hash: None,
            from_address: None,
            to_address: Some("0x1234".to_string()),
            metadata: serde_json::json!({}),
            idempotency_key: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            expires_at: None,
            completed_at: None,
        };
        intent_repo.create(&intent).await.unwrap();

        let provider = IntentBasedWithdrawPolicyDataProvider::new(intent_repo);

        let amount = provider
            .get_daily_withdraw_amount(&TenantId::new("tenant1"), &UserId::new("user1"))
            .await
            .unwrap();

        assert_eq!(amount, dec!(1.5));
    }

    #[tokio::test]
    async fn test_get_hourly_withdraw_count() {
        let intent_repo = Arc::new(MockIntentRepository::new());

        // Add multiple withdrawal intents
        for i in 0..3 {
            let intent = IntentRow {
                id: format!("withdraw-{}", i),
                tenant_id: "tenant1".to_string(),
                user_id: "user1".to_string(),
                intent_type: "WITHDRAW_ONCHAIN".to_string(),
                state: "POLICY_APPROVED".to_string(),
                state_history: serde_json::json!([]),
                amount: dec!(1.0),
                currency: "ETH".to_string(),
                actual_amount: None,
                rails_provider: None,
                reference_code: None,
                bank_tx_id: None,
                chain_id: Some("1".to_string()),
                tx_hash: None,
                from_address: None,
                to_address: Some("0x1234".to_string()),
                metadata: serde_json::json!({}),
                idempotency_key: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                expires_at: None,
                completed_at: None,
            };
            intent_repo.create(&intent).await.unwrap();
        }

        let provider = IntentBasedWithdrawPolicyDataProvider::new(intent_repo);

        let count = provider
            .get_hourly_withdraw_count(&TenantId::new("tenant1"), &UserId::new("user1"))
            .await
            .unwrap();

        assert_eq!(count, 3);
    }

    #[tokio::test]
    async fn test_get_last_withdraw_time() {
        let intent_repo = Arc::new(MockIntentRepository::new());
        let provider = IntentBasedWithdrawPolicyDataProvider::new(intent_repo.clone());

        // No withdrawals initially
        let last_time = provider
            .get_last_withdraw_time(&TenantId::new("tenant1"), &UserId::new("user1"))
            .await
            .unwrap();
        assert!(last_time.is_none());

        // Add a withdrawal
        let now = Utc::now();
        let intent = IntentRow {
            id: "withdraw-1".to_string(),
            tenant_id: "tenant1".to_string(),
            user_id: "user1".to_string(),
            intent_type: "WITHDRAW_ONCHAIN".to_string(),
            state: "COMPLETED".to_string(),
            state_history: serde_json::json!([]),
            amount: dec!(1.0),
            currency: "ETH".to_string(),
            actual_amount: None,
            rails_provider: None,
            reference_code: None,
            bank_tx_id: None,
            chain_id: Some("1".to_string()),
            tx_hash: None,
            from_address: None,
            to_address: Some("0x1234".to_string()),
            metadata: serde_json::json!({}),
            idempotency_key: None,
            created_at: now,
            updated_at: now,
            expires_at: None,
            completed_at: Some(now),
        };
        intent_repo.create(&intent).await.unwrap();

        let last_time = provider
            .get_last_withdraw_time(&TenantId::new("tenant1"), &UserId::new("user1"))
            .await
            .unwrap();

        assert!(last_time.is_some());
    }
}
