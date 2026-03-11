//! ZK Credential Issuer
//!
//! Issues verifiable credentials to users who have proven their KYC status
//! via zero-knowledge proofs. Credentials use EIP-712 style typed data
//! signing (simulated with HMAC-SHA256).

use chrono::{DateTime, Duration, Utc};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tracing::info;
use uuid::Uuid;

type HmacSha256 = Hmac<Sha256>;

/// A zero-knowledge KYC credential.
///
/// This credential proves that a user has completed KYC without
/// revealing any personal data. Only the commitment hash is stored.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZkCredential {
    /// Unique credential identifier
    pub id: String,
    /// The commitment hash from the ZK proof
    pub commitment_hash: String,
    /// When the credential was issued
    pub issued_at: DateTime<Utc>,
    /// When the credential expires
    pub expires_at: DateTime<Utc>,
    /// EIP-712 style signature (simulated with HMAC)
    pub issuer_signature: String,
}

/// Status of a credential.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CredentialStatus {
    Active,
    Expired,
    Revoked,
}

/// Internal credential record with revocation tracking.
#[derive(Debug, Clone)]
struct CredentialRecord {
    credential: ZkCredential,
    revoked: bool,
    revoked_at: Option<DateTime<Utc>>,
}

/// Issuer of ZK-KYC credentials.
///
/// Signs credentials using HMAC-SHA256 (simulating EIP-712 typed data signing).
/// In production, this would use actual EIP-712 signing with an Ethereum private key.
pub struct ZkCredentialIssuer {
    /// Signing key for credentials
    signing_key: Vec<u8>,
    /// Default credential validity duration
    validity_duration: Duration,
    /// Issued credentials: credential_id -> CredentialRecord
    credentials: Arc<RwLock<HashMap<String, CredentialRecord>>>,
    /// User -> credential IDs mapping
    user_credentials: Arc<RwLock<HashMap<String, Vec<String>>>>,
}

