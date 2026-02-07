//! SSO API Handlers

use axum::{
    extract::{Path, Query, State},
    response::{IntoResponse, Redirect},
    Json,
};
use ramp_core::sso::{
    SsoAuthRequest, SsoCallback,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::AppState;
use crate::error::ApiError;

/// Summary of an SSO provider for listing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SsoProviderSummary {
    pub provider_id: String,
    pub provider_type: String,
    pub protocol: String,
    pub enabled: bool,
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
        (status = 404, description = "Provider not found")
    )
)]
pub async fn sso_login(
    State(state): State<AppState>,
    Path(provider_id): Path<String>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<impl IntoResponse, ApiError> {
    let redirect_to = params.get("redirect_to").cloned().unwrap_or_else(|| "/dashboard".to_string());

    // Construct return URL (callback URL)
    // Read base URL from environment to avoid hardcoded localhost in production
    let api_base_url = std::env::var("API_BASE_URL")
        .unwrap_or_else(|_| "http://localhost:3000".to_string());
    let callback_url = format!("{}/v1/auth/sso/{}/callback", api_base_url, provider_id);

    let request = SsoAuthRequest {
        tenant_id: ramp_common::types::TenantId::new(&provider_id),
        redirect_uri: callback_url,
        state: redirect_to, // Store final destination in state for simplicity (CSRF token should be here too)
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
) -> Result<impl IntoResponse, ApiError> {
    // Extract parameters
    let code = params.get("code").or(params.get("SAMLResponse")).cloned();
    let state_param = params.get("state").cloned();
    let error = params.get("error").cloned();
    let error_description = params.get("error_description").cloned();

    let callback = SsoCallback {
        code,
        saml_response: params.get("SAMLResponse").cloned(),
        state: state_param.clone().unwrap_or_default(),
        error,
        error_description,
    };

    // Authenticate with provider
    let tenant_id = ramp_common::types::TenantId::new(&provider_id);
    let _sso_user = state.sso_service.handle_callback(&tenant_id, &callback).await.map_err(ApiError::from)?;

    // Create session for user
    // 1. Check if user exists (by email) or JIT provision
    // 2. Create JWT/Session
    // 3. Redirect

    // For MVP, we'll assume JIT provisioning logic is in sso_service or handled here
    // Let's assume sso_service handles the user mapping and returns a resolved user

    // Generate session token (mock)
    let session_token = format!("sso_{}_{}", provider_id, uuid::Uuid::new_v4());

    // Store session (omitted)

    // Redirect to original destination
    let destination = state_param.unwrap_or_else(|| "/dashboard".to_string());
    let redirect_url = format!("{}?token={}", destination, session_token);

    Ok(Redirect::to(&redirect_url))
}

/// List Identity Providers
#[utoipa::path(
    get,
    path = "/v1/admin/sso/providers",
    tag = "Admin",
    responses(
        (status = 200, description = "List of configured IdPs")
    )
)]
pub async fn list_providers(
    State(_state): State<AppState>,
) -> Result<Json<Vec<SsoProviderSummary>>, ApiError> {
    // Implementation requires Admin API extensions
    // This is a placeholder
    Ok(Json(vec![]))
}
