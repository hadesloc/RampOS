//! Passkey credential management service
//!
//! Stores passkey credentials (WebAuthn P256 public keys) and links them
//! to user smart account addresses. Uses in-memory storage (mock DB) for
//! development; production would use PostgreSQL.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

/// Passkey credential stored in the backend
#[derive(Debug, Clone, Serialize, Deserialize)]
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

/// Passkey service - manages passkey credentials in memory
pub struct PasskeyService {
    /// In-memory credential store: user_id -> Vec<PasskeyCredential>
    credentials: Arc<RwLock<HashMap<String, Vec<PasskeyCredential>>>>,
}

impl PasskeyService {
    /// Create a new PasskeyService
    pub fn new() -> Self {
        Self {
            credentials: Arc::new(RwLock::new(HashMap::new())),
        }
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

        let now = Utc::now();
        let credential = PasskeyCredential {
            credential_id: request.credential_id.clone(),
            user_id: request.user_id.clone(),
            public_key_x: request.public_key_x,
            public_key_y: request.public_key_y,
            smart_account_address: None,
            display_name: request.display_name,
            is_active: true,
            created_at: now,
            last_used_at: None,
        };

        let mut store = self.credentials.write().await;

        // Check for duplicate credential ID
        if let Some(user_creds) = store.get(&request.user_id) {
            if user_creds
                .iter()
                .any(|c| c.credential_id == request.credential_id)
            {
                return Err(PasskeyError::CredentialAlreadyExists);
            }
        }

        store
            .entry(request.user_id.clone())
            .or_default()
            .push(credential);

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
        let store = self.credentials.read().await;

        let user_creds = store
            .get(user_id)
            .ok_or_else(|| PasskeyError::UserNotFound(user_id.to_string()))?;

        user_creds
            .iter()
            .find(|c| c.credential_id == credential_id && c.is_active)
            .cloned()
            .ok_or_else(|| PasskeyError::CredentialNotFound(credential_id.to_string()))
    }

    /// List all passkey credentials for a user
    pub async fn list_passkeys(
        &self,
        user_id: &str,
    ) -> Result<Vec<PasskeyCredential>, PasskeyError> {
        let store = self.credentials.read().await;

        Ok(store
            .get(user_id)
            .map(|creds| creds.iter().filter(|c| c.is_active).cloned().collect())
            .unwrap_or_default())
    }

    /// Link a passkey credential to a smart account address
    pub async fn link_smart_account(
        &self,
        request: LinkAccountRequest,
    ) -> Result<(), PasskeyError> {
        let mut store = self.credentials.write().await;

        let user_creds = store
            .get_mut(&request.user_id)
            .ok_or_else(|| PasskeyError::UserNotFound(request.user_id.clone()))?;

        let credential = user_creds
            .iter_mut()
            .find(|c| c.credential_id == request.credential_id)
            .ok_or_else(|| PasskeyError::CredentialNotFound(request.credential_id.clone()))?;

        credential.smart_account_address = Some(request.smart_account_address.clone());

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
        let mut store = self.credentials.write().await;

        let user_creds = store
            .get_mut(user_id)
            .ok_or_else(|| PasskeyError::UserNotFound(user_id.to_string()))?;

        let credential = user_creds
            .iter_mut()
            .find(|c| c.credential_id == credential_id)
            .ok_or_else(|| PasskeyError::CredentialNotFound(credential_id.to_string()))?;

        credential.is_active = false;

        warn!(
            user_id = %user_id,
            credential_id = %credential_id,
            "Passkey credential deactivated"
        );

        Ok(())
    }

    /// Update the last_used_at timestamp for a credential
    pub async fn mark_used(&self, user_id: &str, credential_id: &str) -> Result<(), PasskeyError> {
        let mut store = self.credentials.write().await;

        let user_creds = store
            .get_mut(user_id)
            .ok_or_else(|| PasskeyError::UserNotFound(user_id.to_string()))?;

        let credential = user_creds
            .iter_mut()
            .find(|c| c.credential_id == credential_id && c.is_active)
            .ok_or_else(|| PasskeyError::CredentialNotFound(credential_id.to_string()))?;

        credential.last_used_at = Some(Utc::now());

        Ok(())
    }

    /// Get credential count for a user
    pub async fn credential_count(&self, user_id: &str) -> usize {
        let store = self.credentials.read().await;
        store
            .get(user_id)
            .map(|creds| creds.iter().filter(|c| c.is_active).count())
            .unwrap_or(0)
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
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_request() -> RegisterPasskeyRequest {
        RegisterPasskeyRequest {
            user_id: "user-123".to_string(),
            credential_id: "cred-abc".to_string(),
            public_key_x: "6B17D1F2E12C4247F8BCE6E563A440F277037D812DEB33A0F4A13945D898C296"
                .to_string(),
            public_key_y: "4FE342E2FE1A7F9B8EE7EB4A7C0F9E162BCE33576B315ECECBB6406837BF51F5"
                .to_string(),
            display_name: "Test Passkey".to_string(),
        }
    }

    #[tokio::test]
    async fn test_register_passkey() {
        let service = PasskeyService::new();
        let request = test_request();

        let result = service.register_passkey(request.clone()).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response.credential_id, "cred-abc");
        assert!(response.smart_account_address.is_none());
    }

