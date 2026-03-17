//! Admin authentication handlers — JWT-based admin sessions
//!
//! Replaces the shared RAMPOS_ADMIN_KEY with per-admin JWT tokens.
//! Supports:
//! - `POST /v1/admin/auth/login`   — email + password → access + refresh tokens
//! - `POST /v1/admin/auth/refresh` — refresh token → new access token
//! - `POST /v1/admin/auth/logout`  — revoke refresh token

use axum::{extract::State, http::HeaderMap, Json};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use tracing::{info, warn};

use crate::error::ApiError;

// ============================================================================
// Configuration
// ============================================================================

/// JWT signing secret — reads from `RAMPOS_ADMIN_JWT_SECRET`.
///
/// SECURITY: fail closed if the explicit JWT secret is missing. A public fallback
/// or implicit reuse of the legacy admin key would make Bearer admin tokens
/// forgeable across the entire admin surface.
fn jwt_secret() -> Result<String, ApiError> {
    std::env::var("RAMPOS_ADMIN_JWT_SECRET").map_err(|_| {
        ApiError::Internal(
            "Admin JWT secret not configured. Set RAMPOS_ADMIN_JWT_SECRET.".to_string(),
        )
    })
}

const ACCESS_TOKEN_EXPIRY_MINUTES: i64 = 30;
const REFRESH_TOKEN_EXPIRY_DAYS: i64 = 7;
const MAX_FAILED_LOGIN_ATTEMPTS: i32 = 5;
const LOCKOUT_DURATION_MINUTES: i64 = 30;

// ============================================================================
// JWT Claims
// ============================================================================

/// JWT claims for admin access tokens
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AdminClaims {
    /// Subject — admin user ID
    pub sub: String,
    /// Admin email
    pub email: String,
    /// Admin role (viewer, operator, admin, superadmin)
    pub role: String,
    /// Issued at (Unix timestamp)
    pub iat: i64,
    /// Expiration (Unix timestamp)
    pub exp: i64,
    /// Token type
    pub token_type: String,
}

// ============================================================================
// Request / Response DTOs
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    pub expires_in: i64,
    pub admin: AdminInfo,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminInfo {
    pub id: String,
    pub email: String,
    pub display_name: String,
    pub role: String,
}

#[derive(Debug, Deserialize)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RefreshResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: i64,
}

#[derive(Debug, Deserialize)]
pub struct LogoutRequest {
    pub refresh_token: String,
}

// ============================================================================
// Database row types
// ============================================================================

#[derive(Debug, sqlx::FromRow)]
struct AdminUserRow {
    id: uuid::Uuid,
    email: String,
    password_hash: String,
    display_name: String,
    role: String,
    is_active: bool,
    failed_login_count: i32,
    locked_until: Option<chrono::DateTime<Utc>>,
}

// ============================================================================
// Handlers
// ============================================================================

