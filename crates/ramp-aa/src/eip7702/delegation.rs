//! EIP-7702 Delegation Management
//!
//! Manages EOA delegations to smart contracts with:
//! - Session-based delegations with expiry
//! - Revocation mechanism
//! - Delegation registry for tracking active delegations

use chrono::{DateTime, Utc};
use alloy::primitives::{Address, U256};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::RwLock;

use super::{Eip7702Config, Eip7702Error, Result};

/// Delegation status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DelegationStatus {
    /// Delegation is active and valid
    Active,
    /// Delegation has expired
    Expired,
    /// Delegation was revoked by the EOA owner
    Revoked,
    /// Delegation is pending (not yet on-chain)
    Pending,
}

/// Core delegation data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Delegation {
    /// The EOA that is delegating
    pub delegator: Address,
    /// The smart contract code to delegate to
    pub delegate: Address,
    /// Chain ID
    pub chain_id: U256,
    /// EOA nonce at time of delegation
    pub nonce: u64,
    /// When the delegation was created
    pub created_at: DateTime<Utc>,
    /// Current status
    pub status: DelegationStatus,
}

impl Delegation {
    pub fn new(delegator: Address, delegate: Address, chain_id: U256, nonce: u64) -> Self {
        Self {
            delegator,
            delegate,
            chain_id,
            nonce,
            created_at: Utc::now(),
            status: DelegationStatus::Pending,
        }
    }

    /// Check if delegation is currently valid
    pub fn is_active(&self) -> bool {
        self.status == DelegationStatus::Active
    }

    /// Mark delegation as active (confirmed on-chain)
    pub fn activate(&mut self) {
        self.status = DelegationStatus::Active;
    }

    /// Mark delegation as revoked
    pub fn revoke(&mut self) {
        self.status = DelegationStatus::Revoked;
    }
}

/// Session-based delegation with time bounds
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionDelegation {
    /// Base delegation data
    pub delegation: Delegation,
    /// When the session becomes valid
    pub valid_after: DateTime<Utc>,
    /// When the session expires
    pub valid_until: DateTime<Utc>,
    /// Optional permissions/restrictions
    pub permissions: Option<SessionPermissions>,
    /// Session identifier
    pub session_id: String,
}

impl SessionDelegation {
    pub fn new(
        delegator: Address,
        delegate: Address,
        chain_id: U256,
        nonce: u64,
        duration_secs: u64,
    ) -> Self {
        let now = Utc::now();
        let valid_until = now + chrono::Duration::seconds(duration_secs as i64);

        Self {
            delegation: Delegation::new(delegator, delegate, chain_id, nonce),
            valid_after: now,
            valid_until,
            permissions: None,
            session_id: generate_session_id(),
        }
    }

    pub fn with_permissions(mut self, permissions: SessionPermissions) -> Self {
        self.permissions = Some(permissions);
        self
    }

    /// Check if the session is currently valid
    pub fn is_valid(&self) -> bool {
        let now = Utc::now();
        self.delegation.is_active() && now >= self.valid_after && now < self.valid_until
    }

    /// Check if the session has expired
    pub fn is_expired(&self) -> bool {
        Utc::now() >= self.valid_until
    }

    /// Get remaining validity duration in seconds
    pub fn remaining_seconds(&self) -> i64 {
        let remaining = self.valid_until - Utc::now();
        remaining.num_seconds().max(0)
    }
}

/// Permissions for a session delegation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionPermissions {
    /// Allowed target contracts
    pub allowed_targets: Vec<Address>,
    /// Maximum value per transaction
    pub max_value_per_tx: Option<U256>,
    /// Maximum total value for session
    pub max_total_value: Option<U256>,
    /// Allowed function selectors
    pub allowed_selectors: Vec<[u8; 4]>,
}

impl SessionPermissions {
    pub fn new() -> Self {
        Self {
            allowed_targets: Vec::new(),
            max_value_per_tx: None,
            max_total_value: None,
            allowed_selectors: Vec::new(),
        }
    }

    pub fn allow_target(mut self, target: Address) -> Self {
        self.allowed_targets.push(target);
        self
    }

    pub fn with_max_value(mut self, max: U256) -> Self {
        self.max_value_per_tx = Some(max);
        self
    }

    pub fn allow_selector(mut self, selector: [u8; 4]) -> Self {
        self.allowed_selectors.push(selector);
        self
    }

    /// Check if a call is permitted
    pub fn is_permitted(&self, target: Address, value: U256, selector: Option<[u8; 4]>) -> bool {
        // Check target
        if !self.allowed_targets.is_empty() && !self.allowed_targets.contains(&target) {
            return false;
        }

        // Check value
        if let Some(max) = self.max_value_per_tx {
            if value > max {
                return false;
            }
        }

        // Check selector
        if let Some(sel) = selector {
            if !self.allowed_selectors.is_empty() && !self.allowed_selectors.contains(&sel) {
                return false;
            }
        }

        true
    }
}

