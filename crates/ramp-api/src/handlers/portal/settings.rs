//! Portal Settings Handlers
//!
//! Endpoints for user profile and settings management:
//! - GET/PUT /v1/portal/settings/profile
//! - GET/PUT /v1/portal/settings/security
//! - GET/PUT /v1/portal/settings/notifications

use axum::{extract::State, routing::get, Json, Router};
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::error::ApiError;
use crate::middleware::PortalUser;
use crate::router::AppState;

// ============================================================================
// Profile DTOs
// ============================================================================

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileResponse {
    pub full_name: String,
    pub email: String,
    pub phone: Option<String>,
    pub avatar_url: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProfileRequest {
    pub full_name: Option<String>,
    pub phone: Option<String>,
    pub avatar_url: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProfileResponse {
    pub success: bool,
    pub profile: ProfileResponse,
}

// ============================================================================
// Security DTOs
// ============================================================================

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WebAuthnCredentialInfo {
    pub id: String,
    pub name: String,
    pub created_at: String,
    pub last_used_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SecurityResponse {
    pub two_factor_enabled: bool,
    pub webauthn_credentials: Vec<WebAuthnCredentialInfo>,
    pub last_password_change: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdatePasswordRequest {
    pub current_password: String,
    pub new_password: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSecurityResponse {
    pub success: bool,
    pub message: String,
}

// ============================================================================
// Notification DTOs
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationPreferences {
    pub email_notifications: bool,
    pub sms_notifications: bool,
    pub push_notifications: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateNotificationsResponse {
    pub success: bool,
    pub preferences: NotificationPreferences,
}

// ============================================================================
// Router
// ============================================================================

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/profile", get(get_profile).put(update_profile))
        .route("/security", get(get_security).put(update_security))
        .route(
            "/notifications",
            get(get_notifications).put(update_notifications),
        )
}

// ============================================================================
// Handlers
// ============================================================================

/// GET /v1/portal/settings/profile
pub async fn get_profile(
    State(app_state): State<AppState>,
    portal_user: PortalUser,
) -> Result<Json<ProfileResponse>, ApiError> {
    info!(
        user_id = %portal_user.user_id,
        "Get profile requested"
    );

    let pool = app_state
        .db_pool
        .as_ref()
        .ok_or_else(|| ApiError::Internal("Database not configured".to_string()))?;

    // Query profile from database
    let row: Option<(String, String, Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT COALESCE(full_name, ''), email, phone, avatar_url FROM portal_users WHERE id = $1",
    )
    .bind(portal_user.user_id.to_string())
    .fetch_optional(pool)
    .await
    .map_err(|e| {
        warn!(error = %e, "Failed to query user profile");
        ApiError::Internal("Failed to retrieve profile".to_string())
    })?;

    let (full_name, email, phone, avatar_url) =
        row.ok_or_else(|| ApiError::NotFound("User not found".to_string()))?;

    Ok(Json(ProfileResponse {
        full_name,
        email,
        phone,
        avatar_url,
    }))
}

/// PUT /v1/portal/settings/profile
pub async fn update_profile(
    State(app_state): State<AppState>,
    portal_user: PortalUser,
    Json(req): Json<UpdateProfileRequest>,
) -> Result<Json<UpdateProfileResponse>, ApiError> {
    info!(
        user_id = %portal_user.user_id,
        "Update profile requested"
    );

    let pool = app_state
        .db_pool
        .as_ref()
        .ok_or_else(|| ApiError::Internal("Database not configured".to_string()))?;

    let user_id = portal_user.user_id.to_string();

    // Build dynamic update query
    let mut updates = Vec::new();
    let mut param_idx = 2u32; // $1 is user_id

    if req.full_name.is_some() {
        updates.push(format!("full_name = ${}", param_idx));
        param_idx += 1;
    }
    if req.phone.is_some() {
        updates.push(format!("phone = ${}", param_idx));
        param_idx += 1;
    }
    if req.avatar_url.is_some() {
        updates.push(format!("avatar_url = ${}", param_idx));
    }

    if updates.is_empty() {
        return Err(ApiError::Validation("No fields to update".to_string()));
    }

    updates.push("updated_at = NOW()".to_string());
    let sql = format!(
        "UPDATE portal_users SET {} WHERE id = $1",
        updates.join(", ")
    );

    let mut query = sqlx::query(&sql).bind(&user_id);
    if let Some(ref name) = req.full_name {
        query = query.bind(name);
    }
    if let Some(ref phone) = req.phone {
        query = query.bind(phone);
    }
    if let Some(ref avatar) = req.avatar_url {
        query = query.bind(avatar);
    }

    query.execute(pool).await.map_err(|e| {
        warn!(error = %e, "Failed to update user profile");
        ApiError::Internal("Failed to update profile".to_string())
    })?;

    // Re-fetch updated profile
    let row: (String, String, Option<String>, Option<String>) = sqlx::query_as(
        "SELECT COALESCE(full_name, ''), email, phone, avatar_url FROM portal_users WHERE id = $1",
    )
    .bind(&user_id)
    .fetch_one(pool)
    .await
    .map_err(|e| {
        warn!(error = %e, "Failed to re-fetch profile after update");
        ApiError::Internal("Failed to retrieve updated profile".to_string())
    })?;

    Ok(Json(UpdateProfileResponse {
        success: true,
        profile: ProfileResponse {
            full_name: row.0,
            email: row.1,
            phone: row.2,
            avatar_url: row.3,
        },
    }))
}

/// GET /v1/portal/settings/security
pub async fn get_security(
    State(app_state): State<AppState>,
    portal_user: PortalUser,
) -> Result<Json<SecurityResponse>, ApiError> {
    info!(
        user_id = %portal_user.user_id,
        "Get security settings requested"
    );

    let pool = app_state
        .db_pool
        .as_ref()
        .ok_or_else(|| ApiError::Internal("Database not configured".to_string()))?;

    let user_id = portal_user.user_id.to_string();

    // Check if 2FA is enabled and get last password change
    let user_row: Option<(bool, Option<chrono::DateTime<chrono::Utc>>)> = sqlx::query_as(
        "SELECT COALESCE(two_factor_enabled, false), last_password_change FROM portal_users WHERE id = $1",
    )
    .bind(&user_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| {
        warn!(error = %e, "Failed to query security settings");
        ApiError::Internal("Failed to retrieve security settings".to_string())
    })?;

    let (two_factor_enabled, last_password_change) = user_row.unwrap_or((false, None));

    // Get WebAuthn credentials
    let creds: Vec<(String, Option<String>, chrono::DateTime<chrono::Utc>, Option<chrono::DateTime<chrono::Utc>>)> = sqlx::query_as(
        "SELECT id::text, credential_name, created_at, last_used_at FROM webauthn_credentials WHERE user_id = $1",
    )
    .bind(&user_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let webauthn_credentials = creds
        .into_iter()
        .map(|(id, name, created, last_used)| WebAuthnCredentialInfo {
            id,
            name: name.unwrap_or_else(|| "Passkey".to_string()),
            created_at: created.to_rfc3339(),
            last_used_at: last_used.map(|d| d.to_rfc3339()),
        })
        .collect();

    Ok(Json(SecurityResponse {
        two_factor_enabled,
        webauthn_credentials,
        last_password_change: last_password_change.map(|d| d.to_rfc3339()),
    }))
}

/// PUT /v1/portal/settings/security - Change password
pub async fn update_security(
    State(app_state): State<AppState>,
    portal_user: PortalUser,
    Json(req): Json<UpdatePasswordRequest>,
) -> Result<Json<UpdateSecurityResponse>, ApiError> {
    info!(
        user_id = %portal_user.user_id,
        "Update security (password change) requested"
    );

    // Validate new password length
    if req.new_password.len() < 8 {
        return Err(ApiError::Validation(
            "New password must be at least 8 characters".to_string(),
        ));
    }

    let pool = app_state
        .db_pool
        .as_ref()
        .ok_or_else(|| ApiError::Internal("Database not configured".to_string()))?;

    let user_id = portal_user.user_id.to_string();

    // Fetch current password hash
    let row: Option<(Option<String>,)> =
        sqlx::query_as("SELECT password_hash FROM portal_users WHERE id = $1")
            .bind(&user_id)
            .fetch_optional(pool)
            .await
            .map_err(|e| {
                warn!(error = %e, "Failed to query user password");
                ApiError::Internal("Failed to verify password".to_string())
            })?;

    let (stored_hash,) = row.ok_or_else(|| ApiError::NotFound("User not found".to_string()))?;

    // Verify current password
    if let Some(hash) = stored_hash {
        use argon2::{
            password_hash::{PasswordHash, PasswordVerifier},
            Argon2,
        };
        let parsed = PasswordHash::new(&hash)
            .map_err(|_| ApiError::Internal("Invalid password hash in database".to_string()))?;
        Argon2::default()
            .verify_password(req.current_password.as_bytes(), &parsed)
            .map_err(|_| ApiError::BadRequest("Current password is incorrect".to_string()))?;
    } else {
        // No password set (WebAuthn-only account) - current_password must be empty
        if !req.current_password.is_empty() {
            return Err(ApiError::BadRequest(
                "Current password is incorrect".to_string(),
            ));
        }
    }

    // Hash new password
    use argon2::{
        password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
        Argon2,
    };
    let salt = SaltString::generate(&mut OsRng);
    let new_hash = Argon2::default()
        .hash_password(req.new_password.as_bytes(), &salt)
        .map_err(|e| {
            warn!(error = %e, "Failed to hash new password");
            ApiError::Internal("Failed to update password".to_string())
        })?
        .to_string();

    // Update password in database
    sqlx::query(
        "UPDATE portal_users SET password_hash = $1, last_password_change = NOW(), updated_at = NOW() WHERE id = $2",
    )
    .bind(&new_hash)
    .bind(&user_id)
    .execute(pool)
    .await
    .map_err(|e| {
        warn!(error = %e, "Failed to update password in database");
        ApiError::Internal("Failed to update password".to_string())
    })?;

    info!(user_id = %user_id, "Password updated successfully");

    Ok(Json(UpdateSecurityResponse {
        success: true,
        message: "Password updated successfully".to_string(),
    }))
}

/// GET /v1/portal/settings/notifications
pub async fn get_notifications(
    State(app_state): State<AppState>,
    portal_user: PortalUser,
) -> Result<Json<NotificationPreferences>, ApiError> {
    info!(
        user_id = %portal_user.user_id,
        "Get notification preferences requested"
    );

    let pool = app_state
        .db_pool
        .as_ref()
        .ok_or_else(|| ApiError::Internal("Database not configured".to_string()))?;

    let user_id = portal_user.user_id.to_string();

    let row: Option<(bool, bool, bool)> = sqlx::query_as(
        "SELECT COALESCE(email_notifications, true), COALESCE(sms_notifications, false), COALESCE(push_notifications, true) FROM portal_users WHERE id = $1",
    )
    .bind(&user_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| {
        warn!(error = %e, "Failed to query notification preferences");
        ApiError::Internal("Failed to retrieve notification preferences".to_string())
    })?;

    let (email, sms, push) = row.unwrap_or((true, false, true));

    Ok(Json(NotificationPreferences {
        email_notifications: email,
        sms_notifications: sms,
        push_notifications: push,
    }))
}

/// PUT /v1/portal/settings/notifications
pub async fn update_notifications(
    State(app_state): State<AppState>,
    portal_user: PortalUser,
    Json(prefs): Json<NotificationPreferences>,
) -> Result<Json<UpdateNotificationsResponse>, ApiError> {
    info!(
        user_id = %portal_user.user_id,
        "Update notification preferences requested"
    );

    let pool = app_state
        .db_pool
        .as_ref()
        .ok_or_else(|| ApiError::Internal("Database not configured".to_string()))?;

    let user_id = portal_user.user_id.to_string();

    sqlx::query(
        "UPDATE portal_users SET email_notifications = $1, sms_notifications = $2, push_notifications = $3, updated_at = NOW() WHERE id = $4",
    )
    .bind(prefs.email_notifications)
    .bind(prefs.sms_notifications)
    .bind(prefs.push_notifications)
    .bind(&user_id)
    .execute(pool)
    .await
    .map_err(|e| {
        warn!(error = %e, "Failed to update notification preferences");
        ApiError::Internal("Failed to update notification preferences".to_string())
    })?;

    Ok(Json(UpdateNotificationsResponse {
        success: true,
        preferences: prefs,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profile_response_serialization() {
        let profile = ProfileResponse {
            full_name: "Test User".to_string(),
            email: "test@example.com".to_string(),
            phone: Some("+84123456789".to_string()),
            avatar_url: None,
        };
        let json = serde_json::to_string(&profile).unwrap();
        assert!(json.contains("\"fullName\":\"Test User\""));
        assert!(json.contains("\"email\":\"test@example.com\""));
        assert!(json.contains("\"phone\":\"+84123456789\""));
    }

    #[test]
    fn test_update_profile_request_deserialization() {
        let json = r#"{"fullName":"New Name","phone":"+84999888777"}"#;
        let req: UpdateProfileRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.full_name.unwrap(), "New Name");
        assert_eq!(req.phone.unwrap(), "+84999888777");
        assert!(req.avatar_url.is_none());
    }

    #[test]
    fn test_security_response_serialization() {
        let resp = SecurityResponse {
            two_factor_enabled: false,
            webauthn_credentials: vec![WebAuthnCredentialInfo {
                id: "cred-1".to_string(),
                name: "My Passkey".to_string(),
                created_at: "2024-01-01T00:00:00Z".to_string(),
                last_used_at: None,
            }],
            last_password_change: Some("2024-06-01T12:00:00Z".to_string()),
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"twoFactorEnabled\":false"));
        assert!(json.contains("\"webauthnCredentials\""));
        assert!(json.contains("\"lastPasswordChange\""));
    }

    #[test]
    fn test_update_password_request_deserialization() {
        let json = r#"{"currentPassword":"old123456","newPassword":"new123456"}"#;
        let req: UpdatePasswordRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.current_password, "old123456");
        assert_eq!(req.new_password, "new123456");
    }

    #[test]
    fn test_notification_preferences_roundtrip() {
        let prefs = NotificationPreferences {
            email_notifications: true,
            sms_notifications: false,
            push_notifications: true,
        };
        let json = serde_json::to_string(&prefs).unwrap();
        let decoded: NotificationPreferences = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.email_notifications, true);
        assert_eq!(decoded.sms_notifications, false);
        assert_eq!(decoded.push_notifications, true);
    }
}
