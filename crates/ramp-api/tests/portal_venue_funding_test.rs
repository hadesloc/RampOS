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

const TEST_JWT_SECRET: &str = "test-secret-key-for-portal-venue-funding";
const TEST_USER_ID: &str = "550e8400-e29b-41d4-a716-446655440000";
const TEST_TENANT_ID: &str = "660e8400-e29b-41d4-a716-446655440001";

fn create_portal_auth_config() -> Arc<PortalAuthConfig> {
    Arc::new(PortalAuthConfig {
        jwt_secret: TEST_JWT_SECRET.to_string(),
        issuer: None,
        audience: None,
        allow_missing_tenant: false,
    })
}

fn create_jwt_token() -> String {
    create_jwt_token_for_user(TEST_USER_ID)
}

fn create_jwt_token_for_user(user_id: &str) -> String {
    let now = Utc::now().timestamp();
    let claims = PortalClaims {
        sub: user_id.to_string(),
        tenant_id: Some(TEST_TENANT_ID.to_string()),
        email: "venue-funding@example.com".to_string(),
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
    hasher.update(b"portal_venue_funding_api_key");
    let api_key_hash = hex::encode(hasher.finalize());

    tenant_repo.add_tenant(TenantRow {
        id: TEST_TENANT_ID.to_string(),
        name: "Portal Venue Funding Tenant".to_string(),
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
        kyc_tier: 1,
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
        PgPool::connect_lazy("postgres://postgres:postgres@localhost/postgres").expect("lazy pool"),
        Arc::new(MockDocumentStorage::new()),
    ));
    let case_manager = Arc::new(CaseManager::new(Arc::new(InMemoryCaseStore::new())));

    let app_state = AppState {
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
    };

    create_router(app_state)
}

#[tokio::test]
async fn portal_venue_funding_eligibility_path_returns_decision_payload() {
    let app = setup_app().await;

    let request = Request::builder()
        .method("GET")
        .uri("/v1/portal/venue-funding/eligibility?venueKey=hyperliquid&jurisdiction=VN&asset=USDT&network=ethereum")
        .header("Authorization", format!("Bearer {}", create_jwt_token()))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["subjectType"], "user");
    assert_eq!(json["subjectId"], TEST_USER_ID);
    assert!(json.get("decision").is_some());
    assert!(json.get("reasons").is_some());
    assert!(json.get("sourceOfFunds").is_some());
}

#[tokio::test]
async fn portal_venue_funding_venues_path_returns_curated_registry_entries() {
    let app = setup_app().await;

    let request = Request::builder()
        .method("GET")
        .uri("/v1/portal/venue-funding/venues")
        .header("Authorization", format!("Bearer {}", create_jwt_token()))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let venues = json["venues"].as_array().expect("venues must be an array");

    assert!(
        venues
            .iter()
            .any(|venue| venue["venueKey"] == "hyperliquid"),
        "curated venues should include hyperliquid"
    );
    assert!(
        venues.iter().any(|venue| venue["venueKey"] == "hyperliquid"
            && venue["supportsWalletFunding"] == serde_json::Value::Bool(true)),
        "hyperliquid should be advertised as the active wallet-funding pilot"
    );
    assert!(
        venues.iter().any(|venue| venue["venueKey"] == "kraken"
            && venue["supportsWalletFunding"] == serde_json::Value::Bool(false)),
        "non-pilot venues should not be advertised as wallet-funding ready"
    );
    assert!(
        venues
            .iter()
            .all(|venue| venue.get("supportsWalletFunding").is_some()),
        "each venue should expose wallet-funding support"
    );
}

