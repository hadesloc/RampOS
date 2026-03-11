//! MPC Signing Service
//!
//! Implements threshold signing sessions with a 2-of-3 approval workflow.
//! Each signing request requires approval from at least 2 parties before
//! partial signatures are combined into a final signature.

use chrono::{DateTime, Utc};
use rand::Rng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Mutex;
use tracing::info;
use uuid::Uuid;

/// Threshold required for signing (2 out of 3).
const SIGNING_THRESHOLD: usize = 2;

/// A request to sign a message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SigningRequest {
    /// The hash of the message to be signed
    pub message_hash: Vec<u8>,
    /// The party initiating the signing request
    pub requester_party_id: u8,
    /// The parties whose approval is being sought
    pub approval_parties: Vec<u8>,
}

/// Status of a signing session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SigningSessionStatus {
    /// Waiting for approvals
    Pending,
    /// Threshold met, ready to combine
    Approved,
    /// Final signature produced
    Signed,
    /// Signing was rejected
    Rejected,
}

/// A partial signature contribution from one party.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartialSignature {
    /// Party that produced this partial signature
    pub party_id: u8,
    /// The partial signature bytes (simulated)
    pub signature_bytes: Vec<u8>,
    /// When the approval was given
    pub approved_at: DateTime<Utc>,
}

/// A signing session tracking the lifecycle of a threshold signing operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SigningSession {
    /// Unique session identifier
    pub id: String,
    /// The user requesting the signature
    pub user_id: String,
    /// The signing request details
    pub request: SigningRequest,
    /// Current session status
    pub status: SigningSessionStatus,
    /// Partial signatures collected so far
    pub partial_signatures: Vec<PartialSignature>,
    /// The combined final signature (available when status is Signed)
    pub final_signature: Option<Vec<u8>>,
    /// When the session was created
    pub created_at: DateTime<Utc>,
    /// When the session was last updated
    pub updated_at: DateTime<Utc>,
}

/// Service for managing MPC signing sessions.
///
/// Handles the lifecycle of threshold signing: request creation,
/// partial signature collection, threshold checking, and signature combination.
pub struct MpcSigningService {
    /// Active signing sessions: session_id -> SigningSession
    sessions: Mutex<HashMap<String, SigningSession>>,
}

impl MpcSigningService {
    pub fn new() -> Self {
        Self {
            sessions: Mutex::new(HashMap::new()),
        }
    }

    /// Create a new signing request and session.
    ///
    /// The requester is automatically added as the first approver with a partial signature.
    pub fn create_signing_request(
        &self,
        user_id: &str,
        message_hash: Vec<u8>,
    ) -> ramp_common::Result<SigningSession> {
        if message_hash.is_empty() {
            return Err(ramp_common::Error::Validation(
                "message_hash cannot be empty".into(),
            ));
        }

        let session_id = Uuid::new_v4().to_string();
        let now = Utc::now();

        let request = SigningRequest {
            message_hash: message_hash.clone(),
            requester_party_id: 1,        // Default requester is party 1
            approval_parties: vec![2, 3], // Request approval from other parties
        };

        // Generate the requester's partial signature
        let partial_sig = self.generate_partial_signature(1, &message_hash);

        let session = SigningSession {
            id: session_id.clone(),
            user_id: user_id.to_string(),
            request,
            status: SigningSessionStatus::Pending,
            partial_signatures: vec![partial_sig],
            final_signature: None,
            created_at: now,
            updated_at: now,
        };

        self.sessions
            .lock()
            .unwrap()
            .insert(session_id.clone(), session.clone());

        info!(
            session_id = session_id.as_str(),
            user_id, "Created signing session"
        );

        Ok(session)
    }

    /// Approve a signing session by providing a partial signature from a party.
    ///
    /// Returns the updated session. If the threshold is met, status changes to Approved.
    pub fn approve_signing(
        &self,
        session_id: &str,
        party_id: u8,
        partial_sig: Vec<u8>,
    ) -> ramp_common::Result<SigningSession> {
        let mut sessions = self.sessions.lock().unwrap();
        let session = sessions.get_mut(session_id).ok_or_else(|| {
            ramp_common::Error::NotFound(format!("Signing session not found: {}", session_id))
        })?;

        // Validate session state
        if session.status != SigningSessionStatus::Pending {
            return Err(ramp_common::Error::Validation(format!(
                "Session is not pending, current status: {:?}",
                session.status
            )));
        }

        // Check if this party already approved
        if session
            .partial_signatures
            .iter()
            .any(|ps| ps.party_id == party_id)
        {
            return Err(ramp_common::Error::Validation(format!(
                "Party {} has already approved this session",
                party_id
            )));
        }

        // Validate party_id
        if party_id < 1 || party_id > 3 {
            return Err(ramp_common::Error::Validation(
                "party_id must be 1, 2, or 3".into(),
            ));
        }

        // Add partial signature
        session.partial_signatures.push(PartialSignature {
            party_id,
            signature_bytes: partial_sig,
            approved_at: Utc::now(),
        });

        // Check if threshold is met
        if session.partial_signatures.len() >= SIGNING_THRESHOLD {
            session.status = SigningSessionStatus::Approved;
            info!(
                session_id,
                approvals = session.partial_signatures.len(),
                "Signing threshold met"
            );
        }

        session.updated_at = Utc::now();

        Ok(session.clone())
    }

