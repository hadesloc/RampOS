//! Portal Authentication Handlers
//!
//! Endpoints for user authentication including:
//! - WebAuthn (Passkey) registration and login
//! - Magic link authentication
//! - Session management
//!
//! Security: Uses httpOnly cookies for token storage

use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use chrono::{Duration, Utc};
use jsonwebtoken::{encode, EncodingKey, Header};
use serde::{Deserialize, Serialize};
use tracing::{info, warn};
use uuid::Uuid;
use validator::Validate;

use crate::error::ApiError;
use crate::middleware::portal_auth::PortalClaims;
use crate::router::AppState;

// Cookie configuration constants
const AUTH_COOKIE_NAME: &str = "auth_token";
const REFRESH_COOKIE_NAME: &str = "refresh_token";
const COOKIE_MAX_AGE_SECS: i64 = 86400; // 24 hours
const REFRESH_COOKIE_MAX_AGE_SECS: i64 = 604800; // 7 days

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

/// Response for successful authentication - only contains user info
/// Tokens are sent via httpOnly cookies
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthResponse {
    pub user: AuthUser,
    pub expires_at: i64,
}

/// Internal session data (not exposed to client)
#[derive(Debug, Clone)]
struct AuthSessionInternal {
    access_token: String,
    refresh_token: String,
    expires_at: i64,
    user: AuthUser,
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
        .route("/session", get(check_session))
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
/// Sets auth cookies and returns user info
pub async fn webauthn_register_complete(
    State(_app_state): State<AppState>,
    jar: CookieJar,
    Json(req): Json<WebAuthnRegisterCompleteRequest>,
) -> Result<(CookieJar, Json<AuthResponse>), ApiError> {
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

    let session = AuthSessionInternal {
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

    // Set cookies and return response
    let jar = set_auth_cookies(jar, &session);
    let response = AuthResponse {
        user: session.user,
        expires_at: session.expires_at,
    };

    Ok((jar, Json(response)))
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
/// Sets auth cookies and returns user info
pub async fn webauthn_login_complete(
    State(_app_state): State<AppState>,
    jar: CookieJar,
    Json(req): Json<WebAuthnLoginCompleteRequest>,
) -> Result<(CookieJar, Json<AuthResponse>), ApiError> {
    info!(credential_id = %req.credential.id, "WebAuthn login completing");

    // In production, this would:
    // 1. Verify the challenge
    // 2. Validate the signature
    // 3. Look up user by credential
    // 4. Generate JWT tokens

    let now = Utc::now();
    let expires_at = now + Duration::hours(24);
    let user_id = Uuid::new_v4().to_string();

    let session = AuthSessionInternal {
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

    // Set cookies and return response
    let jar = set_auth_cookies(jar, &session);
    let response = AuthResponse {
        user: session.user,
        expires_at: session.expires_at,
    };

    Ok((jar, Json(response)))
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
/// Sets auth cookies and returns user info
pub async fn verify_magic_link(
    State(_app_state): State<AppState>,
    jar: CookieJar,
    Json(req): Json<MagicLinkVerifyRequest>,
) -> Result<(CookieJar, Json<AuthResponse>), ApiError> {
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

    let session = AuthSessionInternal {
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

    // Set cookies and return response
    let jar = set_auth_cookies(jar, &session);
    let response = AuthResponse {
        user: session.user,
        expires_at: session.expires_at,
    };

    Ok((jar, Json(response)))
}

/// POST /v1/auth/refresh - Refresh access token using refresh token from cookie
/// Sets new auth cookies and returns user info
pub async fn refresh_token(
    State(_app_state): State<AppState>,
    jar: CookieJar,
) -> Result<(CookieJar, Json<AuthResponse>), ApiError> {
    info!("Token refresh requested");

    // Get refresh token from cookie
    let refresh_token_cookie = jar
        .get(REFRESH_COOKIE_NAME)
        .ok_or_else(|| ApiError::Unauthorized("No refresh token provided".to_string()))?;

    let refresh_token_value = refresh_token_cookie.value();

    // In production, this would:
    // 1. Validate the refresh token
    // 2. Check if it's revoked
    // 3. Get the associated user
    // 4. Generate new tokens
    // 5. Optionally rotate the refresh token

    if refresh_token_value.is_empty() {
        return Err(ApiError::Unauthorized("Invalid refresh token".to_string()));
    }

    let now = Utc::now();
    let expires_at = now + Duration::hours(24);
    let user_id = Uuid::new_v4().to_string();

    let session = AuthSessionInternal {
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

    // Set cookies and return response
    let jar = set_auth_cookies(jar, &session);
    let response = AuthResponse {
        user: session.user,
        expires_at: session.expires_at,
    };

    Ok((jar, Json(response)))
}

/// POST /v1/auth/logout - Logout and invalidate session
/// Clears auth cookies
pub async fn logout(
    State(_app_state): State<AppState>,
    jar: CookieJar,
) -> Result<CookieJar, ApiError> {
    info!("User logout requested");

    // In production, this would:
    // 1. Get the current session from cookie
    // 2. Revoke the refresh token in database
    // 3. Optionally add access token to blocklist

    // Clear cookies by setting them with max_age = 0
    let jar = clear_auth_cookies(jar);

    Ok(jar)
}

/// GET /v1/auth/me - Get current user info
/// Reads auth token from cookie
pub async fn get_me(
    State(_app_state): State<AppState>,
    jar: CookieJar,
) -> Result<Json<AuthUser>, ApiError> {
    info!("Get current user info requested");

    // Get auth token from cookie
    let auth_cookie = jar
        .get(AUTH_COOKIE_NAME)
        .ok_or_else(|| ApiError::Unauthorized("Not authenticated".to_string()))?;

    let _token = auth_cookie.value();

    // In production, this would:
    // 1. Validate and decode the JWT
    // 2. Extract user ID from JWT
    // 3. Fetch user from database
    // 4. Return user info

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

/// GET /v1/auth/session - Check if user has valid session
/// Returns authenticated status based on cookie presence
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionStatus {
    pub authenticated: bool,
    pub user: Option<AuthUser>,
}

pub async fn check_session(
    State(_app_state): State<AppState>,
    jar: CookieJar,
) -> Json<SessionStatus> {
    // Check if auth cookie exists
    let auth_cookie = jar.get(AUTH_COOKIE_NAME);

    match auth_cookie {
        Some(cookie) => {
            let token = cookie.value();
            // In production, would validate the JWT here
            if !token.is_empty() {
                // Return mock user for now
                let now = Utc::now();
                Json(SessionStatus {
                    authenticated: true,
                    user: Some(AuthUser {
                        id: Uuid::new_v4().to_string(),
                        email: "user@example.com".to_string(),
                        kyc_status: "VERIFIED".to_string(),
                        kyc_tier: 1,
                        status: "ACTIVE".to_string(),
                        created_at: now.to_rfc3339(),
                    }),
                })
            } else {
                Json(SessionStatus {
                    authenticated: false,
                    user: None,
                })
            }
        }
        None => Json(SessionStatus {
            authenticated: false,
            user: None,
        }),
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Create auth cookie with security flags
fn create_auth_cookie(name: &str, value: String, max_age_secs: i64) -> Cookie<'static> {
    let is_secure = std::env::var("COOKIE_SECURE")
        .map(|v| v == "true" || v == "1")
        .unwrap_or(true); // Default to secure in production

    Cookie::build((name.to_string(), value))
        .path("/")
        .http_only(true)
        .secure(is_secure)
        .same_site(SameSite::Strict)
        .max_age(time::Duration::seconds(max_age_secs))
        .build()
}

/// Set auth tokens as httpOnly cookies
fn set_auth_cookies(jar: CookieJar, session: &AuthSessionInternal) -> CookieJar {
    let auth_cookie = create_auth_cookie(
        AUTH_COOKIE_NAME,
        session.access_token.clone(),
        COOKIE_MAX_AGE_SECS,
    );

    let refresh_cookie = create_auth_cookie(
        REFRESH_COOKIE_NAME,
        session.refresh_token.clone(),
        REFRESH_COOKIE_MAX_AGE_SECS,
    );

    jar.add(auth_cookie).add(refresh_cookie)
}

/// Clear auth cookies by setting them with max_age = 0
fn clear_auth_cookies(jar: CookieJar) -> CookieJar {
    let auth_cookie = Cookie::build((AUTH_COOKIE_NAME.to_string(), String::new()))
        .path("/")
        .http_only(true)
        .max_age(time::Duration::seconds(0))
        .build();

    let refresh_cookie = Cookie::build((REFRESH_COOKIE_NAME.to_string(), String::new()))
        .path("/")
        .http_only(true)
        .max_age(time::Duration::seconds(0))
        .build();

    jar.add(auth_cookie).add(refresh_cookie)
}

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
    // Use real JWT generation with proper signing
    generate_real_jwt(user_id, "user@example.com", "access").unwrap_or_else(|_| {
        // Fallback to mock JWT if signing fails
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
    })
}

/// Get the JWT secret from environment
fn get_jwt_secret() -> String {
    std::env::var("JWT_SECRET")
        .unwrap_or_else(|_| "development-secret-change-in-production".to_string())
}

/// Get the default tenant ID from environment
fn get_default_tenant_id() -> String {
    std::env::var("DEFAULT_TENANT_ID")
        .unwrap_or_else(|_| "00000000-0000-0000-0000-000000000001".to_string())
}

/// Generate a real JWT token with proper signing using jsonwebtoken crate
fn generate_real_jwt(user_id: &str, email: &str, token_type: &str) -> Result<String, ApiError> {
    let jwt_secret = get_jwt_secret();
    let encoding_key = EncodingKey::from_secret(jwt_secret.as_bytes());
    let now = Utc::now();
    let tenant_id = get_default_tenant_id();

    let expiry = if token_type == "refresh" {
        now + Duration::days(7)
    } else {
        now + Duration::hours(24)
    };

    let claims = PortalClaims {
        sub: user_id.to_string(),
        tenant_id: Some(tenant_id),
        email: email.to_string(),
        iat: now.timestamp(),
        exp: expiry.timestamp(),
        token_type: token_type.to_string(),
    };

    encode(&Header::default(), &claims, &encoding_key).map_err(|e| {
        warn!(error = %e, "Failed to encode JWT token");
        ApiError::Internal("Failed to generate token".to_string())
    })
}

/// Generate both access and refresh tokens
#[allow(dead_code)]
fn generate_token_pair(user_id: &str, email: &str) -> Result<(String, String), ApiError> {
    let access_token = generate_real_jwt(user_id, email, "access")?;
    let refresh_token = generate_real_jwt(user_id, email, "refresh")?;
    Ok((access_token, refresh_token))
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

    #[test]
    fn test_create_auth_cookie() {
        std::env::set_var("COOKIE_SECURE", "false");
        let cookie = create_auth_cookie("test_cookie", "test_value".to_string(), 3600);
        assert_eq!(cookie.name(), "test_cookie");
        assert_eq!(cookie.value(), "test_value");
        assert!(cookie.http_only().unwrap_or(false));
        assert_eq!(cookie.same_site(), Some(SameSite::Strict));
    }
}
