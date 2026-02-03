//! Portal Authentication Handlers
//!
//! Endpoints for user authentication including:
//! - WebAuthn (Passkey) registration and login
//! - Magic link authentication
//! - Session management

use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use tracing::info;
use uuid::Uuid;
use validator::Validate;

use crate::error::ApiError;
use crate::router::AppState;

// ============================================================================
// DTOs
// ============================================================================

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthUser {
    pub id: String,
    pub email: String,
    pub kyc_status: String,
    pub kyc_tier: i32,
    pub status: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthSession {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: i64,
    pub user: AuthUser,
}

#[derive(Debug, Clone, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct WebAuthnChallengeRequest {
    #[validate(email(message = "Invalid email address"))]
    pub email: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WebAuthnChallenge {
    pub challenge: String,
    pub rp_id: String,
    pub rp_name: String,
    pub user_id: String,
    pub user_name: String,
    pub user_display_name: String,
    pub timeout: u32,
    pub attestation: String,
    pub authenticator_selection: AuthenticatorSelection,
    pub pub_key_cred_params: Vec<PubKeyCredParam>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub exclude_credentials: Vec<CredentialDescriptor>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthenticatorSelection {
    pub authenticator_attachment: Option<String>,
    pub resident_key: String,
    pub user_verification: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PubKeyCredParam {
    #[serde(rename = "type")]
    pub credential_type: String,
    pub alg: i32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CredentialDescriptor {
    pub id: String,
    #[serde(rename = "type")]
    pub credential_type: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub transports: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebAuthnCredentialResponse {
    pub id: String,
    pub raw_id: String,
    #[serde(rename = "type")]
    pub credential_type: String,
    pub response: WebAuthnAuthenticatorResponse,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebAuthnAuthenticatorResponse {
    pub client_data_json: String,
    pub attestation_object: Option<String>,
    pub authenticator_data: Option<String>,
    pub signature: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct WebAuthnRegisterCompleteRequest {
    #[validate(email(message = "Invalid email address"))]
    pub email: String,
    pub credential: WebAuthnCredentialResponse,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebAuthnLoginCompleteRequest {
    pub credential: WebAuthnCredentialResponse,
}

#[derive(Debug, Clone, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct MagicLinkRequest {
    #[validate(email(message = "Invalid email address"))]
    pub email: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MagicLinkResponse {
    pub message: String,
}

#[derive(Debug, Clone, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct MagicLinkVerifyRequest {
    #[validate(length(min = 1, message = "Token is required"))]
    pub token: String,
}

#[derive(Debug, Clone, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct RefreshTokenRequest {
    #[validate(length(min = 1, message = "Refresh token is required"))]
    pub refresh_token: String,
}

// ============================================================================
// Router
// ============================================================================

pub fn router() -> Router<AppState> {
    Router::new()
        // WebAuthn endpoints
        .route(
            "/webauthn/register/challenge",
            post(webauthn_register_challenge),
        )
        .route(
            "/webauthn/register/complete",
            post(webauthn_register_complete),
        )
        .route("/webauthn/login/challenge", post(webauthn_login_challenge))
        .route("/webauthn/login/complete", post(webauthn_login_complete))
        // Magic link endpoints
        .route("/magic-link", post(request_magic_link))
        .route("/magic-link/verify", post(verify_magic_link))
        // Session endpoints
        .route("/refresh", post(refresh_token))
        .route("/logout", post(logout))
        .route("/me", get(get_me))
}

// ============================================================================
// Handlers
// ============================================================================

/// POST /v1/auth/webauthn/register/challenge - Get WebAuthn registration challenge
pub async fn webauthn_register_challenge(
    State(_app_state): State<AppState>,
    Json(req): Json<WebAuthnChallengeRequest>,
) -> Result<Json<WebAuthnChallenge>, ApiError> {
    // Validate request
    req.validate()
        .map_err(|e| ApiError::Validation(e.to_string()))?;

    info!(email = %req.email, "WebAuthn registration challenge requested");

    // Generate a random challenge
    let challenge = base64_url_encode(&generate_random_bytes(32));
    let user_id = Uuid::new_v4().to_string();

    // In production, this would:
    // 1. Check if user already exists
    // 2. Store the challenge temporarily
    // 3. Use proper RP configuration from environment

    let response = WebAuthnChallenge {
        challenge,
        rp_id: std::env::var("WEBAUTHN_RP_ID").unwrap_or_else(|_| "localhost".to_string()),
        rp_name: std::env::var("WEBAUTHN_RP_NAME").unwrap_or_else(|_| "RampOS Portal".to_string()),
        user_id: base64_url_encode(user_id.as_bytes()),
        user_name: req.email.clone(),
        user_display_name: req
            .email
            .split('@')
            .next()
            .unwrap_or(&req.email)
            .to_string(),
        timeout: 60000,
        attestation: "none".to_string(),
        authenticator_selection: AuthenticatorSelection {
            authenticator_attachment: Some("platform".to_string()),
            resident_key: "preferred".to_string(),
            user_verification: "required".to_string(),
        },
        pub_key_cred_params: vec![
            PubKeyCredParam {
                credential_type: "public-key".to_string(),
                alg: -7, // ES256
            },
            PubKeyCredParam {
                credential_type: "public-key".to_string(),
                alg: -257, // RS256
            },
        ],
        exclude_credentials: vec![],
    };

    Ok(Json(response))
}

/// POST /v1/auth/webauthn/register/complete - Complete WebAuthn registration
pub async fn webauthn_register_complete(
    State(_app_state): State<AppState>,
    Json(req): Json<WebAuthnRegisterCompleteRequest>,
) -> Result<Json<AuthSession>, ApiError> {
    // Validate request
    req.validate()
        .map_err(|e| ApiError::Validation(e.to_string()))?;

    info!(email = %req.email, credential_id = %req.credential.id, "WebAuthn registration completing");

    // In production, this would:
    // 1. Verify the challenge
    // 2. Validate the attestation
    // 3. Store the credential
    // 4. Create the user if new
    // 5. Generate JWT tokens

    let now = Utc::now();
    let expires_at = now + Duration::hours(24);
    let user_id = Uuid::new_v4().to_string();

    let session = AuthSession {
        access_token: generate_mock_jwt(&user_id),
        refresh_token: Uuid::new_v4().to_string(),
        expires_at: expires_at.timestamp(),
        user: AuthUser {
            id: user_id,
            email: req.email,
            kyc_status: "NONE".to_string(),
            kyc_tier: 0,
            status: "ACTIVE".to_string(),
            created_at: now.to_rfc3339(),
        },
    };

    Ok(Json(session))
}

/// POST /v1/auth/webauthn/login/challenge - Get WebAuthn login challenge
pub async fn webauthn_login_challenge(
    State(_app_state): State<AppState>,
    Json(req): Json<Option<WebAuthnChallengeRequest>>,
) -> Result<Json<WebAuthnChallenge>, ApiError> {
    let email = req.map(|r| r.email).unwrap_or_default();

    info!(email = %email, "WebAuthn login challenge requested");

    // Generate a random challenge
    let challenge = base64_url_encode(&generate_random_bytes(32));

    // In production, this would:
    // 1. Look up user's credentials if email provided
    // 2. Store the challenge temporarily

    let response = WebAuthnChallenge {
        challenge,
        rp_id: std::env::var("WEBAUTHN_RP_ID").unwrap_or_else(|_| "localhost".to_string()),
        rp_name: std::env::var("WEBAUTHN_RP_NAME").unwrap_or_else(|_| "RampOS Portal".to_string()),
        user_id: String::new(),
        user_name: email.clone(),
        user_display_name: email.split('@').next().unwrap_or(&email).to_string(),
        timeout: 60000,
        attestation: "none".to_string(),
        authenticator_selection: AuthenticatorSelection {
            authenticator_attachment: None,
            resident_key: "preferred".to_string(),
            user_verification: "required".to_string(),
        },
        pub_key_cred_params: vec![
            PubKeyCredParam {
                credential_type: "public-key".to_string(),
                alg: -7,
            },
            PubKeyCredParam {
                credential_type: "public-key".to_string(),
                alg: -257,
            },
        ],
        exclude_credentials: vec![],
    };

    Ok(Json(response))
}

/// POST /v1/auth/webauthn/login/complete - Complete WebAuthn login
pub async fn webauthn_login_complete(
    State(_app_state): State<AppState>,
    Json(req): Json<WebAuthnLoginCompleteRequest>,
) -> Result<Json<AuthSession>, ApiError> {
    info!(credential_id = %req.credential.id, "WebAuthn login completing");

    // In production, this would:
    // 1. Verify the challenge
    // 2. Validate the signature
    // 3. Look up user by credential
    // 4. Generate JWT tokens

    let now = Utc::now();
    let expires_at = now + Duration::hours(24);
    let user_id = Uuid::new_v4().to_string();

    let session = AuthSession {
        access_token: generate_mock_jwt(&user_id),
        refresh_token: Uuid::new_v4().to_string(),
        expires_at: expires_at.timestamp(),
        user: AuthUser {
            id: user_id,
            email: "user@example.com".to_string(), // Would come from credential lookup
            kyc_status: "VERIFIED".to_string(),
            kyc_tier: 1,
            status: "ACTIVE".to_string(),
            created_at: now.to_rfc3339(),
        },
    };

    Ok(Json(session))
}

/// POST /v1/auth/magic-link - Request magic link
pub async fn request_magic_link(
    State(_app_state): State<AppState>,
    Json(req): Json<MagicLinkRequest>,
) -> Result<Json<MagicLinkResponse>, ApiError> {
    req.validate()
        .map_err(|e| ApiError::Validation(e.to_string()))?;

    info!(email = %req.email, "Magic link requested");

    // In production, this would:
    // 1. Generate a secure token
    // 2. Store token with email and expiry
    // 3. Send email with the link

    // Always return success to prevent email enumeration
    Ok(Json(MagicLinkResponse {
        message: "If an account exists with this email, a login link has been sent.".to_string(),
    }))
}

/// POST /v1/auth/magic-link/verify - Verify magic link token
pub async fn verify_magic_link(
    State(_app_state): State<AppState>,
    Json(req): Json<MagicLinkVerifyRequest>,
) -> Result<Json<AuthSession>, ApiError> {
    req.validate()
        .map_err(|e| ApiError::Validation(e.to_string()))?;

    info!("Magic link verification attempted");

    // In production, this would:
    // 1. Look up the token
    // 2. Verify it hasn't expired
    // 3. Get or create the user
    // 4. Invalidate the token (one-time use)
    // 5. Generate session tokens

    // For now, return an error since we can't actually verify
    // In a real implementation, we'd check the token against stored values
    if req.token.is_empty() {
        return Err(ApiError::Unauthorized(
            "Invalid or expired token".to_string(),
        ));
    }

    let now = Utc::now();
    let expires_at = now + Duration::hours(24);
    let user_id = Uuid::new_v4().to_string();

    let session = AuthSession {
        access_token: generate_mock_jwt(&user_id),
        refresh_token: Uuid::new_v4().to_string(),
        expires_at: expires_at.timestamp(),
        user: AuthUser {
            id: user_id,
            email: "user@example.com".to_string(),
            kyc_status: "NONE".to_string(),
            kyc_tier: 0,
            status: "ACTIVE".to_string(),
            created_at: now.to_rfc3339(),
        },
    };

    Ok(Json(session))
}

/// POST /v1/auth/refresh - Refresh access token
pub async fn refresh_token(
    State(_app_state): State<AppState>,
    Json(req): Json<RefreshTokenRequest>,
) -> Result<Json<AuthSession>, ApiError> {
    req.validate()
        .map_err(|e| ApiError::Validation(e.to_string()))?;

    info!("Token refresh requested");

    // In production, this would:
    // 1. Validate the refresh token
    // 2. Check if it's revoked
    // 3. Get the associated user
    // 4. Generate new tokens
    // 5. Optionally rotate the refresh token

    if req.refresh_token.is_empty() {
        return Err(ApiError::Unauthorized("Invalid refresh token".to_string()));
    }

    let now = Utc::now();
    let expires_at = now + Duration::hours(24);
    let user_id = Uuid::new_v4().to_string();

    let session = AuthSession {
        access_token: generate_mock_jwt(&user_id),
        refresh_token: Uuid::new_v4().to_string(),
        expires_at: expires_at.timestamp(),
        user: AuthUser {
            id: user_id,
            email: "user@example.com".to_string(),
            kyc_status: "VERIFIED".to_string(),
            kyc_tier: 1,
            status: "ACTIVE".to_string(),
            created_at: now.to_rfc3339(),
        },
    };

    Ok(Json(session))
}

/// POST /v1/auth/logout - Logout and invalidate session
pub async fn logout(State(_app_state): State<AppState>) -> Result<(), ApiError> {
    info!("User logout requested");

    // In production, this would:
    // 1. Get the current session from auth header
    // 2. Revoke the refresh token
    // 3. Optionally add access token to blocklist

    Ok(())
}

/// GET /v1/auth/me - Get current user info
pub async fn get_me(
    State(_app_state): State<AppState>,
    // In production, would extract user from auth middleware
) -> Result<Json<AuthUser>, ApiError> {
    info!("Get current user info requested");

    // In production, this would:
    // 1. Extract user ID from JWT
    // 2. Fetch user from database
    // 3. Return user info

    // For now, return mock user (would be replaced by portal auth middleware)
    let now = Utc::now();

    Ok(Json(AuthUser {
        id: Uuid::new_v4().to_string(),
        email: "user@example.com".to_string(),
        kyc_status: "VERIFIED".to_string(),
        kyc_tier: 1,
        status: "ACTIVE".to_string(),
        created_at: now.to_rfc3339(),
    }))
}

// ============================================================================
// Helper Functions
// ============================================================================

fn generate_random_bytes(len: usize) -> Vec<u8> {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    (0..len).map(|_| rng.gen()).collect()
}

fn base64_url_encode(data: &[u8]) -> String {
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
    URL_SAFE_NO_PAD.encode(data)
}

fn generate_mock_jwt(user_id: &str) -> String {
    // This is a mock JWT for development
    // In production, use proper JWT library with signing
    let header = base64_url_encode(b"{\"alg\":\"HS256\",\"typ\":\"JWT\"}");
    let now = Utc::now().timestamp();
    let payload = format!(
        "{{\"sub\":\"{}\",\"iat\":{},\"exp\":{}}}",
        user_id,
        now,
        now + 86400
    );
    let payload_b64 = base64_url_encode(payload.as_bytes());
    let signature = base64_url_encode(b"mock_signature");

    format!("{}.{}.{}", header, payload_b64, signature)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base64_url_encode() {
        let data = b"hello world";
        let encoded = base64_url_encode(data);
        assert!(!encoded.contains('+'));
        assert!(!encoded.contains('/'));
        assert!(!encoded.contains('='));
    }

    #[test]
    fn test_generate_random_bytes() {
        let bytes = generate_random_bytes(32);
        assert_eq!(bytes.len(), 32);
    }
}
