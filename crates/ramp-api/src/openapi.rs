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
#[allow(unused_imports)]
use crate::handlers::portal::venue_cashout::{
    confirm_hyperliquid_wallet_receipt, prepare_hyperliquid_cashout,
    HyperliquidCashoutPrepareRequest, HyperliquidCashoutPrepareResponse,
    HyperliquidCashoutReceiptRequest, HyperliquidCashoutReceiptResponse,
};
#[allow(unused_imports)]
use crate::handlers::portal::venue_funding::{
    connect_venue_funding, get_venue_funding_eligibility, get_venue_funding_status,
    list_venue_funding_venues, prepare_venue_funding, submit_venue_funding,
    VenueFundingAccountSummaryResponse, VenueFundingChecklistItemResponse,
    VenueFundingConnectionRequest, VenueFundingConnectionResponse,
    VenueFundingConnectionSummaryResponse, VenueFundingEligibilityReasonResponse,
    VenueFundingEligibilityResponse, VenueFundingPrepareRequest, VenueFundingPrepareResponse,
    VenueFundingSourceOfFundsResponse, VenueFundingStatusResponse, VenueFundingSubmitRequest,
    VenueFundingSubmitResponse, VenueListResponse, VenueSummaryResponse,
};
use crate::handlers::stablecoin::*;
use crate::handlers::trade::*;
// Import admin and bank_webhook handlers + utoipa __path_xxx generated types via re-export
#[allow(unused_imports)]
use crate::handlers::portal::venue_cashout::{
    __path_confirm_hyperliquid_wallet_receipt, __path_prepare_hyperliquid_cashout,
};
#[allow(unused_imports)]
use crate::handlers::portal::venue_funding::{
    __path_connect_venue_funding, __path_get_venue_funding_eligibility,
    __path_get_venue_funding_status, __path_list_venue_funding_venues,
    __path_prepare_venue_funding, __path_submit_venue_funding,
};
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
        (name = "portal", description = "Portal end-user flows including explicit venue listing, connection, eligibility, prepare, submit, and status"),
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
        // Portal
        prepare_hyperliquid_cashout,
        confirm_hyperliquid_wallet_receipt,
        list_venue_funding_venues,
        connect_venue_funding,
        get_venue_funding_eligibility,
        prepare_venue_funding,
        submit_venue_funding,
        get_venue_funding_status,
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
            HyperliquidCashoutPrepareRequest,
            HyperliquidCashoutPrepareResponse,
            HyperliquidCashoutReceiptRequest,
            HyperliquidCashoutReceiptResponse,
            VenueListResponse,
            VenueSummaryResponse,
            VenueFundingConnectionRequest,
            VenueFundingConnectionResponse,
            VenueFundingConnectionSummaryResponse,
            VenueFundingAccountSummaryResponse,
            VenueFundingPrepareRequest,
            VenueFundingPrepareResponse,
            VenueFundingStatusResponse,
            VenueFundingChecklistItemResponse,
            VenueFundingEligibilityResponse,
            VenueFundingEligibilityReasonResponse,
            VenueFundingSourceOfFundsResponse,
            VenueFundingSubmitRequest,
            VenueFundingSubmitResponse,
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
        attach_manual_liquidity_paths(openapi);
        attach_manual_audit_paths(openapi);
        attach_manual_passport_paths(openapi);
        attach_manual_kyb_paths(openapi);
        attach_manual_config_bundle_paths(openapi);
        attach_manual_commercial_readiness_paths(openapi);
        attach_manual_commercialization_pack_paths(openapi);
        attach_manual_provider_routing_paths(openapi);
        attach_manual_partner_registry_paths(openapi);
        attach_manual_execution_explainability_paths(openapi);
        attach_manual_venue_trust_paths(openapi);
    }
}

