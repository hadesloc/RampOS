use async_trait::async_trait;
use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use chrono::Utc;
use hmac::{Hmac, Mac};
use ramp_api::middleware::PortalAuthConfig;
use ramp_api::{create_router, AppState};
use ramp_common::licensing::{LicenseRequirementId, LicenseStatus, SubmissionStatus};
use ramp_common::types::TenantId;
use ramp_compliance::{
    case::CaseManager, reports::ReportGenerator, storage::MockDocumentStorage, InMemoryCaseStore,
};
use ramp_core::event::InMemoryEventPublisher;
use ramp_core::repository::licensing::{
    CreateLicenseRequirementRequest, CreateLicenseSubmissionRequest, LicenseRequirementRow,
    LicenseSubmissionRow, LicensingRepository, TenantLicenseStatusRow,
};
use ramp_core::repository::tenant::TenantRow;
use ramp_core::service::{
    ledger::LedgerService, payin::PayinService, payout::PayoutService, trade::TradeService,
};
use ramp_core::test_utils::*;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use std::sync::{Arc, Mutex, OnceLock};
use tower::ServiceExt;

type HmacSha256 = Hmac<Sha256>;

const TEST_API_KEY: &str = "admin_security_test_api_key";
const TEST_API_SECRET: &str = "admin_security_test_api_secret";
const TEST_ADMIN_KEY: &str = "admin_security_admin_key";

fn env_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

struct TestApp {
    router: axum::Router,
    api_key: String,
    api_secret: String,
}

#[derive(Default)]
struct MockLicensingRepository {
    requirements: Vec<LicenseRequirementRow>,
    statuses: Vec<TenantLicenseStatusRow>,
    submissions: Mutex<Vec<LicenseSubmissionRow>>,
}

#[async_trait]
impl LicensingRepository for MockLicensingRepository {
    async fn list_requirements(
        &self,
        limit: i64,
        offset: i64,
    ) -> ramp_common::Result<Vec<LicenseRequirementRow>> {
        Ok(self
            .requirements
            .iter()
            .skip(offset.max(0) as usize)
            .take(limit.max(0) as usize)
            .cloned()
            .collect())
    }

    async fn get_requirement(
        &self,
        id: &LicenseRequirementId,
    ) -> ramp_common::Result<Option<LicenseRequirementRow>> {
        Ok(self.requirements.iter().find(|row| row.id == id.0).cloned())
    }

    async fn create_requirement(
        &self,
        _req: &CreateLicenseRequirementRequest,
    ) -> ramp_common::Result<LicenseRequirementRow> {
        Err(ramp_common::Error::NotImplemented(
            "not needed in tests".to_string(),
        ))
    }

    async fn get_tenant_license_statuses(
        &self,
        tenant_id: &TenantId,
    ) -> ramp_common::Result<Vec<TenantLicenseStatusRow>> {
        Ok(self
            .statuses
            .iter()
            .filter(|row| row.tenant_id == tenant_id.0)
            .cloned()
            .collect())
    }

    async fn get_tenant_license_status(
        &self,
        tenant_id: &TenantId,
        requirement_id: &LicenseRequirementId,
    ) -> ramp_common::Result<Option<TenantLicenseStatusRow>> {
        Ok(self
            .statuses
            .iter()
            .find(|row| row.tenant_id == tenant_id.0 && row.requirement_id == requirement_id.0)
            .cloned())
    }

    async fn upsert_tenant_license_status(
        &self,
        tenant_id: &TenantId,
        requirement_id: &LicenseRequirementId,
        status: LicenseStatus,
        license_number: Option<&str>,
        expiry_date: Option<chrono::DateTime<chrono::Utc>>,
    ) -> ramp_common::Result<TenantLicenseStatusRow> {
        Ok(TenantLicenseStatusRow {
            id: "tls_test".to_string(),
            tenant_id: tenant_id.0.clone(),
            requirement_id: requirement_id.0.clone(),
            status: status.as_str().to_string(),
            license_number: license_number.map(ToString::to_string),
            issue_date: None,
            expiry_date,
            last_submission_id: Some("ls_test".to_string()),
            notes: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        })
    }

