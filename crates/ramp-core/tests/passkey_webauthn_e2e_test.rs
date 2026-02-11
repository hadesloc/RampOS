//! E2E integration tests for the Passkey/WebAuthn feature (F06).
//!
//! These tests exercise the full WebAuthn ceremony lifecycle using
//! in-memory challenge and credential stores, validating:
//!
//! 1. Registration ceremony: generate challenge -> create credential -> store credential
//! 2. Authentication ceremony: generate challenge -> verify assertion -> session created
//! 3. Multiple credentials per user
//! 4. Credential revocation (deactivation)
//! 5. Challenge expiry (old challenges should fail)
//! 6. Cross-origin protection
//! 7. Replay attack prevention (same assertion used twice should fail)
//! 8. Concurrent registration/authentication attempts
//!
//! The approach follows the same pattern as `webhook_delivery_e2e_test.rs`:
//! in-memory repositories with synchronous access via `Arc<Mutex<..>>` or
//! `Arc<RwLock<..>>`.

use chrono::{Duration, Utc};
use ramp_core::service::passkey::{
    LinkAccountRequest, PasskeyError, PasskeyService, RegisterPasskeyRequest,
    RegisterPasskeyResponse,
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

// ============================================================================
// WebAuthn Ceremony Simulator
// ============================================================================
// The PasskeyService in ramp-core handles credential storage. To test
// the full WebAuthn ceremony (challenge generation, challenge validation,
// assertion verification, replay protection), we build a thin ceremony
// layer on top of it that mirrors the API handler logic in ramp-api.

/// A pending WebAuthn challenge stored server-side.
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct PendingChallenge {
    /// Random challenge bytes (base64url-encoded)
    challenge: String,
    /// The user email or identifier requesting the challenge
    user_email: String,
    /// The relying party ID (origin check)
    rp_id: String,
    /// When this challenge was created
    created_at: chrono::DateTime<Utc>,
    /// How long this challenge is valid (in seconds)
    timeout_secs: i64,
    /// Whether this is a registration or login challenge
    ceremony_type: CeremonyType,
    /// Whether this challenge has been consumed
    consumed: bool,
}

#[derive(Debug, Clone, PartialEq)]
enum CeremonyType {
    Registration,
    Authentication,
}

/// Simulated WebAuthn credential response from the authenticator.
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct SimulatedCredentialResponse {
    credential_id: String,
    /// P256 public key x coordinate (hex)
    public_key_x: String,
    /// P256 public key y coordinate (hex)
    public_key_y: String,
    /// The challenge this response is for
    challenge: String,
    /// The origin the authenticator is responding to
    origin: String,
    /// Simulated authenticator data
    authenticator_data: Vec<u8>,
    /// Display name for the credential
    display_name: String,
}

/// Simulated WebAuthn assertion from the authenticator (for login).
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct SimulatedAssertion {
    credential_id: String,
    /// The challenge this assertion is for
    challenge: String,
    /// The origin the authenticator is responding to
    origin: String,
    /// Simulated signature (r, s components)
    signature_r: [u8; 32],
    signature_s: [u8; 32],
    /// Simulated authenticator data
    authenticator_data: Vec<u8>,
}

/// Session created after successful authentication.
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct AuthSession {
    user_id: String,
    credential_id: String,
    created_at: chrono::DateTime<Utc>,
    expires_at: chrono::DateTime<Utc>,
    session_token: String,
}

/// The WebAuthn ceremony service wrapping PasskeyService.
struct WebAuthnCeremonyService {
    passkey_service: PasskeyService,
    /// Pending challenges: challenge_string -> PendingChallenge
    challenges: Arc<RwLock<HashMap<String, PendingChallenge>>>,
    /// Used assertions for replay protection: hash(credential_id + challenge) -> timestamp
    used_assertions: Arc<RwLock<HashMap<String, chrono::DateTime<Utc>>>>,
    /// Active sessions: session_token -> AuthSession
    sessions: Arc<RwLock<HashMap<String, AuthSession>>>,
    /// The relying party ID for this service
    rp_id: String,
    /// Expected origin for cross-origin checks
    expected_origin: String,
    /// Challenge timeout in seconds
    challenge_timeout_secs: i64,
}

impl WebAuthnCeremonyService {
    fn new(rp_id: &str, expected_origin: &str) -> Self {
        Self {
            passkey_service: PasskeyService::new(),
            challenges: Arc::new(RwLock::new(HashMap::new())),
            used_assertions: Arc::new(RwLock::new(HashMap::new())),
            sessions: Arc::new(RwLock::new(HashMap::new())),
            rp_id: rp_id.to_string(),
            expected_origin: expected_origin.to_string(),
            challenge_timeout_secs: 60, // 60 second default
        }
    }

    fn with_timeout(mut self, timeout_secs: i64) -> Self {
        self.challenge_timeout_secs = timeout_secs;
        self
    }

    /// Step 1 of registration: Generate a challenge.
    async fn begin_registration(
        &self,
        user_email: &str,
    ) -> Result<String, WebAuthnCeremonyError> {
        if user_email.is_empty() {
            return Err(WebAuthnCeremonyError::InvalidInput(
                "Email is required".to_string(),
            ));
        }

        let challenge = generate_challenge();

        let pending = PendingChallenge {
            challenge: challenge.clone(),
            user_email: user_email.to_string(),
            rp_id: self.rp_id.clone(),
            created_at: Utc::now(),
            timeout_secs: self.challenge_timeout_secs,
            ceremony_type: CeremonyType::Registration,
            consumed: false,
        };

        let mut challenges = self.challenges.write().await;
        challenges.insert(challenge.clone(), pending);

        Ok(challenge)
    }

    /// Step 2 of registration: Complete with credential from authenticator.
    async fn complete_registration(
        &self,
        response: &SimulatedCredentialResponse,
    ) -> Result<RegisterPasskeyResponse, WebAuthnCeremonyError> {
        // 1. Validate the challenge exists and is not expired
        let mut challenges = self.challenges.write().await;
        let pending = challenges
            .get_mut(&response.challenge)
            .ok_or(WebAuthnCeremonyError::ChallengeNotFound)?;

        if pending.consumed {
            return Err(WebAuthnCeremonyError::ChallengeAlreadyUsed);
        }

        if pending.ceremony_type != CeremonyType::Registration {
            return Err(WebAuthnCeremonyError::WrongCeremonyType);
        }

        // Check expiry
        let elapsed = (Utc::now() - pending.created_at).num_seconds();
        if elapsed > pending.timeout_secs {
            return Err(WebAuthnCeremonyError::ChallengeExpired);
        }

        // 2. Cross-origin check
        if !self.validate_origin(&response.origin) {
            return Err(WebAuthnCeremonyError::OriginMismatch {
                expected: self.expected_origin.clone(),
                actual: response.origin.clone(),
            });
        }

        // 3. Mark challenge as consumed
        pending.consumed = true;
        let user_email = pending.user_email.clone();
        drop(challenges);

        // 4. Register the credential via PasskeyService
        let register_req = RegisterPasskeyRequest {
            user_id: user_email,
            credential_id: response.credential_id.clone(),
            public_key_x: response.public_key_x.clone(),
            public_key_y: response.public_key_y.clone(),
            display_name: response.display_name.clone(),
        };

        let result = self
            .passkey_service
            .register_passkey(register_req)
            .await
            .map_err(WebAuthnCeremonyError::PasskeyError)?;

        Ok(result)
    }

    /// Step 1 of authentication: Generate a login challenge.
    async fn begin_authentication(
        &self,
        user_email: &str,
    ) -> Result<String, WebAuthnCeremonyError> {
        let challenge = generate_challenge();

        let pending = PendingChallenge {
            challenge: challenge.clone(),
            user_email: user_email.to_string(),
            rp_id: self.rp_id.clone(),
            created_at: Utc::now(),
            timeout_secs: self.challenge_timeout_secs,
            ceremony_type: CeremonyType::Authentication,
            consumed: false,
        };

        let mut challenges = self.challenges.write().await;
        challenges.insert(challenge.clone(), pending);

        Ok(challenge)
    }

    /// Step 2 of authentication: Verify the assertion.
    async fn complete_authentication(
        &self,
        assertion: &SimulatedAssertion,
    ) -> Result<AuthSession, WebAuthnCeremonyError> {
        // 1. Validate the challenge
        let mut challenges = self.challenges.write().await;
        let pending = challenges
            .get_mut(&assertion.challenge)
            .ok_or(WebAuthnCeremonyError::ChallengeNotFound)?;

        if pending.consumed {
            return Err(WebAuthnCeremonyError::ChallengeAlreadyUsed);
        }

        if pending.ceremony_type != CeremonyType::Authentication {
            return Err(WebAuthnCeremonyError::WrongCeremonyType);
        }

        // Check expiry
        let elapsed = (Utc::now() - pending.created_at).num_seconds();
        if elapsed > pending.timeout_secs {
            return Err(WebAuthnCeremonyError::ChallengeExpired);
        }

        // 2. Cross-origin check
        if !self.validate_origin(&assertion.origin) {
            return Err(WebAuthnCeremonyError::OriginMismatch {
                expected: self.expected_origin.clone(),
                actual: assertion.origin.clone(),
            });
        }

        // 3. Replay protection
        let assertion_key = format!("{}:{}", assertion.credential_id, assertion.challenge);
        {
            let used = self.used_assertions.read().await;
            if used.contains_key(&assertion_key) {
                return Err(WebAuthnCeremonyError::ReplayDetected);
            }
        }

        // 4. Mark challenge as consumed
        let user_email = pending.user_email.clone();
        pending.consumed = true;
        drop(challenges);

        // 5. Look up the credential via PasskeyService
        let credential = self
            .passkey_service
            .get_passkey(&user_email, &assertion.credential_id)
            .await
            .map_err(WebAuthnCeremonyError::PasskeyError)?;

        // 6. In a real implementation, we would verify the P256 signature here.
        //    For E2E testing, we verify the credential exists and is active.
        if !credential.is_active {
            return Err(WebAuthnCeremonyError::CredentialRevoked);
        }

        // 7. Record assertion as used (replay protection)
        {
            let mut used = self.used_assertions.write().await;
            used.insert(assertion_key, Utc::now());
        }

        // 8. Update last_used_at
        self.passkey_service
            .mark_used(&user_email, &assertion.credential_id)
            .await
            .map_err(WebAuthnCeremonyError::PasskeyError)?;

        // 9. Create session
        let session = AuthSession {
            user_id: user_email,
            credential_id: assertion.credential_id.clone(),
            created_at: Utc::now(),
            expires_at: Utc::now() + Duration::hours(24),
            session_token: Uuid::new_v4().to_string(),
        };

        let token = session.session_token.clone();
        let mut sessions = self.sessions.write().await;
        sessions.insert(token, session.clone());

        Ok(session)
    }

    fn validate_origin(&self, origin: &str) -> bool {
        origin == self.expected_origin
    }

    /// Get underlying passkey service for direct operations.
    fn passkey_service(&self) -> &PasskeyService {
        &self.passkey_service
    }

    /// Get active session count.
    async fn active_session_count(&self) -> usize {
        let sessions = self.sessions.read().await;
        sessions
            .values()
            .filter(|s| s.expires_at > Utc::now())
            .count()
    }
}

/// Errors from the WebAuthn ceremony layer.
#[derive(Debug)]
enum WebAuthnCeremonyError {
    ChallengeNotFound,
    ChallengeExpired,
    ChallengeAlreadyUsed,
    WrongCeremonyType,
    OriginMismatch { expected: String, actual: String },
    ReplayDetected,
    CredentialRevoked,
    InvalidInput(String),
    PasskeyError(PasskeyError),
}

impl std::fmt::Display for WebAuthnCeremonyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ChallengeNotFound => write!(f, "Challenge not found"),
            Self::ChallengeExpired => write!(f, "Challenge expired"),
            Self::ChallengeAlreadyUsed => write!(f, "Challenge already used"),
            Self::WrongCeremonyType => write!(f, "Wrong ceremony type"),
            Self::OriginMismatch { expected, actual } => {
                write!(f, "Origin mismatch: expected {}, got {}", expected, actual)
            }
            Self::ReplayDetected => write!(f, "Replay attack detected"),
            Self::CredentialRevoked => write!(f, "Credential has been revoked"),
            Self::InvalidInput(msg) => write!(f, "Invalid input: {}", msg),
            Self::PasskeyError(e) => write!(f, "Passkey error: {}", e),
        }
    }
}

