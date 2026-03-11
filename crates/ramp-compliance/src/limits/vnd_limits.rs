//! VND Transaction Limits
//!
//! Implements configurable daily/monthly VND transaction limits per KYC tier
//! following Vietnam SBV (State Bank of Vietnam) regulations.
//!
//! ## Tier Limits (Default - SBV Compliant)
//!
//! | Tier | Daily Limit | Monthly Limit | Description |
//! |------|-------------|---------------|-------------|
//! | Tier 1 (Basic) | 100M VND | 1B VND | Basic eKYC verified |
//! | Tier 2 (Verified) | 500M VND | 5B VND | Enhanced KYC verified |
//! | Tier 3 (Premium) | Custom | Custom | Business/high-value customers |

use async_trait::async_trait;
use chrono::{DateTime, Datelike, NaiveTime, TimeZone, Utc};
use chrono_tz::Asia::Ho_Chi_Minh;
use ramp_common::{
    types::{TenantId, UserId},
    Result,
};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{info, warn};

use crate::types::KycTier;

/// VND limit check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VndLimitResult {
    /// Transaction is within limits
    Approved {
        daily_remaining: Decimal,
        monthly_remaining: Decimal,
    },
    /// Daily limit exceeded
    DailyLimitExceeded {
        current_daily_used: Decimal,
        daily_limit: Decimal,
        requested_amount: Decimal,
    },
    /// Monthly limit exceeded
    MonthlyLimitExceeded {
        current_monthly_used: Decimal,
        monthly_limit: Decimal,
        requested_amount: Decimal,
    },
    /// Single transaction exceeds limit
    SingleTransactionExceeded {
        single_limit: Decimal,
        requested_amount: Decimal,
    },
    /// User tier not allowed
    TierNotAllowed { tier: KycTier, reason: String },
}

impl VndLimitResult {
    pub fn is_approved(&self) -> bool {
        matches!(self, VndLimitResult::Approved { .. })
    }
}

/// Default VND transaction limits per tier (SBV compliant)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VndTierLimits {
    /// Maximum single transaction amount in VND
    pub single_transaction_limit: Decimal,
    /// Daily transaction limit in VND
    pub daily_limit: Decimal,
    /// Monthly transaction limit in VND
    pub monthly_limit: Decimal,
    /// Whether this tier requires manual approval for large transactions
    pub requires_manual_approval_threshold: Option<Decimal>,
}

impl VndTierLimits {
    /// Default limits for Tier 0 (Unverified) - No transactions allowed
    pub fn tier0() -> Self {
        Self {
            single_transaction_limit: Decimal::ZERO,
            daily_limit: Decimal::ZERO,
            monthly_limit: Decimal::ZERO,
            requires_manual_approval_threshold: None,
        }
    }

    /// Default limits for Tier 1 (Basic eKYC) - 100M VND/day, 1B VND/month
    pub fn tier1() -> Self {
        Self {
            single_transaction_limit: Decimal::from(50_000_000), // 50M VND per transaction
            daily_limit: Decimal::from(100_000_000),             // 100M VND per day
            monthly_limit: Decimal::from(1_000_000_000),         // 1B VND per month
            requires_manual_approval_threshold: Some(Decimal::from(80_000_000)), // 80M VND
        }
    }

    /// Default limits for Tier 2 (Verified) - 500M VND/day, 5B VND/month
    pub fn tier2() -> Self {
        Self {
            single_transaction_limit: Decimal::from(200_000_000), // 200M VND per transaction
            daily_limit: Decimal::from(500_000_000),              // 500M VND per day
            monthly_limit: Decimal::from(5_000_000_000i64),       // 5B VND per month
            requires_manual_approval_threshold: Some(Decimal::from(400_000_000)), // 400M VND
        }
    }

    /// Default limits for Tier 3 (Premium) - Custom limits with approval
    pub fn tier3() -> Self {
        Self {
            single_transaction_limit: Decimal::from(1_000_000_000), // 1B VND per transaction
            daily_limit: Decimal::MAX,                              // Unlimited
            monthly_limit: Decimal::MAX,                            // Unlimited
            requires_manual_approval_threshold: Some(Decimal::from(500_000_000)), // 500M VND
        }
    }

