//! OpenAPI documentation for RampOS API

use axum::http::{header, StatusCode};
use axum::response::{Html, IntoResponse};
use axum::Json;
use serde_json::json;
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
    __path_get_case, __path_get_case_stats, __path_get_dashboard, __path_get_user,
    __path_handle_bank_webhook, __path_list_cases, __path_list_users, __path_update_case, get_case,
    get_case_stats, get_dashboard, get_user, handle_bank_webhook, list_cases, list_users,
    update_case,
};
// Admin types with aliases to avoid name conflicts
use crate::handlers::admin::{
    CaseResponse, CaseStats, DashboardStats, IntentStats as AdminIntentStats, ListCasesResponse,
    ListUsersResponse, SeverityStats, UpdateCaseRequest, UserResponse as AdminUserResponse,
    UserStats, VolumeStats,
};

/// RampOS API Documentation
#[derive(OpenApi)]
#[openapi(
    info(
        title = "RampOS API",
        version = "1.0.0",
        description = "BYOR (Bring Your Own Rails) - Crypto/VND Exchange Infrastructure API\n\n## Request Validation\n\nAll request bodies are validated before processing. Validation errors return HTTP 400 with detailed field-level error information in the following format:\n\n```json\n{\n  \"error\": {\n    \"code\": \"VALIDATION_ERROR\",\n    \"message\": \"Validation failed for N fields\",\n    \"details\": {\n      \"fieldName\": [{\"code\": \"length\", \"message\": \"...\"}]\n    }\n  }\n}\n```\n\n## Authentication\n\nAll endpoints require authentication via Bearer token (API key) in the Authorization header.\n\n## Event Catalog\n\nWebhook and event payloads follow the current `v1` catalog contract. Event names are stable public identifiers such as `intent.status.changed` and `risk.review.required`. Current webhook wrappers use the `webhook_event` envelope with top-level `id`, `type`, `created_at`, and event-specific fields nested under `data`.\n\n## Contract-Driven SDKs and CLI\n\nPublic SDKs stay pinned to the OpenAPI contract. Bounded operator surfaces that are not yet promoted into first-class SDK namespaces are exposed through the thin `rampos-cli` preview and the existing admin endpoints, rather than a second client stack.",
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
        (name = "events", description = "Event ingestion from rails providers with stable cataloged event names and `v1` payload semantics"),
        (name = "users", description = "User management and balances"),
        (name = "admin", description = "Administrative endpoints for tenant and user management"),
        (name = "health", description = "Health check endpoints"),
        (name = "account-abstraction", description = "ERC-4337 Account Abstraction endpoints for smart accounts and UserOperations"),
        (name = "chains", description = "Multi-chain operations and cross-chain bridging"),
        (name = "stablecoin", description = "VNST stablecoin mint, burn, reserves, and peg status"),
        (name = "domains", description = "Custom domain management for tenants"),
        (name = "webhooks", description = "Incoming bank webhook processing and outgoing tenant webhook contracts aligned to the `v1` event catalog")
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
    Html(
        r#"<!DOCTYPE html>
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
</html>"#
            .to_string(),
    )
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

        attach_manual_reconciliation_paths(openapi);
        attach_manual_treasury_paths(openapi);
        attach_manual_settlement_paths(openapi);
        attach_manual_passport_paths(openapi);
        attach_manual_kyb_paths(openapi);
        attach_manual_config_bundle_paths(openapi);
    }
}

