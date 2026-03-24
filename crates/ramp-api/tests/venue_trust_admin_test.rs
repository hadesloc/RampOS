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
use ramp_core::repository::{
    EnsureWalletAttestationRequest, UpsertBeneficiaryProfileRequest,
    UpsertSourceOfFundsPackageRequest, UpsertVenueAccountRequest, UpsertVenueConnectionRequest,
    UpsertVenueTransferRequest,
};
use ramp_core::service::{
    ledger::LedgerService, payin::PayinService, payout::PayoutService, trade::TradeService,
    VenueTrustService,
};
use ramp_core::test_utils::*;
use rust_decimal::Decimal;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use std::sync::Arc;
use tower::ServiceExt;
use uuid::Uuid;

type HmacSha256 = Hmac<Sha256>;

const TEST_API_KEY: &str = "venue_trust_test_api_key";
const TEST_API_SECRET: &str = "venue_trust_test_api_secret";
const TEST_ADMIN_JWT_SECRET: &str = "venue-trust-admin-jwt-secret";

struct TestApp {
    router: axum::Router,
    api_key: String,
    api_secret: String,
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
        sub: "venue_trust_admin_test_user".to_string(),
        email: "venue-trust-admin@rampos.local".to_string(),
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

async fn setup_app_with_pool(tenant_id: &str, db_pool: Option<PgPool>) -> TestApp {
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
        id: tenant_id.to_string(),
        name: "Venue Trust Test Tenant".to_string(),
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
    let pool = db_pool.clone().unwrap_or_else(|| {
        PgPool::connect_lazy("postgres://postgres:postgres@localhost/postgres")
            .expect("Failed to create lazy pool")
    });
    let report_generator = Arc::new(ReportGenerator::new(
        pool,
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
            ramp_core::service::webhook::WebhookService::new(webhook_repo, tenant_repo.clone())
                .unwrap(),
        ),
        tenant_repo,
        intent_repo,
        report_generator,
        case_manager,
        rule_manager: None,
        rate_limiter: None,
        idempotency_handler: None,
        aa_service: None,
        portal_auth_config: Arc::new(PortalAuthConfig {
            jwt_secret: "venue-trust-test-secret".to_string(),
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
        db_pool,
        ctr_service: None,
        ws_state: None,
        metrics_registry: Arc::new(ramp_core::service::MetricsRegistry::new()),
        event_publisher,
        document_storage: None,
        kyc_service: None,
        kyt_service: None,
    };

    TestApp {
        router: create_router(app_state),
        api_key: TEST_API_KEY.to_string(),
        api_secret: TEST_API_SECRET.to_string(),
    }
}

async fn seed_db_tenant(pool: &PgPool, tenant_id: &str, api_key_hash: &str) {
    sqlx::query(
        r#"
        INSERT INTO tenants (
            id,
            name,
            status,
            api_key_hash,
            webhook_secret_hash,
            webhook_url,
            config
        ) VALUES ($1, $2, 'ACTIVE', $3, 'secret', NULL, '{}'::jsonb)
        ON CONFLICT (id) DO UPDATE SET
            name = EXCLUDED.name,
            status = EXCLUDED.status,
            api_key_hash = EXCLUDED.api_key_hash
        "#,
    )
    .bind(tenant_id)
    .bind("Venue Trust Test Tenant")
    .bind(api_key_hash)
    .execute(pool)
    .await
    .expect("tenant seed should succeed");
}

async fn seed_venue_trust_graph(pool: &PgPool, tenant_id: &str) -> Uuid {
    let service = VenueTrustService::with_pool(pool.clone());
    let attestation_id =
        Uuid::parse_str("40000000-0000-0000-0000-000000000023").expect("attestation uuid");

    service
        .ensure_wallet_attestation(&EnsureWalletAttestationRequest {
            attestation_id,
            tenant_id: tenant_id.to_string(),
            user_id: "user-venue-1".to_string(),
            wallet_address: "0xabc123".to_string(),
            chain_id: "ethereum".to_string(),
            attestation_status: "pending".to_string(),
            proof_kind: "signature".to_string(),
            proof_artifact_uri: Some("s3://proofs/venue-trust-attestation.json".to_string()),
            risk_state: Some("clear".to_string()),
            metadata: serde_json::json!({ "scope": "venue_funding" }),
        })
        .await
        .expect("wallet attestation should persist");

    service
        .upsert_connection(&UpsertVenueConnectionRequest {
            connection_id: "connection-venue-1".to_string(),
            tenant_id: tenant_id.to_string(),
            subject_type: "user".to_string(),
            subject_id: "user-venue-1".to_string(),
            user_id: Some("user-venue-1".to_string()),
            venue_key: "hyperliquid".to_string(),
            connection_mode: "wallet_linked".to_string(),
            status: "pending".to_string(),
            metadata: serde_json::json!({ "tier": "gold" }),
            last_verified_at: None,
        })
        .await
        .expect("connection should persist");

    service
        .upsert_account(&UpsertVenueAccountRequest {
            account_id: "account-venue-1".to_string(),
            tenant_id: tenant_id.to_string(),
            venue_connection_id: "connection-venue-1".to_string(),
            venue_key: "hyperliquid".to_string(),
            account_label: Some("Primary".to_string()),
            account_ref: Some("acct-venue-1".to_string()),
            wallet_address: Some("0xabc123".to_string()),
            subaccount_ref: None,
            api_scope_summary: serde_json::json!({ "permissions": ["read", "transfer"] }),
            status: "active".to_string(),
            metadata: serde_json::json!({}),
            last_verified_at: None,
        })
        .await
        .expect("account should persist");

    service
        .upsert_beneficiary_profile(&UpsertBeneficiaryProfileRequest {
            beneficiary_profile_id: "beneficiary-venue-1".to_string(),
            tenant_id: tenant_id.to_string(),
            subject_type: "user".to_string(),
            subject_id: "user-venue-1".to_string(),
            user_id: Some("user-venue-1".to_string()),
            destination_type: "exchange_wallet".to_string(),
            destination_ref: "0xbeneficiary".to_string(),
            display_name: Some("Treasury Wallet".to_string()),
            asset_symbol: Some("USDC".to_string()),
            network: Some("ethereum".to_string()),
            verification_status: "verified".to_string(),
            cooldown_ends_at: None,
            metadata: serde_json::json!({}),
            last_verified_at: None,
        })
        .await
        .expect("beneficiary should persist");

    service
        .upsert_transfer(&UpsertVenueTransferRequest {
            transfer_id: "transfer-venue-1".to_string(),
            tenant_id: tenant_id.to_string(),
            user_id: "user-venue-1".to_string(),
            beneficiary_profile_id: Some("beneficiary-venue-1".to_string()),
            wallet_attestation_id: attestation_id,
            venue_connection_id: "connection-venue-1".to_string(),
            venue_account_id: "account-venue-1".to_string(),
            transfer_direction: "wallet_to_venue".to_string(),
            asset_symbol: "USDC".to_string(),
            network: "ethereum".to_string(),
            amount: Decimal::new(25_000, 2),
            origin_intent_id: Some("intent-venue-1".to_string()),
            rfq_id: Some("rfq-venue-1".to_string()),
            status: "draft".to_string(),
            wallet_tx_hash: Some("0xtransferhash".to_string()),
            venue_credit_ref: Some("credit-venue-1".to_string()),
            failure_code: None,
            metadata: serde_json::json!({ "lane": "fast" }),
            submitted_at: Some(Utc::now()),
            completed_at: None,
        })
        .await
        .expect("transfer should persist");

    service
        .upsert_source_of_funds_package(&UpsertSourceOfFundsPackageRequest {
            package_id: "sof-package-1".to_string(),
            tenant_id: tenant_id.to_string(),
            subject_type: "user".to_string(),
            subject_id: "user-venue-1".to_string(),
            wallet_attestation_id: Some(attestation_id),
            venue_account_id: Some("account-venue-1".to_string()),
            venue_transfer_id: Some("transfer-venue-1".to_string()),
            review_status: "draft".to_string(),
            package_uri: Some("s3://packages/source-of-funds-1.json".to_string()),
            metadata: serde_json::json!({ "submittedBy": "user" }),
            reviewed_at: None,
        })
        .await
        .expect("source of funds package should persist");

    attestation_id
}

#[tokio::test]
async fn venue_trust_admin_reads_views_and_persists_review_flows() {
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

    let mut hasher = Sha256::new();
    hasher.update(TEST_API_KEY.as_bytes());
    let api_key_hash = hex::encode(hasher.finalize());
    seed_db_tenant(&pool, "tenant_venue_trust_admin", &api_key_hash).await;
    let attestation_id = seed_venue_trust_graph(&pool, "tenant_venue_trust_admin").await;

    std::env::set_var("RAMPOS_ADMIN_JWT_SECRET", TEST_ADMIN_JWT_SECRET);
    let app = setup_app_with_pool("tenant_venue_trust_admin", Some(pool)).await;
    let viewer_jwt = make_admin_jwt("viewer");
    let operator_jwt = make_admin_jwt("operator");

    let subject_request = build_signed_admin_jwt_request(
        "GET",
        "/v1/admin/venue-trust/subjects/user/user-venue-1",
        "",
        &app.api_key,
        &app.api_secret,
        &viewer_jwt,
    );
    let subject_response = app.router.clone().oneshot(subject_request).await.unwrap();
    assert_eq!(subject_response.status(), StatusCode::OK);

    let subject_payload: serde_json::Value = serde_json::from_slice(
        &to_bytes(subject_response.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(subject_payload["source"], "registry");
    assert_eq!(subject_payload["subjectType"], "user");
    assert_eq!(subject_payload["connections"][0]["status"], "pending");
    assert_eq!(
        subject_payload["walletAttestations"][0]["attestationStatus"],
        "pending"
    );
    assert_eq!(
        subject_payload["walletAttestations"][0]["proofArtifactUri"],
        "s3://proofs/venue-trust-attestation.json"
    );

    let transfer_request = build_signed_admin_jwt_request(
        "GET",
        "/v1/admin/venue-trust/transfers/transfer-venue-1",
        "",
        &app.api_key,
        &app.api_secret,
        &viewer_jwt,
    );
    let transfer_response = app.router.clone().oneshot(transfer_request).await.unwrap();
    assert_eq!(transfer_response.status(), StatusCode::OK);

    let transfer_payload: serde_json::Value = serde_json::from_slice(
        &to_bytes(transfer_response.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(
        transfer_payload["transfer"]["transferId"],
        "transfer-venue-1"
    );
    assert_eq!(transfer_payload["transfer"]["status"], "draft");
    assert_eq!(transfer_payload["connection"]["venueKey"], "hyperliquid");
    assert_eq!(
        transfer_payload["sourceOfFundsPackages"][0]["packageId"],
        "sof-package-1"
    );
    assert_eq!(
        transfer_payload["sourceOfFundsPackages"][0]["reviewStatus"],
        "draft"
    );

    let connection_review_body = serde_json::json!({
        "status": "active",
        "reviewReason": "manual_connection_review",
        "provenance": {
            "reviewer": "ops",
            "ticket": "closure-delta-025"
        }
    })
    .to_string();
    let connection_review_request = build_signed_admin_jwt_request(
        "POST",
        "/v1/admin/venue-trust/connections/connection-venue-1/review",
        &connection_review_body,
        &app.api_key,
        &app.api_secret,
        &operator_jwt,
    );
    let connection_review_response = app
        .router
        .clone()
        .oneshot(connection_review_request)
        .await
        .unwrap();
    assert_eq!(connection_review_response.status(), StatusCode::OK);

    let connection_review_payload: serde_json::Value = serde_json::from_slice(
        &to_bytes(connection_review_response.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(connection_review_payload["status"], "active");
    assert_eq!(
        connection_review_payload["metadata"]["reviewReason"],
        "manual_connection_review"
    );
    assert_eq!(
        connection_review_payload["metadata"]["provenance"]["ticket"],
        "closure-delta-025"
    );

    let transfer_review_body = serde_json::json!({
        "status": "submitted",
        "reviewReason": "manual_transfer_review",
        "failureReason": "awaiting_credit",
        "provenance": {
            "reviewer": "ops",
            "ticket": "closure-delta-025"
        }
    })
    .to_string();
    let transfer_review_request = build_signed_admin_jwt_request(
        "POST",
        "/v1/admin/venue-trust/transfers/transfer-venue-1/review",
        &transfer_review_body,
        &app.api_key,
        &app.api_secret,
        &operator_jwt,
    );
    let transfer_review_response = app
        .router
        .clone()
        .oneshot(transfer_review_request)
        .await
        .unwrap();
    assert_eq!(transfer_review_response.status(), StatusCode::OK);

    let transfer_review_payload: serde_json::Value = serde_json::from_slice(
        &to_bytes(transfer_review_response.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(transfer_review_payload["status"], "submitted");
    assert_eq!(
        transfer_review_payload["metadata"]["reviewReason"],
        "manual_transfer_review"
    );
    assert_eq!(
        transfer_review_payload["metadata"]["failureReason"],
        "awaiting_credit"
    );
    assert_eq!(
        transfer_review_payload["metadata"]["provenance"]["reviewer"],
        "ops"
    );

    let source_of_funds_in_review_body = serde_json::json!({
        "status": "in_review",
        "reviewReason": "manual_sof_triage",
        "provenance": {
            "reviewer": "ops",
            "ticket": "closure-delta-025"
        }
    })
    .to_string();
    let source_of_funds_in_review_request = build_signed_admin_jwt_request(
        "POST",
        "/v1/admin/venue-trust/source-of-funds-packages/sof-package-1/review",
        &source_of_funds_in_review_body,
        &app.api_key,
        &app.api_secret,
        &operator_jwt,
    );
    let source_of_funds_in_review_response = app
        .router
        .clone()
        .oneshot(source_of_funds_in_review_request)
        .await
        .unwrap();
    assert_eq!(source_of_funds_in_review_response.status(), StatusCode::OK);

    let source_of_funds_in_review_payload: serde_json::Value = serde_json::from_slice(
        &to_bytes(source_of_funds_in_review_response.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(
        source_of_funds_in_review_payload["reviewStatus"],
        "in_review"
    );
    assert_eq!(
        source_of_funds_in_review_payload["metadata"]["reviewReason"],
        "manual_sof_triage"
    );

    let source_of_funds_review_body = serde_json::json!({
        "status": "approved",
        "reviewReason": "manual_sof_review",
        "provenance": {
            "reviewer": "ops",
            "ticket": "closure-delta-025"
        }
    })
    .to_string();
    let source_of_funds_review_request = build_signed_admin_jwt_request(
        "POST",
        "/v1/admin/venue-trust/source-of-funds-packages/sof-package-1/review",
        &source_of_funds_review_body,
        &app.api_key,
        &app.api_secret,
        &operator_jwt,
    );
    let source_of_funds_review_response = app
        .router
        .clone()
        .oneshot(source_of_funds_review_request)
        .await
        .unwrap();
    assert_eq!(source_of_funds_review_response.status(), StatusCode::OK);

    let source_of_funds_review_payload: serde_json::Value = serde_json::from_slice(
        &to_bytes(source_of_funds_review_response.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(source_of_funds_review_payload["reviewStatus"], "approved");
    assert_eq!(
        source_of_funds_review_payload["metadata"]["reviewReason"],
        "manual_sof_review"
    );
    assert_eq!(
        source_of_funds_review_payload["metadata"]["provenance"]["ticket"],
        "closure-delta-025"
    );

    let transfer_complete_body = serde_json::json!({
        "status": "completed",
        "reviewReason": "manual_transfer_completion",
        "provenance": {
            "reviewer": "ops",
            "ticket": "closure-delta-025"
        }
    })
    .to_string();
    let transfer_complete_request = build_signed_admin_jwt_request(
        "POST",
        "/v1/admin/venue-trust/transfers/transfer-venue-1/review",
        &transfer_complete_body,
        &app.api_key,
        &app.api_secret,
        &operator_jwt,
    );
    let transfer_complete_response = app
        .router
        .clone()
        .oneshot(transfer_complete_request)
        .await
        .unwrap();
    assert_eq!(transfer_complete_response.status(), StatusCode::OK);

    let transfer_complete_payload: serde_json::Value = serde_json::from_slice(
        &to_bytes(transfer_complete_response.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(transfer_complete_payload["status"], "completed");
    assert_eq!(
        transfer_complete_payload["metadata"]["reviewReason"],
        "manual_transfer_completion"
    );

    let review_body = serde_json::json!({
        "status": "flagged",
        "reviewReason": "manual_review",
        "failureReason": "chain_mismatch"
    })
    .to_string();
    let review_request = build_signed_admin_jwt_request(
        "POST",
        &format!("/v1/admin/venue-trust/wallet-attestations/{attestation_id}/review"),
        &review_body,
        &app.api_key,
        &app.api_secret,
        &operator_jwt,
    );
    let review_response = app.router.clone().oneshot(review_request).await.unwrap();
    assert_eq!(review_response.status(), StatusCode::OK);

    let review_payload: serde_json::Value = serde_json::from_slice(
        &to_bytes(review_response.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(review_payload["attestationStatus"], "flagged");
    assert_eq!(review_payload["metadata"]["reviewReason"], "manual_review");
    assert_eq!(
        review_payload["metadata"]["failureReason"],
        "chain_mismatch"
    );

    let reopen_connection_body = serde_json::json!({
        "status": "pending",
        "reviewReason": "rollback"
    })
    .to_string();
    let reopen_connection_request = build_signed_admin_jwt_request(
        "POST",
        "/v1/admin/venue-trust/connections/connection-venue-1/review",
        &reopen_connection_body,
        &app.api_key,
        &app.api_secret,
        &operator_jwt,
    );
    let reopen_connection_response = app
        .router
        .clone()
        .oneshot(reopen_connection_request)
        .await
        .unwrap();
    assert_eq!(reopen_connection_response.status(), StatusCode::BAD_REQUEST);

    let reopen_transfer_body = serde_json::json!({
        "status": "submitted",
        "reviewReason": "rollback"
    })
    .to_string();
    let reopen_transfer_request = build_signed_admin_jwt_request(
        "POST",
        "/v1/admin/venue-trust/transfers/transfer-venue-1/review",
        &reopen_transfer_body,
        &app.api_key,
        &app.api_secret,
        &operator_jwt,
    );
    let reopen_transfer_response = app
        .router
        .clone()
        .oneshot(reopen_transfer_request)
        .await
        .unwrap();
    assert_eq!(reopen_transfer_response.status(), StatusCode::BAD_REQUEST);

    let reopen_source_of_funds_body = serde_json::json!({
        "status": "in_review",
        "reviewReason": "rollback"
    })
    .to_string();
    let reopen_source_of_funds_request = build_signed_admin_jwt_request(
        "POST",
        "/v1/admin/venue-trust/source-of-funds-packages/sof-package-1/review",
        &reopen_source_of_funds_body,
        &app.api_key,
        &app.api_secret,
        &operator_jwt,
    );
    let reopen_source_of_funds_response = app
        .router
        .clone()
        .oneshot(reopen_source_of_funds_request)
        .await
        .unwrap();
    assert_eq!(
        reopen_source_of_funds_response.status(),
        StatusCode::BAD_REQUEST
    );

    let reopen_body = serde_json::json!({
        "status": "pending",
        "reviewReason": "rollback"
    })
    .to_string();
    let reopen_request = build_signed_admin_jwt_request(
        "POST",
        &format!("/v1/admin/venue-trust/wallet-attestations/{attestation_id}/review"),
        &reopen_body,
        &app.api_key,
        &app.api_secret,
        &operator_jwt,
    );
    let reopen_response = app.router.clone().oneshot(reopen_request).await.unwrap();
    assert_eq!(reopen_response.status(), StatusCode::BAD_REQUEST);

    let reopen_payload: serde_json::Value = serde_json::from_slice(
        &to_bytes(reopen_response.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    assert!(reopen_payload["error"]["message"]
        .as_str()
        .unwrap_or_default()
        .contains("invalid state transition"));

    let refreshed_subject_request = build_signed_admin_jwt_request(
        "GET",
        "/v1/admin/venue-trust/subjects/user/user-venue-1",
        "",
        &app.api_key,
        &app.api_secret,
        &viewer_jwt,
    );
    let refreshed_subject_response = app
        .router
        .clone()
        .oneshot(refreshed_subject_request)
        .await
        .unwrap();
    assert_eq!(refreshed_subject_response.status(), StatusCode::OK);

    let refreshed_subject_payload: serde_json::Value = serde_json::from_slice(
        &to_bytes(refreshed_subject_response.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(
        refreshed_subject_payload["walletAttestations"][0]["attestationStatus"],
        "flagged"
    );
    assert_eq!(
        refreshed_subject_payload["connections"][0]["status"],
        "active"
    );
    assert_eq!(
        refreshed_subject_payload["connections"][0]["metadata"]["reviewReason"],
        "manual_connection_review"
    );
    assert_eq!(
        refreshed_subject_payload["connections"][0]["metadata"]["provenance"]["reviewer"],
        "ops"
    );
    assert_eq!(
        refreshed_subject_payload["walletAttestations"][0]["metadata"]["reviewReason"],
        "manual_review"
    );
    assert_eq!(
        refreshed_subject_payload["walletAttestations"][0]["metadata"]["failureReason"],
        "chain_mismatch"
    );
    assert_eq!(
        refreshed_subject_payload["sourceOfFundsPackages"][0]["reviewStatus"],
        "approved"
    );
    assert_eq!(
        refreshed_subject_payload["sourceOfFundsPackages"][0]["metadata"]["reviewReason"],
        "manual_sof_review"
    );

    let refreshed_transfer_request = build_signed_admin_jwt_request(
        "GET",
        "/v1/admin/venue-trust/transfers/transfer-venue-1",
        "",
        &app.api_key,
        &app.api_secret,
        &viewer_jwt,
    );
    let refreshed_transfer_response = app
        .router
        .clone()
        .oneshot(refreshed_transfer_request)
        .await
        .unwrap();
    assert_eq!(refreshed_transfer_response.status(), StatusCode::OK);

    let refreshed_transfer_payload: serde_json::Value = serde_json::from_slice(
        &to_bytes(refreshed_transfer_response.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(
        refreshed_transfer_payload["transfer"]["status"],
        "completed"
    );
    assert_eq!(
        refreshed_transfer_payload["transfer"]["metadata"]["reviewReason"],
        "manual_transfer_completion"
    );
    assert_eq!(
        refreshed_transfer_payload["transfer"]["submittedAt"].is_null(),
        false
    );
    assert_eq!(
        refreshed_transfer_payload["transfer"]["completedAt"].is_null(),
        false
    );
    assert_eq!(
        refreshed_transfer_payload["sourceOfFundsPackages"][0]["reviewStatus"],
        "approved"
    );
    assert_eq!(
        refreshed_transfer_payload["sourceOfFundsPackages"][0]["metadata"]["provenance"]
            ["reviewer"],
        "ops"
    );
}