// ============================================================================
// Test Helpers
// ============================================================================

/// Generate a random challenge string (simulates server-side challenge).
fn generate_challenge() -> String {
    Uuid::new_v4().to_string()
}

/// Well-known P256 test coordinates (the generator point of the P-256 curve).
const TEST_PUB_KEY_X: &str =
    "6B17D1F2E12C4247F8BCE6E563A440F277037D812DEB33A0F4A13945D898C296";
const TEST_PUB_KEY_Y: &str =
    "4FE342E2FE1A7F9B8EE7EB4A7C0F9E162BCE33576B315ECECBB6406837BF51F5";

const TEST_RP_ID: &str = "localhost";
const TEST_ORIGIN: &str = "https://localhost";
const TEST_EMAIL: &str = "user@example.com";

/// Create a simulated credential response for a given challenge.
fn make_credential_response(
    challenge: &str,
    credential_id: &str,
    display_name: &str,
) -> SimulatedCredentialResponse {
    SimulatedCredentialResponse {
        credential_id: credential_id.to_string(),
        public_key_x: TEST_PUB_KEY_X.to_string(),
        public_key_y: TEST_PUB_KEY_Y.to_string(),
        challenge: challenge.to_string(),
        origin: TEST_ORIGIN.to_string(),
        authenticator_data: vec![0u8; 37],
        display_name: display_name.to_string(),
    }
}

