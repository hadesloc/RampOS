//! ZK-KYC Service
//!
//! Manages zero-knowledge proof challenges and verification for KYC.
//! Users generate proofs off-chain that attest to their KYC level
//! without revealing personal data.

use chrono::{DateTime, Utc};
use hmac::{Hmac, Mac};
use rand::Rng;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tracing::info;

use crate::types::KycTier;

type HmacSha256 = Hmac<Sha256>;

/// A challenge issued to a user who wants to prove their KYC status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZkKycProofRequest {
    /// Unique challenge nonce (hex-encoded)
    pub challenge: String,
    /// Required minimum KYC tier the user must prove
    pub required_kyc_level: KycTier,
    /// If set, user must prove nationality is in this list
    pub allowed_nationalities: Vec<String>,
    /// When the challenge was created
    pub created_at: DateTime<Utc>,
    /// When the challenge expires (5 minutes by default)
    pub expires_at: DateTime<Utc>,
}

/// A zero-knowledge proof submitted by the user.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZkKycProof {
    /// The commitment hash (H(user_data || salt))
    pub commitment_hash: String,
    /// Serialized proof data (simulated; in production this would be a Groth16 proof)
    pub proof_data: Vec<u8>,
    /// Public inputs: [challenge, required_kyc_level]
    pub public_inputs: Vec<String>,
}

/// Result of verifying a ZK-KYC proof.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationResult {
    /// Whether the proof is valid
    pub valid: bool,
    /// The commitment hash from the proof
    pub commitment_hash: String,
    /// The KYC tier that was proven
    pub proven_tier: KycTier,
    /// Verification timestamp
    pub verified_at: DateTime<Utc>,
    /// Reason for rejection, if invalid
    pub rejection_reason: Option<String>,
}

/// Internal record of a verified user.
#[derive(Debug, Clone)]
struct VerifiedUser {
    commitment_hash: String,
    verified_at: DateTime<Utc>,
    proven_tier: KycTier,
}

/// Service for ZK-KYC challenge creation and proof verification.
pub struct ZkKycService {
    /// HMAC key for proof verification (simulated verifier secret)
    verification_key: Vec<u8>,
    /// Pending challenges: user_id -> ZkKycProofRequest
    pending_challenges: Arc<RwLock<HashMap<String, ZkKycProofRequest>>>,
    /// Verified users: user_id -> VerifiedUser
    verified_users: Arc<RwLock<HashMap<String, VerifiedUser>>>,
    /// Challenge validity duration in seconds
    challenge_ttl_secs: i64,
}

impl ZkKycService {
    /// Create a new ZkKycService with the given verification key.
    pub fn new(verification_key: Vec<u8>) -> Self {
        Self {
            verification_key,
            pending_challenges: Arc::new(RwLock::new(HashMap::new())),
            verified_users: Arc::new(RwLock::new(HashMap::new())),
            challenge_ttl_secs: 300, // 5 minutes
        }
    }

    /// Create a new ZkKycService with a custom challenge TTL.
    pub fn with_ttl(verification_key: Vec<u8>, challenge_ttl_secs: i64) -> Self {
        Self {
            verification_key,
            pending_challenges: Arc::new(RwLock::new(HashMap::new())),
            verified_users: Arc::new(RwLock::new(HashMap::new())),
            challenge_ttl_secs,
        }
    }

    /// Generate a new challenge for the given user.
    ///
    /// The challenge is a random 32-byte hex string that the user must
    /// include in their proof to prevent replay attacks.
    pub fn create_challenge(
        &self,
        user_id: &str,
        required_kyc_level: KycTier,
        allowed_nationalities: Vec<String>,
    ) -> ZkKycProofRequest {
        let mut rng = rand::thread_rng();
        let challenge_bytes: [u8; 32] = rng.gen();
        let challenge = hex::encode(challenge_bytes);

        let now = Utc::now();
        let expires_at = now + chrono::Duration::seconds(self.challenge_ttl_secs);

        let request = ZkKycProofRequest {
            challenge,
            required_kyc_level,
            allowed_nationalities,
            created_at: now,
            expires_at,
        };

        {
            let mut challenges = self.pending_challenges.write().unwrap();
            challenges.insert(user_id.to_string(), request.clone());
        }

        info!(
            user_id = user_id,
            required_level = ?required_kyc_level,
            "ZK-KYC challenge created"
        );

        request
    }

