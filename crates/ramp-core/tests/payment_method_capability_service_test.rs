use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use ramp_common::Result;
use ramp_core::repository::{
    PaymentMethodCapabilityRecord, PaymentMethodCapabilityRepository,
    PgPaymentMethodCapabilityRepository, UpsertPaymentMethodCapabilityRequest,
};
use ramp_core::service::PaymentMethodCapabilityService;
use sqlx::PgPool;

#[derive(Default)]
struct MockPaymentMethodCapabilityRepository {
    records: Mutex<Vec<PaymentMethodCapabilityRecord>>,
}

#[async_trait]
impl PaymentMethodCapabilityRepository for MockPaymentMethodCapabilityRepository {
    async fn upsert_payment_method_capability(
        &self,
        request: &UpsertPaymentMethodCapabilityRequest,
    ) -> Result<()> {
        let mut records = self.records.lock().expect("records lock");
        records.retain(|record| {
            record.payment_method_capability_id != request.payment_method_capability_id
        });
        records.push(PaymentMethodCapabilityRecord {
            payment_method_capability_id: request.payment_method_capability_id.clone(),
            corridor_pack_id: request.corridor_pack_id.clone(),
            partner_capability_id: request.partner_capability_id.clone(),
            method_family: request.method_family.clone(),
            funding_source: request.funding_source.clone(),
            settlement_direction: request.settlement_direction.clone(),
            presentment_model: request.presentment_model.clone(),
            card_funding_enabled: request.card_funding_enabled,
            policy_flags: request.policy_flags.clone(),
            metadata: request.metadata.clone(),
        });
        Ok(())
    }

    async fn list_payment_method_capabilities(
        &self,
        corridor_pack_id: Option<&str>,
        partner_capability_id: Option<&str>,
    ) -> Result<Vec<PaymentMethodCapabilityRecord>> {
        let records = self.records.lock().expect("records lock");
        Ok(records
            .iter()
            .filter(|record| {
                corridor_pack_id
                    .map(|value| record.corridor_pack_id == value)
                    .unwrap_or(true)
                    && partner_capability_id
                        .map(|value| record.partner_capability_id.as_deref() == Some(value))
                        .unwrap_or(true)
            })
            .cloned()
            .collect())
    }
}

#[tokio::test]
async fn service_returns_empty_fallback_without_repository() {
    let service = PaymentMethodCapabilityService::new();
    let snapshot = service
        .list_capabilities(Some("corridor_vn_sg"), None)
        .await
        .expect("fallback snapshot");

    assert_eq!(snapshot.source, "fallback");
    assert!(snapshot.capabilities.is_empty());
}

#[tokio::test]
async fn service_upsert_and_list_capabilities() {
    let service = PaymentMethodCapabilityService::with_repository(Arc::new(
        MockPaymentMethodCapabilityRepository::default(),
    ));

    let snapshot = service
        .upsert_capability(&UpsertPaymentMethodCapabilityRequest {
            payment_method_capability_id: "pmc_push_transfer".to_string(),
            corridor_pack_id: "corridor_vn_sg".to_string(),
            partner_capability_id: Some("partner_capability_bank_sg".to_string()),
            method_family: "push_transfer".to_string(),
            funding_source: Some("bank_account".to_string()),
            settlement_direction: "payout".to_string(),
            presentment_model: Some("server_driven".to_string()),
            card_funding_enabled: false,
            policy_flags: serde_json::json!({"travelRule": true}),
            metadata: serde_json::json!({"optionalCardFunding": false}),
        })
        .await
        .expect("upsert snapshot");

    assert_eq!(snapshot.source, "registry");
    assert_eq!(snapshot.capabilities.len(), 1);
    assert_eq!(snapshot.capabilities[0].method_family, "push_transfer");
}

#[tokio::test]
async fn repository_round_trip_when_database_available() {
    let database_url = match std::env::var("DATABASE_URL") {
        Ok(url) => url,
        Err(_) => return,
    };

    let pool = PgPool::connect(&database_url)
        .await
        .expect("database connection should succeed");

    sqlx::migrate!("../../migrations")
        .run(&pool)
        .await
        .expect("migrations should succeed");

    sqlx::query(
        r#"
        INSERT INTO corridor_packs (
            id,
            tenant_id,
            corridor_code,
            source_market,
            destination_market,
            source_currency,
            destination_currency,
            settlement_direction,
            fee_model,
            lifecycle_state,
            rollout_state,
            eligibility_state,
            metadata
        ) VALUES (
            'corridor_vn_sg_payment_methods',
            'tenant_payment_method_capabilities',
            'VN_SG_PAYOUT_PM',
            'VN',
            'SG',
            'VND',
            'SGD',
            'payout',
            'shared',
            'pilot',
            'approved',
            'active',
            '{}'::jsonb
        )
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .execute(&pool)
    .await
    .expect("corridor pack seed");

    let repository = PgPaymentMethodCapabilityRepository::new(pool);
    repository
        .upsert_payment_method_capability(&UpsertPaymentMethodCapabilityRequest {
            payment_method_capability_id: "pmc_card_funding".to_string(),
            corridor_pack_id: "corridor_vn_sg_payment_methods".to_string(),
            partner_capability_id: None,
            method_family: "card_funding".to_string(),
            funding_source: Some("card".to_string()),
            settlement_direction: "payin".to_string(),
            presentment_model: Some("hosted".to_string()),
            card_funding_enabled: true,
            policy_flags: serde_json::json!({"disabledByDefault": true}),
            metadata: serde_json::json!({"optional": true}),
        })
        .await
        .expect("payment method capability should persist");

    let rows = repository
        .list_payment_method_capabilities(Some("corridor_vn_sg_payment_methods"), None)
        .await
        .expect("payment method capability list");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].method_family, "card_funding");
    assert!(rows[0].card_funding_enabled);
}
