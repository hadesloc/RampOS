use axum::{
    extract::{Path, Query, State},
    routing::{get, post},
    Json, Router,
};
use ramp_core::repository::PgVenueTrustRepository;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::Arc;

use ramp_core::service::{
    CommercialReadinessService, CorridorPackService, PaymentMethodCapabilityService,
    PrepareVenueFundingTransferRequest, ProductEligibilityDecision,
    ProductEligibilityDecisionState, ProductEligibilityRequest, ProductEligibilityService,
    SourceOfFundsPackageAction, SubmitVenueFundingTransferRequest, VenueFundingService,
    VenueTrustService,
};

use crate::error::ApiError;
use crate::middleware::PortalUser;
#[allow(unused_imports)]
use crate::openapi::ErrorResponse;
use crate::router::AppState;

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct VenueSummaryResponse {
    pub venue_key: String,
    pub display_name: String,
    pub status: String,
    pub supports_wallet_funding: bool,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct VenueListResponse {
    pub venues: Vec<VenueSummaryResponse>,
}

#[derive(Debug, Clone, Deserialize, utoipa::IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct VenueFundingEligibilityQuery {
    pub venue_key: String,
    pub jurisdiction: String,
    pub asset: String,
    pub network: String,
    pub payment_method_family: Option<String>,
    pub funding_source: Option<String>,
    pub wallet_attestation_state: Option<String>,
    pub user_tier: Option<String>,
    pub kyb_state: Option<String>,
    pub commercial_extension_id: Option<String>,
    pub action: Option<String>,
    pub connection_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct VenueFundingEligibilityReasonResponse {
    pub code: String,
    pub source: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct VenueFundingSourceOfFundsResponse {
    pub action: String,
    pub package_id: Option<String>,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct VenueFundingEligibilityResponse {
    pub subject_type: String,
    pub subject_id: String,
    pub decision: String,
    pub source: String,
    pub reasons: Vec<VenueFundingEligibilityReasonResponse>,
    pub source_of_funds: VenueFundingSourceOfFundsResponse,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct VenueFundingPrepareRequest {
    pub venue_key: String,
    pub jurisdiction: String,
    pub asset: String,
    pub network: String,
    pub venue_connection_id: String,
    pub venue_account_id: String,
    pub wallet_attestation_id: uuid::Uuid,
    pub amount: String,
    pub origin_intent_id: Option<String>,
    pub payment_method_family: Option<String>,
    pub funding_source: Option<String>,
    pub wallet_attestation_state: Option<String>,
    pub user_tier: Option<String>,
    pub kyb_state: Option<String>,
    pub commercial_extension_id: Option<String>,
    pub action: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct VenueFundingConnectionRequest {
    pub venue_key: String,
    pub jurisdiction: String,
    pub asset: String,
    pub network: String,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct VenueFundingConnectionSummaryResponse {
    pub id: String,
    pub venue_key: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct VenueFundingAccountSummaryResponse {
    pub id: String,
    pub network: String,
    pub asset: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct VenueFundingConnectionResponse {
    pub id: String,
    pub subject_type: String,
    pub subject_id: String,
    pub source: String,
    pub connections: Vec<VenueFundingConnectionSummaryResponse>,
    pub accounts: Vec<VenueFundingAccountSummaryResponse>,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct VenueFundingChecklistItemResponse {
    pub code: String,
    pub status: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct VenueFundingPrepareResponse {
    pub id: String,
    pub subject_type: String,
    pub subject_id: String,
    pub status: String,
    pub eligibility_decision: String,
    pub source: String,
    pub checklist: Vec<VenueFundingChecklistItemResponse>,
    pub source_of_funds: VenueFundingSourceOfFundsResponse,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct VenueFundingStatusResponse {
    pub id: String,
    pub subject_type: String,
    pub subject_id: String,
    pub status: String,
    pub eligibility_decision: String,
    pub source: String,
    pub blocking_reasons: Vec<VenueFundingEligibilityReasonResponse>,
    pub source_of_funds: VenueFundingSourceOfFundsResponse,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct VenueFundingConnectionContext {
    tenant_id: String,
    subject_id: String,
    request: VenueFundingConnectionRequest,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct VenueFundingSubmitRequest {
    pub wallet_transfer_reference: String,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct VenueFundingSubmitResponse {
    pub id: String,
    pub subject_type: String,
    pub subject_id: String,
    pub status: String,
    pub eligibility_decision: String,
    pub source: String,
    pub next_action: String,
    pub wallet_transfer_reference: String,
    pub source_of_funds: VenueFundingSourceOfFundsResponse,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/venues", get(list_venue_funding_venues))
        .route("/connection", post(connect_venue_funding))
        .route("/eligibility", get(get_venue_funding_eligibility))
        .route("/prepare", post(prepare_venue_funding))
        .route("/:id/submit", post(submit_venue_funding))
        .route("/:id/status", get(get_venue_funding_status))
}

#[utoipa::path(
    get,
    path = "/v1/portal/venue-funding/venues",
    tag = "portal",
    security(
        ("bearer_auth" = [])
    ),
    responses(
        (
            status = 200,
            description = "List curated venues that support explicit wallet-first venue funding",
            body = VenueListResponse
        )
    )
)]
pub async fn list_venue_funding_venues(
    _portal_user: PortalUser,
) -> Result<Json<VenueListResponse>, ApiError> {
    Ok(Json(VenueListResponse {
        venues: curated_venues(),
    }))
}

#[utoipa::path(
    post,
    path = "/v1/portal/venue-funding/connection",
    request_body = VenueFundingConnectionRequest,
    tag = "portal",
    security(
        ("bearer_auth" = [])
    ),
    responses(
        (
            status = 200,
            description = "Create a venue connection token for the authenticated subject",
            body = VenueFundingConnectionResponse
        ),
        (status = 422, description = "Unprocessable entity"),
        (status = 500, description = "Internal error", body = ErrorResponse)
    )
)]
pub async fn connect_venue_funding(
    portal_user: PortalUser,
    Json(request): Json<VenueFundingConnectionRequest>,
) -> Result<Json<VenueFundingConnectionResponse>, ApiError> {
    validate_required_fields([
        ("venueKey", request.venue_key.trim()),
        ("jurisdiction", request.jurisdiction.trim()),
        ("asset", request.asset.trim()),
        ("network", request.network.trim()),
    ])?;
    validate_hyperliquid_funding_guardrails(&request.venue_key, &request.asset)?;

    let id = encode_connection_id(&VenueFundingConnectionContext {
        tenant_id: portal_user.tenant_id.to_string(),
        subject_id: portal_user.user_id.to_string(),
        request: request.clone(),
    })?;

    Ok(Json(VenueFundingConnectionResponse {
        id: id.clone(),
        subject_type: "user".to_string(),
        subject_id: portal_user.user_id.to_string(),
        source: "registry".to_string(),
        connections: vec![VenueFundingConnectionSummaryResponse {
            id: id.clone(),
            venue_key: request.venue_key.clone(),
            status: "active".to_string(),
        }],
        accounts: vec![VenueFundingAccountSummaryResponse {
            id: format!("acct_{}", request.venue_key),
            network: request.network,
            asset: request.asset,
            status: "active".to_string(),
        }],
    }))
}

#[utoipa::path(
    get,
    path = "/v1/portal/venue-funding/eligibility",
    params(VenueFundingEligibilityQuery),
    tag = "portal",
    security(
        ("bearer_auth" = [])
    ),
    responses(
        (
            status = 200,
            description = "Evaluate portal venue funding eligibility for the authenticated user",
            body = VenueFundingEligibilityResponse
        ),
        (status = 400, description = "Validation error", body = ErrorResponse),
        (status = 500, description = "Internal error", body = ErrorResponse)
    )
)]
pub async fn get_venue_funding_eligibility(
    portal_user: PortalUser,
    State(state): State<AppState>,
    Query(query): Query<VenueFundingEligibilityQuery>,
) -> Result<Json<VenueFundingEligibilityResponse>, ApiError> {
    validate_query(&query)?;
    validate_hyperliquid_funding_guardrails(&query.venue_key, &query.asset)?;

    let tenant_id = portal_user.tenant_id.to_string();
    let service = eligibility_service(&state, &tenant_id).await;
    let mut decision = service
        .evaluate(&ProductEligibilityRequest {
            tenant_id,
            subject_type: "user".to_string(),
            subject_id: portal_user.user_id.to_string(),
            jurisdiction: query.jurisdiction.clone(),
            user_tier: query.user_tier.clone(),
            kyb_state: query.kyb_state.clone(),
            wallet_attestation_state: query
                .wallet_attestation_state
                .clone()
                .unwrap_or_else(|| "verified".to_string()),
            venue_key: query.venue_key.clone(),
            action: query
                .action
                .clone()
                .unwrap_or_else(|| "deposit".to_string()),
            asset: query.asset.clone(),
            network: query.network.clone(),
            payment_method_family: query.payment_method_family.clone(),
            funding_source: query.funding_source.clone(),
            commercial_extension_id: query.commercial_extension_id.clone(),
        })
        .await
        .map_err(|error| ApiError::Internal(error.to_string()))?;

    if let Some(connection_id) = query.connection_id.as_deref() {
        decision = apply_connection_context(
            decision,
            portal_user.clone(),
            connection_id,
            &query.venue_key,
        )?;
    }

    Ok(Json(map_response(portal_user, decision)))
}

#[utoipa::path(
    post,
    path = "/v1/portal/venue-funding/prepare",
    request_body = VenueFundingPrepareRequest,
    tag = "portal",
    security(
        ("bearer_auth" = [])
    ),
    responses(
        (
            status = 200,
            description = "Prepare a portal venue funding lane by creating a durable wallet-to-venue transfer draft",
            body = VenueFundingPrepareResponse
        ),
        (status = 422, description = "Unprocessable entity"),
        (status = 500, description = "Internal error", body = ErrorResponse)
    )
)]
pub async fn prepare_venue_funding(
    portal_user: PortalUser,
    State(state): State<AppState>,
    Json(request): Json<VenueFundingPrepareRequest>,
) -> Result<Json<VenueFundingPrepareResponse>, ApiError> {
    validate_required_fields([
        ("venueKey", request.venue_key.trim()),
        ("jurisdiction", request.jurisdiction.trim()),
        ("asset", request.asset.trim()),
        ("network", request.network.trim()),
        ("venueConnectionId", request.venue_connection_id.trim()),
        ("venueAccountId", request.venue_account_id.trim()),
        ("amount", request.amount.trim()),
    ])?;
    validate_hyperliquid_funding_guardrails(&request.venue_key, &request.asset)?;

    let tenant_id = portal_user.tenant_id.to_string();
    let decision = evaluate_request(&state, &tenant_id, &portal_user, &request).await?;
    let amount = request
        .amount
        .parse()
        .map_err(|_| ApiError::Validation("amount must be a valid decimal".to_string()))?;
    let funding_service = build_venue_funding_service(&state)?;
    let prepared = funding_service
        .prepare_wallet_to_venue_transfer(&PrepareVenueFundingTransferRequest {
            tenant_id,
            user_id: portal_user.user_id.to_string(),
            venue_connection_id: request.venue_connection_id.clone(),
            venue_account_id: request.venue_account_id.clone(),
            wallet_attestation_id: request.wallet_attestation_id,
            asset_symbol: request.asset.clone(),
            network: request.network.clone(),
            amount,
            origin_intent_id: request.origin_intent_id.clone(),
        })
        .await
        .map_err(ApiError::from)?;
    let response = map_prepare_response(portal_user, prepared, decision)?;

    Ok(Json(response))
}

#[utoipa::path(
    post,
    path = "/v1/portal/venue-funding/{id}/submit",
    params(
        ("id" = String, Path, description = "Opaque prepare identifier returned by the prepare endpoint")
    ),
    request_body = VenueFundingSubmitRequest,
    tag = "portal",
    security(
        ("bearer_auth" = [])
    ),
    responses(
        (
            status = 200,
            description = "Attach wallet transfer proof and promote a durable venue funding transfer to submitted",
            body = VenueFundingSubmitResponse
        ),
        (status = 404, description = "Prepared venue funding lane not found", body = ErrorResponse),
        (status = 422, description = "Unprocessable entity"),
        (status = 500, description = "Internal error", body = ErrorResponse)
    )
)]
pub async fn submit_venue_funding(
    portal_user: PortalUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(request): Json<VenueFundingSubmitRequest>,
) -> Result<Json<VenueFundingSubmitResponse>, ApiError> {
    validate_required_fields([(
        "walletTransferReference",
        request.wallet_transfer_reference.trim(),
    )])?;

    let funding_service = build_venue_funding_service(&state)?;
    let submitted = funding_service
        .submit_wallet_to_venue_transfer(&SubmitVenueFundingTransferRequest {
            tenant_id: portal_user.tenant_id.to_string(),
            user_id: portal_user.user_id.to_string(),
            transfer_id: id,
            wallet_transfer_reference: request.wallet_transfer_reference.clone(),
        })
        .await
        .map_err(ApiError::from)?;

    Ok(Json(VenueFundingSubmitResponse {
        id: submitted.transfer_id,
        subject_type: "user".to_string(),
        subject_id: portal_user.user_id.to_string(),
        status: submitted.status,
        eligibility_decision: "allow".to_string(),
        source: "persisted_transfer".to_string(),
        next_action: "await_venue_credit".to_string(),
        wallet_transfer_reference: submitted.wallet_transfer_reference,
        source_of_funds: VenueFundingSourceOfFundsResponse {
            action: "none".to_string(),
            package_id: None,
            source: "persisted_transfer".to_string(),
        },
    }))
}

#[utoipa::path(
    get,
    path = "/v1/portal/venue-funding/{id}/status",
    params(
        ("id" = String, Path, description = "Opaque prepare identifier returned by the prepare endpoint")
    ),
    tag = "portal",
    security(
        ("bearer_auth" = [])
    ),
    responses(
        (
            status = 200,
            description = "Resolve the persisted status for a durable venue funding transfer",
            body = VenueFundingStatusResponse
        ),
        (status = 404, description = "Prepared venue funding lane not found", body = ErrorResponse),
        (status = 500, description = "Internal error", body = ErrorResponse)
    )
)]
pub async fn get_venue_funding_status(
    portal_user: PortalUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<VenueFundingStatusResponse>, ApiError> {
    let funding_service = build_venue_funding_service(&state)?;
    let transfer = funding_service
        .get_wallet_to_venue_transfer(
            &portal_user.tenant_id.to_string(),
            &portal_user.user_id.to_string(),
            &id,
        )
        .await
        .map_err(ApiError::from)?
        .ok_or_else(|| ApiError::NotFound("Prepared venue funding lane not found".to_string()))?;

    Ok(Json(VenueFundingStatusResponse {
        id: transfer.transfer_id,
        subject_type: "user".to_string(),
        subject_id: portal_user.user_id.to_string(),
        status: transfer.status,
        eligibility_decision: "allow".to_string(),
        source: "persisted_transfer".to_string(),
        blocking_reasons: Vec::new(),
        source_of_funds: VenueFundingSourceOfFundsResponse {
            action: "none".to_string(),
            package_id: None,
            source: "persisted_transfer".to_string(),
        },
    }))
}

fn validate_query(query: &VenueFundingEligibilityQuery) -> Result<(), ApiError> {
    validate_required_fields([
        ("venueKey", query.venue_key.trim()),
        ("jurisdiction", query.jurisdiction.trim()),
        ("asset", query.asset.trim()),
        ("network", query.network.trim()),
    ])
}

fn validate_hyperliquid_funding_guardrails(venue_key: &str, asset: &str) -> Result<(), ApiError> {
    if venue_key != "hyperliquid" {
        return Err(ApiError::Business(
            "venue funding currently supports only hyperliquid".to_string(),
        ));
    }

    if asset != "USDT" {
        return Err(ApiError::Business(
            "hyperliquid funding currently supports only USDT".to_string(),
        ));
    }

    Ok(())
}

fn validate_required_fields<'a>(
    fields: impl IntoIterator<Item = (&'a str, &'a str)>,
) -> Result<(), ApiError> {
    for (field, value) in fields {
        if value.is_empty() {
            return Err(ApiError::Validation(format!("{field} is required")));
        }
    }

    Ok(())
}

async fn evaluate_request(
    state: &AppState,
    tenant_id: &str,
    portal_user: &PortalUser,
    request: &VenueFundingPrepareRequest,
) -> Result<ProductEligibilityDecision, ApiError> {
    let service = eligibility_service(state, tenant_id).await;
    service
        .evaluate(&ProductEligibilityRequest {
            tenant_id: tenant_id.to_string(),
            subject_type: "user".to_string(),
            subject_id: portal_user.user_id.to_string(),
            jurisdiction: request.jurisdiction.clone(),
            user_tier: request.user_tier.clone(),
            kyb_state: request.kyb_state.clone(),
            wallet_attestation_state: request
                .wallet_attestation_state
                .clone()
                .unwrap_or_else(|| "verified".to_string()),
            venue_key: request.venue_key.clone(),
            action: request
                .action
                .clone()
                .unwrap_or_else(|| "deposit".to_string()),
            asset: request.asset.clone(),
            network: request.network.clone(),
            payment_method_family: request.payment_method_family.clone(),
            funding_source: request.funding_source.clone(),
            commercial_extension_id: request.commercial_extension_id.clone(),
        })
        .await
        .map_err(|error| ApiError::Internal(error.to_string()))
}

async fn eligibility_service(state: &AppState, tenant_id: &str) -> ProductEligibilityService {
    let Some(pool) = state.db_pool.clone() else {
        return ProductEligibilityService::new(
            VenueTrustService::new(),
            CorridorPackService::new(),
            PaymentMethodCapabilityService::new(),
        );
    };

    let venue_trust = VenueTrustService::with_pool(pool.clone());
    let corridor_pack = CorridorPackService::with_pool(pool.clone());
    let capability = PaymentMethodCapabilityService::with_pool(pool.clone());
    let readiness = CommercialReadinessService::from_pool_for_tenant(pool, Some(tenant_id))
        .await
        .ok();

    match readiness {
        Some(readiness) => ProductEligibilityService::with_commercial_readiness(
            venue_trust,
            corridor_pack,
            capability,
            readiness,
        ),
        None => ProductEligibilityService::new(venue_trust, corridor_pack, capability),
    }
}

fn map_response(
    portal_user: PortalUser,
    decision: ProductEligibilityDecision,
) -> VenueFundingEligibilityResponse {
    VenueFundingEligibilityResponse {
        subject_type: "user".to_string(),
        subject_id: portal_user.user_id.to_string(),
        decision: map_decision_state(decision.decision).to_string(),
        source: decision.source,
        reasons: decision
            .reasons
            .into_iter()
            .map(|reason| VenueFundingEligibilityReasonResponse {
                code: reason.code,
                source: reason.source,
                message: reason.message,
            })
            .collect(),
        source_of_funds: VenueFundingSourceOfFundsResponse {
            action: map_source_of_funds_action(decision.source_of_funds.action).to_string(),
            package_id: decision.source_of_funds.package_id,
            source: decision.source_of_funds.source,
        },
    }
}

fn map_prepare_response(
    portal_user: PortalUser,
    prepared: ramp_core::service::PreparedVenueFundingTransfer,
    decision: ProductEligibilityDecision,
) -> Result<VenueFundingPrepareResponse, ApiError> {
    let decision_state_value = decision.decision.clone();
    let checklist = build_checklist(&decision);
    let source = decision.source.clone();
    let decision_state = map_decision_state(decision_state_value).to_string();
    let source_of_funds = VenueFundingSourceOfFundsResponse {
        action: map_source_of_funds_action(decision.source_of_funds.action).to_string(),
        package_id: decision.source_of_funds.package_id,
        source: decision.source_of_funds.source,
    };

    Ok(VenueFundingPrepareResponse {
        id: prepared.transfer_id,
        subject_type: "user".to_string(),
        subject_id: portal_user.user_id.to_string(),
        status: prepared.status,
        eligibility_decision: decision_state,
        source,
        checklist,
        source_of_funds,
    })
}

fn build_checklist(
    decision: &ProductEligibilityDecision,
) -> Vec<VenueFundingChecklistItemResponse> {
    let checklist_status = checklist_status_for_decision(decision.decision.clone()).to_string();
    let mut checklist: Vec<VenueFundingChecklistItemResponse> = decision
        .reasons
        .iter()
        .map(|reason| VenueFundingChecklistItemResponse {
            code: reason.code.clone(),
            status: checklist_status.clone(),
            message: reason.message.clone(),
        })
        .collect();

    if !matches!(
        decision.source_of_funds.action,
        SourceOfFundsPackageAction::None
    ) {
        let action = decision.source_of_funds.action.clone();
        checklist.push(VenueFundingChecklistItemResponse {
            code: "source_of_funds".to_string(),
            status: "action_required".to_string(),
            message: format!(
                "Source of funds action: {}",
                map_source_of_funds_action(action)
            ),
        });
    }

    if checklist.is_empty() {
        checklist.push(VenueFundingChecklistItemResponse {
            code: "venue_funding_ready".to_string(),
            status: "ready".to_string(),
            message: "Venue funding lane is ready for the authenticated subject".to_string(),
        });
    }

    checklist
}

fn checklist_status_for_decision(state: ProductEligibilityDecisionState) -> &'static str {
    match state {
        ProductEligibilityDecisionState::Allow => "ready",
        ProductEligibilityDecisionState::Review => "pending_review",
        ProductEligibilityDecisionState::Deny => "action_required",
    }
}

fn map_decision_state(state: ProductEligibilityDecisionState) -> &'static str {
    match state {
        ProductEligibilityDecisionState::Allow => "allow",
        ProductEligibilityDecisionState::Review => "review",
        ProductEligibilityDecisionState::Deny => "deny",
    }
}