/// Create a simulated assertion for a given challenge and credential.
fn make_assertion(challenge: &str, credential_id: &str) -> SimulatedAssertion {
    SimulatedAssertion {
        credential_id: credential_id.to_string(),
        challenge: challenge.to_string(),
        origin: TEST_ORIGIN.to_string(),
        signature_r: [0x01u8; 32],
        signature_s: [0x02u8; 32],
        authenticator_data: vec![0u8; 37],
    }
}

/// Build the ceremony service with default test configuration.
fn test_ceremony_service() -> WebAuthnCeremonyService {
    WebAuthnCeremonyService::new(TEST_RP_ID, TEST_ORIGIN)
}

// ============================================================================
// E2E Test 1: Registration Ceremony - Full Lifecycle
// ============================================================================

#[tokio::test]
async fn test_registration_ceremony_full_lifecycle() {
    let service = test_ceremony_service();

    // Step 1: Begin registration (generate challenge)
    let challenge = service
        .begin_registration(TEST_EMAIL)
        .await
        .expect("begin_registration should succeed");

    assert!(!challenge.is_empty(), "Challenge must not be empty");

    // Step 2: Simulate authenticator creating a credential
    let cred_response = make_credential_response(&challenge, "cred-reg-001", "iPhone Face ID");

    // Step 3: Complete registration
    let result = service
        .complete_registration(&cred_response)
        .await
        .expect("complete_registration should succeed");

    assert_eq!(result.credential_id, "cred-reg-001");
    assert!(result.smart_account_address.is_none());

    // Step 4: Verify credential was stored
    let stored = service
        .passkey_service()
        .get_passkey(TEST_EMAIL, "cred-reg-001")
        .await
        .expect("credential should be retrievable");

    assert_eq!(stored.user_id, TEST_EMAIL);
    assert_eq!(stored.credential_id, "cred-reg-001");
    assert_eq!(stored.display_name, "iPhone Face ID");
    assert!(stored.is_active);
    assert!(stored.last_used_at.is_none());
}

// ============================================================================
// E2E Test 2: Authentication Ceremony - Full Lifecycle
// ============================================================================

#[tokio::test]
async fn test_authentication_ceremony_full_lifecycle() {
    let service = test_ceremony_service();

    // Pre-register a credential
    let reg_challenge = service
        .begin_registration(TEST_EMAIL)
        .await
        .unwrap();
    let cred_response = make_credential_response(&reg_challenge, "cred-auth-001", "MacBook Touch ID");
    service
        .complete_registration(&cred_response)
        .await
        .unwrap();

    // Step 1: Begin authentication
    let auth_challenge = service
        .begin_authentication(TEST_EMAIL)
        .await
        .expect("begin_authentication should succeed");

    assert!(!auth_challenge.is_empty());

    // Step 2: Simulate authenticator signing the challenge
    let assertion = make_assertion(&auth_challenge, "cred-auth-001");

    // Step 3: Complete authentication
    let session = service
        .complete_authentication(&assertion)
        .await
        .expect("complete_authentication should succeed");

    assert_eq!(session.user_id, TEST_EMAIL);
    assert_eq!(session.credential_id, "cred-auth-001");
    assert!(!session.session_token.is_empty());
    assert!(session.expires_at > Utc::now());

    // Step 4: Verify credential was marked as used
    let cred = service
        .passkey_service()
        .get_passkey(TEST_EMAIL, "cred-auth-001")
        .await
        .unwrap();
    assert!(cred.last_used_at.is_some());

    // Step 5: Verify session was created
    assert_eq!(service.active_session_count().await, 1);
}

// ============================================================================
// E2E Test 3: Multiple Credentials Per User
// ============================================================================

#[tokio::test]
async fn test_multiple_credentials_per_user() {
    let service = test_ceremony_service();
    let user_email = "multi-cred@example.com";

    // Register 5 different credentials for the same user
    let credential_names = vec![
        ("cred-multi-001", "iPhone Face ID"),
        ("cred-multi-002", "MacBook Touch ID"),
        ("cred-multi-003", "YubiKey 5"),
        ("cred-multi-004", "Windows Hello"),
        ("cred-multi-005", "Android Fingerprint"),
    ];

    for (cred_id, display_name) in &credential_names {
        let challenge = service
            .begin_registration(user_email)
            .await
            .unwrap();
        let response = make_credential_response(&challenge, cred_id, display_name);
        service
            .complete_registration(&response)
            .await
            .expect(&format!("registration of {} should succeed", cred_id));
    }

    // Verify all 5 credentials exist
    let all_creds = service
        .passkey_service()
        .list_passkeys(user_email)
        .await
        .unwrap();
    assert_eq!(all_creds.len(), 5, "User should have 5 credentials");

    // Verify credential count
    assert_eq!(
        service.passkey_service().credential_count(user_email).await,
        5
    );

    // Authenticate with each credential
    for (cred_id, _) in &credential_names {
        let challenge = service
            .begin_authentication(user_email)
            .await
            .unwrap();
        let assertion = make_assertion(&challenge, cred_id);
        let session = service
            .complete_authentication(&assertion)
            .await
            .expect(&format!("authentication with {} should succeed", cred_id));
        assert_eq!(session.credential_id, *cred_id);
    }

    // All 5 authentications should have created sessions
    assert_eq!(service.active_session_count().await, 5);

    // All credentials should have been marked as used
    for (cred_id, _) in &credential_names {
        let cred = service
            .passkey_service()
            .get_passkey(user_email, cred_id)
            .await
            .unwrap();
        assert!(
            cred.last_used_at.is_some(),
            "Credential {} should have last_used_at set",
            cred_id
        );
    }
}

