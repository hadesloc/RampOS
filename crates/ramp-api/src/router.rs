use axum::http::{header, HeaderName, HeaderValue, Method};
use axum::{
    middleware,
    routing::{get, patch, post, put},
    Router,
};
use std::sync::Arc;
use tower::ServiceBuilder;
use tower_http::{
    cors::{AllowHeaders, AllowMethods, AllowOrigin, CorsLayer},
    sensitive_headers::{SetSensitiveRequestHeadersLayer, SetSensitiveResponseHeadersLayer},
    set_header::SetResponseHeaderLayer,
    timeout::TimeoutLayer,
    trace::TraceLayer,
};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use ramp_core::repository::intent::IntentRepository;
use ramp_core::repository::tenant::TenantRepository;
use ramp_core::repository::BankConfirmationRepository;
use ramp_core::service::{
    ledger::LedgerService, onboarding::OnboardingService, payin::PayinService,
    payout::PayoutService, trade::TradeService, user::UserService, webhook::WebhookService,
};

use crate::handlers;
use crate::handlers::aa::AAServiceState;
use crate::handlers::bank_webhooks::BankWebhookState;
use crate::middleware::{
    auth_middleware, idempotency_middleware, portal_auth_middleware, rate_limit_middleware,
    request_id_middleware, IdempotencyHandler, PortalAuthConfig, RateLimiter,
};
use crate::openapi::ApiDoc;

use ramp_compliance::case::CaseManager;
use ramp_compliance::reports::ReportGenerator;
use ramp_compliance::rules::RuleCacheManager;

/// Application state shared across handlers
#[derive(Clone)]
pub struct AppState {
    pub payin_service: Arc<PayinService>,
    pub payout_service: Arc<PayoutService>,
    pub trade_service: Arc<TradeService>,
    pub ledger_service: Arc<LedgerService>,
    pub onboarding_service: Arc<OnboardingService>,
    pub user_service: Arc<UserService>,
    pub webhook_service: Arc<WebhookService>,
    pub tenant_repo: Arc<dyn TenantRepository>,
    pub intent_repo: Arc<dyn IntentRepository>,
    pub report_generator: Arc<ReportGenerator>,
    pub case_manager: Arc<CaseManager>,
    pub rule_manager: Option<Arc<RuleCacheManager>>,
    pub rate_limiter: Option<Arc<RateLimiter>>,
    pub idempotency_handler: Option<Arc<IdempotencyHandler>>,
    pub aa_service: Option<AAServiceState>,
    pub portal_auth_config: Arc<PortalAuthConfig>,
    pub bank_confirmation_repo: Option<Arc<dyn BankConfirmationRepository>>,
}