fn map_source_of_funds_action(action: SourceOfFundsPackageAction) -> &'static str {
    match action {
        SourceOfFundsPackageAction::None => "none",
        SourceOfFundsPackageAction::Reuse => "reuse",
        SourceOfFundsPackageAction::CreateDraft => "create_draft",
        SourceOfFundsPackageAction::CreateInReview => "create_in_review",
    }
}

fn curated_venues() -> Vec<VenueSummaryResponse> {
    vec![
        VenueSummaryResponse {
            venue_key: "kraken".to_string(),
            display_name: "Kraken".to_string(),
            status: "active".to_string(),
            supports_wallet_funding: false,
        },
        VenueSummaryResponse {
            venue_key: "hyperliquid".to_string(),
            display_name: "Hyperliquid".to_string(),
            status: "pilot".to_string(),
            supports_wallet_funding: true,
        },
    ]
}

fn apply_connection_context(
    mut decision: ProductEligibilityDecision,
    portal_user: PortalUser,
    connection_id: &str,
    expected_venue_key: &str,
) -> Result<ProductEligibilityDecision, ApiError> {
    let connection = decode_connection_id(connection_id)?;
    if connection.tenant_id != portal_user.tenant_id.to_string()
        || connection.subject_id != portal_user.user_id.to_string()
        || connection.request.venue_key != expected_venue_key
    {
        return Err(ApiError::NotFound(
            "Prepared venue funding lane not found".to_string(),
        ));
    }

    let original_reason_count = decision.reasons.len();
    decision
        .reasons
        .retain(|reason| reason.code != "venue_not_ready");
    if matches!(decision.decision, ProductEligibilityDecisionState::Deny)
        && original_reason_count != decision.reasons.len()
        && decision.reasons.is_empty()
    {
        decision.decision = ProductEligibilityDecisionState::Allow;
    }

    Ok(decision)
}