// ============================================================================
// E2E Test 4: Credential Revocation (Deactivation)
// ============================================================================

#[tokio::test]
async fn test_credential_revocation() {
    let service = test_ceremony_service();
    let user_email = "revoke@example.com";

    // Register two credentials
    let challenge1 = service.begin_registration(user_email).await.unwrap();
    let resp1 = make_credential_response(&challenge1, "cred-revoke-001", "Primary Device");
    service.complete_registration(&resp1).await.unwrap();

    let challenge2 = service.begin_registration(user_email).await.unwrap();
    let resp2 = make_credential_response(&challenge2, "cred-revoke-002", "Backup Key");
    service.complete_registration(&resp2).await.unwrap();

    assert_eq!(
        service.passkey_service().credential_count(user_email).await,
        2
    );

    // Revoke the first credential
    service
        .passkey_service()
        .deactivate_passkey(user_email, "cred-revoke-001")
        .await
        .expect("deactivation should succeed");

    // Verify count decreased
    assert_eq!(
        service.passkey_service().credential_count(user_email).await,
        1,
        "Only 1 active credential should remain"
    );

    // Attempt to authenticate with the revoked credential
    let auth_challenge = service
        .begin_authentication(user_email)
        .await
        .unwrap();
    let assertion = make_assertion(&auth_challenge, "cred-revoke-001");
    let result = service.complete_authentication(&assertion).await;
    assert!(
        result.is_err(),
        "Authentication with revoked credential should fail"
    );

    // Verify we get a specific error (either CredentialRevoked or PasskeyError::CredentialNotFound)
    match result {
        Err(WebAuthnCeremonyError::PasskeyError(PasskeyError::CredentialNotFound(_))) => {
            // Expected: PasskeyService filters out inactive credentials in get_passkey
        }
        Err(WebAuthnCeremonyError::CredentialRevoked) => {
            // Also acceptable
        }
        Err(other) => panic!(
            "Expected CredentialNotFound or CredentialRevoked, got: {}",
            other
        ),
        Ok(_) => panic!("Should have failed"),
    }

    // The second credential should still work
    let auth_challenge2 = service
        .begin_authentication(user_email)
        .await
        .unwrap();
    let assertion2 = make_assertion(&auth_challenge2, "cred-revoke-002");
    let session = service
        .complete_authentication(&assertion2)
        .await
        .expect("authentication with active credential should succeed");
    assert_eq!(session.credential_id, "cred-revoke-002");

    // List should only show active credentials
    let active = service
        .passkey_service()
        .list_passkeys(user_email)
        .await
        .unwrap();
    assert_eq!(active.len(), 1);
    assert_eq!(active[0].credential_id, "cred-revoke-002");
}

// ============================================================================
// E2E Test 5: Challenge Expiry
// ============================================================================

#[tokio::test]
async fn test_challenge_expiry() {
    // Create service with a very short timeout (1 second)
    let service = WebAuthnCeremonyService::new(TEST_RP_ID, TEST_ORIGIN).with_timeout(1);

    let user_email = "expiry@example.com";

    // Begin registration
    let challenge = service
        .begin_registration(user_email)
        .await
        .unwrap();

    // Wait for the challenge to expire
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // Attempt to complete with expired challenge
    let response = make_credential_response(&challenge, "cred-expired-001", "Expired Device");
    let result = service.complete_registration(&response).await;

    assert!(
        result.is_err(),
        "Registration with expired challenge should fail"
    );
    match result {
        Err(WebAuthnCeremonyError::ChallengeExpired) => {
            // Expected
        }
        Err(other) => panic!("Expected ChallengeExpired, got: {}", other),
        Ok(_) => panic!("Should have failed"),
    }

    // Same test for authentication: register a credential first with fresh challenge
    let service2 = WebAuthnCeremonyService::new(TEST_RP_ID, TEST_ORIGIN).with_timeout(1);

    // Register with fresh service that has long enough timeout
    let reg_service = test_ceremony_service();
    let reg_challenge = reg_service
        .begin_registration(user_email)
        .await
        .unwrap();
    let reg_resp = make_credential_response(&reg_challenge, "cred-for-auth-expiry", "Test Device");
    reg_service.complete_registration(&reg_resp).await.unwrap();

    // Now use the short-timeout service for auth test.
    // We need to register the credential in service2 as well, since each service
    // has its own PasskeyService instance.
    let reg_challenge2 = service2.begin_registration(user_email).await.unwrap();
    let reg_resp2 = make_credential_response(&reg_challenge2, "cred-for-auth-expiry-2", "Test Device 2");
    service2.complete_registration(&reg_resp2).await.unwrap();

    let auth_challenge = service2
        .begin_authentication(user_email)
        .await
        .unwrap();

    // Wait for expiry
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    let assertion = make_assertion(&auth_challenge, "cred-for-auth-expiry-2");
    let auth_result = service2.complete_authentication(&assertion).await;

    assert!(
        auth_result.is_err(),
        "Authentication with expired challenge should fail"
    );
    match auth_result {
        Err(WebAuthnCeremonyError::ChallengeExpired) => {
            // Expected
        }
        Err(other) => panic!("Expected ChallengeExpired, got: {}", other),
        Ok(_) => panic!("Should have failed"),
    }
}

// ============================================================================
// E2E Test 5b: Fresh Challenge Succeeds Within Timeout
// ============================================================================

#[tokio::test]
async fn test_challenge_within_timeout_succeeds() {
    // Use a generous timeout to prove non-expired challenges work
    let service = WebAuthnCeremonyService::new(TEST_RP_ID, TEST_ORIGIN).with_timeout(300);
    let user_email = "fresh@example.com";

    let challenge = service
        .begin_registration(user_email)
        .await
        .unwrap();

    // Immediately complete -- well within 300s timeout
    let response = make_credential_response(&challenge, "cred-fresh-001", "Fresh Device");
    let result = service.complete_registration(&response).await;

    assert!(
        result.is_ok(),
        "Registration with fresh challenge should succeed"
    );
}

