//! SSO API Handlers

use axum::{
    extract::{Path, Query, State},
    response::{IntoResponse, Redirect},
    Json,
};
use ramp_core::sso::{
    SsoAuthRequest, SsoCallback, SsoProviderSummary,
};
use chrono::{Duration, Utc};
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::Mutex;

use crate::AppState;
use crate::error::ApiError;

fn sanitize_redirect_target(input: &str) -> String {
    // Only allow relative in-app paths to prevent open redirects.
    // Accept: /dashboard, /settings?tab=security
    // Reject: https://evil.com, //evil.com, javascript:, path without leading slash
    if input.starts_with('/') && !input.starts_with("//") && !input.contains(['\r', '\n']) {
        input.to_string()
    } else {
        "/dashboard".to_string()
    }
}

#[derive(Debug, Clone)]
struct SsoAuthFlowState {
    redirect_to: String,
    callback_url: String,
    expires_at: chrono::DateTime<Utc>,
}

const SSO_STATE_TTL_MINUTES: i64 = 10;

static SSO_AUTH_STATES: Lazy<Mutex<HashMap<String, SsoAuthFlowState>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

fn store_sso_state(state: String, flow: SsoAuthFlowState) {
    let mut map = SSO_AUTH_STATES.lock().unwrap();
    map.retain(|_, entry| entry.expires_at > Utc::now());
    map.insert(state, flow);
}

fn consume_sso_state(state: &str) -> Option<SsoAuthFlowState> {
    let mut map = SSO_AUTH_STATES.lock().unwrap();
    map.retain(|_, entry| entry.expires_at > Utc::now());
    map.remove(state)
}

/// Initiate SSO login
#[utoipa::path(
    get,
    path = "/v1/auth/sso/{provider}/login",
    tag = "Auth",
    params(
        ("provider" = String, Path, description = "Identity Provider ID or alias"),
        ("redirect_to" = Option<String>, Query, description = "Post-login redirect URL")
    ),
    responses(
        (status = 302, description = "Redirect to Identity Provider"),
        (status = 404, description = "Provider not found",
         example = json!({"error": {"code": "NOT_FOUND", "message": "SSO provider 'unknown_idp' not found"}}))
    )
)]
pub async fn sso_login(
    State(state): State<AppState>,
    Path(provider_id): Path<String>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<impl IntoResponse, ApiError> {
    let redirect_to = params
        .get("redirect_to")
        .map(|v| sanitize_redirect_target(v))
        .unwrap_or_else(|| "/dashboard".to_string());

    // Construct return URL (callback URL)
    // Read base URL from environment to avoid hardcoded localhost in production
    let api_base_url = std::env::var("API_BASE_URL")
        .unwrap_or_else(|_| "http://localhost:3000".to_string());
    let callback_url = format!("{}/v1/auth/sso/{}/callback", api_base_url, provider_id);

    let state_token = uuid::Uuid::new_v4().to_string();
    store_sso_state(
        state_token.clone(),
        SsoAuthFlowState {
            redirect_to,
            callback_url: callback_url.clone(),
            expires_at: Utc::now() + Duration::minutes(SSO_STATE_TTL_MINUTES),
        },
    );

    let request = SsoAuthRequest {
        tenant_id: ramp_common::types::TenantId::new(&provider_id),
        redirect_uri: callback_url,
        state: state_token,
        nonce: Some(uuid::Uuid::new_v4().to_string()),
    };

    let response = state.sso_service.initiate_auth(&request).await.map_err(ApiError::from)?;

    Ok(Redirect::to(&response.auth_url))
}

/// SSO Callback
#[utoipa::path(
    get,
    path = "/v1/auth/sso/{provider}/callback",
    tag = "Auth",
    params(
        ("provider" = String, Path, description = "Identity Provider ID"),
        ("code" = Option<String>, Query, description = "Auth code (OIDC)"),
        ("state" = Option<String>, Query, description = "State parameter"),
        ("SAMLResponse" = Option<String>, Query, description = "SAML Response (if SAML)"),
    ),
    responses(
        (status = 302, description = "Redirect to dashboard with session token")
    )
)]
pub async fn sso_callback(
    State(state): State<AppState>,
    Path(provider_id): Path<String>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Redirect, ApiError> {
    // Extract parameters
    let code = params.get("code").or(params.get("SAMLResponse")).cloned();
    let state_param = params.get("state").cloned();
    let error = params.get("error").cloned();
    let error_description = params.get("error_description").cloned();

    let state_token = state_param
        .as_deref()
        .ok_or_else(|| ApiError::BadRequest("Missing state parameter".to_string()))?;
    let flow_state = consume_sso_state(state_token)
        .ok_or_else(|| ApiError::Unauthorized("Invalid or expired state parameter".to_string()))?;

    let callback_url = format!(
        "{}/v1/auth/sso/{}/callback",
        std::env::var("API_BASE_URL").unwrap_or_else(|_| "http://localhost:3000".to_string()),
        provider_id
    );
    if flow_state.callback_url != callback_url {
        return Err(ApiError::Unauthorized(
            "SSO callback URL mismatch for state parameter".to_string(),
        ));
    }

    let callback = SsoCallback {
        code,
        saml_response: params.get("SAMLResponse").cloned(),
        state: state_token.to_string(),
        error,
        error_description,
    };

    // Authenticate with provider
    let tenant_id = ramp_common::types::TenantId::new(&provider_id);
    let _sso_user = state.sso_service.handle_callback(&tenant_id, &callback).await.map_err(ApiError::from)?;

    // Session issuance is intentionally hard-failed until production session plumbing exists.
    // Never mint or return synthetic SSO tokens from runtime handlers.
    let destination = sanitize_redirect_target(&flow_state.redirect_to);
    Err(ApiError::Internal(format!(
        "SSO authentication succeeded for provider '{}' but session issuance is not configured. Refusing to issue synthetic token. Requested redirect: {}",
        provider_id, destination
    )))
}

/// List Identity Providers
#[utoipa::path(
    get,
    path = "/v1/admin/sso/providers",
    tag = "Admin",
    responses(
        (status = 200, description = "List of configured IdPs",
         example = json!([{
             "providerId": "okta_acme",
             "providerType": "okta",
             "protocol": "oidc",
             "enabled": true
         }]))
    )
)]
pub async fn list_providers(
    State(state): State<AppState>,
) -> Result<Json<Vec<SsoProviderSummary>>, ApiError> {
    let providers = state.sso_service.list_provider_summaries();

    if providers.is_empty() {
        return Err(ApiError::NotFound(
            "No SSO providers are configured for this runtime".to_string(),
        ));
    }

    Ok(Json(providers))
}