    /// Combine partial signatures into a final signature.
    ///
    /// Requires the session to be in Approved status (threshold met).
    /// In production, this would use MPC signature combination.
    /// Here we simulate by hashing all partial signatures together with the message.
    pub fn combine_signatures(&self, session_id: &str) -> ramp_common::Result<Vec<u8>> {
        let mut sessions = self.sessions.lock().unwrap();
        let session = sessions.get_mut(session_id).ok_or_else(|| {
            ramp_common::Error::NotFound(format!("Signing session not found: {}", session_id))
        })?;

        if session.status != SigningSessionStatus::Approved {
            return Err(ramp_common::Error::Validation(format!(
                "Session must be Approved to combine signatures, current status: {:?}",
                session.status
            )));
        }

        // Simulate signature combination:
        // Hash(message_hash || partial_sig_1 || partial_sig_2 || ...)
        let mut hasher = Sha256::new();
        hasher.update(&session.request.message_hash);
        for ps in &session.partial_signatures {
            hasher.update(&ps.signature_bytes);
        }
        let combined_sig = hasher.finalize().to_vec();

        session.final_signature = Some(combined_sig.clone());
        session.status = SigningSessionStatus::Signed;
        session.updated_at = Utc::now();

        info!(
            session_id,
            sig_hex = hex::encode(&combined_sig),
            "Combined signatures into final signature"
        );

        Ok(combined_sig)
    }

    /// Reject a signing session.
    pub fn reject_signing(&self, session_id: &str) -> ramp_common::Result<SigningSession> {
        let mut sessions = self.sessions.lock().unwrap();
        let session = sessions.get_mut(session_id).ok_or_else(|| {
            ramp_common::Error::NotFound(format!("Signing session not found: {}", session_id))
        })?;

        if session.status == SigningSessionStatus::Signed {
            return Err(ramp_common::Error::Validation(
                "Cannot reject a session that is already signed".into(),
            ));
        }

        session.status = SigningSessionStatus::Rejected;
        session.updated_at = Utc::now();

        info!(session_id, "Signing session rejected");

        Ok(session.clone())
    }

    /// Retrieve a signing session by ID.
    pub fn get_session(&self, session_id: &str) -> Option<SigningSession> {
        self.sessions.lock().unwrap().get(session_id).cloned()
    }

    /// List all sessions for a user.
    pub fn list_user_sessions(&self, user_id: &str) -> Vec<SigningSession> {
        self.sessions
            .lock()
            .unwrap()
            .values()
            .filter(|s| s.user_id == user_id)
            .cloned()
            .collect()
    }

    /// Generate a simulated partial signature for a party.
    fn generate_partial_signature(&self, party_id: u8, message_hash: &[u8]) -> PartialSignature {
        let mut hasher = Sha256::new();
        hasher.update(&[party_id]);
        hasher.update(message_hash);
        // Add some randomness to make each partial sig unique
        let mut rng = rand::thread_rng();
        let nonce: [u8; 16] = rng.gen();
        hasher.update(nonce);

        PartialSignature {
            party_id,
            signature_bytes: hasher.finalize().to_vec(),
            approved_at: Utc::now(),
        }
    }
}

