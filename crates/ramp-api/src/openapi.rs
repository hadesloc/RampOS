//! OpenAPI documentation for RampOS API

use axum::http::{header, StatusCode};
use axum::response::{Html, IntoResponse};
use axum::Json;
use utoipa::OpenApi;

use crate::dto::*;
use crate::handlers::aa::*;
use crate::handlers::balance::*;
use crate::handlers::chain::*;
use crate::handlers::domain::*;
use crate::handlers::health::*;
use crate::handlers::intent::*;
use crate::handlers::payin::*;
use crate::handlers::payout::*;
use crate::handlers::stablecoin::*;
use crate::handlers::trade::*;
// Import admin and bank_webhook handlers + utoipa __path_xxx generated types via re-export
#[allow(unused_imports)]
use crate::handlers::{
    list_cases, get_case, update_case, get_case_stats,
    list_users, get_user, get_dashboard,
    handle_bank_webhook,
    __path_list_cases, __path_get_case, __path_update_case, __path_get_case_stats,
    __path_list_users, __path_get_user, __path_get_dashboard,
    __path_handle_bank_webhook,
};
// Admin types with aliases to avoid name conflicts
use crate::handlers::admin::{
    CaseResponse, ListCasesResponse, UpdateCaseRequest, CaseStats, SeverityStats,
    UserResponse as AdminUserResponse, ListUsersResponse,
    DashboardStats, IntentStats as AdminIntentStats, UserStats, VolumeStats,
};

/// RampOS API Documentation
#[derive(OpenApi)]
#[openapi(
    info(
        title = "RampOS API",
        version = "1.0.0",
        description = "BYOR (Bring Your Own Rails) - Crypto/VND Exchange Infrastructure API\n\n## Request Validation\n\nAll request bodies are validated before processing. Validation errors return HTTP 400 with detailed field-level error information in the following format:\n\n```json\n{\n  \"error\": {\n    \"code\": \"VALIDATION_ERROR\",\n    \"message\": \"Validation failed for N fields\",\n    \"details\": {\n      \"fieldName\": [{\"code\": \"length\", \"message\": \"...\"}]\n    }\n  }\n}\n```\n\n## Authentication\n\nAll endpoints require authentication via Bearer token (API key) in the Authorization header.",
        contact(
            name = "RampOS Team",
            email = "support@rampos.io",
            url = "https://rampos.io"
        ),
        license(
            name = "MIT",
            url = "https://opensource.org/licenses/MIT"
        )
    ),
    servers(
        (url = "https://api.rampos.io", description = "Production"),
        (url = "https://staging-api.rampos.io", description = "Staging"),
        (url = "http://localhost:3000", description = "Development")
    ),
    tags(
        (name = "intents", description = "Intent management (payin, payout, trade)"),
        (name = "events", description = "Event ingestion from rails providers"),
        (name = "users", description = "User management and balances"),
        (name = "admin", description = "Administrative endpoints for tenant and user management"),
        (name = "health", description = "Health check endpoints"),
        (name = "account-abstraction", description = "ERC-4337 Account Abstraction endpoints for smart accounts and UserOperations"),
        (name = "chains", description = "Multi-chain operations and cross-chain bridging"),
        (name = "stablecoin", description = "VNST stablecoin mint, burn, reserves, and peg status"),
        (name = "domains", description = "Custom domain management for tenants"),
        (name = "webhooks", description = "Incoming bank webhook processing")
    ),
    paths(
        // Intents
        create_payin,
        confirm_payin,
        create_payout,
        get_intent,
        list_intents,
        list_intents_cursor,
        // Events
        record_trade,
        // Users
        get_user_balances,
        get_user_balances_for_tenant,
        // Health
        health_check,
        readiness_check,
        // Account Abstraction
        create_account,
        get_account,
        send_user_operation,
        estimate_gas,
        get_user_operation,
        get_user_operation_receipt,
        // Chains
        list_chains,
        get_chain_detail,
        get_bridge_quote,
        initiate_bridge,
        // Stablecoin
        mint_vnst,
        burn_vnst,
        get_vnst_reserves,
        get_vnst_peg_status,
        get_vnst_config,
        // Domains
        list_domains,
        create_domain,
        get_domain,
        delete_domain,
        verify_dns,
        provision_ssl,
        // Webhooks
        handle_bank_webhook,
        // Admin
        list_cases,
        get_case,
        update_case,
        get_case_stats,
        list_users,
        get_user,
        get_dashboard
    ),
    components(
        schemas(
            // Request DTOs
            CreatePayinRequest,
            ConfirmPayinRequest,
            CreatePayoutRequest,
            TradeExecutedRequest,
            BankAccountDto,
            // Response DTOs
            CreatePayinResponse,
            ConfirmPayinResponse,
            CreatePayoutResponse,
            TradeExecutedResponse,
            VirtualAccountDto,
            IntentResponse,
            ListIntentsResponse,
            PaginationInfo,
            StateHistoryEntry,
            UserBalancesResponse,
            BalanceDto,
            HealthResponse,
            // Admin DTOs
            CreateTenantRequest,
            UpdateTenantRequest,
            SuspendTenantRequest,
            TierChangeRequest,
            CaseResponse,
            ListCasesResponse,
            UpdateCaseRequest,
            CaseStats,
            SeverityStats,
            AdminUserResponse,
            ListUsersResponse,
            DashboardStats,
            AdminIntentStats,
            UserStats,
            VolumeStats,
            // Account Abstraction DTOs
            CreateAccountRequest,
            CreateAccountResponse,
            GetAccountResponse,
            UserOperationDto,
            SendUserOpRequest,
            SendUserOpResponse,
            EstimateGasRequest,
            EstimateGasResponse,
            UserOpReceiptDto,
            // Chain DTOs
            ChainListResponse,
            ChainDetailResponse,
            BridgeQuoteRequest,
            BridgeQuoteResponse,
            FeeBreakdown,
            BridgeRequest,
            BridgeResponse,
            // Stablecoin DTOs
            VnstMintApiRequest,
            VnstMintApiResponse,
            VnstBurnApiRequest,
            VnstBurnApiResponse,
            VnstReservesApiResponse,
            ReserveAssetResponse,
            VnstPegStatusResponse,
            VnstConfigResponse,
            // Domain DTOs
            CreateDomainRequest,
            DomainResponse,
            SslCertificateInfoResponse,
            DomainListResponse,
            DnsVerificationResponse,
            SslProvisioningResponse,
            DeleteDomainResponse,
            // Error responses
            ErrorResponse,
            ErrorBody,
            ValidationErrorResponse,
            ValidationErrorBody,
            ValidationFieldError
        )
    ),
    modifiers(&SecurityAddon)
)]
pub struct ApiDoc;