impl Default for SessionPermissions {
    fn default() -> Self {
        Self::new()
    }
}

/// In-memory delegation registry
pub struct DelegationRegistry {
    /// Map of delegator address to their delegations
    delegations: RwLock<HashMap<Address, Vec<Delegation>>>,
    /// Map of session ID to session delegations
    sessions: RwLock<HashMap<String, SessionDelegation>>,
}

impl DelegationRegistry {
    pub fn new() -> Self {
        Self {
            delegations: RwLock::new(HashMap::new()),
            sessions: RwLock::new(HashMap::new()),
        }
    }

    /// Register a new delegation
    pub fn register(&self, delegation: Delegation) -> Result<()> {
        let mut delegations = self.delegations.write().unwrap();
        delegations
            .entry(delegation.delegator)
            .or_insert_with(Vec::new)
            .push(delegation);
        Ok(())
    }

    /// Register a session delegation
    pub fn register_session(&self, session: SessionDelegation) -> Result<String> {
        let session_id = session.session_id.clone();
        let _delegator = session.delegation.delegator;

        // Register base delegation
        self.register(session.delegation.clone())?;

        // Store session
        let mut sessions = self.sessions.write().unwrap();
        sessions.insert(session_id.clone(), session);

        Ok(session_id)
    }

    /// Get active delegation for an address
    pub fn get_active(&self, delegator: Address) -> Option<Delegation> {
        let delegations = self.delegations.read().unwrap();
        delegations.get(&delegator).and_then(|list| {
            list.iter()
                .find(|d| d.status == DelegationStatus::Active)
                .cloned()
        })
    }

    /// Get session by ID
    pub fn get_session(&self, session_id: &str) -> Option<SessionDelegation> {
        let sessions = self.sessions.read().unwrap();
        sessions.get(session_id).cloned()
    }

    /// Get all sessions for a delegator
    pub fn get_sessions_for(&self, delegator: Address) -> Vec<SessionDelegation> {
        let sessions = self.sessions.read().unwrap();
        sessions
            .values()
            .filter(|s| s.delegation.delegator == delegator)
            .cloned()
            .collect()
    }

    /// Revoke a delegation
    pub fn revoke(&self, delegator: Address) -> Result<()> {
        let mut delegations = self.delegations.write().unwrap();
        if let Some(list) = delegations.get_mut(&delegator) {
            for d in list.iter_mut() {
                if d.status == DelegationStatus::Active {
                    d.status = DelegationStatus::Revoked;
                }
            }
            Ok(())
        } else {
            Err(Eip7702Error::DelegationNotFound(delegator))
        }
    }

    /// Revoke a specific session
    pub fn revoke_session(&self, session_id: &str) -> Result<()> {
        let mut sessions = self.sessions.write().unwrap();
        if let Some(session) = sessions.get_mut(session_id) {
            session.delegation.revoke();
            Ok(())
        } else {
            Err(Eip7702Error::DelegationNotFound(Address::ZERO))
        }
    }

    /// Clean up expired sessions
    pub fn cleanup_expired(&self) {
        let mut sessions = self.sessions.write().unwrap();
        sessions.retain(|_, s| !s.is_expired());
    }
}

impl Default for DelegationRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Delegation manager with configuration
pub struct DelegationManager {
    config: Eip7702Config,
    registry: DelegationRegistry,
}

impl DelegationManager {
    pub fn new(config: Eip7702Config) -> Self {
        Self {
            config,
            registry: DelegationRegistry::new(),
        }
    }

    /// Create a new delegation
    pub fn create_delegation(
        &self,
        delegator: Address,
        delegate: Address,
        nonce: u64,
    ) -> Result<Delegation> {
        // Check for existing active or pending delegation
        let delegations = self.registry.delegations.read().unwrap();
        let has_existing = delegations.get(&delegator).map_or(false, |list| {
            list.iter().any(|d| d.status == DelegationStatus::Active || d.status == DelegationStatus::Pending)
        });
        drop(delegations);

        if has_existing {
            return Err(Eip7702Error::DelegationAlreadyExists(delegator));
        }

        let delegation = Delegation::new(delegator, delegate, self.config.chain_id, nonce);
        self.registry.register(delegation.clone())?;

        Ok(delegation)
    }

    /// Create a session-based delegation
    pub fn create_session(
        &self,
        delegator: Address,
        delegate: Address,
        nonce: u64,
        duration_secs: u64,
        permissions: Option<SessionPermissions>,
    ) -> Result<SessionDelegation> {
        // Validate duration
        if duration_secs > self.config.max_delegation_duration {
            return Err(Eip7702Error::DurationExceedsMax(
                duration_secs,
                self.config.max_delegation_duration,
            ));
        }

        let mut session =
            SessionDelegation::new(delegator, delegate, self.config.chain_id, nonce, duration_secs);

        if let Some(perms) = permissions {
            session = session.with_permissions(perms);
        }

        self.registry.register_session(session.clone())?;

        Ok(session)
    }

