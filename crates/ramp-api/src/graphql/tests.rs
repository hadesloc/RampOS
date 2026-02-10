//! Tests for the GraphQL API module

use async_graphql::{Request, Schema};
use chrono::Utc;
use rust_decimal_macros::dec;
use serde_json::json;
use std::sync::Arc;

use ramp_core::event::InMemoryEventPublisher;
use ramp_core::repository::intent::{IntentRepository, IntentRow};
use ramp_core::repository::user::UserRow;
use ramp_core::service::payin::PayinService;
use ramp_core::service::payout::PayoutService;
use ramp_core::service::user::UserService;
use ramp_core::test_utils::{MockIntentRepository, MockLedgerRepository, MockUserRepository};
use ramp_common::types::*;
use ramp_common::ledger::{AccountType, LedgerCurrency};

use super::mutation::MutationRoot;
use super::pagination;
use super::query::QueryRoot;
use super::subscription::{
    create_intent_event_channel, publish_intent_status, SubscriptionRoot,
};

/// Helper: create a test schema with mocked repositories
fn create_test_schema() -> (
    Schema<QueryRoot, MutationRoot, SubscriptionRoot>,
    Arc<MockIntentRepository>,
    Arc<MockUserRepository>,
    Arc<MockLedgerRepository>,
) {
    let intent_repo = Arc::new(MockIntentRepository::new());
    let user_repo = Arc::new(MockUserRepository::new());
    let ledger_repo = Arc::new(MockLedgerRepository::new());
    let event_publisher = Arc::new(InMemoryEventPublisher::new());
    let intent_event_sender = Arc::new(create_intent_event_channel());

    let payin_service = Arc::new(PayinService::new(
        intent_repo.clone(),
        ledger_repo.clone(),
        user_repo.clone(),
        event_publisher.clone(),
    ));

    let payout_service = Arc::new(PayoutService::new(
        intent_repo.clone(),
        ledger_repo.clone(),
        user_repo.clone(),
        event_publisher.clone(),
    ));

    let user_service = Arc::new(UserService::new(
        user_repo.clone(),
        event_publisher.clone(),
    ));

    let schema = Schema::build(QueryRoot, MutationRoot, SubscriptionRoot)
        .data::<Arc<dyn ramp_core::repository::intent::IntentRepository>>(intent_repo.clone())
        .data(payin_service)
        .data(payout_service)
        .data(user_service)
        .data(intent_event_sender)
        .finish();

    (schema, intent_repo, user_repo, ledger_repo)
}