    /// Verify a ZK-KYC proof submitted by a user.
    ///
    /// In a real system, this would verify a Groth16/PLONK proof.
    /// Here we simulate verification by checking:
    /// 1. The challenge is still valid (not expired)
    /// 2. The commitment hash is well-formed
    /// 3. The proof data contains a valid HMAC over (commitment || challenge)
    pub fn verify_proof(&self, user_id: &str, proof: &ZkKycProof) -> VerificationResult {
        let now = Utc::now();

        // Check that we have a pending challenge for this user
        let challenge = {
            let challenges = self.pending_challenges.read().unwrap();
            challenges.get(user_id).cloned()
        };

        let challenge = match challenge {
            Some(c) => c,
            None => {
                return VerificationResult {
                    valid: false,
                    commitment_hash: proof.commitment_hash.clone(),
                    proven_tier: KycTier::Tier0,
                    verified_at: now,
                    rejection_reason: Some("No pending challenge for user".to_string()),
                };
            }
        };

        // Check challenge expiry
        if now > challenge.expires_at {
            return VerificationResult {
                valid: false,
                commitment_hash: proof.commitment_hash.clone(),
                proven_tier: KycTier::Tier0,
                verified_at: now,
                rejection_reason: Some("Challenge has expired".to_string()),
            };
        }

        // Validate commitment hash is a valid hex string of correct length (32 bytes = 64 hex chars)
        if proof.commitment_hash.len() != 64 || hex::decode(&proof.commitment_hash).is_err() {
            return VerificationResult {
                valid: false,
                commitment_hash: proof.commitment_hash.clone(),
                proven_tier: KycTier::Tier0,
                verified_at: now,
                rejection_reason: Some("Invalid commitment hash format".to_string()),
            };
        }

        // Validate public inputs contain the challenge
        if proof.public_inputs.is_empty() || proof.public_inputs[0] != challenge.challenge {
            return VerificationResult {
                valid: false,
                commitment_hash: proof.commitment_hash.clone(),
                proven_tier: KycTier::Tier0,
                verified_at: now,
                rejection_reason: Some("Proof does not match challenge".to_string()),
            };
        }

        // Simulated proof verification:
        // Verify HMAC(verification_key, commitment_hash || challenge) == proof_data
        let expected_mac = self.compute_proof_mac(&proof.commitment_hash, &challenge.challenge);

        if proof.proof_data != expected_mac {
            return VerificationResult {
                valid: false,
                commitment_hash: proof.commitment_hash.clone(),
                proven_tier: KycTier::Tier0,
                verified_at: now,
                rejection_reason: Some("Invalid proof".to_string()),
            };
        }

        // Remove used challenge (one-time use)
        {
            let mut challenges = self.pending_challenges.write().unwrap();
            challenges.remove(user_id);
        }

        info!(
            user_id = user_id,
            proven_tier = ?challenge.required_kyc_level,
            "ZK-KYC proof verified successfully"
        );

        VerificationResult {
            valid: true,
            commitment_hash: proof.commitment_hash.clone(),
            proven_tier: challenge.required_kyc_level,
            verified_at: now,
            rejection_reason: None,
        }
    }

    /// Store a successful verification for a user.
    pub fn store_verification(&self, user_id: &str, commitment_hash: &str, proven_tier: KycTier) {
        let record = VerifiedUser {
            commitment_hash: commitment_hash.to_string(),
            verified_at: Utc::now(),
            proven_tier,
        };

        let mut users = self.verified_users.write().unwrap();
        users.insert(user_id.to_string(), record);

        info!(
            user_id = user_id,
            commitment = commitment_hash,
            "ZK-KYC verification stored"
        );
    }

    /// Check if a user has a stored verification.
    pub fn is_verified(&self, user_id: &str) -> bool {
        let users = self.verified_users.read().unwrap();
        users.contains_key(user_id)
    }

    /// Get the verified tier for a user, if any.
    pub fn get_verified_tier(&self, user_id: &str) -> Option<KycTier> {
        let users = self.verified_users.read().unwrap();
        users.get(user_id).map(|u| u.proven_tier)
    }

    /// Remove a user's verification (e.g., on revocation).
    pub fn remove_verification(&self, user_id: &str) -> bool {
        let mut users = self.verified_users.write().unwrap();
        users.remove(user_id).is_some()
    }