fn attach_manual_reconciliation_paths(openapi: &mut utoipa::openapi::OpenApi) {
    insert_manual_path(
        openapi,
        "/v1/admin/reconciliation/workbench",
        json!({
            "get": {
                "tags": ["admin"],
                "operationId": "getReconciliationWorkbench",
                "summary": "Load reconciliation workbench",
                "description": "Returns the bounded reconciliation workbench snapshot used by the thin CLI and admin UI.",
                "parameters": [
                    {
                        "name": "scenario",
                        "in": "query",
                        "required": false,
                        "description": "Optional bounded fixture scenario such as `clean`.",
                        "schema": { "type": "string" }
                    }
                ],
                "responses": {
                    "200": {
                        "description": "Reconciliation workbench snapshot",
                        "content": {
                            "application/json": {
                                "schema": { "type": "object" }
                            }
                        }
                    }
                }
            }
        }),
    );

    insert_manual_path(
        openapi,
        "/v1/admin/reconciliation/export",
        json!({
            "get": {
                "tags": ["admin"],
                "operationId": "exportReconciliationWorkbench",
                "summary": "Export reconciliation workbench",
                "description": "Exports the bounded reconciliation queue snapshot as JSON or CSV.",
                "parameters": [
                    {
                        "name": "scenario",
                        "in": "query",
                        "required": false,
                        "description": "Optional bounded fixture scenario such as `clean`.",
                        "schema": { "type": "string" }
                    },
                    {
                        "name": "format",
                        "in": "query",
                        "required": false,
                        "description": "Export format.",
                        "schema": { "type": "string", "enum": ["json", "csv"] }
                    }
                ],
                "responses": {
                    "200": {
                        "description": "Reconciliation export artifact",
                        "content": {
                            "application/json": {
                                "schema": { "type": "object" }
                            },
                            "text/csv": {
                                "schema": { "type": "string" }
                            }
                        }
                    }
                }
            }
        }),
    );

    insert_manual_path(
        openapi,
        "/v1/admin/reconciliation/evidence/{id}",
        json!({
            "get": {
                "tags": ["admin"],
                "operationId": "getReconciliationEvidence",
                "summary": "Load reconciliation evidence pack",
                "description": "Returns the linked evidence pack for one reconciliation discrepancy.",
                "parameters": [
                    {
                        "name": "id",
                        "in": "path",
                        "required": true,
                        "description": "Stable discrepancy identifier.",
                        "schema": { "type": "string" }
                    },
                    {
                        "name": "scenario",
                        "in": "query",
                        "required": false,
                        "description": "Optional bounded fixture scenario such as `clean`.",
                        "schema": { "type": "string" }
                    }
                ],
                "responses": {
                    "200": {
                        "description": "Reconciliation evidence pack",
                        "content": {
                            "application/json": {
                                "schema": { "type": "object" }
                            }
                        }
                    }
                }
            }
        }),
    );

    insert_manual_path(
        openapi,
        "/v1/admin/reconciliation/evidence/{id}/export",
        json!({
            "get": {
                "tags": ["admin"],
                "operationId": "exportReconciliationEvidence",
                "summary": "Export reconciliation evidence pack",
                "description": "Exports one reconciliation evidence pack as JSON.",
                "parameters": [
                    {
                        "name": "id",
                        "in": "path",
                        "required": true,
                        "description": "Stable discrepancy identifier.",
                        "schema": { "type": "string" }
                    },
                    {
                        "name": "scenario",
                        "in": "query",
                        "required": false,
                        "description": "Optional bounded fixture scenario such as `clean`.",
                        "schema": { "type": "string" }
                    }
                ],
                "responses": {
                    "200": {
                        "description": "Reconciliation evidence export artifact",
                        "content": {
                            "application/json": {
                                "schema": { "type": "object" }
                            }
                        }
                    }
                }
            }
        }),
    );
}

