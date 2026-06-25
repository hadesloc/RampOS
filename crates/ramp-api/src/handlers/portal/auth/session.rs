use axum::{
    extract::State,
    response::{IntoResponse, Response},
    Json,
};
use axum_extra::extract::cookie::CookieJar;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use chrono::{DateTime, Duration, Utc};
use jsonwebtoken::{encode, EncodingKey, Header};
use rand::RngCore;
use serde::Serialize;
use sha2::{Digest, Sha256};
use sqlx::FromRow;
use tracing::{info, warn};
use uuid::Uuid;

use super::identity::{find_identity_by_id, PortalIdentity};
use super::{
    clear_auth_cookies, set_auth_cookies, AuthResponse, AuthUser, PortalClaims,
    ACCESS_TOKEN_EXPIRY_SECS, REFRESH_COOKIE_NAME, REFRESH_TOKEN_EXPIRY_SECS,
};
use crate::error::ApiError;
use crate::middleware::PortalUser;
use crate::router::AppState;

#[derive(Debug, FromRow)]
struct RefreshTokenRow {
    id: Uuid,
    user_id: String,
    tenant_id: String,
    family_id: Uuid,
    revoked: bool,
    expires_at: DateTime<Utc>,
    rotated_to_id: Option<Uuid>,
}