fn build_venue_funding_service(state: &AppState) -> Result<VenueFundingService, ApiError> {
    let pool = state.db_pool.clone().ok_or_else(|| {
        ApiError::Internal(
            "Venue funding runtime is unavailable: database not configured".to_string(),
        )
    })?;

    Ok(VenueFundingService::new(Arc::new(
        PgVenueTrustRepository::new(pool),
    )))
}

fn encode_connection_id(context: &VenueFundingConnectionContext) -> Result<String, ApiError> {
    let encoded =
        serde_json::to_vec(context).map_err(|error| ApiError::Internal(error.to_string()))?;
    let checksum = {
        let mut hasher = Sha256::new();
        hasher.update(&encoded);
        hex::encode(hasher.finalize())
    };

    Ok(format!(
        "vfconn_{}{}",
        &checksum[..12],
        hex::encode(encoded)
    ))
}

fn decode_connection_id(id: &str) -> Result<VenueFundingConnectionContext, ApiError> {
    let Some(encoded) = id.strip_prefix("vfconn_").and_then(|value| value.get(12..)) else {
        return Err(ApiError::NotFound(
            "Prepared venue funding lane not found".to_string(),
        ));
    };

    let bytes = hex::decode(encoded)
        .map_err(|_| ApiError::NotFound("Prepared venue funding lane not found".to_string()))?;

    serde_json::from_slice(&bytes)
        .map_err(|_| ApiError::NotFound("Prepared venue funding lane not found".to_string()))
}