    #[tokio::test]
    async fn test_register_duplicate_passkey() {
        let service = PasskeyService::new();
        let request = test_request();

        service.register_passkey(request.clone()).await.unwrap();

        let result = service.register_passkey(request).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            PasskeyError::CredentialAlreadyExists
        ));
    }

    #[tokio::test]
    async fn test_get_passkey() {
        let service = PasskeyService::new();
        let request = test_request();

        service.register_passkey(request).await.unwrap();

        let result = service.get_passkey("user-123", "cred-abc").await;
        assert!(result.is_ok());

        let cred = result.unwrap();
        assert_eq!(cred.credential_id, "cred-abc");
        assert_eq!(cred.user_id, "user-123");
        assert!(cred.is_active);
    }

    #[tokio::test]
    async fn test_get_passkey_not_found() {
        let service = PasskeyService::new();

        let result = service.get_passkey("user-123", "nonexistent").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_list_passkeys() {
        let service = PasskeyService::new();

        // Register two passkeys
        let req1 = test_request();
        let mut req2 = test_request();
        req2.credential_id = "cred-def".to_string();
        req2.display_name = "Second Passkey".to_string();

        service.register_passkey(req1).await.unwrap();
        service.register_passkey(req2).await.unwrap();

        let result = service.list_passkeys("user-123").await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 2);
    }

    #[tokio::test]
    async fn test_list_passkeys_empty() {
        let service = PasskeyService::new();

        let result = service.list_passkeys("nonexistent-user").await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_link_smart_account() {
        let service = PasskeyService::new();
        let request = test_request();

        service.register_passkey(request).await.unwrap();

        let link_request = LinkAccountRequest {
            user_id: "user-123".to_string(),
            credential_id: "cred-abc".to_string(),
            smart_account_address: "0x1234567890abcdef1234567890abcdef12345678".to_string(),
        };

        let result = service.link_smart_account(link_request).await;
        assert!(result.is_ok());

        let cred = service.get_passkey("user-123", "cred-abc").await.unwrap();
        assert_eq!(
            cred.smart_account_address,
            Some("0x1234567890abcdef1234567890abcdef12345678".to_string())
        );
    }

    #[tokio::test]
    async fn test_deactivate_passkey() {
        let service = PasskeyService::new();
        let request = test_request();

        service.register_passkey(request).await.unwrap();
        service
            .deactivate_passkey("user-123", "cred-abc")
            .await
            .unwrap();

        // Deactivated credential should not be found via get_passkey
        let result = service.get_passkey("user-123", "cred-abc").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_mark_used() {
        let service = PasskeyService::new();
        let request = test_request();

        service.register_passkey(request).await.unwrap();

        let cred_before = service.get_passkey("user-123", "cred-abc").await.unwrap();
        assert!(cred_before.last_used_at.is_none());

        service.mark_used("user-123", "cred-abc").await.unwrap();

        let cred_after = service.get_passkey("user-123", "cred-abc").await.unwrap();
        assert!(cred_after.last_used_at.is_some());
    }

    #[tokio::test]
    async fn test_credential_count() {
        let service = PasskeyService::new();

        assert_eq!(service.credential_count("user-123").await, 0);

        let req1 = test_request();
        service.register_passkey(req1).await.unwrap();
        assert_eq!(service.credential_count("user-123").await, 1);

        let mut req2 = test_request();
        req2.credential_id = "cred-def".to_string();
        service.register_passkey(req2).await.unwrap();
        assert_eq!(service.credential_count("user-123").await, 2);

        service
            .deactivate_passkey("user-123", "cred-abc")
            .await
            .unwrap();
        assert_eq!(service.credential_count("user-123").await, 1);
    }

    #[tokio::test]
    async fn test_invalid_public_key_hex() {
        let service = PasskeyService::new();
        let mut request = test_request();
        request.public_key_x = "GGGG".to_string(); // Invalid hex

        let result = service.register_passkey(request).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            PasskeyError::InvalidPublicKey(_)
        ));
    }

    #[tokio::test]
    async fn test_empty_credential_id() {
        let service = PasskeyService::new();
        let mut request = test_request();
        request.credential_id = "".to_string();

        let result = service.register_passkey(request).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            PasskeyError::InvalidCredentialId
        ));
    }

    #[tokio::test]
    async fn test_empty_user_id() {
        let service = PasskeyService::new();
        let mut request = test_request();
        request.user_id = "".to_string();

        let result = service.register_passkey(request).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), PasskeyError::InvalidUserId));
    }

    #[tokio::test]
    async fn test_public_key_with_0x_prefix() {
        let service = PasskeyService::new();
        let mut request = test_request();
        request.public_key_x =
            "0x6B17D1F2E12C4247F8BCE6E563A440F277037D812DEB33A0F4A13945D898C296".to_string();

        let result = service.register_passkey(request).await;
        assert!(result.is_ok());
    }
}