    /// Get active delegation for an address
    pub fn get_delegation(&self, delegator: Address) -> Option<Delegation> {
        self.registry.get_active(delegator)
    }

    /// Get session by ID
    pub fn get_session(&self, session_id: &str) -> Option<SessionDelegation> {
        self.registry.get_session(session_id)
    }

    /// Revoke delegation for an address
    pub fn revoke(&self, delegator: Address) -> Result<()> {
        if !self.config.allow_revocation {
            return Err(Eip7702Error::RevocationNotAllowed);
        }
        self.registry.revoke(delegator)
    }

    /// Revoke a specific session
    pub fn revoke_session(&self, session_id: &str) -> Result<()> {
        if !self.config.allow_revocation {
            return Err(Eip7702Error::RevocationNotAllowed);
        }
        self.registry.revoke_session(session_id)
    }

    /// Check if a call is permitted for a session
    pub fn check_session_permission(
        &self,
        session_id: &str,
        target: Address,
        value: U256,
        selector: Option<[u8; 4]>,
    ) -> Result<bool> {
        let session = self
            .registry
            .get_session(session_id)
            .ok_or(Eip7702Error::DelegationNotFound(Address::ZERO))?;

        if !session.is_valid() {
            if session.is_expired() {
                return Err(Eip7702Error::AuthorizationExpired(
                    session.valid_until.timestamp() as u64,
                ));
            }
            return Err(Eip7702Error::DelegationRevoked);
        }

        if let Some(ref perms) = session.permissions {
            Ok(perms.is_permitted(target, value, selector))
        } else {
            // No permissions means everything is allowed
            Ok(true)
        }
    }
}

/// Generate a unique session ID
fn generate_session_id() -> String {
    use uuid::Uuid;
    format!("session_{}", Uuid::now_v7())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_address() -> Address {
        "0x1234567890123456789012345678901234567890"
            .parse()
            .unwrap()
    }

    fn test_delegate() -> Address {
        "0xabcdefabcdefabcdefabcdefabcdefabcdefabcd"
            .parse()
            .unwrap()
    }

    #[test]
    fn test_delegation_creation() {
        let delegation = Delegation::new(test_address(), test_delegate(), U256::from(1), 0);

        assert_eq!(delegation.delegator, test_address());
        assert_eq!(delegation.delegate, test_delegate());
        assert_eq!(delegation.status, DelegationStatus::Pending);
        assert!(!delegation.is_active());
    }

    #[test]
    fn test_delegation_activation() {
        let mut delegation = Delegation::new(test_address(), test_delegate(), U256::from(1), 0);

        delegation.activate();
        assert!(delegation.is_active());
        assert_eq!(delegation.status, DelegationStatus::Active);
    }

    #[test]
    fn test_session_delegation() {
        let session =
            SessionDelegation::new(test_address(), test_delegate(), U256::from(1), 0, 3600);

        assert!(!session.is_expired());
        assert!(session.remaining_seconds() > 0);
        assert!(session.remaining_seconds() <= 3600);
    }

    #[test]
    fn test_session_permissions() {
        let perms = SessionPermissions::new()
            .allow_target(test_address())
            .with_max_value(U256::from(1000));

        assert!(perms.is_permitted(test_address(), U256::from(500), None));
        assert!(!perms.is_permitted(test_address(), U256::from(1500), None));
        assert!(!perms.is_permitted(test_delegate(), U256::from(500), None));
    }

    #[test]
    fn test_delegation_registry() {
        let registry = DelegationRegistry::new();

        let delegation = Delegation::new(test_address(), test_delegate(), U256::from(1), 0);
        registry.register(delegation).unwrap();

        // Should not find active since it's pending
        assert!(registry.get_active(test_address()).is_none());
    }

    #[test]
    fn test_delegation_manager() {
        let config = Eip7702Config::new(1, test_delegate());
        let manager = DelegationManager::new(config);

        let delegation = manager
            .create_delegation(test_address(), test_delegate(), 0)
            .unwrap();

        assert_eq!(delegation.chain_id, U256::from(1));

        // Should fail to create another
        assert!(manager
            .create_delegation(test_address(), test_delegate(), 1)
            .is_err());
    }

    #[test]
    fn test_session_creation_with_duration_limit() {
        let config = Eip7702Config::new(1, test_delegate()).with_max_duration(3600);
        let manager = DelegationManager::new(config);

        // Should succeed within limit
        let session = manager
            .create_session(test_address(), test_delegate(), 0, 1800, None)
            .unwrap();
        assert!(session.remaining_seconds() > 0);

        // Different delegator should also work
        let other = "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
            .parse()
            .unwrap();

        // Should fail exceeding limit
        let result = manager.create_session(other, test_delegate(), 0, 7200, None);
        assert!(matches!(result, Err(Eip7702Error::DurationExceedsMax(_, _))));
    }
}
