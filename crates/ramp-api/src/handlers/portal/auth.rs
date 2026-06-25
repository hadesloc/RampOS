//! Portal Authentication Handlers
//!
//! Endpoints for portal authentication.
//! - WebAuthn and magic-link remain fail-closed stubs (no native backend on Windows).
//! - Wallet login (SIWE / personal_sign) is fully wired.
//! - Session endpoints (`/session`, `/me`, `/refresh`, `/logout`) are wired to the DB + JWT.

pub mod identity;

use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
use axum_extra::extract::cookie::{Cookie, CookieJar};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation, Algorithm};
use ramp_aa::recover_personal_sign;
use ramp_common::types::{TenantId, UserId};
use ramp_core::repository::CreateSmartAccountRequest;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
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
        .route("/refresh", post(refresh_token))
        .route("/logout", post(logout))
        .route("/me", get(get_me))
        .route("/session", get(check_session))
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

    // 1. Parse address and nonce from the SIWE message
    let (msg_address, msg_nonce) = parse_siwe_message(&req.message)
        .map_err(|e| ApiError::Unauthorized(format!("Invalid SIWE message: {}", e)))?;

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

    // 4. Validate nonce: must exist, match address, be unused, not expired
    #[derive(sqlx::FromRow)]
    struct NonceRow {
        id: uuid::Uuid,
        used_at: Option<chrono::DateTime<Utc>>,
        expires_at: chrono::DateTime<Utc>,
        address: String,
    }

    let nonce_row: Option<NonceRow> = sqlx::query_as(
        "SELECT id, used_at, expires_at, address
           FROM portal_auth_nonces
          WHERE nonce = $1",
    )
    .bind(&msg_nonce)
    .fetch_optional(pool)
    .await
    .map_err(|e| ApiError::Internal(format!("Database error: {}", e)))?;

    let nonce_row = nonce_row
        .ok_or_else(|| ApiError::Unauthorized("Unknown nonce".to_string()))?;

    if nonce_row.address.to_lowercase() != msg_address {
        return Err(ApiError::Unauthorized("Nonce address mismatch".to_string()));
    }
    if nonce_row.used_at.is_some() {
        return Err(ApiError::Unauthorized("Nonce already used".to_string()));
    }
    if Utc::now() > nonce_row.expires_at {
        return Err(ApiError::Unauthorized("Nonce expired".to_string()));
    }

    // 5. Mark nonce as used (single-use)
    sqlx::query("UPDATE portal_auth_nonces SET used_at = NOW() WHERE id = $1")
        .bind(nonce_row.id)
        .execute(pool)
        .await
        .map_err(|e| ApiError::Internal(format!("Database error: {}", e)))?;

    // 6. Upsert portal user by (portal_tenant, lower(wallet_address))
    let portal_tenant_id = std::env::var("PORTAL_TENANT_ID")
        .unwrap_or_else(|_| PORTAL_TENANT_ID_DEFAULT.to_string());

    #[derive(sqlx::FromRow)]
    struct UserRow {
        id: String,
        kyc_status: String,
        kyc_tier: i16,
        status: String,
        created_at: chrono::DateTime<Utc>,
    }

    // Try to find existing user
    let existing: Option<UserRow> = sqlx::query_as(
        "SELECT id, kyc_status, kyc_tier, status, created_at
           FROM users
          WHERE tenant_id = $1
            AND lower(wallet_address) = lower($2)",
    )
    .bind(&portal_tenant_id)
    .bind(&msg_address)
    .fetch_optional(pool)
    .await
    .map_err(|e| ApiError::Internal(format!("Database error: {}", e)))?;

    let user_row = if let Some(u) = existing {
        u
    } else {
        // Create new portal user
        let new_id = Uuid::new_v4().to_string();
        let now = Utc::now();
        sqlx::query(
            "INSERT INTO users (
                id, tenant_id, kyc_tier, kyc_status, status,
                risk_flags, created_at, updated_at,
                email, wallet_address, auth_method
             ) VALUES ($1, $2, 0, 'PENDING', 'ACTIVE',
                       '[]'::jsonb, $3, $3,
                       '', $4, 'wallet')",
        )
        .bind(&new_id)
        .bind(&portal_tenant_id)
        .bind(now)
        .bind(&msg_address)
        .execute(pool)
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to create portal user: {}", e)))?;

        UserRow {
            id: new_id,
            kyc_status: "PENDING".to_string(),
            kyc_tier: 0,
            status: "ACTIVE".to_string(),
            created_at: now,
        }
    };

    // 7. Provision AA smart account (owner = wallet address)
    if let Some(ref aa_service) = app_state.aa_service {
        let tenant_id = TenantId::new(&portal_tenant_id);
        let user_id = UserId::new(&user_row.id);

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
                        user_id: user_row.id.clone(),
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

    // 8. Mint access JWT + opaque refresh token
    let now = Utc::now();
    let access_exp = now + Duration::seconds(ACCESS_TOKEN_EXPIRY_SECS);
    let access_claims = PortalClaims {
        sub: user_row.id.clone(),
        tenant_id: Some(portal_tenant_id.clone()),
        email: String::new(),
        iat: now.timestamp(),
        exp: access_exp.timestamp(),
        token_type: "access".to_string(),
    };

    let jwt_secret = app_state.portal_auth_config.jwt_secret.clone();
    let access_token = encode(
        &Header::default(),
        &access_claims,
        &EncodingKey::from_secret(jwt_secret.as_bytes()),
    )
    .map_err(|e| ApiError::Internal(format!("Failed to create access token: {}", e)))?;

    // Opaque refresh token — store sha256 hash in refresh_tokens table
    let refresh_raw = format!("prt_{}", Uuid::new_v4());
    let refresh_hash = sha256_bytes(refresh_raw.as_bytes());
    let refresh_exp = now + Duration::seconds(REFRESH_TOKEN_EXPIRY_SECS);
    let family_id = Uuid::new_v4();

    sqlx::query(
        "INSERT INTO refresh_tokens (token_hash, user_id, expires_at, family_id)
         VALUES ($1, $2, $3, $4)",
    )
    .bind(&refresh_hash)
    .bind(&user_row.id)
    .bind(refresh_exp)
    .bind(family_id)
    .execute(pool)
    .await
    .map_err(|e| ApiError::Internal(format!("Failed to store refresh token: {}", e)))?;

    // 9. Set cookies
    let is_production = std::env::var("RAMPOS_ENV").map(|v| v == "production").unwrap_or(false);
    let jar = set_auth_cookies(jar, &access_token, &refresh_raw, is_production);

    let auth_user = AuthUser {
        id: user_row.id.clone(),
        email: String::new(),
        kyc_status: user_row.kyc_status,
        kyc_tier: user_row.kyc_tier as i32,
        status: user_row.status,
        created_at: user_row.created_at.to_rfc3339(),
        wallet_address: msg_address,
    };

    info!(user_id = %user_row.id, "Portal wallet login successful");

    Ok((
        jar,
        Json(AuthResponse {
            user: auth_user,
            expires_at: access_exp.timestamp(),
        }),
    ))
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

/// POST /refresh — validate opaque refresh token cookie, issue new access token.
pub async fn refresh_token(
    State(app_state): State<AppState>,
    jar: CookieJar,
) -> Result<(CookieJar, Json<AuthResponse>), ApiError> {
    let refresh_val = jar
        .get(REFRESH_COOKIE_NAME)
        .filter(|c| !c.value().is_empty())
        .ok_or_else(|| ApiError::Unauthorized("No refresh token provided".to_string()))?
        .value()
        .to_string();

    let pool = app_state
        .db_pool
        .as_ref()
        .ok_or_else(|| ApiError::Unauthorized("No refresh token provided".to_string()))?;

    let token_hash = sha256_bytes(refresh_val.as_bytes());

    // Look up non-revoked, non-expired refresh token
    #[derive(sqlx::FromRow)]
    struct RefreshRow {
        user_id: String,
    }

    let row: Option<RefreshRow> = sqlx::query_as(
        "SELECT user_id
           FROM refresh_tokens
          WHERE token_hash = $1
            AND revoked = FALSE
            AND expires_at > NOW()",
    )
    .bind(&token_hash)
    .fetch_optional(pool)
    .await
    .map_err(|e| ApiError::Internal(format!("Database error: {}", e)))?;

    let row = row.ok_or_else(|| ApiError::Unauthorized("Invalid or expired refresh token".to_string()))?;

    // Fetch user from DB to populate claims
    let portal_tenant_id = std::env::var("PORTAL_TENANT_ID")
        .unwrap_or_else(|_| PORTAL_TENANT_ID_DEFAULT.to_string());

    #[derive(sqlx::FromRow)]
    struct UserRow2 {
        id: String,
        kyc_status: String,
        kyc_tier: i16,
        status: String,
        created_at: chrono::DateTime<Utc>,
        wallet_address: Option<String>,
    }

    let user_row: Option<UserRow2> = sqlx::query_as(
        "SELECT id, kyc_status, kyc_tier, status, created_at, wallet_address
           FROM users
          WHERE id = $1 AND tenant_id = $2",
    )
    .bind(&row.user_id)
    .bind(&portal_tenant_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| ApiError::Internal(format!("Database error: {}", e)))?;

    let user_row = user_row.ok_or_else(|| ApiError::Unauthorized("User not found".to_string()))?;

    // Revoke old refresh token (rotation)
    sqlx::query("UPDATE refresh_tokens SET revoked = TRUE WHERE token_hash = $1")
        .bind(&token_hash)
        .execute(pool)
        .await
        .ok();

    // Mint new tokens
    let now = Utc::now();
    let access_exp = now + Duration::seconds(ACCESS_TOKEN_EXPIRY_SECS);
    let access_claims = PortalClaims {
        sub: user_row.id.clone(),
        tenant_id: Some(portal_tenant_id),
        email: String::new(),
        iat: now.timestamp(),
        exp: access_exp.timestamp(),
        token_type: "access".to_string(),
    };

    let jwt_secret = app_state.portal_auth_config.jwt_secret.clone();
    let access_token = encode(
        &Header::default(),
        &access_claims,
        &EncodingKey::from_secret(jwt_secret.as_bytes()),
    )
    .map_err(|e| ApiError::Internal(format!("Failed to create access token: {}", e)))?;

    let refresh_raw = format!("prt_{}", Uuid::new_v4());
    let refresh_hash_new = sha256_bytes(refresh_raw.as_bytes());
    let refresh_exp = now + Duration::seconds(REFRESH_TOKEN_EXPIRY_SECS);
    let family_id = Uuid::new_v4();

    sqlx::query(
        "INSERT INTO refresh_tokens (token_hash, user_id, expires_at, family_id)
         VALUES ($1, $2, $3, $4)",
    )
    .bind(&refresh_hash_new)
    .bind(&user_row.id)
    .bind(refresh_exp)
    .bind(family_id)
    .execute(pool)
    .await
    .map_err(|e| ApiError::Internal(format!("Failed to store refresh token: {}", e)))?;

    let is_production = std::env::var("RAMPOS_ENV").map(|v| v == "production").unwrap_or(false);
    let jar = set_auth_cookies(jar, &access_token, &refresh_raw, is_production);

    let auth_user = AuthUser {
        id: user_row.id,
        email: String::new(),
        kyc_status: user_row.kyc_status,
        kyc_tier: user_row.kyc_tier as i32,
        status: user_row.status,
        created_at: user_row.created_at.to_rfc3339(),
        wallet_address: user_row.wallet_address.unwrap_or_default(),
    };

    Ok((
        jar,
        Json(AuthResponse {
            user: auth_user,
            expires_at: access_exp.timestamp(),
        }),
    ))
}

/// POST /logout — revoke refresh token in DB and clear cookies.
pub async fn logout(
    State(app_state): State<AppState>,
    jar: CookieJar,
) -> Result<CookieJar, ApiError> {
    info!("User logout requested");

    // Revoke refresh token if present and DB is available
    if let Some(pool) = app_state.db_pool.as_ref() {
        if let Some(cookie) = jar.get(REFRESH_COOKIE_NAME) {
            let token_hash = sha256_bytes(cookie.value().as_bytes());
            let _ = sqlx::query(
                "UPDATE refresh_tokens SET revoked = TRUE WHERE token_hash = $1 AND revoked = FALSE",
            )
            .bind(&token_hash)
            .execute(pool)
            .await;
        }
    }

    Ok(clear_auth_cookies(jar))
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
            kyc_status: String,
            kyc_tier: i16,
            status: String,
            created_at: chrono::DateTime<Utc>,
            wallet_address: Option<String>,
        }

        if let Ok(Some(row)) = sqlx::query_as::<_, Row>(
            "SELECT id, kyc_status, kyc_tier, status, created_at, wallet_address
               FROM users
              WHERE id = $1 AND tenant_id = $2",
        )
        .bind(&claims.sub)
        .bind(&portal_tenant_id)
        .fetch_optional(pool)
        .await
        {
            return AuthUser {
                id: row.id,
                email: claims.email.clone(),
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

/// Parse `address` and `nonce` from a raw EIP-4361 SIWE message.
///
/// Expected layout:
/// ```text
/// {domain} wants you to sign in with your Ethereum account:
/// {address}
///
/// ...
/// Nonce: {nonce}
/// ...
/// ```
fn parse_siwe_message(message: &str) -> Result<(String, String), String> {
    let lines: Vec<&str> = message.lines().collect();

    // Line 1 is the address after the "wants you to sign in" line.
    let address = lines
        .get(1)
        .map(|s| s.trim().to_lowercase())
        .filter(|s| s.starts_with("0x") && s.len() == 42)
        .ok_or_else(|| "Cannot parse Ethereum address from SIWE message".to_string())?;

    // Find "Nonce: " line
    let nonce = lines
        .iter()
        .find_map(|l| l.trim().strip_prefix("Nonce: "))
        .map(|s| s.trim().to_string())
        .ok_or_else(|| "Cannot parse Nonce from SIWE message".to_string())?;

    Ok((address, nonce))
}

fn set_auth_cookies(jar: CookieJar, access_token: &str, refresh_token: &str, secure: bool) -> CookieJar {
    let mut auth = Cookie::build((AUTH_COOKIE_NAME.to_string(), access_token.to_string()))
        .path("/")
        .http_only(true)
        .same_site(axum_extra::extract::cookie::SameSite::Lax)
        .max_age(time::Duration::seconds(ACCESS_TOKEN_EXPIRY_SECS));

    if secure {
        auth = auth.secure(true);
    }

    let mut refresh = Cookie::build((REFRESH_COOKIE_NAME.to_string(), refresh_token.to_string()))
        .path("/")
        .http_only(true)
        .same_site(axum_extra::extract::cookie::SameSite::Lax)
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

/// SHA-256 digest as raw bytes stored in `BYTEA`.
fn sha256_bytes(input: &[u8]) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.update(input);
    hasher.finalize().to_vec()
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

    #[test]
    fn test_parse_siwe_message_valid() {
        let msg = "localhost:3000 wants you to sign in with your Ethereum account:\n\
                   0xabcdef1234567890abcdef1234567890abcdef12\n\
                   \n\
                   Sign in to RampOS Portal.\n\
                   \n\
                   URI: http://localhost:3000\n\
                   Version: 1\n\
                   Chain ID: 137\n\
                   Nonce: AbCdEfGhIjKlMnOp\n\
                   Issued At: 2025-01-01T00:00:00Z";

        let (addr, nonce) = parse_siwe_message(msg).unwrap();
        assert_eq!(addr, "0xabcdef1234567890abcdef1234567890abcdef12");
        assert_eq!(nonce, "AbCdEfGhIjKlMnOp");
    }

    #[test]
    fn test_parse_siwe_message_missing_nonce() {
        let msg = "localhost:3000 wants you to sign in with your Ethereum account:\n\
                   0xabcdef1234567890abcdef1234567890abcdef12\n\
                   \n\
                   Sign in to RampOS Portal.";
        assert!(parse_siwe_message(msg).is_err());
    }
}