/// Helper: create a test user
fn create_test_user(tenant_id: &str, user_id: &str) -> UserRow {
    UserRow {
        id: user_id.to_string(),
        tenant_id: tenant_id.to_string(),
        kyc_tier: 1,
        kyc_status: "VERIFIED".to_string(),
        kyc_verified_at: Some(Utc::now()),
        risk_score: None,
        risk_flags: json!({}),
        daily_payin_limit_vnd: None,
        daily_payout_limit_vnd: None,
        status: "ACTIVE".to_string(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

/// Helper: create a test intent
fn create_test_intent(tenant_id: &str, user_id: &str, intent_id: &str) -> IntentRow {
    IntentRow {
        id: intent_id.to_string(),
        tenant_id: tenant_id.to_string(),
        user_id: user_id.to_string(),
        intent_type: "PAYIN_VND".to_string(),
        state: "COMPLETED".to_string(),
        state_history: json!([]),
        amount: dec!(100000),
        currency: "VND".to_string(),
        actual_amount: Some(dec!(100000)),
        rails_provider: Some("VIETCOMBANK".to_string()),
        reference_code: Some("REF123".to_string()),
        bank_tx_id: Some("BANK456".to_string()),
        chain_id: None,
        tx_hash: None,
        from_address: None,
        to_address: None,
        metadata: json!({}),
        idempotency_key: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        expires_at: None,
        completed_at: Some(Utc::now()),
    }
}

// ============================================================================
// Schema Validation Tests
// ============================================================================

#[tokio::test]
async fn test_schema_builds_successfully() {
    let (schema, _, _, _) = create_test_schema();
    let sdl = schema.sdl();
    assert!(sdl.contains("type Query"));
    assert!(sdl.contains("type Mutation"));
    assert!(sdl.contains("type Subscription"));
}

#[tokio::test]
async fn test_schema_contains_intent_type() {
    let (schema, _, _, _) = create_test_schema();
    let sdl = schema.sdl();
    assert!(sdl.contains("IntentType"));
    assert!(sdl.contains("intentType"));
    assert!(sdl.contains("tenantId"));
}

#[tokio::test]
async fn test_schema_contains_user_type() {
    let (schema, _, _, _) = create_test_schema();
    let sdl = schema.sdl();
    assert!(sdl.contains("UserType"));
    assert!(sdl.contains("kycTier"));
    assert!(sdl.contains("kycStatus"));
}

// ============================================================================
// Query Tests
// ============================================================================

#[tokio::test]
async fn test_query_intent_not_found() {
    let (schema, _, _, _) = create_test_schema();

    let query = r#"
        query {
            intent(tenantId: "tenant1", id: "nonexistent") {
                id
                state
            }
        }
    "#;

    let resp = schema.execute(Request::new(query)).await;
    assert!(resp.errors.is_empty(), "Errors: {:?}", resp.errors);

    let data = resp.data.into_json().unwrap();
    assert!(data["intent"].is_null());
}

#[tokio::test]
async fn test_query_intent_found() {
    let (schema, intent_repo, _, _) = create_test_schema();

    let intent = create_test_intent("tenant1", "user1", "intent-001");
    intent_repo.create(&intent).await.unwrap();

    let query = r#"
        query {
            intent(tenantId: "tenant1", id: "intent-001") {
                id
                state
                amount
                currency
                intentType
            }
        }
    "#;

    let resp = schema.execute(Request::new(query)).await;
    assert!(resp.errors.is_empty(), "Errors: {:?}", resp.errors);

    let data = resp.data.into_json().unwrap();
    assert_eq!(data["intent"]["id"], "intent-001");
    assert_eq!(data["intent"]["state"], "COMPLETED");
    assert_eq!(data["intent"]["amount"], "100000");
    assert_eq!(data["intent"]["currency"], "VND");
    assert_eq!(data["intent"]["intentType"], "PAYIN_VND");
}

#[tokio::test]
async fn test_query_user_found() {
    let (schema, _, user_repo, _) = create_test_schema();

    let user = create_test_user("tenant1", "user-001");
    user_repo.add_user(user);

    let query = r#"
        query {
            user(tenantId: "tenant1", id: "user-001") {
                id
                kycTier
                kycStatus
                status
            }
        }
    "#;

    let resp = schema.execute(Request::new(query)).await;
    assert!(resp.errors.is_empty(), "Errors: {:?}", resp.errors);

    let data = resp.data.into_json().unwrap();
    assert_eq!(data["user"]["id"], "user-001");
    assert_eq!(data["user"]["kycTier"], 1);
    assert_eq!(data["user"]["kycStatus"], "VERIFIED");
    assert_eq!(data["user"]["status"], "ACTIVE");
}

#[tokio::test]
async fn test_query_user_not_found() {
    let (schema, _, _, _) = create_test_schema();

    let query = r#"
        query {
            user(tenantId: "tenant1", id: "nonexistent") {
                id
                status
            }
        }
    "#;

    let resp = schema.execute(Request::new(query)).await;
    assert!(resp.errors.is_empty(), "Errors: {:?}", resp.errors);

    let data = resp.data.into_json().unwrap();
    assert!(data["user"].is_null());
}

#[tokio::test]
async fn test_query_users_paginated() {
    let (schema, _, user_repo, _) = create_test_schema();

    for i in 0..5 {
        let user = create_test_user("tenant1", &format!("user-{:03}", i));
        user_repo.add_user(user);
    }

    let query = r#"
        query {
            users(tenantId: "tenant1", first: 3) {
                edges {
                    cursor
                    node {
                        id
                        status
                    }
                }
                pageInfo {
                    hasNextPage
                    endCursor
                }
                totalCount
            }
        }
    "#;

    let resp = schema.execute(Request::new(query)).await;
    assert!(resp.errors.is_empty(), "Errors: {:?}", resp.errors);

    let data = resp.data.into_json().unwrap();
    let edges = data["users"]["edges"].as_array().unwrap();
    assert!(edges.len() <= 3);
    assert!(data["users"]["pageInfo"]["endCursor"].is_string());
}

#[tokio::test]
async fn test_query_dashboard_stats() {
    let (schema, _, user_repo, _) = create_test_schema();

    user_repo.add_user(create_test_user("tenant1", "user-001"));
    user_repo.add_user(create_test_user("tenant1", "user-002"));

    let query = r#"
        query {
            dashboardStats(tenantId: "tenant1") {
                totalUsers
                activeUsers
                totalIntentsToday
            }
        }
    "#;

    let resp = schema.execute(Request::new(query)).await;
    assert!(resp.errors.is_empty(), "Errors: {:?}", resp.errors);

    let data = resp.data.into_json().unwrap();
    // totalUsers comes from list_users which returns the count
    assert!(data["dashboardStats"]["totalUsers"].is_number());
    assert!(data["dashboardStats"]["activeUsers"].is_number());
}

// ============================================================================
// Mutation Tests
// ============================================================================

#[tokio::test]
async fn test_mutation_create_payin() {
    let (schema, _, user_repo, _) = create_test_schema();

    user_repo.add_user(create_test_user("tenant1", "user-001"));

    let query = r#"
        mutation {
            createPayIn(tenantId: "tenant1", input: {
                userId: "user-001",
                amountVnd: "100000",
                railsProvider: "VIETCOMBANK"
            }) {
                intentId
                referenceCode
                status
                dailyLimit
                dailyRemaining
            }
        }
    "#;

    let resp = schema.execute(Request::new(query)).await;
    assert!(resp.errors.is_empty(), "Errors: {:?}", resp.errors);

    let data = resp.data.into_json().unwrap();
    assert!(!data["createPayIn"]["intentId"].as_str().unwrap().is_empty());
    assert!(!data["createPayIn"]["referenceCode"].as_str().unwrap().is_empty());
    assert_eq!(data["createPayIn"]["status"], "INSTRUCTION_ISSUED");
}

#[tokio::test]
async fn test_mutation_create_payout() {
    let (schema, _, user_repo, ledger_repo) = create_test_schema();

    user_repo.add_user(create_test_user("tenant1", "user-001"));

    // Give the user a balance
    ledger_repo.set_balance(
        &TenantId::new("tenant1"),
        Some(&UserId::new("user-001")),
        &AccountType::LiabilityUserVnd,
        &LedgerCurrency::VND,
        dec!(500000),
    );

    let query = r#"
        mutation {
            createPayout(tenantId: "tenant1", input: {
                userId: "user-001",
                amountVnd: "100000",
                railsProvider: "VIETCOMBANK",
                bankCode: "VCB",
                accountNumber: "123456789",
                accountName: "NGUYEN VAN A"
            }) {
                intentId
                status
                dailyLimit
                dailyRemaining
            }
        }
    "#;

    let resp = schema.execute(Request::new(query)).await;
    assert!(resp.errors.is_empty(), "Errors: {:?}", resp.errors);

    let data = resp.data.into_json().unwrap();
    assert!(!data["createPayout"]["intentId"].as_str().unwrap().is_empty());
    assert_eq!(data["createPayout"]["status"], "PAYOUT_SUBMITTED");
}

#[tokio::test]
async fn test_mutation_create_payin_invalid_amount() {
    let (schema, _, user_repo, _) = create_test_schema();

    user_repo.add_user(create_test_user("tenant1", "user-001"));

    let query = r#"
        mutation {
            createPayIn(tenantId: "tenant1", input: {
                userId: "user-001",
                amountVnd: "not-a-number",
                railsProvider: "VIETCOMBANK"
            }) {
                intentId
            }
        }
    "#;

    let resp = schema.execute(Request::new(query)).await;
    assert!(!resp.errors.is_empty(), "Expected an error for invalid amount");
}

// ============================================================================
// Pagination Tests
// ============================================================================

#[test]
fn test_cursor_roundtrip() {
    let cursor = pagination::encode_cursor(42);
    assert_eq!(pagination::decode_cursor(&cursor), Some(42));
}

#[test]
fn test_cursor_decode_invalid() {
    assert_eq!(pagination::decode_cursor("garbage"), None);
    assert_eq!(pagination::decode_cursor(""), None);
}

#[test]
fn test_cursor_encode_zero() {
    let cursor = pagination::encode_cursor(0);
    assert_eq!(pagination::decode_cursor(&cursor), Some(0));
}

// ============================================================================
// Subscription Tests
// ============================================================================

#[test]
fn test_intent_event_channel_creation() {
    let sender = create_intent_event_channel();
    let _rx = sender.subscribe();
}

#[tokio::test]
async fn test_publish_and_receive_intent_event() {
    let sender = create_intent_event_channel();
    let mut rx = sender.subscribe();

    publish_intent_status(
        &sender,
        "intent-001".to_string(),
        "tenant1".to_string(),
        "COMPLETED".to_string(),
    );

    let event = rx.recv().await.unwrap();
    assert_eq!(event.intent_id, "intent-001");
    assert_eq!(event.tenant_id, "tenant1");
    assert_eq!(event.new_status, "COMPLETED");
}

#[tokio::test]
async fn test_publish_multiple_events() {
    let sender = create_intent_event_channel();
    let mut rx = sender.subscribe();

    publish_intent_status(&sender, "i1".into(), "t1".into(), "CREATED".into());
    publish_intent_status(&sender, "i2".into(), "t1".into(), "COMPLETED".into());
    publish_intent_status(&sender, "i3".into(), "t2".into(), "PENDING".into());

    let e1 = rx.recv().await.unwrap();
    let e2 = rx.recv().await.unwrap();
    let e3 = rx.recv().await.unwrap();

    assert_eq!(e1.intent_id, "i1");
    assert_eq!(e2.intent_id, "i2");
    assert_eq!(e3.tenant_id, "t2");
}
