use crate::aml::{AmlEngine, MockDeviceHistoryStore, TransactionData, TransactionType};
use crate::{case::CaseManager, InMemoryCaseStore, MockTransactionHistoryStore};
use crate::sanctions::MockSanctionsProvider;
use chrono::Utc;
use ramp_common::types::{IntentId, TenantId, UserId, VndAmount};
use std::sync::Arc;

#[tokio::test]
async fn test_sanctions_integration() {
    // Setup
    let case_manager = Arc::new(CaseManager::new(Arc::new(InMemoryCaseStore::new())));
    let sanctions_provider = Arc::new(MockSanctionsProvider::new());
    let device_store = Arc::new(MockDeviceHistoryStore::new());

    // Add a blocked user to the mock provider
    sanctions_provider.add_blocked_individual("Osama Bin Laden", 100.0);

    let engine = AmlEngine::new(
        case_manager,
        Some(sanctions_provider),
        device_store,
        Arc::new(MockTransactionHistoryStore::new()),
    );

    // Test Case 1: Clean user
    let clean_tx = TransactionData {
        intent_id: IntentId::new_payin(),
        tenant_id: TenantId::new("tenant1"),
        user_id: UserId::new("user1"),
        amount_vnd: VndAmount::from_i64(1_000_000),
        transaction_type: TransactionType::Payin,
        timestamp: Utc::now(),
        metadata: serde_json::json!({}),
        user_full_name: Some("John Doe".to_string()),
        user_country: Some("US".to_string()),
        user_address: None,
    };

    let result = engine.check_transaction(&clean_tx).await.unwrap();
    assert!(result.passed);
    assert!(!result.requires_review);
    assert!(result.risk_score.0 < 50.0);

    // Test Case 2: Sanctioned user
    let sanctioned_tx = TransactionData {
        intent_id: IntentId::new_payin(),
        tenant_id: TenantId::new("tenant1"),
        user_id: UserId::new("user2"),
        amount_vnd: VndAmount::from_i64(1_000_000),
        transaction_type: TransactionType::Payin,
        timestamp: Utc::now(),
        metadata: serde_json::json!({}),
        user_full_name: Some("Osama Bin Laden".to_string()),
        user_country: Some("AF".to_string()),
        user_address: None,
    };

    let result = engine.check_transaction(&sanctioned_tx).await.unwrap();
    assert!(!result.passed);
    assert!(result.requires_review);
    assert_eq!(result.risk_score.0, 100.0);
    assert!(result.flags.iter().any(|f| f.contains("Sanctions match")));

    // Test Case 3: Sanctioned address (mock)
    // NOTE: MockSanctionsProvider address check is currently clean, so this is just verification it runs
    let address_tx = TransactionData {
        intent_id: IntentId::new_payin(),
        tenant_id: TenantId::new("tenant1"),
        user_id: UserId::new("user3"),
        amount_vnd: VndAmount::from_i64(1_000_000),
        transaction_type: TransactionType::Payin,
        timestamp: Utc::now(),
        metadata: serde_json::json!({}),
        user_full_name: Some("Jane Doe".to_string()),
        user_country: Some("US".to_string()),
        user_address: Some("123 Blocked St".to_string()),
    };

    let result = engine.check_transaction(&address_tx).await.unwrap();
    assert!(result.passed);
}