fn attach_manual_treasury_paths(openapi: &mut utoipa::openapi::OpenApi) {
    insert_manual_path(
        openapi,
        "/v1/admin/treasury/workbench",
        json!({
            "get": {
                "tags": ["admin"],
                "operationId": "getTreasuryWorkbench",
                "summary": "Load treasury workbench",
                "description": "Returns the bounded recommendation-only treasury snapshot used by the admin UI.",
                "parameters": [
                    {
                        "name": "scenario",
                        "in": "query",
                        "required": false,
                        "description": "Optional bounded fixture scenario such as `stable`.",
                        "schema": { "type": "string" }
                    }
                ],
                "responses": {
                    "200": {
                        "description": "Treasury workbench snapshot",
                        "content": {
                            "application/json": {
                                "schema": { "type": "object" }
                            }
                        }
                    }
                }
            }
        }),
    );

    insert_manual_path(
        openapi,
        "/v1/admin/treasury/export",
        json!({
            "get": {
                "tags": ["admin"],
                "operationId": "exportTreasuryWorkbench",
                "summary": "Export treasury workbench",
                "description": "Exports the bounded treasury recommendation set as JSON or CSV.",
                "parameters": [
                    {
                        "name": "scenario",
                        "in": "query",
                        "required": false,
                        "description": "Optional bounded fixture scenario such as `stable`.",
                        "schema": { "type": "string" }
                    },
                    {
                        "name": "format",
                        "in": "query",
                        "required": false,
                        "description": "Export format.",
                        "schema": { "type": "string", "enum": ["json", "csv"] }
                    }
                ],
                "responses": {
                    "200": {
                        "description": "Treasury export artifact",
                        "content": {
                            "application/json": {
                                "schema": { "type": "object" }
                            },
                            "text/csv": {
                                "schema": { "type": "string" }
                            }
                        }
                    }
                }
            }
        }),
    );
}

fn attach_manual_settlement_paths(openapi: &mut utoipa::openapi::OpenApi) {
    insert_manual_path(
        openapi,
        "/v1/admin/settlement/workbench",
        json!({
            "get": {
                "tags": ["admin"],
                "operationId": "getSettlementWorkbench",
                "summary": "Load bilateral settlement workbench",
                "description": "Returns the bounded bilateral, approval-gated settlement proposal snapshot used by the admin UI.",
                "parameters": [
                    {
                        "name": "scenario",
                        "in": "query",
                        "required": false,
                        "description": "Optional bounded fixture scenario such as `clean` or `approval_pending`.",
                        "schema": { "type": "string" }
                    }
                ],
                "responses": {
                    "200": {
                        "description": "Settlement workbench snapshot",
                        "content": {
                            "application/json": { "schema": { "type": "object" } }
                        }
                    }
                }
            }
        }),
    );

    insert_manual_path(
        openapi,
        "/v1/admin/settlement/export",
        json!({
            "get": {
                "tags": ["admin"],
                "operationId": "exportSettlementWorkbench",
                "summary": "Export bilateral settlement workbench",
                "description": "Exports the bounded bilateral settlement proposal queue as JSON or CSV.",
                "parameters": [
                    {
                        "name": "scenario",
                        "in": "query",
                        "required": false,
                        "description": "Optional bounded fixture scenario such as `clean` or `approval_pending`.",
                        "schema": { "type": "string" }
                    },
                    {
                        "name": "format",
                        "in": "query",
                        "required": false,
                        "description": "Export format.",
                        "schema": { "type": "string", "enum": ["json", "csv"] }
                    }
                ],
                "responses": {
                    "200": {
                        "description": "Settlement export artifact",
                        "content": {
                            "application/json": { "schema": { "type": "object" } },
                            "text/csv": { "schema": { "type": "string" } }
                        }
                    }
                }
            }
        }),
    );
}

fn attach_manual_passport_paths(openapi: &mut utoipa::openapi::OpenApi) {
    insert_manual_path(
        openapi,
        "/v1/admin/passport/queue",
        json!({
            "get": {
                "tags": ["admin"],
                "operationId": "listPassportQueue",
                "summary": "List passport vault queue",
                "description": "Returns the bounded shared-vault passport queue used for consent and review workflows.",
                "parameters": [
                    {
                        "name": "scenario",
                        "in": "query",
                        "required": false,
                        "description": "Optional bounded fixture scenario such as `revoked`.",
                        "schema": { "type": "string" }
                    }
                ],
                "responses": {
                    "200": {
                        "description": "Passport queue snapshot",
                        "content": {
                            "application/json": { "schema": { "type": "object" } }
                        }
                    }
                }
            }
        }),
    );

    insert_manual_path(
        openapi,
        "/v1/admin/passport/packages/{id}",
        json!({
            "get": {
                "tags": ["admin"],
                "operationId": "getPassportPackage",
                "summary": "Load passport package detail",
                "description": "Returns one bounded passport package detail including consent, freshness, and acceptance status.",
                "parameters": [
                    {
                        "name": "id",
                        "in": "path",
                        "required": true,
                        "description": "Passport package identifier.",
                        "schema": { "type": "string" }
                    },
                    {
                        "name": "scenario",
                        "in": "query",
                        "required": false,
                        "description": "Optional bounded fixture scenario such as `revoked`.",
                        "schema": { "type": "string" }
                    }
                ],
                "responses": {
                    "200": {
                        "description": "Passport package detail",
                        "content": {
                            "application/json": { "schema": { "type": "object" } }
                        }
                    }
                }
            }
        }),
    );
}

