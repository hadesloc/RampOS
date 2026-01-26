//! OpenAPI documentation for RampOS API

use utoipa::OpenApi;

use crate::dto::*;
use crate::handlers::intent::*;
use crate::handlers::payin::*;
use crate::handlers::payout::*;
use crate::handlers::trade::*;
use crate::handlers::balance::*;
use crate::handlers::health::*;

/// RampOS API Documentation
#[derive(OpenApi)]
#[openapi(
    info(
        title = "RampOS API",
        version = "1.0.0",
        description = "BYOR (Bring Your Own Rails) - Crypto/VND Exchange Infrastructure API",
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
        (name = "health", description = "Health check endpoints")
    ),
    paths(
        // Intents
        create_payin,
        confirm_payin,
        create_payout,
        get_intent,
        list_intents,
        // Events
        record_trade,
        // Users
        get_user_balances,
        // Health
        health_check,
        readiness_check
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
            // Error responses
            ErrorResponse,
            ErrorBody
        )
    ),
    modifiers(&SecurityAddon)
)]
pub struct ApiDoc;

/// Security addon for API authentication
struct SecurityAddon;

impl utoipa::Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi.components.as_mut().unwrap();

        // Add Bearer token authentication
        components.add_security_scheme(
            "bearer_auth",
            utoipa::openapi::security::SecurityScheme::Http(
                utoipa::openapi::security::HttpBuilder::new()
                    .scheme(utoipa::openapi::security::HttpAuthScheme::Bearer)
                    .bearer_format("API Key")
                    .description(Some("API key authentication. Use your tenant API key."))
                    .build()
            ),
        );

        // Add HMAC signature authentication
        components.add_security_scheme(
            "hmac_signature",
            utoipa::openapi::security::SecurityScheme::ApiKey(
                utoipa::openapi::security::ApiKey::Header(
                    utoipa::openapi::security::ApiKeyValue::new("X-Signature")
                )
            ),
        );

        // Add Idempotency key header
        components.add_security_scheme(
            "idempotency_key",
            utoipa::openapi::security::SecurityScheme::ApiKey(
                utoipa::openapi::security::ApiKey::Header(
                    utoipa::openapi::security::ApiKeyValue::new("Idempotency-Key")
                )
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
