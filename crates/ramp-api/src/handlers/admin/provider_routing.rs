use axum::{extract::State, http::HeaderMap, Json};
use rust_decimal::Decimal;
use std::str::FromStr;

use ramp_compliance::provider_routing::{
    ProviderFamily, ProviderRoutingPolicyStore, ProviderRoutingQuery,
};
use ramp_core::service::{
    ProviderRoutingContext, ProviderRoutingDecision, ProviderRoutingService,
    ProviderRoutingSnapshot,
};

use crate::error::ApiError;
use crate::router::AppState;

pub async fn get_provider_routing_snapshot(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Result<Json<ProviderRoutingSnapshot>, ApiError> {
    super::tier::check_admin_key(&headers)?;

    if let Some(pool) = state.db_pool.clone() {
        let store = ProviderRoutingPolicyStore::new(pool);
        let mut policies = Vec::new();
        for family in all_provider_families() {
            let mut family_policies = store
                .list_policies(None, family)
                .await
                .map_err(|error| ApiError::Internal(error.to_string()))?;
            policies.append(&mut family_policies);
        }
        let service = ProviderRoutingService::from_policies(policies);
        return Ok(Json(service.snapshot()));
    }

    let service = ProviderRoutingService::new();
    Ok(Json(service.snapshot()))
}

pub async fn evaluate_provider_routing(
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(context): Json<ProviderRoutingContext>,
) -> Result<Json<ProviderRoutingDecision>, ApiError> {
    super::tier::check_admin_key(&headers)?;

    if let (Some(pool), Some(provider_family)) = (
        state.db_pool.clone(),
        context
            .provider_family
            .as_deref()
            .and_then(parse_provider_family),
    ) {
        let store = ProviderRoutingPolicyStore::new(pool);
        let query = ProviderRoutingQuery {
            provider_family,
            corridor_code: context.corridor_code.clone(),
            entity_type: context.entity_type.clone(),
            risk_tier: context.risk_tier.clone(),
            partner_key: context.partner_id.clone(),
            asset_code: context.asset.clone(),
            amount: context
                .amount
                .as_deref()
                .and_then(|value| Decimal::from_str(value).ok()),
        };
        if let Some(policy) = store
            .select_policy(context.tenant_id.as_deref(), &query)
            .await
            .map_err(|error| ApiError::Internal(error.to_string()))?
        {
            return Ok(Json(ProviderRoutingService::decision_from_policy(
                &policy, &context,
            )));
        }
    }

    let service = ProviderRoutingService::new();
    let decision = service
        .evaluate(&context)
        .map_err(|error| ApiError::Internal(error.to_string()))?;
    Ok(Json(decision))
}

fn parse_provider_family(value: &str) -> Option<ProviderFamily> {
    match value.trim().to_ascii_lowercase().as_str() {
        "kyc" => Some(ProviderFamily::Kyc),
        "kyb" => Some(ProviderFamily::Kyb),
        "kyt" => Some(ProviderFamily::Kyt),
        "sanctions" => Some(ProviderFamily::Sanctions),
        "adverse_media" => Some(ProviderFamily::AdverseMedia),
        "travel_rule" => Some(ProviderFamily::TravelRule),
        _ => None,
    }
}

fn all_provider_families() -> [ProviderFamily; 6] {
    [
        ProviderFamily::Kyc,
        ProviderFamily::Kyb,
        ProviderFamily::Kyt,
        ProviderFamily::Sanctions,
        ProviderFamily::AdverseMedia,
        ProviderFamily::TravelRule,
    ]
}
