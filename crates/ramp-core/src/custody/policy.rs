//! Custody Policy Engine
//!
//! Enforces transaction authorization policies including:
//! - Address whitelisting
//! - Daily transaction limits
//! - Multi-approval thresholds
//! - Time-based restrictions

use chrono::{DateTime, Datelike, NaiveTime, Utc, Weekday};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
use tracing::info;

/// A transaction to be evaluated against custody policy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionRequest {
    /// Destination address
    pub to_address: String,
    /// Transaction amount
    pub amount: Decimal,
    /// Currency / token symbol
    pub currency: String,
    /// Chain identifier
    pub chain_id: Option<String>,
}

/// Time restriction for transactions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeRestriction {
    /// Allowed start time (UTC)
    pub start_time: NaiveTime,
    /// Allowed end time (UTC)
    pub end_time: NaiveTime,
    /// Allowed days of the week (empty = all days allowed)
    pub allowed_days: Vec<Weekday>,
}

/// Custody policy configuration for a user or account.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustodyPolicy {
    /// Whitelisted destination addresses (empty = all allowed)
    pub whitelist_addresses: Vec<String>,
    /// Maximum daily transaction amount (in USD equivalent)
    pub daily_limit: Decimal,
    /// Transactions above this amount require multi-party approval
    pub require_multi_approval_above: Decimal,
    /// Time-based restrictions (None = no time restrictions)
    pub time_restrictions: Option<TimeRestriction>,
    /// Whether the policy is active
    pub enabled: bool,
    /// When the policy was created
    pub created_at: DateTime<Utc>,
    /// When the policy was last updated
    pub updated_at: DateTime<Utc>,
}

impl CustodyPolicy {
    /// Create a default permissive policy.
    pub fn permissive() -> Self {
        let now = Utc::now();
        Self {
            whitelist_addresses: Vec::new(),
            daily_limit: Decimal::new(1_000_000, 0), // 1M USD
            require_multi_approval_above: Decimal::new(100_000, 0), // 100K USD
            time_restrictions: None,
            enabled: true,
            created_at: now,
            updated_at: now,
        }
    }

    /// Create a strict policy with lower limits.
    pub fn strict() -> Self {
        let now = Utc::now();
        Self {
            whitelist_addresses: Vec::new(),
            daily_limit: Decimal::new(10_000, 0), // 10K USD
            require_multi_approval_above: Decimal::new(1_000, 0), // 1K USD
            time_restrictions: Some(TimeRestriction {
                start_time: NaiveTime::from_hms_opt(8, 0, 0).unwrap(),
                end_time: NaiveTime::from_hms_opt(18, 0, 0).unwrap(),
                allowed_days: vec![
                    Weekday::Mon,
                    Weekday::Tue,
                    Weekday::Wed,
                    Weekday::Thu,
                    Weekday::Fri,
                ],
            }),
            enabled: true,
            created_at: now,
            updated_at: now,
        }
    }
}

/// Result of a policy check.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicyDecision {
    /// Transaction is allowed under the current policy
    Allow,
    /// Transaction is denied with a reason
    Deny(String),
    /// Transaction requires additional multi-party approval
    RequireApproval(String),
}

/// Policy Engine for evaluating transactions against custody policies.
pub struct PolicyEngine {
    /// User policies: user_id -> CustodyPolicy
    policies: Mutex<HashMap<String, CustodyPolicy>>,
    /// Daily spend tracking: user_id -> cumulative amount today
    daily_spend: Mutex<HashMap<String, Decimal>>,
}

impl PolicyEngine {
    pub fn new() -> Self {
        Self {
            policies: Mutex::new(HashMap::new()),
            daily_spend: Mutex::new(HashMap::new()),
        }
    }

