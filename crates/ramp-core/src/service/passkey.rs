//! Passkey credential management service
//!
//! Stores passkey credentials (WebAuthn P256 public keys) and links them
//! to user smart account addresses.
//!
//! **Storage**: PostgreSQL-backed via `passkey_credentials` table (migration 050).
//! Previous versions used in-memory HashMap; this has been upgraded for
//! production durability.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use tracing::{info, warn};

/// Passkey credential stored in the backend
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct PasskeyCredential {
    /// Unique credential ID (from WebAuthn registration)
    pub credential_id: String,
    /// User ID that owns this credential
    pub user_id: String,
    /// P256 public key x coordinate (hex-encoded)
    pub public_key_x: String,
    /// P256 public key y coordinate (hex-encoded)
    pub public_key_y: String,
    /// Smart account address linked to this passkey
    pub smart_account_address: Option<String>,
    /// Human-readable name for this passkey (e.g., "iPhone Face ID")
    pub display_name: String,
    /// Whether this credential is currently active
    pub is_active: bool,
    /// When this credential was registered
    pub created_at: DateTime<Utc>,
    /// When this credential was last used
    pub last_used_at: Option<DateTime<Utc>>,
}

/// Request to register a new passkey credential
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterPasskeyRequest {
    pub user_id: String,
    pub credential_id: String,
    pub public_key_x: String,
    pub public_key_y: String,
    pub display_name: String,
}

