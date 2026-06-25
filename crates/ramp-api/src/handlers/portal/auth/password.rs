use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use axum::{extract::State, Json};
use axum_extra::extract::cookie::CookieJar;
use serde::Deserialize;
use std::sync::OnceLock;
use tracing::{info, warn};
use validator::Validate;

use super::identity::{create_password_identity, find_identity_by_email, normalize_email};
use super::{session, AuthResponse, PORTAL_TENANT_ID_DEFAULT};
use crate::error::ApiError;
use crate::router::AppState;

const INVALID_CREDENTIALS: &str = "Invalid email or password";

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct RegisterRequest {
    #[validate(email(message = "Invalid email address"))]
    pub email: String,
    #[validate(length(
        min = 12,
        max = 128,
        message = "Password must be between 12 and 128 characters"
    ))]
    pub password: String,
    #[validate(length(max = 200, message = "Full name is too long"))]
    pub full_name: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct LoginRequest {
    #[validate(email(message = "Invalid email address"))]
    pub email: String,
    #[validate(length(min = 1, max = 128, message = "Invalid password"))]
    pub password: String,
}

pub async fn register(
    State(app_state): State<AppState>,
    jar: CookieJar,
    Json(mut request): Json<RegisterRequest>,
) -> Result<(CookieJar, Json<AuthResponse>), ApiError> {
    request.email = normalize_email(&request.email);
    request
        .validate()
        .map_err(|error| ApiError::Validation(error.to_string()))?;

    let pool = app_state
        .db_pool
        .as_ref()
        .ok_or_else(|| ApiError::Internal("Database not configured".to_string()))?;
    let tenant_id = portal_tenant_id();
    let password_hash = hash_password(&request.password)?;

    let identity = create_password_identity(
        pool,
        &tenant_id,
        &request.email,
        &password_hash,
        request.full_name.as_deref(),
    )
    .await
    .map_err(map_registration_error)?;

    info!(
        portal_user_id = %identity.portal_user_id,
        tenant_id = %identity.tenant_id,
        "Portal password registration successful"
    );

    session::issue_session(&app_state, jar, &identity).await
}

pub async fn login(
    State(app_state): State<AppState>,
    jar: CookieJar,
    Json(mut request): Json<LoginRequest>,
) -> Result<(CookieJar, Json<AuthResponse>), ApiError> {
    request.email = normalize_email(&request.email);
    request
        .validate()
        .map_err(|error| ApiError::Validation(error.to_string()))?;

    let pool = app_state
        .db_pool
        .as_ref()
        .ok_or_else(|| ApiError::Internal("Database not configured".to_string()))?;
    let tenant_id = portal_tenant_id();
    let identity = find_identity_by_email(pool, &tenant_id, &request.email)
        .await
        .map_err(|error| {
            warn!(error = %error, "Portal password identity lookup failed");
            ApiError::Internal("Authentication failed".to_string())
        })?;

    let stored_hash: Option<String> = if let Some(identity) = identity.as_ref() {
        sqlx::query_scalar(
            "SELECT password_hash FROM portal_users WHERE tenant_id = $1 AND id = $2",
        )
        .bind(&tenant_id)
        .bind(&identity.portal_user_id)
        .fetch_optional(pool)
        .await
        .map_err(|error| {
            warn!(error = %error, "Portal password hash lookup failed");
            ApiError::Internal("Authentication failed".to_string())
        })?
        .flatten()
    } else {
        None
    };

    let password_valid = match stored_hash.as_deref() {
        Some(hash) => verify_password(&request.password, hash),
        None => verify_password(&request.password, dummy_password_hash()),
    };

    let identity = match identity {
        Some(identity)
            if stored_hash.is_some() && password_valid && identity.status == "ACTIVE" =>
        {
            identity
        }
        _ => return Err(ApiError::Unauthorized(INVALID_CREDENTIALS.to_string())),
    };

    info!(
        portal_user_id = %identity.portal_user_id,
        tenant_id = %identity.tenant_id,
        "Portal password login successful"
    );

    session::issue_session(&app_state, jar, &identity).await
}

fn hash_password(password: &str) -> Result<String, ApiError> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|error| {
            warn!(error = %error, "Portal password hashing failed");
            ApiError::Internal("Failed to secure password".to_string())
        })
}

fn verify_password(password: &str, encoded_hash: &str) -> bool {
    PasswordHash::new(encoded_hash)
        .ok()
        .and_then(|hash| {
            Argon2::default()
                .verify_password(password.as_bytes(), &hash)
                .ok()
        })
        .is_some()
}

fn dummy_password_hash() -> &'static str {
    static DUMMY_HASH: OnceLock<String> = OnceLock::new();
    DUMMY_HASH
        .get_or_init(|| {
            let salt = SaltString::encode_b64(b"rampos-auth-dummy")
                .expect("fixed dummy salt should be valid");
            Argon2::default()
                .hash_password(b"not-a-real-portal-password", &salt)
                .expect("fixed dummy password should hash")
                .to_string()
        })
        .as_str()
}

fn portal_tenant_id() -> String {
    std::env::var("PORTAL_TENANT_ID").unwrap_or_else(|_| PORTAL_TENANT_ID_DEFAULT.to_string())
}

fn map_registration_error(error: sqlx::Error) -> ApiError {
    if error
        .as_database_error()
        .and_then(|database_error| database_error.code())
        .as_deref()
        == Some("23505")
    {
        return ApiError::Conflict("An account with this email already exists".to_string());
    }

    warn!(error = %error, "Portal password registration failed");
    ApiError::Internal("Failed to create account".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn password_hash_round_trip() {
        let hash = hash_password("correct horse battery staple").unwrap();
        assert!(verify_password("correct horse battery staple", &hash));
        assert!(!verify_password("incorrect password", &hash));
    }

    #[test]
    fn dummy_hash_is_valid() {
        assert!(!verify_password(
            "any submitted password",
            dummy_password_hash()
        ));
    }
}