    /// Check a transaction against the user's custody policy.
    pub fn check_policy(
        &self,
        user_id: &str,
        transaction: &TransactionRequest,
    ) -> PolicyDecision {
        let policies = self.policies.lock().unwrap();
        let policy = match policies.get(user_id) {
            Some(p) => p.clone(),
            None => {
                // No policy configured: allow by default
                return PolicyDecision::Allow;
            }
        };
        drop(policies);

        if !policy.enabled {
            return PolicyDecision::Allow;
        }

        // Check 1: Whitelist
        if !policy.whitelist_addresses.is_empty() {
            let addr_lower = transaction.to_address.to_lowercase();
            let is_whitelisted = policy
                .whitelist_addresses
                .iter()
                .any(|a| a.to_lowercase() == addr_lower);

            if !is_whitelisted {
                return PolicyDecision::Deny(format!(
                    "Address {} is not whitelisted",
                    transaction.to_address
                ));
            }
        }

        // Check 2: Time restrictions
        if let Some(ref time_restriction) = policy.time_restrictions {
            let now = Utc::now();
            let current_time = now.time();
            let current_day = now.weekday();

            // Check day restriction
            if !time_restriction.allowed_days.is_empty()
                && !time_restriction.allowed_days.contains(&current_day)
            {
                return PolicyDecision::Deny(format!(
                    "Transactions not allowed on {:?}",
                    current_day
                ));
            }

            // Check time window
            let in_window = if time_restriction.start_time <= time_restriction.end_time {
                current_time >= time_restriction.start_time
                    && current_time <= time_restriction.end_time
            } else {
                // Overnight window (e.g., 22:00 - 06:00)
                current_time >= time_restriction.start_time
                    || current_time <= time_restriction.end_time
            };

            if !in_window {
                return PolicyDecision::Deny(format!(
                    "Transaction outside allowed time window ({} - {})",
                    time_restriction.start_time, time_restriction.end_time
                ));
            }
        }

        // Check 3: Daily limit
        let current_daily = self
            .daily_spend
            .lock()
            .unwrap()
            .get(user_id)
            .cloned()
            .unwrap_or(Decimal::ZERO);

        let projected = current_daily + transaction.amount;
        if projected > policy.daily_limit {
            return PolicyDecision::Deny(format!(
                "Daily limit exceeded: {} + {} > {} limit",
                current_daily, transaction.amount, policy.daily_limit
            ));
        }

        // Check 4: Multi-approval threshold
        if transaction.amount > policy.require_multi_approval_above {
            return PolicyDecision::RequireApproval(format!(
                "Amount {} exceeds multi-approval threshold of {}",
                transaction.amount, policy.require_multi_approval_above
            ));
        }

        info!(
            user_id,
            to_address = transaction.to_address.as_str(),
            amount = %transaction.amount,
            "Policy check passed"
        );

        PolicyDecision::Allow
    }

    /// Update or create a custody policy for a user.
    pub fn update_policy(
        &self,
        user_id: &str,
        policy: CustodyPolicy,
    ) {
        let mut policies = self.policies.lock().unwrap();
        policies.insert(user_id.to_string(), policy);
        info!(user_id, "Updated custody policy");
    }

    /// Get the custody policy for a user.
    pub fn get_policy(&self, user_id: &str) -> Option<CustodyPolicy> {
        self.policies.lock().unwrap().get(user_id).cloned()
    }

    /// Record a transaction amount against the user's daily spend.
    pub fn record_spend(&self, user_id: &str, amount: Decimal) {
        let mut daily = self.daily_spend.lock().unwrap();
        let current = daily.entry(user_id.to_string()).or_insert(Decimal::ZERO);
        *current += amount;
    }

    /// Reset daily spend tracking (should be called at day boundary).
    pub fn reset_daily_spend(&self) {
        self.daily_spend.lock().unwrap().clear();
    }

    /// Get current daily spend for a user.
    pub fn get_daily_spend(&self, user_id: &str) -> Decimal {
        self.daily_spend
            .lock()
            .unwrap()
            .get(user_id)
            .cloned()
            .unwrap_or(Decimal::ZERO)
    }
}

