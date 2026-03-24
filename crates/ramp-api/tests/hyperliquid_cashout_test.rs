use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use chrono::Utc;
use jsonwebtoken::{encode, EncodingKey, Header};
use ramp_api::middleware::{PortalAuthConfig, PortalClaims};
use ramp_api::{create_router, AppState};
use ramp_compliance::{
    case::CaseManager, reports::ReportGenerator, storage::MockDocumentStorage, InMemoryCaseStore,
};
use ramp_core::event::InMemoryEventPublisher;
use ramp_core::repository::tenant::TenantRow;
use ramp_core::repository::user::UserRow;
use ramp_core::service::{
    ledger::LedgerService, onboarding::OnboardingService, payin::PayinService,
    payout::PayoutService, trade::TradeService, user::UserService,
};
use ramp_core::test_utils::*;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use std::sync::Arc;
use tower::ServiceExt;
use utoipa::OpenApi;

const TEST_JWT_SECRET: &str = "test-secret-key-for-hyperliquid-cashout";
const TEST_USER_ID: &str = "750e8400-e29b-41d4-a716-446655440000";
const TEST_TENANT_ID: &str = "860e8400-e29b-41d4-a716-446655440001";

fn create_portal_auth_config() -> Arc<PortalAuthConfig> {
    Arc::new(PortalAuthConfig {
        jwt_secret: TEST_JWT_SECRET.to_string(),
        issuer: None,
        audience: None,
        allow_missing_tenant: false,
    })
}

fn create_jwt_token() -> String {
    let now = Utc::now().timestamp();
    let claims = PortalClaims {
        sub: TEST_USER_ID.to_string(),
        tenant_id: Some(TEST_TENANT_ID.to_string()),
        email: "cashout@example.com".to_string(),
        iat: now,
        exp: now + 3600,
        token_type: "access".to_string(),
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(TEST_JWT_SECRET.as_bytes()),
    )
    .unwrap()
}

async fn setup_app() -> axum::Router {
    let intent_repo = Arc::new(MockIntentRepository::new());
    let ledger_repo = Arc::new(MockLedgerRepository::new());
    let user_repo = Arc::new(MockUserRepository::new());
    let tenant_repo = Arc::new(MockTenantRepository::new());
    let event_publisher = Arc::new(InMemoryEventPublisher::new());

    let mut hasher = Sha256::new();
    hasher.update(b"hyperliquid_cashout_api_key");
    let api_key_hash = hex::encode(hasher.finalize());

    tenant_repo.add_tenant(TenantRow {
        id: TEST_TENANT_ID.to_string(),
        name: "Hyperliquid Cashout Tenant".to_string(),
        status: "ACTIVE".to_string(),
        api_key_hash,
        api_secret_encrypted: None,
        webhook_secret_hash: "secret".to_string(),
        webhook_secret_encrypted: None,
        webhook_url: None,
        config: serde_json::json!({}),
        daily_payin_limit_vnd: None,
        daily_payout_limit_vnd: None,
        api_version: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    });

    user_repo.add_user(UserRow {
        id: TEST_USER_ID.to_string(),
        tenant_id: TEST_TENANT_ID.to_string(),
        status: "ACTIVE".to_string(),
        kyc_tier: 2,
        kyc_status: "VERIFIED".to_string(),
        kyc_verified_at: Some(Utc::now()),
        risk_score: None,
        risk_flags: serde_json::json!({}),
        daily_payin_limit_vnd: None,
        daily_payout_limit_vnd: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    });

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
    let trade_service = Arc::new(TradeService::new(
        intent_repo.clone(),
        ledger_repo.clone(),
        event_publisher.clone(),
    ));
    let ledger_service = Arc::new(LedgerService::new(ledger_repo.clone()));
    let onboarding_service = Arc::new(OnboardingService::new(
        tenant_repo.clone(),
        ledger_service.clone(),
    ));
    let user_service = Arc::new(UserService::new(user_repo.clone(), event_publisher.clone()));
    let report_generator = Arc::new(ReportGenerator::new(
        PgPool::connect_lazy("postgres://postgres:postgres@localhost/postgres")
            .expect("lazy pool"),
        Arc::new(MockDocumentStorage::new()),
    ));
    let case_manager = Arc::new(CaseManager::new(Arc::new(InMemoryCaseStore::new())));

    create_router(AppState {
        payin_service,
        payout_service,
        trade_service,
        ledger_service,
        onboarding_service,
        user_service,
        webhook_service: Arc::new(
            ramp_core::service::webhook::WebhookService::new(
                Arc::new(ramp_core::test_utils::MockWebhookRepository::new()),
                tenant_repo.clone(),
            )
            .unwrap(),
        ),
        tenant_repo: tenant_repo.clone(),
        intent_repo: intent_repo.clone(),
        report_generator,
        case_manager,
        rule_manager: None,
        rate_limiter: None,
        idempotency_handler: None,
        aa_service: None,
        portal_auth_config: create_portal_auth_config(),
        bank_confirmation_repo: None,
        licensing_repo: None,
        compliance_audit_service: None,
        sso_service: Arc::new(ramp_core::sso::SsoService::new()),
        billing_service: Arc::new(ramp_core::billing::BillingService::new(
            ramp_core::billing::BillingConfig::default(),
            Arc::new(ramp_core::billing::mock::MockBillingDataProvider::new()),
        )),
        vnst_protocol: Arc::new(
            ramp_core::stablecoin::vnst_protocol::VnstProtocolService::new(
                ramp_core::stablecoin::vnst_protocol::VnstProtocolConfig::default(),
                Arc::new(ramp_core::stablecoin::vnst_protocol::MockVnstProtocolDataProvider::new()),
            ),
        ),
        event_publisher,
        db_pool: None,
        ctr_service: None,
        ws_state: None,
        metrics_registry: Arc::new(ramp_core::service::MetricsRegistry::new()),
        document_storage: None,
        kyc_service: None,
        kyt_service: None,
    })
}

#[tokio::test]
async fn hyperliquid_cashout_prepare_path_exists_and_fails_closed_without_runtime() {
    let app = setup_app().await;

    let request = Request::builder()
        .method("POST")
        .uri("/v1/portal/venue-cashout/hyperliquid/prepare")
        .header("Authorization", format!("Bearer {}", create_jwt_token()))
        .header("Content-Type", "application/json")
        .body(Body::from(
            serde_json::json!({
                "venueConnectionId": "conn-hl-1",
                "venueAccountId": "acct-hl-1",
                "beneficiaryProfileId": "beneficiary-1",
                "walletAttestationId": "51000000-0000-0000-0000-000000000029",
                "assetSymbol": "USDT",
                "network": "ethereum",
                "amount": "125"
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
}

#[test]
fn openapi_documents_hyperliquid_cashout_paths() {
    let spec = serde_json::to_value(ramp_api::openapi::ApiDoc::openapi()).expect("spec json");
    let paths = spec["paths"].as_object().expect("paths object");

    assert!(paths.contains_key("/v1/portal/venue-cashout/hyperliquid/prepare"));
    assert!(paths.contains_key("/v1/portal/venue-cashout/{transfer_id}/wallet-received"));
}