/// POST /v1/admin/auth/login
///
/// Authenticate admin user with email + password.
/// Returns JWT access token + refresh token.
pub async fn login(
    State(app_state): State<crate::router::AppState>,
    headers: HeaderMap,
    Json(request): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, ApiError> {
    let pool = app_state
        .db_pool
        .as_ref()
        .ok_or_else(|| ApiError::Internal("Admin auth database is not configured".to_string()))?;
    let ip = extract_ip(&headers);
    let user_agent = extract_user_agent(&headers);

    // 1. Find admin user by email
    let admin: AdminUserRow = sqlx::query_as(
        "SELECT id, email, password_hash, display_name, role, is_active, failed_login_count, locked_until
         FROM admin_users WHERE email = $1",
    )
    .bind(&request.email)
    .fetch_optional(pool)
    .await
    .map_err(|e| ApiError::Internal(format!("Database error: {}", e)))?
    .ok_or_else(|| {
        // Don't reveal whether email exists
        ApiError::Forbidden("Invalid email or password".to_string())
    })?;

    // 2. Check account status
    if !admin.is_active {
        log_auth_event(&pool, Some(admin.id), "login_failed_inactive", ip.as_deref(), user_agent.as_deref()).await;
        return Err(ApiError::Forbidden("Account is disabled".to_string()));
    }

    // 3. Check lockout
    if let Some(locked_until) = admin.locked_until {
        if Utc::now() < locked_until {
            log_auth_event(&pool, Some(admin.id), "login_failed_locked", ip.as_deref(), user_agent.as_deref()).await;
            return Err(ApiError::Forbidden(format!(
                "Account is locked. Try again after {}",
                locked_until.format("%H:%M UTC")
            )));
        }
    }

    // 4. Verify password
    let password_valid = verify_password(&request.password, &admin.password_hash);
    if !password_valid {
        // Increment failed login count
        let new_count = admin.failed_login_count + 1;
        let lockout = if new_count >= MAX_FAILED_LOGIN_ATTEMPTS {
            Some(Utc::now() + Duration::minutes(LOCKOUT_DURATION_MINUTES))
        } else {
            None
        };

        sqlx::query(
            "UPDATE admin_users SET failed_login_count = $1, locked_until = $2 WHERE id = $3"
        )
        .bind(new_count)
        .bind(lockout)
        .bind(admin.id)
        .execute(pool)
        .await
        .ok();

        log_auth_event(&pool, Some(admin.id), "login_failed", ip.as_deref(), user_agent.as_deref()).await;
        warn!(email = %request.email, attempts = new_count, "Admin login failed");
        return Err(ApiError::Forbidden("Invalid email or password".to_string()));
    }

    // 5. Reset failed login count + update last_login_at
    sqlx::query(
        "UPDATE admin_users SET failed_login_count = 0, locked_until = NULL, last_login_at = NOW() WHERE id = $1"
    )
    .bind(admin.id)
    .execute(pool)
    .await
    .ok();

    // 6. Generate access token (JWT)
    let now = Utc::now();
    let access_claims = AdminClaims {
        sub: admin.id.to_string(),
        email: admin.email.clone(),
        role: admin.role.clone(),
        iat: now.timestamp(),
        exp: (now + Duration::minutes(ACCESS_TOKEN_EXPIRY_MINUTES)).timestamp(),
        token_type: "access".to_string(),
    };

    let access_token = encode(
        &Header::default(),
        &access_claims,
        &EncodingKey::from_secret(jwt_secret()?.as_bytes()),
    )
    .map_err(|e| ApiError::Internal(format!("Failed to create token: {}", e)))?;

    // 7. Generate refresh token (opaque + stored hashed)
    let refresh_token_raw = format!("rrt_{}", uuid::Uuid::new_v4());
    let refresh_token_hash = hash_token(&refresh_token_raw);
    let refresh_expires = now + Duration::days(REFRESH_TOKEN_EXPIRY_DAYS);

    sqlx::query(
        "INSERT INTO admin_refresh_tokens (admin_id, token_hash, user_agent, ip_address, expires_at)
         VALUES ($1, $2, $3, $4, $5)"
    )
    .bind(admin.id)
    .bind(&refresh_token_hash)
    .bind(user_agent.as_deref())
    .bind(ip.as_deref())
    .bind(refresh_expires)
    .execute(pool)
    .await
    .map_err(|e| ApiError::Internal(format!("Failed to store refresh token: {}", e)))?;

    log_auth_event(&pool, Some(admin.id), "login", ip.as_deref(), user_agent.as_deref()).await;
    info!(admin_id = %admin.id, email = %admin.email, role = %admin.role, "Admin logged in");

    Ok(Json(LoginResponse {
        access_token,
        refresh_token: refresh_token_raw,
        token_type: "Bearer".to_string(),
        expires_in: ACCESS_TOKEN_EXPIRY_MINUTES * 60,
        admin: AdminInfo {
            id: admin.id.to_string(),
            email: admin.email,
            display_name: admin.display_name,
            role: admin.role,
        },
    }))
}

/// POST /v1/admin/auth/refresh
///
/// Exchange a valid refresh token for a new access token.
pub async fn refresh(
    State(app_state): State<crate::router::AppState>,
    headers: HeaderMap,
    Json(request): Json<RefreshRequest>,
) -> Result<Json<RefreshResponse>, ApiError> {
    let pool = app_state
        .db_pool
        .as_ref()
        .ok_or_else(|| ApiError::Internal("Admin auth database is not configured".to_string()))?;
    let ip = extract_ip(&headers);
    let user_agent = extract_user_agent(&headers);
    let token_hash = hash_token(&request.refresh_token);

    // 1. Find valid (non-revoked, non-expired) refresh token
    let row: Option<(uuid::Uuid,)> = sqlx::query_as(
        "SELECT admin_id FROM admin_refresh_tokens
         WHERE token_hash = $1 AND revoked_at IS NULL AND expires_at > NOW()"
    )
    .bind(&token_hash)
    .fetch_optional(pool)
    .await
    .map_err(|e| ApiError::Internal(format!("Database error: {}", e)))?;

    let (admin_id,) = row.ok_or_else(|| {
        ApiError::Forbidden("Invalid or expired refresh token".to_string())
    })?;

    // 2. Fetch admin user
    let admin: AdminUserRow = sqlx::query_as(
        "SELECT id, email, password_hash, display_name, role, is_active, failed_login_count, locked_until
         FROM admin_users WHERE id = $1 AND is_active = true",
    )
    .bind(admin_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| ApiError::Internal(format!("Database error: {}", e)))?
    .ok_or_else(|| ApiError::Forbidden("Admin account not found or disabled".to_string()))?;

    // 3. Issue new access token
    let now = Utc::now();
    let access_claims = AdminClaims {
        sub: admin.id.to_string(),
        email: admin.email.clone(),
        role: admin.role.clone(),
        iat: now.timestamp(),
        exp: (now + Duration::minutes(ACCESS_TOKEN_EXPIRY_MINUTES)).timestamp(),
        token_type: "access".to_string(),
    };

    let access_token = encode(
        &Header::default(),
        &access_claims,
        &EncodingKey::from_secret(jwt_secret()?.as_bytes()),
    )
    .map_err(|e| ApiError::Internal(format!("Failed to create token: {}", e)))?;

    log_auth_event(&pool, Some(admin_id), "token_refresh", ip.as_deref(), user_agent.as_deref()).await;

    Ok(Json(RefreshResponse {
        access_token,
        token_type: "Bearer".to_string(),
        expires_in: ACCESS_TOKEN_EXPIRY_MINUTES * 60,
    }))
}

/// POST /v1/admin/auth/logout
///
/// Revoke a refresh token.
pub async fn logout(
    State(app_state): State<crate::router::AppState>,
    headers: HeaderMap,
    Json(request): Json<LogoutRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let pool = app_state
        .db_pool
        .as_ref()
        .ok_or_else(|| ApiError::Internal("Admin auth database is not configured".to_string()))?;
    let ip = extract_ip(&headers);
    let user_agent = extract_user_agent(&headers);
    let token_hash = hash_token(&request.refresh_token);

    // Revoke the refresh token
    let result = sqlx::query(
        "UPDATE admin_refresh_tokens SET revoked_at = NOW() WHERE token_hash = $1 AND revoked_at IS NULL"
    )
    .bind(&token_hash)
    .execute(pool)
    .await
    .map_err(|e| ApiError::Internal(format!("Database error: {}", e)))?;

    if result.rows_affected() > 0 {
        // Find admin_id for audit log
        let row: Option<(uuid::Uuid,)> = sqlx::query_as(
            "SELECT admin_id FROM admin_refresh_tokens WHERE token_hash = $1"
        )
        .bind(&token_hash)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten();

        if let Some((admin_id,)) = row {
            log_auth_event(&pool, Some(admin_id), "logout", ip.as_deref(), user_agent.as_deref()).await;
        }
    }

    Ok(Json(serde_json::json!({ "status": "ok" })))
}

// ============================================================================
// Public utilities for other modules
// ============================================================================

/// Verify an admin JWT access token and extract claims.
/// Used by `check_admin_key_with_role` in tier.rs.
pub fn verify_admin_jwt(token: &str) -> Result<AdminClaims, ApiError> {
    let mut validation = Validation::default();
    validation.set_required_spec_claims(&["sub", "exp", "iat"]);

    let token_data = decode::<AdminClaims>(
        token,
        &DecodingKey::from_secret(jwt_secret()?.as_bytes()),
        &validation,
    )
    .map_err(|e| ApiError::Forbidden(format!("Invalid admin token: {}", e)))?;

    if token_data.claims.token_type != "access" {
        return Err(ApiError::Forbidden("Not an access token".to_string()));
    }

    Ok(token_data.claims)
}

/// Extract Bearer token from Authorization header
pub fn extract_bearer_token(headers: &HeaderMap) -> Option<&str> {
    headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
}

/// Hash a password using argon2id
pub fn hash_password(password: &str) -> Result<String, ApiError> {
    use argon2::{
        password_hash::{rand_core::OsRng, SaltString},
        Argon2, PasswordHasher,
    };

    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();

    argon2
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| ApiError::Internal(format!("Failed to hash password: {}", e)))
}

