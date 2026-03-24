use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use chrono::Utc;
use hmac::{Hmac, Mac};
use jsonwebtoken::{encode, EncodingKey, Header};
use ramp_api::middleware::PortalAuthConfig;
use ramp_api::{create_router, AppState};
use ramp_compliance::{
    case::CaseManager, reports::ReportGenerator, storage::MockDocumentStorage, InMemoryCaseStore,
};
use ramp_core::event::InMemoryEventPublisher;
use ramp_core::repository::tenant::TenantRow;
use ramp_core::repository::webhook::{WebhookEventRow, WebhookRepository};
use ramp_core::service::{
    ledger::LedgerService, payin::PayinService, payout::PayoutService, trade::TradeService,
};
use ramp_core::test_utils::*;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use std::sync::{Arc, Mutex, OnceLock};
use tower::ServiceExt;

type HmacSha256 = Hmac<Sha256>;

const TEST_API_KEY: &str = "webhook_test_api_key";
const TEST_API_SECRET: &str = "webhook_test_api_secret";
const TEST_ADMIN_JWT_SECRET: &str = "webhook-admin-jwt-secret";

fn env_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

struct TestApp {
    router: axum::Router,
    api_key: String,
    api_secret: String,
    webhook_repo: Arc<MockWebhookRepository>,
}