    /// Get default limits for a given tier
    pub fn for_tier(tier: KycTier) -> Self {
        match tier {
            KycTier::Tier0 => Self::tier0(),
            KycTier::Tier1 => Self::tier1(),
            KycTier::Tier2 => Self::tier2(),
            KycTier::Tier3 => Self::tier3(),
        }
    }
}

/// VND limit configuration for a tenant
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VndLimitConfig {
    /// Custom tier limits (overrides defaults)
    #[serde(default)]
    pub tier_limits: HashMap<i16, VndTierLimits>,
    /// Whether to reset daily limits at midnight Vietnam time
    #[serde(default = "default_true")]
    pub reset_at_vietnam_midnight: bool,
    /// Whether to enforce limits on payin transactions
    #[serde(default = "default_true")]
    pub enforce_on_payin: bool,
    /// Whether to enforce limits on payout transactions
    #[serde(default = "default_true")]
    pub enforce_on_payout: bool,
    /// Timezone for limit reset (default: Asia/Ho_Chi_Minh)
    #[serde(default = "default_timezone")]
    pub timezone: String,
}

fn default_true() -> bool {
    true
}

fn default_timezone() -> String {
    "Asia/Ho_Chi_Minh".to_string()
}

impl Default for VndLimitConfig {
    fn default() -> Self {
        Self {
            tier_limits: HashMap::new(),
            reset_at_vietnam_midnight: true,
            enforce_on_payin: true,
            enforce_on_payout: true,
            timezone: default_timezone(),
        }
    }
}

impl VndLimitConfig {
    /// Get limits for a specific tier, using custom config or defaults
    pub fn get_tier_limits(&self, tier: KycTier) -> VndTierLimits {
        self.tier_limits
            .get(&(tier as i16))
            .cloned()
            .unwrap_or_else(|| VndTierLimits::for_tier(tier))
    }
}

/// User's current limit status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VndUserLimitStatus {
    pub user_id: String,
    pub tenant_id: String,
    pub tier: KycTier,
    /// Amount used today in VND
    pub daily_used: Decimal,
    /// Amount used this month in VND
    pub monthly_used: Decimal,
    /// Daily limit for user's tier
    pub daily_limit: Decimal,
    /// Monthly limit for user's tier
    pub monthly_limit: Decimal,
    /// Remaining daily limit
    pub daily_remaining: Decimal,
    /// Remaining monthly limit
    pub monthly_remaining: Decimal,
    /// Last reset timestamp for daily limit
    pub daily_reset_at: DateTime<Utc>,
    /// Last reset timestamp for monthly limit
    pub monthly_reset_at: DateTime<Utc>,
    /// Next daily reset time (Vietnam midnight)
    pub next_daily_reset: DateTime<Utc>,
    /// Next monthly reset time (1st of next month, Vietnam time)
    pub next_monthly_reset: DateTime<Utc>,
}

/// Data provider for VND limit checking
#[async_trait]
pub trait VndLimitDataProvider: Send + Sync {
    /// Get user's daily used amount (since last reset)
    async fn get_daily_used(
        &self,
        tenant_id: &TenantId,
        user_id: &UserId,
        since: DateTime<Utc>,
    ) -> Result<Decimal>;

    /// Get user's monthly used amount (since last reset)
    async fn get_monthly_used(
        &self,
        tenant_id: &TenantId,
        user_id: &UserId,
        since: DateTime<Utc>,
    ) -> Result<Decimal>;

    /// Get user's KYC tier
    async fn get_user_tier(&self, tenant_id: &TenantId, user_id: &UserId) -> Result<KycTier>;

    /// Record a successful transaction for limit tracking
    async fn record_transaction(
        &self,
        tenant_id: &TenantId,
        user_id: &UserId,
        amount_vnd: Decimal,
        transaction_type: &str,
    ) -> Result<()>;

    /// Get user's custom limit override (if any)
    async fn get_user_limit_override(
        &self,
        tenant_id: &TenantId,
        user_id: &UserId,
    ) -> Result<Option<VndTierLimits>>;
}

/// VND Transaction Limit Checker
pub struct VndLimitChecker {
    config: VndLimitConfig,
    data_provider: Arc<dyn VndLimitDataProvider>,
}