// ============================================================================
// Internal helpers
// ============================================================================

fn verify_password(password: &str, hash: &str) -> bool {
    use argon2::{Argon2, PasswordHash, PasswordVerifier};

    let Ok(parsed_hash) = PasswordHash::new(hash) else {
        return false;
    };

    Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok()
}

fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    hex::encode(hasher.finalize())
}

fn extract_ip(headers: &HeaderMap) -> Option<String> {
    headers
        .get("X-Forwarded-For")
        .or_else(|| headers.get("X-Real-IP"))
        .and_then(|v| v.to_str().ok())
        .map(|s| s.split(',').next().unwrap_or(s).trim().to_string())
}

fn extract_user_agent(headers: &HeaderMap) -> Option<String> {
    headers
        .get("User-Agent")
        .and_then(|v| v.to_str().ok())
        .map(String::from)
}

async fn log_auth_event(
    pool: &PgPool,
    admin_id: Option<uuid::Uuid>,
    action: &str,
    ip: Option<&str>,
    user_agent: Option<&str>,
) {
    let _ = sqlx::query(
        "INSERT INTO admin_auth_audit_log (admin_id, action, ip_address, user_agent) VALUES ($1, $2, $3, $4)"
    )
    .bind(admin_id)
    .bind(action)
    .bind(ip)
    .bind(user_agent)
    .execute(pool)
    .await;
}
