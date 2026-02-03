use axum::{
    body::Body,
    http::{Request, StatusCode},
    routing::get,
    Router,
};
use ramp_api::{handlers::intent::get_intent, middleware::tenant::TenantContext};
use ramp_common::types::{IntentId, TenantId};
use ramp_core::repository::intent::{IntentRepository, IntentRow};
use std::sync::Arc;
use tower::ServiceExt;

// Mock repository
struct MockIntentRepository;

#[async_trait::async_trait]
impl IntentRepository for MockIntentRepository {
    async fn create(&self, _intent: &IntentRow) -> ramp_common::Result<()> {
        Ok(())
    }

    async fn get_by_id(
        &self,
        _tenant_id: &TenantId,
        id: &IntentId,
    ) -> ramp_common::Result<Option<IntentRow>> {
        // If the ID contains SQL injection characters, it should still be handled safely.
        // In a real DB test, we'd check if the query executes efficiently.
        // Here we just check if it reaches here without erroring on validation if validation allows it,
        // but typically string IDs are safe if treated as strings.

        // For this mock, we pretend we found it if it's "valid_id",
        // and return None for anything else.
        if id.0 == "valid_id" {
            Ok(Some(IntentRow {
                id: "valid_id".to_string(),
                tenant_id: "tenant_123".to_string(),
                user_id: "user_123".to_string(),
                intent_type: "PAYIN".to_string(),
                state: "CREATED".to_string(),
                state_history: serde_json::json!([]),
                amount: rust_decimal::Decimal::new(100, 0),
                currency: "VND".to_string(),
                actual_amount: None,
                rails_provider: None,
                reference_code: None,
                bank_tx_id: None,
                chain_id: None,
                tx_hash: None,
                from_address: None,
                to_address: None,
                metadata: serde_json::json!({}),
                idempotency_key: None,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
                expires_at: None,
                completed_at: None,
            }))
        } else {
            Ok(None)
        }
    }

    async fn get_by_idempotency_key(
        &self,
        _tenant_id: &TenantId,
        _key: &ramp_common::types::IdempotencyKey,
    ) -> ramp_common::Result<Option<IntentRow>> {
        Ok(None)
    }

    async fn get_by_reference_code(
        &self,
        _tenant_id: &TenantId,
        _code: &ramp_common::types::ReferenceCode,
    ) -> ramp_common::Result<Option<IntentRow>> {
        Ok(None)
    }

    async fn update_state(
        &self,
        _tenant_id: &TenantId,
        _id: &IntentId,
        _new_state: &str,
    ) -> ramp_common::Result<()> {
        Ok(())
    }

    async fn update_bank_confirmed(
        &self,
        _tenant_id: &TenantId,
        _id: &IntentId,
        _bank_tx_id: &str,
        _actual_amount: rust_decimal::Decimal,
    ) -> ramp_common::Result<()> {
        Ok(())
    }

    async fn get_daily_payin_amount(
        &self,
        _tenant_id: &TenantId,
        _user_id: &ramp_common::types::UserId,
    ) -> ramp_common::Result<rust_decimal::Decimal> {
        Ok(rust_decimal::Decimal::ZERO)
    }

    async fn get_daily_payout_amount(
        &self,
        _tenant_id: &TenantId,
        _user_id: &ramp_common::types::UserId,
    ) -> ramp_common::Result<rust_decimal::Decimal> {
        Ok(rust_decimal::Decimal::ZERO)
    }

    async fn get_daily_withdraw_amount(
        &self,
        _tenant_id: &TenantId,
        _user_id: &ramp_common::types::UserId,
    ) -> ramp_common::Result<rust_decimal::Decimal> {
        Ok(rust_decimal::Decimal::ZERO)
    }

    async fn get_monthly_withdraw_amount(
        &self,
        _tenant_id: &TenantId,
        _user_id: &ramp_common::types::UserId,
    ) -> ramp_common::Result<rust_decimal::Decimal> {
        Ok(rust_decimal::Decimal::ZERO)
    }

    async fn get_hourly_withdraw_count(
        &self,
        _tenant_id: &TenantId,
        _user_id: &ramp_common::types::UserId,
    ) -> ramp_common::Result<u32> {
        Ok(0)
    }

    async fn get_daily_withdraw_count(
        &self,
        _tenant_id: &TenantId,
        _user_id: &ramp_common::types::UserId,
    ) -> ramp_common::Result<u32> {
        Ok(0)
    }

    async fn get_last_withdraw_time(
        &self,
        _tenant_id: &TenantId,
        _user_id: &ramp_common::types::UserId,
    ) -> ramp_common::Result<Option<chrono::DateTime<chrono::Utc>>> {
        Ok(None)
    }

    async fn list_by_user(
        &self,
        _tenant_id: &TenantId,
        _user_id: &ramp_common::types::UserId,
        _limit: i64,
        _offset: i64,
    ) -> ramp_common::Result<Vec<IntentRow>> {
        Ok(vec![])
    }

    async fn list_expired(&self, _limit: i64) -> ramp_common::Result<Vec<IntentRow>> {
        Ok(vec![])
    }
}

#[tokio::test]
async fn test_sql_injection_attempt() {
    let intent_repo = Arc::new(MockIntentRepository);
    let app = Router::new()
        .route("/v1/intents/:id", get(get_intent))
        .layer(axum::Extension(TenantContext {
            tenant_id: TenantId::new("tenant_123"),
            name: "Test Tenant".to_string(),
        }))
        .with_state(intent_repo);

    // Attempt SQL injection in the ID
    let injection_payload = "valid_id' OR '1'='1";
    let response = app
        .oneshot(
            Request::builder()
                .uri(&format!("/v1/intents/{}", injection_payload))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Should be Not Found because "valid_id' OR '1'='1" is treated as a literal string ID
    // and doesn't match "valid_id".
    // If injection worked, it might have returned 200 (if we were running against a real DB with vulnerable query).
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
