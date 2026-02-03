//! RampOS Server - Main binary

use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::info;

use ramp_aa::types::ChainConfig;
use ramp_api::{
    create_router,
    handlers::aa::AAServiceState,
    middleware::{IdempotencyConfig, IdempotencyHandler, PortalAuthConfig},
    router::AppState,
};
use ramp_common::telemetry::{init_telemetry, shutdown_telemetry, TelemetryConfig};
use ramp_core::{
    config::Config,
    event::InMemoryEventPublisher,
    jobs::intent_timeout::IntentTimeoutJob,
    repository::{
        intent::PgIntentRepository, ledger::PgLedgerRepository, tenant::PgTenantRepository,
        user::PgUserRepository, webhook::PgWebhookRepository, PgSmartAccountRepository,
    },
    service::{
        ledger::LedgerService, onboarding::OnboardingService, payin::PayinService,
        payout::PayoutService, trade::TradeService, user::UserService, webhook::WebhookService,
    },
};

use ramp_compliance::case::CaseManager;
use ramp_compliance::providers::create_document_storage_async;
use ramp_compliance::reports::ReportGenerator;
use ramp_compliance::rules::RuleCacheManager;
use ramp_compliance::store::postgres::PostgresCaseStore;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize telemetry
    let telemetry_config = TelemetryConfig::from_env();
    init_telemetry(telemetry_config)?;

    info!("Starting RampOS Server v{}", env!("CARGO_PKG_VERSION"));

    // Load configuration
    let config = Config::from_env().unwrap_or_default();

    // Create database pool
    let pool = sqlx::PgPool::connect(&config.database.url)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to connect to database: {}", e))?;

    info!("Connected to database");

    // Run migrations
    sqlx::migrate!("../../migrations").run(&pool).await?;

    // Create repositories
    let intent_repo = Arc::new(PgIntentRepository::new(pool.clone()));
    let ledger_repo = Arc::new(PgLedgerRepository::new(pool.clone()));
    let tenant_repo = Arc::new(PgTenantRepository::new(pool.clone()));
    let user_repo = Arc::new(PgUserRepository::new(pool.clone()));
    let webhook_repo = Arc::new(PgWebhookRepository::new(pool.clone()));

    // Create event publisher (in production, would use NATS)
    let event_publisher = Arc::new(InMemoryEventPublisher::new());

    // Create services
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

    let webhook_service = Arc::new(WebhookService::new(
        webhook_repo.clone(),
        tenant_repo.clone(),
    ));

    let onboarding_service = Arc::new(OnboardingService::new(
        tenant_repo.clone(),
        ledger_service.clone(),
    ));

    let user_service = Arc::new(UserService::new(user_repo.clone(), event_publisher.clone()));

    // Create ReportGenerator
    // Use factory to create document storage from config
    let document_storage = create_document_storage_async(&config.providers.document_storage).await;
    let report_generator = Arc::new(ReportGenerator::new(pool.clone(), document_storage.into()));

    let case_store = Arc::new(PostgresCaseStore::new(pool.clone()));
    let case_manager = Arc::new(CaseManager::new(case_store));

    // Create Redis connection and idempotency handler
    let redis_client = redis::Client::open(config.redis.url.as_str())
        .map_err(|e| anyhow::anyhow!("Invalid Redis URL: {}", e))?;
    let redis_manager = redis::aio::ConnectionManager::new(redis_client.clone())
        .await
        .map_err(|e| anyhow::anyhow!("Failed to connect to Redis: {}", e))?;

    let rule_manager = Arc::new(RuleCacheManager::new(redis_client, 3600));

    let idempotency_handler = Arc::new(IdempotencyHandler::with_redis(
        redis_manager.clone(),
        IdempotencyConfig::default(),
    ));

    let rate_limiter = Arc::new(ramp_api::middleware::RateLimiter::with_redis(
        redis_manager,
        ramp_api::middleware::RateLimitConfig::default(),
    ));

    // Create AA (Account Abstraction) service if enabled
    let aa_service = if std::env::var("AA_ENABLED").unwrap_or_default() == "true" {
        info!("Account Abstraction (AA) service enabled");

        // Create smart account repository for ownership verification
        let smart_account_repo = Arc::new(PgSmartAccountRepository::new(pool.clone()));

        // Get chain configuration from environment
        let chain_id: u64 = std::env::var("AA_CHAIN_ID")
            .unwrap_or_else(|_| "1".to_string())
            .parse()
            .map_err(|e| anyhow::anyhow!("AA_CHAIN_ID must be a valid number: {}", e))?;

        let chain_name =
            std::env::var("AA_CHAIN_NAME").unwrap_or_else(|_| "Ethereum Mainnet".to_string());

        let bundler_url =
            std::env::var("AA_BUNDLER_URL").unwrap_or_else(|_| "http://localhost:4337".to_string());

        let entry_point_address = std::env::var("AA_ENTRY_POINT_ADDRESS")
            .unwrap_or_else(|_| "0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789".to_string())
            .parse()
            .map_err(|e| {
                anyhow::anyhow!(
                    "AA_ENTRY_POINT_ADDRESS must be a valid Ethereum address: {}",
                    e
                )
            })?;

        let paymaster_address = std::env::var("AA_PAYMASTER_ADDRESS")
            .ok()
            .and_then(|s| s.parse().ok());

        let chain_config = ChainConfig {
            chain_id,
            name: chain_name,
            entry_point_address,
            bundler_url,
            paymaster_address,
        };

        Some(AAServiceState::new_with_repo(
            chain_config,
            Some(smart_account_repo),
        )?)
    } else {
        info!("Account Abstraction (AA) service disabled - set AA_ENABLED=true to enable");
        None
    };

    // Create portal auth configuration
    let portal_auth_config = Arc::new(PortalAuthConfig::default());

    // Create application state
    let app_state = AppState {
        payin_service,
        payout_service,
        trade_service,
        ledger_service,
        onboarding_service,
        user_service,
        webhook_service: webhook_service.clone(),
        tenant_repo: tenant_repo.clone(),
        intent_repo: intent_repo.clone(),
        report_generator: report_generator.clone(),
        case_manager: case_manager.clone(),
        rule_manager: Some(rule_manager),
        rate_limiter: Some(rate_limiter),
        idempotency_handler: Some(idempotency_handler),
        aa_service,
        portal_auth_config,
        bank_confirmation_repo: None,
    };

    // Create router
    let app = create_router(app_state);

    // Start server
    let addr = format!("{}:{}", config.server.host, config.server.port);
    let listener = TcpListener::bind(&addr).await?;

    info!("RampOS API server listening on {}", addr);

    // Start webhook processor in background
    let webhook_service_clone = webhook_service.clone();
    tokio::spawn(async move {
        loop {
            match webhook_service_clone.process_pending_events(100).await {
                Ok(delivered) => {
                    if delivered > 0 {
                        info!(delivered = delivered, "Processed webhook events");
                    }
                }
                Err(e) => {
                    tracing::error!(error = %e, "Error processing webhooks");
                }
            }
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        }
    });

    // Start intent timeout job
    let timeout_service = Arc::new(ramp_core::service::TimeoutService::new(
        intent_repo.clone(),
        event_publisher.clone(),
    ));
    let timeout_job = IntentTimeoutJob::new(timeout_service);
    tokio::spawn(async move {
        timeout_job.run().await;
    });

    // Serve
    axum::serve(listener, app).await?;

    shutdown_telemetry();

    Ok(())
}