// ============================================================================
// E2E Test 6: Cross-Origin Protection
// ============================================================================

#[tokio::test]
async fn test_cross_origin_protection() {
    let service = test_ceremony_service();
    let user_email = "origin@example.com";

    // Registration: wrong origin
    let challenge = service
        .begin_registration(user_email)
        .await
        .unwrap();

    let mut response = make_credential_response(&challenge, "cred-origin-001", "Origin Test");
    response.origin = "https://evil.com".to_string(); // Wrong origin

    let result = service.complete_registration(&response).await;
    assert!(result.is_err(), "Registration from wrong origin should fail");
    match result {
        Err(WebAuthnCeremonyError::OriginMismatch { expected, actual }) => {
            assert_eq!(expected, TEST_ORIGIN);
            assert_eq!(actual, "https://evil.com");
        }
        Err(other) => panic!("Expected OriginMismatch, got: {}", other),
        Ok(_) => panic!("Should have failed"),
    }

    // Authentication: wrong origin (register first with correct origin)
    let reg_challenge = service
        .begin_registration(user_email)
        .await
        .unwrap();
    let reg_response = make_credential_response(&reg_challenge, "cred-origin-002", "Good Origin");
    service
        .complete_registration(&reg_response)
        .await
        .unwrap();

    let auth_challenge = service
        .begin_authentication(user_email)
        .await
        .unwrap();

    let mut assertion = make_assertion(&auth_challenge, "cred-origin-002");
    assertion.origin = "https://phishing.example.com".to_string();

    let auth_result = service.complete_authentication(&assertion).await;
    assert!(
        auth_result.is_err(),
        "Authentication from wrong origin should fail"
    );
    match auth_result {
        Err(WebAuthnCeremonyError::OriginMismatch { expected, actual }) => {
            assert_eq!(expected, TEST_ORIGIN);
            assert_eq!(actual, "https://phishing.example.com");
        }
        Err(other) => panic!("Expected OriginMismatch, got: {}", other),
        Ok(_) => panic!("Should have failed"),
    }

    // Correct origin works
    let auth_challenge2 = service
        .begin_authentication(user_email)
        .await
        .unwrap();
    let assertion2 = make_assertion(&auth_challenge2, "cred-origin-002");
    let session = service
        .complete_authentication(&assertion2)
        .await
        .expect("authentication from correct origin should succeed");
    assert_eq!(session.user_id, user_email);
}

// ============================================================================
// E2E Test 7: Replay Attack Prevention
// ============================================================================

#[tokio::test]
async fn test_replay_attack_prevention() {
    let service = test_ceremony_service();
    let user_email = "replay@example.com";

    // Register a credential
    let reg_challenge = service
        .begin_registration(user_email)
        .await
        .unwrap();
    let reg_response = make_credential_response(&reg_challenge, "cred-replay-001", "Replay Test");
    service
        .complete_registration(&reg_response)
        .await
        .unwrap();

    // Authenticate successfully once
    let auth_challenge = service
        .begin_authentication(user_email)
        .await
        .unwrap();
    let assertion = make_assertion(&auth_challenge, "cred-replay-001");

    let session = service
        .complete_authentication(&assertion)
        .await
        .expect("first authentication should succeed");
    assert_eq!(session.credential_id, "cred-replay-001");

    // Attempt to replay the same assertion (same challenge + credential_id)
    // This should fail because the challenge is already consumed
    let replay_result = service.complete_authentication(&assertion).await;
    assert!(
        replay_result.is_err(),
        "Replayed assertion should be rejected"
    );
    match replay_result {
        Err(WebAuthnCeremonyError::ChallengeAlreadyUsed) => {
            // Expected: the challenge was already consumed
        }
        Err(WebAuthnCeremonyError::ReplayDetected) => {
            // Also acceptable
        }
        Err(other) => panic!(
            "Expected ChallengeAlreadyUsed or ReplayDetected, got: {}",
            other
        ),
        Ok(_) => panic!("Should have failed"),
    }

    // Registration challenge replay should also fail
    let reg_challenge2 = service
        .begin_registration(user_email)
        .await
        .unwrap();
    let reg_response2 =
        make_credential_response(&reg_challenge2, "cred-replay-002", "Replay Test 2");
    service
        .complete_registration(&reg_response2)
        .await
        .unwrap();

    // Try to use the same registration challenge again with a different credential
    let replay_reg = make_credential_response(&reg_challenge2, "cred-replay-003", "Replay Reg");
    let replay_reg_result = service.complete_registration(&replay_reg).await;
    assert!(
        replay_reg_result.is_err(),
        "Replayed registration challenge should be rejected"
    );
    match replay_reg_result {
        Err(WebAuthnCeremonyError::ChallengeAlreadyUsed) => {
            // Expected
        }
        Err(other) => panic!("Expected ChallengeAlreadyUsed, got: {}", other),
        Ok(_) => panic!("Should have failed"),
    }
}

// ============================================================================
// E2E Test 8: Concurrent Registration/Authentication Attempts
// ============================================================================

#[tokio::test]
async fn test_concurrent_registration() {
    let service = Arc::new(test_ceremony_service());

    // Spawn 20 concurrent registrations for different users
    let handles: Vec<_> = (0..20)
        .map(|i| {
            let svc = service.clone();
            tokio::spawn(async move {
                let email = format!("concurrent-{}@example.com", i);
                let cred_id = format!("cred-concurrent-{}", i);
                let display = format!("Device {}", i);

                let challenge = svc.begin_registration(&email).await?;
                let response = make_credential_response(&challenge, &cred_id, &display);
                svc.complete_registration(&response).await?;

                // Verify it was stored
                let cred = svc
                    .passkey_service()
                    .get_passkey(&email, &cred_id)
                    .await
                    .map_err(WebAuthnCeremonyError::PasskeyError)?;
                assert_eq!(cred.credential_id, cred_id);
                assert!(cred.is_active);

                Ok::<String, WebAuthnCeremonyError>(cred_id)
            })
        })
        .collect();

    let results = futures::future::join_all(handles).await;

    let mut successful = 0;
    for (i, result) in results.iter().enumerate() {
        match result {
            Ok(Ok(cred_id)) => {
                successful += 1;
                assert_eq!(*cred_id, format!("cred-concurrent-{}", i));
            }
            Ok(Err(e)) => panic!("Registration {} failed: {}", i, e),
            Err(e) => panic!("Task {} panicked: {}", i, e),
        }
    }

    assert_eq!(successful, 20, "All 20 concurrent registrations should succeed");
}