pub async fn refresh(
    State(app_state): State<AppState>,
    jar: CookieJar,
) -> Result<(CookieJar, Json<AuthResponse>), Response> {
    let refresh_value = jar
        .get(REFRESH_COOKIE_NAME)
        .filter(|cookie| !cookie.value().is_empty())
        .map(|cookie| cookie.value().to_string())
        .ok_or_else(|| {
            reject(
                jar.clone(),
                ApiError::Unauthorized("No refresh token provided".to_string()),
            )
        })?;
    let pool = app_state.db_pool.as_ref().ok_or_else(|| {
        reject(
            jar.clone(),
            ApiError::Internal("Database not configured".to_string()),
        )
    })?;
    let token_hash = Sha256::digest(refresh_value.as_bytes()).to_vec();
    let mut transaction = pool.begin().await.map_err(|error| {
        reject(
            jar.clone(),
            ApiError::Internal(format!("Failed to start refresh transaction: {error}")),
        )
    })?;

    let row = sqlx::query_as::<_, RefreshTokenRow>(
        r#"
        SELECT
            id, user_id, tenant_id, family_id, revoked, expires_at, rotated_to_id
        FROM refresh_tokens
        WHERE token_hash = $1
        FOR UPDATE
        "#,
    )
    .bind(&token_hash)
    .fetch_optional(&mut *transaction)
    .await
    .map_err(|error| {
        reject(
            jar.clone(),
            ApiError::Internal(format!("Failed to load refresh token: {error}")),
        )
    })?
    .ok_or_else(|| {
        reject(
            clear_auth_cookies(jar.clone()),
            ApiError::Unauthorized("Invalid or expired refresh token".to_string()),
        )
    })?;

    if row.revoked {
        if row.rotated_to_id.is_some() {
            sqlx::query(
                r#"
                UPDATE refresh_tokens
                SET
                    revoked = TRUE,
                    revoked_at = COALESCE(revoked_at, NOW()),
                    revoke_reason = 'replay_detected',
                    updated_at = NOW()
                WHERE family_id = $1
                "#,
            )
            .bind(row.family_id)
            .execute(&mut *transaction)
            .await
            .map_err(|error| {
                reject(
                    jar.clone(),
                    ApiError::Internal(format!("Failed to revoke replayed family: {error}")),
                )
            })?;
            transaction.commit().await.map_err(|error| {
                reject(
                    jar.clone(),
                    ApiError::Internal(format!("Failed to commit replay revocation: {error}")),
                )
            })?;
        }
        return Err(reject(
            clear_auth_cookies(jar),
            ApiError::Unauthorized("Invalid or expired refresh token".to_string()),
        ));
    }

    if row.expires_at <= Utc::now() {
        sqlx::query(
            r#"
            UPDATE refresh_tokens
            SET revoked = TRUE, revoked_at = NOW(), revoke_reason = 'expired', updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(row.id)
        .execute(&mut *transaction)
        .await
        .map_err(|error| {
            reject(
                jar.clone(),
                ApiError::Internal(format!("Failed to expire refresh token: {error}")),
            )
        })?;
        transaction.commit().await.map_err(|error| {
            reject(
                jar.clone(),
                ApiError::Internal(format!("Failed to commit token expiry: {error}")),
            )
        })?;
        return Err(reject(
            clear_auth_cookies(jar),
            ApiError::Unauthorized("Invalid or expired refresh token".to_string()),
        ));
    }

    let identity = find_identity_by_id(pool, &row.tenant_id, &row.user_id)
        .await
        .map_err(|error| {
            reject(
                jar.clone(),
                ApiError::Internal(format!("Failed to load portal identity: {error}")),
            )
        })?
        .filter(|identity| identity.status == "ACTIVE")
        .ok_or_else(|| {
            reject(
                clear_auth_cookies(jar.clone()),
                ApiError::Unauthorized("Portal identity is not active".to_string()),
            )
        })?;

    let replacement_id = Uuid::new_v4();
    let replacement_raw = generate_refresh_token();
    let replacement_hash = Sha256::digest(replacement_raw.as_bytes()).to_vec();
    let replacement_expiry = Utc::now() + Duration::seconds(REFRESH_TOKEN_EXPIRY_SECS);

    sqlx::query(
        r#"
        INSERT INTO refresh_tokens (
            id, token_hash, user_id, tenant_id, expires_at, family_id
        ) VALUES ($1, $2, $3, $4, $5, $6)
        "#,
    )
    .bind(replacement_id)
    .bind(replacement_hash)
    .bind(&row.user_id)
    .bind(&row.tenant_id)
    .bind(replacement_expiry)
    .bind(row.family_id)
    .execute(&mut *transaction)
    .await
    .map_err(|error| {
        reject(
            jar.clone(),
            ApiError::Internal(format!(
                "Failed to store replacement refresh token: {error}"
            )),
        )
    })?;

    sqlx::query(
        r#"
        UPDATE refresh_tokens
        SET
            revoked = TRUE,
            revoked_at = NOW(),
            revoke_reason = 'rotated',
            rotated_to_id = $1,
            updated_at = NOW()
        WHERE id = $2
        "#,
    )
    .bind(replacement_id)
    .bind(row.id)
    .execute(&mut *transaction)
    .await
    .map_err(|error| {
        reject(
            jar.clone(),
            ApiError::Internal(format!("Failed to rotate refresh token: {error}")),
        )
    })?;

    transaction.commit().await.map_err(|error| {
        reject(
            jar.clone(),
            ApiError::Internal(format!("Failed to commit refresh rotation: {error}")),
        )
    })?;

    let (access_token, expires_at) =
        mint_access_token(&app_state, &identity).map_err(|error| reject(jar.clone(), error))?;
    let secure = is_production();
    let jar = set_auth_cookies(jar, &access_token, &replacement_raw, secure);

    Ok((
        jar,
        Json(AuthResponse {
            user: auth_user(&identity),
            expires_at,
        }),
    ))
}

pub async fn logout(
    State(app_state): State<AppState>,
    jar: CookieJar,
) -> Result<CookieJar, ApiError> {
    if let (Some(pool), Some(cookie)) = (app_state.db_pool.as_ref(), jar.get(REFRESH_COOKIE_NAME)) {
        let token_hash = Sha256::digest(cookie.value().as_bytes()).to_vec();
        sqlx::query(
            r#"
            UPDATE refresh_tokens
            SET
                revoked = TRUE,
                revoked_at = COALESCE(revoked_at, NOW()),
                revoke_reason = COALESCE(revoke_reason, 'logout'),
                updated_at = NOW()
            WHERE token_hash = $1
            "#,
        )
        .bind(token_hash)
        .execute(pool)
        .await
        .map_err(|error| ApiError::Internal(format!("Failed to revoke session: {error}")))?;
    }
    Ok(clear_auth_cookies(jar))
}

pub async fn logout_all(
    State(app_state): State<AppState>,
    portal_user: PortalUser,
    jar: CookieJar,
) -> Result<CookieJar, ApiError> {
    let pool = app_state
        .db_pool
        .as_ref()
        .ok_or_else(|| ApiError::Internal("Database not configured".to_string()))?;
    sqlx::query(
        r#"
        UPDATE refresh_tokens
        SET
            revoked = TRUE,
            revoked_at = COALESCE(revoked_at, NOW()),
            revoke_reason = COALESCE(revoke_reason, 'logout_all'),
            updated_at = NOW()
        WHERE tenant_id = $1
          AND user_id = $2
          AND revoked = FALSE
        "#,
    )
    .bind(portal_user.tenant_id.to_string())
    .bind(portal_user.user_id.to_string())
    .execute(pool)
    .await
    .map_err(|error| ApiError::Internal(format!("Failed to revoke all sessions: {error}")))?;

    info!(portal_user_id = %portal_user.user_id, "All portal sessions revoked");
    Ok(clear_auth_cookies(jar))
}

pub(super) async fn issue_session(
    app_state: &AppState,
    jar: CookieJar,
    identity: &PortalIdentity,
) -> Result<(CookieJar, Json<AuthResponse>), ApiError> {
    let pool = app_state
        .db_pool
        .as_ref()
        .ok_or_else(|| ApiError::Internal("Database not configured".to_string()))?;
    let family_id = Uuid::new_v4();
    let refresh_raw = generate_refresh_token();
    let refresh_hash = Sha256::digest(refresh_raw.as_bytes()).to_vec();
    let refresh_expiry = Utc::now() + Duration::seconds(REFRESH_TOKEN_EXPIRY_SECS);

    sqlx::query(
        r#"
        INSERT INTO refresh_tokens (
            token_hash, user_id, tenant_id, expires_at, family_id
        ) VALUES ($1, $2, $3, $4, $5)
        "#,
    )
    .bind(refresh_hash)
    .bind(&identity.portal_user_id)
    .bind(&identity.tenant_id)
    .bind(refresh_expiry)
    .bind(family_id)
    .execute(pool)
    .await
    .map_err(|error| {
        warn!(error = %error, "Portal refresh token persistence failed");
        ApiError::Internal("Failed to create session".to_string())
    })?;

    let (access_token, expires_at) = mint_access_token(app_state, identity)?;
    let jar = set_auth_cookies(jar, &access_token, &refresh_raw, is_production());
    Ok((
        jar,
        Json(AuthResponse {
            user: auth_user(identity),
            expires_at,
        }),
    ))
}

fn mint_access_token(
    app_state: &AppState,
    identity: &PortalIdentity,
) -> Result<(String, i64), ApiError> {
    let now = Utc::now();
    let expires_at = now + Duration::seconds(ACCESS_TOKEN_EXPIRY_SECS);
    let claims = PortalClaims {
        sub: identity.portal_user_id.clone(),
        tenant_id: Some(identity.tenant_id.clone()),
        email: identity.email.clone().unwrap_or_default(),
        iat: now.timestamp(),
        exp: expires_at.timestamp(),
        token_type: "access".to_string(),
    };
    #[derive(Serialize)]
    struct SessionClaims<'a> {
        #[serde(flatten)]
        claims: &'a PortalClaims,
        financial_user_id: &'a str,
    }

    let token = encode(
        &Header::default(),
        &SessionClaims {
            claims: &claims,
            financial_user_id: &identity.financial_user_id,
        },
        &EncodingKey::from_secret(app_state.portal_auth_config.jwt_secret.as_bytes()),
    )
    .map_err(|error| ApiError::Internal(format!("Failed to create access token: {error}")))?;
    Ok((token, expires_at.timestamp()))
}

fn auth_user(identity: &PortalIdentity) -> AuthUser {
    AuthUser {
        id: identity.portal_user_id.clone(),
        email: identity.email.clone().unwrap_or_default(),
        kyc_status: identity.kyc_status.clone(),
        kyc_tier: identity.kyc_tier as i32,
        status: identity.status.clone(),
        created_at: identity.created_at.to_rfc3339(),
        wallet_address: identity.wallet_address.clone().unwrap_or_default(),
    }
}

fn generate_refresh_token() -> String {
    let mut bytes = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    format!("prt_{}", URL_SAFE_NO_PAD.encode(bytes))
}

fn is_production() -> bool {
    std::env::var("RAMPOS_ENV")
        .map(|value| value == "production")
        .unwrap_or(false)
}

fn reject(jar: CookieJar, error: ApiError) -> Response {
    (jar, error).into_response()
}