/// Create the API router with full middleware stack
pub fn create_router(state: AppState) -> Router {
    // Health routes (no auth required)
    let health_routes = Router::new()
        .route("/health", get(handlers::health_check))
        .route("/ready", get(handlers::readiness_check));

    // Intent routes (auth required)
    // Split into sub-routers because they require different state types
    let intent_read_routes = Router::new()
        .route("/", get(handlers::list_intents))
        .route("/:id", get(handlers::get_intent))
        .with_state(state.intent_repo.clone());

    let payin_routes = Router::new()
        .route("/payin", post(handlers::create_payin))
        .route("/payin/confirm", post(handlers::confirm_payin))
        .with_state(state.payin_service.clone());

    let payout_routes = Router::new()
        .route("/payout", post(handlers::create_payout))
        .with_state(state.payout_service.clone());

    let intent_routes = Router::new()
        .merge(intent_read_routes)
        .merge(payin_routes)
        .merge(payout_routes);

    // Event routes (auth required)
    let event_routes = Router::new()
        .route("/trade-executed", post(handlers::record_trade))
        .with_state(state.trade_service.clone());

    // Balance routes (auth required)
    let balance_routes = Router::new()
        .route("/:user_id", get(handlers::get_user_balances))
        .with_state(state.ledger_service.clone());

    // Legacy/alias balance route for SDK compatibility
    let user_balance_alias_routes = Router::new()
        .route(
            "/users/:tenant_id/:user_id/balances",
            get(handlers::get_user_balances_for_tenant),
        )
        .with_state(state.ledger_service.clone());

    // Report routes
    let report_routes = Router::new()
        .route("/aml", get(handlers::admin::reports::generate_aml_report))
        .route(
            "/aml/export",
            get(handlers::admin::reports::export_aml_report),
        )
        .route("/kyc", get(handlers::admin::reports::generate_kyc_report))
        .route(
            "/kyc/export",
            get(handlers::admin::reports::export_kyc_report),
        )
        .with_state(state.report_generator.clone());

    // Admin routes (auth required + admin role)
    // We need to split admin routes because they use different states

    // 1. Tenant Management Routes -> OnboardingService
    let tenant_routes = Router::new()
        .route("/tenants", post(handlers::create_tenant))
        .route("/tenants/:id", patch(handlers::update_tenant))
        .route("/tenants/:id/api-keys", post(handlers::generate_api_keys))
        .route("/tenants/:id/activate", post(handlers::activate_tenant))
        .route("/tenants/:id/suspend", post(handlers::suspend_tenant))
        .with_state(state.onboarding_service.clone());

    // 2. Report Routes -> ReportGenerator
    // Already defined above as report_routes

    // 3. Other Admin Routes (Cases, Users, etc) - These use AppState
    let admin_general_routes = Router::new()
        // Dashboard
        .route("/dashboard", get(handlers::get_dashboard))
        // Intents
        .route("/intents/:id/cancel", post(handlers::admin::cancel_intent))
        .route("/intents/:id/retry", post(handlers::admin::retry_intent))
        // Rules
        .route("/rules", get(handlers::admin::list_rules))
        .route("/rules", post(handlers::admin::create_rule))
        .route("/rules/:id", put(handlers::admin::update_rule))
        .route("/rules/:id/toggle", put(handlers::admin::toggle_rule))
        // Ledger
        .route("/ledger/entries", get(handlers::admin::list_entries))
        .route("/ledger/balances", get(handlers::admin::list_balances))
        // Webhooks
        .route("/webhooks", get(handlers::admin::list_webhooks))
        .route("/webhooks/:id", get(handlers::admin::get_webhook))
        .route("/webhooks/:id/retry", post(handlers::admin::retry_webhook))
        // Cases
        .route("/cases", get(handlers::list_cases))
        .route("/cases/stats", get(handlers::get_case_stats))
        .route("/cases/:id", get(handlers::get_case))
        .route("/cases/:id", patch(handlers::update_case))
        .route(
            "/cases/:id/sar",
            post(handlers::admin::reports::generate_sar),
        )
        // Users
        .route("/users", get(handlers::list_users))
        .route("/users/:id", get(handlers::get_user))
        .route("/users/:id", patch(handlers::update_user))
        // Reconciliation
        .route("/recon/batches", get(handlers::list_recon_batches))
        .route("/recon/batches", post(handlers::create_recon_batch))
        .with_state(state.clone());

    // Admin Reports - needs to be separated or use AppState if ReportGenerator is in AppState
    // The previous error was: expected `Arc<ReportGenerator>`, found `AppState` at line 133 .with_state(state.clone());
    // This is because we used .nest("/reports", report_routes) and report_routes requires state.report_generator (Arc<ReportGenerator>)
    // but here we are merging admin_general_routes which has .with_state(AppState).
    // Wait, report_routes was created with .with_state(state.report_generator.clone()) earlier.
    // So report_routes itself is a Router<()>, not Router<S>. It already has its state captured?
    // Let's check report_routes definition.
    // Line 92: let report_routes = Router::new()... .with_state(state.report_generator.clone());
    // So report_routes is a Router (which is Router<()>).
    // admin_general_routes is Router<AppState> because we call .with_state(state.clone()) at the end.
    // Wait, if we call .with_state() it becomes Router<()>.
    // So admin_general_routes is Router<()>.
    // tenant_routes is Router<()> (state.onboarding_service).
    // tier_routes is Router<()> (state.user_service).
    // So all are Router<()>.
    // We should be able to merge/nest them.
    // The previous error was at admin_general_routes declaration?
    // "expected `Arc<ReportGenerator>`, found `AppState`"
    // No, the error message:
    // error[E0308]: mismatched types
    //    --> crates\ramp-api\src\router.rs:133:21
    //     |
    // 133 |         .with_state(state.clone());
    //     |          ---------- ^^^^^^^^^^^^^ expected `Arc<ReportGenerator>`, found `AppState`
    //
    // Ah, `generate_sar` handler probably expects `State<Arc<ReportGenerator>>` but we are providing `AppState`.
    // Let's check `generate_sar`.

    // If generate_sar takes ReportGenerator, we must use a route with that state OR change generate_sar to take AppState.
    // Let's verify generate_sar signature.

    // Tier Management
    let tier_routes = Router::new()
        .route("/tiers", get(handlers::list_tiers))
        .route("/users/:user_id/tier", get(handlers::get_user_tier))
        .route(
            "/users/:user_id/tier/upgrade",
            post(handlers::upgrade_user_tier),
        )
        .route(
            "/users/:user_id/tier/downgrade",
            post(handlers::downgrade_user_tier),
        )
        .route("/users/:user_id/limits", get(handlers::get_user_limits))
        .with_state(state.clone());

    // Combine them
    let admin_routes = Router::new()
        .merge(admin_general_routes)
        .merge(tenant_routes)
        .merge(tier_routes)
        .nest("/reports", report_routes);

    // Account Abstraction (AA) routes
    // SECURITY: AA routes have stricter rate limiting due to expensive on-chain operations
    let aa_routes = if let Some(ref aa_service) = state.aa_service {
        let mut aa_router = Router::new()
            .route("/accounts", post(handlers::aa::create_account))
            .route("/accounts/:address", get(handlers::aa::get_account))
            .route("/user-operations", post(handlers::aa::send_user_operation))
            .route(
                "/user-operations/estimate",
                post(handlers::aa::estimate_gas),
            )
            .route(
                "/user-operations/:hash",
                get(handlers::aa::get_user_operation),
            )
            .route(
                "/user-operations/:hash/receipt",
                get(handlers::aa::get_user_operation_receipt),
            )
            .with_state(aa_service.clone());

        // SECURITY FIX: Apply stricter rate limiting to AA routes
        // AA operations are expensive (on-chain transactions), so we need tighter limits
        if let Some(ref limiter) = state.rate_limiter {
            aa_router = aa_router.layer(middleware::from_fn_with_state(
                limiter.clone(),
                rate_limit_middleware,
            ));
        }

        aa_router
    } else {
        // AA service not configured
        Router::new()
    };

    // Portal routes (user authentication via JWT, not tenant API key)
    // These routes are for the end-user portal application
    // Auth routes are excluded from JWT middleware (login/register don't need auth)
    let mut portal_protected_routes = Router::new()
        .nest("/kyc", handlers::portal::kyc::router())
        .nest("/wallet", handlers::portal::wallet::router())
        .nest("/transactions", handlers::portal::transactions::router())
        .nest("/intents", handlers::portal::intents::router())
        .layer(middleware::from_fn_with_state(
            state.portal_auth_config.clone(),
            portal_auth_middleware,
        ))
        .with_state(state.clone());

    if let Some(ref limiter) = state.rate_limiter {
        portal_protected_routes = portal_protected_routes.layer(middleware::from_fn_with_state(
            limiter.clone(),
            rate_limit_middleware,
        ));
    }

    // API v1 routes with authentication
    let mut api_v1 = Router::new()
        .nest("/intents", intent_routes)
        .nest("/events", event_routes)
        .nest("/balance", balance_routes)
        .merge(user_balance_alias_routes)
        .nest("/admin", admin_routes)
        .nest("/aa", aa_routes)
        .layer(middleware::from_fn_with_state(
            state.tenant_repo.clone(),
            auth_middleware,
        ));

    // Add rate limiting if available
    if let Some(ref limiter) = state.rate_limiter {
        api_v1 = api_v1.layer(middleware::from_fn_with_state(
            limiter.clone(),
            rate_limit_middleware,
        ));
    }

    // Add idempotency handling if available
    if let Some(ref handler) = state.idempotency_handler {
        api_v1 = api_v1.layer(middleware::from_fn_with_state(
            handler.clone(),
            idempotency_middleware,
        ));
    }

    // Bank webhook routes (no tenant auth required - uses provider-specific signature verification)
    // POST /v1/webhooks/bank/:provider - receives bank confirmations for pay-ins
    let mut bank_webhook_routes = if let Some(ref confirmation_repo) = state.bank_confirmation_repo {
        let bank_webhook_state = BankWebhookState::new(confirmation_repo.clone());
        Router::new()
            .route(
                "/:provider",
                post(handlers::bank_webhooks::handle_bank_webhook),
            )
            .with_state(bank_webhook_state)
    } else {
        Router::new()
    };

    if let Some(ref limiter) = state.rate_limiter {
        bank_webhook_routes = bank_webhook_routes.layer(middleware::from_fn_with_state(
            limiter.clone(),
            rate_limit_middleware,
        ));
    }

    // OpenAPI documentation
    let openapi = ApiDoc::openapi();

    let mut portal_auth_routes = handlers::portal::auth::router().with_state(state.clone());
    if let Some(ref limiter) = state.rate_limiter {
        portal_auth_routes = portal_auth_routes.layer(middleware::from_fn_with_state(
            limiter.clone(),
            rate_limit_middleware,
        ));
    }

    // Combine all routes
    // Note: More specific routes should be registered first to ensure proper matching
    Router::new()
        .merge(health_routes)
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", openapi))
        .nest("/v1", api_v1)
        // Portal auth routes (no JWT required - these issue tokens)
        .nest(
            "/v1/portal/auth",
            portal_auth_routes.clone(),
        )
        // Portal protected routes (JWT required)
        .nest("/v1/portal", portal_protected_routes)
        // Legacy auth endpoint (same as /v1/portal/auth for backwards compatibility)
        .nest(
            "/v1/auth",
            portal_auth_routes,
        )
        // Bank webhook routes (no auth required - uses signature verification)
        .nest("/v1/webhooks/bank", bank_webhook_routes)
        .layer(middleware::from_fn(request_id_middleware))
        .layer({
            let request_headers = Arc::new([
                header::AUTHORIZATION,
                header::PROXY_AUTHORIZATION,
                header::COOKIE,
                header::SET_COOKIE,
            ]);
            let response_headers = Arc::new([header::SET_COOKIE]);
            ServiceBuilder::new()
                .layer(SetSensitiveRequestHeadersLayer::from_shared(
                    request_headers,
                ))
                .layer(TraceLayer::new_for_http())
                .layer(SetSensitiveResponseHeadersLayer::from_shared(
                    response_headers,
                ))
        })
        .layer(TimeoutLayer::new(std::time::Duration::from_secs(30)))
        .layer(SetResponseHeaderLayer::overriding(
            header::STRICT_TRANSPORT_SECURITY,
            HeaderValue::from_static("max-age=31536000; includeSubDomains"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            header::X_CONTENT_TYPE_OPTIONS,
            HeaderValue::from_static("nosniff"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            header::X_FRAME_OPTIONS,
            HeaderValue::from_static("DENY"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            header::CONTENT_SECURITY_POLICY,
            HeaderValue::from_static("default-src 'self'"),
        ))
        .layer({
            // CORS configuration with credentials support
            // Note: When allow_credentials(true) is used, we CANNOT use wildcards (*) for
            // origins, methods, or headers. We must specify explicit values.
            let cors_origins = std::env::var("CORS_ALLOWED_ORIGINS")
                .unwrap_or_else(|_| "http://localhost:3000,http://localhost:3001".to_string());

            let origins: Vec<HeaderValue> = cors_origins
                .split(',')
                .filter_map(|s| s.trim().parse::<HeaderValue>().ok())
                .collect();

            let origins = if origins.is_empty() {
                vec![HeaderValue::from_static("http://localhost:3000")]
            } else {
                origins
            };

            // Explicit list of allowed headers (required when credentials are enabled)
            let allowed_headers = AllowHeaders::list([
                header::AUTHORIZATION,
                header::CONTENT_TYPE,
                header::ACCEPT,
                header::ORIGIN,
                header::COOKIE,
                HeaderName::from_static("x-tenant-id"),
                HeaderName::from_static("x-signature"),
                HeaderName::from_static("x-timestamp"),
                HeaderName::from_static("x-request-id"),
                HeaderName::from_static("x-idempotency-key"),
            ]);

            // Explicit list of allowed methods (required when credentials are enabled)
            let allowed_methods = AllowMethods::list([
                Method::GET,
                Method::POST,
                Method::PUT,
                Method::PATCH,
                Method::DELETE,
                Method::OPTIONS,
            ]);

            CorsLayer::new()
                .allow_origin(AllowOrigin::list(origins))
                .allow_methods(allowed_methods)
                .allow_headers(allowed_headers)
                .allow_credentials(true)
                .expose_headers([
                    header::CONTENT_TYPE,
                    HeaderName::from_static("x-request-id"),
                ])
        })
}