impl ZkCredentialIssuer {
    /// Create a new credential issuer with the given signing key.
    pub fn new(signing_key: Vec<u8>) -> Self {
        Self {
            signing_key,
            validity_duration: Duration::days(365), // 1 year default
            credentials: Arc::new(RwLock::new(HashMap::new())),
            user_credentials: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a new credential issuer with a custom validity duration.
    pub fn with_validity(signing_key: Vec<u8>, validity_days: i64) -> Self {
        Self {
            signing_key,
            validity_duration: Duration::days(validity_days),
            credentials: Arc::new(RwLock::new(HashMap::new())),
            user_credentials: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Issue a new credential for a user with the given commitment hash.
    pub fn issue_credential(&self, user_id: &str, commitment_hash: &str) -> ZkCredential {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let expires_at = now + self.validity_duration;

        // EIP-712 style typed data signing (simulated)
        let signature = self.sign_credential(&id, commitment_hash, now, expires_at);

        let credential = ZkCredential {
            id: id.clone(),
            commitment_hash: commitment_hash.to_string(),
            issued_at: now,
            expires_at,
            issuer_signature: signature,
        };

        // Store the credential
        {
            let mut creds = self.credentials.write().unwrap();
            creds.insert(
                id.clone(),
                CredentialRecord {
                    credential: credential.clone(),
                    revoked: false,
                    revoked_at: None,
                },
            );
        }

        // Track user -> credential mapping
        {
            let mut user_creds = self.user_credentials.write().unwrap();
            user_creds
                .entry(user_id.to_string())
                .or_default()
                .push(id.clone());
        }

        info!(
            user_id = user_id,
            credential_id = %id,
            commitment = commitment_hash,
            "ZK credential issued"
        );

        credential
    }

    /// Verify a credential's signature and status.
    pub fn verify_credential(&self, credential: &ZkCredential) -> bool {
        let now = Utc::now();

        // Check expiry
        if now > credential.expires_at {
            return false;
        }

        // Check revocation
        {
            let creds = self.credentials.read().unwrap();
            if let Some(record) = creds.get(&credential.id) {
                if record.revoked {
                    return false;
                }
            }
        }

        // Verify signature
        let expected_sig = self.sign_credential(
            &credential.id,
            &credential.commitment_hash,
            credential.issued_at,
            credential.expires_at,
        );

        credential.issuer_signature == expected_sig
    }

    /// Revoke a credential by its ID.
    pub fn revoke_credential(&self, credential_id: &str) -> bool {
        let mut creds = self.credentials.write().unwrap();
        if let Some(record) = creds.get_mut(credential_id) {
            if record.revoked {
                return false; // Already revoked
            }
            record.revoked = true;
            record.revoked_at = Some(Utc::now());

            info!(credential_id = credential_id, "ZK credential revoked");
            true
        } else {
            false
        }
    }

    /// Get the status of a credential.
    pub fn credential_status(&self, credential_id: &str) -> Option<CredentialStatus> {
        let creds = self.credentials.read().unwrap();
        creds.get(credential_id).map(|record| {
            if record.revoked {
                CredentialStatus::Revoked
            } else if Utc::now() > record.credential.expires_at {
                CredentialStatus::Expired
            } else {
                CredentialStatus::Active
            }
        })
    }

    /// Get all credential IDs for a user.
    pub fn get_user_credentials(&self, user_id: &str) -> Vec<String> {
        let user_creds = self.user_credentials.read().unwrap();
        user_creds.get(user_id).cloned().unwrap_or_default()
    }

    /// EIP-712 style typed data signing (simulated with HMAC-SHA256).
    ///
    /// In production, the message would be:
    /// ```text
    /// EIP712Domain { name: "RampOS ZK-KYC", version: "1", chainId: ... }
    /// ZkCredential { id, commitment_hash, issued_at, expires_at }
    /// ```
    fn sign_credential(
        &self,
        id: &str,
        commitment_hash: &str,
        issued_at: DateTime<Utc>,
        expires_at: DateTime<Utc>,
    ) -> String {
        let mut mac =
            HmacSha256::new_from_slice(&self.signing_key).expect("HMAC accepts any key length");

        // Domain separator (simulated EIP-712)
        mac.update(b"EIP712:RampOS-ZK-KYC:v1:");

        // Struct hash
        mac.update(id.as_bytes());
        mac.update(b":");
        mac.update(commitment_hash.as_bytes());
        mac.update(b":");
        mac.update(issued_at.timestamp().to_string().as_bytes());
        mac.update(b":");
        mac.update(expires_at.timestamp().to_string().as_bytes());

        hex::encode(mac.finalize().into_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_issuer() -> ZkCredentialIssuer {
        ZkCredentialIssuer::new(b"test-signing-key-for-credentials".to_vec())
    }

    #[test]
    fn test_issue_credential() {
        let issuer = test_issuer();
        let commitment = hex::encode([0xAAu8; 32]);

        let cred = issuer.issue_credential("user-1", &commitment);

        assert!(!cred.id.is_empty());
        assert_eq!(cred.commitment_hash, commitment);
        assert!(cred.expires_at > cred.issued_at);
        assert!(!cred.issuer_signature.is_empty());
    }

    #[test]
    fn test_verify_valid_credential() {
        let issuer = test_issuer();
        let commitment = hex::encode([0xBBu8; 32]);

        let cred = issuer.issue_credential("user-2", &commitment);
        assert!(issuer.verify_credential(&cred));
    }

    #[test]
    fn test_verify_tampered_commitment() {
        let issuer = test_issuer();
        let commitment = hex::encode([0xCCu8; 32]);

        let mut cred = issuer.issue_credential("user-3", &commitment);
        cred.commitment_hash = hex::encode([0xFFu8; 32]); // Tamper

        assert!(!issuer.verify_credential(&cred));
    }

    #[test]
    fn test_verify_tampered_signature() {
        let issuer = test_issuer();
        let commitment = hex::encode([0xDDu8; 32]);

        let mut cred = issuer.issue_credential("user-4", &commitment);
        cred.issuer_signature = "deadbeef".to_string(); // Tamper

        assert!(!issuer.verify_credential(&cred));
    }

    #[test]
    fn test_verify_expired_credential() {
        let issuer = ZkCredentialIssuer::with_validity(
            b"test-key".to_vec(),
            -1, // Already expired (negative days)
        );
        let commitment = hex::encode([0xEEu8; 32]);

        let cred = issuer.issue_credential("user-5", &commitment);
        assert!(!issuer.verify_credential(&cred));
    }

    #[test]
    fn test_revoke_credential() {
        let issuer = test_issuer();
        let commitment = hex::encode([0x11u8; 32]);

        let cred = issuer.issue_credential("user-6", &commitment);
        assert!(issuer.verify_credential(&cred));

        let revoked = issuer.revoke_credential(&cred.id);
        assert!(revoked);

        // After revocation, verification should fail
        assert!(!issuer.verify_credential(&cred));
    }

    #[test]
    fn test_revoke_already_revoked() {
        let issuer = test_issuer();
        let commitment = hex::encode([0x22u8; 32]);

        let cred = issuer.issue_credential("user-7", &commitment);

        assert!(issuer.revoke_credential(&cred.id));
        assert!(!issuer.revoke_credential(&cred.id)); // Already revoked
    }

    #[test]
    fn test_revoke_nonexistent() {
        let issuer = test_issuer();
        assert!(!issuer.revoke_credential("no-such-id"));
    }

    #[test]
    fn test_credential_status_active() {
        let issuer = test_issuer();
        let commitment = hex::encode([0x33u8; 32]);

        let cred = issuer.issue_credential("user-8", &commitment);
        assert_eq!(
            issuer.credential_status(&cred.id),
            Some(CredentialStatus::Active)
        );
    }

    #[test]
    fn test_credential_status_revoked() {
        let issuer = test_issuer();
        let commitment = hex::encode([0x44u8; 32]);

        let cred = issuer.issue_credential("user-9", &commitment);
        issuer.revoke_credential(&cred.id);

        assert_eq!(
            issuer.credential_status(&cred.id),
            Some(CredentialStatus::Revoked)
        );
    }

    #[test]
    fn test_credential_status_expired() {
        let issuer = ZkCredentialIssuer::with_validity(b"test-key".to_vec(), -1);
        let commitment = hex::encode([0x55u8; 32]);

        let cred = issuer.issue_credential("user-10", &commitment);
        assert_eq!(
            issuer.credential_status(&cred.id),
            Some(CredentialStatus::Expired)
        );
    }

    #[test]
    fn test_credential_status_nonexistent() {
        let issuer = test_issuer();
        assert_eq!(issuer.credential_status("ghost"), None);
    }

    #[test]
    fn test_get_user_credentials() {
        let issuer = test_issuer();

        let c1 = issuer.issue_credential("user-11", &hex::encode([0x66u8; 32]));
        let c2 = issuer.issue_credential("user-11", &hex::encode([0x77u8; 32]));

        let ids = issuer.get_user_credentials("user-11");
        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&c1.id));
        assert!(ids.contains(&c2.id));
    }

    #[test]
    fn test_get_user_credentials_empty() {
        let issuer = test_issuer();
        assert!(issuer.get_user_credentials("nobody").is_empty());
    }

    #[test]
    fn test_different_keys_produce_different_signatures() {
        let issuer_a = ZkCredentialIssuer::new(b"key-alpha".to_vec());
        let issuer_b = ZkCredentialIssuer::new(b"key-bravo".to_vec());

        let commitment = hex::encode([0x88u8; 32]);
        let cred_a = issuer_a.issue_credential("user-12", &commitment);
        let cred_b = issuer_b.issue_credential("user-12", &commitment);

        // Signatures should differ
        assert_ne!(cred_a.issuer_signature, cred_b.issuer_signature);

        // Cross-verification should fail
        assert!(!issuer_b.verify_credential(&cred_a));
        assert!(!issuer_a.verify_credential(&cred_b));
    }

    #[test]
    fn test_full_lifecycle() {
        let issuer = test_issuer();
        let commitment = hex::encode([0x99u8; 32]);

        // Issue
        let cred = issuer.issue_credential("user-13", &commitment);
        assert_eq!(
            issuer.credential_status(&cred.id),
            Some(CredentialStatus::Active)
        );
        assert!(issuer.verify_credential(&cred));

        // Verify user has it
        let user_creds = issuer.get_user_credentials("user-13");
        assert_eq!(user_creds.len(), 1);

        // Revoke
        assert!(issuer.revoke_credential(&cred.id));
        assert_eq!(
            issuer.credential_status(&cred.id),
            Some(CredentialStatus::Revoked)
        );
        assert!(!issuer.verify_credential(&cred));
    }
}