impl Default for MpcSigningService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_message_hash() -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(b"test transaction data");
        hasher.finalize().to_vec()
    }

    #[test]
    fn test_create_signing_request() {
        let service = MpcSigningService::new();
        let session = service
            .create_signing_request("user-1", test_message_hash())
            .unwrap();

        assert_eq!(session.user_id, "user-1");
        assert_eq!(session.status, SigningSessionStatus::Pending);
        assert_eq!(session.partial_signatures.len(), 1);
        assert_eq!(session.partial_signatures[0].party_id, 1);
        assert!(session.final_signature.is_none());
    }

    #[test]
    fn test_create_signing_request_empty_hash() {
        let service = MpcSigningService::new();
        let result = service.create_signing_request("user-1", vec![]);
        assert!(result.is_err());
    }

    #[test]
    fn test_approve_signing_meets_threshold() {
        let service = MpcSigningService::new();
        let session = service
            .create_signing_request("user-1", test_message_hash())
            .unwrap();

        // Party 2 approves
        let updated = service
            .approve_signing(&session.id, 2, vec![1, 2, 3])
            .unwrap();

        // Threshold of 2 met: 1 (requester) + 1 (party 2)
        assert_eq!(updated.status, SigningSessionStatus::Approved);
        assert_eq!(updated.partial_signatures.len(), 2);
    }

    #[test]
    fn test_approve_signing_duplicate_party() {
        let service = MpcSigningService::new();
        let session = service
            .create_signing_request("user-1", test_message_hash())
            .unwrap();

        // Party 1 already approved (requester), try again
        let result = service.approve_signing(&session.id, 1, vec![1, 2, 3]);
        assert!(result.is_err());
    }

    #[test]
    fn test_approve_signing_invalid_party() {
        let service = MpcSigningService::new();
        let session = service
            .create_signing_request("user-1", test_message_hash())
            .unwrap();

        let result = service.approve_signing(&session.id, 4, vec![1, 2, 3]);
        assert!(result.is_err());
    }

    #[test]
    fn test_combine_signatures() {
        let service = MpcSigningService::new();
        let session = service
            .create_signing_request("user-1", test_message_hash())
            .unwrap();

        // Approve to meet threshold
        service
            .approve_signing(&session.id, 2, vec![4, 5, 6])
            .unwrap();

        // Combine
        let signature = service.combine_signatures(&session.id).unwrap();
        assert_eq!(signature.len(), 32); // SHA-256

        // Verify session is now Signed
        let final_session = service.get_session(&session.id).unwrap();
        assert_eq!(final_session.status, SigningSessionStatus::Signed);
        assert!(final_session.final_signature.is_some());
    }

    #[test]
    fn test_combine_signatures_not_approved() {
        let service = MpcSigningService::new();
        let session = service
            .create_signing_request("user-1", test_message_hash())
            .unwrap();

        // Try to combine without meeting threshold
        let result = service.combine_signatures(&session.id);
        assert!(result.is_err());
    }

    #[test]
    fn test_reject_signing() {
        let service = MpcSigningService::new();
        let session = service
            .create_signing_request("user-1", test_message_hash())
            .unwrap();

        let rejected = service.reject_signing(&session.id).unwrap();
        assert_eq!(rejected.status, SigningSessionStatus::Rejected);
    }

    #[test]
    fn test_reject_already_signed() {
        let service = MpcSigningService::new();
        let session = service
            .create_signing_request("user-1", test_message_hash())
            .unwrap();

        service
            .approve_signing(&session.id, 2, vec![4, 5, 6])
            .unwrap();
        service.combine_signatures(&session.id).unwrap();

        // Cannot reject after signing
        let result = service.reject_signing(&session.id);
        assert!(result.is_err());
    }

    #[test]
    fn test_get_session() {
        let service = MpcSigningService::new();
        let session = service
            .create_signing_request("user-1", test_message_hash())
            .unwrap();

        let retrieved = service.get_session(&session.id);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, session.id);

        let nonexistent = service.get_session("nonexistent");
        assert!(nonexistent.is_none());
    }

    #[test]
    fn test_list_user_sessions() {
        let service = MpcSigningService::new();

        service
            .create_signing_request("user-1", test_message_hash())
            .unwrap();
        service
            .create_signing_request("user-1", test_message_hash())
            .unwrap();
        service
            .create_signing_request("user-2", test_message_hash())
            .unwrap();

        let u1_sessions = service.list_user_sessions("user-1");
        assert_eq!(u1_sessions.len(), 2);

        let u2_sessions = service.list_user_sessions("user-2");
        assert_eq!(u2_sessions.len(), 1);
    }

    #[test]
    fn test_full_signing_workflow() {
        let service = MpcSigningService::new();
        let msg = test_message_hash();

        // Step 1: Create request (party 1 auto-approves)
        let session = service.create_signing_request("user-1", msg).unwrap();
        assert_eq!(session.status, SigningSessionStatus::Pending);

        // Step 2: Party 3 approves (threshold met: 2 of 3)
        let approved = service
            .approve_signing(&session.id, 3, vec![7, 8, 9])
            .unwrap();
        assert_eq!(approved.status, SigningSessionStatus::Approved);

        // Step 3: Combine into final signature
        let signature = service.combine_signatures(&session.id).unwrap();
        assert!(!signature.is_empty());

        // Step 4: Verify final state
        let final_session = service.get_session(&session.id).unwrap();
        assert_eq!(final_session.status, SigningSessionStatus::Signed);
        assert_eq!(final_session.final_signature.unwrap(), signature);
    }

    #[test]
    fn test_approve_after_rejection() {
        let service = MpcSigningService::new();
        let session = service
            .create_signing_request("user-1", test_message_hash())
            .unwrap();

        service.reject_signing(&session.id).unwrap();

        // Cannot approve a rejected session
        let result = service.approve_signing(&session.id, 2, vec![1, 2, 3]);
        assert!(result.is_err());
    }

    #[test]
    fn test_nonexistent_session() {
        let service = MpcSigningService::new();

        let result = service.approve_signing("nonexistent", 2, vec![1, 2, 3]);
        assert!(result.is_err());

        let result = service.combine_signatures("nonexistent");
        assert!(result.is_err());

        let result = service.reject_signing("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_all_three_parties_approve() {
        let service = MpcSigningService::new();
        let session = service
            .create_signing_request("user-1", test_message_hash())
            .unwrap();

        // Party 2 approves (threshold met)
        let s = service
            .approve_signing(&session.id, 2, vec![1, 2, 3])
            .unwrap();
        assert_eq!(s.status, SigningSessionStatus::Approved);

        // Party 3 also approves (already approved status, still pending acceptance)
        // This should fail since session is already Approved
        let result = service.approve_signing(&session.id, 3, vec![4, 5, 6]);
        assert!(result.is_err());
    }
}
