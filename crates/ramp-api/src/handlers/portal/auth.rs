//! Portal Authentication Handlers
//!
//! Endpoints for portal authentication.
//! - WebAuthn and magic-link remain fail-closed stubs (no native backend on Windows).
//! - Wallet login (SIWE / personal_sign) is fully wired.
//! - Session endpoints (`/session`, `/me`, `/refresh`, `/logout`) are wired to the DB + JWT.

pub mod identity;
pub mod password;
pub mod session;
pub mod siwe;

use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
use axum_extra::extract::cookie::{Cookie, CookieJar};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, DecodingKey, Validation, Algorithm};
use ramp_aa::recover_personal_sign;
use ramp_common::types::{TenantId, UserId};
use ramp_core::repository::CreateSmartAccountRequest;
use serde::{Deserialize, Serialize};
use tracing::{info, warn};
use uuid::Uuid;
use validator::Validate;

use crate::error::ApiError;
use crate::middleware::PortalClaims;
use crate::router::AppState;

// ============================================================================
// Constants
// ============================================================================

const AUTH_COOKIE_NAME: &str = "auth_token";
const REFRESH_COOKIE_NAME: &str = "refresh_token";

/// Access token lifetime (30 minutes).
const ACCESS_TOKEN_EXPIRY_SECS: i64 = 1800;
/// Refresh token lifetime (7 days).
const REFRESH_TOKEN_EXPIRY_SECS: i64 = 604800;

/// Fixed portal tenant UUID.
const PORTAL_TENANT_ID_DEFAULT: &str = "11111111-1111-1111-1111-111111111111";

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
    pub wallet_address: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthResponse {
    pub user: AuthUser,
    pub expires_at: i64,
}

// ---- WebAuthn DTOs (unchanged stubs) ----

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