    /// Compute the simulated proof MAC.
    /// In production, this would be replaced by actual ZK circuit verification.
    pub fn compute_proof_mac(&self, commitment_hash: &str, challenge: &str) -> Vec<u8> {
        let mut mac = HmacSha256::new_from_slice(&self.verification_key)
            .expect("HMAC accepts any key length");
        mac.update(commitment_hash.as_bytes());
        mac.update(challenge.as_bytes());
        mac.finalize().into_bytes().to_vec()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::KycTier;

    fn test_service() -> ZkKycService {
        ZkKycService::new(b"test-verification-key-32bytes!!".to_vec())
    }

    #[test]
    fn test_create_challenge_returns_valid_request() {
        let svc = test_service();
        let req = svc.create_challenge("user-1", KycTier::Tier1, vec![]);

        assert_eq!(req.challenge.len(), 64); // 32 bytes hex
        assert_eq!(req.required_kyc_level, KycTier::Tier1);
        assert!(req.allowed_nationalities.is_empty());
        assert!(req.expires_at > req.created_at);
    }

    #[test]
    fn test_create_challenge_with_nationalities() {
        let svc = test_service();
        let nations = vec!["VN".to_string(), "SG".to_string()];
        let req = svc.create_challenge("user-2", KycTier::Tier2, nations.clone());

        assert_eq!(req.allowed_nationalities, nations);
        assert_eq!(req.required_kyc_level, KycTier::Tier2);
    }

    #[test]
    fn test_create_challenge_unique_per_call() {
        let svc = test_service();
        let req1 = svc.create_challenge("user-3", KycTier::Tier1, vec![]);
        let req2 = svc.create_challenge("user-3", KycTier::Tier1, vec![]);

        // Each call should produce a different challenge
        assert_ne!(req1.challenge, req2.challenge);
    }

    #[test]
    fn test_verify_proof_success() {
        let svc = test_service();
        let req = svc.create_challenge("user-4", KycTier::Tier1, vec![]);

        let commitment_hash = hex::encode([0xABu8; 32]);
        let mac = svc.compute_proof_mac(&commitment_hash, &req.challenge);

        let proof = ZkKycProof {
            commitment_hash: commitment_hash.clone(),
            proof_data: mac,
            public_inputs: vec![req.challenge.clone()],
        };

        let result = svc.verify_proof("user-4", &proof);
        assert!(
            result.valid,
            "Expected valid proof: {:?}",
            result.rejection_reason
        );
        assert_eq!(result.commitment_hash, commitment_hash);
        assert_eq!(result.proven_tier, KycTier::Tier1);
        assert!(result.rejection_reason.is_none());
    }

    #[test]
    fn test_verify_proof_no_pending_challenge() {
        let svc = test_service();

        let proof = ZkKycProof {
            commitment_hash: hex::encode([0xCDu8; 32]),
            proof_data: vec![0; 32],
            public_inputs: vec!["nonexistent-challenge".to_string()],
        };

        let result = svc.verify_proof("no-such-user", &proof);
        assert!(!result.valid);
        assert_eq!(
            result.rejection_reason.as_deref(),
            Some("No pending challenge for user")
        );
    }

    #[test]
    fn test_verify_proof_expired_challenge() {
        let svc = ZkKycService::with_ttl(b"test-key".to_vec(), -1); // Already expired
        let req = svc.create_challenge("user-5", KycTier::Tier1, vec![]);

        let commitment_hash = hex::encode([0xEFu8; 32]);
        let mac = svc.compute_proof_mac(&commitment_hash, &req.challenge);

        let proof = ZkKycProof {
            commitment_hash,
            proof_data: mac,
            public_inputs: vec![req.challenge],
        };

        let result = svc.verify_proof("user-5", &proof);
        assert!(!result.valid);
        assert_eq!(
            result.rejection_reason.as_deref(),
            Some("Challenge has expired")
        );
    }

    #[test]
    fn test_verify_proof_invalid_commitment_hash() {
        let svc = test_service();
        let req = svc.create_challenge("user-6", KycTier::Tier1, vec![]);

        let proof = ZkKycProof {
            commitment_hash: "too-short".to_string(),
            proof_data: vec![0; 32],
            public_inputs: vec![req.challenge],
        };

        let result = svc.verify_proof("user-6", &proof);
        assert!(!result.valid);
        assert_eq!(
            result.rejection_reason.as_deref(),
            Some("Invalid commitment hash format")
        );
    }

    #[test]
    fn test_verify_proof_wrong_challenge() {
        let svc = test_service();
        let _req = svc.create_challenge("user-7", KycTier::Tier1, vec![]);

        let commitment_hash = hex::encode([0xAAu8; 32]);

        let proof = ZkKycProof {
            commitment_hash,
            proof_data: vec![0; 32],
            public_inputs: vec!["wrong-challenge".to_string()],
        };

        let result = svc.verify_proof("user-7", &proof);
        assert!(!result.valid);
        assert_eq!(
            result.rejection_reason.as_deref(),
            Some("Proof does not match challenge")
        );
    }

    #[test]
    fn test_verify_proof_invalid_mac() {
        let svc = test_service();
        let req = svc.create_challenge("user-8", KycTier::Tier1, vec![]);

        let commitment_hash = hex::encode([0xBBu8; 32]);

        let proof = ZkKycProof {
            commitment_hash,
            proof_data: vec![0; 32], // Wrong MAC
            public_inputs: vec![req.challenge],
        };

        let result = svc.verify_proof("user-8", &proof);
        assert!(!result.valid);
        assert_eq!(result.rejection_reason.as_deref(), Some("Invalid proof"));
    }

    #[test]
    fn test_verify_proof_consumes_challenge() {
        let svc = test_service();
        let req = svc.create_challenge("user-9", KycTier::Tier1, vec![]);

        let commitment_hash = hex::encode([0xCCu8; 32]);
        let mac = svc.compute_proof_mac(&commitment_hash, &req.challenge);

        let proof = ZkKycProof {
            commitment_hash: commitment_hash.clone(),
            proof_data: mac.clone(),
            public_inputs: vec![req.challenge.clone()],
        };

        // First verification succeeds
        let result = svc.verify_proof("user-9", &proof);
        assert!(result.valid);

        // Second verification should fail (challenge consumed)
        let result2 = svc.verify_proof("user-9", &proof);
        assert!(!result2.valid);
        assert_eq!(
            result2.rejection_reason.as_deref(),
            Some("No pending challenge for user")
        );
    }

    #[test]
    fn test_store_and_check_verification() {
        let svc = test_service();

        assert!(!svc.is_verified("user-10"));

        svc.store_verification("user-10", &hex::encode([0xDDu8; 32]), KycTier::Tier2);
        assert!(svc.is_verified("user-10"));
        assert_eq!(svc.get_verified_tier("user-10"), Some(KycTier::Tier2));
    }

    #[test]
    fn test_remove_verification() {
        let svc = test_service();

        svc.store_verification("user-11", &hex::encode([0xEEu8; 32]), KycTier::Tier1);
        assert!(svc.is_verified("user-11"));

        let removed = svc.remove_verification("user-11");
        assert!(removed);
        assert!(!svc.is_verified("user-11"));
    }

    #[test]
    fn test_remove_nonexistent_verification() {
        let svc = test_service();
        let removed = svc.remove_verification("ghost-user");
        assert!(!removed);
    }

    #[test]
    fn test_get_verified_tier_none() {
        let svc = test_service();
        assert_eq!(svc.get_verified_tier("nobody"), None);
    }

    #[test]
    fn test_full_flow_challenge_verify_store() {
        let svc = test_service();

        // 1. Create challenge
        let req = svc.create_challenge("user-12", KycTier::Tier2, vec!["VN".to_string()]);

        // 2. User generates proof off-chain
        let commitment_hash = hex::encode([0x42u8; 32]);
        let mac = svc.compute_proof_mac(&commitment_hash, &req.challenge);

        let proof = ZkKycProof {
            commitment_hash: commitment_hash.clone(),
            proof_data: mac,
            public_inputs: vec![req.challenge],
        };

        // 3. Verify proof
        let result = svc.verify_proof("user-12", &proof);
        assert!(result.valid);

        // 4. Store verification
        svc.store_verification("user-12", &result.commitment_hash, result.proven_tier);
        assert!(svc.is_verified("user-12"));
        assert_eq!(svc.get_verified_tier("user-12"), Some(KycTier::Tier2));
    }

    #[test]
    fn test_different_users_independent_challenges() {
        let svc = test_service();

        let req_a = svc.create_challenge("alice", KycTier::Tier1, vec![]);
        let req_b = svc.create_challenge("bob", KycTier::Tier2, vec![]);

        // Verify Alice
        let commit_a = hex::encode([0x11u8; 32]);
        let mac_a = svc.compute_proof_mac(&commit_a, &req_a.challenge);
        let proof_a = ZkKycProof {
            commitment_hash: commit_a,
            proof_data: mac_a,
            public_inputs: vec![req_a.challenge],
        };
        let res_a = svc.verify_proof("alice", &proof_a);
        assert!(res_a.valid);
        assert_eq!(res_a.proven_tier, KycTier::Tier1);

        // Verify Bob
        let commit_b = hex::encode([0x22u8; 32]);
        let mac_b = svc.compute_proof_mac(&commit_b, &req_b.challenge);
        let proof_b = ZkKycProof {
            commitment_hash: commit_b,
            proof_data: mac_b,
            public_inputs: vec![req_b.challenge],
        };
        let res_b = svc.verify_proof("bob", &proof_b);
        assert!(res_b.valid);
        assert_eq!(res_b.proven_tier, KycTier::Tier2);
    }
}