fn attach_manual_venue_trust_paths(openapi: &mut utoipa::openapi::OpenApi) {
    insert_manual_path(
        openapi,
        "/v1/admin/venue-trust/reports/{subject_type}/{subject_id}",
        json!({
            "get": {
                "tags": ["admin"],
                "operationId": "getVenueTrustReportSnapshot",
                "summary": "Get venue trust report snapshot",
                "description": "Returns a read-only trust report snapshot for the supplied subject, covering cash-in and cash-out transfer summaries plus evidence references.",
                "parameters": [
                    {
                        "name": "subject_type",
                        "in": "path",
                        "required": true,
                        "schema": { "type": "string" }
                    },
                    {
                        "name": "subject_id",
                        "in": "path",
                        "required": true,
                        "schema": { "type": "string" }
                    }
                ],
                "responses": {
                    "200": {
                        "description": "Venue trust report snapshot",
                        "content": {
                            "application/json": {
                                "schema": {
                                    "type": "object",
                                    "required": [
                                        "source",
                                        "subjectType",
                                        "subjectId",
                                        "connectionIds",
                                        "accountIds",
                                        "walletAttestationIds",
                                        "beneficiaryProfileIds",
                                        "sourceOfFundsPackageIds",
                                        "cashIn",
                                        "cashOut",
                                        "evidenceReferences"
                                    ],
                                    "properties": {
                                        "source": { "type": "string" },
                                        "subjectType": { "type": "string" },
                                        "subjectId": { "type": "string" },
                                        "connectionIds": { "type": "array", "items": { "type": "string" } },
                                        "accountIds": { "type": "array", "items": { "type": "string" } },
                                        "walletAttestationIds": { "type": "array", "items": { "type": "string" } },
                                        "beneficiaryProfileIds": { "type": "array", "items": { "type": "string" } },
                                        "sourceOfFundsPackageIds": { "type": "array", "items": { "type": "string" } },
                                        "cashIn": { "type": "object" },
                                        "cashOut": { "type": "object" },
                                        "evidenceReferences": { "type": "array", "items": { "type": "object" } }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }),
    );

    insert_manual_path(
        openapi,
        "/v1/admin/venue-trust/reports/{subject_type}/{subject_id}/export",
        json!({
            "get": {
                "tags": ["admin"],
                "operationId": "exportVenueTrustReport",
                "summary": "Export venue trust evidence report",
                "description": "Exports the venue trust evidence report as JSON for the supplied subject.",
                "parameters": [
                    {
                        "name": "subject_type",
                        "in": "path",
                        "required": true,
                        "schema": { "type": "string" }
                    },
                    {
                        "name": "subject_id",
                        "in": "path",
                        "required": true,
                        "schema": { "type": "string" }
                    }
                ],
                "responses": {
                    "200": {
                        "description": "Venue trust evidence export",
                        "content": {
                            "application/json": {
                                "schema": {
                                    "type": "object",
                                    "required": ["subjectType", "subjectId", "cashIn", "cashOut", "evidenceReferences"],
                                    "properties": {
                                        "subjectType": { "type": "string" },
                                        "subjectId": { "type": "string" },
                                        "cashIn": { "type": "object" },
                                        "cashOut": { "type": "object" },
                                        "evidenceReferences": { "type": "array", "items": { "type": "object" } }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }),
    );

    insert_manual_path(
        openapi,
        "/v1/admin/venue-trust/connectors/cex/{connector_key}/readiness/{subject_type}/{subject_id}",
        json!({
            "get": {
                "tags": ["admin"],
                "operationId": "getCexConnectorReadinessSnapshot",
                "summary": "Get generic CEX connector readiness snapshot",
                "description": "Returns a read-only operator-first readiness snapshot for a generic CEX connector, including API key mode, subaccount mode, withdrawal allowlist status, custody boundary mode, and requirement satisfaction.",
                "parameters": [
                    {
                        "name": "connector_key",
                        "in": "path",
                        "required": true,
                        "schema": { "type": "string" }
                    },
                    {
                        "name": "subject_type",
                        "in": "path",
                        "required": true,
                        "schema": { "type": "string" }
                    },
                    {
                        "name": "subject_id",
                        "in": "path",
                        "required": true,
                        "schema": { "type": "string" }
                    }
                ],
                "responses": {
                    "200": {
                        "description": "Generic CEX connector readiness snapshot",
                        "content": {
                            "application/json": {
                                "schema": {
                                    "type": "object",
                                    "required": [
                                        "connectorKey",
                                        "status",
                                        "connectionId",
                                        "accountId",
                                        "apiKeyMode",
                                        "subaccountMode",
                                        "withdrawalAllowlistStatus",
                                        "custodyBoundaryMode",
                                        "requirements"
                                    ],
                                    "properties": {
                                        "connectorKey": { "type": "string" },
                                        "status": { "type": "string" },
                                        "connectionId": { "type": "string", "nullable": true },
                                        "accountId": { "type": "string", "nullable": true },
                                        "apiKeyMode": { "type": "string", "nullable": true },
                                        "subaccountMode": { "type": "string", "nullable": true },
                                        "withdrawalAllowlistStatus": { "type": "string", "nullable": true },
                                        "custodyBoundaryMode": { "type": "string", "nullable": true },
                                        "requirements": {
                                            "type": "array",
                                            "items": {
                                                "type": "object",
                                                "required": ["code", "satisfied", "source", "message"],
                                                "properties": {
                                                    "code": { "type": "string" },
                                                    "satisfied": { "type": "boolean" },
                                                    "source": { "type": "string" },
                                                    "message": { "type": "string" }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }),
    );

    insert_manual_path(
        openapi,
        "/v1/admin/venue-trust/connectors/lighter/readiness/{subject_type}/{subject_id}",
        json!({
            "get": {
                "tags": ["admin"],
                "operationId": "getLighterConnectorReadinessSnapshot",
                "summary": "Get Lighter connector readiness snapshot",
                "description": "Returns a read-only operator-first readiness snapshot for the Lighter connector, including public pool mode, proof anchor mode, operator linkage, institutional evidence status, and requirement satisfaction.",
                "parameters": [
                    {
                        "name": "subject_type",
                        "in": "path",
                        "required": true,
                        "schema": { "type": "string" }
                    },
                    {
                        "name": "subject_id",
                        "in": "path",
                        "required": true,
                        "schema": { "type": "string" }
                    }
                ],
                "responses": {
                    "200": {
                        "description": "Lighter connector readiness snapshot",
                        "content": {
                            "application/json": {
                                "schema": {
                                    "type": "object",
                                    "required": [
                                        "connectorKey",
                                        "status",
                                        "connectionId",
                                        "accountId",
                                        "publicPoolMode",
                                        "proofAnchorMode",
                                        "operatorLinkageStatus",
                                        "institutionalEvidenceStatus",
                                        "requirements"
                                    ],
                                    "properties": {
                                        "connectorKey": { "type": "string" },
                                        "status": { "type": "string" },
                                        "connectionId": { "type": "string", "nullable": true },
                                        "accountId": { "type": "string", "nullable": true },
                                        "publicPoolMode": { "type": "string", "nullable": true },
                                        "proofAnchorMode": { "type": "string", "nullable": true },
                                        "operatorLinkageStatus": { "type": "string", "nullable": true },
                                        "institutionalEvidenceStatus": { "type": "string", "nullable": true },
                                        "requirements": {
                                            "type": "array",
                                            "items": {
                                                "type": "object",
                                                "required": ["code", "satisfied", "source", "message"],
                                                "properties": {
                                                    "code": { "type": "string" },
                                                    "satisfied": { "type": "boolean" },
                                                    "source": { "type": "string" },
                                                    "message": { "type": "string" }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }),
    );

    insert_manual_path(
        openapi,
        "/v1/admin/venue-trust/delegation-prerequisites/{subject_type}/{subject_id}/delegates/{delegate}",
        json!({
            "get": {
                "tags": ["admin"],
                "operationId": "getDelegationPrerequisitesSnapshot",
                "summary": "Get delegation prerequisites snapshot",
                "description": "Returns a read-only admin snapshot of effective delegation prerequisites and execution-envelope evaluation for a subject/delegate pair.",
                "parameters": [
                    {
                        "name": "subject_type",
                        "in": "path",
                        "required": true,
                        "schema": { "type": "string" }
                    },
                    {
                        "name": "subject_id",
                        "in": "path",
                        "required": true,
                        "schema": { "type": "string" }
                    },
                    {
                        "name": "delegate",
                        "in": "path",
                        "required": true,
                        "schema": { "type": "string" }
                    },
                    {
                        "name": "tool_surface",
                        "in": "query",
                        "required": true,
                        "schema": { "type": "string" }
                    },
                    {
                        "name": "approval_boundary",
                        "in": "query",
                        "required": true,
                        "schema": { "type": "string" }
                    }
                ],
                "responses": {
                    "200": {
                        "description": "Delegation prerequisites snapshot",
                        "content": {
                            "application/json": {
                                "schema": {
                                    "type": "object",
                                    "required": [
                                        "subjectType",
                                        "subjectId",
                                        "tenantId",
                                        "delegate",
                                        "prerequisites",
                                        "evaluation"
                                    ],
                                    "properties": {
                                        "subjectType": { "type": "string" },
                                        "subjectId": { "type": "string" },
                                        "tenantId": { "type": "string" },
                                        "delegate": { "type": "string" },
                                        "prerequisites": {
                                            "type": "object",
                                            "required": ["allowedToolSurfaces", "approvalBoundary"],
                                            "properties": {
                                                "allowedToolSurfaces": {
                                                    "type": "array",
                                                    "items": { "type": "string" }
                                                },
                                                "approvalBoundary": { "type": "string" }
                                            }
                                        },
                                        "evaluation": {
                                            "type": "object",
                                            "required": [
                                                "allowed",
                                                "requestedToolSurface",
                                                "requestedApprovalBoundary",
                                                "denialReasons"
                                            ],
                                            "properties": {
                                                "allowed": { "type": "boolean" },
                                                "requestedToolSurface": { "type": "string" },
                                                "requestedApprovalBoundary": { "type": "string" },
                                                "denialReasons": {
                                                    "type": "array",
                                                    "items": { "type": "string" }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }),
    );

    insert_manual_path(
        openapi,
        "/v1/admin/venue-trust/subjects/{subject_type}/{subject_id}",
        json!({
            "get": {
                "tags": ["admin"],
                "operationId": "getVenueTrustSubjectSnapshot",
                "summary": "Get venue trust subject snapshot",
                "description": "Returns the venue trust subject snapshot for the provided subject type and subject id, including connections, attestations, transfers, and source-of-funds packages.",
                "parameters": [
                    {
                        "name": "subject_type",
                        "in": "path",
                        "required": true,
                        "schema": { "type": "string" }
                    },
                    {
                        "name": "subject_id",
                        "in": "path",
                        "required": true,
                        "schema": { "type": "string" }
                    }
                ],
                "responses": {
                    "200": {
                        "description": "Venue trust subject snapshot",
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
        "/v1/admin/venue-trust/connections/{id}/review",
        json!({
            "post": {
                "tags": ["admin"],
                "operationId": "reviewVenueTrustConnection",
                "summary": "Review venue trust connection",
                "description": "Transitions a venue connection into an operator-reviewed status and records review metadata such as review reason, optional failure reason, and provenance.",
                "parameters": [
                    {
                        "name": "id",
                        "in": "path",
                        "required": true,
                        "schema": { "type": "string" }
                    }
                ],
                "requestBody": {
                    "required": true,
                    "content": {
                        "application/json": {
                            "schema": {
                                "type": "object",
                                "required": ["status", "reviewReason"],
                                "properties": {
                                    "status": { "type": "string" },
                                    "reviewReason": { "type": "string" },
                                    "failureReason": { "type": "string" },
                                    "provenance": { "type": "object" }
                                }
                            }
                        }
                    }
                },
                "responses": {
                    "200": {
                        "description": "Reviewed venue connection",
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
        "/v1/admin/venue-trust/transfers/{id}",
        json!({
            "get": {
                "tags": ["admin"],
                "operationId": "getVenueTrustTransferDetail",
                "summary": "Get venue trust transfer detail",
                "description": "Returns a venue transfer detail view with the transfer, connection, wallet attestation, and source-of-funds packages for the supplied transfer id.",
                "parameters": [
                    {
                        "name": "id",
                        "in": "path",
                        "required": true,
                        "schema": { "type": "string" }
                    }
                ],
                "responses": {
                    "200": {
                        "description": "Venue trust transfer detail",
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
        "/v1/admin/venue-trust/transfers/{id}/review",
        json!({
            "post": {
                "tags": ["admin"],
                "operationId": "reviewVenueTrustTransfer",
                "summary": "Review venue trust transfer",
                "description": "Transitions a venue transfer into an operator-reviewed status and records review metadata such as review reason, optional failure reason, and provenance.",
                "parameters": [
                    {
                        "name": "id",
                        "in": "path",
                        "required": true,
                        "schema": { "type": "string" }
                    }
                ],
                "requestBody": {
                    "required": true,
                    "content": {
                        "application/json": {
                            "schema": {
                                "type": "object",
                                "required": ["status", "reviewReason"],
                                "properties": {
                                    "status": { "type": "string" },
                                    "reviewReason": { "type": "string" },
                                    "failureReason": { "type": "string" },
                                    "provenance": { "type": "object" }
                                }
                            }
                        }
                    }
                },
                "responses": {
                    "200": {
                        "description": "Reviewed venue transfer",
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
        "/v1/admin/venue-trust/source-of-funds-packages/{id}/review",
        json!({
            "post": {
                "tags": ["admin"],
                "operationId": "reviewVenueTrustSourceOfFundsPackage",
                "summary": "Review venue trust source-of-funds package",
                "description": "Transitions a source-of-funds package into an operator-reviewed status and records review metadata such as review reason, optional failure reason, and provenance.",
                "parameters": [
                    {
                        "name": "id",
                        "in": "path",
                        "required": true,
                        "schema": { "type": "string" }
                    }
                ],
                "requestBody": {
                    "required": true,
                    "content": {
                        "application/json": {
                            "schema": {
                                "type": "object",
                                "required": ["status", "reviewReason"],
                                "properties": {
                                    "status": { "type": "string" },
                                    "reviewReason": { "type": "string" },
                                    "failureReason": { "type": "string" },
                                    "provenance": { "type": "object" }
                                }
                            }
                        }
                    }
                },
                "responses": {
                    "200": {
                        "description": "Reviewed source-of-funds package",
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
        "/v1/admin/venue-trust/wallet-attestations/{id}/review",
        json!({
            "post": {
                "tags": ["admin"],
                "operationId": "reviewVenueTrustWalletAttestation",
                "summary": "Review venue trust wallet attestation",
                "description": "Transitions a wallet attestation into an operator-reviewed status and records review metadata such as review reason and optional failure reason.",
                "parameters": [
                    {
                        "name": "id",
                        "in": "path",
                        "required": true,
                        "schema": { "type": "string", "format": "uuid" }
                    }
                ],
                "requestBody": {
                    "required": true,
                    "content": {
                        "application/json": {
                            "schema": {
                                "type": "object",
                                "required": ["status", "reviewReason"],
                                "properties": {
                                    "status": { "type": "string" },
                                    "reviewReason": { "type": "string" },
                                    "failureReason": { "type": "string" },
                                    "provenance": { "type": "object" }
                                }
                            }
                        }
                    }
                },
                "responses": {
                    "200": {
                        "description": "Reviewed wallet attestation",
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

fn attach_manual_execution_explainability_paths(openapi: &mut utoipa::openapi::OpenApi) {
    insert_manual_path(
        openapi,
        "/v1/admin/execution-explainability/explain",
        json!({
            "post": {
                "tags": ["admin"],
                "operationId": "explainExecutionRoute",
                "summary": "Explain a single execution route",
                "description": "Scores a route candidate using runtime LP, treasury, corridor, and compliance signals. Returns eligibility, factors, and provenance.",
                "requestBody": {
                    "required": true,
                    "content": {
                        "application/json": {
                            "schema": {
                                "type": "object",
                                "required": [
                                    "routeId",
                                    "lpId",
                                    "direction",
                                    "asset",
                                    "treasuryStressActive",
                                    "corridorPolicyEligible",
                                    "complianceEligible",
                                    "quotedRate",
                                    "quotedVndAmount"
                                ],
                                "properties": {
                                    "routeId": { "type": "string" },
                                    "corridorCode": { "type": "string" },
                                    "lpId": { "type": "string" },
                                    "direction": { "type": "string" },
                                    "asset": { "type": "string" },
                                    "providerFamily": { "type": "string" },
                                    "lpReliabilityScore": { "type": "string" },
                                    "lpFillRate": { "type": "string" },
                                    "lpDisputeRate": { "type": "string" },
                                    "treasuryFloatAvailable": { "type": "string" },
                                    "treasuryStressActive": { "type": "boolean" },
                                    "corridorPolicyEligible": { "type": "boolean" },
                                    "complianceEligible": { "type": "boolean" },
                                    "quotedRate": { "type": "string" },
                                    "quotedVndAmount": { "type": "string" }
                                }
                            }
                        }
                    }
                },
                "responses": {
                    "200": {
                        "description": "Explainability result",
                        "content": {
                            "application/json": {
                                "schema": {
                                    "type": "object",
                                    "required": ["routeId", "eligible", "factors", "provenance"],
                                    "properties": {
                                        "routeId": { "type": "string" },
                                        "eligible": { "type": "boolean" },
                                        "factors": {
                                            "type": "array",
                                            "items": {
                                                "type": "object",
                                                "required": [
                                                    "factorName",
                                                    "weight",
                                                    "rawValue",
                                                    "contribution",
                                                    "explanation"
                                                ],
                                                "properties": {
                                                    "factorName": { "type": "string" },
                                                    "weight": { "type": "string" },
                                                    "rawValue": { "type": "string" },
                                                    "contribution": { "type": "string" },
                                                    "explanation": { "type": "string" }
                                                }
                                            }
                                        },
                                        "provenance": { "type": "object" }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }),
    );

    insert_manual_path(
        openapi,
        "/v1/admin/execution-explainability/compare",
        json!({
            "post": {
                "tags": ["admin"],
                "operationId": "compareExecutionRoutes",
                "summary": "Compare execution routes",
                "description": "Ranks multiple routes using runtime signals and exposes a provenance-rich snapshot.",
                "requestBody": {
                    "required": true,
                    "content": {
                        "application/json": {
                            "schema": {
                                "type": "object",
                                "required": ["direction", "inputs"],
                                "properties": {
                                    "corridorCode": { "type": "string" },
                                    "direction": { "type": "string" },
                                    "inputs": {
                                        "type": "array",
                                        "items": {
                                            "type": "object",
                                            "required": [
                                                "routeId",
                                                "lpId",
                                                "direction",
                                                "asset",
                                                "treasuryStressActive",
                                                "corridorPolicyEligible",
                                                "complianceEligible",
                                                "quotedRate",
                                                "quotedVndAmount"
                                            ],
                                            "properties": {
                                                "routeId": { "type": "string" },
                                                "corridorCode": { "type": "string" },
                                                "lpId": { "type": "string" },
                                                "direction": { "type": "string" },
                                                "asset": { "type": "string" },
                                                "providerFamily": { "type": "string" },
                                                "lpReliabilityScore": { "type": "string" },
                                                "lpFillRate": { "type": "string" },
                                                "lpDisputeRate": { "type": "string" },
                                                "treasuryFloatAvailable": { "type": "string" },
                                                "treasuryStressActive": { "type": "boolean" },
                                                "corridorPolicyEligible": { "type": "boolean" },
                                                "complianceEligible": { "type": "boolean" },
                                                "quotedRate": { "type": "string" },
                                                "quotedVndAmount": { "type": "string" }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                },
                "responses": {
                    "200": {
                        "description": "Comparison snapshot",
                        "content": {
                            "application/json": {
                                "schema": {
                                    "type": "object",
                                    "required": ["candidates", "winningRouteId", "provenance"],
                                    "properties": {
                                        "candidates": {
                                            "type": "array",
                                            "items": {
                                                "type": "object",
                                                "required": [
                                                    "routeId",
                                                    "lpId",
                                                    "direction",
                                                    "compositeScore",
                                                    "factors",
                                                    "eligible",
                                                    "ineligibilityReasons",
                                                    "summary",
                                                    "provenance"
                                                ],
                                                "properties": {
                                                    "routeId": { "type": "string" },
                                                    "lpId": { "type": "string" },
                                                    "direction": { "type": "string" },
                                                    "compositeScore": { "type": "string" },
                                                    "factors": {
                                                        "type": "array",
                                                        "items": {
                                                            "type": "object",
                                                            "required": [
                                                                "factorName",
                                                                "weight",
                                                                "rawValue",
                                                                "contribution",
                                                                "explanation"
                                                            ],
                                                            "properties": {
                                                                "factorName": { "type": "string" },
                                                                "weight": { "type": "string" },
                                                                "rawValue": { "type": "string" },
                                                                "contribution": { "type": "string" },
                                                                "explanation": { "type": "string" }
                                                            }
                                                        }
                                                    },
                                                    "eligible": { "type": "boolean" },
                                                    "ineligibilityReasons": {
                                                        "type": "array",
                                                        "items": { "type": "string" }
                                                    },
                                                    "summary": { "type": "string" },
                                                    "provenance": { "type": "object" }
                                                }
                                            }
                                        },
                                        "winningRouteId": { "type": "string", "nullable": true },
                                        "provenance": { "type": "object" }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }),
    );
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
                "description": "Returns the bounded reconciliation workbench snapshot used by the thin CLI and admin UI. Runtime inputs are surfaced as evidence-backed provenance, while sample fallback remains explicitly bounded fallback with warning context; gated actions remain approval-linked and audit-linked on the current admin seam.",
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
                                "schema": {
                                    "type": "object",
                                    "required": ["snapshot", "actionMode", "exportFormats", "incidentLinkHint", "gatedActions"],
                                    "properties": {
                                        "snapshot": {
                                            "type": "object",
                                            "required": ["generatedAt", "report", "queue", "provenance"],
                                            "properties": {
                                                "generatedAt": { "type": "string", "format": "date-time" },
                                                "report": { "type": "object" },
                                                "queue": { "type": "array", "items": { "type": "object" } },
                                                "provenance": {
                                                    "type": "object",
                                                    "required": ["sourceKind", "sourceClass", "settlementCount", "onChainTxCount"],
                                                    "properties": {
                                                        "sourceKind": { "type": "string" },
                                                        "sourceClass": { "type": "string" },
                                                        "settlementCount": { "type": "integer", "minimum": 0 },
                                                        "onChainTxCount": { "type": "integer", "minimum": 0 },
                                                        "freshnessWarning": { "type": "string", "nullable": true }
                                                    }
                                                }
                                            }
                                        },
                                        "actionMode": { "type": "string" },
                                        "exportFormats": { "type": "array", "items": { "type": "string" } },
                                        "incidentLinkHint": { "type": "string" },
                                        "gatedActions": { "type": "array", "items": { "type": "object" } }
                                    }
                                }
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
                "description": "Exports the bounded reconciliation queue snapshot as JSON or CSV, preserving provenance classification so evidence-backed runtime inputs remain distinct from bounded fallback exports.",
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
                "description": "Returns the linked evidence pack for one reconciliation discrepancy, including additive evidence-source and lineage context on the current admin seam. When the surrounding workbench originated from bounded fallback instead of evidence-backed runtime inputs, operators should treat this evidence context as bounded fallback rather than production truth.",
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
                "description": "Exports one reconciliation evidence pack as JSON, including additive evidence-source and lineage context, while preserving the distinction between evidence-backed runtime context and bounded fallback operator review context.",
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
                "description": "Returns the bounded recommendation-only treasury snapshot used by the admin UI, including additive safeguarding, client-money, and reserve overlays tied to evidence context.",
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
                "description": "Exports the bounded treasury recommendation set as JSON or CSV, including additive safeguarding, client-money, and reserve overlays in the JSON artifact.",
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

fn attach_manual_liquidity_paths(openapi: &mut utoipa::openapi::OpenApi) {
    insert_manual_path(
        openapi,
        "/v1/admin/liquidity/explain",
        json!({
            "get": {
                "tags": ["admin"],
                "operationId": "getLiquidityRouteExplainability",
                "summary": "Load route explainability",
                "description": "Returns a bounded route explainability artifact on the current admin liquidity surface, including winning-lane and rejected-lane rationale with treasury, partner, corridor, and compliance constraints.",
                "parameters": [
                    {
                        "name": "direction",
                        "in": "query",
                        "required": false,
                        "description": "Liquidity direction such as OFFRAMP or ONRAMP.",
                        "schema": { "type": "string" }
                    }
                ],
                "responses": {
                    "200": {
                        "description": "Liquidity route explainability artifact",
                        "content": {
                            "application/json": { "schema": { "type": "object" } }
                        }
                    }
                }
            }
        }),
    );
}

fn attach_manual_audit_paths(openapi: &mut utoipa::openapi::OpenApi) {
    insert_manual_path(
        openapi,
        "/v1/admin/audit/break-glass/export",
        json!({
            "get": {
                "tags": ["admin"],
                "operationId": "exportBreakGlassAuditLog",
                "summary": "Export break-glass audit bundle",
                "description": "Exports an immutable break-glass audit bundle on the current admin audit surface, including actor attribution, emergency scope, evidence reference, rollback context, compatibility impact, and the underlying audit export.",
                "parameters": [
                    { "name": "emergencyScope", "in": "query", "required": true, "schema": { "type": "string" } },
                    { "name": "evidenceRef", "in": "query", "required": true, "schema": { "type": "string" } },
                    { "name": "rollbackContext", "in": "query", "required": true, "schema": { "type": "string" } },
                    { "name": "compatibilityImpact", "in": "query", "required": true, "schema": { "type": "string" } }
                ],
                "responses": {
                    "200": {
                        "description": "Immutable break-glass audit export",
                        "content": {
                            "application/json": { "schema": { "type": "object" } }
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

    insert_manual_path(
        openapi,
        "/v1/admin/kyb/evidence",
        json!({
            "get": {
                "tags": ["admin"],
                "operationId": "listKybEvidencePackages",
                "summary": "List KYB evidence packages",
                "description": "Returns persisted institutional KYB evidence packages with provider-routing, corridor, review, and export context on the existing admin KYB surface.",
                "parameters": [
                    {
                        "name": "institutionEntityId",
                        "in": "query",
                        "required": false,
                        "description": "Optional institution entity identifier filter.",
                        "schema": { "type": "string" }
                    },
                    {
                        "name": "corridorCode",
                        "in": "query",
                        "required": false,
                        "description": "Optional corridor code filter.",
                        "schema": { "type": "string" }
                    },
                    {
                        "name": "reviewStatus",
                        "in": "query",
                        "required": false,
                        "description": "Optional review status filter.",
                        "schema": { "type": "string" }
                    }
                ],
                "responses": {
                    "200": {
                        "description": "KYB evidence package list",
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
        "/v1/admin/kyb/evidence/{id}",
        json!({
            "get": {
                "tags": ["admin"],
                "operationId": "getKybEvidencePackage",
                "summary": "Load KYB evidence package detail",
                "description": "Returns one persisted KYB evidence package including evidence-source and UBO-link context.",
                "parameters": [
                    {
                        "name": "id",
                        "in": "path",
                        "required": true,
                        "description": "Evidence package identifier.",
                        "schema": { "type": "string" }
                    }
                ],
                "responses": {
                    "200": {
                        "description": "KYB evidence package detail",
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
        "/v1/admin/kyb/evidence/{id}/export",
        json!({
            "get": {
                "tags": ["admin"],
                "operationId": "exportKybEvidencePackage",
                "summary": "Export KYB evidence package",
                "description": "Exports one KYB evidence package as JSON from the existing admin KYB surface.",
                "parameters": [
                    {
                        "name": "id",
                        "in": "path",
                        "required": true,
                        "description": "Evidence package identifier.",
                        "schema": { "type": "string" }
                    }
                ],
                "responses": {
                    "200": {
                        "description": "KYB evidence package export artifact",
                        "content": {
                            "application/json": { "schema": { "type": "object" } }
                        }
                    }
                }
            }
        }),
    );
}

fn attach_manual_partner_registry_paths(openapi: &mut utoipa::openapi::OpenApi) {
    insert_manual_path(
        openapi,
        "/v1/admin/partners",
        json!({
            "get": {
                "tags": ["admin"],
                "operationId": "listPartnerRegistry",
                "summary": "List governed partner registry entries",
                "description": "Returns the bounded partner and connector registry used to govern rails, providers, rollout scopes, health signals, and credential references. Secret material is not returned; only auditable credential metadata is exposed.",
                "responses": {
                    "200": {
                        "description": "Partner registry snapshot",
                        "content": {
                            "application/json": { "schema": { "type": "object" } }
                        }
                    }
                }
            },
            "post": {
                "tags": ["admin"],
                "operationId": "upsertPartnerRegistry",
                "summary": "Upsert governed partner registry entry",
                "description": "Creates or updates a governed partner registry record together with approval references, capabilities, rollout scopes, health signals, and credential references through one additive snapshot payload.",
                "requestBody": {
                    "required": true,
                    "content": {
                        "application/json": {
                            "schema": { "type": "object" }
                        }
                    }
                },
                "responses": {
                    "200": {
                        "description": "Partner registry snapshot after upsert",
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
                            "application/json": {
                                "schema": {
                                    "type": "object",
                                    "required": ["bundle"],
                                    "properties": {
                                        "bundle": {
                                            "type": "object",
                                            "required": [
                                                "bundleId",
                                                "tenantName",
                                                "exportedAt",
                                                "actionMode",
                                                "sections",
                                                "payload",
                                                "approvalStatus",
                                                "rolloutScope",
                                                "provenance",
                                                "source"
                                            ],
                                            "properties": {
                                                "bundleId": { "type": "string" },
                                                "tenantName": { "type": "string" },
                                                "exportedAt": { "type": "string" },
                                                "actionMode": { "type": "string" },
                                                "sections": {
                                                    "type": "array",
                                                    "items": { "type": "string" }
                                                },
                                                "payload": { "type": "object" },
                                                "approvalStatus": { "type": "string" },
                                                "rolloutScope": { "type": "object" },
                                                "provenance": { "type": "object" },
                                                "source": { "type": "string" }
                                            }
                                        }
                                    }
                                }
                            }
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
                            "application/json": {
                                "schema": {
                                    "type": "object",
                                    "required": ["actionMode", "actions", "provenance"],
                                    "properties": {
                                        "actionMode": { "type": "string" },
                                        "actions": {
                                            "type": "array",
                                            "items": {
                                                "type": "object",
                                                "required": [
                                                    "actionId",
                                                    "label",
                                                    "description",
                                                    "enabled",
                                                    "approvalRequired",
                                                    "approvalStatus",
                                                    "rolloutScope",
                                                    "provenance",
                                                    "source"
                                                ],
                                                "properties": {
                                                    "actionId": { "type": "string" },
                                                    "label": { "type": "string" },
                                                    "description": { "type": "string" },
                                                    "enabled": { "type": "boolean" },
                                                    "approvalRequired": { "type": "boolean" },
                                                    "approvalStatus": { "type": "string" },
                                                    "rolloutScope": { "type": "object" },
                                                    "provenance": { "type": "object" },
                                                    "source": { "type": "string" }
                                                }
                                            }
                                        },
                                        "provenance": {
                                            "type": "object",
                                            "required": ["mode", "sourceClass", "actionCount", "sources"],
                                            "properties": {
                                                "mode": { "type": "string" },
                                                "sourceClass": { "type": "string" },
                                                "reason": { "type": "string" },
                                                "actionCount": { "type": "integer" },
                                                "sources": {
                                                    "type": "array",
                                                    "items": { "type": "string" }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }),
    );
}

fn attach_manual_provider_routing_paths(openapi: &mut utoipa::openapi::OpenApi) {
    insert_manual_path(
        openapi,
        "/v1/admin/provider-routing/snapshot",
        json!({
            "get": {
                "tags": ["admin"],
                "operationId": "getProviderRoutingSnapshot",
                "summary": "Get provider routing snapshot",
                "description": "Returns the current provider-routing runtime snapshot. When a database-backed policy store is available, the snapshot is sourced from persisted provider routing policies and exposes top-level provenance.",
                "responses": {
                    "200": {
                        "description": "Provider routing snapshot",
                        "content": {
                            "application/json": {
                                "schema": {
                                    "type": "object",
                                    "required": ["actionMode", "source", "rules", "institutionalRecords", "provenance"],
                                    "properties": {
                                        "actionMode": { "type": "string" },
                                        "source": { "type": "string" },
                                        "rules": {
                                            "type": "array",
                                            "items": { "type": "object" }
                                        },
                                        "institutionalRecords": {
                                            "type": "array",
                                            "items": { "type": "object" }
                                        },
                                        "provenance": { "type": "object" }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }),
    );

    insert_manual_path(
        openapi,
        "/v1/admin/provider-routing/evaluate",
        json!({
            "post": {
                "tags": ["admin"],
                "operationId": "evaluateProviderRouting",
                "summary": "Evaluate provider routing",
                "description": "Evaluates provider routing for a supplied context. When `providerFamily` and a DB-backed policy store are present, the decision uses authoritative persisted policy selection and exposes matched-policy provenance.",
                "requestBody": {
                    "required": true,
                    "content": {
                        "application/json": {
                            "schema": {
                                "type": "object",
                                "properties": {
                                    "providerFamily": { "type": "string" },
                                    "corridorCode": { "type": "string" },
                                    "entityType": { "type": "string" },
                                    "riskTier": { "type": "string" },
                                    "amount": { "type": "string" },
                                    "asset": { "type": "string" },
                                    "partnerId": { "type": "string" },
                                    "tenantId": { "type": "string" }
                                }
                            }
                        }
                    }
                },
                "responses": {
                    "200": {
                        "description": "Provider routing decision",
                        "content": {
                            "application/json": {
                                "schema": {
                                    "type": "object",
                                    "required": [
                                        "selectedProviderKey",
                                        "selectedProviderClass",
                                        "matchedRuleId",
                                        "matchPriority",
                                        "evaluationContext",
                                        "fallbackUsed",
                                        "explanation",
                                        "provenance"
                                    ],
                                    "properties": {
                                        "selectedProviderKey": { "type": "string" },
                                        "selectedProviderClass": { "type": "string" },
                                        "matchedRuleId": { "type": "string" },
                                        "matchPriority": { "type": "integer" },
                                        "evaluationContext": { "type": "object" },
                                        "fallbackUsed": { "type": "boolean" },
                                        "explanation": { "type": "string" },
                                        "provenance": { "type": "object" }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }),
    );
}

fn attach_manual_commercial_readiness_paths(openapi: &mut utoipa::openapi::OpenApi) {
    insert_manual_path(
        openapi,
        "/v1/admin/commercial-readiness/snapshot",
        json!({
            "get": {
                "tags": ["admin"],
                "operationId": "getCommercialReadinessSnapshot",
                "summary": "Get commercial readiness snapshot",
                "description": "Returns the commercial readiness control-plane snapshot. Governed partner registry, corridor pack, compliance, and approval records are used by default when available; otherwise a bounded fallback catalog is returned with explicit provenance.",
                "responses": {
                    "200": {
                        "description": "Commercial readiness snapshot",
                        "content": {
                            "application/json": {
                                "schema": {
                                    "type": "object",
                                    "required": ["actionMode", "source", "extensions", "enabledCount", "disabledCount", "provenance"],
                                    "properties": {
                                        "actionMode": { "type": "string" },
                                        "source": { "type": "string" },
                                        "extensions": {
                                            "type": "array",
                                            "items": { "type": "object" }
                                        },
                                        "enabledCount": { "type": "integer" },
                                        "disabledCount": { "type": "integer" },
                                        "provenance": { "type": "object" }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }),
    );

    insert_manual_path(
        openapi,
        "/v1/admin/commercial-readiness/check",
        json!({
            "post": {
                "tags": ["admin"],
                "operationId": "checkCommercialReadinessEnablement",
                "summary": "Check commercial readiness enablement",
                "description": "Evaluates whether a bounded commercial readiness extension can be enabled and returns missing prerequisites clearly, together with provenance for the readiness record used.",
                "requestBody": {
                    "required": true,
                    "content": {
                        "application/json": {
                            "schema": {
                                "type": "object",
                                "required": [
                                    "extensionId",
                                    "availablePartnerCapabilities",
                                    "availableCorridorPacks",
                                    "complianceChecksPassed"
                                ],
                                "properties": {
                                    "extensionId": { "type": "string" },
                                    "availablePartnerCapabilities": {
                                        "type": "array",
                                        "items": { "type": "string" }
                                    },
                                    "availableCorridorPacks": {
                                        "type": "array",
                                        "items": { "type": "string" }
                                    },
                                    "complianceChecksPassed": {
                                        "type": "array",
                                        "items": { "type": "string" }
                                    }
                                }
                            }
                        }
                    }
                },
                "responses": {
                    "200": {
                        "description": "Commercial readiness enablement result",
                        "content": {
                            "application/json": {
                                "schema": {
                                    "type": "object",
                                    "required": [
                                        "extensionId",
                                        "canEnable",
                                        "hasApproval",
                                        "missingCapabilities",
                                        "missingCorridors",
                                        "missingCompliance",
                                        "provenance"
                                    ],
                                    "properties": {
                                        "extensionId": { "type": "string" },
                                        "canEnable": { "type": "boolean" },
                                        "hasApproval": { "type": "boolean" },
                                        "missingCapabilities": {
                                            "type": "array",
                                            "items": { "type": "string" }
                                        },
                                        "missingCorridors": {
                                            "type": "array",
                                            "items": { "type": "string" }
                                        },
                                        "missingCompliance": {
                                            "type": "array",
                                            "items": { "type": "string" }
                                        },
                                        "provenance": { "type": "object" }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }),
    );
}

fn attach_manual_commercialization_pack_paths(openapi: &mut utoipa::openapi::OpenApi) {
    insert_manual_path(
        openapi,
        "/v1/admin/commercialization-packs",
        json!({
            "get": {
                "tags": ["admin"],
                "operationId": "listCommercializationPacks",
                "summary": "List commercialization packs",
                "description": "Returns commercialization packs composed from governed partner, commercial readiness, and corridor state, including a narrow runtime reference suitable for pilot corridor consumption.",
                "responses": {
                    "200": {
                        "description": "Commercialization pack snapshot",
                        "content": {
                            "application/json": {
                                "schema": {
                                    "type": "object",
                                    "required": ["actionMode", "source", "packs", "provenance"],
                                    "properties": {
                                        "actionMode": { "type": "string" },
                                        "source": { "type": "string" },
                                        "packs": {
                                            "type": "array",
                                            "items": { "type": "object" }
                                        },
                                        "provenance": { "type": "object" }
                                    }
                                }
                            }
                        }
                    }
                }
            },
            "post": {
                "tags": ["admin"],
                "operationId": "upsertCommercializationPack",
                "summary": "Upsert commercialization pack",
                "description": "Creates or updates a commercialization pack reference and returns the composed snapshot on the current governed seams.",
                "requestBody": {
                    "required": true,
                    "content": {
                        "application/json": {
                            "schema": {
                                "type": "object",
                                "required": [
                                    "commercializationPackId",
                                    "packCode",
                                    "partnerId",
                                    "partnerCapabilityId",
                                    "commercialExtensionId",
                                    "corridorCode",
                                    "lifecycleState",
                                    "rolloutState",
                                    "metadata"
                                ],
                                "properties": {
                                    "commercializationPackId": { "type": "string" },
                                    "tenantId": { "type": "string" },
                                    "packCode": { "type": "string" },
                                    "partnerId": { "type": "string" },
                                    "partnerCapabilityId": { "type": "string" },
                                    "commercialExtensionId": { "type": "string" },
                                    "corridorCode": { "type": "string" },
                                    "approvalReference": { "type": "string" },
                                    "lifecycleState": { "type": "string" },
                                    "rolloutState": { "type": "string" },
                                    "metadata": { "type": "object" }
                                }
                            }
                        }
                    }
                },
                "responses": {
                    "200": {
                        "description": "Commercialization pack snapshot",
                        "content": {
                            "application/json": {
                                "schema": {
                                    "type": "object",
                                    "required": ["actionMode", "source", "packs", "provenance"],
                                    "properties": {
                                        "actionMode": { "type": "string" },
                                        "source": { "type": "string" },
                                        "packs": {
                                            "type": "array",
                                            "items": { "type": "object" }
                                        },
                                        "provenance": { "type": "object" }
                                    }
                                }
                            }
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
