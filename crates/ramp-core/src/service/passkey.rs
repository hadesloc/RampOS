//! Passkey credential management service
//!
//! Stores passkey credentials (WebAuthn P256 public keys) and links them
//! to user smart account addresses.
//!
//! Production code should use PostgreSQL-backed storage via `with_pool()`.
//! The in-memory backend remains available for focused tests that exercise
//! WebAuthn ceremony logic without external database setup.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
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

#[derive(Debug, Clone, Default)]
struct InMemoryPasskeyStore {
    credentials: HashMap<String, Vec<PasskeyCredential>>,
}

enum PasskeyBackend {
    Postgres(PgPool),
    InMemory(Arc<RwLock<InMemoryPasskeyStore>>),
}

/// Passkey service — manages passkey credentials via pluggable storage.
pub struct PasskeyService {
    backend: PasskeyBackend,
}

impl PasskeyService {
    /// Create a new in-memory PasskeyService.
    ///
    /// This backend is intended for focused tests and local ceremony simulation.
    pub fn new() -> Self {
        Self {
            backend: PasskeyBackend::InMemory(Arc::new(RwLock::new(
                InMemoryPasskeyStore::default(),
            ))),
        }
    }

    /// Create a PostgreSQL-backed PasskeyService for production durability.
    pub fn with_pool(pool: PgPool) -> Self {
        Self {
            backend: PasskeyBackend::Postgres(pool),
        }
    }

    /// Register a new passkey credential for a user
    pub async fn register_passkey(
        &self,
        request: RegisterPasskeyRequest,
    ) -> Result<RegisterPasskeyResponse, PasskeyError> {
        validate_hex_coordinate(&request.public_key_x)?;
        validate_hex_coordinate(&request.public_key_y)?;

        if request.credential_id.is_empty() {
            return Err(PasskeyError::InvalidCredentialId);
        }

        if request.user_id.is_empty() {
            return Err(PasskeyError::InvalidUserId);
        }

        let now = Utc::now();

        match &self.backend {
            PasskeyBackend::Postgres(pool) => {
                let existing: Option<(String,)> = sqlx::query_as(
                    "SELECT credential_id FROM passkey_credentials WHERE credential_id = $1",
                )
                .bind(&request.credential_id)
                .fetch_optional(pool)
                .await
                .map_err(|e| PasskeyError::DatabaseError(e.to_string()))?;

                if existing.is_some() {
                    return Err(PasskeyError::CredentialAlreadyExists);
                }

                sqlx::query(
                    "INSERT INTO passkey_credentials (credential_id, user_id, public_key_x, public_key_y, display_name, is_active, created_at)
                     VALUES ($1, $2, $3, $4, $5, true, $6)",
                )
                .bind(&request.credential_id)
                .bind(&request.user_id)
                .bind(&request.public_key_x)
                .bind(&request.public_key_y)
                .bind(&request.display_name)
                .bind(now)
                .execute(pool)
                .await
                .map_err(|e| PasskeyError::DatabaseError(e.to_string()))?;
            }
            PasskeyBackend::InMemory(store) => {
                let mut store = store.write().await;
                if store
                    .credentials
                    .values()
                    .flat_map(|creds| creds.iter())
                    .any(|cred| cred.credential_id == request.credential_id)
                {
                    return Err(PasskeyError::CredentialAlreadyExists);
                }

                store
                    .credentials
                    .entry(request.user_id.clone())
                    .or_default()
                    .push(PasskeyCredential {
                        credential_id: request.credential_id.clone(),
                        user_id: request.user_id.clone(),
                        public_key_x: request.public_key_x.clone(),
                        public_key_y: request.public_key_y.clone(),
                        smart_account_address: None,
                        display_name: request.display_name.clone(),
                        is_active: true,
                        created_at: now,
                        last_used_at: None,
                    });
            }
        }

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
        match &self.backend {
            PasskeyBackend::Postgres(pool) => {
                let user_exists = self.user_has_any_credential(user_id).await?;
                let credential: Option<PasskeyCredential> = sqlx::query_as(
                    "SELECT credential_id, user_id, public_key_x, public_key_y, smart_account_address, display_name, is_active, created_at, last_used_at
                     FROM passkey_credentials
                     WHERE user_id = $1 AND credential_id = $2 AND is_active = true",
                )
                .bind(user_id)
                .bind(credential_id)
                .fetch_optional(pool)
                .await
                .map_err(|e| PasskeyError::DatabaseError(e.to_string()))?;

                match credential {
                    Some(credential) => Ok(credential),
                    None if user_exists => {
                        Err(PasskeyError::CredentialNotFound(credential_id.to_string()))
                    }
                    None => Err(PasskeyError::UserNotFound(user_id.to_string())),
                }
            }
            PasskeyBackend::InMemory(store) => {
                let store = store.read().await;
                let user_creds = store
                    .credentials
                    .get(user_id)
                    .ok_or_else(|| PasskeyError::UserNotFound(user_id.to_string()))?;

                user_creds
                    .iter()
                    .find(|c| c.credential_id == credential_id && c.is_active)
                    .cloned()
                    .ok_or_else(|| PasskeyError::CredentialNotFound(credential_id.to_string()))
            }
        }
    }