// ---- Wallet SIWE DTOs ----

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WalletNonceRequest {
    pub address: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WalletNonceResponse {
    pub nonce: String,
    pub message: String,
    pub expires_at: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WalletVerifyRequest {
    pub message: String,
    pub signature: String,
}

// ---- Session DTO ----

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionStatus {
    pub authenticated: bool,
    pub user: Option<AuthUser>,
}

// ============================================================================
// Router
// ============================================================================

pub fn router() -> Router<AppState> {
    Router::new()
        // Core password authentication
        .route("/register", post(password::register))
        .route("/login", post(password::login))
        // WebAuthn endpoints (stubs)
        .route("/webauthn/register/challenge", post(webauthn_register_challenge))
        .route("/webauthn/register/complete", post(webauthn_register_complete))
        .route("/webauthn/login/challenge", post(webauthn_login_challenge))
        .route("/webauthn/login/complete", post(webauthn_login_complete))
        // Magic link endpoints (stubs)
        .route("/magic-link", post(request_magic_link))
        .route("/magic-link/verify", post(verify_magic_link))
        // Wallet SIWE endpoints
        .route("/wallet/nonce", post(wallet_nonce))
        .route("/wallet/verify", post(wallet_verify))
        // Session endpoints (real)
        .route("/refresh", post(session::refresh))
        .route("/logout", post(session::logout))
        .route("/me", get(get_me))
        .route("/session", get(check_session))
}

pub fn protected_router() -> Router<AppState> {
    Router::new()
        .route("/wallet/link/nonce", post(siwe::wallet_link_nonce))
        .route("/wallet/link/verify", post(siwe::wallet_link_verify))
        .route("/logout-all", post(session::logout_all))
}

// ============================================================================
// WebAuthn stubs (unchanged from original)
// ============================================================================

pub async fn webauthn_register_challenge(
    State(_app_state): State<AppState>,
    Json(req): Json<WebAuthnChallengeRequest>,
) -> Result<Json<WebAuthnChallenge>, ApiError> {
    req.validate().map_err(|e| ApiError::Validation(e.to_string()))?;
    info!(email = %req.email, "WebAuthn registration challenge requested");

    let challenge = base64_url_encode(&generate_random_bytes(32));
    let user_id = Uuid::new_v4().to_string();

    Ok(Json(WebAuthnChallenge {
        challenge,
        rp_id: std::env::var("WEBAUTHN_RP_ID").unwrap_or_else(|_| "localhost".to_string()),
        rp_name: std::env::var("WEBAUTHN_RP_NAME")
            .unwrap_or_else(|_| "RampOS Portal".to_string()),
        user_id: base64_url_encode(user_id.as_bytes()),
        user_name: req.email.clone(),
        user_display_name: req.email.split('@').next().unwrap_or(&req.email).to_string(),
        timeout: 60000,
        attestation: "none".to_string(),
        authenticator_selection: AuthenticatorSelection {
            authenticator_attachment: Some("platform".to_string()),
            resident_key: "preferred".to_string(),
            user_verification: "required".to_string(),
        },
        pub_key_cred_params: vec![
            PubKeyCredParam { credential_type: "public-key".to_string(), alg: -7 },
            PubKeyCredParam { credential_type: "public-key".to_string(), alg: -257 },
        ],
        exclude_credentials: vec![],
    }))
}

pub async fn webauthn_register_complete(
    State(_app_state): State<AppState>,
    _jar: CookieJar,
    Json(req): Json<WebAuthnRegisterCompleteRequest>,
) -> Result<(CookieJar, Json<AuthResponse>), ApiError> {
    req.validate().map_err(|e| ApiError::Validation(e.to_string()))?;
    info!(email = %req.email, credential_id = %req.credential.id, "WebAuthn registration completion not implemented");
    Err(ApiError::Unauthorized(
        "WebAuthn registration completion is not available".to_string(),
    ))
}

pub async fn webauthn_login_challenge(
    State(_app_state): State<AppState>,
    Json(req): Json<Option<WebAuthnChallengeRequest>>,
) -> Result<Json<WebAuthnChallenge>, ApiError> {
    let email = req.map(|r| r.email).unwrap_or_default();
    info!(email = %email, "WebAuthn login challenge requested");

    let challenge = base64_url_encode(&generate_random_bytes(32));

    Ok(Json(WebAuthnChallenge {
        challenge,
        rp_id: std::env::var("WEBAUTHN_RP_ID").unwrap_or_else(|_| "localhost".to_string()),
        rp_name: std::env::var("WEBAUTHN_RP_NAME")
            .unwrap_or_else(|_| "RampOS Portal".to_string()),
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
            PubKeyCredParam { credential_type: "public-key".to_string(), alg: -7 },
            PubKeyCredParam { credential_type: "public-key".to_string(), alg: -257 },
        ],
        exclude_credentials: vec![],
    }))
}

pub async fn webauthn_login_complete(
    State(_app_state): State<AppState>,
    _jar: CookieJar,
    Json(req): Json<WebAuthnLoginCompleteRequest>,
) -> Result<(CookieJar, Json<AuthResponse>), ApiError> {
    info!(credential_id = %req.credential.id, "WebAuthn login completion not implemented");
    Err(ApiError::Unauthorized(
        "WebAuthn login completion is not available".to_string(),
    ))
}

pub async fn request_magic_link(
    State(_app_state): State<AppState>,
    Json(req): Json<MagicLinkRequest>,
) -> Result<Json<MagicLinkResponse>, ApiError> {
    req.validate().map_err(|e| ApiError::Validation(e.to_string()))?;
    info!(email = %req.email, "Magic link requested");
    Ok(Json(MagicLinkResponse {
        message: "If an account exists with this email, a login link has been sent.".to_string(),
    }))
}

pub async fn verify_magic_link(
    State(_app_state): State<AppState>,
    _jar: CookieJar,
    Json(req): Json<MagicLinkVerifyRequest>,
) -> Result<(CookieJar, Json<AuthResponse>), ApiError> {
    req.validate().map_err(|e| ApiError::Validation(e.to_string()))?;
    info!("Magic link verification not implemented");
    Err(ApiError::Unauthorized(
        "Magic link verification is not available".to_string(),
    ))
}

// ============================================================================
// Wallet SIWE handlers
// ============================================================================

/// POST /wallet/nonce — issue a SIWE nonce for the given wallet address.
pub async fn wallet_nonce(
    State(app_state): State<AppState>,
    Json(req): Json<WalletNonceRequest>,
) -> Result<Json<WalletNonceResponse>, ApiError> {
    // Validate address format: must start with 0x and be 42 chars total
    let address = req.address.trim().to_lowercase();
    if !address.starts_with("0x") || address.len() != 42 {
        return Err(ApiError::Validation(
            "address must be a 0x-prefixed 40-hex Ethereum address".to_string(),
        ));
    }

    let pool = app_state
        .db_pool
        .as_ref()
        .ok_or_else(|| ApiError::Internal("Database not configured".to_string()))?;

    let domain = std::env::var("PORTAL_SIWE_DOMAIN")
        .unwrap_or_else(|_| "localhost:3000".to_string());
    let uri = std::env::var("PORTAL_SIWE_URI")
        .unwrap_or_else(|_| "http://localhost:3000".to_string());
    let chain_id: u64 = app_state
        .aa_service
        .as_ref()
        .map(|aa| aa.chain_config.chain_id)
        .unwrap_or(137);

    // Generate ≥16-char base62 nonce
    let nonce = generate_base62_nonce(24);
    let now = Utc::now();
    let expires_at = now + Duration::minutes(10);

    // Build EIP-4361 message
    let issued_at_str = now.format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let message = format!(
        "{domain} wants you to sign in with your Ethereum account:\n\
         {address}\n\
         \n\
         Sign in to RampOS Portal.\n\
         \n\
         URI: {uri}\n\
         Version: 1\n\
         Chain ID: {chain_id}\n\
         Nonce: {nonce}\n\
         Issued At: {issued_at_str}",
        domain = domain,
        address = address,
        uri = uri,
        chain_id = chain_id,
        nonce = nonce,
        issued_at_str = issued_at_str,
    );

    // Delete any existing unused nonces for this address (keep table clean)
    let _ = sqlx::query(
        "DELETE FROM portal_auth_nonces WHERE address = $1 AND used_at IS NULL"
    )
    .bind(&address)
    .execute(pool)
    .await;

    // Store nonce
    sqlx::query(
        "INSERT INTO portal_auth_nonces (address, nonce, domain, issued_at, expires_at)
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(&address)
    .bind(&nonce)
    .bind(&domain)
    .bind(now)
    .bind(expires_at)
    .execute(pool)
    .await
    .map_err(|e| ApiError::Internal(format!("Failed to store nonce: {}", e)))?;

    info!(address = %address, "SIWE nonce issued");

    Ok(Json(WalletNonceResponse {
        nonce,
        message,
        expires_at: expires_at.timestamp(),
    }))
}

/// POST /wallet/verify — verify the signed SIWE message and issue a session.
pub async fn wallet_verify(
    State(app_state): State<AppState>,
    jar: CookieJar,
    Json(req): Json<WalletVerifyRequest>,
) -> Result<(CookieJar, Json<AuthResponse>), ApiError> {
    let pool = app_state
        .db_pool
        .as_ref()
        .ok_or_else(|| ApiError::Internal("Database not configured".to_string()))?;

    let domain = std::env::var("PORTAL_SIWE_DOMAIN")
        .unwrap_or_else(|_| "localhost:3000".to_string());
    let uri = std::env::var("PORTAL_SIWE_URI")
        .unwrap_or_else(|_| "http://localhost:3000".to_string());
    let chain_id = app_state
        .aa_service
        .as_ref()
        .map(|aa| aa.chain_config.chain_id)
        .unwrap_or(137);
    let validation = siwe::SiweValidationConfig {
        domain: domain.clone(),
        uri,
        chain_id,
        max_age: Duration::minutes(10),
        clock_skew: Duration::seconds(30),
    };
    let parsed = siwe::parse_and_validate_message(&req.message, &validation, Utc::now())
        .map_err(|e| ApiError::Unauthorized(format!("Invalid SIWE message: {}", e)))?;
    let msg_address = parsed.address;
    let msg_nonce = parsed.nonce;

    // 2. Decode hex signature (0x-prefixed 130-hex = 65 bytes)
    let sig_hex = req.signature.trim_start_matches("0x");
    if sig_hex.len() != 130 {
        return Err(ApiError::Unauthorized(
            "Signature must be 0x-prefixed 65-byte hex (130 hex chars)".to_string(),
        ));
    }
    let sig_bytes = hex::decode(sig_hex)
        .map_err(|_| ApiError::Unauthorized("Invalid signature hex encoding".to_string()))?;

    // 3. Recover signer from personal_sign
    let recovered = recover_personal_sign(req.message.as_bytes(), &sig_bytes)
        .map_err(|e| ApiError::Unauthorized(format!("Signature recovery failed: {}", e)))?;
    let recovered_str = format!("{:?}", recovered).to_lowercase();

    if recovered_str != msg_address {
        warn!(
            recovered = %recovered_str,
            claimed  = %msg_address,
            "SIWE: recovered address does not match message address"
        );
        return Err(ApiError::Unauthorized(
            "Signature does not match the claimed address".to_string(),
        ));
    }

    // Consume the nonce atomically so concurrent verification has one winner.
    let consumed_nonce: Option<Uuid> = sqlx::query_scalar(
        r#"
        UPDATE portal_auth_nonces
        SET used_at = NOW()
        WHERE nonce = $1
          AND lower(address) = $2
          AND domain = $3
          AND purpose = 'login'
          AND portal_user_id IS NULL
          AND used_at IS NULL
          AND expires_at > NOW()
        RETURNING id
        "#,
    )
    .bind(&msg_nonce)
    .bind(&msg_address)
    .bind(&domain)
    .fetch_optional(pool)
    .await
    .map_err(|e| ApiError::Internal(format!("Database error: {}", e)))?;

    if consumed_nonce.is_none() {
        return Err(ApiError::Unauthorized(
            "Invalid, expired, or already used nonce".to_string(),
        ));
    }

    let portal_tenant_id = std::env::var("PORTAL_TENANT_ID")
        .unwrap_or_else(|_| PORTAL_TENANT_ID_DEFAULT.to_string());
    let identity = identity::resolve_or_create_wallet_identity(
        pool,
        &portal_tenant_id,
        &msg_address,
    )
    .await
    .map_err(|e| ApiError::Internal(format!("Failed to resolve wallet identity: {}", e)))?;
    if identity.status != "ACTIVE" {
        return Err(ApiError::Unauthorized(
            "Wallet identity is not active".to_string(),
        ));
    }

    // Provision AA smart account against the linked financial identity.
    if let Some(ref aa_service) = app_state.aa_service {
        let tenant_id = TenantId::new(&portal_tenant_id);
        let user_id = UserId::new(&identity.financial_user_id);

        // Parse wallet address as alloy Address
        let owner_addr: alloy_primitives::Address = recovered;

        match aa_service
            .smart_account_service
            .get_or_create_account(&tenant_id, &user_id, owner_addr)
            .await
        {
            Ok(account) => {
                if let Some(ref repo) = aa_service.smart_account_repo {
                    let create_req = CreateSmartAccountRequest {
                        tenant_id: portal_tenant_id.clone(),
                        user_id: identity.financial_user_id.clone(),
                        address: format!("{:?}", account.address),
                        owner_address: format!("{:?}", account.owner),
                        account_type: format!("{:?}", account.account_type),
                        chain_id: aa_service.chain_config.chain_id,
                        factory_address: Some(format!(
                            "{:?}",
                            aa_service.chain_config.entry_point_address
                        )),
                        entry_point_address: Some(format!(
                            "{:?}",
                            aa_service.chain_config.entry_point_address
                        )),
                    };
                    if let Err(e) = repo.create(&create_req).await {
                        warn!(error = %e, "Failed to persist portal smart account mapping");
                    }
                }
            }
            Err(e) => {
                warn!(error = %e, "Failed to provision portal smart account; continuing login");
            }
        }
    }

    info!(
        portal_user_id = %identity.portal_user_id,
        financial_user_id = %identity.financial_user_id,
        "Portal wallet login successful"
    );
    session::issue_session(&app_state, jar, &identity).await
}

// ============================================================================
// Session endpoints (real logic)
// ============================================================================

/// GET /session — validate the auth_token cookie and return session status.
pub async fn check_session(
    State(app_state): State<AppState>,
    jar: CookieJar,
) -> Json<SessionStatus> {
    let token = match jar.get(AUTH_COOKIE_NAME) {
        Some(c) if !c.value().is_empty() => c.value().to_string(),
        _ => {
            return Json(SessionStatus { authenticated: false, user: None });
        }
    };

    match decode_portal_jwt(&token, &app_state.portal_auth_config.jwt_secret) {
        Ok(claims) => {
            // Optionally look up the user from DB; fall back to JWT claims if no DB
            let user = build_auth_user_from_claims(&claims, &app_state).await;
            Json(SessionStatus {
                authenticated: true,
                user: Some(user),
            })
        }
        Err(_) => Json(SessionStatus { authenticated: false, user: None }),
    }
}

/// GET /me — return current user; 401 if not authenticated.
pub async fn get_me(
    State(app_state): State<AppState>,
    jar: CookieJar,
) -> Result<Json<AuthUser>, ApiError> {
    let token = jar
        .get(AUTH_COOKIE_NAME)
        .filter(|c| !c.value().is_empty())
        .ok_or_else(|| ApiError::Unauthorized("Not authenticated".to_string()))?
        .value()
        .to_string();

    let claims = decode_portal_jwt(&token, &app_state.portal_auth_config.jwt_secret)
        .map_err(|_| ApiError::Unauthorized("Invalid or expired session".to_string()))?;

    let user = build_auth_user_from_claims(&claims, &app_state).await;
    Ok(Json(user))
}

// ============================================================================
// Internal helpers
// ============================================================================

/// Decode and validate a portal access JWT.
fn decode_portal_jwt(token: &str, secret: &str) -> Result<PortalClaims, String> {
    let mut validation = Validation::new(Algorithm::HS256);
    validation.validate_exp = true;

    let token_data = decode::<PortalClaims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &validation,
    )
    .map_err(|e| format!("Invalid token: {}", e))?;

    let claims = token_data.claims;
    if claims.token_type != "access" {
        return Err("Not an access token".to_string());
    }
    Ok(claims)
}

/// Build an AuthUser from JWT claims, optionally enriched from DB.
async fn build_auth_user_from_claims(claims: &PortalClaims, app_state: &AppState) -> AuthUser {
    // Try DB lookup for full info
    if let Some(pool) = app_state.db_pool.as_ref() {
        let portal_tenant_id = std::env::var("PORTAL_TENANT_ID")
            .unwrap_or_else(|_| PORTAL_TENANT_ID_DEFAULT.to_string());

        #[derive(sqlx::FromRow)]
        struct Row {
            id: String,
            email: Option<String>,
            kyc_status: String,
            kyc_tier: i16,
            status: String,
            created_at: chrono::DateTime<Utc>,
            wallet_address: Option<String>,
        }

        if let Ok(Some(row)) = sqlx::query_as::<_, Row>(
            "SELECT
                portal.id,
                portal.email,
                financial.kyc_status,
                financial.kyc_tier,
                financial.status,
                portal.created_at,
                portal.wallet_address
               FROM portal_users portal
               JOIN users financial
                 ON financial.tenant_id = portal.tenant_id
                AND financial.id = portal.financial_user_id
              WHERE portal.id = $1 AND portal.tenant_id = $2",
        )
        .bind(&claims.sub)
        .bind(&portal_tenant_id)
        .fetch_optional(pool)
        .await
        {
            return AuthUser {
                id: row.id,
                email: row.email.unwrap_or_else(|| claims.email.clone()),
                kyc_status: row.kyc_status,
                kyc_tier: row.kyc_tier as i32,
                status: row.status,
                created_at: row.created_at.to_rfc3339(),
                wallet_address: row.wallet_address.unwrap_or_default(),
            };
        }
    }

    // Fallback: return minimal info from claims
    AuthUser {
        id: claims.sub.clone(),
        email: claims.email.clone(),
        kyc_status: "UNKNOWN".to_string(),
        kyc_tier: 0,
        status: "ACTIVE".to_string(),
        created_at: String::new(),
        wallet_address: String::new(),
    }
}

fn set_auth_cookies(jar: CookieJar, access_token: &str, refresh_token: &str, secure: bool) -> CookieJar {
    let mut auth = Cookie::build((AUTH_COOKIE_NAME.to_string(), access_token.to_string()))
        .path("/")
        .http_only(true)
        .same_site(axum_extra::extract::cookie::SameSite::Strict)
        .max_age(time::Duration::seconds(ACCESS_TOKEN_EXPIRY_SECS));

    if secure {
        auth = auth.secure(true);
    }

    let mut refresh = Cookie::build((REFRESH_COOKIE_NAME.to_string(), refresh_token.to_string()))
        .path("/")
        .http_only(true)
        .same_site(axum_extra::extract::cookie::SameSite::Strict)
        .max_age(time::Duration::seconds(REFRESH_TOKEN_EXPIRY_SECS));

    if secure {
        refresh = refresh.secure(true);
    }

    jar.add(auth.build()).add(refresh.build())
}

fn clear_auth_cookies(jar: CookieJar) -> CookieJar {
    let auth = Cookie::build((AUTH_COOKIE_NAME.to_string(), String::new()))
        .path("/")
        .http_only(true)
        .max_age(time::Duration::seconds(0))
        .build();

    let refresh = Cookie::build((REFRESH_COOKIE_NAME.to_string(), String::new()))
        .path("/")
        .http_only(true)
        .max_age(time::Duration::seconds(0))
        .build();

    jar.add(auth).add(refresh)
}

/// Generate a random base62 string of the given length.
fn generate_base62_nonce(len: usize) -> String {
    use rand::Rng;
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    let mut rng = rand::thread_rng();
    (0..len)
        .map(|_| ALPHABET[rng.gen::<usize>() % ALPHABET.len()] as char)
        .collect()
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

// ============================================================================
// Tests
// ============================================================================

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
    fn test_base62_nonce_length() {
        let nonce = generate_base62_nonce(24);
        assert_eq!(nonce.len(), 24);
        assert!(nonce.chars().all(|c| c.is_ascii_alphanumeric()));
    }

}