fn attach_manual_kyb_paths(openapi: &mut utoipa::openapi::OpenApi) {
    insert_manual_path(
        openapi,
        "/v1/admin/kyb/reviews",
        json!({
            "get": {
                "tags": ["admin"],
                "operationId": "listKybReviews",
                "summary": "List KYB ownership reviews",
                "description": "Returns the bounded KYB review queue built from relational ownership edges.",
                "parameters": [
                    {
                        "name": "scenario",
                        "in": "query",
                        "required": false,
                        "description": "Optional bounded fixture scenario such as `clean`.",
                        "schema": { "type": "string" }
                    }
                ],
                "responses": {
                    "200": {
                        "description": "KYB review queue",
                        "content": {
                            "application/json": { "schema": { "type": "object" } }
                        }
                    }
                }
            }
        }),
    );

    insert_manual_path(
        openapi,
        "/v1/admin/kyb/graph/{id}",
        json!({
            "get": {
                "tags": ["admin"],
                "operationId": "getKybGraph",
                "summary": "Load KYB graph review detail",
                "description": "Returns one relational ownership graph review item for a business entity.",
                "parameters": [
                    {
                        "name": "id",
                        "in": "path",
                        "required": true,
                        "description": "Business entity identifier.",
                        "schema": { "type": "string" }
                    },
                    {
                        "name": "scenario",
                        "in": "query",
                        "required": false,
                        "description": "Optional bounded fixture scenario such as `clean`.",
                        "schema": { "type": "string" }
                    }
                ],
                "responses": {
                    "200": {
                        "description": "KYB graph review detail",
                        "content": {
                            "application/json": { "schema": { "type": "object" } }
                        }
                    }
                }
            }
        }),
    );
}

fn attach_manual_config_bundle_paths(openapi: &mut utoipa::openapi::OpenApi) {
    insert_manual_path(
        openapi,
        "/v1/admin/config-bundles/export",
        json!({
            "get": {
                "tags": ["admin"],
                "operationId": "exportConfigBundle",
                "summary": "Export whitelisted config bundle",
                "description": "Returns the bounded config bundle export used by admin settings tooling. When governance data is available it is sourced from persisted registry-backed records; otherwise an explicit fallback artifact is returned.",
                "responses": {
                    "200": {
                        "description": "Config bundle export artifact",
                        "content": {
                            "application/json": { "schema": { "type": "object" } }
                        }
                    }
                }
            }
        }),
    );

    insert_manual_path(
        openapi,
        "/v1/admin/extensions",
        json!({
            "get": {
                "tags": ["admin"],
                "operationId": "listWhitelistedExtensionActions",
                "summary": "List whitelisted extension actions",
                "description": "Returns the bounded registry of allowed extension actions. When governance data is available it is sourced from persisted registry-backed records; otherwise an explicit fallback action registry is returned. No arbitrary extension execution is exposed.",
                "responses": {
                    "200": {
                        "description": "Whitelisted extension action registry",
                        "content": {
                            "application/json": { "schema": { "type": "object" } }
                        }
                    }
                }
            }
        }),
    );
}

fn insert_manual_path(
    openapi: &mut utoipa::openapi::OpenApi,
    path: &str,
    value: serde_json::Value,
) {
    let paths = &mut openapi.paths;
    let path_item: utoipa::openapi::path::PathItem =
        serde_json::from_value(value).expect("manual OpenAPI path item should deserialize");
    paths.paths.insert(path.to_string(), path_item);
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