/// Serve the raw OpenAPI JSON spec at /openapi.json
pub async fn openapi_json() -> impl IntoResponse {
    let spec = ApiDoc::openapi();
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/json")],
        Json(spec),
    )
}

/// Serve Scalar API reference UI at /docs
pub async fn docs_handler() -> Html<String> {
    Html(r#"<!DOCTYPE html>
<html>
<head>
    <title>RampOS API Documentation</title>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
</head>
<body>
    <script id="api-reference" data-url="/openapi.json"></script>
    <script src="https://cdn.jsdelivr.net/npm/@scalar/api-reference"></script>
</body>
</html>"#.to_string())
}

/// Security addon for API authentication
struct SecurityAddon;

impl utoipa::Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi
            .components
            .get_or_insert_with(utoipa::openapi::Components::default);

        // Add Bearer token authentication
        components.add_security_scheme(
            "bearer_auth",
            utoipa::openapi::security::SecurityScheme::Http(
                utoipa::openapi::security::HttpBuilder::new()
                    .scheme(utoipa::openapi::security::HttpAuthScheme::Bearer)
                    .bearer_format("API Key")
                    .description(Some("API key authentication. Use your tenant API key."))
                    .build(),
            ),
        );

        // Add HMAC signature authentication
        components.add_security_scheme(
            "hmac_signature",
            utoipa::openapi::security::SecurityScheme::ApiKey(
                utoipa::openapi::security::ApiKey::Header(
                    utoipa::openapi::security::ApiKeyValue::new("X-Signature"),
                ),
            ),
        );

        // Add Idempotency key header
        components.add_security_scheme(
            "idempotency_key",
            utoipa::openapi::security::SecurityScheme::ApiKey(
                utoipa::openapi::security::ApiKey::Header(
                    utoipa::openapi::security::ApiKeyValue::new("Idempotency-Key"),
                ),
            ),
        );
    }
}

// Re-export error types for OpenAPI
#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct ErrorResponse {
    pub error: ErrorBody,
}

#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct ErrorBody {
    /// Error code (e.g., "NOT_FOUND", "BAD_REQUEST")
    pub code: String,
    /// Human-readable error message
    pub message: String,
}

/// Validation error response with field-level details
#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct ValidationErrorResponse {
    pub error: ValidationErrorBody,
}

/// Validation error body with detailed field errors
#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct ValidationErrorBody {
    /// Error code (always "VALIDATION_ERROR")
    #[schema(example = "VALIDATION_ERROR")]
    pub code: String,
    /// Summary message
    #[schema(example = "Validation failed for 2 fields")]
    pub message: String,
    /// Field-level error details (field name -> list of errors)
    pub details: std::collections::HashMap<String, Vec<ValidationFieldError>>,
}

/// Individual field validation error
#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct ValidationFieldError {
    /// Validation rule code (e.g., "length", "range", "email")
    #[schema(example = "length")]
    pub code: String,
    /// Human-readable error message
    #[schema(example = "Length must be between 1 and 64 characters")]
    pub message: String,
    /// Optional parameters for the validation rule
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use utoipa::OpenApi;

    #[test]
    fn test_openapi_spec_valid() {
        let spec = ApiDoc::openapi();
        let json = spec.to_json().unwrap();
        assert!(json.contains("RampOS API"));
        assert!(json.contains("/v1/intents"));
    }
}