#[tokio::test]
async fn portal_venue_funding_connection_path_round_trips_into_eligibility() {
    let app = setup_app().await;

    let connection_request = Request::builder()
        .method("POST")
        .uri("/v1/portal/venue-funding/connection")
        .header("Authorization", format!("Bearer {}", create_jwt_token()))
        .header("Content-Type", "application/json")
        .body(Body::from(
            serde_json::json!({
                "venueKey": "hyperliquid",
                "jurisdiction": "VN",
                "asset": "USDT",
                "network": "ethereum"
            })
            .to_string(),
        ))
        .unwrap();

    let connection_response = app.clone().oneshot(connection_request).await.unwrap();
    assert_eq!(connection_response.status(), StatusCode::OK);

    let connection_body = axum::body::to_bytes(connection_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let connection_json: serde_json::Value = serde_json::from_slice(&connection_body).unwrap();
    let connection_id = connection_json["id"]
        .as_str()
        .expect("connection response must include id");

    assert_eq!(connection_json["source"], "registry");
    assert_eq!(connection_json["subjectType"], "user");
    assert_eq!(connection_json["subjectId"], TEST_USER_ID);
    assert_eq!(
        connection_json["connections"]
            .as_array()
            .expect("connections must be an array")
            .len(),
        1
    );
    assert_eq!(
        connection_json["accounts"]
            .as_array()
            .expect("accounts must be an array")
            .len(),
        1
    );
    assert_eq!(connection_json["connections"][0]["status"], "active");
    assert_eq!(connection_json["accounts"][0]["status"], "active");

    let eligibility_request = Request::builder()
        .method("GET")
        .uri(format!(
            "/v1/portal/venue-funding/eligibility?venueKey=hyperliquid&jurisdiction=VN&asset=USDT&network=ethereum&connectionId={connection_id}"
        ))
        .header("Authorization", format!("Bearer {}", create_jwt_token()))
        .body(Body::empty())
        .unwrap();

    let eligibility_response = app.oneshot(eligibility_request).await.unwrap();
    assert_eq!(eligibility_response.status(), StatusCode::OK);

    let eligibility_body = axum::body::to_bytes(eligibility_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let eligibility_json: serde_json::Value = serde_json::from_slice(&eligibility_body).unwrap();

    assert_eq!(eligibility_json["subjectType"], "user");
    assert_eq!(eligibility_json["subjectId"], TEST_USER_ID);
    assert!(
        eligibility_json["reasons"]
            .as_array()
            .expect("reasons must be an array")
            .iter()
            .all(|reason| reason["code"] != "venue_not_ready"),
        "connection token should advance eligibility beyond the missing-connection hard deny"
    );
}

#[tokio::test]
async fn portal_venue_funding_prepare_path_exists_and_fails_closed_without_runtime() {
    let app = setup_app().await;

    let request = Request::builder()
        .method("POST")
        .uri("/v1/portal/venue-funding/prepare")
        .header("Authorization", format!("Bearer {}", create_jwt_token()))
        .header("Content-Type", "application/json")
        .body(Body::from(
            serde_json::json!({
                "venueKey": "hyperliquid",
                "jurisdiction": "VN",
                "asset": "USDT",
                "network": "ethereum",
                "venueConnectionId": "conn-hyperliquid-1",
                "venueAccountId": "acct-hyperliquid-1",
                "walletAttestationId": "51000000-0000-0000-0000-000000000028",
                "amount": "125"
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn portal_venue_funding_connection_rejects_non_hyperliquid_venue() {
    let app = setup_app().await;

    let request = Request::builder()
        .method("POST")
        .uri("/v1/portal/venue-funding/connection")
        .header("Authorization", format!("Bearer {}", create_jwt_token()))
        .header("Content-Type", "application/json")
        .body(Body::from(
            serde_json::json!({
                "venueKey": "kraken",
                "jurisdiction": "VN",
                "asset": "USDT",
                "network": "ethereum"
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn portal_venue_funding_eligibility_rejects_non_hyperliquid_venue() {
    let app = setup_app().await;

    let request = Request::builder()
        .method("GET")
        .uri("/v1/portal/venue-funding/eligibility?venueKey=kraken&jurisdiction=VN&asset=USDT&network=ethereum")
        .header("Authorization", format!("Bearer {}", create_jwt_token()))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn portal_venue_funding_prepare_rejects_non_hyperliquid_venue() {
    let app = setup_app().await;

    let request = Request::builder()
        .method("POST")
        .uri("/v1/portal/venue-funding/prepare")
        .header("Authorization", format!("Bearer {}", create_jwt_token()))
        .header("Content-Type", "application/json")
        .body(Body::from(
            serde_json::json!({
                "venueKey": "kraken",
                "jurisdiction": "VN",
                "asset": "USDT",
                "network": "ethereum",
                "venueConnectionId": "conn-kraken-1",
                "venueAccountId": "acct-kraken-1",
                "walletAttestationId": "51000000-0000-0000-0000-000000000028",
                "amount": "125"
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn portal_venue_funding_prepare_rejects_non_usdt_asset_for_hyperliquid() {
    let app = setup_app().await;

    let request = Request::builder()
        .method("POST")
        .uri("/v1/portal/venue-funding/prepare")
        .header("Authorization", format!("Bearer {}", create_jwt_token()))
        .header("Content-Type", "application/json")
        .body(Body::from(
            serde_json::json!({
                "venueKey": "hyperliquid",
                "jurisdiction": "VN",
                "asset": "BTC",
                "network": "ethereum",
                "venueConnectionId": "conn-hyperliquid-1",
                "venueAccountId": "acct-hyperliquid-1",
                "walletAttestationId": "51000000-0000-0000-0000-000000000028",
                "amount": "125"
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn portal_venue_funding_submit_path_exists_and_fails_closed_without_runtime() {
    let app = setup_app().await;

    let submit_request = Request::builder()
        .method("POST")
        .uri("/v1/portal/venue-funding/transfer-123/submit")
        .header("Authorization", format!("Bearer {}", create_jwt_token()))
        .header("Content-Type", "application/json")
        .body(Body::from(
            serde_json::json!({
                "walletTransferReference": "wallet-tx-001"
            })
            .to_string(),
        ))
        .unwrap();

    let submit_response = app.oneshot(submit_request).await.unwrap();
    assert_eq!(submit_response.status(), StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn portal_venue_funding_status_path_exists_and_fails_closed_without_runtime() {
    let app = setup_app().await;

    let request = Request::builder()
        .method("GET")
        .uri("/v1/portal/venue-funding/transfer-123/status")
        .header("Authorization", format!("Bearer {}", create_jwt_token()))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn portal_venue_funding_prepare_requires_authentication() {
    let app = setup_app().await;

    let request = Request::builder()
        .method("POST")
        .uri("/v1/portal/venue-funding/prepare")
        .header("Content-Type", "application/json")
        .body(Body::from(
            serde_json::json!({
                "venueKey": "kraken",
                "jurisdiction": "VN",
                "asset": "USDT",
                "network": "ethereum"
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn portal_venue_funding_prepare_rejects_unprocessable_entity() {
    let app = setup_app().await;

    let request = Request::builder()
        .method("POST")
        .uri("/v1/portal/venue-funding/prepare")
        .header("Authorization", format!("Bearer {}", create_jwt_token()))
        .header("Content-Type", "application/json")
        .body(Body::from(
            serde_json::json!({
                "venueKey": "hyperliquid",
                "jurisdiction": "VN",
                "asset": "USDT",
                "amount": "125"
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn portal_venue_funding_prepare_requires_amount_for_durable_transfer_contract() {
    let app = setup_app().await;

    let request = Request::builder()
        .method("POST")
        .uri("/v1/portal/venue-funding/prepare")
        .header("Authorization", format!("Bearer {}", create_jwt_token()))
        .header("Content-Type", "application/json")
        .body(Body::from(
            serde_json::json!({
                "venueKey": "hyperliquid",
                "jurisdiction": "VN",
                "asset": "USDT",
                "network": "ethereum",
                "venueConnectionId": "conn-hyperliquid-1",
                "venueAccountId": "acct-hyperliquid-1",
                "walletAttestationId": "51000000-0000-0000-0000-000000000028"
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}
