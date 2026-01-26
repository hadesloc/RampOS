use ramp_common::{
    ledger::{AccountType, LedgerCurrency, LedgerTransaction, LedgerEntry, EntryDirection},
    types::{TenantId, UserId, IntentId},
};
use ramp_core::{
    service::{
        onboarding::OnboardingService,
        ledger::LedgerService,
    },
    test_utils::{MockTenantRepository, MockLedgerRepository},
};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::sync::Arc;
use tokio;
use uuid::Uuid;
use chrono::Utc;

#[tokio::test]
async fn test_tenant_onboarding_flow() {
    // Setup repositories and services
    let tenant_repo = Arc::new(MockTenantRepository::new());
    let ledger_repo = Arc::new(MockLedgerRepository::new());
    let ledger_service = Arc::new(LedgerService::new(ledger_repo.clone()));
    let onboarding_service = OnboardingService::new(tenant_repo.clone(), ledger_service);

    // 1. Create tenant
    let config = serde_json::json!({
        "supported_currencies": ["VND", "USDT"],
        "timezone": "Asia/Ho_Chi_Minh"
    });

    let tenant = onboarding_service
        .create_tenant("Test Tenant", config)
        .await
        .expect("Failed to create tenant");

    assert_eq!(tenant.name, "Test Tenant");
    assert_eq!(tenant.status, "PENDING");
    let tenant_id = TenantId::new(tenant.id);

    // 2. Generate API keys
    let keys = onboarding_service
        .generate_api_keys(&tenant_id)
        .await
        .expect("Failed to generate API keys");

    assert!(keys.public_key.starts_with("pk_"));
    assert!(keys.secret_key.starts_with("sk_"));

    // Verify API key hash was stored
    let updated_tenant = tenant_repo.get_by_id(&tenant_id).await.unwrap().unwrap();
    assert!(!updated_tenant.api_key_hash.is_empty());

    // 3. Configure webhook
    onboarding_service
        .configure_webhooks(&tenant_id, "https://example.com/webhook")
        .await
        .expect("Failed to configure webhook");

    let updated_tenant = tenant_repo.get_by_id(&tenant_id).await.unwrap().unwrap();
    assert_eq!(updated_tenant.webhook_url, Some("https://example.com/webhook".to_string()));

    // 4. Set limits
    onboarding_service
        .set_limits(
            &tenant_id,
            Some(dec!(1_000_000_000)), // 1B VND payin
            Some(dec!(500_000_000)),   // 500M VND payout
        )
        .await
        .expect("Failed to set limits");

    let updated_tenant = tenant_repo.get_by_id(&tenant_id).await.unwrap().unwrap();
    assert_eq!(updated_tenant.daily_payin_limit_vnd, Some(dec!(1_000_000_000)));
    assert_eq!(updated_tenant.daily_payout_limit_vnd, Some(dec!(500_000_000)));

    // 5. Activate tenant
    onboarding_service
        .activate_tenant(&tenant_id)
        .await
        .expect("Failed to activate tenant");

    let active_tenant = tenant_repo.get_by_id(&tenant_id).await.unwrap().unwrap();
    assert_eq!(active_tenant.status, "ACTIVE");

    // 6. Suspend tenant
    onboarding_service
        .suspend_tenant(&tenant_id, "Violation of terms")
        .await
        .expect("Failed to suspend tenant");

    let suspended_tenant = tenant_repo.get_by_id(&tenant_id).await.unwrap().unwrap();
    assert_eq!(suspended_tenant.status, "SUSPENDED");
}

#[tokio::test]
async fn test_tenant_activation_failure_if_not_found() {
    let tenant_repo = Arc::new(MockTenantRepository::new());
    let ledger_repo = Arc::new(MockLedgerRepository::new());
    let ledger_service = Arc::new(LedgerService::new(ledger_repo));
    let onboarding_service = OnboardingService::new(tenant_repo, ledger_service);

    let result = onboarding_service
        .activate_tenant(&TenantId::new("non_existent"))
        .await;

    assert!(result.is_err());
}