/// Response from registering a passkey
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterPasskeyResponse {
    pub credential_id: String,
    pub smart_account_address: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Request to link a passkey to a smart account
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkAccountRequest {
    pub user_id: String,
    pub credential_id: String,
    pub smart_account_address: String,
}

/// Passkey service — manages passkey credentials in PostgreSQL
pub struct PasskeyService {
    pool: PgPool,
}

impl PasskeyService {
    /// Create a new PasskeyService with a PostgreSQL connection pool
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Register a new passkey credential for a user
    pub async fn register_passkey(
        &self,
        request: RegisterPasskeyRequest,
    ) -> Result<RegisterPasskeyResponse, PasskeyError> {
        // Validate public key coordinates are valid hex
        validate_hex_coordinate(&request.public_key_x)?;
        validate_hex_coordinate(&request.public_key_y)?;

        if request.credential_id.is_empty() {
            return Err(PasskeyError::InvalidCredentialId);
        }

        if request.user_id.is_empty() {
            return Err(PasskeyError::InvalidUserId);
        }

        // Check for duplicate credential ID
        let existing: Option<(String,)> = sqlx::query_as(
            "SELECT credential_id FROM passkey_credentials WHERE credential_id = $1",
        )
        .bind(&request.credential_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| PasskeyError::DatabaseError(e.to_string()))?;

        if existing.is_some() {
            return Err(PasskeyError::CredentialAlreadyExists);
        }

        let now = Utc::now();

        sqlx::query(
            "INSERT INTO passkey_credentials (credential_id, user_id, public_key_x, public_key_y, display_name, is_active, created_at)
             VALUES ($1, $2, $3, $4, $5, true, $6)"
        )
        .bind(&request.credential_id)
        .bind(&request.user_id)
        .bind(&request.public_key_x)
        .bind(&request.public_key_y)
        .bind(&request.display_name)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(|e| PasskeyError::DatabaseError(e.to_string()))?;

        info!(
            user_id = %request.user_id,
            credential_id = %request.credential_id,
            "Passkey credential registered"
        );

        Ok(RegisterPasskeyResponse {
            credential_id: request.credential_id,
            smart_account_address: None,
            created_at: now,
        })
    }

    /// Get a specific passkey credential for a user
    pub async fn get_passkey(
        &self,
        user_id: &str,
        credential_id: &str,
    ) -> Result<PasskeyCredential, PasskeyError> {
        let credential: Option<PasskeyCredential> = sqlx::query_as(
            "SELECT credential_id, user_id, public_key_x, public_key_y, smart_account_address, display_name, is_active, created_at, last_used_at
             FROM passkey_credentials
             WHERE user_id = $1 AND credential_id = $2 AND is_active = true"
        )
        .bind(user_id)
        .bind(credential_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| PasskeyError::DatabaseError(e.to_string()))?;

        credential.ok_or_else(|| PasskeyError::CredentialNotFound(credential_id.to_string()))
    }

    /// List all passkey credentials for a user
    pub async fn list_passkeys(
        &self,
        user_id: &str,
    ) -> Result<Vec<PasskeyCredential>, PasskeyError> {
        let credentials: Vec<PasskeyCredential> = sqlx::query_as(
            "SELECT credential_id, user_id, public_key_x, public_key_y, smart_account_address, display_name, is_active, created_at, last_used_at
             FROM passkey_credentials
             WHERE user_id = $1 AND is_active = true
             ORDER BY created_at ASC"
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| PasskeyError::DatabaseError(e.to_string()))?;

        Ok(credentials)
    }

    /// Link a passkey credential to a smart account address
    pub async fn link_smart_account(
        &self,
        request: LinkAccountRequest,
    ) -> Result<(), PasskeyError> {
        let result = sqlx::query(
            "UPDATE passkey_credentials SET smart_account_address = $1
             WHERE user_id = $2 AND credential_id = $3"
        )
        .bind(&request.smart_account_address)
        .bind(&request.user_id)
        .bind(&request.credential_id)
        .execute(&self.pool)
        .await
        .map_err(|e| PasskeyError::DatabaseError(e.to_string()))?;

        if result.rows_affected() == 0 {
            return Err(PasskeyError::CredentialNotFound(request.credential_id));
        }

        info!(
            user_id = %request.user_id,
            credential_id = %request.credential_id,
            smart_account = %request.smart_account_address,
            "Passkey linked to smart account"
        );

        Ok(())
    }

    /// Deactivate a passkey credential
    pub async fn deactivate_passkey(
        &self,
        user_id: &str,
        credential_id: &str,
    ) -> Result<(), PasskeyError> {
        let result = sqlx::query(
            "UPDATE passkey_credentials SET is_active = false
             WHERE user_id = $1 AND credential_id = $2"
        )
        .bind(user_id)
        .bind(credential_id)
        .execute(&self.pool)
        .await
        .map_err(|e| PasskeyError::DatabaseError(e.to_string()))?;

        if result.rows_affected() == 0 {
            return Err(PasskeyError::CredentialNotFound(credential_id.to_string()));
        }

        warn!(
            user_id = %user_id,
            credential_id = %credential_id,
            "Passkey credential deactivated"
        );

        Ok(())
    }

    /// Update the last_used_at timestamp for a credential
    pub async fn mark_used(&self, user_id: &str, credential_id: &str) -> Result<(), PasskeyError> {
        let result = sqlx::query(
            "UPDATE passkey_credentials SET last_used_at = NOW()
             WHERE user_id = $1 AND credential_id = $2 AND is_active = true"
        )
        .bind(user_id)
        .bind(credential_id)
        .execute(&self.pool)
        .await
        .map_err(|e| PasskeyError::DatabaseError(e.to_string()))?;

        if result.rows_affected() == 0 {
            return Err(PasskeyError::CredentialNotFound(credential_id.to_string()));
        }

        Ok(())
    }

    /// Get credential count for a user
    pub async fn credential_count(&self, user_id: &str) -> usize {
        let result: Option<(i64,)> = sqlx::query_as(
            "SELECT COUNT(*) FROM passkey_credentials WHERE user_id = $1 AND is_active = true"
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .ok()
        .flatten();

        result.map(|(count,)| count as usize).unwrap_or(0)
    }
}

/// Validate that a hex string is a valid P256 coordinate (64 hex chars = 32 bytes)
fn validate_hex_coordinate(hex_str: &str) -> Result<(), PasskeyError> {
    let cleaned = hex_str.strip_prefix("0x").unwrap_or(hex_str);
    if cleaned.is_empty() {
        return Err(PasskeyError::InvalidPublicKey(
            "Empty coordinate".to_string(),
        ));
    }
    if cleaned.len() > 64 {
        return Err(PasskeyError::InvalidPublicKey(
            "Coordinate too long".to_string(),
        ));
    }
    if !cleaned.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(PasskeyError::InvalidPublicKey(
            "Invalid hex characters".to_string(),
        ));
    }
    Ok(())
}

/// Passkey service errors
#[derive(Debug, thiserror::Error)]
pub enum PasskeyError {
    #[error("User not found: {0}")]
    UserNotFound(String),

    #[error("Credential not found: {0}")]
    CredentialNotFound(String),

    #[error("Credential already exists")]
    CredentialAlreadyExists,

    #[error("Invalid credential ID")]
    InvalidCredentialId,

    #[error("Invalid user ID")]
    InvalidUserId,

    #[error("Invalid public key: {0}")]
    InvalidPublicKey(String),

    #[error("Database error: {0}")]
    DatabaseError(String),
}