    async fn create_submission(
        &self,
        req: &CreateLicenseSubmissionRequest,
    ) -> ramp_common::Result<LicenseSubmissionRow> {
        let row = LicenseSubmissionRow {
            id: "ls_test".to_string(),
            tenant_id: req.tenant_id.0.clone(),
            requirement_id: req.requirement_id.0.clone(),
            documents: req.documents.clone(),
            status: SubmissionStatus::Submitted.as_str().to_string(),
            submitted_by: req.submitted_by.clone(),
            submitted_at: Utc::now(),
            reviewed_at: None,
            reviewer_notes: None,
        };
        self.submissions.lock().unwrap().push(row.clone());
        Ok(row)
    }

    async fn list_submissions(
        &self,
        tenant_id: &TenantId,
        requirement_id: Option<&LicenseRequirementId>,
        limit: i64,
        offset: i64,
    ) -> ramp_common::Result<Vec<LicenseSubmissionRow>> {
        let submissions = self.submissions.lock().unwrap();
        Ok(submissions
            .iter()
            .filter(|row| row.tenant_id == tenant_id.0)
            .filter(|row| {
                requirement_id
                    .map(|value| row.requirement_id == value.0)
                    .unwrap_or(true)
            })
            .skip(offset.max(0) as usize)
            .take(limit.max(0) as usize)
            .cloned()
            .collect())
    }

    async fn update_submission_status(
        &self,
        _submission_id: &ramp_common::licensing::LicenseSubmissionId,
        _status: SubmissionStatus,
        _reviewer_notes: Option<&str>,
    ) -> ramp_common::Result<()> {
        Ok(())
    }

    async fn get_upcoming_deadlines(
        &self,
        tenant_id: &TenantId,
        _days_ahead: i32,
    ) -> ramp_common::Result<Vec<(LicenseRequirementRow, Option<TenantLicenseStatusRow>)>> {
        Ok(self
            .requirements
            .iter()
            .cloned()
            .map(|requirement| {
                let status = self
                    .statuses
                    .iter()
                    .find(|row| {
                        row.tenant_id == tenant_id.0 && row.requirement_id == requirement.id
                    })
                    .cloned();
                (requirement, status)
            })
            .collect())
    }

    async fn count_requirements(&self) -> ramp_common::Result<i64> {
        Ok(self.requirements.len() as i64)
    }
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

fn build_signed_admin_request(
    method: &str,
    uri: &str,
    body: &str,
    api_key: &str,
    api_secret: &str,
    admin_key: &str,
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
        .header("X-Admin-Key", admin_key);

    if !body.is_empty() {
        builder = builder.header("Content-Type", "application/json");
    }

    builder.body(Body::from(body.to_string())).unwrap()
}

async fn setup_app() -> TestApp {
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
        id: "tenant_security_test".to_string(),
        name: "Security Test Tenant".to_string(),
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
        .expect("failed to create lazy pool");
    let report_generator = Arc::new(ReportGenerator::new(
        pool,
        Arc::new(MockDocumentStorage::new()),
    ));
    let case_manager = Arc::new(CaseManager::new(Arc::new(InMemoryCaseStore::new())));
    let licensing_repo = Arc::new(MockLicensingRepository {
        requirements: vec![LicenseRequirementRow {
            id: "req_sbv".to_string(),
            name: "SBV permit".to_string(),
            description: "Permit".to_string(),
            license_type: "SBV".to_string(),
            regulatory_body: "SBV".to_string(),
            deadline: None,
            renewal_period_days: None,
            required_documents: serde_json::json!(["permit"]),
            is_mandatory: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }],
        statuses: vec![TenantLicenseStatusRow {
            id: "tls_other".to_string(),
            tenant_id: "tenant_other".to_string(),
            requirement_id: "req_sbv".to_string(),
            status: "APPROVED".to_string(),
            license_number: Some("LIC-123".to_string()),
            issue_date: None,
            expiry_date: None,
            last_submission_id: None,
            notes: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }],
        submissions: Mutex::new(Vec::new()),
    });

    let app_state = AppState {
        payin_service,
        payout_service,
        trade_service,
        ledger_service,
        onboarding_service,
        user_service,
        webhook_service: Arc::new(
            ramp_core::service::webhook::WebhookService::new(
                webhook_repo,
                tenant_repo.clone(),
            )
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
            jwt_secret: "admin-security-test-secret".to_string(),
            issuer: None,
            audience: None,
            allow_missing_tenant: false,
        }),
        bank_confirmation_repo: None,
        licensing_repo: Some(licensing_repo),
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
        db_pool: None,
        ctr_service: None,
        ws_state: None,
        metrics_registry: Arc::new(ramp_core::service::MetricsRegistry::new()),
        event_publisher,
    };