#[tokio::test]
async fn test_concurrent_authentication() {
    let service = Arc::new(test_ceremony_service());

    // Pre-register 10 users, each with one credential
    for i in 0..10 {
        let email = format!("concurrent-auth-{}@example.com", i);
        let cred_id = format!("cred-cauth-{}", i);
        let challenge = service.begin_registration(&email).await.unwrap();
        let response = make_credential_response(&challenge, &cred_id, &format!("Device {}", i));
        service.complete_registration(&response).await.unwrap();
    }

    // Spawn 10 concurrent authentications
    let handles: Vec<_> = (0..10)
        .map(|i| {
            let svc = service.clone();
            tokio::spawn(async move {
                let email = format!("concurrent-auth-{}@example.com", i);
                let cred_id = format!("cred-cauth-{}", i);

                let challenge = svc.begin_authentication(&email).await?;
                let assertion = make_assertion(&challenge, &cred_id);
                let session = svc.complete_authentication(&assertion).await?;

                assert_eq!(session.user_id, email);
                assert_eq!(session.credential_id, cred_id);
                assert!(!session.session_token.is_empty());

                Ok::<String, WebAuthnCeremonyError>(session.session_token)
            })
        })
        .collect();

    let results = futures::future::join_all(handles).await;

    let mut session_tokens = Vec::new();
    for (i, result) in results.iter().enumerate() {
        match result {
            Ok(Ok(token)) => {
                session_tokens.push(token.clone());
            }
            Ok(Err(e)) => panic!("Authentication {} failed: {}", i, e),
            Err(e) => panic!("Task {} panicked: {}", i, e),
        }
    }

    assert_eq!(session_tokens.len(), 10);

    // All session tokens should be unique
    let unique: std::collections::HashSet<&String> = session_tokens.iter().collect();
    assert_eq!(
        unique.len(),
        10,
        "All session tokens must be unique"
    );

    // All sessions should be active
    assert_eq!(service.active_session_count().await, 10);
}

// ============================================================================
// E2E Test 9: Duplicate Credential Registration
// ============================================================================

#[tokio::test]
async fn test_duplicate_credential_registration() {
    let service = test_ceremony_service();
    let user_email = "dup@example.com";

    // Register first credential
    let challenge1 = service.begin_registration(user_email).await.unwrap();
    let resp1 = make_credential_response(&challenge1, "cred-dup-001", "First");
    service.complete_registration(&resp1).await.unwrap();

    // Try to register the same credential ID again
    let challenge2 = service.begin_registration(user_email).await.unwrap();
    let resp2 = make_credential_response(&challenge2, "cred-dup-001", "Duplicate");
    let result = service.complete_registration(&resp2).await;

    assert!(
        result.is_err(),
        "Duplicate credential registration should fail"
    );
    match result {
        Err(WebAuthnCeremonyError::PasskeyError(PasskeyError::CredentialAlreadyExists)) => {
            // Expected
        }
        Err(other) => panic!("Expected CredentialAlreadyExists, got: {}", other),
        Ok(_) => panic!("Should have failed"),
    }

    // Only one credential should exist
    assert_eq!(
        service.passkey_service().credential_count(user_email).await,
        1
    );
}

// ============================================================================
// E2E Test 10: Wrong Ceremony Type (Registration vs Authentication)
// ============================================================================

#[tokio::test]
async fn test_wrong_ceremony_type() {
    let service = test_ceremony_service();
    let user_email = "ceremony@example.com";

    // Register a credential first
    let reg_challenge = service.begin_registration(user_email).await.unwrap();
    let reg_resp = make_credential_response(&reg_challenge, "cred-ceremony-001", "Test");
    service.complete_registration(&reg_resp).await.unwrap();

    // Get an authentication challenge
    let auth_challenge = service.begin_authentication(user_email).await.unwrap();

    // Try to use it for registration
    let wrong_resp = make_credential_response(&auth_challenge, "cred-ceremony-002", "Wrong");
    let result = service.complete_registration(&wrong_resp).await;

    assert!(
        result.is_err(),
        "Using auth challenge for registration should fail"
    );
    match result {
        Err(WebAuthnCeremonyError::WrongCeremonyType) => {
            // Expected
        }
        Err(other) => panic!("Expected WrongCeremonyType, got: {}", other),
        Ok(_) => panic!("Should have failed"),
    }

    // Get a registration challenge
    let reg_challenge2 = service.begin_registration(user_email).await.unwrap();

    // Try to use it for authentication
    let wrong_assertion = make_assertion(&reg_challenge2, "cred-ceremony-001");
    let result2 = service.complete_authentication(&wrong_assertion).await;

    assert!(
        result2.is_err(),
        "Using registration challenge for authentication should fail"
    );
    match result2 {
        Err(WebAuthnCeremonyError::WrongCeremonyType) => {
            // Expected
        }
        Err(other) => panic!("Expected WrongCeremonyType, got: {}", other),
        Ok(_) => panic!("Should have failed"),
    }
}

// ============================================================================
// E2E Test 11: Nonexistent Challenge
// ============================================================================

#[tokio::test]
async fn test_nonexistent_challenge() {
    let service = test_ceremony_service();

    // Try to complete registration with a fabricated challenge
    let fake_response = make_credential_response("fake-challenge-xyz", "cred-fake-001", "Fake");
    let result = service.complete_registration(&fake_response).await;

    assert!(
        result.is_err(),
        "Fabricated challenge should be rejected"
    );
    match result {
        Err(WebAuthnCeremonyError::ChallengeNotFound) => {
            // Expected
        }
        Err(other) => panic!("Expected ChallengeNotFound, got: {}", other),
        Ok(_) => panic!("Should have failed"),
    }

    // Try to complete authentication with a fabricated challenge
    let fake_assertion = make_assertion("fake-challenge-xyz", "cred-fake-001");
    let result2 = service.complete_authentication(&fake_assertion).await;

    assert!(
        result2.is_err(),
        "Fabricated auth challenge should be rejected"
    );
    match result2 {
        Err(WebAuthnCeremonyError::ChallengeNotFound) => {
            // Expected
        }
        Err(other) => panic!("Expected ChallengeNotFound, got: {}", other),
        Ok(_) => panic!("Should have failed"),
    }
}