impl VndLimitChecker {
    pub fn new(config: VndLimitConfig, data_provider: Arc<dyn VndLimitDataProvider>) -> Self {
        Self {
            config,
            data_provider,
        }
    }

    /// Get the start of the current day in Vietnam time (for daily reset)
    pub fn get_daily_reset_time(&self) -> DateTime<Utc> {
        let now_vn = Utc::now().with_timezone(&Ho_Chi_Minh);
        let today_midnight_vn = Ho_Chi_Minh
            .with_ymd_and_hms(now_vn.year(), now_vn.month(), now_vn.day(), 0, 0, 0)
            .single()
            .unwrap_or_else(|| {
                now_vn
                    .date_naive()
                    .and_time(NaiveTime::MIN)
                    .and_utc()
                    .with_timezone(&Ho_Chi_Minh)
            });
        today_midnight_vn.with_timezone(&Utc)
    }

    /// Get the start of the current month in Vietnam time (for monthly reset)
    pub fn get_monthly_reset_time(&self) -> DateTime<Utc> {
        let now_vn = Utc::now().with_timezone(&Ho_Chi_Minh);
        let month_start_vn = Ho_Chi_Minh
            .with_ymd_and_hms(now_vn.year(), now_vn.month(), 1, 0, 0, 0)
            .single()
            .unwrap_or_else(|| {
                now_vn
                    .date_naive()
                    .and_time(NaiveTime::MIN)
                    .and_utc()
                    .with_timezone(&Ho_Chi_Minh)
            });
        month_start_vn.with_timezone(&Utc)
    }

    /// Get next daily reset time (next midnight Vietnam time)
    pub fn get_next_daily_reset(&self) -> DateTime<Utc> {
        let now_vn = Utc::now().with_timezone(&Ho_Chi_Minh);
        let tomorrow = now_vn
            .date_naive()
            .succ_opt()
            .unwrap_or(now_vn.date_naive());
        let tomorrow_midnight_vn = Ho_Chi_Minh
            .with_ymd_and_hms(tomorrow.year(), tomorrow.month(), tomorrow.day(), 0, 0, 0)
            .single()
            .unwrap_or_else(|| {
                tomorrow
                    .and_time(NaiveTime::MIN)
                    .and_utc()
                    .with_timezone(&Ho_Chi_Minh)
            });
        tomorrow_midnight_vn.with_timezone(&Utc)
    }

    /// Get next monthly reset time (1st of next month, midnight Vietnam time)
    pub fn get_next_monthly_reset(&self) -> DateTime<Utc> {
        let now_vn = Utc::now().with_timezone(&Ho_Chi_Minh);
        let (next_year, next_month) = if now_vn.month() == 12 {
            (now_vn.year() + 1, 1)
        } else {
            (now_vn.year(), now_vn.month() + 1)
        };
        let next_month_start_vn = Ho_Chi_Minh
            .with_ymd_and_hms(next_year, next_month, 1, 0, 0, 0)
            .single()
            .unwrap_or_else(|| {
                now_vn
                    .date_naive()
                    .and_time(NaiveTime::MIN)
                    .and_utc()
                    .with_timezone(&Ho_Chi_Minh)
            });
        next_month_start_vn.with_timezone(&Utc)
    }