    TestApp {
        router: create_router(app_state),
        api_key: TEST_API_KEY.to_string(),
        api_secret: TEST_API_SECRET.to_string(),
    }
}

#[tokio::test]
async fn licensing_status_rejects_cross_tenant_lookup() {
    let _guard = env_lock().lock().unwrap();
    std::env::set_var("RAMPOS_ADMIN_KEY", TEST_ADMIN_KEY);
    std::env::remove_var("RAMPOS_ADMIN_ROLE");
    let app = setup_app().await;

    let request = build_signed_admin_request(
        "GET",
        "/v1/admin/licensing/status/tenant_other",
        "",
        &app.api_key,
        &app.api_secret,
        TEST_ADMIN_KEY,
    );

    let response = app.router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn licensing_submit_rejects_viewer_role() {
    let _guard = env_lock().lock().unwrap();
    std::env::set_var("RAMPOS_ADMIN_KEY", TEST_ADMIN_KEY);
    std::env::set_var("RAMPOS_ADMIN_ROLE", "viewer");
    let app = setup_app().await;

    let body = serde_json::json!({
        "requirementId": "req_sbv",
        "documents": [{
            "name": "permit.pdf",
            "fileUrl": "https://files.example/permit.pdf",
            "fileHash": "abc123",
            "fileSizeBytes": 1024
        }]
    })
    .to_string();

    let request = build_signed_admin_request(
        "POST",
        "/v1/admin/licensing/submit",
        &body,
        &app.api_key,
        &app.api_secret,
        TEST_ADMIN_KEY,
    );

    let response = app.router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    std::env::remove_var("RAMPOS_ADMIN_ROLE");
}

#[tokio::test]
async fn recon_batch_creation_rejects_viewer_role() {
    let _guard = env_lock().lock().unwrap();
    std::env::set_var("RAMPOS_ADMIN_KEY", TEST_ADMIN_KEY);
    std::env::set_var("RAMPOS_ADMIN_ROLE", "viewer");
    let app = setup_app().await;

    let body = serde_json::json!({
        "railsProvider": "vietqr",
        "periodStart": "2026-03-01T00:00:00Z",
        "periodEnd": "2026-03-02T00:00:00Z"
    })
    .to_string();

    let request = build_signed_admin_request(
        "POST",
        "/v1/admin/recon/batches",
        &body,
        &app.api_key,
        &app.api_secret,
        TEST_ADMIN_KEY,
    );

    let response = app.router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    std::env::remove_var("RAMPOS_ADMIN_ROLE");
}

#[tokio::test]
async fn licensing_status_returns_not_found_for_other_tenant_without_body_leak() {
    let _guard = env_lock().lock().unwrap();
    std::env::set_var("RAMPOS_ADMIN_KEY", TEST_ADMIN_KEY);
    std::env::remove_var("RAMPOS_ADMIN_ROLE");
    let app = setup_app().await;

    let request = build_signed_admin_request(
        "GET",
        "/v1/admin/licensing/status/tenant_other",
        "",
        &app.api_key,
        &app.api_secret,
        TEST_ADMIN_KEY,
    );

    let response = app.router.oneshot(request).await.unwrap();
    let status = response.status();
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let payload = serde_json::from_slice::<serde_json::Value>(&body).unwrap();

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_ne!(payload["tenantId"], "tenant_other");
}
