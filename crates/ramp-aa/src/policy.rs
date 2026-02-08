use alloy::primitives::{Address, U256};
use ramp_common::{types::TenantId, Result};

use crate::types::{PermissionRule, SessionKey, SessionPermission};
use crate::user_operation::UserOperation;

/// Policy engine for AA operations
pub struct PolicyEngine {
    _tenant_id: TenantId,
}

impl PolicyEngine {
    pub fn new(tenant_id: TenantId) -> Self {
        Self {
            _tenant_id: tenant_id,
        }
    }

    /// Validate UserOperation against policies
    pub async fn validate_user_operation(&self, user_op: &UserOperation) -> Result<PolicyResult> {
        let mut result = PolicyResult::default();

        // Check gas limits
        if user_op.call_gas_limit > U256::from(1_000_000) {
            result.add_violation("Call gas limit too high");
        }

        // Check value (if encoded in call_data)
        // In production, would decode and check

        result.is_valid = result.violations.is_empty();
        Ok(result)
    }

    /// Validate session key usage
    pub async fn validate_session_key(
        &self,
        session: &SessionKey,
        target: Address,
        selector: [u8; 4],
        value: U256,
    ) -> Result<PolicyResult> {
        let mut result = PolicyResult::default();
        let now = chrono::Utc::now().timestamp() as u64;

        // Check time validity
        if now < session.valid_after {
            result.add_violation("Session not yet valid");
            return Ok(result);
        }

        if now > session.valid_until {
            result.add_violation("Session expired");
            return Ok(result);
        }

        // Find matching permission
        let permission = session
            .permissions
            .iter()
            .find(|p| p.target == target && p.selector == selector);

        let Some(permission) = permission else {
            result.add_violation("No permission for this call");
            return Ok(result);
        };

        // Check value limit
        if value > permission.max_value {
            result.add_violation("Value exceeds permission limit");
        }

        // Check rules
        for rule in &permission.rules {
            match rule {
                PermissionRule::MaxAmount(max) => {
                    if value > *max {
                        result.add_violation("Amount exceeds rule limit");
                    }
                }
                PermissionRule::AllowedRecipients(_recipients) => {
                    // Would need to decode call_data to get recipient
                }
                PermissionRule::TimeWindow { start, end } => {
                    if now < *start || now > *end {
                        result.add_violation("Outside allowed time window");
                    }
                }
                PermissionRule::RateLimit {
                    count: _,
                    period_secs: _,
                } => {
                    // Would need to query recent usage
                }
            }
        }

        result.is_valid = result.violations.is_empty();
        Ok(result)
    }

    /// Create a new session key with default permissions
    pub fn create_session_key(
        &self,
        key_address: Address,
        valid_for_secs: u64,
        permissions: Vec<SessionPermission>,
    ) -> SessionKey {
        let now = chrono::Utc::now().timestamp() as u64;

        SessionKey {
            key_address,
            valid_after: now,
            valid_until: now + valid_for_secs,
            permissions,
        }
    }
}

/// Result of policy validation
#[derive(Debug, Clone, Default)]
pub struct PolicyResult {
    pub is_valid: bool,
    pub violations: Vec<String>,
}

impl PolicyResult {
    pub fn add_violation(&mut self, msg: &str) {
        self.is_valid = false;
        self.violations.push(msg.to_string());
    }
}