    /// Check if a transaction is within limits
    pub async fn check_limit(
        &self,
        tenant_id: &TenantId,
        user_id: &UserId,
        amount_vnd: Decimal,
        transaction_type: &str,
    ) -> Result<VndLimitResult> {
        // Always check tier first - Tier0 users cannot transact regardless of config
        let tier = self.data_provider.get_user_tier(tenant_id, user_id).await?;

        if tier == KycTier::Tier0 {
            return Ok(VndLimitResult::TierNotAllowed {
                tier,
                reason: "Tier 0 users are not allowed to perform transactions. Please complete KYC verification.".to_string(),
            });
        }

        // Skip enforcement based on config (only after tier check)
        match transaction_type {
            "PAYIN" | "PAYIN_VND" if !self.config.enforce_on_payin => {
                return Ok(VndLimitResult::Approved {
                    daily_remaining: Decimal::MAX,
                    monthly_remaining: Decimal::MAX,
                });
            }
            "PAYOUT" | "PAYOUT_VND" | "WITHDRAW" if !self.config.enforce_on_payout => {
                return Ok(VndLimitResult::Approved {
                    daily_remaining: Decimal::MAX,
                    monthly_remaining: Decimal::MAX,
                });
            }
            _ => {}
        }

        // Get limits (check for user override first)
        let limits = if let Some(override_limits) = self
            .data_provider
            .get_user_limit_override(tenant_id, user_id)
            .await?
        {
            override_limits
        } else {
            self.config.get_tier_limits(tier)
        };

        // Check single transaction limit
        if !limits.single_transaction_limit.is_zero()
            && amount_vnd > limits.single_transaction_limit
        {
            warn!(
                user_id = %user_id,
                amount = %amount_vnd,
                limit = %limits.single_transaction_limit,
                "Single transaction limit exceeded"
            );
            return Ok(VndLimitResult::SingleTransactionExceeded {
                single_limit: limits.single_transaction_limit,
                requested_amount: amount_vnd,
            });
        }

        // Get reset times
        let daily_reset = self.get_daily_reset_time();
        let monthly_reset = self.get_monthly_reset_time();

        // Get current usage
        let daily_used = self
            .data_provider
            .get_daily_used(tenant_id, user_id, daily_reset)
            .await?;

        let monthly_used = self
            .data_provider
            .get_monthly_used(tenant_id, user_id, monthly_reset)
            .await?;

        // Check daily limit
        if !limits.daily_limit.is_zero() && limits.daily_limit != Decimal::MAX {
            let new_daily_total = daily_used + amount_vnd;
            if new_daily_total > limits.daily_limit {
                warn!(
                    user_id = %user_id,
                    daily_used = %daily_used,
                    amount = %amount_vnd,
                    limit = %limits.daily_limit,
                    "Daily limit exceeded"
                );
                return Ok(VndLimitResult::DailyLimitExceeded {
                    current_daily_used: daily_used,
                    daily_limit: limits.daily_limit,
                    requested_amount: amount_vnd,
                });
            }
        }

        // Check monthly limit
        if !limits.monthly_limit.is_zero() && limits.monthly_limit != Decimal::MAX {
            let new_monthly_total = monthly_used + amount_vnd;
            if new_monthly_total > limits.monthly_limit {
                warn!(
                    user_id = %user_id,
                    monthly_used = %monthly_used,
                    amount = %amount_vnd,
                    limit = %limits.monthly_limit,
                    "Monthly limit exceeded"
                );
                return Ok(VndLimitResult::MonthlyLimitExceeded {
                    current_monthly_used: monthly_used,
                    monthly_limit: limits.monthly_limit,
                    requested_amount: amount_vnd,
                });
            }
        }

        // Calculate remaining limits
        let daily_remaining = if limits.daily_limit == Decimal::MAX {
            Decimal::MAX
        } else {
            (limits.daily_limit - daily_used - amount_vnd).max(Decimal::ZERO)
        };

        let monthly_remaining = if limits.monthly_limit == Decimal::MAX {
            Decimal::MAX
        } else {
            (limits.monthly_limit - monthly_used - amount_vnd).max(Decimal::ZERO)
        };

        info!(
            user_id = %user_id,
            amount = %amount_vnd,
            tier = ?tier,
            daily_remaining = %daily_remaining,
            monthly_remaining = %monthly_remaining,
            "Transaction limit check passed"
        );

        Ok(VndLimitResult::Approved {
            daily_remaining,
            monthly_remaining,
        })
    }

    /// Record a successful transaction (call after transaction completes)
    pub async fn record_transaction(
        &self,
        tenant_id: &TenantId,
        user_id: &UserId,
        amount_vnd: Decimal,
        transaction_type: &str,
    ) -> Result<()> {
        self.data_provider
            .record_transaction(tenant_id, user_id, amount_vnd, transaction_type)
            .await
    }