fn generate_signature(
    method: &str,
    path: &str,
    timestamp: &str,
    body: &str,
    secret: &str,
) -> String {
    let message = format!("{method}\n{path}\n{timestamp}\n{body}");
    let mut mac =
        HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC can take any size key");
    mac.update(message.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

fn make_admin_jwt(role: &str) -> String {
    let claims = ramp_api::handlers::admin::admin_auth::AdminClaims {
        sub: "webhook_admin_test_user".to_string(),
        email: "webhook-admin@rampos.local".to_string(),
        role: role.to_string(),
        iat: Utc::now().timestamp(),
        exp: (Utc::now() + chrono::Duration::minutes(30)).timestamp(),
        token_type: "access".to_string(),
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(TEST_ADMIN_JWT_SECRET.as_bytes()),
    )
    .expect("jwt should encode")
}

fn build_signed_admin_jwt_request(
    method: &str,
    uri: &str,
    body: &str,
    api_key: &str,
    api_secret: &str,
    admin_jwt: &str,
) -> Request<Body> {
    let timestamp = Utc::now().to_rfc3339();
    let path = uri.split('?').next().unwrap_or(uri);
    let signature = generate_signature(method, path, &timestamp, body, api_secret);

    let mut builder = Request::builder()
        .uri(uri)
        .method(method)
        .header("Authorization", format!("Bearer {api_key}"))
        .header("X-Timestamp", &timestamp)
        .header("X-Signature", signature)
        .header("X-Admin-Authorization", format!("Bearer {admin_jwt}"));

    if !body.is_empty() {
        builder = builder.header("Content-Type", "application/json");
    }

    builder.body(Body::from(body.to_string())).unwrap()
}

async fn setup_app() -> TestApp {
    setup_app_with_pool(None).await
}

async fn setup_app_with_pool(db_pool: Option<PgPool>) -> TestApp {
    let intent_repo = Arc::new(MockIntentRepository::new());
    let ledger_repo = Arc::new(MockLedgerRepository::new());
    let user_repo = Arc::new(MockUserRepository::new());
    let tenant_repo = Arc::new(MockTenantRepository::new());
    let webhook_repo = Arc::new(MockWebhookRepository::new());
    let event_publisher = Arc::new(InMemoryEventPublisher::new());

    let mut hasher = Sha256::new();
    hasher.update(TEST_API_KEY.as_bytes());
    let api_key_hash = hex::encode(hasher.finalize());

    tenant_repo.add_tenant(TenantRow {
        id: "tenant_webhook_test".to_string(),
        name: "Webhook Test Tenant".to_string(),
        status: "ACTIVE".to_string(),
        api_key_hash,
        api_secret_encrypted: Some(TEST_API_SECRET.as_bytes().to_vec()),
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
    let ledger_service = Arc::new(LedgerService::new(ledger_repo));
    let onboarding_service = Arc::new(ramp_core::service::onboarding::OnboardingService::new(
        tenant_repo.clone(),
        ledger_service.clone(),
    ));
    let user_service = Arc::new(ramp_core::service::user::UserService::new(
        user_repo,
        event_publisher.clone(),
    ));
    let pool = PgPool::connect_lazy("postgres://postgres:postgres@localhost/postgres")
        .expect("Failed to create lazy pool");
    let report_generator = Arc::new(ReportGenerator::new(
        pool,
        Arc::new(MockDocumentStorage::new()),
    ));
    let case_manager = Arc::new(CaseManager::new(Arc::new(InMemoryCaseStore::new())));

    let webhook_service = if let Some(pool) = db_pool.clone() {
        Arc::new(
            ramp_core::service::webhook::WebhookService::new(
                Arc::new(ramp_core::repository::webhook::PgWebhookRepository::new(
                    pool,
                )),
                tenant_repo.clone(),
            )
            .expect("webhook service with pg repository"),
        )
    } else {
        Arc::new(
            ramp_core::service::webhook::WebhookService::new(
                webhook_repo.clone(),
                tenant_repo.clone(),
            )
            .expect("webhook service with mock repository"),
        )
    };

    let app_state = AppState {
        payin_service,
        payout_service,
        trade_service,
        ledger_service,
        onboarding_service,
        user_service,
        webhook_service,
        tenant_repo: tenant_repo.clone(),
        intent_repo,
        report_generator,
        case_manager,
        rule_manager: None,
        rate_limiter: None,
        idempotency_handler: None,
        aa_service: None,
        portal_auth_config: Arc::new(PortalAuthConfig {
            jwt_secret: "webhook-test-secret".to_string(),
            issuer: None,
            audience: None,
            allow_missing_tenant: false,
        }),
        bank_confirmation_repo: None,
        licensing_repo: None,
        compliance_audit_service: None,
        sso_service: Arc::new(ramp_core::sso::SsoService::new()),
        billing_service: Arc::new(ramp_core::billing::BillingService::new(
            ramp_core::billing::BillingConfig::default(),
            Arc::new(ramp_core::billing::mock::MockBillingDataProvider::new()),
        )),
        vnst_protocol: Arc::new(ramp_core::stablecoin::VnstProtocolService::new(
            ramp_core::stablecoin::VnstProtocolConfig::default(),
            Arc::new(ramp_core::stablecoin::MockVnstProtocolDataProvider::new()),
        )),
        event_publisher: event_publisher.clone(),
        db_pool,
        ctr_service: None,
        ws_state: None,
        metrics_registry: Arc::new(ramp_core::service::MetricsRegistry::new()),
        document_storage: None,
        kyc_service: None,
        kyt_service: None,
    };

    TestApp {
        router: create_router(app_state),
        api_key: TEST_API_KEY.to_string(),
        api_secret: TEST_API_SECRET.to_string(),
        webhook_repo,
    }
}

fn sample_event_row(event_id: &str, event_type: &str) -> WebhookEventRow {
    WebhookEventRow {
        id: event_id.to_string(),
        tenant_id: "tenant_webhook_test".to_string(),
        event_type: event_type.to_string(),
        intent_id: Some("intent_webhook_001".to_string()),
        payload: serde_json::json!({
            "intentId": "intent_webhook_001",
            "newStatus": "FUNDS_CONFIRMED"
        }),
        status: "PENDING".to_string(),
        attempts: 0,
        max_attempts: 10,
        last_attempt_at: None,
        next_attempt_at: Some(Utc::now()),
        last_error: None,
        delivered_at: None,
        response_status: None,
        created_at: Utc::now(),
    }
}

#[tokio::test]
async fn webhook_admin_catalog_returns_current_event_contract() {
    let _guard = env_lock().lock().unwrap();
    std::env::set_var("RAMPOS_ADMIN_JWT_SECRET", TEST_ADMIN_JWT_SECRET);
    let app = setup_app().await;
    let admin_jwt = make_admin_jwt("viewer");

    let request = build_signed_admin_jwt_request(
        "GET",
        "/v1/admin/webhooks/catalog",
        "",
        &app.api_key,
        &app.api_secret,
        &admin_jwt,
    );

    let response = app.router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let entries = payload.as_array().expect("catalog should be an array");

    assert!(entries
        .iter()
        .any(|entry| entry["eventName"] == "intent.status.changed"));
    assert!(entries
        .iter()
        .any(|entry| entry["payloadWrapper"] == "webhook_event"));
}

#[tokio::test]
async fn webhook_admin_history_accepts_filters_and_stays_tenant_scoped() {
    let _guard = env_lock().lock().unwrap();
    std::env::set_var("RAMPOS_ADMIN_JWT_SECRET", TEST_ADMIN_JWT_SECRET);
    let app = setup_app().await;
    let admin_jwt = make_admin_jwt("viewer");

    let request = build_signed_admin_jwt_request(
        "GET",
        "/v1/admin/webhooks/history?eventId=evt_history_001&eventType=intent.status.changed&endpointUrl=https%3A%2F%2Fprimary.example.com%2Fwh",
        "",
        &app.api_key,
        &app.api_secret,
        &admin_jwt,
    );

    let response = app.router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(payload, serde_json::json!([]));
}

#[tokio::test]
async fn webhook_admin_replay_by_event_requeues_existing_event() {
    let _guard = env_lock().lock().unwrap();
    std::env::set_var("RAMPOS_ADMIN_JWT_SECRET", TEST_ADMIN_JWT_SECRET);
    let app = setup_app().await;
    let operator_jwt = make_admin_jwt("operator");

    app.webhook_repo
        .queue_event(&sample_event_row(
            "evt_replay_admin_001",
            "intent.status.changed",
        ))
        .await
        .unwrap();

    let request = build_signed_admin_jwt_request(
        "POST",
        "/v1/admin/webhooks/evt_replay_admin_001/replay",
        "",
        &app.api_key,
        &app.api_secret,
        &operator_jwt,
    );

    let response = app.router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(payload["eventId"], "evt_replay_admin_001");
    assert_eq!(payload["status"], "REPLAY_SCHEDULED");
    assert_eq!(payload["eventType"], "intent.status.changed");
    assert_eq!(payload["eventStatus"], "PENDING");
    assert!(payload["deliveredAt"].is_null());
    assert!(payload["responseStatus"].is_null());
    assert!(payload["lastError"].is_null());
    assert!(payload["nextAttemptAt"].is_string());
}

#[tokio::test]
async fn webhook_admin_retry_clears_stale_delivery_metadata() {
    let _guard = env_lock().lock().unwrap();
    std::env::set_var("RAMPOS_ADMIN_JWT_SECRET", TEST_ADMIN_JWT_SECRET);
    let app = setup_app().await;
    let operator_jwt = make_admin_jwt("operator");

    let mut event = sample_event_row("evt_retry_clears_delivery_state", "intent.status.changed");
    event.status = "DELIVERED".to_string();
    event.attempts = 3;
    event.last_attempt_at = Some(Utc::now());
    event.next_attempt_at = None;
    event.last_error = Some("stale delivery error".to_string());
    event.delivered_at = Some(Utc::now());
    event.response_status = Some(200);
    app.webhook_repo.queue_event(&event).await.unwrap();

    let request = build_signed_admin_jwt_request(
        "POST",
        "/v1/admin/webhooks/evt_retry_clears_delivery_state/retry",
        "",
        &app.api_key,
        &app.api_secret,
        &operator_jwt,
    );

    let response = app.router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(payload["id"], "evt_retry_clears_delivery_state");
    assert_eq!(payload["status"], "PENDING");
    assert!(payload["next_attempt_at"].is_string());
    assert!(payload["delivered_at"].is_null());
    assert!(payload["response_status"].is_null());
    assert!(payload["last_error"].is_null());
}

#[tokio::test]
async fn webhook_admin_history_reflects_retry_reset_truthfully_and_stays_tenant_scoped() {
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

    let _guard = env_lock().lock().unwrap();
    std::env::set_var("RAMPOS_ADMIN_JWT_SECRET", TEST_ADMIN_JWT_SECRET);
    let app = setup_app_with_pool(Some(pool.clone())).await;
    let operator_jwt = make_admin_jwt("operator");

    sqlx::query(
        r#"
        INSERT INTO tenants (
            id, name, status, api_key_hash, webhook_secret_hash, config, created_at, updated_at
        ) VALUES
            ('tenant_webhook_test', 'Webhook Admin Runtime Tenant', 'ACTIVE', 'hash', 'secret', '{}'::jsonb, NOW(), NOW()),
            ('tenant_webhook_other', 'Webhook Admin Other Tenant', 'ACTIVE', 'hash', 'secret', '{}'::jsonb, NOW(), NOW())
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .execute(&pool)
    .await
    .expect("seed runtime tenants");

    sqlx::query(
        r#"
        INSERT INTO webhook_events (
            id, tenant_id, event_type, intent_id, payload, status, attempts, max_attempts,
            last_attempt_at, next_attempt_at, last_error, delivered_at, response_status, created_at
        ) VALUES
            (
                'evt_history_runtime_a',
                'tenant_webhook_test',
                'intent.status.changed',
                'intent_runtime_a',
                '{"state":"COMPLETED"}'::jsonb,
                'DELIVERED',
                2,
                10,
                NOW(),
                NULL,
                'stale error'::text,
                NOW(),
                200,
                NOW()
            ),
            (
                'evt_history_runtime_b',
                'tenant_webhook_other',
                'intent.status.changed',
                'intent_runtime_b',
                '{"state":"COMPLETED"}'::jsonb,
                'DELIVERED',
                1,
                10,
                NOW(),
                NULL,
                NULL,
                NOW(),
                200,
                NOW()
            )
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .execute(&pool)
    .await
    .expect("seed runtime webhook events");

    let retry_request = build_signed_admin_jwt_request(
        "POST",
        "/v1/admin/webhooks/evt_history_runtime_a/retry",
        "",
        &app.api_key,
        &app.api_secret,
        &operator_jwt,
    );
    let retry_response = app.router.clone().oneshot(retry_request).await.unwrap();
    assert_eq!(retry_response.status(), StatusCode::OK);

    let retry_body = to_bytes(retry_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let retry_payload: serde_json::Value = serde_json::from_slice(&retry_body).unwrap();
    assert_eq!(retry_payload["id"], "evt_history_runtime_a");
    assert_eq!(retry_payload["status"], "PENDING");
    assert!(retry_payload["delivered_at"].is_null());
    assert!(retry_payload["response_status"].is_null());
    assert!(retry_payload["last_error"].is_null());

    let history_request = build_signed_admin_jwt_request(
        "GET",
        "/v1/admin/webhooks/history?eventType=intent.status.changed",
        "",
        &app.api_key,
        &app.api_secret,
        &operator_jwt,
    );
    let history_response = app.router.oneshot(history_request).await.unwrap();
    assert_eq!(history_response.status(), StatusCode::OK);

    let history_body = to_bytes(history_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let history_payload: serde_json::Value = serde_json::from_slice(&history_body).unwrap();
    let history_items = history_payload
        .as_array()
        .expect("history should be an array");

    assert!(history_items
        .iter()
        .any(|item| item["id"] == "evt_history_runtime_a"
            && item["status"] == "PENDING"
            && item["deliveredAt"].is_null()
            && item["responseStatus"].is_null()));
    assert!(!history_items
        .iter()
        .any(|item| item["id"] == "evt_history_runtime_b"));
}

#[tokio::test]
async fn webhook_admin_history_reflects_replay_reset_truthfully_and_stays_tenant_scoped() {
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

    let _guard = env_lock().lock().unwrap();
    std::env::set_var("RAMPOS_ADMIN_JWT_SECRET", TEST_ADMIN_JWT_SECRET);
    let app = setup_app_with_pool(Some(pool.clone())).await;
    let operator_jwt = make_admin_jwt("operator");

    sqlx::query(
        r#"
        INSERT INTO tenants (
            id, name, status, api_key_hash, webhook_secret_hash, config, created_at, updated_at
        ) VALUES
            ('tenant_webhook_test', 'Webhook Admin Runtime Tenant', 'ACTIVE', 'hash', 'secret', '{}'::jsonb, NOW(), NOW()),
            ('tenant_webhook_other', 'Webhook Admin Other Tenant', 'ACTIVE', 'hash', 'secret', '{}'::jsonb, NOW(), NOW())
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .execute(&pool)
    .await
    .expect("seed runtime tenants");

    sqlx::query(
        r#"
        INSERT INTO webhook_events (
            id, tenant_id, event_type, intent_id, payload, status, attempts, max_attempts,
            last_attempt_at, next_attempt_at, last_error, delivered_at, response_status, created_at
        ) VALUES
            (
                'evt_history_runtime_replay_a',
                'tenant_webhook_test',
                'intent.status.changed',
                'intent_runtime_replay_a',
                '{"state":"COMPLETED"}'::jsonb,
                'DELIVERED',
                2,
                10,
                NOW(),
                NULL,
                'stale error'::text,
                NOW(),
                200,
                NOW()
            ),
            (
                'evt_history_runtime_replay_b',
                'tenant_webhook_other',
                'intent.status.changed',
                'intent_runtime_replay_b',
                '{"state":"COMPLETED"}'::jsonb,
                'DELIVERED',
                1,
                10,
                NOW(),
                NULL,
                NULL,
                NOW(),
                200,
                NOW()
            )
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .execute(&pool)
    .await
    .expect("seed runtime webhook events");

    let replay_request = build_signed_admin_jwt_request(
        "POST",
        "/v1/admin/webhooks/evt_history_runtime_replay_a/replay",
        "",
        &app.api_key,
        &app.api_secret,
        &operator_jwt,
    );
    let replay_response = app.router.clone().oneshot(replay_request).await.unwrap();
    assert_eq!(replay_response.status(), StatusCode::OK);

    let replay_body = to_bytes(replay_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let replay_payload: serde_json::Value = serde_json::from_slice(&replay_body).unwrap();
    assert_eq!(replay_payload["eventId"], "evt_history_runtime_replay_a");
    assert_eq!(replay_payload["status"], "REPLAY_SCHEDULED");
    assert_eq!(replay_payload["eventStatus"], "PENDING");
    assert!(replay_payload["deliveredAt"].is_null());
    assert!(replay_payload["responseStatus"].is_null());
    assert!(replay_payload["lastError"].is_null());
    assert!(replay_payload["nextAttemptAt"].is_string());

    let history_request = build_signed_admin_jwt_request(
        "GET",
        "/v1/admin/webhooks/history?eventType=intent.status.changed",
        "",
        &app.api_key,
        &app.api_secret,
        &operator_jwt,
    );
    let history_response = app.router.oneshot(history_request).await.unwrap();
    assert_eq!(history_response.status(), StatusCode::OK);

    let history_body = to_bytes(history_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let history_payload: serde_json::Value = serde_json::from_slice(&history_body).unwrap();
    let history_items = history_payload
        .as_array()
        .expect("history should be an array");

    assert!(history_items
        .iter()
        .any(|item| item["id"] == "evt_history_runtime_replay_a"
            && item["status"] == "PENDING"
            && item["deliveredAt"].is_null()
            && item["responseStatus"].is_null()));
    assert!(!history_items
        .iter()
        .any(|item| item["id"] == "evt_history_runtime_replay_b"));
}

#[tokio::test]
async fn webhook_admin_retry_rejects_viewer_role() {
    let _guard = env_lock().lock().unwrap();
    std::env::set_var("RAMPOS_ADMIN_JWT_SECRET", TEST_ADMIN_JWT_SECRET);
    let app = setup_app().await;
    let viewer_jwt = make_admin_jwt("viewer");

    app.webhook_repo
        .queue_event(&sample_event_row(
            "evt_retry_viewer_001",
            "intent.status.changed",
        ))
        .await
        .unwrap();

    let request = build_signed_admin_jwt_request(
        "POST",
        "/v1/admin/webhooks/evt_retry_viewer_001/retry",
        "",
        &app.api_key,
        &app.api_secret,
        &viewer_jwt,
    );

    let response = app.router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn webhook_admin_replay_rejects_viewer_role() {
    let _guard = env_lock().lock().unwrap();
    std::env::set_var("RAMPOS_ADMIN_JWT_SECRET", TEST_ADMIN_JWT_SECRET);
    let app = setup_app().await;
    let viewer_jwt = make_admin_jwt("viewer");

    app.webhook_repo
        .queue_event(&sample_event_row(
            "evt_replay_viewer_001",
            "intent.status.changed",
        ))
        .await
        .unwrap();

    let request = build_signed_admin_jwt_request(
        "POST",
        "/v1/admin/webhooks/evt_replay_viewer_001/replay",
        "",
        &app.api_key,
        &app.api_secret,
        &viewer_jwt,
    );

    let response = app.router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn webhook_admin_retry_requires_operator_role() {
    let _guard = env_lock().lock().unwrap();
    std::env::set_var("RAMPOS_ADMIN_JWT_SECRET", TEST_ADMIN_JWT_SECRET);
    let app = setup_app().await;
    let viewer_jwt = make_admin_jwt("viewer");

    app.webhook_repo
        .queue_event(&sample_event_row(
            "evt_retry_admin_forbidden",
            "intent.status.changed",
        ))
        .await
        .unwrap();

    let request = build_signed_admin_jwt_request(
        "POST",
        "/v1/admin/webhooks/evt_retry_admin_forbidden/retry",
        "",
        &app.api_key,
        &app.api_secret,
        &viewer_jwt,
    );

    let response = app.router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn webhook_admin_replay_requires_operator_role() {
    let _guard = env_lock().lock().unwrap();
    std::env::set_var("RAMPOS_ADMIN_JWT_SECRET", TEST_ADMIN_JWT_SECRET);
    let app = setup_app().await;
    let viewer_jwt = make_admin_jwt("viewer");

    app.webhook_repo
        .queue_event(&sample_event_row(
            "evt_replay_admin_forbidden",
            "intent.status.changed",
        ))
        .await
        .unwrap();

    let request = build_signed_admin_jwt_request(
        "POST",
        "/v1/admin/webhooks/evt_replay_admin_forbidden/replay",
        "",
        &app.api_key,
        &app.api_secret,
        &viewer_jwt,
    );

    let response = app.router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}