impl Default for PolicyEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_transaction(to: &str, amount: i64) -> TransactionRequest {
        TransactionRequest {
            to_address: to.to_string(),
            amount: Decimal::new(amount, 0),
            currency: "USDT".to_string(),
            chain_id: Some("1".to_string()),
        }
    }

    #[test]
    fn test_allow_when_no_policy() {
        let engine = PolicyEngine::new();
        let tx = test_transaction("0xabc", 100);

        let decision = engine.check_policy("user-1", &tx);
        assert_eq!(decision, PolicyDecision::Allow);
    }

    #[test]
    fn test_allow_when_policy_disabled() {
        let engine = PolicyEngine::new();
        let mut policy = CustodyPolicy::strict();
        policy.enabled = false;
        engine.update_policy("user-1", policy);

        let tx = test_transaction("0xabc", 100);
        let decision = engine.check_policy("user-1", &tx);
        assert_eq!(decision, PolicyDecision::Allow);
    }

    #[test]
    fn test_whitelist_allows_listed_address() {
        let engine = PolicyEngine::new();
        let mut policy = CustodyPolicy::permissive();
        policy.whitelist_addresses = vec!["0xAAA".to_string(), "0xBBB".to_string()];
        engine.update_policy("user-1", policy);

        let tx = test_transaction("0xaaa", 100); // Case-insensitive match
        let decision = engine.check_policy("user-1", &tx);
        assert_eq!(decision, PolicyDecision::Allow);
    }

    #[test]
    fn test_whitelist_denies_unlisted_address() {
        let engine = PolicyEngine::new();
        let mut policy = CustodyPolicy::permissive();
        policy.whitelist_addresses = vec!["0xAAA".to_string()];
        engine.update_policy("user-1", policy);

        let tx = test_transaction("0xCCC", 100);
        let decision = engine.check_policy("user-1", &tx);
        assert!(matches!(decision, PolicyDecision::Deny(_)));
    }

    #[test]
    fn test_daily_limit_enforcement() {
        let engine = PolicyEngine::new();
        let mut policy = CustodyPolicy::permissive();
        policy.daily_limit = Decimal::new(1000, 0);
        policy.require_multi_approval_above = Decimal::new(10_000, 0);
        engine.update_policy("user-1", policy);

        // Record some spend
        engine.record_spend("user-1", Decimal::new(800, 0));

        // This should exceed the limit (800 + 300 > 1000)
        let tx = test_transaction("0xabc", 300);
        let decision = engine.check_policy("user-1", &tx);
        assert!(matches!(decision, PolicyDecision::Deny(_)));

        // This should fit (800 + 100 <= 1000)
        let tx2 = test_transaction("0xabc", 100);
        let decision2 = engine.check_policy("user-1", &tx2);
        assert_eq!(decision2, PolicyDecision::Allow);
    }

    #[test]
    fn test_multi_approval_threshold() {
        let engine = PolicyEngine::new();
        let mut policy = CustodyPolicy::permissive();
        policy.require_multi_approval_above = Decimal::new(5000, 0);
        engine.update_policy("user-1", policy);

        // Under threshold - allowed
        let tx_small = test_transaction("0xabc", 4999);
        let decision = engine.check_policy("user-1", &tx_small);
        assert_eq!(decision, PolicyDecision::Allow);

        // Over threshold - requires approval
        let tx_large = test_transaction("0xabc", 5001);
        let decision = engine.check_policy("user-1", &tx_large);
        assert!(matches!(decision, PolicyDecision::RequireApproval(_)));
    }

    #[test]
    fn test_get_and_update_policy() {
        let engine = PolicyEngine::new();

        // No policy initially
        assert!(engine.get_policy("user-1").is_none());

        // Set a policy
        let policy = CustodyPolicy::strict();
        engine.update_policy("user-1", policy.clone());

        let retrieved = engine.get_policy("user-1").unwrap();
        assert_eq!(retrieved.daily_limit, Decimal::new(10_000, 0));
        assert!(retrieved.enabled);
    }

    #[test]
    fn test_daily_spend_tracking() {
        let engine = PolicyEngine::new();

        assert_eq!(engine.get_daily_spend("user-1"), Decimal::ZERO);

        engine.record_spend("user-1", Decimal::new(100, 0));
        assert_eq!(engine.get_daily_spend("user-1"), Decimal::new(100, 0));

        engine.record_spend("user-1", Decimal::new(200, 0));
        assert_eq!(engine.get_daily_spend("user-1"), Decimal::new(300, 0));

        engine.reset_daily_spend();
        assert_eq!(engine.get_daily_spend("user-1"), Decimal::ZERO);
    }

    #[test]
    fn test_permissive_policy_defaults() {
        let policy = CustodyPolicy::permissive();
        assert!(policy.whitelist_addresses.is_empty());
        assert_eq!(policy.daily_limit, Decimal::new(1_000_000, 0));
        assert_eq!(
            policy.require_multi_approval_above,
            Decimal::new(100_000, 0)
        );
        assert!(policy.time_restrictions.is_none());
        assert!(policy.enabled);
    }

    #[test]
    fn test_strict_policy_defaults() {
        let policy = CustodyPolicy::strict();
        assert!(policy.whitelist_addresses.is_empty());
        assert_eq!(policy.daily_limit, Decimal::new(10_000, 0));
        assert_eq!(policy.require_multi_approval_above, Decimal::new(1_000, 0));
        assert!(policy.time_restrictions.is_some());
        assert!(policy.enabled);

        let tr = policy.time_restrictions.unwrap();
        assert_eq!(tr.start_time, NaiveTime::from_hms_opt(8, 0, 0).unwrap());
        assert_eq!(tr.end_time, NaiveTime::from_hms_opt(18, 0, 0).unwrap());
        assert_eq!(tr.allowed_days.len(), 5);
    }

    #[test]
    fn test_policy_check_priority_order() {
        // Whitelist check should happen before daily limit and approval threshold
        let engine = PolicyEngine::new();
        let mut policy = CustodyPolicy::permissive();
        policy.whitelist_addresses = vec!["0xAAA".to_string()];
        policy.daily_limit = Decimal::new(100, 0);
        policy.require_multi_approval_above = Decimal::new(50, 0);
        engine.update_policy("user-1", policy);

        // Non-whitelisted address should be denied even if amount is fine
        let tx = test_transaction("0xBBB", 10);
        let decision = engine.check_policy("user-1", &tx);
        assert!(matches!(decision, PolicyDecision::Deny(_)));
    }

    #[test]
    fn test_empty_whitelist_allows_all() {
        let engine = PolicyEngine::new();
        let mut policy = CustodyPolicy::permissive();
        policy.whitelist_addresses = vec![]; // Empty = no whitelist restriction
        engine.update_policy("user-1", policy);

        let tx = test_transaction("0xANYTHING", 100);
        let decision = engine.check_policy("user-1", &tx);
        assert_eq!(decision, PolicyDecision::Allow);
    }

    #[test]
    fn test_multiple_users_independent_policies() {
        let engine = PolicyEngine::new();

        engine.update_policy("user-1", CustodyPolicy::permissive());
        engine.update_policy("user-2", CustodyPolicy::strict());

        let p1 = engine.get_policy("user-1").unwrap();
        let p2 = engine.get_policy("user-2").unwrap();

        assert_ne!(p1.daily_limit, p2.daily_limit);
    }

    #[test]
    fn test_daily_limit_exact_boundary() {
        let engine = PolicyEngine::new();
        let mut policy = CustodyPolicy::permissive();
        policy.daily_limit = Decimal::new(1000, 0);
        policy.require_multi_approval_above = Decimal::new(10_000, 0);
        engine.update_policy("user-1", policy);

        // Exactly at the limit should be allowed
        let tx = test_transaction("0xabc", 1000);
        let decision = engine.check_policy("user-1", &tx);
        assert_eq!(decision, PolicyDecision::Allow);

        // Record the spend
        engine.record_spend("user-1", Decimal::new(1000, 0));

        // Now any additional spend should be denied
        let tx2 = test_transaction("0xabc", 1);
        let decision2 = engine.check_policy("user-1", &tx2);
        assert!(matches!(decision2, PolicyDecision::Deny(_)));
    }

    #[test]
    fn test_approval_threshold_exact_boundary() {
        let engine = PolicyEngine::new();
        let mut policy = CustodyPolicy::permissive();
        policy.require_multi_approval_above = Decimal::new(5000, 0);
        engine.update_policy("user-1", policy);

        // Exactly at threshold - allowed (not above)
        let tx = test_transaction("0xabc", 5000);
        let decision = engine.check_policy("user-1", &tx);
        assert_eq!(decision, PolicyDecision::Allow);
    }
}