    /// Get user's current limit status
    pub async fn get_user_limit_status(
        &self,
        tenant_id: &TenantId,
        user_id: &UserId,
    ) -> Result<VndUserLimitStatus> {
        let tier = self.data_provider.get_user_tier(tenant_id, user_id).await?;
        let limits = self.config.get_tier_limits(tier);

        let daily_reset = self.get_daily_reset_time();
        let monthly_reset = self.get_monthly_reset_time();

        let daily_used = self
            .data_provider
            .get_daily_used(tenant_id, user_id, daily_reset)
            .await?;
        let monthly_used = self
            .data_provider
            .get_monthly_used(tenant_id, user_id, monthly_reset)
            .await?;

        let daily_remaining = if limits.daily_limit == Decimal::MAX {
            Decimal::MAX
        } else {
            (limits.daily_limit - daily_used).max(Decimal::ZERO)
        };

        let monthly_remaining = if limits.monthly_limit == Decimal::MAX {
            Decimal::MAX
        } else {
            (limits.monthly_limit - monthly_used).max(Decimal::ZERO)
        };

        Ok(VndUserLimitStatus {
            user_id: user_id.0.clone(),
            tenant_id: tenant_id.0.clone(),
            tier,
            daily_used,
            monthly_used,
            daily_limit: limits.daily_limit,
            monthly_limit: limits.monthly_limit,
            daily_remaining,
            monthly_remaining,
            daily_reset_at: daily_reset,
            monthly_reset_at: monthly_reset,
            next_daily_reset: self.get_next_daily_reset(),
            next_monthly_reset: self.get_next_monthly_reset(),
        })
    }

    /// Get the current configuration
    pub fn get_config(&self) -> &VndLimitConfig {
        &self.config
    }

    /// Update configuration (for admin use)
    pub fn update_config(&mut self, config: VndLimitConfig) {
        self.config = config;
    }
}

/// Mock data provider for testing

pub struct MockVndLimitDataProvider {
    pub daily_used: std::sync::Mutex<Decimal>,
    pub monthly_used: std::sync::Mutex<Decimal>,
    pub tier: std::sync::Mutex<KycTier>,
    pub user_override: std::sync::Mutex<Option<VndTierLimits>>,
    pub recorded_transactions: std::sync::Mutex<Vec<(String, Decimal, String)>>,
}

impl MockVndLimitDataProvider {
    pub fn new() -> Self {
        Self {
            daily_used: std::sync::Mutex::new(Decimal::ZERO),
            monthly_used: std::sync::Mutex::new(Decimal::ZERO),
            tier: std::sync::Mutex::new(KycTier::Tier1),
            user_override: std::sync::Mutex::new(None),
            recorded_transactions: std::sync::Mutex::new(Vec::new()),
        }
    }

    pub fn with_daily_used(self, amount: Decimal) -> Self {
        *self.daily_used.lock().expect("Lock poisoned") = amount;
        self
    }

    pub fn with_monthly_used(self, amount: Decimal) -> Self {
        *self.monthly_used.lock().expect("Lock poisoned") = amount;
        self
    }

    pub fn with_tier(self, tier: KycTier) -> Self {
        *self.tier.lock().expect("Lock poisoned") = tier;
        self
    }

    pub fn with_override(self, limits: VndTierLimits) -> Self {
        *self.user_override.lock().expect("Lock poisoned") = Some(limits);
        self
    }
}

impl Default for MockVndLimitDataProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl VndLimitDataProvider for MockVndLimitDataProvider {
    async fn get_daily_used(
        &self,
        _tenant_id: &TenantId,
        _user_id: &UserId,
        _since: DateTime<Utc>,
    ) -> Result<Decimal> {
        Ok(*self.daily_used.lock().expect("Lock poisoned"))
    }

    async fn get_monthly_used(
        &self,
        _tenant_id: &TenantId,
        _user_id: &UserId,
        _since: DateTime<Utc>,
    ) -> Result<Decimal> {
        Ok(*self.monthly_used.lock().expect("Lock poisoned"))
    }

    async fn get_user_tier(&self, _tenant_id: &TenantId, _user_id: &UserId) -> Result<KycTier> {
        Ok(*self.tier.lock().expect("Lock poisoned"))
    }

    async fn record_transaction(
        &self,
        _tenant_id: &TenantId,
        user_id: &UserId,
        amount_vnd: Decimal,
        transaction_type: &str,
    ) -> Result<()> {
        self.recorded_transactions
            .lock()
            .expect("Lock poisoned")
            .push((user_id.0.clone(), amount_vnd, transaction_type.to_string()));
        Ok(())
    }