// ============================================================================
// E2E Test 12: Smart Account Linking
// ============================================================================

#[tokio::test]
async fn test_smart_account_linking_after_registration() {
    let service = test_ceremony_service();
    let user_email = "link@example.com";

    // Register credential
    let challenge = service.begin_registration(user_email).await.unwrap();
    let resp = make_credential_response(&challenge, "cred-link-001", "Linkable Key");
    let reg_result = service.complete_registration(&resp).await.unwrap();

    assert!(
        reg_result.smart_account_address.is_none(),
        "Initially no smart account linked"
    );

    // Link to a smart account
    let link_request = LinkAccountRequest {
        user_id: user_email.to_string(),
        credential_id: "cred-link-001".to_string(),
        smart_account_address: "0xdead000000000000000000000000000000000001".to_string(),
    };

    service
        .passkey_service()
        .link_smart_account(link_request)
        .await
        .expect("linking should succeed");

    // Verify the link
    let cred = service
        .passkey_service()
        .get_passkey(user_email, "cred-link-001")
        .await
        .unwrap();

    assert_eq!(
        cred.smart_account_address,
        Some("0xdead000000000000000000000000000000000001".to_string())
    );

    // Authentication should still work after linking
    let auth_challenge = service
        .begin_authentication(user_email)
        .await
        .unwrap();
    let assertion = make_assertion(&auth_challenge, "cred-link-001");
    let session = service
        .complete_authentication(&assertion)
        .await
        .expect("authentication after linking should succeed");
    assert_eq!(session.user_id, user_email);
}

// ============================================================================
// E2E Test 13: Invalid Public Key Rejection During Registration
// ============================================================================

#[tokio::test]
async fn test_invalid_public_key_during_registration() {
    let service = test_ceremony_service();
    let user_email = "badkey@example.com";

    // Invalid hex in public key x
    let challenge = service.begin_registration(user_email).await.unwrap();
    let mut response = make_credential_response(&challenge, "cred-badkey-001", "Bad Key");
    response.public_key_x = "ZZZZ_not_hex".to_string();

    let result = service.complete_registration(&response).await;
    assert!(
        result.is_err(),
        "Registration with invalid public key should fail"
    );
    match result {
        Err(WebAuthnCeremonyError::PasskeyError(PasskeyError::InvalidPublicKey(_))) => {
            // Expected
        }
        Err(other) => panic!("Expected InvalidPublicKey, got: {}", other),
        Ok(_) => panic!("Should have failed"),
    }

    // Empty public key
    let challenge2 = service.begin_registration(user_email).await.unwrap();
    let mut response2 = make_credential_response(&challenge2, "cred-badkey-002", "Empty Key");
    response2.public_key_y = "".to_string();

    let result2 = service.complete_registration(&response2).await;
    assert!(
        result2.is_err(),
        "Registration with empty public key should fail"
    );
}

// ============================================================================
// E2E Test 14: Authentication With Nonexistent User/Credential
// ============================================================================

#[tokio::test]
async fn test_authentication_nonexistent_credential() {
    let service = test_ceremony_service();
    let user_email = "ghost@example.com";

    // Begin authentication for a user that has no registered credentials
    let challenge = service
        .begin_authentication(user_email)
        .await
        .unwrap();

    let assertion = make_assertion(&challenge, "nonexistent-cred-001");
    let result = service.complete_authentication(&assertion).await;

    assert!(
        result.is_err(),
        "Authentication with nonexistent credential should fail"
    );
    match result {
        Err(WebAuthnCeremonyError::PasskeyError(PasskeyError::UserNotFound(_))) => {
            // Expected: user has no credentials
        }
        Err(WebAuthnCeremonyError::PasskeyError(PasskeyError::CredentialNotFound(_))) => {
            // Also acceptable
        }
        Err(other) => panic!("Expected UserNotFound or CredentialNotFound, got: {}", other),
        Ok(_) => panic!("Should have failed"),
    }
}

// ============================================================================
// E2E Test 15: Full Registration-Authentication-Revocation Lifecycle
// ============================================================================

#[tokio::test]
async fn test_full_lifecycle_registration_auth_revoke() {
    let service = test_ceremony_service();
    let user_email = "lifecycle@example.com";

    // Phase 1: Register
    let reg_challenge = service.begin_registration(user_email).await.unwrap();
    let reg_resp = make_credential_response(&reg_challenge, "cred-lifecycle-001", "Lifecycle Key");
    service.complete_registration(&reg_resp).await.unwrap();

    // Phase 2: Authenticate successfully
    let auth_challenge = service.begin_authentication(user_email).await.unwrap();
    let assertion = make_assertion(&auth_challenge, "cred-lifecycle-001");
    let session = service.complete_authentication(&assertion).await.unwrap();
    assert_eq!(session.credential_id, "cred-lifecycle-001");

    // Phase 3: Link smart account
    let link = LinkAccountRequest {
        user_id: user_email.to_string(),
        credential_id: "cred-lifecycle-001".to_string(),
        smart_account_address: "0x1111111111111111111111111111111111111111".to_string(),
    };
    service
        .passkey_service()
        .link_smart_account(link)
        .await
        .unwrap();

    // Phase 4: Register a second credential
    let reg_challenge2 = service.begin_registration(user_email).await.unwrap();
    let reg_resp2 =
        make_credential_response(&reg_challenge2, "cred-lifecycle-002", "Backup Key");
    service.complete_registration(&reg_resp2).await.unwrap();

    assert_eq!(
        service.passkey_service().credential_count(user_email).await,
        2
    );

    // Phase 5: Revoke the first credential
    service
        .passkey_service()
        .deactivate_passkey(user_email, "cred-lifecycle-001")
        .await
        .unwrap();

    assert_eq!(
        service.passkey_service().credential_count(user_email).await,
        1
    );

    // Phase 6: First credential fails, second still works
    let auth_challenge2 = service.begin_authentication(user_email).await.unwrap();
    let assertion2 = make_assertion(&auth_challenge2, "cred-lifecycle-001");
    assert!(
        service
            .complete_authentication(&assertion2)
            .await
            .is_err(),
        "Revoked credential should fail"
    );

    let auth_challenge3 = service.begin_authentication(user_email).await.unwrap();
    let assertion3 = make_assertion(&auth_challenge3, "cred-lifecycle-002");
    let session2 = service.complete_authentication(&assertion3).await.unwrap();
    assert_eq!(session2.credential_id, "cred-lifecycle-002");
}

