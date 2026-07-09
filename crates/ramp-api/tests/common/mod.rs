pub async fn run_test_migrations(pool: &sqlx::PgPool) {
    let source_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../migrations");
    let unique_suffix = format!(
        "{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock should be after UNIX_EPOCH")
            .as_nanos()
    );
    let temp_dir = std::env::temp_dir().join(format!("rampos-test-migrations-{unique_suffix}"));
    std::fs::create_dir_all(&temp_dir).expect("Failed to create temp migration directory");
    let mut deferred_enum_additions = Vec::new();

    for entry in std::fs::read_dir(&source_dir).expect("Failed to list migrations") {
        let path = entry.expect("Failed to read migration entry").path();
        if !path.is_file() || path.extension().and_then(|ext| ext.to_str()) != Some("sql") {
            continue;
        }

        let file_name = path.file_name().expect("Migration should have a file name");
        let mut sql = std::fs::read_to_string(&path).expect("Failed to read migration");
        if matches!(
            file_name.to_str(),
            Some("037_travel_rule.sql" | "039_rescreening_runs.sql")
        ) {
            let mut filtered = Vec::new();
            for line in sql.lines() {
                let trimmed = line.trim_start();
                if trimmed.starts_with("ALTER TYPE compliance_event_type ADD VALUE") {
                    deferred_enum_additions.push(trimmed.to_string());
                } else {
                    filtered.push(line);
                }
            }
            sql = filtered.join("\n");
        }
        std::fs::write(temp_dir.join(file_name), sql).expect("Failed to stage migration");
    }

    let migrator = sqlx::migrate::Migrator::new(temp_dir.clone())
        .await
        .expect("Failed to build test migrator");
    migrator
        .run(pool)
        .await
        .expect("Failed to run test migrations");

    for statement in deferred_enum_additions {
        sqlx::query(&statement)
            .execute(pool)
            .await
            .expect("Failed to apply deferred enum addition");
    }

    let _ = std::fs::remove_dir_all(temp_dir);
}

pub fn setup_portal_app(pool: sqlx::PgPool, jwt_secret: &str) -> axum::Router {
    use ramp_api::middleware::PortalAuthConfig;
    use ramp_api::{create_router, AppState};
    use ramp_compliance::{
        case::CaseManager, reports::ReportGenerator, storage::MockDocumentStorage,
        InMemoryCaseStore,
    };
    use ramp_core::event::InMemoryEventPublisher;
    use ramp_core::service::{
        ledger::LedgerService, payin::PayinService, payout::PayoutService, trade::TradeService,
    };
    use ramp_core::test_utils::*;
    use std::sync::Arc;

    let intent_repo = Arc::new(MockIntentRepository::new());
    let ledger_repo = Arc::new(MockLedgerRepository::new());
    let user_repo = Arc::new(MockUserRepository::new());
    let tenant_repo = Arc::new(MockTenantRepository::new());
    let event_publisher = Arc::new(InMemoryEventPublisher::new());
    let ledger_service = Arc::new(LedgerService::new(ledger_repo.clone()));

    create_router(AppState {
        payin_service: Arc::new(PayinService::new(
            intent_repo.clone(),
            ledger_repo.clone(),
            user_repo.clone(),
            event_publisher.clone(),
        )),
        payout_service: Arc::new(PayoutService::new(
            intent_repo.clone(),
            ledger_repo.clone(),
            user_repo.clone(),
            event_publisher.clone(),
        )),
        trade_service: Arc::new(TradeService::new(
            intent_repo.clone(),
            ledger_repo,
            event_publisher.clone(),
        )),
        ledger_service: ledger_service.clone(),
        onboarding_service: Arc::new(ramp_core::service::onboarding::OnboardingService::new(
            tenant_repo.clone(),
            ledger_service,
        )),
        user_service: Arc::new(ramp_core::service::user::UserService::new(
            user_repo,
            event_publisher.clone(),
        )),
        webhook_service: Arc::new(
            ramp_core::service::webhook::WebhookService::new(
                Arc::new(MockWebhookRepository::new()),
                tenant_repo.clone(),
            )
            .unwrap(),
        ),
        tenant_repo,
        intent_repo,
        report_generator: Arc::new(ReportGenerator::new(
            pool.clone(),
            Arc::new(MockDocumentStorage::new()),
        )),
        case_manager: Arc::new(CaseManager::new(Arc::new(InMemoryCaseStore::new()))),
        rule_manager: None,
        rate_limiter: None,
        idempotency_handler: None,
        aa_service: None,
        portal_auth_config: Arc::new(PortalAuthConfig {
            jwt_secret: jwt_secret.to_string(),
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
        event_publisher,
        db_pool: Some(pool),
        ctr_service: None,
        ws_state: None,
        metrics_registry: Arc::new(ramp_core::service::MetricsRegistry::new()),
        document_storage: None,
        kyc_service: None,
        kyt_service: None,
    })
}