    async fn get_user_limit_override(
        &self,
        _tenant_id: &TenantId,
        _user_id: &UserId,
    ) -> Result<Option<VndTierLimits>> {
        Ok(self.user_override.lock().expect("Lock poisoned").clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn create_test_checker(provider: Arc<MockVndLimitDataProvider>) -> VndLimitChecker {
        VndLimitChecker::new(VndLimitConfig::default(), provider)
    }

    #[tokio::test]
    async fn test_tier1_within_limits() {
        let provider = Arc::new(MockVndLimitDataProvider::new().with_tier(KycTier::Tier1));
        let checker = create_test_checker(provider);

        let result = checker
            .check_limit(
                &TenantId::new("tenant1"),
                &UserId::new("user1"),
                dec!(10_000_000), // 10M VND
                "PAYIN_VND",
            )
            .await
            .unwrap();

        assert!(result.is_approved());
    }

    #[tokio::test]
    async fn test_tier1_single_transaction_exceeded() {
        let provider = Arc::new(MockVndLimitDataProvider::new().with_tier(KycTier::Tier1));
        let checker = create_test_checker(provider);

        let result = checker
            .check_limit(
                &TenantId::new("tenant1"),
                &UserId::new("user1"),
                dec!(60_000_000), // 60M VND, exceeds 50M single limit
                "PAYIN_VND",
            )
            .await
            .unwrap();

        assert!(matches!(
            result,
            VndLimitResult::SingleTransactionExceeded { .. }
        ));
    }

    #[tokio::test]
    async fn test_tier1_daily_limit_exceeded() {
        let provider = Arc::new(
            MockVndLimitDataProvider::new()
                .with_tier(KycTier::Tier1)
                .with_daily_used(dec!(90_000_000)), // Already used 90M
        );
        let checker = create_test_checker(provider);

        let result = checker
            .check_limit(
                &TenantId::new("tenant1"),
                &UserId::new("user1"),
                dec!(20_000_000), // 20M VND, would exceed 100M daily
                "PAYIN_VND",
            )
            .await
            .unwrap();

        assert!(matches!(result, VndLimitResult::DailyLimitExceeded { .. }));
    }

    #[tokio::test]
    async fn test_tier1_monthly_limit_exceeded() {
        let provider = Arc::new(
            MockVndLimitDataProvider::new()
                .with_tier(KycTier::Tier1)
                .with_monthly_used(dec!(990_000_000)), // Already used 990M
        );
        let checker = create_test_checker(provider);

        let result = checker
            .check_limit(
                &TenantId::new("tenant1"),
                &UserId::new("user1"),
                dec!(20_000_000), // 20M VND, would exceed 1B monthly
                "PAYIN_VND",
            )
            .await
            .unwrap();

        assert!(matches!(
            result,
            VndLimitResult::MonthlyLimitExceeded { .. }
        ));
    }

    #[tokio::test]
    async fn test_tier0_not_allowed() {
        let provider = Arc::new(MockVndLimitDataProvider::new().with_tier(KycTier::Tier0));
        let checker = create_test_checker(provider);

        let result = checker
            .check_limit(
                &TenantId::new("tenant1"),
                &UserId::new("user1"),
                dec!(1_000_000),
                "PAYIN_VND",
            )
            .await
            .unwrap();

        assert!(matches!(result, VndLimitResult::TierNotAllowed { .. }));
    }

    #[tokio::test]
    async fn test_tier2_higher_limits() {
        let provider = Arc::new(
            MockVndLimitDataProvider::new()
                .with_tier(KycTier::Tier2)
                .with_daily_used(dec!(100_000_000)), // 100M used, within Tier2's 500M
        );
        let checker = create_test_checker(provider);

        let result = checker
            .check_limit(
                &TenantId::new("tenant1"),
                &UserId::new("user1"),
                dec!(100_000_000), // 100M VND
                "PAYIN_VND",
            )
            .await
            .unwrap();

        assert!(result.is_approved());
    }

    #[tokio::test]
    async fn test_tier3_unlimited_daily() {
        let provider = Arc::new(
            MockVndLimitDataProvider::new()
                .with_tier(KycTier::Tier3)
                .with_daily_used(dec!(10_000_000_000)), // 10B used
        );
        let checker = create_test_checker(provider);

        let result = checker
            .check_limit(
                &TenantId::new("tenant1"),
                &UserId::new("user1"),
                dec!(500_000_000), // 500M VND
                "PAYIN_VND",
            )
            .await
            .unwrap();

        assert!(result.is_approved());
    }

    #[tokio::test]
    async fn test_user_limit_status() {
        let provider = Arc::new(
            MockVndLimitDataProvider::new()
                .with_tier(KycTier::Tier1)
                .with_daily_used(dec!(30_000_000))
                .with_monthly_used(dec!(200_000_000)),
        );
        let checker = create_test_checker(provider);

        let status = checker
            .get_user_limit_status(&TenantId::new("tenant1"), &UserId::new("user1"))
            .await
            .unwrap();

        assert_eq!(status.tier, KycTier::Tier1);
        assert_eq!(status.daily_used, dec!(30_000_000));
        assert_eq!(status.monthly_used, dec!(200_000_000));
        assert_eq!(status.daily_limit, dec!(100_000_000));
        assert_eq!(status.monthly_limit, dec!(1_000_000_000));
        assert_eq!(status.daily_remaining, dec!(70_000_000));
        assert_eq!(status.monthly_remaining, dec!(800_000_000));
    }

    #[tokio::test]
    async fn test_custom_tier_limits() {
        let mut config = VndLimitConfig::default();
        config.tier_limits.insert(
            1,
            VndTierLimits {
                single_transaction_limit: dec!(200_000_000), // 200M
                daily_limit: dec!(500_000_000),              // 500M
                monthly_limit: dec!(5_000_000_000),          // 5B
                requires_manual_approval_threshold: None,
            },
        );

        let provider = Arc::new(MockVndLimitDataProvider::new().with_tier(KycTier::Tier1));
        let checker = VndLimitChecker::new(config, provider);

        let result = checker
            .check_limit(
                &TenantId::new("tenant1"),
                &UserId::new("user1"),
                dec!(100_000_000), // 100M VND, within custom 200M limit
                "PAYIN_VND",
            )
            .await
            .unwrap();

        assert!(result.is_approved());
    }

    #[tokio::test]
    async fn test_record_transaction() {
        let provider = Arc::new(MockVndLimitDataProvider::new().with_tier(KycTier::Tier1));
        let checker = create_test_checker(provider.clone());

        checker
            .record_transaction(
                &TenantId::new("tenant1"),
                &UserId::new("user1"),
                dec!(10_000_000),
                "PAYIN_VND",
            )
            .await
            .unwrap();

        let recorded = provider.recorded_transactions.lock().unwrap();
        assert_eq!(recorded.len(), 1);
        assert_eq!(recorded[0].0, "user1");
        assert_eq!(recorded[0].1, dec!(10_000_000));
    }

    #[test]
    fn test_tier_limits_defaults() {
        let tier0 = VndTierLimits::tier0();
        assert!(tier0.daily_limit.is_zero());

        let tier1 = VndTierLimits::tier1();
        assert_eq!(tier1.daily_limit, dec!(100_000_000));
        assert_eq!(tier1.monthly_limit, dec!(1_000_000_000));

        let tier2 = VndTierLimits::tier2();
        assert_eq!(tier2.daily_limit, dec!(500_000_000));
        assert_eq!(tier2.monthly_limit, dec!(5_000_000_000));

        let tier3 = VndTierLimits::tier3();
        assert_eq!(tier3.daily_limit, Decimal::MAX);
    }

    #[tokio::test]
    async fn test_skip_enforcement_on_payin() {
        let config = VndLimitConfig {
            enforce_on_payin: false,
            ..Default::default()
        };
        let provider = Arc::new(
            MockVndLimitDataProvider::new()
                .with_tier(KycTier::Tier1)
                .with_daily_used(dec!(200_000_000)), // Over limit
        );
        let checker = VndLimitChecker::new(config, provider);

        let result = checker
            .check_limit(
                &TenantId::new("tenant1"),
                &UserId::new("user1"),
                dec!(100_000_000),
                "PAYIN_VND",
            )
            .await
            .unwrap();

        // Should be approved because enforcement is disabled
        assert!(result.is_approved());
    }
}