// ============================================================================
// E2E Test 16: Empty Input Validation
// ============================================================================

#[tokio::test]
async fn test_empty_input_validation() {
    let service = test_ceremony_service();

    // Empty email for registration
    let result = service.begin_registration("").await;
    assert!(result.is_err(), "Empty email should be rejected");

    // Empty credential ID
    let challenge = service
        .begin_registration("valid@example.com")
        .await
        .unwrap();
    let response = make_credential_response(&challenge, "", "Empty Cred ID");
    let result2 = service.complete_registration(&response).await;
    assert!(
        result2.is_err(),
        "Empty credential ID should be rejected"
    );

    // Empty user ID via direct passkey service
    let req = RegisterPasskeyRequest {
        user_id: "".to_string(),
        credential_id: "cred-empty-user".to_string(),
        public_key_x: TEST_PUB_KEY_X.to_string(),
        public_key_y: TEST_PUB_KEY_Y.to_string(),
        display_name: "Test".to_string(),
    };
    let result3 = service
        .passkey_service()
        .register_passkey(req)
        .await;
    assert!(result3.is_err());
    assert!(matches!(
        result3.unwrap_err(),
        PasskeyError::InvalidUserId
    ));
}

// ============================================================================
// E2E Test 17: Multiple Origins (Cross-Origin Variants)
// ============================================================================

#[tokio::test]
async fn test_multiple_cross_origin_variants() {
    let service = test_ceremony_service(); // expects "https://localhost"
    let user_email = "origins@example.com";

    let invalid_origins = vec![
        "http://localhost",           // http vs https
        "https://localhost:3000",     // different port
        "https://localhost.evil.com", // subdomain attack
        "https://LOCALHOST",          // case sensitivity
        "",                           // empty origin
        "https://localhost/path",     // with path
    ];

    for origin in invalid_origins {
        let challenge = service.begin_registration(user_email).await.unwrap();
        let mut response = make_credential_response(
            &challenge,
            &format!("cred-origin-{}", origin.len()),
            "Origin Test",
        );
        response.origin = origin.to_string();

        let result = service.complete_registration(&response).await;
        assert!(
            result.is_err(),
            "Origin '{}' should be rejected",
            origin
        );
    }
}

// ============================================================================
// E2E Test 18: Concurrent Registration and Authentication Mixed
// ============================================================================

#[tokio::test]
async fn test_concurrent_mixed_operations() {
    let service = Arc::new(test_ceremony_service());

    // Pre-register 5 users
    for i in 0..5 {
        let email = format!("mixed-{}@example.com", i);
        let cred_id = format!("cred-mixed-{}", i);
        let challenge = service.begin_registration(&email).await.unwrap();
        let response = make_credential_response(&challenge, &cred_id, &format!("Device {}", i));
        service.complete_registration(&response).await.unwrap();
    }

    // Spawn mixed operations:
    // - 5 new registrations (users 5-9)
    // - 5 authentications (users 0-4)
    let mut handles = Vec::new();

    // New registrations
    for i in 5..10 {
        let svc = service.clone();
        handles.push(tokio::spawn(async move {
            let email = format!("mixed-{}@example.com", i);
            let cred_id = format!("cred-mixed-{}", i);
            let challenge = svc.begin_registration(&email).await.unwrap();
            let response =
                make_credential_response(&challenge, &cred_id, &format!("Device {}", i));
            svc.complete_registration(&response).await.unwrap();
            format!("registered:{}", cred_id)
        }));
    }

    // Authentications for existing users
    for i in 0..5 {
        let svc = service.clone();
        handles.push(tokio::spawn(async move {
            let email = format!("mixed-{}@example.com", i);
            let cred_id = format!("cred-mixed-{}", i);
            let challenge = svc.begin_authentication(&email).await.unwrap();
            let assertion = make_assertion(&challenge, &cred_id);
            let session = svc.complete_authentication(&assertion).await.unwrap();
            format!("authenticated:{}", session.session_token)
        }));
    }

    let results = futures::future::join_all(handles).await;

    let mut registrations = 0;
    let mut authentications = 0;

    for result in results {
        let output = result.expect("task should not panic");
        if output.starts_with("registered:") {
            registrations += 1;
        } else if output.starts_with("authenticated:") {
            authentications += 1;
        }
    }

    assert_eq!(registrations, 5);
    assert_eq!(authentications, 5);

    // All 10 users should now have credentials
    for i in 0..10 {
        let email = format!("mixed-{}@example.com", i);
        let cred_id = format!("cred-mixed-{}", i);
        let cred = service
            .passkey_service()
            .get_passkey(&email, &cred_id)
            .await
            .unwrap();
        assert!(cred.is_active);
    }
}

// ============================================================================
// E2E Test 19: Public Key With 0x Prefix
// ============================================================================

#[tokio::test]
async fn test_public_key_with_0x_prefix() {
    let service = test_ceremony_service();
    let user_email = "prefix@example.com";

    let challenge = service.begin_registration(user_email).await.unwrap();
    let mut response = make_credential_response(&challenge, "cred-prefix-001", "0x Prefix Key");
    response.public_key_x = format!("0x{}", TEST_PUB_KEY_X);
    response.public_key_y = format!("0x{}", TEST_PUB_KEY_Y);

    let result = service.complete_registration(&response).await;
    assert!(
        result.is_ok(),
        "Public keys with 0x prefix should be accepted"
    );

    let cred = service
        .passkey_service()
        .get_passkey(user_email, "cred-prefix-001")
        .await
        .unwrap();
    assert!(cred.is_active);
}

// ============================================================================
// E2E Test 20: Challenge Uniqueness
// ============================================================================

#[tokio::test]
async fn test_challenge_uniqueness() {
    let service = test_ceremony_service();
    let user_email = "unique@example.com";

    // Generate 100 challenges and verify they are all unique
    let mut challenges = std::collections::HashSet::new();

    for _ in 0..100 {
        let challenge = service.begin_registration(user_email).await.unwrap();
        assert!(
            challenges.insert(challenge.clone()),
            "Challenge '{}' was duplicated",
            challenge
        );
    }

    assert_eq!(challenges.len(), 100, "All 100 challenges must be unique");
}