    /// List all passkey credentials for a user
    pub async fn list_passkeys(
        &self,
        user_id: &str,
    ) -> Result<Vec<PasskeyCredential>, PasskeyError> {
        match &self.backend {
            PasskeyBackend::Postgres(pool) => {
                let credentials: Vec<PasskeyCredential> = sqlx::query_as(
                    "SELECT credential_id, user_id, public_key_x, public_key_y, smart_account_address, display_name, is_active, created_at, last_used_at
                     FROM passkey_credentials
                     WHERE user_id = $1 AND is_active = true
                     ORDER BY created_at ASC",
                )
                .bind(user_id)
                .fetch_all(pool)
                .await
                .map_err(|e| PasskeyError::DatabaseError(e.to_string()))?;

                Ok(credentials)
            }
            PasskeyBackend::InMemory(store) => {
                let store = store.read().await;
                Ok(store
                    .credentials
                    .get(user_id)
                    .map(|creds| creds.iter().filter(|c| c.is_active).cloned().collect())
                    .unwrap_or_default())
            }
        }
    }

    /// Link a passkey credential to a smart account address
    pub async fn link_smart_account(
        &self,
        request: LinkAccountRequest,
    ) -> Result<(), PasskeyError> {
        match &self.backend {
            PasskeyBackend::Postgres(pool) => {
                let result = sqlx::query(
                    "UPDATE passkey_credentials SET smart_account_address = $1
                     WHERE user_id = $2 AND credential_id = $3",
                )
                .bind(&request.smart_account_address)
                .bind(&request.user_id)
                .bind(&request.credential_id)
                .execute(pool)
                .await
                .map_err(|e| PasskeyError::DatabaseError(e.to_string()))?;

                if result.rows_affected() == 0 {
                    return Err(PasskeyError::CredentialNotFound(request.credential_id));
                }
            }
            PasskeyBackend::InMemory(store) => {
                let mut store = store.write().await;
                let user_creds = store
                    .credentials
                    .get_mut(&request.user_id)
                    .ok_or_else(|| PasskeyError::UserNotFound(request.user_id.clone()))?;

                let credential = user_creds
                    .iter_mut()
                    .find(|c| c.credential_id == request.credential_id)
                    .ok_or_else(|| {
                        PasskeyError::CredentialNotFound(request.credential_id.clone())
                    })?;

                credential.smart_account_address = Some(request.smart_account_address.clone());
            }
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
        match &self.backend {
            PasskeyBackend::Postgres(pool) => {
                let result = sqlx::query(
                    "UPDATE passkey_credentials SET is_active = false
                     WHERE user_id = $1 AND credential_id = $2",
                )
                .bind(user_id)
                .bind(credential_id)
                .execute(pool)
                .await
                .map_err(|e| PasskeyError::DatabaseError(e.to_string()))?;

                if result.rows_affected() == 0 {
                    return Err(PasskeyError::CredentialNotFound(credential_id.to_string()));
                }
            }
            PasskeyBackend::InMemory(store) => {
                let mut store = store.write().await;
                let user_creds = store
                    .credentials
                    .get_mut(user_id)
                    .ok_or_else(|| PasskeyError::UserNotFound(user_id.to_string()))?;

                let credential = user_creds
                    .iter_mut()
                    .find(|c| c.credential_id == credential_id)
                    .ok_or_else(|| PasskeyError::CredentialNotFound(credential_id.to_string()))?;

                credential.is_active = false;
            }
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
        match &self.backend {
            PasskeyBackend::Postgres(pool) => {
                let result = sqlx::query(
                    "UPDATE passkey_credentials SET last_used_at = NOW()
                     WHERE user_id = $1 AND credential_id = $2 AND is_active = true",
                )
                .bind(user_id)
                .bind(credential_id)
                .execute(pool)
                .await
                .map_err(|e| PasskeyError::DatabaseError(e.to_string()))?;

                if result.rows_affected() == 0 {
                    return Err(PasskeyError::CredentialNotFound(credential_id.to_string()));
                }
            }
            PasskeyBackend::InMemory(store) => {
                let mut store = store.write().await;
                let user_creds = store
                    .credentials
                    .get_mut(user_id)
                    .ok_or_else(|| PasskeyError::UserNotFound(user_id.to_string()))?;

                let credential = user_creds
                    .iter_mut()
                    .find(|c| c.credential_id == credential_id && c.is_active)
                    .ok_or_else(|| PasskeyError::CredentialNotFound(credential_id.to_string()))?;

                credential.last_used_at = Some(Utc::now());
            }
        }

        Ok(())
    }

    /// Get credential count for a user
    pub async fn credential_count(&self, user_id: &str) -> usize {
        match &self.backend {
            PasskeyBackend::Postgres(pool) => {
                let result: Option<(i64,)> = sqlx::query_as(
                    "SELECT COUNT(*) FROM passkey_credentials WHERE user_id = $1 AND is_active = true",
                )
                .bind(user_id)
                .fetch_optional(pool)
                .await
                .ok()
                .flatten();

                result.map(|(count,)| count as usize).unwrap_or(0)
            }
            PasskeyBackend::InMemory(store) => {
                let store = store.read().await;
                store
                    .credentials
                    .get(user_id)
                    .map(|creds| creds.iter().filter(|c| c.is_active).count())
                    .unwrap_or(0)
            }
        }
    }

    async fn user_has_any_credential(&self, user_id: &str) -> Result<bool, PasskeyError> {
        match &self.backend {
            PasskeyBackend::Postgres(pool) => {
                let row: Option<(i64,)> =
                    sqlx::query_as("SELECT COUNT(*) FROM passkey_credentials WHERE user_id = $1")
                        .bind(user_id)
                        .fetch_optional(pool)
                        .await
                        .map_err(|e| PasskeyError::DatabaseError(e.to_string()))?;

                Ok(row.map(|(count,)| count > 0).unwrap_or(false))
            }
            PasskeyBackend::InMemory(store) => {
                let store = store.read().await;
                Ok(store.credentials.contains_key(user_id))
            }
        }
    }
}

impl Default for PasskeyService {
    fn default() -> Self {
        Self::new()
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
